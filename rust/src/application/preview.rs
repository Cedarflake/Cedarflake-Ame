#[cfg(test)]
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

#[cfg(test)]
use std::collections::HashMap;

use crate::adapters::{
    LocalMediaInspector, open_preview_publication_guard, open_preview_source,
    revalidate_open_preview_source,
};
#[cfg(test)]
use crate::adapters::{LocalPreviewStore, canonical_source_root_path};
use crate::domain::{
    AssetLocationView, DiscoveredFile, ExpectedFileState, LibraryChangeLane, PreviewRequest,
    PreviewStatus, ScanError, ScanIssue,
};
use crate::ports::{CatalogRepository, PreviewStore};

use super::{StoragePaths, storage_paths};

mod failure;
use failure::{FailureDisposition, apply_failure};
pub(super) mod store_admission;
#[cfg(test)]
pub(crate) use store_admission::active_preview_store;
pub(crate) use store_admission::invalidate_active_preview_store;
use store_admission::{PreviewGenerationAdmission, PreviewStoreSource};

#[cfg(test)]
type PreviewTestHook = Box<dyn FnOnce() + Send>;
#[cfg(test)]
static BEFORE_PREVIEW_COMMIT_HOOKS: OnceLock<Mutex<HashMap<String, PreviewTestHook>>> =
    OnceLock::new();
#[cfg(test)]
static AFTER_PREVIEW_REVALIDATION_HOOKS: OnceLock<Mutex<HashMap<String, PreviewTestHook>>> =
    OnceLock::new();
#[cfg(test)]
static AFTER_PREVIEW_PREFLIGHT_HOOKS: OnceLock<Mutex<HashMap<String, PreviewTestHook>>> =
    OnceLock::new();
#[cfg(test)]
static BEFORE_PREVIEW_CATALOG_PUBLISH_HOOKS: OnceLock<Mutex<HashMap<String, PreviewTestHook>>> =
    OnceLock::new();
#[cfg(debug_assertions)]
const SLOW_PREVIEW_DIAGNOSTIC: Duration = Duration::from_millis(250);

struct PreviewStageTimings {
    access_ms: u128,
    store_ms: u128,
    catalog_ms: u128,
    source_ms: u128,
    materialize_ms: u128,
    commit_ms: u128,
    reclaim_ms: u128,
    publish_ms: u128,
    materialization: &'static str,
}

#[derive(Debug)]
struct PreviewCatalogContext {
    location: AssetLocationView,
    stored_source_root: Option<(
        String,
        crate::domain::LibraryRootGeneration,
        Option<crate::domain::FileIdentityEvidence>,
    )>,
}

impl Default for PreviewStageTimings {
    fn default() -> Self {
        Self {
            access_ms: 0,
            store_ms: 0,
            catalog_ms: 0,
            source_ms: 0,
            materialize_ms: 0,
            commit_ms: 0,
            reclaim_ms: 0,
            publish_ms: 0,
            materialization: "not_started",
        }
    }
}

pub fn materialize_preview(request: PreviewRequest) -> Result<AssetLocationView, ScanError> {
    let started = Instant::now();
    let location_id = request.location_id.clone();
    let is_retry = request.retry_failed;
    let mut timings = PreviewStageTimings::default();
    let result = (|| {
        validate_request(&request)?;
        let storage = storage_paths()?;
        materialize_active_preview(request, storage, &mut timings)
    })();
    log_preview_diagnostic(&location_id, is_retry, started.elapsed(), &timings, &result);
    result
}

fn materialize_active_preview(
    request: PreviewRequest,
    storage: StoragePaths,
    timings: &mut PreviewStageTimings,
) -> Result<AssetLocationView, ScanError> {
    let catalog_started = Instant::now();
    let preflight = load_preview_catalog_context(&storage, &request);
    timings.catalog_ms = timings
        .catalog_ms
        .saturating_add(catalog_started.elapsed().as_millis());
    preflight?;
    #[cfg(test)]
    run_preview_test_hook(&AFTER_PREVIEW_PREFLIGHT_HOOKS, &request.location_id);
    let admission = PreviewStoreSource::Active(&storage).generation(timings)?;
    materialize_preview_attempt(request, admission, true, timings)
}

#[cfg(test)]
pub(crate) fn materialize_preview_with_storage(
    request: PreviewRequest,
    storage: StoragePaths,
) -> Result<AssetLocationView, ScanError> {
    let _test_lock = super::PREVIEW_LIFECYCLE_TEST_LOCK
        .lock()
        .expect("preview lifecycle test lock");
    validate_request(&request)?;
    load_preview_catalog_context(&storage, &request)?;
    #[cfg(test)]
    run_preview_test_hook(&AFTER_PREVIEW_PREFLIGHT_HOOKS, &request.location_id);
    let preview_store =
        LocalPreviewStore::new(storage.preview_root.clone(), storage.preview_budget_bytes)
            .map_err(|issue| ScanError::new(issue.code, issue.message))?;
    materialize_preview_with_store(request, storage, &preview_store)
}

#[cfg(test)]
pub(crate) fn materialize_preview_with_store(
    request: PreviewRequest,
    storage: StoragePaths,
    preview_store: &LocalPreviewStore,
) -> Result<AssetLocationView, ScanError> {
    validate_request(&request)?;
    let mut timings = PreviewStageTimings::default();
    let admission = PreviewStoreSource::Injected {
        storage: &storage,
        store: preview_store,
    }
    .generation(&mut timings)?;
    materialize_preview_attempt(request, admission, true, &mut timings)
}

