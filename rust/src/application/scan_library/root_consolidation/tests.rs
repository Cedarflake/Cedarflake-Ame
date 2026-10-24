use std::fs;
use std::path::{Path, PathBuf};

use image::{Rgb, RgbImage};
use rusqlite::{Connection, params};
use tempfile::{TempDir, tempdir};

use crate::adapters::{
    LocalPreviewStore, SqliteCatalog, set_before_scan_projection_replacement_hook,
};
use crate::application::StoragePaths;
use crate::domain::{
    CatalogSnapshot, GalleryQuery, PreviewRequest, PreviewStatus, ScanEvent, ScanRequest,
};
use crate::ports::CatalogRepository;

use super::super::{cancel_scan, pause_scan, resume_scan_with_storage, run_scan_with_storage};

mod organization;

#[test]
fn parent_pause_reopen_and_resume_keeps_child_until_complete() {
    let fixture = Fixture::new();
    let original = fixture.snapshot().assets[0].asset_id.clone();
    let request = fixture.parent_request("paused-parent");
    run_scan_with_storage(
        request.clone(),
        |event| {
            if matches!(event, ScanEvent::Finalizing { .. }) {
                assert!(pause_scan(&request.scan_id));
            }
            true
        },
        fixture.storage.clone(),
    )
    .expect("pause the parent before publication");
    fixture.assert_child_publication(&original);
    let mut mappings = Vec::new();
    resume_scan_with_storage(
        request,
        |event| {
            if let ScanEvent::Completed {
                consolidated_roots, ..
            } = event
            {
                mappings = consolidated_roots;
            }
            true
        },
        fixture.storage.clone(),
    )
    .expect("resume parent");
    let after = fixture.snapshot();
    assert_eq!((after.roots.len(), after.assets.len()), (1, 2));
    assert!(after.assets.iter().any(|asset| asset.asset_id == original));
    assert_eq!(mappings.len(), 1);
    assert_eq!(mappings[0].relative_path, "旅行");
    fixture.assert_sources();
}

#[test]
fn parent_commit_cancellation_rolls_back_child_retirement() {
    let fixture = Fixture::new();
    let original = fixture.snapshot().assets[0].asset_id.clone();
    let request = fixture.parent_request("cancelled-parent");
    let scan_id = request.scan_id.clone();
    let _hook = set_before_scan_projection_replacement_hook(&request.scan_id, move |_| {
        assert!(cancel_scan(&scan_id));
    });
    let mut cancelled = false;
    run_scan_with_storage(
        request,
        |event| {
            cancelled |= matches!(event, ScanEvent::Cancelled { .. });
            assert!(!matches!(event, ScanEvent::Completed { .. }));
            true
        },
        fixture.storage.clone(),
    )
    .expect("cancel before commit");
    assert!(cancelled);
    fixture.assert_child_publication(&original);
    fixture.assert_sources();
}

#[test]
fn limited_parent_cannot_replace_a_complete_child() {
    let fixture = Fixture::new();
    let original = fixture.snapshot().assets[0].asset_id.clone();
    let mut request = fixture.parent_request("limited-parent");
    request.max_items = Some(1);
    let error = run_scan_with_storage(request, |_| true, fixture.storage.clone())
        .expect_err("partial parent cannot retire a child");
    assert_eq!(error.code, "catalog_root_consolidation_incomplete");
    fixture.assert_child_publication(&original);
    fixture.assert_sources();
}

