use std::sync::{Arc, Mutex, OnceLock};

use crate::adapters::{LocalPreviewStore, SqliteCatalog, revalidate_file_state};
use crate::domain::{
    AssetLocationView, DiscoveredFile, ExpectedFileState, PreviewArtifact, PreviewRequest,
    PreviewStatus, ScanError, ScanIssue,
};
use crate::ports::{CatalogRepository, PreviewStore};

use super::{StoragePaths, storage_paths};

static ACTIVE_PREVIEW_STORE: OnceLock<Mutex<Option<ActivePreviewStore>>> = OnceLock::new();

struct ActivePreviewStore {
    root: std::path::PathBuf,
    budget_bytes: u64,
    store: Arc<LocalPreviewStore>,
}

pub fn materialize_preview(request: PreviewRequest) -> Result<AssetLocationView, ScanError> {
    validate_request(&request)?;
    let storage = storage_paths()?;
    let preview_access = super::acquire_preview_generation()?;
    let preview_store = active_preview_store(&storage)?;
    materialize_preview_attempt(request, storage, &preview_store, true, Some(preview_access))
}

#[cfg(test)]
pub(crate) fn materialize_preview_with_storage(
    request: PreviewRequest,
    storage: StoragePaths,
) -> Result<AssetLocationView, ScanError> {
    let _test_lock = super::PREVIEW_LIFECYCLE_TEST_LOCK
        .lock()
        .expect("preview lifecycle test lock");
    let preview_store =
        LocalPreviewStore::new(storage.preview_root.clone(), storage.preview_budget_bytes)
            .map_err(|issue| ScanError::new(issue.code, issue.message))?;
    materialize_preview_with_store(request, storage, &preview_store)
}

#[cfg(test)]
fn materialize_preview_with_store(
    request: PreviewRequest,
    storage: StoragePaths,
    preview_store: &LocalPreviewStore,
) -> Result<AssetLocationView, ScanError> {
    validate_request(&request)?;
    materialize_preview_attempt(request, storage, preview_store, true, None)
}

fn materialize_preview_attempt(
    request: PreviewRequest,
    storage: StoragePaths,
    preview_store: &LocalPreviewStore,
    can_reclaim: bool,
    preview_access: Option<std::sync::RwLockReadGuard<'static, ()>>,
) -> Result<AssetLocationView, ScanError> {
    let preview_access = match preview_access {
        Some(access) => access,
        None => super::acquire_preview_generation()?,
    };
    let mut catalog = SqliteCatalog::open(storage.catalog_path.clone())?;
    let mut location = catalog
        .load_active_location(&request.location_id)?
        .ok_or_else(|| {
            ScanError::new(
                "preview_location_not_found",
                "The requested location is not active in the catalog",
            )
        })?;
    if matches!(location.preview_status, PreviewStatus::Failed) && !request.retry_failed {
        return Ok(location);
    }

    let expected = ExpectedFileState {
        absolute_path: location.absolute_path.clone(),
        file_size: location.file_size,
        modified_unix_ms: location.modified_unix_ms,
        file_identity: location.file_identity.clone(),
    };
    if let Err(issue) = revalidate_file_state(&expected) {
        apply_failure(&mut location, &issue);
        catalog.update_active_preview(&location, None)?;
        return Ok(location);
    }
    let file = DiscoveredFile {
        absolute_path: location.absolute_path.clone(),
        relative_path: location.relative_path.clone(),
        file_size: location.file_size,
        created_unix_ms: location.created_unix_ms,
        modified_unix_ms: location.modified_unix_ms,
        file_identity: location.file_identity.clone(),
        issues: Vec::new(),
    };
    let artifact = match preview_store.materialize(
        &file,
        request.preview_edge,
        location.width,
        location.height,
    ) {
        Ok(preview) => match revalidate_materialized_preview(&expected, preview, preview_store) {
            Ok(preview) => {
                location.preview_path = preview.path.clone();
                location.width = preview.width;
                location.height = preview.height;
                location.preview_status = PreviewStatus::Ready;
                location.preview_issue_code = None;
                location.preview_issue_message = None;
                Some(preview)
            }
            Err(issue) => {
                apply_failure(&mut location, &issue);
                None
            }
        },
        Err(issue) if issue.code == "preview_cache_budget_exceeded" && can_reclaim => {
            let required_bytes = preview_store.take_rejected_reservation_bytes();
            drop(catalog);
            drop(preview_access);
            let mut protected_location_ids = request.protected_location_ids.clone();
            protected_location_ids.push(request.location_id.clone());
            protected_location_ids.sort_unstable();
            protected_location_ids.dedup();
            super::preview_reclamation::reclaim_preview_capacity(
                &storage,
                preview_store,
                &protected_location_ids,
                required_bytes,
            )?;
            return materialize_preview_attempt(request, storage, preview_store, false, None);
        }
        Err(issue) => {
            apply_failure(&mut location, &issue);
            None
        }
    };
    catalog.update_active_preview(&location, artifact.as_ref())?;
    Ok(location)
}