fn materialize_preview_attempt(
    request: PreviewRequest,
    preview_access: PreviewGenerationAdmission<'_>,
    can_reclaim: bool,
    timings: &mut PreviewStageTimings,
) -> Result<AssetLocationView, ScanError> {
    let storage = preview_access.storage();
    let preview_store = preview_access.store();
    let catalog_started = Instant::now();
    let catalog_state = load_preview_catalog_context(storage, &request);
    timings.catalog_ms = timings
        .catalog_ms
        .saturating_add(catalog_started.elapsed().as_millis());
    let PreviewCatalogContext {
        mut location,
        stored_source_root,
    } = catalog_state?;
    let Some((stored_source_root_path, expected_root_generation, expected_root_identity)) =
        stored_source_root
    else {
        timings.materialization = "skipped_failed";
        return Ok(location);
    };
    #[cfg(windows)]
    if expected_root_identity.is_none() {
        return Err(ScanError::new(
            "preview_root_identity_unproven",
            "The preview source root lacks durable Windows identity evidence",
        ));
    }
    let had_ready_preview = matches!(location.preview_status, PreviewStatus::Ready);

    let mut expected = ExpectedFileState {
        absolute_path: location.absolute_path.clone(),
        file_size: location.file_size,
        modified_unix_ms: location.modified_unix_ms,
        file_identity: location.file_identity.clone(),
        source_revision: location.source_revision.clone(),
    };
    let source_started = Instant::now();
    let opened_source = open_preview_source(
        &expected,
        std::path::Path::new(&stored_source_root_path),
        expected_root_identity.as_ref(),
    )
    .map_err(preview_source_open_error);
    timings.source_ms = timings
        .source_ms
        .saturating_add(source_started.elapsed().as_millis());
    let opened_source = opened_source?;
    let source_root_path = opened_source
        .source_root_path
        .to_string_lossy()
        .into_owned();
    if expected.source_revision.is_none() {
        expected.source_revision = opened_source.source_revision.clone();
        location.source_revision = opened_source.source_revision.clone();
    }
    if expected.source_revision.is_none() {
        return Err(ScanError::new(
            "preview_source_revision_unproven",
            "The requested source has no revision evidence",
        ));
    }
    let file = DiscoveredFile {
        source_root_path: source_root_path.clone(),
        absolute_path: location.absolute_path.clone(),
        relative_path: location.relative_path.clone(),
        file_size: location.file_size,
        created_unix_ms: location.created_unix_ms,
        modified_unix_ms: location.modified_unix_ms,
        file_identity: location.file_identity.clone(),
        source_revision: location.source_revision.clone(),
        source_generation: location.source_generation,
        issues: Vec::new(),
    };
    if super::media_metadata_is_invalidated(&location)
        && let Ok(inspection) =
            LocalMediaInspector::new().inspect_open_source(&file, &opened_source.file)
    {
        location.width = inspection.width;
        location.height = inspection.height;
        location.metadata_engine_id = inspection.metadata.engine_id;
        location.metadata_engine_version = inspection.metadata.engine_version;
        location.capture_time = inspection.metadata.capture_time;
    }
    let materialize_started = Instant::now();
    let materialization = preview_store.materialize(
        &file,
        &opened_source.file,
        request.preview_edge,
        location.width,
        location.height,
        request.retry_failed,
    );
    timings.materialize_ms = timings
        .materialize_ms
        .saturating_add(materialize_started.elapsed().as_millis());
    let publication_guard;
    let mut retained_preview_failure = None;
    let artifact = match materialization {
        Ok(materialization) => {
            timings.materialization = if materialization.staged_path.is_some() {
                "generated"
            } else {
                "cache_hit"
            };
            #[cfg(test)]
            run_preview_test_hook(&BEFORE_PREVIEW_COMMIT_HOOKS, &request.location_id);
            let commit_started = Instant::now();
            if let Err(issue) = revalidate_open_preview_source(&opened_source.file, &expected) {
                let _ = preview_store.discard_staged(&materialization);
                return Err(source_superseded(issue));
            }
            #[cfg(test)]
            run_preview_test_hook(&AFTER_PREVIEW_REVALIDATION_HOOKS, &request.location_id);
            publication_guard = Some(
                match open_preview_publication_guard(
                    &expected,
                    std::path::Path::new(&source_root_path),
                    expected_root_identity.as_ref(),
                ) {
                    Ok(guard) => guard,
                    Err(issue) => {
                        let _ = preview_store.discard_staged(&materialization);
                        return Err(source_superseded(issue));
                    }
                },
            );
            match preview_store.commit(materialization) {
                Ok(preview) => {
                    timings.commit_ms = timings
                        .commit_ms
                        .saturating_add(commit_started.elapsed().as_millis());
                    location.preview_path = preview.path.clone();
                    location.width = preview.width;
                    location.height = preview.height;
                    location.preview_status = PreviewStatus::Ready;
                    location.preview_issue_code = None;
                    location.preview_issue_message = None;
                    Some(preview)
                }
                Err(issue) => {
                    timings.commit_ms = timings
                        .commit_ms
                        .saturating_add(commit_started.elapsed().as_millis());
                    if failure::disposition(&issue, request.retry_failed, had_ready_preview)
                        == FailureDisposition::RetainReady
                        && preview_store.has_usable_artifact(&location.preview_path)
                    {
                        timings.materialization = "retained_ready_after_commit_failure";
                        retained_preview_failure = Some(issue);
                    } else {
                        apply_failure(&mut location, &issue);
                    }
                    None
                }
            }
        }
        Err(issue) if issue.code == "preview_cache_budget_exceeded" && can_reclaim => {
            timings.materialization = "capacity_reclaim";
            let required_bytes = preview_store.take_rejected_reservation_bytes();
            let source = preview_access.source();
            drop(preview_access);
            #[cfg(all(test, windows))]
            tests::store_reacquisition::before_reclamation(&request.location_id);
            let mut protected_location_ids = request.protected_location_ids.clone();
            protected_location_ids.push(request.location_id.clone());
            protected_location_ids.sort_unstable();
            protected_location_ids.dedup();
            let reclaim_started = Instant::now();
            super::preview_reclamation::reclaim_admitted_capacity(
                source.reclamation()?,
                &protected_location_ids,
                required_bytes,
            )?;
            timings.reclaim_ms = timings
                .reclaim_ms
                .saturating_add(reclaim_started.elapsed().as_millis());
            #[cfg(all(test, windows))]
            tests::store_reacquisition::after_reclamation(&request.location_id);
            let admission = source.generation(timings)?;
            return materialize_preview_attempt(request, admission, false, timings);
        }
        Err(issue) => {
            timings.materialization = "failed";
            revalidate_open_preview_source(&opened_source.file, &expected)
                .map_err(source_superseded)?;
            #[cfg(test)]
            run_preview_test_hook(&AFTER_PREVIEW_REVALIDATION_HOOKS, &request.location_id);
            publication_guard = Some(
                open_preview_publication_guard(
                    &expected,
                    std::path::Path::new(&source_root_path),
                    expected_root_identity.as_ref(),
                )
                .map_err(source_superseded)?,
            );
            if failure::disposition(&issue, request.retry_failed, had_ready_preview)
                == FailureDisposition::RetainReady
                && preview_store.has_usable_artifact(&location.preview_path)
            {
                timings.materialization = "retained_ready_after_materialization_failure";
                retained_preview_failure = Some(issue);
            } else {
                apply_failure(&mut location, &issue);
            }
            None
        }
    };
    if let Some(publication_guard) = publication_guard.as_ref() {
        revalidate_open_preview_source(publication_guard.source_file(), &expected)
            .map_err(source_superseded)?;
    }
    if let Some(issue) = retained_preview_failure {
        return Err(ScanError::new(issue.code, issue.message));
    }
    let publication_authority = crate::ports::PreviewPublicationAuthority {
        root_generation: expected_root_generation,
        root_identity: expected_root_identity,
    };
    publish_preview_state(
        storage,
        &request,
        &publication_authority,
        &location,
        artifact.as_ref(),
        timings,
    )?;
    drop(publication_guard);
    Ok(location)
}

fn load_preview_catalog_context(
    storage: &StoragePaths,
    request: &PreviewRequest,
) -> Result<PreviewCatalogContext, ScanError> {
    let reader = super::catalog_session::open_catalog_reader(&storage.catalog_path)?;
    let (location, source_root) = reader.load_preview_context(&request.location_id)?;
    let location = location.ok_or_else(|| {
        ScanError::new(
            "preview_location_not_found",
            "The requested location is not active in the catalog",
        )
    })?;
    if location.root_id != request.expected_root_id
        || location.scan_id != request.expected_scan_id
        || location.source_generation != request.expected_source_generation
        || location.source_revision != request.expected_source_revision
    {
        return Err(ScanError::new(
            "preview_request_superseded",
            "The requested source context is no longer active",
        ));
    }
    if matches!(location.preview_status, PreviewStatus::Failed) && !request.retry_failed {
        return Ok(PreviewCatalogContext {
            location,
            stored_source_root: None,
        });
    }
    let source_root = source_root.ok_or_else(|| {
        ScanError::new(
            "preview_root_not_found",
            "The requested location no longer belongs to a registered library root",
        )
    })?;
    #[cfg(windows)]
    if source_root.publication_root_identity.is_none() {
        return Err(ScanError::new(
            "preview_root_identity_unproven",
            "The preview source root lacks durable Windows identity evidence",
        ));
    }
    Ok(PreviewCatalogContext {
        location,
        stored_source_root: Some((
            source_root.root_path,
            source_root.root_generation,
            source_root.publication_root_identity,
        )),
    })
}

