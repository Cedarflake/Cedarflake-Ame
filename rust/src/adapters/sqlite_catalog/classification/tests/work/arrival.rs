use crate::adapters::sqlite_catalog::tests::publish_gallery_fixture;

use super::*;

#[test]
fn arrivals_enter_before_a_large_backlog_walk_finishes() {
    let (_directory, mut catalog) = fixture();
    let names = (0..130)
        .map(|index| format!("old-{index:03}"))
        .collect::<Vec<_>>();
    let images = names
        .iter()
        .map(|name| (name.as_str(), None, 1))
        .collect::<Vec<_>>();
    publish_gallery_fixture(
        &mut catalog,
        "old-scan",
        "old-root",
        "E:\\OldFixture",
        &images,
    );
    let (_, session) = start(&mut catalog);
    publish_gallery_fixture(
        &mut catalog,
        "new-scan",
        "new-root",
        "F:\\NewFixture",
        &[("arrival", None, 1)],
    );
    let page = catalog.seed_classification_backlog(&session, 10).unwrap();
    assert_eq!(page.visited_sources, 64);
    assert!(page.has_more);
    let lease = catalog
        .claim_classification_work(&session, 10)
        .unwrap()
        .unwrap();
    assert_eq!(lease.priority, ClassificationWorkPriority::NewContent);
    assert!(
        matches!(lease.target, ClassificationWorkTarget::VerifySource(source) if source.location_id == "arrival")
    );
    assert_eq!(
        scalar(
            &catalog,
            "SELECT count(*) FROM classification_work_items WHERE priority = 2"
        ),
        63
    );
}

#[test]
fn new_arrivals_keep_priority_across_pause_and_restart_without_starving_backlog() {
    let (directory, mut catalog) = fixture();
    let (_, session) = start(&mut catalog);
    catalog.seed_classification_backlog(&session, 10).unwrap();
    change(&mut catalog, ClassificationDispatchCommand::Pause);
    publish_gallery_fixture(
        &mut catalog,
        "arrival-scan",
        "arrival-root",
        "E:\\Fixture",
        &[
            ("new-one", None, 1),
            ("new-two", None, 2),
            ("new-three", None, 3),
        ],
    );
    drop(catalog);
    let mut catalog = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
    let session = id("restarted");
    let revision = catalog.classification_dispatch_state().unwrap().revision;
    catalog
        .open_classification_session(revision, &session)
        .unwrap();
    assert_eq!(
        catalog.classification_dispatch_state().unwrap().mode,
        ClassificationDispatchMode::Paused
    );
    change(&mut catalog, ClassificationDispatchCommand::Continue);
    let page = catalog.seed_classification_backlog(&session, 20).unwrap();
    assert_eq!(page.queued_sources, 3);
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
            "SELECT count(*) FROM classification_work_items WHERE priority = 1"
        ),
        3
    );
    let mut priorities = Vec::new();
    for _ in 0..6 {
        let lease = catalog
            .claim_classification_work(&session, 20)
            .unwrap()
            .unwrap();
        priorities.push(lease.priority);
        catalog
            .fail_classification_work(
                &lease,
                &ClassificationWorkIssue {
                    kind: ClassificationWorkFailure::Unavailable,
                    code: "fixture_unavailable".into(),
                    retry_unix_ms: 20,
                },
            )
            .unwrap();
    }
    assert_eq!(priorities[0], ClassificationWorkPriority::NewContent);
    assert!(priorities[..3].contains(&ClassificationWorkPriority::Backlog));
    assert_eq!(
        catalog
            .seed_classification_backlog(&session, 30)
            .unwrap()
            .queued_sources,
        0
    );
}

#[test]
fn replacement_run_captures_a_new_backlog_boundary() {
    let (_directory, mut catalog) = fixture();
    let (first, session) = start(&mut catalog);
    publish_gallery_fixture(
        &mut catalog,
        "arrival-scan",
        "arrival-root",
        "E:\\Fixture",
        &[("new", None, 1)],
    );
    let second = ClassificationRun::new(
        id("replacement"),
        first.model(),
        first.primary_rules(),
        None,
    )
    .unwrap();
    catalog.begin_classification_run(&second, None).unwrap();
    change(
        &mut catalog,
        ClassificationDispatchCommand::ReplaceRun {
            run_id: second.id().clone(),
        },
    );
    let page = catalog.seed_classification_backlog(&session, 20).unwrap();
    assert_eq!(page.queued_sources, 4);
    assert_eq!(
        scalar(
            &catalog,
            "SELECT count(*) FROM classification_work_items WHERE priority = 2"
        ),
        4
    );
}
