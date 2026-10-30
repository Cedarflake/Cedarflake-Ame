use crate::adapters::sqlite_catalog::tests::publish_gallery_fixture;
use crate::domain::ScanRequest;
use crate::ports::CatalogRepository;

use super::*;

fn stage(catalog: &mut SqliteCatalog, scan: &str, root: &str, count: usize) {
    let template = catalog.load_active_location("a").unwrap().unwrap();
    let path = format!("E:\\PublicationFixture\\{root}");
    catalog
        .begin_scan(
            &ScanRequest {
                scan_id: scan.into(),
                root_path: path.clone(),
                max_items: None,
                max_entries: None,
                preview_edge: 128,
            },
            root,
            &path,
        )
        .unwrap();
    catalog
        .prove_live_only_first_import_handoff_for_test(scan)
        .unwrap();
    for index in 0..count {
        let name = format!("{root}-{index:03}");
        let mut location = template.clone();
        location.asset_id = format!("asset-{name}");
        location.location_id = name.clone();
        location.root_id = root.into();
        location.scan_id = scan.into();
        location.relative_path = format!("{name}.png");
        location.absolute_path = format!("{path}\\{name}.png");
        location.display_path.clone_from(&location.absolute_path);
        location.source_generation = 0;
        catalog.stage_location(scan, root, &location).unwrap();
    }
    catalog.flush_pending_locations().unwrap();
}

fn drain(catalog: &mut SqliteCatalog, session: &ClassificationRecordId) -> u32 {
    let mut queued = 0;
    for _ in 0..20 {
        let page = catalog.seed_classification_backlog(session, 20).unwrap();
        assert!(page.visited_sources <= 64);
        queued += page.queued_sources;
        if !page.has_more {
            return queued;
        }
    }
    panic!("fixture admission did not finish");
}

#[test]
fn first_import_published_behind_the_backlog_cursor_is_still_classified() {
    let (_directory, mut catalog) = fixture();
    stage(&mut catalog, "delayed-scan", "delayed", 2);
    let (_, session) = start(&mut catalog);
    assert_eq!(drain(&mut catalog, &session), 3);
    assert_eq!(
        scalar(
            &catalog,
            "SELECT count(*) FROM classification_source_publications"
        ),
        0
    );
    catalog
        .publish_scan("delayed-scan", "delayed", 2, 0)
        .unwrap();
    assert_eq!(drain(&mut catalog, &session), 2);
    assert_eq!(
        scalar(
            &catalog,
            "SELECT count(*) FROM classification_work_items WHERE priority = 1"
        ),
        2
    );
    assert_eq!(
        scalar(
            &catalog,
            "SELECT count(*) FROM classification_work_items WHERE priority = 2"
        ),
        3
    );
    assert_eq!(drain(&mut catalog, &session), 0);
}

#[test]
fn late_publication_is_bounded_and_resumes_after_pause_and_restart() {
    let (directory, mut catalog) = fixture();
    let (_, session) = start(&mut catalog);
    drain(&mut catalog, &session);
    stage(&mut catalog, "delayed-scan", "delayed", 70);
    publish_gallery_fixture(
        &mut catalog,
        "ahead-scan",
        "ahead",
        "F:\\AheadFixture",
        &[("ahead", None, 1)],
    );
    assert_eq!(drain(&mut catalog, &session), 1);
    catalog
        .publish_scan("delayed-scan", "delayed", 70, 0)
        .unwrap();
    let page = catalog.seed_classification_backlog(&session, 20).unwrap();
    assert_eq!(page.visited_sources, 32);
    assert_eq!(page.queued_sources, 32);
    assert!(page.has_more);
    let paused = change(&mut catalog, ClassificationDispatchCommand::Pause);
    drop(catalog);
    let mut catalog = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
    let session = id("restart-publication");
    catalog
        .open_classification_session(paused.revision, &session)
        .unwrap();
    assert_eq!(
        catalog.classification_dispatch_state().unwrap().mode,
        ClassificationDispatchMode::Paused
    );
    assert!(
        catalog
            .claim_classification_work(&session, 30)
            .unwrap()
            .is_none()
    );
    change(&mut catalog, ClassificationDispatchCommand::Continue);
    assert_eq!(drain(&mut catalog, &session), 38);
    assert_eq!(
        scalar(
            &catalog,
            "SELECT count(*) FROM classification_work_items WHERE kind = 'source'"
        ),
        74
    );
    assert_eq!(
        scalar(
            &catalog,
            "SELECT count(*) FROM classification_source_publications"
        ),
        0
    );
}