fn publish_preview_state(
    storage: &StoragePaths,
    request: &PreviewRequest,
    publication_authority: &crate::ports::PreviewPublicationAuthority,
    location: &AssetLocationView,
    artifact: Option<&crate::domain::PreviewArtifact>,
    timings: &mut PreviewStageTimings,
) -> Result<(), ScanError> {
    let publish_started = Instant::now();
    let result = (|| {
        let mut catalog = super::catalog_session::open_catalog(
            &storage.catalog_path,
            LibraryChangeLane::Recovery,
        )?;
        #[cfg(test)]
        run_preview_test_hook(&BEFORE_PREVIEW_CATALOG_PUBLISH_HOOKS, &request.location_id);
        catalog.update_active_preview_with_authority(
            location,
            artifact,
            Some(request),
            Some(publication_authority),
        )
    })();
    timings.publish_ms = timings
        .publish_ms
        .saturating_add(publish_started.elapsed().as_millis());
    result.map_err(normalize_preview_publication_error)
}

fn normalize_preview_publication_error(error: ScanError) -> ScanError {
    if matches!(
        error.code.as_str(),
        "active_preview_location_stale" | "active_preview_authority_stale"
    ) {
        return ScanError::new("preview_request_superseded", error.message);
    }
    error
}

fn log_preview_diagnostic(
    location_id: &str,
    is_retry: bool,
    elapsed: Duration,
    timings: &PreviewStageTimings,
    result: &Result<AssetLocationView, ScanError>,
) {
    #[cfg(debug_assertions)]
    {
        let (outcome, code) = match result {
            Ok(location) if matches!(location.preview_status, PreviewStatus::Ready) => {
                ("ready", "none")
            }
            Ok(location) if matches!(location.preview_status, PreviewStatus::Failed) => (
                "failed",
                location.preview_issue_code.as_deref().unwrap_or("unknown"),
            ),
            Ok(_) => ("pending", "none"),
            Err(error) => ("error", error.code.as_str()),
        };
        if elapsed >= SLOW_PREVIEW_DIAGNOSTIC || !matches!(code, "none") {
            eprintln!(
                "[Ame preview] outcome={outcome} code={code} retry={is_retry} total_ms={} \
                 access_ms={} store_ms={} catalog_ms={} source_ms={} materialize_ms={} \
                 commit_ms={} reclaim_ms={} publish_ms={} materialization={} location_id={location_id}",
                elapsed.as_millis(),
                timings.access_ms,
                timings.store_ms,
                timings.catalog_ms,
                timings.source_ms,
                timings.materialize_ms,
                timings.commit_ms,
                timings.reclaim_ms,
                timings.publish_ms,
                timings.materialization,
            );
        }
    }
    #[cfg(not(debug_assertions))]
    let _ = (location_id, is_retry, elapsed, timings, result);
}

fn source_superseded(issue: ScanIssue) -> ScanError {
    ScanError::new("preview_request_superseded", issue.message)
}

fn preview_source_open_error(issue: ScanIssue) -> ScanError {
    if matches!(
        issue.code.as_str(),
        "preview_root_unavailable"
            | "preview_root_identity_unproven"
            | "preview_root_identity_changed"
    ) {
        return ScanError::new(issue.code, issue.message);
    }
    source_superseded(issue)
}

fn validate_request(request: &PreviewRequest) -> Result<(), ScanError> {
    if request.location_id.trim().is_empty() {
        return Err(ScanError::new(
            "preview_location_id_empty",
            "The preview location identifier is required",
        ));
    }
    if request.expected_root_id.trim().is_empty() {
        return Err(ScanError::new(
            "preview_root_id_empty",
            "The preview root identifier is required",
        ));
    }
    if request.expected_scan_id.trim().is_empty() {
        return Err(ScanError::new(
            "preview_scan_id_empty",
            "The preview scan identifier is required",
        ));
    }
    if request.expected_source_generation == 0 {
        return Err(ScanError::new(
            "preview_source_generation_invalid",
            "The preview source generation must be nonzero",
        ));
    }
    if let Some(revision) = request.expected_source_revision.as_ref()
        && (revision.scheme != "windows-file-change-time-100ns-v1"
            || revision.value.len() != 16
            || !revision
                .value
                .bytes()
                .all(|value| value.is_ascii_hexdigit() && !value.is_ascii_uppercase()))
    {
        return Err(ScanError::new(
            "preview_source_revision_invalid",
            "The preview source revision is not a canonical Windows ChangeTime token",
        ));
    }
    if !(96..=1024).contains(&request.preview_edge) {
        return Err(ScanError::new(
            "preview_edge_invalid",
            "Preview edge must be between 96 and 1024 pixels",
        ));
    }
    if request.protected_location_ids.len() > 4_096 {
        return Err(ScanError::new(
            "preview_protected_set_too_large",
            "Preview reclamation protection is limited to 4096 locations",
        ));
    }
    Ok(())
}

#[cfg(test)]
fn run_preview_test_hook(
    hooks: &OnceLock<Mutex<HashMap<String, PreviewTestHook>>>,
    location_id: &str,
) {
    let hook = hooks
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .expect("preview test hooks")
        .remove(location_id);
    if let Some(hook) = hook {
        hook();
    }
}

#[cfg(test)]
fn install_preview_test_hook(
    hooks: &OnceLock<Mutex<HashMap<String, PreviewTestHook>>>,
    location_id: &str,
    hook: impl FnOnce() + Send + 'static,
) {
    hooks
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .expect("preview test hooks")
        .insert(location_id.to_owned(), Box::new(hook));
}

#[cfg(test)]
mod tests {
    #[cfg(windows)]
    mod cache_namespace;
    #[cfg(windows)]
    mod failure;
    #[cfg(windows)]
    pub(super) mod store_reacquisition;
    use std::fs;
    use std::io::Cursor;
    use std::path::Path;

    use exif::experimental::Writer;
    use exif::{Field, In, Tag, Value};
    use image::codecs::jpeg::JpegEncoder;
    use image::{ExtendedColorType, ImageEncoder, Rgb, RgbImage};
    use tempfile::tempdir;

    use crate::adapters::SqliteCatalog;
    use crate::domain::ScanRequest;

    use super::*;

