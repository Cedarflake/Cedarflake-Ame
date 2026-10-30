use crate::ports::CatalogRepository;

use super::*;

#[test]
fn abandoned_claims_exhaust_the_retry_budget_across_process_sessions() {
    let (directory, mut catalog) = fixture();
    let (_, mut session) = start(&mut catalog);
    request(
        &mut catalog,
        &session,
        &["a"],
        ClassificationWorkPriority::Requested,
    );
    for attempt in 1..=3 {
        let claim = catalog
            .claim_classification_work(&session, 10)
            .unwrap()
            .unwrap();
        drop(catalog);
        catalog = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
        session = id(&format!("session-{attempt}"));
        let revision = catalog.classification_dispatch_state().unwrap().revision;
        catalog
            .open_classification_session(revision, &session)
            .unwrap();
        assert!(!catalog.yield_classification_work(&claim).unwrap());
    }
    assert!(
        catalog
            .claim_classification_work(&session, 10)
            .unwrap()
            .is_none()
    );
    assert_eq!(
        scalar(
            &catalog,
            "SELECT failure_count FROM classification_work_items"
        ),
        3
    );
    assert_eq!(catalog.retry_classification_work(&session, 20).unwrap(), 1);
    assert!(
        catalog
            .claim_classification_work(&session, 20)
            .unwrap()
            .is_some()
    );
}

#[test]
fn removed_source_rejects_late_result_and_surviving_copy_can_resume() {
    let (_directory, mut catalog) = fixture();
    let (run, session) = start(&mut catalog);
    verify(&mut catalog, "a", 'a');
    verify(&mut catalog, "c", 'a');
    request(
        &mut catalog,
        &session,
        &["a"],
        ClassificationWorkPriority::Requested,
    );
    let source = catalog
        .claim_classification_work(&session, 10)
        .unwrap()
        .unwrap();
    catalog.finish_classification_verification(&source).unwrap();
    let original = catalog
        .claim_classification_work(&session, 10)
        .unwrap()
        .unwrap();
    let prediction = evidence(&run, &original);
    catalog.unregister_root("root-a").unwrap();
    assert!(
        !catalog
            .publish_classification_work(&original, &prediction)
            .unwrap()
    );
    assert_eq!(
        scalar(&catalog, "SELECT count(*) FROM classification_predictions"),
        0
    );
    let surviving = catalog
        .claim_classification_work(&session, 20)
        .unwrap()
        .unwrap();
    assert_eq!(surviving.work_id, original.work_id);
    assert!(matches!(&surviving.target,
        ClassificationWorkTarget::AnalyzeContent { source, .. } if source.location_id == "c"));
    assert!(
        catalog
            .publish_classification_work(&surviving, &prediction)
            .unwrap()
    );
}

#[test]
fn disabled_analysis_rejects_publication_and_retains_completed_source_work() {
    let (_directory, mut catalog) = fixture();
    let (run, session) = start(&mut catalog);
    verify(&mut catalog, "a", 'a');
    request(
        &mut catalog,
        &session,
        &["a"],
        ClassificationWorkPriority::Requested,
    );
    let source = catalog
        .claim_classification_work(&session, 10)
        .unwrap()
        .unwrap();
    catalog.finish_classification_verification(&source).unwrap();
    let old = catalog
        .claim_classification_work(&session, 10)
        .unwrap()
        .unwrap();
    let prediction = evidence(&run, &old);
    change(&mut catalog, ClassificationDispatchCommand::Disable);
    assert!(
        !catalog
            .publish_classification_work(&old, &prediction)
            .unwrap()
    );
    assert!(
        catalog
            .claim_classification_work(&session, 10)
            .unwrap()
            .is_none()
    );
    change(
        &mut catalog,
        ClassificationDispatchCommand::Enable {
            run_id: run.id().clone(),
        },
    );
    let resumed = catalog
        .claim_classification_work(&session, 20)
        .unwrap()
        .unwrap();
    assert_eq!(resumed.work_id, old.work_id);
    assert!(
        catalog
            .publish_classification_work(&resumed, &prediction)
            .unwrap()
    );
    assert_eq!(
        scalar(
            &catalog,
            "SELECT count(*) FROM classification_work_items WHERE kind = 'source'"
        ),
        1
    );
}

#[test]
fn unavailable_source_waits_without_using_the_failure_budget() {
    let (_directory, mut catalog) = fixture();
    let (_, session) = start(&mut catalog);
    request(
        &mut catalog,
        &session,
        &["a"],
        ClassificationWorkPriority::NewContent,
    );
    let lease = catalog
        .claim_classification_work(&session, 10)
        .unwrap()
        .unwrap();
    catalog
        .fail_classification_work(
            &lease,
            &ClassificationWorkIssue {
                kind: ClassificationWorkFailure::Unavailable,
                code: "cloud_placeholder".into(),
                retry_unix_ms: 0,
            },
        )
        .unwrap();
    request(
        &mut catalog,
        &session,
        &["a"],
        ClassificationWorkPriority::Requested,
    );
    assert!(
        catalog
            .claim_classification_work(&session, 1_000_000)
            .unwrap()
            .is_none()
    );
    assert_eq!(
        scalar(
            &catalog,
            "SELECT failure_count FROM classification_work_items"
        ),
        0
    );
    assert_eq!(catalog.retry_classification_work(&session, 20).unwrap(), 1);
    assert!(
        catalog
            .claim_classification_work(&session, 20)
            .unwrap()
            .is_some()
    );
}

#[test]
fn backlog_walk_is_bounded_idempotent_and_allows_only_one_active_claim() {
    let (_directory, mut catalog) = fixture();
    let names = (0..130)
        .map(|index| format!("backlog-{index:03}"))
        .collect::<Vec<_>>();
    let images = names
        .iter()
        .map(|name| (name.as_str(), None, 1))
        .collect::<Vec<_>>();
    crate::adapters::sqlite_catalog::tests::publish_gallery_fixture(
        &mut catalog,
        "backlog-scan",
        "backlog-root",
        "E:\\Fixture",
        &images,
    );
    let (_, session) = start(&mut catalog);
    for (count, has_more) in [(64, true), (64, true), (5, false), (0, false)] {
        let page = catalog.seed_classification_backlog(&session, 10).unwrap();
        assert_eq!(page.visited_sources, count);
        assert_eq!(page.queued_sources, count);
        assert_eq!(page.has_more, has_more);
    }
    assert_eq!(
        scalar(&catalog, "SELECT count(*) FROM classification_work_items"),
        133
    );
    let first = catalog
        .claim_classification_work(&session, 10)
        .unwrap()
        .unwrap();
    assert!(
        catalog
            .claim_classification_work(&session, 10)
            .unwrap()
            .is_none()
    );
    assert!(catalog.yield_classification_work(&first).unwrap());
    assert!(
        catalog
            .claim_classification_work(&session, 10)
            .unwrap()
            .is_some()
    );
}
