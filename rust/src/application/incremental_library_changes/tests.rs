use std::fs;
use std::path::Path;

use image::{Rgb, RgbImage};
use tempfile::{TempDir, tempdir};

use crate::adapters::{FileDiscovery, FileVisitOutcome, LocalMediaInspector, SqliteCatalog};
use crate::domain::{
    AssetLocationView, DerivedEvidenceDisposition, LibraryChangeIntent, LibraryChangeIntentKind,
    LibraryChangeOrigin, LibraryChangeQueuePolicy, LibraryChangeScope, LibraryRootGeneration,
    PreviewStatus, ScanRequest,
};
use crate::ports::{
    CatalogRepository, IncrementalCatalogRepository, LibraryChangeQueue, MediaInspector,
};

use super::{
    prepare_change, process_ready_library_changes, stable_id, stable_location_id, user_visible_path,
};

#[test]
fn unchanged_path_completes_without_incrementing_the_catalog_revision() {
    let source = tempdir().expect("source directory");
    write_png(&source.path().join("same.png"), 2, 2, [10, 20, 30]);
    let original_bytes = fs::read(source.path().join("same.png")).expect("source bytes");
    let mut fixture = seed_catalog(source, &["same.png"]);
    let revision = fixture.revision();
    fixture.enqueue(&[intent(&fixture.root_id, "same.png", None, 1)]);

    let report = fixture.process();

    assert_eq!(report.completed_count, 1);
    assert_eq!(report.applied_mutation_count, 0);
    assert_eq!(report.catalog_revision, revision);
    assert_eq!(fixture.revision(), revision);
    assert_eq!(
        fs::read(fixture.source.path().join("same.png")).expect("source bytes after delta"),
        original_bytes
    );
}

#[test]
fn created_file_is_added_in_one_incremental_revision() {
    let source = tempdir().expect("source directory");
    let mut fixture = seed_catalog(source, &[]);
    let revision = fixture.revision();
    write_png(
        &fixture.source.path().join("created.png"),
        3,
        2,
        [40, 50, 60],
    );
    let source_bytes = fs::read(fixture.source.path().join("created.png")).expect("source bytes");
    fixture.enqueue(&[intent(&fixture.root_id, "created.png", None, 1)]);

    let report = fixture.process();

    assert_eq!(report.completed_count, 1);
    assert_eq!(report.applied_mutation_count, 1);
    assert_eq!(report.catalog_revision, revision + 1);
    let stored = fixture.location("created.png").expect("created location");
    assert_eq!((stored.width, stored.height), (3, 2));
    assert_eq!(
        fs::read(fixture.source.path().join("created.png")).expect("source bytes after delta"),
        source_bytes
    );
}

#[test]
fn related_valid_files_publish_together_at_one_catalog_revision() {
    let source = tempdir().expect("source directory");
    let mut fixture = seed_catalog(source, &[]);
    let revision = fixture.revision();
    write_png(&fixture.source.path().join("first.png"), 2, 3, [61, 62, 63]);
    write_png(
        &fixture.source.path().join("second.png"),
        4,
        2,
        [64, 65, 66],
    );
    fixture.enqueue(&[
        intent(&fixture.root_id, "first.png", None, 1),
        intent(&fixture.root_id, "second.png", None, 2),
    ]);

    let report = fixture.process();

    assert_eq!(report.completed_count, 2);
    assert_eq!(report.applied_mutation_count, 2);
    assert_eq!(report.catalog_revision, revision + 1);
    assert_eq!(fixture.revision(), revision + 1);
    assert!(fixture.location("first.png").is_some());
    assert!(fixture.location("second.png").is_some());
}