#[test]
fn unresolved_directory_issue_keeps_the_published_child() {
    let fixture = Fixture::new();
    let original = fixture.snapshot().assets[0].asset_id.clone();
    let result = run_scan_with_storage(
        fixture.parent_request("unreadable-parent"),
        |event| {
            if let ScanEvent::Started { scan_id, .. } = event {
                Connection::open(&fixture.storage.catalog_path)
                    .expect("record discovery failure")
                    .execute(
                        "INSERT INTO scan_issues(scan_id, path, code, message)
                         VALUES (?1, ?2, 'directory_read_failed', 'directory unavailable')",
                        params![scan_id, fixture.child.to_string_lossy()],
                    )
                    .expect("persist unresolved directory evidence");
            }
            true
        },
        fixture.storage.clone(),
    );
    assert_eq!(
        result
            .expect_err("incomplete namespace must retain child")
            .code,
        "catalog_root_consolidation_incomplete"
    );
    fixture.assert_child_publication(&original);
    fixture.assert_sources();
}

#[test]
fn one_rejected_media_file_does_not_block_complete_parent_consolidation() {
    let fixture = Fixture::new();
    fs::write(fixture.source.path().join("corrupt.png"), b"not an image")
        .expect("generated corrupt media");
    let mut issue_count = 0;
    run_scan_with_storage(
        fixture.parent_request("parent-with-corrupt-media"),
        |event| {
            if let ScanEvent::Completed {
                issue_count: count, ..
            } = event
            {
                issue_count = count;
            }
            true
        },
        fixture.storage.clone(),
    )
    .expect("isolated media rejection permits complete namespace publication");
    let after = fixture.snapshot();
    assert_eq!((after.roots.len(), after.assets.len()), (1, 2));
    assert!(issue_count > 0);
    fixture.assert_sources();
}

#[test]
fn replaced_child_namespace_cannot_retire_the_original_registration() {
    let fixture = Fixture::new();
    let original = fixture.snapshot().assets[0].asset_id.clone();
    let moved = fixture.source.path().join("former-child");
    fs::rename(&fixture.child, &moved).expect("move generated directory");
    fs::create_dir(&fixture.child).expect("create a different directory at the old path");
    let result = run_scan_with_storage(
        fixture.parent_request("replacement-parent"),
        |_| true,
        fixture.storage.clone(),
    );
    assert!(result.is_err());
    fixture.assert_child_publication(&original);
    assert_eq!(
        fs::read(moved.join("child.png")).expect("original image"),
        fixture.original_child
    );
}

#[test]
fn multiple_children_consolidate_with_stable_assets_and_late_cancel() {
    let fixture = Fixture::new();
    let second = fixture.source.path().join("other");
    fs::create_dir(&second).expect("second child");
    write_image(&second.join("second.png"));
    run_scan_with_storage(
        request("second-child", &second),
        |_| true,
        fixture.storage.clone(),
    )
    .expect("second child import");
    let before = fixture.snapshot();
    let mut mappings = Vec::new();
    run_scan_with_storage(
        fixture.parent_request("combined-parent"),
        |event| {
            if let ScanEvent::Completed {
                consolidated_roots, ..
            } = event
            {
                assert!(cancel_scan("combined-parent"));
                mappings = consolidated_roots;
            }
            true
        },
        fixture.storage.clone(),
    )
    .expect("atomic parent publication");
    let after = fixture.snapshot();
    assert_eq!((after.roots.len(), after.assets.len()), (1, 3));
    assert_eq!(mappings.len(), 2);
    for asset in before.assets {
        assert!(
            after
                .assets
                .iter()
                .any(|current| current.asset_id == asset.asset_id)
        );
    }
    fixture.assert_sources();
}

#[test]
fn cancelled_unpublished_child_is_consolidated_only_with_complete_parent() {
    let fixture = Fixture::new();
    let second = fixture.source.path().join("cancelled");
    fs::create_dir(&second).expect("second child");
    write_image(&second.join("second.png"));
    run_scan_with_storage(
        request("cancelled-child", &second),
        |event| {
            if matches!(event, ScanEvent::Started { .. }) {
                assert!(cancel_scan("cancelled-child"));
            }
            true
        },
        fixture.storage.clone(),
    )
    .expect("cancelled child admission");
    let before = fixture.snapshot();
    assert_eq!(before.roots.len(), 2);
    assert_eq!(before.assets.len(), 1);
    run_scan_with_storage(
        fixture.parent_request("parent-after-cancel"),
        |_| true,
        fixture.storage.clone(),
    )
    .expect("complete parent includes the cancelled child");
    let after = fixture.snapshot();
    assert_eq!((after.roots.len(), after.assets.len()), (1, 3));
    assert!(
        after
            .assets
            .iter()
            .any(|asset| asset.asset_id == before.assets[0].asset_id)
    );
    fixture.assert_sources();
}