    #[test]
    fn preview_request_requires_exact_root_scan_generation_and_canonical_revision() {
        let mut request = PreviewRequest {
            location_id: "location".to_owned(),
            expected_root_id: String::new(),
            expected_scan_id: "scan".to_owned(),
            expected_source_revision: None,
            expected_source_generation: 1,
            preview_edge: 256,
            retry_failed: false,
            protected_location_ids: Vec::new(),
        };
        assert_eq!(
            validate_request(&request).expect_err("empty root").code,
            "preview_root_id_empty"
        );
        request.expected_root_id = "root".to_owned();
        request.expected_scan_id.clear();
        assert_eq!(
            validate_request(&request).expect_err("empty scan").code,
            "preview_scan_id_empty"
        );
        request.expected_scan_id = "scan".to_owned();
        request.expected_source_generation = 0;
        assert_eq!(
            validate_request(&request)
                .expect_err("zero generation")
                .code,
            "preview_source_generation_invalid"
        );
        request.expected_source_generation = 1;
        request.expected_source_revision = Some(crate::domain::SourceRevisionEvidence {
            scheme: "windows-file-change-time-100ns-v1".to_owned(),
            value: "ABCDEF0000000001".to_owned(),
        });
        assert_eq!(
            validate_request(&request)
                .expect_err("noncanonical revision")
                .code,
            "preview_source_revision_invalid"
        );
    }

    #[cfg(windows)]
    #[test]
    fn preview_initial_source_open_binds_the_configured_root_once() {
        let fixture = preview_fixture("single-initial-root-proof");
        let source_root = fixture.source_path.parent().expect("source root");
        let root_path = canonical_source_root_path(source_root)
            .expect("canonical source root")
            .to_string_lossy()
            .into_owned();
        crate::adapters::reset_configured_root_open_instrumentation(&root_path);

        materialize_preview_with_storage(fixture.request, fixture.storage)
            .expect("materialize preview");

        assert_eq!(
            crate::adapters::configured_root_open_count(&root_path, true),
            1,
            "initial source containment and revision proof must share one root handle",
        );
    }

    #[test]
    fn consecutive_preview_materializations_share_one_validated_catalog_session() {
        let directory = tempdir().expect("fixture directory");
        let source_root = directory.path().join("source");
        fs::create_dir_all(&source_root).expect("source root");
        let source_path = source_root.join("source.jpg");
        write_jpeg_with_capture_time(&source_path, 32, 24, b"2031:02:03 04:05:06");
        let metadata = source_path.metadata().expect("source metadata");
        let root_path = canonical_source_root_path(&source_root)
            .expect("canonical source root")
            .to_string_lossy()
            .into_owned();
        let storage = StoragePaths {
            catalog_path: directory.path().join("catalog").join("ame.sqlite3"),
            preview_root: directory.path().join("previews"),
            preview_budget_bytes: 64 * 1024 * 1024,
            settings_path: directory.path().join("settings").join("storage.sqlite3"),
        };
        let request = ScanRequest {
            scan_id: "preview-session-cache".to_owned(),
            root_path: root_path.clone(),
            max_items: None,
            max_entries: None,
            preview_edge: 256,
        };
        let mut catalog = SqliteCatalog::open(storage.catalog_path.clone()).expect("catalog");
        begin_preview_fixture_scan(
            &mut catalog,
            &request,
            "preview-session-root",
            &root_path,
            true,
        );
        catalog
            .prove_live_only_first_import_handoff_for_test(&request.scan_id)
            .expect("prove preview session first-import handoff");
        catalog
            .stage_location(
                &request.scan_id,
                "preview-session-root",
                &AssetLocationView {
                    asset_id: "preview-session-asset".to_owned(),
                    location_id: "preview-session-location".to_owned(),
                    root_id: "preview-session-root".to_owned(),
                    scan_id: request.scan_id.clone(),
                    absolute_path: source_path.to_string_lossy().into_owned(),
                    display_path: "source\\source.jpg".to_owned(),
                    relative_path: "source.jpg".to_owned(),
                    preview_path: String::new(),
                    file_size: metadata.len(),
                    created_unix_ms: None,
                    modified_unix_ms: source_path_modified_unix_ms(&source_path),
                    file_identity: None,
                    source_revision: None,
                    source_generation: 0,
                    width: 7,
                    height: 9,
                    preview_status: PreviewStatus::Pending,
                    preview_issue_code: None,
                    preview_issue_message: None,
                    metadata_engine_id: super::super::INVALIDATED_MEDIA_METADATA_ENGINE_ID
                        .to_owned(),
                    metadata_engine_version:
                        super::super::INVALIDATED_MEDIA_METADATA_ENGINE_VERSION.to_owned(),
                    capture_time: None,
                },
            )
            .expect("stage location");
        catalog
            .publish_scan(&request.scan_id, "preview-session-root", 1, 0)
            .expect("publish scan");
        drop(catalog);
        super::super::catalog_session::reset_catalog_session(&storage.catalog_path);
        crate::adapters::reset_full_schema_validation_count(&storage.catalog_path);

        let preview_request = PreviewRequest {
            location_id: "preview-session-location".to_owned(),
            expected_root_id: "preview-session-root".to_owned(),
            expected_scan_id: request.scan_id.clone(),
            expected_source_revision: None,
            expected_source_generation: 1,
            preview_edge: 256,
            retry_failed: false,
            protected_location_ids: Vec::new(),
        };
        let first = materialize_preview_with_storage(preview_request, storage.clone())
            .expect("first preview");
        let second = materialize_preview_with_storage(
            preview_request_for(&first, 256, false),
            storage.clone(),
        )
        .expect("second preview");

        assert!(matches!(first.preview_status, PreviewStatus::Ready));
        assert!(matches!(second.preview_status, PreviewStatus::Ready));
        assert_eq!(first.preview_path, second.preview_path);
        assert_eq!((first.width, first.height), (32, 24));
        assert_ne!(
            first.metadata_engine_id,
            super::super::INVALIDATED_MEDIA_METADATA_ENGINE_ID
        );
        assert_ne!(
            first.metadata_engine_version,
            super::super::INVALIDATED_MEDIA_METADATA_ENGINE_VERSION
        );
        assert_eq!(
            first
                .capture_time
                .as_ref()
                .expect("refreshed capture evidence")
                .local_time,
            "2031-02-03T04:05:06.000000000"
        );
        let first_capture_time = first
            .capture_time
            .as_ref()
            .expect("first refreshed capture evidence");
        let second_capture_time = second
            .capture_time
            .as_ref()
            .expect("second refreshed capture evidence");
        assert_eq!(
            second_capture_time.local_time,
            first_capture_time.local_time
        );
        assert_eq!(
            second_capture_time.offset_minutes,
            first_capture_time.offset_minutes
        );
        assert_eq!(second_capture_time.raw_value, first_capture_time.raw_value);
        assert!(matches!(
            (&second_capture_time.source, &first_capture_time.source),
            (
                crate::domain::CaptureTimeSource::Original,
                crate::domain::CaptureTimeSource::Original
            ) | (
                crate::domain::CaptureTimeSource::Digitized,
                crate::domain::CaptureTimeSource::Digitized
            ) | (
                crate::domain::CaptureTimeSource::Image,
                crate::domain::CaptureTimeSource::Image
            )
        ));
        assert_eq!(
            crate::adapters::full_schema_validation_count(&storage.catalog_path),
            1,
            "repeated preview writes must reuse one validated catalog session",
        );
    }