#[test]
fn modified_file_retains_asset_identity_and_invalidates_derived_preview_state() {
    let source = tempdir().expect("source directory");
    write_png(&source.path().join("modified.png"), 2, 2, [70, 80, 90]);
    let mut fixture = seed_catalog(source, &["modified.png"]);
    let before = fixture.location("modified.png").expect("original location");
    let revision = fixture.revision();
    write_png(
        &fixture.source.path().join("modified.png"),
        5,
        3,
        [90, 80, 70],
    );
    let source_bytes = fs::read(fixture.source.path().join("modified.png")).expect("source bytes");
    fixture.enqueue(&[intent(&fixture.root_id, "modified.png", None, 1)]);

    let report = fixture.process();

    assert_eq!(report.completed_count, 1);
    assert_eq!(report.applied_mutation_count, 1);
    assert_eq!(report.catalog_revision, revision + 1);
    let after = fixture.location("modified.png").expect("modified location");
    assert_eq!(after.asset_id, before.asset_id);
    assert_eq!((after.width, after.height), (5, 3));
    assert!(matches!(after.preview_status, PreviewStatus::Pending));
    assert!(after.preview_path.is_empty());
    assert_eq!(
        fs::read(fixture.source.path().join("modified.png")).expect("source bytes after delta"),
        source_bytes
    );
}

#[test]
fn unchanged_source_is_reinspected_when_metadata_engine_identity_changes() {
    let source = tempdir().expect("source directory");
    write_png(&source.path().join("metadata.png"), 3, 2, [91, 92, 93]);
    let source_bytes = fs::read(source.path().join("metadata.png")).expect("source bytes");
    let mut fixture =
        seed_catalog_with_metadata(source, &["metadata.png"], Some(("legacy-metadata", "0")));
    let before = fixture.location("metadata.png").expect("original location");
    let revision = fixture.revision();
    fixture.enqueue(&[intent(&fixture.root_id, "metadata.png", None, 1)]);

    let report = fixture.process();

    assert_eq!(report.completed_count, 1);
    assert_eq!(report.applied_mutation_count, 1);
    assert_eq!(report.catalog_revision, revision + 1);
    let after = fixture.location("metadata.png").expect("updated location");
    assert_eq!(after.asset_id, before.asset_id);
    assert_ne!(after.metadata_engine_id, "legacy-metadata");
    assert_ne!(after.metadata_engine_version, "0");
    assert!(matches!(after.preview_status, PreviewStatus::Pending));
    assert_eq!(
        fs::read(fixture.source.path().join("metadata.png")).expect("source bytes after delta"),
        source_bytes
    );
}

#[cfg(windows)]
#[test]
fn paired_rename_preserves_the_asset_and_moves_the_location() {
    let source = tempdir().expect("source directory");
    write_png(&source.path().join("old.png"), 4, 3, [100, 110, 120]);
    let mut fixture = seed_catalog(source, &["old.png"]);
    let before = fixture.location("old.png").expect("original location");
    let revision = fixture.revision();
    fs::rename(
        fixture.source.path().join("old.png"),
        fixture.source.path().join("new.png"),
    )
    .expect("rename source fixture");
    let source_bytes = fs::read(fixture.source.path().join("new.png")).expect("renamed bytes");
    fixture.enqueue(&[intent(&fixture.root_id, "new.png", Some("old.png"), 1)]);

    let report = fixture.process();

    assert_eq!(report.completed_count, 1);
    assert_eq!(report.applied_mutation_count, 1);
    assert_eq!(report.catalog_revision, revision + 1);
    assert!(fixture.location("old.png").is_none());
    let after = fixture.location("new.png").expect("renamed location");
    assert_eq!(after.asset_id, before.asset_id);
    assert_ne!(after.location_id, before.location_id);
    assert_eq!(after.file_identity, before.file_identity);
    assert_eq!(
        fs::read(fixture.source.path().join("new.png")).expect("source bytes after delta"),
        source_bytes
    );
}

#[cfg(windows)]
#[test]
fn paired_rename_followed_by_removal_drops_the_obsolete_old_location() {
    let source = tempdir().expect("source directory");
    write_png(&source.path().join("old.png"), 4, 3, [121, 122, 123]);
    let mut fixture = seed_catalog(source, &["old.png"]);
    let revision = fixture.revision();
    fs::rename(
        fixture.source.path().join("old.png"),
        fixture.source.path().join("new.png"),
    )
    .expect("rename source fixture");
    fs::remove_file(fixture.source.path().join("new.png")).expect("remove renamed fixture");
    fixture.enqueue(&[intent(&fixture.root_id, "new.png", Some("old.png"), 1)]);

    let report = fixture.process();

    assert_eq!(report.completed_count, 1);
    assert_eq!(report.applied_mutation_count, 1);
    assert_eq!(report.catalog_revision, revision + 1);
    assert!(fixture.location("old.png").is_none());
    assert!(fixture.location("new.png").is_none());
}