fn revalidate_materialized_preview(
    expected: &ExpectedFileState,
    preview: PreviewArtifact,
    preview_store: &LocalPreviewStore,
) -> Result<PreviewArtifact, ScanIssue> {
    if let Err(issue) = revalidate_file_state(expected) {
        let _ = preview_store.discard(&preview);
        return Err(issue);
    }
    Ok(preview)
}

pub(crate) fn active_preview_store(
    storage: &StoragePaths,
) -> Result<Arc<LocalPreviewStore>, ScanError> {
    let mut active = active_preview_store_slot().lock().map_err(|_| {
        ScanError::new(
            "preview_store_registry_unavailable",
            "Preview store registry is poisoned",
        )
    })?;
    if let Some(current) = active.as_ref()
        && current.root == storage.preview_root
        && current.budget_bytes == storage.preview_budget_bytes
    {
        return Ok(Arc::clone(&current.store));
    }
    let store = Arc::new(
        LocalPreviewStore::new(storage.preview_root.clone(), storage.preview_budget_bytes)
            .map_err(|issue| {
                ScanError::new(
                    issue.code,
                    format!("Preview cache initialization failed: {}", issue.message),
                )
            })?,
    );
    *active = Some(ActivePreviewStore {
        root: storage.preview_root.clone(),
        budget_bytes: storage.preview_budget_bytes,
        store: Arc::clone(&store),
    });
    Ok(store)
}

pub(crate) fn invalidate_active_preview_store() -> Result<(), ScanError> {
    let mut active = active_preview_store_slot().lock().map_err(|_| {
        ScanError::new(
            "preview_store_registry_unavailable",
            "Preview store registry is poisoned",
        )
    })?;
    *active = None;
    Ok(())
}

fn active_preview_store_slot() -> &'static Mutex<Option<ActivePreviewStore>> {
    ACTIVE_PREVIEW_STORE.get_or_init(|| Mutex::new(None))
}

fn apply_failure(location: &mut AssetLocationView, issue: &ScanIssue) {
    location.preview_status = PreviewStatus::Failed;
    location.preview_issue_code = Some(issue.code.clone());
    location.preview_issue_message = Some(issue.message.clone());
}

fn validate_request(request: &PreviewRequest) -> Result<(), ScanError> {
    if request.location_id.trim().is_empty() {
        return Err(ScanError::new(
            "preview_location_id_empty",
            "The preview location identifier is required",
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
mod tests {
    use std::fs;
    use std::path::Path;

    use tempfile::tempdir;

    use super::*;

    #[test]
    fn changed_source_is_rejected_after_preview_materialization() {
        let directory = tempdir().expect("temporary directory");
        let source_path = directory.path().join("source.jpg");
        fs::write(&source_path, b"old").expect("source");
        let expected = ExpectedFileState {
            absolute_path: source_path.to_string_lossy().into_owned(),
            file_size: 3,
            modified_unix_ms: source_path_modified_unix_ms(&source_path),
            file_identity: None,
        };
        let preview_root = directory.path().join("previews");
        let store = LocalPreviewStore::new(preview_root.clone(), 1024 * 1024).expect("store");
        let preview_path = preview_root.join(format!(
            "{}-{}.jpg",
            crate::adapters::PREVIEW_CACHE_VERSION,
            "a".repeat(64)
        ));
        fs::write(&preview_path, b"preview").expect("preview");
        fs::write(&source_path, b"new source bytes").expect("changed source");
        let preview = PreviewArtifact {
            artifact_key: "a".repeat(64),
            path: preview_path.to_string_lossy().into_owned(),
            byte_size: 7,
            size_bucket: 256,
            encoded_width: 256,
            encoded_height: 192,
            width: 4_032,
            height: 3_024,
            algorithm_id: crate::adapters::PREVIEW_ALGORITHM_ID.to_owned(),
            algorithm_version: crate::adapters::PREVIEW_ALGORITHM_VERSION,
            orientation_contract: crate::adapters::PREVIEW_ORIENTATION_CONTRACT.to_owned(),
        };

        let issue = revalidate_materialized_preview(&expected, preview, &store)
            .expect_err("changed source must be rejected");

        assert_eq!(issue.code, "source_changed_during_scan");
        assert!(!preview_path.exists());
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
}
