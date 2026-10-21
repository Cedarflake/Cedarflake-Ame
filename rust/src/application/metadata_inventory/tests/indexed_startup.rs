use super::*;
use crate::application::indexed_startup::prepare;
use crate::ports::search_index::{IndexedFileHint, SearchIndex};
use std::sync::atomic::Ordering;

struct FixtureIndex {
    rows: Vec<IndexedFileHint>,
    interrupted: bool,
}

impl SearchIndex for FixtureIndex {
    fn read(
        &self,
        _: &str,
        _: &AtomicBool,
        accept: &mut dyn FnMut(&[IndexedFileHint]) -> Result<(), ScanError>,
    ) -> Result<u64, ScanError> {
        for page in self.rows.chunks(256) {
            accept(page)?;
        }
        if self.interrupted {
            return Err(ScanError::new(
                "search_index_interrupted",
                "Fixture index interruption",
            ));
        }
        Ok(self.rows.len() as u64)
    }
}

fn hint(fixture: &InventoryFixture, path: &str) -> IndexedFileHint {
    let location = fixture.location(path).unwrap();
    IndexedFileHint {
        relative_path: path.into(),
        file_size: Some(location.file_size),
        modified_unix_ms: Some(location.modified_unix_ms),
        is_image_candidate: true,
    }
}

fn start(fixture: &mut InventoryFixture, index: &FixtureIndex) -> (LeasedLibraryChange, u32) {
    let lease = enqueue_authorized_recovery_control_with_reason(
        fixture,
        "indexed-recovery",
        2_000,
        LibraryRecoveryAuthorityReason::JournalTrim,
    );
    let root = fixture
        .catalog
        .load_incremental_catalog_root(&fixture.root_id)
        .unwrap()
        .unwrap();
    let count = prepare(
        &mut fixture.catalog,
        index,
        &root,
        &lease,
        2_000,
        queue_policy(),
        &AtomicBool::new(false),
    )
    .unwrap();
    (lease, count)
}

