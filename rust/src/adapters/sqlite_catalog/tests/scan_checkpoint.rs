use super::*;

enum ProgressBoundary {
    Checkpoint,
    DirectoryCompletion,
}

impl ProgressBoundary {
    fn persist(&self, catalog: &mut SqliteCatalog) -> Result<(), ScanError> {
        let checkpoint = ScanCheckpoint {
            last_visited_relative_path: Some("image.png".to_owned()),
            visited_entries: 1,
            accepted_items: 1,
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
