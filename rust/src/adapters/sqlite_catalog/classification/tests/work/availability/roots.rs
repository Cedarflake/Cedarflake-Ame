use crate::adapters::sqlite_catalog::tests::publish_gallery_fixture;

use super::*;

#[test]
fn recovering_one_root_after_restart_preserves_pause_and_other_offline_copies() {
    let (directory, mut catalog) = fixture();
    let (_, session) = start(&mut catalog);
    verify(&mut catalog, "c", 'a');
    let first = content_task(&mut catalog, &session);
    unavailable(&mut catalog, &first);
    let peer = claim(&mut catalog, &session);
    unavailable(&mut catalog, &peer);
    change(&mut catalog, ClassificationDispatchCommand::Pause);
    drop(catalog);
    let mut catalog = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
    let revision = catalog.classification_dispatch_state().unwrap().revision;
    let session = id("recovered-session");
    catalog
        .open_classification_session(revision, &session)
        .unwrap();
    let paused = catalog.classification_dispatch_state().unwrap();
    let stale_root = ClassificationRecoveryRoot {
        id: "root-c".into(),
        generation: LibraryRootGeneration::new(2).unwrap(),
    };
    assert!(
        catalog
            .recover_classification_root(&session, &stale_root, None, 10)
            .is_err()
    );
    let page = catalog
        .recover_classification_root(&session, &root("root-c"), None, 10)
        .unwrap();
    assert_eq!((page.released_sources, page.resumed_tasks), (1, 1));
    assert!(page.next.is_none());
    assert_eq!(catalog.classification_dispatch_state().unwrap(), paused);
    assert!(
        catalog
            .claim_classification_work(&session, 10)
            .unwrap()
            .is_none()
    );
    assert_eq!(
        scalar(
            &catalog,
            "SELECT count(*) FROM classification_unavailable_sources WHERE root_id = 'root-a'"
        ),
        1
    );
    change(&mut catalog, ClassificationDispatchCommand::Continue);
    let resumed = claim(&mut catalog, &session);
    assert_eq!(resumed.work_id, first.work_id);
    assert!(
        matches!(&resumed.target, ClassificationWorkTarget::AnalyzeContent { source, .. } if source.location_id == "c")
    );
    assert!(!catalog.yield_classification_work(&peer).unwrap());
}

#[test]
fn a_recovery_page_never_clears_failures_observed_after_its_boundary() {
    let (_directory, mut catalog) = fixture();
    let names = (0..65)
        .map(|index| format!("waiting-{index:02}"))
        .collect::<Vec<_>>();
    let images = names
        .iter()
        .map(|name| (name.as_str(), None, 1))
        .collect::<Vec<_>>();
    publish_gallery_fixture(
        &mut catalog,
        "many-scan",
        "many-root",
        "E:\\Fixture",
        &images,
    );
    let (_, session) = start(&mut catalog);
    let names = names.iter().map(String::as_str).collect::<Vec<_>>();
    request(
        &mut catalog,
        &session,
        &names,
        ClassificationWorkPriority::Requested,
    );
    for _ in 0..65 {
        let work = claim(&mut catalog, &session);
        unavailable(&mut catalog, &work);
    }
    let scope = root("many-root");
    let first = catalog
        .recover_classification_root(&session, &scope, None, 10)
        .unwrap();
    assert_eq!((first.released_sources, first.resumed_tasks), (64, 64));
    let cursor = first.next.unwrap();
    assert!(
        catalog
            .recover_classification_root(&session, &root("root-a"), Some(&cursor), 10)
            .is_err()
    );
    let retried = claim(&mut catalog, &session);
    unavailable(&mut catalog, &retried);
    let last = catalog
        .recover_classification_root(&session, &scope, Some(&cursor), 10)
        .unwrap();
    assert_eq!((last.released_sources, last.resumed_tasks), (1, 1));
    assert!(last.next.is_none());
    assert_eq!(
        scalar(
            &catalog,
            "SELECT count(*) FROM classification_unavailable_sources"
        ),
        1
    );
    assert_eq!(
        scalar(
            &catalog,
            "SELECT count(*) FROM classification_work_items WHERE phase = 'waiting'"
        ),
        1
    );
    assert_eq!(
        scalar(
            &catalog,
            "SELECT count(*) FROM classification_work_items WHERE phase = 'queued'"
        ),
        64
    );
}

#[test]
fn availability_does_not_restart_stopped_disabled_or_permanently_failed_work() {
    for mode in [
        ClassificationDispatchCommand::Stop,
        ClassificationDispatchCommand::Disable,
    ] {
        let (_directory, mut catalog) = fixture();
        let (_, session) = start(&mut catalog);
        request(
            &mut catalog,
            &session,
            &["a"],
            ClassificationWorkPriority::Requested,
        );
        let work = claim(&mut catalog, &session);
        unavailable(&mut catalog, &work);
        let stopped = change(&mut catalog, mode);
        let selected_a = selected(&catalog, "a");
        assert_eq!(
            catalog
                .refresh_classification_sources(&session, &[selected_a], 10)
                .unwrap(),
            0
        );
        let page = catalog
            .recover_classification_root(&session, &root("root-a"), None, 10)
            .unwrap();
        assert_eq!((page.released_sources, page.resumed_tasks), (0, 0));
        assert_eq!(catalog.classification_dispatch_state().unwrap(), stopped);
        assert_eq!(
            scalar(
                &catalog,
                "SELECT count(*) FROM classification_unavailable_sources"
            ),
            1
        );
    }
    let (_directory, mut catalog) = fixture();
    let (_, session) = start(&mut catalog);
    verify(&mut catalog, "c", 'a');
    let first = content_task(&mut catalog, &session);
    unavailable(&mut catalog, &first);
    let peer = claim(&mut catalog, &session);
    catalog
        .fail_classification_work(
            &peer,
            &ClassificationWorkIssue {
                kind: ClassificationWorkFailure::Permanent,
                code: "corrupt_media".into(),
                retry_unix_ms: 0,
            },
        )
        .unwrap();
    let page = catalog
        .recover_classification_root(&session, &root("root-a"), None, 10)
        .unwrap();
    assert_eq!((page.released_sources, page.resumed_tasks), (1, 0));
    assert!(
        catalog
            .claim_classification_work(&session, 100_000)
            .unwrap()
            .is_none()
    );
    assert_eq!(
        scalar(
            &catalog,
            "SELECT failure_count FROM classification_work_items WHERE kind = 'content'"
        ),
        1
    );
}

#[test]
fn failed_recovery_transaction_keeps_the_original_waiting_evidence() {
    let (_directory, mut catalog) = fixture();
    let (_, session) = start(&mut catalog);
    request(
        &mut catalog,
        &session,
        &["a"],
        ClassificationWorkPriority::Requested,
    );
    let work = claim(&mut catalog, &session);
    unavailable(&mut catalog, &work);
    catalog
        .connection
        .execute_batch(
            "CREATE TRIGGER reject_root_resume BEFORE UPDATE OF phase ON classification_work_items
         WHEN NEW.phase = 'queued' BEGIN SELECT RAISE(ABORT, 'injected'); END;",
        )
        .unwrap();
    assert!(
        catalog
            .recover_classification_root(&session, &root("root-a"), None, 10)
            .is_err()
    );
    assert_eq!(
        scalar(
            &catalog,
            "SELECT count(*) FROM classification_unavailable_sources"
        ),
        1
    );
    assert_eq!(
        scalar(
            &catalog,
            "SELECT count(*) FROM classification_work_items WHERE phase = 'waiting'"
        ),
        1
    );
}