#[test]
fn indexed_changes_publish_before_inventory_without_completing_its_authority() {
    let mut fixture = InventoryFixture::new(&["keep.png", "change.png", "delete.png"]);
    let keep = fixture.location("keep.png").unwrap();
    let index = FixtureIndex {
        rows: vec![
            hint(&fixture, "keep.png"),
            IndexedFileHint {
                relative_path: "change.png".into(),
                file_size: Some(1),
                modified_unix_ms: Some(1),
                is_image_candidate: true,
            },
            IndexedFileHint {
                relative_path: "new.png".into(),
                file_size: Some(1),
                modified_unix_ms: Some(1),
                is_image_candidate: true,
            },
        ],
        interrupted: false,
    };
    write_png(
        &fixture.source.path().join("change.png"),
        5,
        3,
        [90, 80, 70],
    );
    write_png(&fixture.source.path().join("new.png"), 3, 5, [90, 80, 70]);
    fs::remove_file(fixture.source.path().join("delete.png")).unwrap();
    let (lease, count) = start(&mut fixture, &index);
    assert_eq!(count, 3);
    process_ready_metadata_inventory_recovery_candidates_cancellable(
        &mut fixture.catalog,
        &fixture.root_id,
        LibraryRootGeneration::initial(),
        2_001,
        queue_policy(),
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(
        fixture
            .location("change.png")
            .map(|item| (item.width, item.height)),
        Some((5, 3))
    );
    assert!(fixture.location("new.png").is_some());
    assert!(fixture.location("delete.png").is_none());
    assert_eq!(fixture.location("keep.png"), Some(keep));
    let run = fixture
        .catalog
        .load_metadata_inventory_run("indexed-recovery")
        .unwrap()
        .unwrap();
    assert!(!run.enumeration_complete);
    assert!(!run.absence_authority);
    assert_eq!(run.staged_entry_count, 0);
    assert!(
        fixture
            .catalog
            .load_metadata_inventory_recovery_authority(lease.change.id)
            .unwrap()
            .unwrap()
            .retired_unix_ms
            .is_none()
    );
    super::recovery_reuse::finish_recovery(&mut fixture, lease, "indexed-recovery");
    SqliteCatalog::open(fixture.catalog.catalog_path().to_owned()).unwrap();
}

#[test]
fn stale_index_names_and_content_are_corrected_by_complete_background_inventory() {
    let mut fixture =
        InventoryFixture::new(&["keep.png", "change.png", "delete.png", "rename.png"]);
    let index = FixtureIndex {
        rows: ["keep.png", "change.png", "delete.png", "rename.png"]
            .into_iter()
            .map(|name| hint(&fixture, name))
            .collect(),
        interrupted: false,
    };
    write_png(
        &fixture.source.path().join("change.png"),
        5,
        3,
        [90, 80, 70],
    );
    write_png(
        &fixture.source.path().join("unindexed.png"),
        3,
        5,
        [90, 80, 70],
    );
    fs::remove_file(fixture.source.path().join("delete.png")).unwrap();
    fs::rename(
        fixture.source.path().join("rename.png"),
        fixture.source.path().join("renamed.png"),
    )
    .unwrap();
    let (lease, count) = start(&mut fixture, &index);
    assert_eq!(count, 0);
    assert!(fixture.location("delete.png").is_some());
    super::recovery_reuse::finish_recovery(&mut fixture, lease, "indexed-recovery");
    assert_eq!(
        fixture
            .location("change.png")
            .map(|item| (item.width, item.height)),
        Some((5, 3))
    );
    assert!(fixture.location("delete.png").is_none());
    assert!(fixture.location("rename.png").is_none());
    assert!(fixture.location("renamed.png").is_some());
    assert!(fixture.location("unindexed.png").is_some());
}

#[test]
fn missing_index_rows_recheck_actual_files_and_preserve_unchanged_results() {
    let mut fixture = InventoryFixture::new(&["keep.png", "unindexed.png"]);
    let preserved = fixture.location("unindexed.png").unwrap();
    let index = FixtureIndex {
        rows: vec![hint(&fixture, "keep.png")],
        interrupted: false,
    };
    let (lease, count) = start(&mut fixture, &index);
    assert_eq!(count, 0);
    process_ready_metadata_inventory_recovery_candidates_cancellable(
        &mut fixture.catalog,
        &fixture.root_id,
        LibraryRootGeneration::initial(),
        2_001,
        queue_policy(),
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(fixture.location("unindexed.png"), Some(preserved));
    super::recovery_reuse::finish_recovery(&mut fixture, lease, "indexed-recovery");
}

#[test]
fn interrupted_index_cannot_publish_partial_hint_absence() {
    let mut fixture = InventoryFixture::new(&["keep.png", "unindexed.png"]);
    let lease = enqueue_authorized_recovery_control_with_reason(
        &mut fixture,
        "indexed-recovery",
        2_000,
        LibraryRecoveryAuthorityReason::JournalTrim,
    );
    let root = fixture
        .catalog
        .load_incremental_catalog_root(&fixture.root_id)
        .unwrap()
        .unwrap();
    let index = FixtureIndex {
        rows: vec![hint(&fixture, "keep.png")],
        interrupted: true,
    };
    assert_eq!(
        prepare(
            &mut fixture.catalog,
            &index,
            &root,
            &lease,
            2_000,
            queue_policy(),
            &AtomicBool::new(false)
        )
        .unwrap_err()
        .code,
        "search_index_interrupted"
    );
    assert!(
        fixture
            .catalog
            .load_metadata_inventory_run("indexed-recovery")
            .unwrap()
            .is_none()
    );
    assert!(fixture.location("unindexed.png").is_some());
    super::recovery_reuse::finish_recovery(&mut fixture, lease, "indexed-recovery");
}

#[test]
fn index_admission_is_bounded_and_retains_unfinished_recovery() {
    let mut fixture = InventoryFixture::new(&["keep.png"]);
    let index = FixtureIndex {
        rows: (0..200)
            .map(|number| IndexedFileHint {
                relative_path: format!("new-{number}.png"),
                file_size: Some(1),
                modified_unix_ms: Some(1),
                is_image_candidate: true,
            })
            .collect(),
        interrupted: false,
    };
    let (lease, count) = start(&mut fixture, &index);
    assert_eq!(count, 128);
    let run = fixture
        .catalog
        .load_metadata_inventory_run("indexed-recovery")
        .unwrap()
        .unwrap();
    assert_eq!(run.candidate_count, 128);
    assert!(!run.enumeration_complete);
    assert!(
        fixture
            .catalog
            .load_metadata_inventory_recovery_authority(lease.change.id)
            .unwrap()
            .unwrap()
            .retired_unix_ms
            .is_none()
    );
}

struct CancellingIndex(FixtureIndex);

impl SearchIndex for CancellingIndex {
    fn read(
        &self,
        root: &str,
        cancelled: &AtomicBool,
        accept: &mut dyn FnMut(&[IndexedFileHint]) -> Result<(), ScanError>,
    ) -> Result<u64, ScanError> {
        let rows = self.0.read(root, cancelled, accept)?;
        cancelled.store(true, Ordering::Release);
        Ok(rows)
    }
}

#[test]
fn cancellation_after_index_read_cannot_publish_candidates() {
    let mut fixture = InventoryFixture::new(&["keep.png", "unindexed.png"]);
    let lease = enqueue_authorized_recovery_control_with_reason(
        &mut fixture,
        "indexed-recovery",
        2_000,
        LibraryRecoveryAuthorityReason::JournalTrim,
    );
    let root = fixture
        .catalog
        .load_incremental_catalog_root(&fixture.root_id)
        .unwrap()
        .unwrap();
    let index = CancellingIndex(FixtureIndex {
        rows: vec![hint(&fixture, "keep.png")],
        interrupted: false,
    });
    let count = prepare(
        &mut fixture.catalog,
        &index,
        &root,
        &lease,
        2_000,
        queue_policy(),
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(count, 0);
    assert!(
        fixture
            .catalog
            .load_metadata_inventory_run("indexed-recovery")
            .unwrap()
            .is_none()
    );
    assert!(fixture.location("unindexed.png").is_some());
    super::recovery_reuse::finish_recovery(&mut fixture, lease, "indexed-recovery");
}

#[test]
fn replaced_lease_cannot_publish_index_candidates() {
    let mut fixture = InventoryFixture::new(&["keep.png"]);
    let lease = enqueue_authorized_recovery_control_with_reason(
        &mut fixture,
        "indexed-recovery",
        2_000,
        LibraryRecoveryAuthorityReason::JournalTrim,
    );
    let root = fixture
        .catalog
        .load_incremental_catalog_root(&fixture.root_id)
        .unwrap()
        .unwrap();
    let index = FixtureIndex {
        rows: vec![IndexedFileHint {
            relative_path: "new.png".into(),
            file_size: Some(1),
            modified_unix_ms: Some(1),
            is_image_candidate: true,
        }],
        interrupted: false,
    };
    fixture
        .catalog
        .defer_library_change(lease.change.id, lease.lease_generation, 2_001)
        .unwrap();
    let error = prepare(
        &mut fixture.catalog,
        &index,
        &root,
        &lease,
        2_002,
        queue_policy(),
        &AtomicBool::new(false),
    )
    .unwrap_err();
    assert_eq!(error.code, "metadata_inventory_spool_authority_mismatch");
    assert!(fixture.location("new.png").is_none());
    assert_eq!(
        fixture
            .catalog
            .load_metadata_inventory_run("indexed-recovery")
            .unwrap()
            .unwrap()
            .candidate_count,
        0
    );
}