#[cfg(windows)]
#[test]
fn same_path_replacement_creates_a_new_asset_identity() {
    let source = tempdir().expect("source directory");
    write_png(&source.path().join("replace.png"), 2, 2, [130, 140, 150]);
    let mut fixture = seed_catalog(source, &["replace.png"]);
    let before = fixture.location("replace.png").expect("original location");
    fs::remove_file(fixture.source.path().join("replace.png")).expect("remove original fixture");
    write_png(
        &fixture.source.path().join("replace.png"),
        6,
        2,
        [150, 140, 130],
    );
    let replacement = fixture.discovered("replace.png");
    assert_ne!(replacement.file_identity, before.file_identity);
    fixture.enqueue(&[intent(&fixture.root_id, "replace.png", None, 1)]);

    let report = fixture.process();

    assert_eq!(report.completed_count, 1);
    assert_eq!(report.applied_mutation_count, 1);
    let after = fixture
        .location("replace.png")
        .expect("replacement location");
    assert_ne!(after.asset_id, before.asset_id);
    assert_eq!(after.file_identity, replacement.file_identity);
}

#[test]
fn authoritative_absence_removes_the_location_from_the_current_projection() {
    let source = tempdir().expect("source directory");
    write_png(&source.path().join("removed.png"), 2, 3, [160, 170, 180]);
    let mut fixture = seed_catalog(source, &["removed.png"]);
    let revision = fixture.revision();
    fs::remove_file(fixture.source.path().join("removed.png")).expect("remove source fixture");
    fixture.enqueue(&[intent(&fixture.root_id, "removed.png", None, 1)]);

    let report = fixture.process();

    assert_eq!(report.completed_count, 1);
    assert_eq!(report.applied_mutation_count, 1);
    assert_eq!(report.catalog_revision, revision + 1);
    assert!(fixture.location("removed.png").is_none());
}

#[test]
fn one_unreadable_image_retries_without_blocking_a_valid_sibling() {
    let source = tempdir().expect("source directory");
    let mut fixture = seed_catalog(source, &[]);
    write_png(
        &fixture.source.path().join("valid.png"),
        3,
        4,
        [190, 200, 210],
    );
    fs::write(fixture.source.path().join("broken.jpg"), b"not a jpeg")
        .expect("malformed image fixture");
    let valid_bytes = fs::read(fixture.source.path().join("valid.png")).expect("valid bytes");
    let broken_bytes = fs::read(fixture.source.path().join("broken.jpg")).expect("broken bytes");
    fixture.enqueue(&[
        intent(&fixture.root_id, "valid.png", None, 1),
        intent(&fixture.root_id, "broken.jpg", None, 2),
    ]);

    let report = fixture.process();

    assert_eq!(report.leased_count, 2);
    assert_eq!(report.completed_count, 1);
    assert_eq!(report.retried_count, 1);
    assert_eq!(report.applied_mutation_count, 1);
    assert!(fixture.location("valid.png").is_some());
    assert!(fixture.location("broken.jpg").is_none());
    let metrics = fixture
        .catalog
        .load_library_change_queue_metrics(2_000, policy())
        .expect("queue metrics");
    assert_eq!(metrics.completed_count, 1);
    assert_eq!(metrics.retry_wait_count, 1);
    assert_eq!(
        fs::read(fixture.source.path().join("valid.png")).expect("valid bytes after delta"),
        valid_bytes
    );
    assert_eq!(
        fs::read(fixture.source.path().join("broken.jpg")).expect("broken bytes after delta"),
        broken_bytes
    );
}

