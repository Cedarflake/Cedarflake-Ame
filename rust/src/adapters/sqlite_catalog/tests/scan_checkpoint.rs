use crate::adapters::sqlite_catalog::tests::support::fixture_request;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use tempfile::tempdir;

use crate::adapters::sqlite_catalog::*;

enum ProgressBoundary {
    Checkpoint,
    DirectoryCompletion,
}

impl ProgressBoundary {
    fn persist(&self, catalog: &mut SqliteCatalog) -> Result<(), ScanError> {
        self.persist_count(catalog, 1)
    }

    fn persist_count(&self, catalog: &mut SqliteCatalog, count: u64) -> Result<(), ScanError> {
        let checkpoint = ScanCheckpoint {
            last_visited_relative_path: Some("image.png".to_owned()),
            visited_entries: count,
            accepted_items: count,
            issue_count: 0,
            requires_previous_snapshot: false,
        };
        match self {
            Self::Checkpoint => catalog.checkpoint_scan("scan", &checkpoint),
            Self::DirectoryCompletion => catalog.complete_directory("scan", &checkpoint),
        }
    }
}

fn prepared_catalog(path: PathBuf) -> SqliteCatalog {
    let mut catalog = SqliteCatalog::open(path).expect("isolated catalog");
    catalog
        .begin_scan(
            &fixture_request("scan", "C:\\Pictures"),
            "root",
            "C:\\Pictures",
        )
        .expect("admit scan");
    assert_eq!(
        catalog
            .claim_next_directory("scan")
            .expect("claim directory"),
        Some(String::new())
    );
    catalog
        .stage_directory_entries("scan", "", &["image.png".to_owned()])
        .expect("persist directory roster");
    catalog
        .complete_directory_enumeration("scan", "")
        .expect("complete enumeration");
    catalog
}

fn location(width: u32) -> AssetLocationView {
    AssetLocationView {
        asset_id: "asset".to_owned(),
        location_id: "location".to_owned(),
        root_id: "root".to_owned(),
        scan_id: "scan".to_owned(),
        absolute_path: "C:\\Pictures\\image.png".to_owned(),
        display_path: "C:\\Pictures\\image.png".to_owned(),
        relative_path: "image.png".to_owned(),
        preview_path: String::new(),
        file_size: 100,
        created_unix_ms: None,
        modified_unix_ms: 1000,
        file_identity: None,
        source_revision: None,
        source_generation: 0,
        width,
        height: 10,
        preview_status: PreviewStatus::Pending,
        preview_issue_code: None,
        preview_issue_message: None,
        metadata_engine_id: "fixture".to_owned(),
        metadata_engine_version: "1".to_owned(),
        camera: Default::default(),
        capture_time: None,
    }
}