#[test]
fn publication_receipt_and_gallery_snapshot_roll_back_together() {
    let (_directory, mut catalog) = fixture();
    stage(&mut catalog, "delayed-scan", "delayed", 1);
    let (_, session) = start(&mut catalog);
    drain(&mut catalog, &session);
    catalog.connection.execute_batch(
        "CREATE TRIGGER reject_classification_publication BEFORE INSERT ON classification_source_publications
         BEGIN SELECT RAISE(ABORT, 'injected'); END;",
    ).unwrap();
    assert!(
        catalog
            .publish_scan("delayed-scan", "delayed", 1, 0)
            .is_err()
    );
    assert_eq!(
        scalar(
            &catalog,
            "SELECT active_scan_id IS NULL FROM library_roots WHERE id = 'delayed'"
        ),
        1
    );
    assert_eq!(drain(&mut catalog, &session), 0);
    catalog
        .connection
        .execute_batch("DROP TRIGGER reject_classification_publication")
        .unwrap();
    catalog
        .publish_scan("delayed-scan", "delayed", 1, 0)
        .unwrap();
    assert_eq!(drain(&mut catalog, &session), 1);
}

#[test]
fn removing_an_unconsumed_source_discards_its_publication_without_restarting_stopped_work() {
    let (_directory, mut catalog) = fixture();
    let (_, session) = start(&mut catalog);
    drain(&mut catalog, &session);
    change(&mut catalog, ClassificationDispatchCommand::Stop);
    publish_gallery_fixture(
        &mut catalog,
        "later-scan",
        "later",
        "F:\\LaterFixture",
        &[("later", None, 1)],
    );
    assert_eq!(
        scalar(
            &catalog,
            "SELECT count(*) FROM classification_source_publications"
        ),
        1
    );
    assert!(
        catalog
            .claim_classification_work(&session, 30)
            .unwrap()
            .is_none()
    );
    catalog.unregister_root("later").unwrap();
    assert_eq!(
        scalar(
            &catalog,
            "SELECT count(*) FROM classification_source_publications"
        ),
        0
    );
    assert_eq!(
        catalog.classification_dispatch_state().unwrap().mode,
        ClassificationDispatchMode::Stopped
    );
}

#[test]
fn repeated_snapshot_keeps_existing_backlog_priority_and_failure_budget() {
    let (_directory, mut catalog) = fixture();
    let (_, session) = start(&mut catalog);
    drain(&mut catalog, &session);
    super::super::super::work::root_published(&catalog.connection, "root-a").unwrap();
    assert_eq!(drain(&mut catalog, &session), 0);
    assert_eq!(
        scalar(
            &catalog,
            "SELECT count(*) FROM classification_work_items WHERE priority = 2"
        ),
        3
    );
    assert_eq!(
        scalar(
            &catalog,
            "SELECT sum(failure_count) FROM classification_work_items"
        ),
        0
    );
}

#[test]
fn admission_cursor_and_queue_entries_roll_back_together() {
    let (_directory, mut catalog) = fixture();
    stage(&mut catalog, "delayed-scan", "delayed", 40);
    let (_, session) = start(&mut catalog);
    drain(&mut catalog, &session);
    catalog
        .publish_scan("delayed-scan", "delayed", 40, 0)
        .unwrap();
    catalog.connection.execute_batch(
        "CREATE TRIGGER reject_publication_cursor BEFORE UPDATE ON classification_source_publications
         BEGIN SELECT RAISE(ABORT, 'injected'); END;",
    ).unwrap();
    assert!(catalog.seed_classification_backlog(&session, 20).is_err());
    assert_eq!(
        scalar(&catalog, "SELECT count(*) FROM classification_work_items"),
        3
    );
    assert_eq!(
        scalar(
            &catalog,
            "SELECT after_source_generation FROM classification_source_publications"
        ),
        0
    );
    catalog
        .connection
        .execute_batch("DROP TRIGGER reject_publication_cursor")
        .unwrap();
    assert_eq!(drain(&mut catalog, &session), 40);
}