#[test]
fn pending_work_waits_for_a_running_full_scan_without_consuming_a_lease() {
    let source = tempdir().expect("source directory");
    let mut fixture = seed_catalog(source, &[]);
    write_png(
        &fixture.source.path().join("waiting.png"),
        3,
        3,
        [211, 212, 213],
    );
    fixture.enqueue(&[intent(&fixture.root_id, "waiting.png", None, 1)]);
    let replacement_scan = ScanRequest {
        scan_id: "replacement-scan".to_owned(),
        root_path: fixture.root_path.clone(),
        max_items: None,
        max_entries: None,
        preview_edge: 256,
    };
    fixture
        .catalog
        .begin_scan(&replacement_scan, &fixture.root_id, &fixture.root_path)
        .expect("begin replacement scan");

    let report = fixture.process();

    assert_eq!(report.leased_count, 0);
    assert_eq!(report.retried_count, 0);
    assert_eq!(report.applied_mutation_count, 0);
    assert!(fixture.location("waiting.png").is_none());
    let metrics = fixture
        .catalog
        .load_library_change_queue_metrics(2_000, policy())
        .expect("queue metrics");
    assert_eq!(metrics.pending_count, 1);
    assert_eq!(metrics.leased_count, 0);
    assert_eq!(metrics.retry_wait_count, 0);
}

#[test]
fn authoritative_work_remains_pending_without_consuming_the_incremental_retry_budget() {
    let source = tempdir().expect("source directory");
    let mut fixture = seed_catalog(source, &[]);
    let strict_policy = LibraryChangeQueuePolicy {
        debounce_millis: 0,
        max_attempts: 1,
        ..LibraryChangeQueuePolicy::default()
    };
    fixture
        .catalog
        .enqueue_library_change_intents(
            &[LibraryChangeIntent {
                kind: LibraryChangeIntentKind::FreshnessUnknown,
                scope: LibraryChangeScope::Root,
                relative_path: String::new(),
                ..intent(&fixture.root_id, "placeholder.png", None, 1)
            }],
            1_000,
            strict_policy,
        )
        .expect("enqueue authoritative work");

    let report = fixture.process_with_policy(strict_policy);

    assert_eq!(report.leased_count, 0);
    assert_eq!(report.retried_count, 0);
    let leased = fixture
        .catalog
        .lease_library_changes(
            &fixture.root_id,
            LibraryRootGeneration::initial(),
            2_000,
            strict_policy,
        )
        .expect("authoritative worker can lease pending work");
    assert_eq!(leased.len(), 1);
    assert_eq!(leased[0].change.attempt_count, 1);
}

#[test]
fn unavailable_root_does_not_consume_a_path_retry_attempt() {
    let source = tempdir().expect("source directory");
    let mut fixture = seed_catalog(source, &[]);
    let strict_policy = LibraryChangeQueuePolicy {
        debounce_millis: 0,
        max_attempts: 1,
        ..LibraryChangeQueuePolicy::default()
    };
    fixture
        .catalog
        .enqueue_library_change_intents(
            &[intent(&fixture.root_id, "waiting.png", None, 1)],
            1_000,
            strict_policy,
        )
        .expect("enqueue path work");
    fs::remove_dir_all(fixture.source.path()).expect("make root unavailable");

    let report = fixture.process_with_policy(strict_policy);

    assert_eq!(report.leased_count, 0);
    assert_eq!(report.retried_count, 0);
    let leased = fixture
        .catalog
        .lease_library_changes(
            &fixture.root_id,
            LibraryRootGeneration::initial(),
            2_000,
            strict_policy,
        )
        .expect("path work remains leasable");
    assert_eq!(leased.len(), 1);
    assert_eq!(leased[0].change.attempt_count, 1);
}