    #[test]
    fn changed_source_before_commit_is_superseded_without_publishing_failed() {
        let fixture = preview_fixture("before-commit");
        let changed_path = fixture.source_path.clone();
        install_preview_test_hook(
            &BEFORE_PREVIEW_COMMIT_HOOKS,
            &fixture.request.location_id,
            move || {
                fs::write(changed_path, b"changed source before commit").expect("change source")
            },
        );

        let error =
            materialize_preview_with_storage(fixture.request.clone(), fixture.storage.clone())
                .expect_err("changed source must supersede preview");

        assert_eq!(error.code, "preview_request_superseded");
        let catalog = SqliteCatalog::open(fixture.storage.catalog_path.clone()).expect("catalog");
        let location = catalog
            .load_active_location(&fixture.request.location_id)
            .expect("load location")
            .expect("active location");
        assert!(matches!(location.preview_status, PreviewStatus::Pending));
    }

    #[test]
    fn changed_source_after_decode_revalidation_is_superseded_before_commit() {
        let fixture = preview_fixture("after-revalidation");
        let changed_path = fixture.source_path.clone();
        let preview_store = LocalPreviewStore::new(
            fixture.storage.preview_root.clone(),
            fixture.storage.preview_budget_bytes,
        )
        .expect("preview store");
        install_preview_test_hook(
            &AFTER_PREVIEW_REVALIDATION_HOOKS,
            &fixture.request.location_id,
            move || {
                fs::write(changed_path, b"changed source after revalidation")
                    .expect("change source")
            },
        );

        let error = materialize_preview_with_store(
            fixture.request.clone(),
            fixture.storage.clone(),
            &preview_store,
        )
        .expect_err("changed source must supersede commit");

        assert_eq!(error.code, "preview_request_superseded");
        assert_eq!(preview_store.used_bytes(), 0);
        assert!(
            fs::read_dir(&fixture.storage.preview_root)
                .expect("preview directory")
                .next()
                .is_none(),
            "a superseded request must not retain a staged preview",
        );
        let catalog = SqliteCatalog::open(fixture.storage.catalog_path.clone()).expect("catalog");
        let location = catalog
            .load_active_location(&fixture.request.location_id)
            .expect("load location")
            .expect("active location");
        assert!(matches!(location.preview_status, PreviewStatus::Pending));
    }

    #[cfg(windows)]
    #[test]
    fn failed_forced_replacement_retains_ready_catalog_owner_and_reports_failure() {
        let fixture = preview_fixture("retained-replacement");
        let source_bytes = fs::read(&fixture.source_path).expect("source bytes");
        let initial =
            materialize_preview_with_storage(fixture.request.clone(), fixture.storage.clone())
                .expect("initial preview");
        let old_bytes = fs::read(&initial.preview_path).expect("initial preview bytes");
        let old_size = u64::try_from(old_bytes.len()).expect("initial preview size");
        let preview_store = LocalPreviewStore::new(
            fixture.storage.preview_root.clone(),
            fixture.storage.preview_budget_bytes,
        )
        .expect("preview store");
        crate::adapters::fail_next_atomic_replace_for_test(Path::new(&initial.preview_path));

        let error = materialize_preview_with_store(
            preview_request_for(&initial, 256, true),
            fixture.storage.clone(),
            &preview_store,
        )
        .expect_err("replacement failure must be reported");

        assert_eq!(error.code, "preview_publish_failed");
        assert_eq!(
            fs::read(&initial.preview_path).expect("retained preview bytes"),
            old_bytes,
        );
        assert_eq!(
            fs::read(&fixture.source_path).expect("source bytes after retry"),
            source_bytes,
        );
        assert_eq!(preview_store.used_bytes(), old_size);
        let catalog_path = fixture.storage.catalog_path.clone();
        let catalog = SqliteCatalog::open(catalog_path.clone()).expect("catalog");
        let retained = catalog
            .load_active_location(&initial.location_id)
            .expect("load location")
            .expect("active location");
        assert!(matches!(retained.preview_status, PreviewStatus::Ready));
        assert_eq!(retained.preview_path, initial.preview_path);
        drop(catalog);
        let connection = rusqlite::Connection::open(catalog_path).expect("catalog connection");
        let owner_count: i64 = connection
            .query_row(
                "SELECT COUNT(*)
                 FROM preview_artifact_locations AS owners
                 JOIN preview_artifacts AS artifacts
                   ON artifacts.artifact_key = owners.artifact_key
                 WHERE owners.location_id = ?1
                   AND artifacts.artifact_path = ?2
                   AND artifacts.lifecycle_state = 'ready'",
                rusqlite::params![retained.location_id, retained.preview_path],
                |row| row.get(0),
            )
            .expect("preview owner query");
        assert_eq!(owner_count, 1);
    }

    #[cfg(windows)]
    #[test]
    fn forced_replacement_succeeds_when_ready_preview_exactly_fills_budget() {
        let fixture = preview_fixture("exact-budget-replacement");
        let source_bytes = fs::read(&fixture.source_path).expect("source bytes");
        let initial =
            materialize_preview_with_storage(fixture.request.clone(), fixture.storage.clone())
                .expect("initial preview");
        let settled_size = Path::new(&initial.preview_path)
            .metadata()
            .expect("initial preview metadata")
            .len();
        let preview_store =
            LocalPreviewStore::new(fixture.storage.preview_root.clone(), settled_size)
                .expect("exact-budget preview store");

        let replaced = materialize_preview_with_store(
            preview_request_for(&initial, 256, true),
            fixture.storage.clone(),
            &preview_store,
        )
        .expect("exact-budget forced replacement");

        assert!(matches!(replaced.preview_status, PreviewStatus::Ready));
        assert_eq!(replaced.preview_path, initial.preview_path);
        assert_eq!(preview_store.used_bytes(), settled_size);
        assert_eq!(
            fs::read(&fixture.source_path).expect("source bytes after retry"),
            source_bytes,
        );
    }

    #[test]
    fn stale_catalog_preview_publication_is_reported_as_superseded() {
        let error = normalize_preview_publication_error(ScanError::new(
            "active_preview_location_stale",
            "The active catalog location changed before its preview was updated",
        ));

        assert_eq!(error.code, "preview_request_superseded");
    }

    #[cfg(windows)]
    #[test]
    fn preview_without_durable_root_identity_fails_before_cache_or_catalog_publication() {
        let fixture = preview_fixture_with_root_identity("unproven-root", false);

        let error =
            materialize_preview_with_storage(fixture.request.clone(), fixture.storage.clone())
                .expect_err("an unproven root must not publish a preview");

        assert_eq!(error.code, "preview_root_identity_unproven");
        assert!(
            !fixture.storage.preview_root.exists()
                || fs::read_dir(&fixture.storage.preview_root)
                    .expect("preview cache directory")
                    .next()
                    .is_none(),
            "root authority must fail before a preview artifact is staged",
        );
        let catalog = SqliteCatalog::open(fixture.storage.catalog_path.clone()).expect("catalog");
        let location = catalog
            .load_active_location(&fixture.request.location_id)
            .expect("load location")
            .expect("active location");
        assert!(matches!(location.preview_status, PreviewStatus::Pending));
        assert!(location.preview_path.is_empty());
    }

    #[cfg(windows)]
    #[test]
    fn unavailable_preview_root_keeps_its_actionable_failure_code() {
        let fixture = preview_fixture("unavailable-root");
        let source_root = fixture.source_path.parent().expect("source root");
        fs::remove_file(&fixture.source_path).expect("remove source");
        fs::remove_dir(source_root).expect("remove source root");

        let error =
            materialize_preview_with_storage(fixture.request.clone(), fixture.storage.clone())
                .expect_err("an unavailable root must fail closed");

        assert_eq!(error.code, "preview_root_unavailable");
        assert!(
            !fixture.storage.preview_root.exists()
                || fs::read_dir(&fixture.storage.preview_root)
                    .expect("preview cache directory")
                    .next()
                    .is_none(),
            "an unavailable source root must not stage a preview artifact",
        );
    }

