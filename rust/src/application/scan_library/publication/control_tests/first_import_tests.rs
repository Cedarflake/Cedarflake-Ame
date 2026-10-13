use super::super::super::{pause_scan, resume_scan_with_storage, suspend_scan};
use super::*;

#[test]
fn first_import_publication_cancel_pause_and_suspend_preserve_unpublished_policy() {
    for (command, expected_status, expected_staging) in [
        (cancel_scan as fn(&str) -> bool, "cancelled", 0_i64),
        (pause_scan, "paused", 1),
        (suspend_scan, "running", 1),
    ] {
        let fixture = Fixture::unpublished();
        let request = fixture.request(fixture.initial_scan.clone());
        let scan_id = request.scan_id.clone();
        let path = fixture.paths.catalog_path.clone();
        let called = Arc::new(AtomicBool::new(false));
        let observed = Arc::clone(&called);
        let _hook = set_before_scan_projection_replacement_hook(&request.scan_id, move |_| {
            let active: Option<String> = read_catalog(&path)
                .query_row("SELECT active_scan_id FROM library_roots", [], |row| {
                    row.get(0)
                })
                .expect("first import is not committed");
            assert!(active.is_none());
            assert!(
                command(&scan_id),
                "actual execution accepts control before publication admission"
            );
            observed.store(true, Ordering::Release);
        });
        let mut events = Vec::new();
        run_scan_with_storage(
            request.clone(),
            |event| {
                events.push(event);
                true
            },
            fixture.paths.clone(),
        )
        .expect("unpublished scan follows existing terminal policy");
        assert!(called.load(Ordering::Acquire));
        assert!(
            !events
                .iter()
                .any(|event| matches!(event, ScanEvent::Completed { .. }))
        );
        match expected_status {
            "cancelled" => assert!(matches!(events.last(), Some(ScanEvent::Cancelled { .. }))),
            "paused" => assert!(matches!(events.last(), Some(ScanEvent::Paused { .. }))),
            "running" => assert!(!events.iter().any(|event| matches!(
                event,
                ScanEvent::Cancelled { .. } | ScanEvent::Paused { .. }
            ))),
            _ => unreachable!(),
        }
        let reader = read_catalog(&fixture.paths.catalog_path);
        let state: (String, Option<String>, i64) = reader.query_row(
            "SELECT scans.status, roots.active_scan_id,
             (SELECT COUNT(*) FROM asset_locations WHERE scan_id = scans.id)
             FROM scan_runs scans JOIN library_roots roots ON roots.id = scans.root_id WHERE scans.id = ?1",
            [&request.scan_id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        ).expect("unpublished terminal state");
        assert_eq!(state, (expected_status.to_owned(), None, expected_staging));
        drop(reader);
        let snapshot = SqliteCatalog::open(fixture.paths.catalog_path.clone())
            .expect("full schema reopen")
            .load_snapshot(
                10,
                &GalleryQuery::default(),
                "first-import-control",
                None,
                None,
                None,
            )
            .expect("catalog remains readable");
        assert!(
            snapshot.assets.is_empty(),
            "unpublished staging is never presented as the completed catalog"
        );
        assert!(
            snapshot
                .roots
                .iter()
                .all(|root| root.active_scan_id.is_none())
        );
        assert_eq!(
            fs::read(fixture.source.path().join("baseline.png")).expect("original source"),
            fixture.baseline_bytes
        );
        if expected_status != "cancelled" {
            fixture.add_incoming();
            let mut resumed = Vec::new();
            resume_scan_with_storage(
                request.clone(),
                |event| {
                    resumed.push(event);
                    true
                },
                fixture.paths.clone(),
            )
            .expect("explicit continuation rebuilds the first-import inventory");
            assert!(matches!(
                resumed.last(),
                Some(ScanEvent::Completed { asset_count: 2, .. })
            ));
            fixture.assert_reopen(2, &request.scan_id);
            fixture.assert_source_bytes();
        }
    }
}

#[test]
fn replacement_publication_pause_and_suspend_keep_the_published_baseline() {
    for command in [pause_scan as fn(&str) -> bool, suspend_scan] {
        let fixture = Fixture::new();
        let request = fixture.replacement();
        let scan_id = request.scan_id.clone();
        let called = Arc::new(AtomicBool::new(false));
        let observed = Arc::clone(&called);
        let _hook = set_before_scan_projection_replacement_hook(&request.scan_id, move |_| {
            assert!(command(&scan_id));
            observed.store(true, Ordering::Release);
        });
        let mut events = Vec::new();
        run_scan_with_storage(
            request.clone(),
            |event| {
                events.push(event);
                true
            },
            fixture.paths.clone(),
        )
        .expect("replacement interruption settles");
        assert!(called.load(Ordering::Acquire));
        assert!(!events.iter().any(|event| matches!(
            event,
            ScanEvent::Completed { .. } | ScanEvent::Paused { .. }
        )));
        assert_eq!(
            publication_state(&fixture.paths.catalog_path, &request.scan_id),
            (fixture.initial_scan.clone(), "cancelled".to_owned(), 1)
        );
        fixture.assert_reopen(1, &fixture.initial_scan);
        fixture.assert_source_bytes();
    }
}
