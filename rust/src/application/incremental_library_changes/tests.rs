use std::fs;
use std::path::Path;

use image::{Rgb, RgbImage};
use tempfile::{TempDir, tempdir};

use crate::adapters::{FileDiscovery, FileVisitOutcome, LocalMediaInspector, SqliteCatalog};
use crate::domain::{
    AssetLocationView, LibraryChangeIntent, LibraryChangeIntentKind, LibraryChangeOrigin,
    LibraryChangeQueuePolicy, LibraryChangeScope, LibraryRootGeneration, PreviewStatus,
    ScanRequest,
};
use crate::ports::{
    CatalogRepository, IncrementalCatalogRepository, LibraryChangeQueue, MediaInspector,
};

use super::{process_ready_library_changes, stable_id, stable_location_id, user_visible_path};

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
        process_ready_library_changes(
            &mut self.catalog,
            &self.root_id,
            LibraryRootGeneration::initial(),
            2_000,
            policy(),
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
    seed_catalog_with_metadata(source, relative_paths, None)
}

fn seed_catalog_with_metadata(
    source: TempDir,
    relative_paths: &[&str],
    metadata_override: Option<(&str, &str)>,
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
            file_identity: file.file_identity,
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