#[test]
fn paused_unpublished_child_keeps_its_checkpoint_and_blocks_consolidation() {
    let fixture = Fixture::new();
    let second = fixture.source.path().join("paused");
    fs::create_dir(&second).expect("second child");
    write_image(&second.join("second.png"));
    run_scan_with_storage(
        request("paused-child", &second),
        |event| {
            if matches!(event, ScanEvent::Finalizing { .. }) {
                assert!(pause_scan("paused-child"));
            }
            true
        },
        fixture.storage.clone(),
    )
    .expect("paused child admission");
    let result = run_scan_with_storage(
        fixture.parent_request("parent-after-pause"),
        |_| true,
        fixture.storage.clone(),
    );
    assert_eq!(
        result.expect_err("retained child task must survive").code,
        "catalog_root_consolidation_busy"
    );
    let catalog = SqliteCatalog::open(fixture.storage.catalog_path.clone()).expect("FULL reopen");
    assert_eq!(
        catalog
            .load_paused_scan()
            .expect("paused task")
            .expect("retained child")
            .scan_id,
        "paused-child"
    );
    fixture.assert_child_publication(&fixture.snapshot().assets[0].asset_id);
    fixture.assert_sources();
}

#[test]
fn second_child_retirement_failure_rolls_back_parent_children_and_ready_previews() {
    let _preview_lock = crate::application::PREVIEW_LIFECYCLE_TEST_LOCK
        .lock()
        .expect("preview lifecycle lock");
    let fixture = Fixture::new();
    let second = fixture.source.path().join("other");
    fs::create_dir(&second).expect("second child");
    write_image(&second.join("second.png"));
    run_scan_with_storage(
        request("second-child", &second),
        |_| true,
        fixture.storage.clone(),
    )
    .expect("second child import");
    let store = LocalPreviewStore::new(
        fixture.storage.preview_root.clone(),
        fixture.storage.preview_budget_bytes,
    )
    .expect("isolated preview store");
    for location in fixture.snapshot().assets {
        let materialized = crate::application::preview::materialize_preview_with_store(
            PreviewRequest {
                location_id: location.location_id,
                expected_root_id: location.root_id,
                expected_scan_id: location.scan_id,
                expected_source_revision: location.source_revision,
                expected_source_generation: location.source_generation,
                preview_edge: 256,
                retry_failed: true,
                protected_location_ids: Vec::new(),
            },
            fixture.storage.clone(),
            &store,
        )
        .expect("ready child preview");
        assert_eq!(materialized.preview_status, PreviewStatus::Ready);
    }
    let before = fixture.snapshot();
    let connection =
        Connection::open(&fixture.storage.catalog_path).expect("isolated failure injection");
    let previews_before = preview_records(&connection);
    connection
        .execute_batch(
            "CREATE TRIGGER fail_second_child_retirement
         BEFORE DELETE ON library_roots
         WHEN OLD.active_scan_id IN ('initial-child', 'second-child')
           AND (SELECT COUNT(*) FROM library_roots
                WHERE active_scan_id IN ('initial-child', 'second-child')) = 1
         BEGIN
           SELECT RAISE(ABORT, 'second child retirement failed');
         END;",
        )
        .expect("failure after the first child has retired inside the transaction");
    let error = run_scan_with_storage(
        fixture.parent_request("rollback-parent"),
        |_| true,
        fixture.storage.clone(),
    )
    .expect_err("second retirement aborts the complete transaction");
    assert_eq!(error.code, "catalog_database_error");
    assert!(error.message.contains("second child retirement failed"));
    let after = fixture.snapshot();
    assert_eq!(after.assets, before.assets);
    assert_eq!(
        after
            .roots
            .iter()
            .filter(|root| root.active_scan_id.is_some())
            .count(),
        2
    );
    assert_eq!(preview_records(&connection), previews_before);
    for asset in after.assets {
        assert!(Path::new(&asset.preview_path).is_file());
    }
    fixture.assert_sources();
}