#[test]
fn scan_progress_and_buffered_locations_share_one_durable_commit() {
    for boundary in [
        ProgressBoundary::Checkpoint,
        ProgressBoundary::DirectoryCompletion,
    ] {
        let directory = tempdir().expect("isolated storage");
        let path = directory.path().join("catalog.sqlite3");
        let mut catalog = prepared_catalog(path.clone());
        let commits = Arc::new(AtomicUsize::new(0));
        let observed = Arc::clone(&commits);
        catalog
            .connection
            .commit_hook(Some(move || {
                observed.fetch_add(1, Ordering::SeqCst);
                false
            }))
            .expect("observe durable commits");
        catalog
            .stage_location("scan", "root", &location(10))
            .expect("buffer location");
        assert_eq!(commits.load(Ordering::SeqCst), 0);
        boundary.persist(&mut catalog).expect("persist progress");
        assert_eq!(
            commits.load(Ordering::SeqCst),
            1,
            "one progress boundary must not require two durable commits"
        );
        assert!(catalog.pending_locations.is_empty());
        drop(catalog);
        let reader = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
            .expect("closed checkpoint");
        let (accepted, staged): (i64, i64) = reader
            .query_row(
                "SELECT accepted_items, (SELECT COUNT(*) FROM asset_locations WHERE scan_id='scan')
             FROM scan_runs WHERE id='scan'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("durable progress and location count");
        assert_eq!((accepted, staged), (1, 1));
    }
}

fn numbered_location(index: usize) -> AssetLocationView {
    let mut item = location(10);
    item.asset_id = format!("asset-{index}");
    item.location_id = format!("location-{index}");
    item.relative_path = format!("image-{index}.png");
    item.absolute_path = format!("C:\\Pictures\\image-{index}.png");
    item.display_path = item.absolute_path.clone();
    item
}

#[test]
fn full_staging_batch_and_progress_share_one_durable_commit() {
    for boundary in [
        ProgressBoundary::Checkpoint,
        ProgressBoundary::DirectoryCompletion,
    ] {
        let directory = tempdir().expect("isolated storage");
        let path = directory.path().join("catalog.sqlite3");
        let mut catalog = prepared_catalog(path.clone());
        let commits = Arc::new(AtomicUsize::new(0));
        let observed = Arc::clone(&commits);
        catalog
            .connection
            .commit_hook(Some(move || {
                observed.fetch_add(1, Ordering::SeqCst);
                false
            }))
            .expect("observe commits");
        for index in 0..LOCATION_STAGE_BATCH {
            catalog
                .stage_location("scan", "root", &numbered_location(index))
                .expect("buffer location");
        }
        assert_eq!(commits.load(Ordering::SeqCst), 0);
        boundary
            .persist_count(&mut catalog, LOCATION_STAGE_BATCH as u64)
            .expect("persist full checkpoint");
        assert_eq!(commits.load(Ordering::SeqCst), 1);
        assert!(catalog.pending_locations.is_empty());
        drop(catalog);
        let reader = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
            .expect("reopen checkpoint");
        let (accepted, staged): (i64, i64) = reader
            .query_row(
                "SELECT accepted_items, (SELECT COUNT(*) FROM asset_locations WHERE scan_id='scan')
             FROM scan_runs WHERE id='scan'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("durable membership");
        assert_eq!(
            (accepted, staged),
            (LOCATION_STAGE_BATCH as i64, LOCATION_STAGE_BATCH as i64)
        );
    }
}

#[test]
fn staging_overflow_keeps_the_full_batch_for_retry_before_admitting_another_item() {
    let directory = tempdir().expect("isolated storage");
    let mut catalog = prepared_catalog(directory.path().join("catalog.sqlite3"));
    for index in 0..LOCATION_STAGE_BATCH {
        catalog
            .stage_location("scan", "root", &numbered_location(index))
            .expect("buffer location");
    }
    catalog
        .connection
        .commit_hook(Some(|| true))
        .expect("reject overflow commit");
    let next = numbered_location(LOCATION_STAGE_BATCH);
    assert!(catalog.stage_location("scan", "root", &next).is_err());
    assert_eq!(catalog.pending_locations.len(), LOCATION_STAGE_BATCH);
    assert!(
        catalog
            .pending_locations
            .iter()
            .all(|pending| pending.location.location_id != next.location_id)
    );
    let staged: i64 = catalog
        .connection
        .query_row("SELECT COUNT(*) FROM asset_locations", [], |row| row.get(0))
        .expect("rolled-back batch");
    assert_eq!(staged, 0);
    catalog
        .connection
        .commit_hook(None::<fn() -> bool>)
        .expect("allow retry");
    catalog
        .stage_location("scan", "root", &next)
        .expect("retry overflow");
    assert_eq!(catalog.pending_locations.len(), 1);
    let staged: i64 = catalog
        .connection
        .query_row("SELECT COUNT(*) FROM asset_locations", [], |row| row.get(0))
        .expect("committed batch");
    assert_eq!(staged, LOCATION_STAGE_BATCH as i64);
    catalog
        .checkpoint_scan(
            "scan",
            &ScanCheckpoint {
                accepted_items: (LOCATION_STAGE_BATCH + 1) as u64,
                ..ScanCheckpoint::default()
            },
        )
        .expect("persist remaining location");
    assert!(catalog.pending_locations.is_empty());
}

#[test]
fn rejected_scan_progress_keeps_buffered_work_uncommitted_for_retry() {
    for boundary in [
        ProgressBoundary::Checkpoint,
        ProgressBoundary::DirectoryCompletion,
    ] {
        let directory = tempdir().expect("isolated storage");
        let mut catalog = prepared_catalog(directory.path().join("catalog.sqlite3"));
        catalog.connection.execute_batch(
            "CREATE TEMP TRIGGER reject_progress BEFORE UPDATE OF accepted_items ON main.scan_runs
             WHEN NEW.id = 'scan' AND NEW.accepted_items = 1
             BEGIN SELECT RAISE(ABORT, 'injected checkpoint failure'); END;",
        ).expect("reject the progress write after location staging");
        catalog
            .stage_location("scan", "root", &location(10))
            .expect("buffer location");
        assert!(boundary.persist(&mut catalog).is_err());
        assert_eq!(
            catalog.pending_locations.len(),
            1,
            "a failed progress boundary must retain its buffered work"
        );
        let staged: i64 = catalog
            .connection
            .query_row(
                "SELECT COUNT(*) FROM asset_locations WHERE scan_id='scan'",
                [],
                |row| row.get(0),
            )
            .expect("uncommitted location count");
        assert_eq!(staged, 0);
        assert!(
            catalog
                .has_directory_entry("scan", "", "image.png")
                .expect("retained frontier")
        );
        catalog
            .connection
            .execute_batch("DROP TRIGGER temp.reject_progress")
            .expect("release failure");
        boundary
            .persist(&mut catalog)
            .expect("retry the same boundary");
        assert!(catalog.pending_locations.is_empty());
    }
}

#[test]
fn rejected_scan_progress_preserves_resume_membership_and_staged_payload() {
    for boundary in [
        ProgressBoundary::Checkpoint,
        ProgressBoundary::DirectoryCompletion,
    ] {
        let directory = tempdir().expect("isolated storage");
        let mut catalog = prepared_catalog(directory.path().join("catalog.sqlite3"));
        catalog
            .stage_location("scan", "root", &location(10))
            .expect("original retained inspection");
        catalog
            .flush_pending_locations()
            .expect("persist inspection");
        catalog.connection.execute(
            "INSERT INTO scan_resume_pending_locations(scan_id, location_id) VALUES ('scan', 'location')", [],
        ).expect("unverified resume membership");
        catalog.connection.execute_batch(
            "CREATE TEMP TRIGGER reject_progress BEFORE UPDATE OF accepted_items ON main.scan_runs
             WHEN NEW.id = 'scan' AND NEW.accepted_items = 1
             BEGIN SELECT RAISE(ABORT, 'injected checkpoint failure'); END;",
        ).expect("reject progress publication");
        catalog
            .stage_location("scan", "root", &location(20))
            .expect("new inspection");
        assert!(boundary.persist(&mut catalog).is_err());
        let (width, pending): (i64, i64) = catalog.connection.query_row(
            "SELECT width, (SELECT COUNT(*) FROM scan_resume_pending_locations WHERE scan_id='scan')
             FROM asset_locations WHERE scan_id='scan' AND location_id='location'", [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        ).expect("retained state after rollback");
        assert_eq!((width, pending), (10, 1));
        catalog
            .connection
            .execute_batch("DROP TRIGGER temp.reject_progress")
            .expect("release failure");
        boundary
            .persist(&mut catalog)
            .expect("retry verified inspection");
        let (width, pending): (i64, i64) = catalog.connection.query_row(
            "SELECT width, (SELECT COUNT(*) FROM scan_resume_pending_locations WHERE scan_id='scan')
             FROM asset_locations WHERE scan_id='scan' AND location_id='location'", [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        ).expect("committed inspection and membership");
        assert_eq!((width, pending), (20, 0));
    }
}

#[test]
fn refused_progress_commit_retains_the_batch_for_a_single_retry() {
    for boundary in [
        ProgressBoundary::Checkpoint,
        ProgressBoundary::DirectoryCompletion,
    ] {
        let directory = tempdir().expect("isolated storage");
        let mut catalog = prepared_catalog(directory.path().join("catalog.sqlite3"));
        let reject_first = Arc::new(AtomicBool::new(true));
        let observed = Arc::clone(&reject_first);
        catalog
            .connection
            .commit_hook(Some(move || observed.swap(false, Ordering::SeqCst)))
            .expect("reject one durable commit");
        catalog
            .stage_location("scan", "root", &location(10))
            .expect("buffer location");
        assert!(boundary.persist(&mut catalog).is_err());
        assert!(!reject_first.load(Ordering::SeqCst));
        assert_eq!(catalog.pending_locations.len(), 1);
        let (accepted, staged): (i64, i64) = catalog
            .connection
            .query_row(
                "SELECT accepted_items, (SELECT COUNT(*) FROM asset_locations WHERE scan_id='scan')
             FROM scan_runs WHERE id='scan'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("rolled-back commit");
        assert_eq!((accepted, staged), (0, 0));
        boundary
            .persist(&mut catalog)
            .expect("retry durable progress");
        assert!(catalog.pending_locations.is_empty());
        let (accepted, staged): (i64, i64) = catalog
            .connection
            .query_row(
                "SELECT accepted_items, (SELECT COUNT(*) FROM asset_locations WHERE scan_id='scan')
             FROM scan_runs WHERE id='scan'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("committed retry");
        assert_eq!((accepted, staged), (1, 1));
    }
}

#[test]
fn directory_entry_frontier_is_idempotent_sorted_and_windowed() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let request = fixture_request("wide-scan", "C:\\Pictures");
    let mut catalog = SqliteCatalog::open(path).expect("catalog");
    catalog
        .begin_scan(&request, "wide-root", "C:\\Pictures")
        .expect("begin scan");
    assert_eq!(
        catalog
            .claim_next_directory("wide-scan")
            .expect("claim root"),
        Some(String::new())
    );
    assert!(
        !catalog
            .is_current_directory_enumerated("wide-scan", "")
            .expect("enumeration state")
    );

    let mut entries = (0..1025)
        .rev()
        .map(|index| format!("image-{index:04}.png"))
        .collect::<Vec<_>>();
    catalog
        .stage_directory_entries("wide-scan", "", &entries[..17])
        .expect("partial enumeration");
    entries.push("image-0000.png".to_owned());
    for batch in entries.chunks(113) {
        catalog
            .stage_directory_entries("wide-scan", "", batch)
            .expect("entry batch");
    }
    catalog
        .complete_directory_enumeration("wide-scan", "")
        .expect("complete enumeration");

    let mut loaded = Vec::new();
    loop {
        let window = catalog
            .load_directory_entry_window("wide-scan", "", loaded.last().map(String::as_str), 128)
            .expect("entry window");
        assert!(window.len() <= 128);
        if window.is_empty() {
            break;
        }
        loaded.extend(window);
    }

    assert_eq!(loaded.len(), 1025);
    assert!(loaded.windows(2).all(|pair| pair[0] < pair[1]));
    assert!(
        catalog
            .has_directory_entry("wide-scan", "", "image-0512.png")
            .expect("entry identity")
    );
    catalog
        .complete_directory("wide-scan", &ScanCheckpoint::default())
        .expect("complete directory");
    let queued: i64 = catalog
        .connection
        .query_row("SELECT COUNT(*) FROM scan_directory_entries", [], |row| {
            row.get(0)
        })
        .expect("entry count");
    assert_eq!(queued, 0);
}

#[test]
fn persists_and_validates_a_published_root_replacement_checkpoint() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let raw_root_path = r"\\?\C:\Pictures";
    let request = fixture_request("scan-resume", raw_root_path);
    let mut catalog = SqliteCatalog::open(path.clone()).expect("catalog");
    let published = fixture_request("published-before-resume", raw_root_path);
    catalog
        .begin_scan(&published, "root-resume", raw_root_path)
        .expect("initial import");
    catalog
        .prove_live_only_first_import_handoff_for_test(&published.scan_id)
        .expect("initial capture");
    catalog
        .publish_scan(&published.scan_id, "root-resume", 0, 0)
        .expect("trustworthy prior baseline");
    let initial = catalog
        .begin_scan(&request, "root-resume", raw_root_path)
        .expect("begin scan");
    assert_eq!(initial.visited_entries, 0);
    let checkpoint = ScanCheckpoint {
        last_visited_relative_path: Some("nested/last.png".to_owned()),
        visited_entries: 128,
        accepted_items: 40,
        issue_count: 3,
        requires_previous_snapshot: true,
    };
    catalog
        .checkpoint_scan("scan-resume", &checkpoint)
        .expect("persist checkpoint");
    drop(catalog);

    let mut restored = SqliteCatalog::open(path).expect("restored catalog");
    let recoverable = restored
        .load_recoverable_scan()
        .expect("recoverable scan")
        .expect("stored running scan");
    assert_eq!(recoverable.scan_id, "scan-resume");
    assert_eq!(recoverable.root_path, raw_root_path);
    assert_eq!(recoverable.display_root_path, "C:\\Pictures");
    assert_eq!(recoverable.visited_entries, 128);
    assert_eq!(recoverable.accepted_items, 40);
    assert_eq!(recoverable.issue_count, 3);
    let create_error = restored
        .begin_scan(&request, "unexpected-root", "C:\\Unexpected")
        .expect_err("new scans cannot reuse a checkpoint identifier");
    assert_eq!(create_error.code, "catalog_scan_already_exists");
    let unexpected_root_count = restored
        .connection
        .query_row(
            "SELECT COUNT(*) FROM library_roots WHERE id = 'unexpected-root'",
            [],
            |row| row.get::<_, i64>(0),
        )
        .expect("unexpected root count");
    assert_eq!(unexpected_root_count, 0);
    let resumed = restored
        .resume_scan(&request, "root-resume", raw_root_path)
        .expect("resume scan");
    assert_eq!(
        resumed.last_visited_relative_path,
        checkpoint.last_visited_relative_path
    );
    assert!(resumed.requires_previous_snapshot);
    let publish_error = restored
        .publish_scan("scan-resume", "root-resume", 40, 3)
        .expect_err("an unresolved prior issue cannot publish");
    assert_eq!(
        publish_error.code,
        "catalog_scan_requires_previous_snapshot"
    );

    let mut changed_request = request.clone();
    changed_request.preview_edge = 256;
    let error = restored
        .resume_scan(&changed_request, "root-resume", "C:\\Pictures")
        .expect_err("changed parameters cannot reuse a checkpoint");
    assert_eq!(error.code, "catalog_scan_resume_mismatch");
}