    #[cfg(windows)]
    #[test]
    fn unproven_root_preflight_does_not_wait_for_preview_recovery_access() {
        let fixture = preview_fixture_with_root_identity("unproven-root-lock-order", false);
        let _recovery_access =
            super::super::acquire_preview_reclamation().expect("hold preview recovery access");
        let started = Instant::now();

        let error = load_preview_catalog_context(&fixture.storage, &fixture.request)
            .expect_err("an unproven root must fail during catalog preflight");

        assert_eq!(error.code, "preview_root_identity_unproven");
        assert!(
            started.elapsed() < Duration::from_secs(2),
            "catalog authority preflight must not queue behind preview recovery access",
        );
        assert!(!fixture.storage.preview_root.exists());
    }

    #[cfg(windows)]
    #[test]
    fn root_identity_is_revalidated_inside_generation_access() {
        let fixture = preview_fixture("root-proof-revalidation");
        let catalog_path = fixture.storage.catalog_path.clone();
        let root_id = fixture.request.expected_root_id.clone();
        install_preview_test_hook(
            &AFTER_PREVIEW_PREFLIGHT_HOOKS,
            &fixture.request.location_id,
            move || {
                rusqlite::Connection::open(catalog_path)
                    .expect("root-proof catalog")
                    .execute(
                        "DELETE FROM library_root_publication_namespaces WHERE root_id = ?1",
                        [root_id],
                    )
                    .expect("remove root proof after preflight");
            },
        );

        let error =
            materialize_preview_with_storage(fixture.request.clone(), fixture.storage.clone())
                .expect_err("root proof removed after preflight must fail closed");

        assert_eq!(error.code, "preview_root_identity_unproven");
        assert!(
            !fixture.storage.preview_root.exists()
                || fs::read_dir(&fixture.storage.preview_root)
                    .expect("preview root")
                    .next()
                    .is_none(),
            "the lock-held authority recheck must run before materialization",
        );
    }

    #[cfg(windows)]
    #[test]
    fn publication_namespace_revocation_after_generation_recheck_rolls_back_publication() {
        let fixture = preview_fixture("namespace-revoked-before-publish");
        let catalog_path = fixture.storage.catalog_path.clone();
        let root_id = fixture.request.expected_root_id.clone();
        install_preview_test_hook(
            &BEFORE_PREVIEW_CATALOG_PUBLISH_HOOKS,
            &fixture.request.location_id,
            move || {
                rusqlite::Connection::open(catalog_path)
                    .expect("publication authority catalog")
                    .execute(
                        "DELETE FROM library_root_publication_namespaces WHERE root_id = ?1",
                        [root_id],
                    )
                    .expect("revoke publication namespace after generation recheck");
            },
        );

        let error =
            materialize_preview_with_storage(fixture.request.clone(), fixture.storage.clone())
                .expect_err("revoked publication authority must supersede the preview");

        assert_eq!(error.code, "preview_request_superseded");
        assert_eq!(preview_owner_count(&fixture.storage.catalog_path), 0);
        assert_active_preview_is_pending(
            &fixture.storage.catalog_path,
            &fixture.request.location_id,
        );
    }

    #[cfg(windows)]
    #[test]
    fn root_generation_change_after_generation_recheck_rolls_back_publication() {
        let fixture = preview_fixture("generation-advanced-before-publish");
        let catalog_path = fixture.storage.catalog_path.clone();
        let root_id = fixture.request.expected_root_id.clone();
        install_preview_test_hook(
            &BEFORE_PREVIEW_CATALOG_PUBLISH_HOOKS,
            &fixture.request.location_id,
            move || {
                rusqlite::Connection::open(catalog_path)
                    .expect("root generation catalog")
                    .execute(
                        "UPDATE library_change_root_state
                         SET generation = generation + 1
                         WHERE root_id = ?1 AND is_active = 1",
                        [root_id],
                    )
                    .expect("advance root generation after generation recheck");
            },
        );

        let error =
            materialize_preview_with_storage(fixture.request.clone(), fixture.storage.clone())
                .expect_err("advanced root generation must supersede the preview");

        assert_eq!(error.code, "preview_request_superseded");
        assert_eq!(preview_owner_count(&fixture.storage.catalog_path), 0);
        assert_active_preview_is_pending(
            &fixture.storage.catalog_path,
            &fixture.request.location_id,
        );
    }

    #[cfg(windows)]
    #[test]
    fn preview_namespace_guard_remains_live_through_catalog_publication() {
        let fixture = preview_fixture("catalog-publish-guard");
        let source_root = fixture
            .source_path
            .parent()
            .expect("source root")
            .to_path_buf();
        let moved_root = source_root.with_file_name("source-moved-during-publish");
        let guarded_root = source_root.clone();
        let guarded_destination = moved_root.clone();
        install_preview_test_hook(
            &BEFORE_PREVIEW_CATALOG_PUBLISH_HOOKS,
            &fixture.request.location_id,
            move || {
                let error = fs::rename(&guarded_root, &guarded_destination)
                    .expect_err("the root must remain pinned while catalog publication runs");
                assert!(matches!(error.raw_os_error(), Some(5) | Some(32)));
            },
        );

        let location =
            materialize_preview_with_storage(fixture.request.clone(), fixture.storage.clone())
                .expect("guarded preview publication");

        assert!(matches!(location.preview_status, PreviewStatus::Ready));
        fs::rename(&source_root, &moved_root).expect("root rename after publication guard drop");
        fs::rename(&moved_root, &source_root).expect("restore source root");
    }

    #[cfg(windows)]
    #[test]
    fn final_source_guard_blocks_write_delete_and_replacement_through_catalog_publication() {
        let fixture = preview_fixture("source-sharing-guard");
        let source_bytes = fs::read(&fixture.source_path).expect("source bytes");
        let replacement_path = fixture.source_path.with_file_name("replacement.png");
        fs::write(&replacement_path, &source_bytes).expect("replacement source fixture");
        let guarded_source = fixture.source_path.clone();
        let guarded_replacement = replacement_path.clone();
        install_preview_test_hook(
            &BEFORE_PREVIEW_CATALOG_PUBLISH_HOOKS,
            &fixture.request.location_id,
            move || {
                let write_error = fs::OpenOptions::new()
                    .write(true)
                    .truncate(true)
                    .open(&guarded_source)
                    .expect_err("the final source guard must reject an in-place write");
                assert_windows_sharing_violation(&write_error);
                let delete_error = fs::remove_file(&guarded_source)
                    .expect_err("the final source guard must reject source deletion");
                assert_windows_sharing_violation(&delete_error);
                let replace_error = fs::rename(&guarded_replacement, &guarded_source)
                    .expect_err("the final source guard must reject source replacement");
                assert_windows_sharing_violation(&replace_error);
                assert!(guarded_replacement.exists());
            },
        );

        let location =
            materialize_preview_with_storage(fixture.request.clone(), fixture.storage.clone())
                .expect("guarded preview publication");

        assert!(matches!(location.preview_status, PreviewStatus::Ready));
        assert_eq!(
            fs::read(&fixture.source_path).expect("guarded source bytes"),
            source_bytes,
        );
        assert_preview_matches_source(&fixture.source_path, Path::new(&location.preview_path));
        assert_unique_ready_preview_owner(
            &fixture.storage.catalog_path,
            &location.location_id,
            &location.preview_path,
        );

        fs::write(&fixture.source_path, &source_bytes)
            .expect("the in-place write must succeed after the guard is released");
        fs::remove_file(&fixture.source_path)
            .expect("source deletion must succeed after the guard is released");
        fs::rename(&replacement_path, &fixture.source_path)
            .expect("source replacement must succeed after the guard is released");
        assert_eq!(
            fs::read(&fixture.source_path).expect("replacement source bytes"),
            source_bytes,
        );
        assert_preview_matches_source(&fixture.source_path, Path::new(&location.preview_path));
    }