#[cfg(windows)]
#[test]
fn identity_backfill_preserves_asset_continuity_for_a_later_rename() {
    let source = tempdir().expect("source directory");
    write_png(&source.path().join("legacy.png"), 3, 3, [11, 12, 13]);
    let mut fixture = seed_catalog_without_identity(source, &["legacy.png"]);
    let original = fixture.location("legacy.png").expect("legacy location");
    assert!(original.file_identity.is_none());
    fixture.enqueue(&[intent(&fixture.root_id, "legacy.png", None, 1)]);

    let backfill = fixture.process();

    assert_eq!(backfill.applied_mutation_count, 1);
    let identified = fixture.location("legacy.png").expect("identified location");
    assert_eq!(identified.asset_id, original.asset_id);
    assert!(identified.file_identity.is_some());
    fs::rename(
        fixture.source.path().join("legacy.png"),
        fixture.source.path().join("moved.png"),
    )
    .expect("rename identified file");
    fixture.enqueue(&[intent(&fixture.root_id, "moved.png", Some("legacy.png"), 2)]);

    fixture.process();

    let moved = fixture.location("moved.png").expect("moved location");
    assert_eq!(moved.asset_id, original.asset_id);
    assert_eq!(moved.file_identity, identified.file_identity);
}

#[cfg(windows)]
#[test]
fn paired_rename_reconciles_a_replacement_at_the_previous_path() {
    let source = tempdir().expect("source directory");
    write_png(&source.path().join("old.png"), 3, 3, [21, 22, 23]);
    let mut fixture = seed_catalog(source, &["old.png"]);
    let original = fixture.location("old.png").expect("original location");
    fs::rename(
        fixture.source.path().join("old.png"),
        fixture.source.path().join("new.png"),
    )
    .expect("rename original");
    write_png(&fixture.source.path().join("old.png"), 5, 2, [31, 32, 33]);
    fixture.enqueue(&[intent(&fixture.root_id, "new.png", Some("old.png"), 1)]);

    let report = fixture.process();

    assert_eq!(report.completed_count, 1);
    assert_eq!(report.applied_mutation_count, 2);
    let replacement = fixture.location("old.png").expect("replacement location");
    let moved = fixture.location("new.png").expect("moved location");
    assert_ne!(replacement.asset_id, original.asset_id);
    assert_eq!(moved.asset_id, original.asset_id);
    assert_ne!(replacement.file_identity, moved.file_identity);
}

#[cfg(windows)]
#[test]
fn case_only_rename_removes_the_obsolete_catalog_spelling() {
    let source = tempdir().expect("source directory");
    write_png(&source.path().join("Photo.png"), 3, 3, [34, 35, 36]);
    let mut fixture = seed_catalog(source, &["Photo.png"]);
    let original = fixture.location("Photo.png").expect("original location");
    fs::rename(
        fixture.source.path().join("Photo.png"),
        fixture.source.path().join("rename-intermediate.png"),
    )
    .expect("rename through intermediate");
    fs::rename(
        fixture.source.path().join("rename-intermediate.png"),
        fixture.source.path().join("photo.png"),
    )
    .expect("apply case-only spelling");
    fixture.enqueue(&[intent(&fixture.root_id, "photo.png", Some("Photo.png"), 1)]);

    let report = fixture.process();

    assert_eq!(report.applied_mutation_count, 1);
    assert!(fixture.location("Photo.png").is_none());
    let renamed = fixture.location("photo.png").expect("renamed location");
    assert_eq!(renamed.asset_id, original.asset_id);
    assert_eq!(renamed.file_identity, original.file_identity);
}

#[cfg(windows)]
#[test]
fn metadata_engine_mismatch_invalidates_a_rename_mutation_contract() {
    let source = tempdir().expect("source directory");
    write_png(&source.path().join("old.png"), 3, 2, [41, 42, 43]);
    let mut fixture =
        seed_catalog_with_metadata(source, &["old.png"], Some(("legacy-metadata", "0")));
    fs::rename(
        fixture.source.path().join("old.png"),
        fixture.source.path().join("new.png"),
    )
    .expect("rename source");
    fixture.enqueue(&[intent(&fixture.root_id, "new.png", Some("old.png"), 1)]);
    let leased = fixture
        .catalog
        .lease_path_library_changes(
            &fixture.root_id,
            LibraryRootGeneration::initial(),
            2_000,
            policy(),
        )
        .expect("lease rename")
        .pop()
        .expect("rename lease");
    let discovery = FileDiscovery::new(&fixture.root_path).expect("file discovery");
    let inspector = LocalMediaInspector::new();

    let prepared =
        prepare_change(&fixture.catalog, &discovery, &inspector, &leased).expect("prepare rename");
    let rename = prepared
        .mutations
        .iter()
        .find(|mutation| mutation.upsert_location.is_some())
        .expect("rename mutation");

    assert_eq!(
        rename.evidence_disposition,
        DerivedEvidenceDisposition::InvalidateDerived
    );
    let location = rename.upsert_location.as_ref().expect("renamed location");
    assert!(matches!(location.preview_status, PreviewStatus::Pending));
    assert_ne!(location.metadata_engine_id, "legacy-metadata");
}

