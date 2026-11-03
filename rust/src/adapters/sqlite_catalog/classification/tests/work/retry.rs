use crate::ports::classification_work::ClassificationRetryRepository;

use super::*;

fn failed_source(catalog: &mut SqliteCatalog, session: &ClassificationRecordId, name: &str) {
    request(
        catalog,
        session,
        &[name],
        ClassificationWorkPriority::Backlog,
    );
    let lease = catalog
        .claim_classification_work(session, 10)
        .unwrap()
        .unwrap();
    catalog
        .fail_classification_work(
            &lease,
            &ClassificationWorkIssue {
                kind: ClassificationWorkFailure::Permanent,
                code: "corrupt_image".into(),
                observed_unix_ms: 20,
            },
        )
        .unwrap();
}

fn retry(catalog: &mut SqliteCatalog, name: &str) -> ClassificationRetryOutcome {
    let revision = catalog.classification_dispatch_state().unwrap().revision;
    catalog
        .retry_failed_classification(revision, &selected(catalog, name), 20)
        .unwrap()
}

#[test]
fn retry_queues_only_the_observed_failed_source_and_is_repeatable() {
    let (_directory, mut catalog) = fixture();
    let (_, session) = start(&mut catalog);
    failed_source(&mut catalog, &session, "a");
    failed_source(&mut catalog, &session, "b");
    let revision = scalar(&catalog, "SELECT revision FROM catalog_state");
    assert_eq!(retry(&mut catalog, "a"), ClassificationRetryOutcome::Queued);
    assert_eq!(
        scalar(&catalog, "SELECT revision FROM catalog_state"),
        revision + 1
    );
    assert_eq!(
        retry(&mut catalog, "a"),
        ClassificationRetryOutcome::Unchanged
    );
    assert_eq!(
        scalar(&catalog, "SELECT revision FROM catalog_state"),
        revision + 1
    );
    assert_eq!(
        scalar(
            &catalog,
            "SELECT count(*) FROM classification_work_items WHERE phase = 'failed'"
        ),
        1
    );
    let claim = catalog
        .claim_classification_work(&session, 20)
        .unwrap()
        .unwrap();
    assert_eq!(claim.priority, ClassificationWorkPriority::Requested);
    assert_eq!(
        claim.target,
        ClassificationWorkTarget::VerifySource(selected(&catalog, "a"))
    );
}

#[test]
fn a_duplicate_retries_the_shared_content_without_changing_human_choices() {
    let (_directory, mut catalog) = fixture();
    let (_, session) = start(&mut catalog);
    verify(&mut catalog, "a", 'a');
    verify(&mut catalog, "c", 'a');
    request(
        &mut catalog,
        &session,
        &["a"],
        ClassificationWorkPriority::Backlog,
    );
    let source = catalog
        .claim_classification_work(&session, 10)
        .unwrap()
        .unwrap();
    catalog.finish_classification_verification(&source).unwrap();
    let content = catalog
        .claim_classification_work(&session, 10)
        .unwrap()
        .unwrap();
    catalog
        .fail_classification_work(
            &content,
            &ClassificationWorkIssue {
                kind: ClassificationWorkFailure::Permanent,
                code: "decode_failed".into(),
                observed_unix_ms: 20,
            },
        )
        .unwrap();
    let prepared = prepare(&catalog, &["a"]);
    catalog
        .change_classification(
            &id("human"),
            &prepared,
            &ClassificationAction::Correct(category("photo")),
        )
        .unwrap();
    let choices = scalar(&catalog, "SELECT count(*) FROM classification_operations");
    assert_eq!(retry(&mut catalog, "c"), ClassificationRetryOutcome::Queued);
    let retried = catalog
        .claim_classification_work(&session, 20)
        .unwrap()
        .unwrap();
    assert_eq!(retried.work_id, content.work_id);
    assert_eq!(
        scalar(&catalog, "SELECT count(*) FROM classification_operations"),
        choices
    );
    assert_eq!(
        scalar(
            &catalog,
            "SELECT count(*) FROM classification_work_items WHERE kind = 'content'"
        ),
        1
    );
}

#[test]
fn stale_source_and_dispatch_reject_without_requeueing() {
    let (_directory, mut catalog) = fixture();
    let (_, session) = start(&mut catalog);
    failed_source(&mut catalog, &session, "a");
    let before = catalog.classification_dispatch_state().unwrap();
    let mut target = selected(&catalog, "a");
    target.source_generation += 1;
    assert!(
        catalog
            .retry_failed_classification(before.revision, &target, 20)
            .is_err()
    );
    change(&mut catalog, ClassificationDispatchCommand::Pause);
    assert!(
        catalog
            .retry_failed_classification(before.revision, &selected(&catalog, "a"), 20)
            .is_err()
    );
    assert_eq!(
        scalar(
            &catalog,
            "SELECT count(*) FROM classification_work_items WHERE phase = 'failed'"
        ),
        1
    );
}

#[test]
fn retry_preserves_pause_and_rejects_stopped_or_disabled_analysis() {
    let (_directory, mut catalog) = fixture();
    let (_, session) = start(&mut catalog);
    failed_source(&mut catalog, &session, "a");
    failed_source(&mut catalog, &session, "b");
    let paused = change(&mut catalog, ClassificationDispatchCommand::Pause);
    assert_eq!(
        retry(&mut catalog, "a"),
        ClassificationRetryOutcome::QueuedWhilePaused
    );
    assert_eq!(catalog.classification_dispatch_state().unwrap(), paused);
    assert!(
        catalog
            .claim_classification_work(&session, 20)
            .unwrap()
            .is_none()
    );
    for command in [
        ClassificationDispatchCommand::Stop,
        ClassificationDispatchCommand::Disable,
    ] {
        let state = change(&mut catalog, command);
        let target = selected(&catalog, "b");
        assert_eq!(
            catalog
                .retry_failed_classification(state.revision, &target, 20)
                .unwrap_err()
                .code,
            "classification_retry_unavailable"
        );
    }
    assert_eq!(
        scalar(
            &catalog,
            "SELECT count(*) FROM classification_work_items WHERE phase = 'failed'"
        ),
        1
    );
}