    #[cfg(windows)]
    #[test]
    fn startup_recovery_removes_artifact_installed_before_deterministic_supersession() {
        let _test_lock = crate::application::PREVIEW_LIFECYCLE_TEST_LOCK
            .lock()
            .expect("preview lifecycle test lock");
        let fixture = preview_fixture("installed-before-supersession");
        invalidate_active_preview_store().expect("reset active preview store");
        let preview_store = active_preview_store(&fixture.storage).expect("active preview store");
        let hook_storage = fixture.storage.clone();
        let hook_source_root = fixture
            .source_path
            .parent()
            .expect("source root")
            .to_path_buf();
        let hook_request = fixture.request.clone();
        install_preview_test_hook(
            &BEFORE_PREVIEW_CATALOG_PUBLISH_HOOKS,
            &fixture.request.location_id,
            move || {
                supersede_preview_fixture_scan(&hook_storage, &hook_source_root, &hook_request);
            },
        );

        let error = materialize_preview_with_store(
            fixture.request.clone(),
            fixture.storage.clone(),
            &preview_store,
        )
        .expect_err("the replacement scan must supersede preview publication");

        assert_eq!(error.code, "preview_request_superseded");
        let orphan_path = only_current_preview_artifact(&fixture.storage.preview_root);
        let orphan_bytes = orphan_path
            .metadata()
            .expect("orphan artifact metadata")
            .len();
        assert!(orphan_bytes > 0);
        assert_eq!(preview_store.used_bytes(), orphan_bytes);
        assert_eq!(preview_owner_count(&fixture.storage.catalog_path), 0);
        assert_active_preview_is_pending(
            &fixture.storage.catalog_path,
            &fixture.request.location_id,
        );

        super::super::preview_recovery::run_preview_recovery_for_test(&fixture.storage)
            .expect("startup preview recovery");

        assert!(!orphan_path.exists());
        assert_eq!(preview_owner_count(&fixture.storage.catalog_path), 0);
        assert_active_preview_is_pending(
            &fixture.storage.catalog_path,
            &fixture.request.location_id,
        );
        let refreshed_store =
            active_preview_store(&fixture.storage).expect("recovered preview store");
        assert!(!Arc::ptr_eq(&preview_store, &refreshed_store));
        assert_eq!(refreshed_store.used_bytes(), 0);
    }

    struct PreviewFixture {
        _directory: tempfile::TempDir,
        storage: StoragePaths,
        source_path: std::path::PathBuf,
        request: PreviewRequest,
    }

    fn preview_fixture(suffix: &str) -> PreviewFixture {
        preview_fixture_with_root_identity(suffix, true)
    }

    fn preview_fixture_with_root_identity(
        suffix: &str,
        retain_root_identity: bool,
    ) -> PreviewFixture {
        let mut bytes = Cursor::new(Vec::new());
        image::DynamicImage::ImageRgb8(RgbImage::from_pixel(32, 24, Rgb([24, 96, 192])))
            .write_to(&mut bytes, image::ImageFormat::Png)
            .expect("source PNG");
        preview_fixture_with_bytes(suffix, retain_root_identity, &bytes.into_inner())
    }

    fn preview_fixture_with_bytes(
        suffix: &str,
        retain_root_identity: bool,
        bytes: &[u8],
    ) -> PreviewFixture {
        let directory = tempdir().expect("fixture directory");
        let source_root = directory.path().join("source");
        fs::create_dir_all(&source_root).expect("source root");
        let source_path = source_root.join("source.png");
        fs::write(&source_path, bytes).expect("source image");
        let metadata = source_path.metadata().expect("source metadata");
        let root_path = canonical_source_root_path(&source_root)
            .expect("canonical source root")
            .to_string_lossy()
            .into_owned();
        let storage = StoragePaths {
            catalog_path: directory.path().join("catalog").join("ame.sqlite3"),
            preview_root: directory.path().join("previews"),
            preview_budget_bytes: 64 * 1024 * 1024,
            settings_path: directory.path().join("settings").join("storage.sqlite3"),
        };
        let scan_id = format!("preview-{suffix}-scan");
        let root_id = format!("preview-{suffix}-root");
        let location_id = format!("preview-{suffix}-location");
        let request = ScanRequest {
            scan_id: scan_id.clone(),
            root_path: root_path.clone(),
            max_items: None,
            max_entries: None,
            preview_edge: 256,
        };
        let mut catalog = SqliteCatalog::open(storage.catalog_path.clone()).expect("catalog");
        begin_preview_fixture_scan(
            &mut catalog,
            &request,
            &root_id,
            &root_path,
            retain_root_identity,
        );
        catalog
            .prove_live_only_first_import_handoff_for_test(&scan_id)
            .expect("prove preview fixture first-import handoff");
        catalog
            .stage_location(
                &scan_id,
                &root_id,
                &AssetLocationView {
                    asset_id: format!("preview-{suffix}-asset"),
                    location_id: location_id.clone(),
                    root_id: root_id.clone(),
                    scan_id: scan_id.clone(),
                    absolute_path: source_path.to_string_lossy().into_owned(),
                    display_path: "source\\source.png".to_owned(),
                    relative_path: "source.png".to_owned(),
                    preview_path: String::new(),
                    file_size: metadata.len(),
                    created_unix_ms: None,
                    modified_unix_ms: source_path_modified_unix_ms(&source_path),
                    file_identity: None,
                    source_revision: None,
                    source_generation: 0,
                    width: 32,
                    height: 24,
                    preview_status: PreviewStatus::Pending,
                    preview_issue_code: None,
                    preview_issue_message: None,
                    metadata_engine_id: "fixture".to_owned(),
                    metadata_engine_version: "1".to_owned(),
                    capture_time: None,
                },
            )
            .expect("stage location");
        catalog
            .publish_scan(&scan_id, &root_id, 1, 0)
            .expect("publish scan");
        PreviewFixture {
            _directory: directory,
            storage,
            source_path,
            request: PreviewRequest {
                location_id,
                expected_root_id: root_id,
                expected_scan_id: scan_id,
                expected_source_revision: None,
                expected_source_generation: 1,
                preview_edge: 256,
                retry_failed: false,
                protected_location_ids: Vec::new(),
            },
        }
    }