#[cfg(windows)]
#[test]
fn compatible_rename_preserves_failed_preview_evidence() {
    let source = tempdir().expect("source directory");
    write_png(&source.path().join("old.png"), 2, 3, [51, 52, 53]);
    let mut fixture = seed_catalog(source, &["old.png"]);
    let mut failed = fixture.location("old.png").expect("original location");
    failed.preview_status = PreviewStatus::Failed;
    failed.preview_issue_code = Some("preview_decode_failed".to_owned());
    failed.preview_issue_message = Some("fixture failure".to_owned());
    fixture
        .catalog
        .update_active_preview(&failed, None)
        .expect("record failed preview");
    fs::rename(
        fixture.source.path().join("old.png"),
        fixture.source.path().join("new.png"),
    )
    .expect("rename source");
    fixture.enqueue(&[intent(&fixture.root_id, "new.png", Some("old.png"), 1)]);

    fixture.process();

    let renamed = fixture.location("new.png").expect("renamed location");
    assert!(matches!(renamed.preview_status, PreviewStatus::Failed));
    assert_eq!(
        renamed.preview_issue_code.as_deref(),
        Some("preview_decode_failed")
    );
    assert_eq!(
        renamed.preview_issue_message.as_deref(),
        Some("fixture failure")
    );
}

struct CatalogFixture {
    source: TempDir,
    _storage: TempDir,
    catalog: SqliteCatalog,
    root_id: String,
    root_path: String,
}

impl CatalogFixture {
    fn enqueue(&mut self, intents: &[LibraryChangeIntent]) {
        self.catalog
            .enqueue_library_change_intents(intents, 1_000, policy())
            .expect("enqueue changes");
    }

    fn process(&mut self) -> crate::domain::IncrementalLibraryChangeReport {
        self.process_with_policy(policy())
    }

    fn process_with_policy(
        &mut self,
        policy: LibraryChangeQueuePolicy,
    ) -> crate::domain::IncrementalLibraryChangeReport {
        process_ready_library_changes(
            &mut self.catalog,
            &self.root_id,
            LibraryRootGeneration::initial(),
            2_000,
            policy,
        )
        .expect("process changes")
    }

    fn revision(&self) -> u64 {
        self.catalog
            .load_incremental_catalog_root(&self.root_id)
            .expect("load root")
            .expect("catalog root")
            .catalog_revision
    }

    fn location(&self, relative_path: &str) -> Option<AssetLocationView> {
        self.catalog
            .load_incremental_location_by_relative_path(&self.root_id, relative_path)
            .expect("load location")
    }

    fn discovered(&self, relative_path: &str) -> crate::domain::DiscoveredFile {
        let discovery = FileDiscovery::new(&self.root_path).expect("file discovery");
        match discovery.visit_relative_path(relative_path).outcome {
            FileVisitOutcome::File(file) => file,
            _ => panic!("expected discovered file"),
        }
    }
}

fn seed_catalog(source: TempDir, relative_paths: &[&str]) -> CatalogFixture {
    seed_catalog_with_options(source, relative_paths, None, false)
}

fn seed_catalog_with_metadata(
    source: TempDir,
    relative_paths: &[&str],
    metadata_override: Option<(&str, &str)>,
) -> CatalogFixture {
    seed_catalog_with_options(source, relative_paths, metadata_override, false)
}

fn seed_catalog_without_identity(source: TempDir, relative_paths: &[&str]) -> CatalogFixture {
    seed_catalog_with_options(source, relative_paths, None, true)
}