fn preview_records(connection: &Connection) -> Vec<(String, String, i64)> {
    let mut statement = connection
        .prepare(
            "SELECT artifact.artifact_key, artifact.lifecycle_state,
                (SELECT COUNT(*) FROM preview_artifact_locations AS reference
                 WHERE reference.artifact_key = artifact.artifact_key)
         FROM preview_artifacts AS artifact ORDER BY artifact.artifact_key",
        )
        .expect("preview reference query");
    statement
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .expect("preview rows")
        .collect::<Result<_, _>>()
        .expect("preview records")
}

struct Fixture {
    source: TempDir,
    _derived: TempDir,
    child: PathBuf,
    storage: StoragePaths,
    original_child: Vec<u8>,
    original_parent: Vec<u8>,
}

impl Fixture {
    fn new() -> Self {
        let source = tempdir().expect("generated source");
        let derived = tempdir().expect("isolated catalog");
        let child = source.path().join("旅行");
        fs::create_dir(&child).expect("child directory");
        write_image(&child.join("child.png"));
        write_image(&source.path().join("parent.png"));
        let original_child = fs::read(child.join("child.png")).expect("original child");
        let original_parent = fs::read(source.path().join("parent.png")).expect("original parent");
        let storage = StoragePaths {
            catalog_path: derived.path().join("catalog.sqlite3"),
            preview_root: derived.path().join("previews"),
            preview_budget_bytes: 1024 * 1024,
            settings_path: derived.path().join("settings.sqlite3"),
        };
        run_scan_with_storage(request("initial-child", &child), |_| true, storage.clone())
            .expect("initial child import");
        Self {
            source,
            _derived: derived,
            child,
            storage,
            original_child,
            original_parent,
        }
    }

    fn parent_request(&self, scan_id: &str) -> ScanRequest {
        request(scan_id, self.source.path())
    }

    fn snapshot(&self) -> CatalogSnapshot {
        SqliteCatalog::open(self.storage.catalog_path.clone())
            .expect("FULL catalog reopen")
            .load_snapshot(
                100,
                &GalleryQuery::default(),
                "consolidation",
                None,
                None,
                None,
            )
            .expect("published gallery")
    }

    fn assert_child_publication(&self, asset_id: &str) {
        let snapshot = self.snapshot();
        assert_eq!(snapshot.assets.len(), 1);
        assert_eq!(snapshot.assets[0].asset_id, asset_id);
        assert!(
            snapshot
                .roots
                .iter()
                .any(|root| root.active_scan_id.as_deref() == Some("initial-child"))
        );
        assert_eq!(
            snapshot
                .roots
                .iter()
                .filter(|root| root.active_scan_id.is_some())
                .count(),
            1
        );
    }

    fn assert_sources(&self) {
        assert_eq!(
            fs::read(self.child.join("child.png")).expect("child remains readable"),
            self.original_child
        );
        assert_eq!(
            fs::read(self.source.path().join("parent.png")).expect("parent remains readable"),
            self.original_parent
        );
    }
}

fn request(scan_id: &str, root: &Path) -> ScanRequest {
    ScanRequest {
        scan_id: scan_id.to_owned(),
        root_path: root.to_string_lossy().into_owned(),
        max_items: None,
        max_entries: None,
        preview_edge: 128,
    }
}

fn write_image(path: &Path) {
    RgbImage::from_pixel(8, 6, Rgb([10, 90, 170]))
        .save(path)
        .expect("generated image");
}