    fn preview_request_for(
        location: &AssetLocationView,
        preview_edge: u32,
        retry_failed: bool,
    ) -> PreviewRequest {
        PreviewRequest {
            location_id: location.location_id.clone(),
            expected_root_id: location.root_id.clone(),
            expected_scan_id: location.scan_id.clone(),
            expected_source_revision: location.source_revision.clone(),
            expected_source_generation: location.source_generation,
            preview_edge,
            retry_failed,
            protected_location_ids: Vec::new(),
        }
    }

    fn source_path_modified_unix_ms(path: &Path) -> i64 {
        let modified = path
            .metadata()
            .expect("source metadata")
            .modified()
            .expect("source modified time")
            .duration_since(std::time::UNIX_EPOCH)
            .expect("source modified time after epoch");
        i64::try_from(modified.as_millis()).expect("modified milliseconds")
    }

    fn write_jpeg_with_capture_time(path: &Path, width: u32, height: u32, capture_time: &[u8]) {
        let capture_field = Field {
            tag: Tag::DateTimeOriginal,
            ifd_num: In::PRIMARY,
            value: Value::Ascii(vec![capture_time.to_vec()]),
        };
        let mut exif_writer = Writer::new();
        exif_writer.push_field(&capture_field);
        let mut exif = Cursor::new(Vec::new());
        exif_writer.write(&mut exif, false).expect("encode EXIF");

        let byte_count = usize::try_from(u64::from(width) * u64::from(height) * 3)
            .expect("JPEG fixture byte count");
        let mut jpeg = Vec::new();
        let mut encoder = JpegEncoder::new(&mut jpeg);
        encoder
            .set_exif_metadata(exif.into_inner())
            .expect("set JPEG EXIF");
        encoder
            .encode(&vec![0; byte_count], width, height, ExtendedColorType::Rgb8)
            .expect("encode JPEG");
        fs::write(path, jpeg).expect("write JPEG fixture");
    }

    fn begin_preview_fixture_scan(
        catalog: &mut SqliteCatalog,
        request: &ScanRequest,
        root_id: &str,
        root_path: &str,
        retain_root_identity: bool,
    ) {
        #[cfg(windows)]
        if retain_root_identity {
            let root_identity = crate::adapters::FileDiscovery::new(root_path)
                .expect("preview fixture source discovery")
                .metadata_inventory_root_identity()
                .expect("preview fixture root identity query")
                .expect("preview fixture Windows root identity");
            catalog
                .begin_scan_with_publication_namespace(request, root_id, root_path, &root_identity)
                .expect("begin preview fixture scan with root identity");
            return;
        }
        let _ = retain_root_identity;
        catalog
            .begin_scan(request, root_id, root_path)
            .expect("begin preview fixture scan");
    }

    #[cfg(windows)]
    fn assert_windows_sharing_violation(error: &std::io::Error) {
        assert!(
            matches!(error.raw_os_error(), Some(5) | Some(32)),
            "expected Windows access-denied or sharing-violation evidence, got {error}",
        );
    }

    #[cfg(windows)]
    fn assert_preview_matches_source(source_path: &Path, preview_path: &Path) {
        let source = image::open(source_path).expect("decode source").to_rgb8();
        let preview = image::open(preview_path).expect("decode preview").to_rgb8();
        assert_eq!(
            u64::from(preview.width()) * u64::from(source.height()),
            u64::from(source.width()) * u64::from(preview.height()),
            "preview aspect ratio must describe the guarded source",
        );
        let source_pixel = source.get_pixel(source.width() / 2, source.height() / 2);
        let preview_pixel = preview.get_pixel(preview.width() / 2, preview.height() / 2);
        for (source_channel, preview_channel) in source_pixel.0.iter().zip(preview_pixel.0.iter()) {
            assert!(
                source_channel.abs_diff(*preview_channel) <= 4,
                "preview pixels must describe the guarded source",
            );
        }
    }

    #[cfg(windows)]
    fn assert_unique_ready_preview_owner(
        catalog_path: &Path,
        location_id: &str,
        preview_path: &str,
    ) {
        let (owner_count, owner_location): (i64, Option<String>) =
            rusqlite::Connection::open(catalog_path)
                .expect("catalog connection")
                .query_row(
                    "SELECT COUNT(*), MIN(owners.location_id)
                 FROM preview_artifact_locations AS owners
                 JOIN preview_artifacts AS artifacts
                   ON artifacts.artifact_key = owners.artifact_key
                 WHERE artifacts.artifact_path = ?1
                   AND artifacts.lifecycle_state = 'ready'",
                    [preview_path],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .expect("ready preview ownership");
        assert_eq!(owner_count, 1);
        assert_eq!(owner_location.as_deref(), Some(location_id));
    }

    #[cfg(windows)]
    fn preview_owner_count(catalog_path: &Path) -> i64 {
        rusqlite::Connection::open(catalog_path)
            .expect("catalog connection")
            .query_row(
                "SELECT COUNT(*) FROM preview_artifact_locations",
                [],
                |row| row.get(0),
            )
            .expect("preview owner count")
    }

    #[cfg(windows)]
    fn only_current_preview_artifact(preview_root: &Path) -> std::path::PathBuf {
        let artifacts = fs::read_dir(preview_root)
            .expect("preview root")
            .map(|entry| entry.expect("preview entry").path())
            .filter(|path| crate::adapters::current_preview_artifact_key(path).is_some())
            .collect::<Vec<_>>();
        assert_eq!(artifacts.len(), 1, "one installed v3 artifact is expected");
        artifacts.into_iter().next().expect("installed v3 artifact")
    }

    #[cfg(windows)]
    fn assert_active_preview_is_pending(catalog_path: &Path, location_id: &str) {
        let (preview_status, preview_path): (String, String) =
            rusqlite::Connection::open(catalog_path)
                .expect("catalog connection")
                .query_row(
                    "SELECT preview_status, preview_path
                     FROM asset_locations
                     WHERE location_id = ?1",
                    [location_id],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .expect("active preview state");
        assert_eq!(preview_status, "pending");
        assert!(preview_path.is_empty());
    }

    #[cfg(windows)]
    fn supersede_preview_fixture_scan(
        storage: &StoragePaths,
        source_root: &Path,
        request: &PreviewRequest,
    ) {
        let mut catalog = SqliteCatalog::open(storage.catalog_path.clone()).expect("catalog");
        let mut replacement = catalog
            .load_active_location(&request.location_id)
            .expect("active location query")
            .expect("active location");
        let scan_id = format!("{}-replacement", request.expected_scan_id);
        let root_path = canonical_source_root_path(source_root)
            .expect("canonical replacement source root")
            .to_string_lossy()
            .into_owned();
        let replacement_scan = ScanRequest {
            scan_id: scan_id.clone(),
            root_path: root_path.clone(),
            max_items: None,
            max_entries: None,
            preview_edge: 256,
        };
        begin_preview_fixture_scan(
            &mut catalog,
            &replacement_scan,
            &request.expected_root_id,
            &root_path,
            true,
        );
        replacement.scan_id = scan_id.clone();
        replacement.source_generation = 0;
        replacement.preview_path.clear();
        replacement.preview_status = PreviewStatus::Pending;
        replacement.preview_issue_code = None;
        replacement.preview_issue_message = None;
        catalog
            .stage_location(&scan_id, &request.expected_root_id, &replacement)
            .expect("stage replacement location");
        catalog
            .publish_scan(&scan_id, &request.expected_root_id, 1, 0)
            .expect("publish replacement scan");
    }
}