fn seed_catalog_with_options(
    source: TempDir,
    relative_paths: &[&str],
    metadata_override: Option<(&str, &str)>,
    clear_file_identity: bool,
) -> CatalogFixture {
    let root_input = source.path().to_string_lossy().into_owned();
    let discovery = FileDiscovery::new(&root_input).expect("file discovery");
    let root_path = discovery
        .canonical_root()
        .expect("canonical root")
        .to_string_lossy()
        .into_owned();
    let root_id = stable_id("library-root-v1", &root_path);
    let storage = tempdir().expect("storage directory");
    let mut catalog =
        SqliteCatalog::open(storage.path().join("catalog.sqlite3")).expect("open catalog");
    let request = ScanRequest {
        scan_id: "baseline-scan".to_owned(),
        root_path: root_path.clone(),
        max_items: None,
        max_entries: None,
        preview_edge: 256,
    };
    catalog
        .begin_scan(&request, &root_id, &root_path)
        .expect("begin baseline scan");
    let inspector = LocalMediaInspector::new();
    for relative_path in relative_paths {
        let file = match discovery.visit_relative_path(relative_path).outcome {
            FileVisitOutcome::File(file) => file,
            _ => panic!("expected baseline file"),
        };
        let inspection = inspector.inspect(&file).expect("inspect baseline image");
        let (metadata_engine_id, metadata_engine_version) = metadata_override
            .map(|(engine_id, engine_version)| (engine_id.to_owned(), engine_version.to_owned()))
            .unwrap_or((
                inspection.metadata.engine_id,
                inspection.metadata.engine_version,
            ));
        let location = AssetLocationView {
            asset_id: stable_id("test-asset-v1", relative_path),
            location_id: stable_location_id(&root_id, relative_path),
            root_id: root_id.clone(),
            absolute_path: file.absolute_path.clone(),
            display_path: user_visible_path(&file.absolute_path),
            relative_path: file.relative_path,
            preview_path: String::new(),
            file_size: file.file_size,
            created_unix_ms: file.created_unix_ms,
            modified_unix_ms: file.modified_unix_ms,
            file_identity: (!clear_file_identity)
                .then_some(file.file_identity)
                .flatten(),
            width: inspection.width,
            height: inspection.height,
            preview_status: PreviewStatus::Pending,
            preview_issue_code: None,
            preview_issue_message: None,
            metadata_engine_id,
            metadata_engine_version,
            capture_time: inspection.metadata.capture_time,
        };
        catalog
            .stage_location(&request.scan_id, &root_id, &location)
            .expect("stage baseline location");
    }
    catalog
        .publish_scan(
            &request.scan_id,
            &root_id,
            u64::try_from(relative_paths.len()).expect("baseline count"),
            0,
        )
        .expect("publish baseline scan");
    CatalogFixture {
        source,
        _storage: storage,
        catalog,
        root_id,
        root_path,
    }
}

fn intent(
    root_id: &str,
    relative_path: &str,
    previous_relative_path: Option<&str>,
    sequence: u64,
) -> LibraryChangeIntent {
    LibraryChangeIntent {
        root_id: root_id.to_owned(),
        root_generation: LibraryRootGeneration::initial(),
        kind: if previous_relative_path.is_some() {
            LibraryChangeIntentKind::RenameCandidate
        } else {
            LibraryChangeIntentKind::Reconcile
        },
        scope: LibraryChangeScope::Path,
        relative_path: relative_path.to_owned(),
        previous_relative_path: previous_relative_path.map(str::to_owned),
        origin: LibraryChangeOrigin::LiveNotification,
        first_observed_unix_ms: 1_000,
        most_recent_observed_unix_ms: 1_000,
        first_sequence: sequence,
        most_recent_sequence: sequence,
        coalesced_observation_count: 1,
    }
}

fn policy() -> LibraryChangeQueuePolicy {
    LibraryChangeQueuePolicy {
        debounce_millis: 0,
        ..LibraryChangeQueuePolicy::default()
    }
}

fn write_png(path: &Path, width: u32, height: u32, color: [u8; 3]) {
    let image = RgbImage::from_pixel(width, height, Rgb(color));
    image.save(path).expect("write PNG fixture");
}
