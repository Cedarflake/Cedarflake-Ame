use crate::domain::classification::work::*;
use crate::ports::classification_work::ClassificationWorkRepository;
use crate::ports::exact_duplicates::ExactDuplicateRepository;

use super::*;

mod analysis;
mod dispatch;
mod recovery;
mod verification;

fn start(catalog: &mut SqliteCatalog) -> (ClassificationRun, ClassificationRecordId) {
    let run = setup(catalog);
    let state = catalog
        .change_classification_dispatch(
            0,
            &ClassificationDispatchCommand::Enable {
                run_id: run.id().clone(),
            },
        )
        .unwrap();
    let session = id("session");
    catalog
        .open_classification_session(state.revision, &session)
        .unwrap();
    (run, session)
}

fn request(
    catalog: &mut SqliteCatalog,
    session: &ClassificationRecordId,
    names: &[&str],
    priority: ClassificationWorkPriority,
) {
    let selections = names
        .iter()
        .map(|name| selected(catalog, name))
        .collect::<Vec<_>>();
    catalog
        .request_classification_sources(session, &selections, priority, 10)
        .unwrap();
}

fn verify(catalog: &mut SqliteCatalog, name: &str, digest: char) {
    publish(catalog, name, digest);
    catalog
        .publish_exact_analysis_batch(
            "organization-fixture",
            &[selected(catalog, name).source_generation],
        )
        .unwrap();
}

fn change(
    catalog: &mut SqliteCatalog,
    command: ClassificationDispatchCommand,
) -> ClassificationDispatchState {
    let revision = catalog.classification_dispatch_state().unwrap().revision;
    catalog
        .change_classification_dispatch(revision, &command)
        .unwrap()
}

fn evidence(run: &ClassificationRun, lease: &ClassificationWorkLease) -> PredictionEvidence {
    let ClassificationWorkTarget::AnalyzeContent { content, .. } = &lease.target else {
        panic!("content claim required");
    };
    PredictionEvidence::new(
        id("queued-prediction"),
        run.clone(),
        content.clone(),
        RankedPrediction::new(
            run.primary_rules(),
            run.model(),
            vec![
                CategoryScore {
                    category_id: category("drawing"),
                    similarity: 0.8,
                },
                CategoryScore {
                    category_id: category("photo"),
                    similarity: 0.2,
                },
            ],
        )
        .unwrap(),
        None,
        ReviewRecommendation::Uncalibrated,
    )
    .unwrap()
}

#[test]
fn duplicates_share_one_prediction_before_the_complete_exact_scan_finishes() {
    let (_directory, mut catalog) = fixture();
    let (run, session) = start(&mut catalog);
    request(
        &mut catalog,
        &session,
        &["a", "c"],
        ClassificationWorkPriority::Backlog,
    );
    verify(&mut catalog, "a", 'a');
    let first = catalog
        .claim_classification_work(&session, 10)
        .unwrap()
        .unwrap();
    assert!(matches!(
        first.target,
        ClassificationWorkTarget::VerifySource(_)
    ));
    assert!(catalog.finish_classification_verification(&first).unwrap());
    let content = catalog
        .claim_classification_work(&session, 10)
        .unwrap()
        .unwrap();
    let prediction = evidence(&run, &content);
    assert!(
        catalog
            .publish_classification_work(&content, &prediction)
            .unwrap()
    );
    verify(&mut catalog, "c", 'a');
    let duplicate = catalog
        .claim_classification_work(&session, 10)
        .unwrap()
        .unwrap();
    assert!(
        catalog
            .finish_classification_verification(&duplicate)
            .unwrap()
    );
    assert!(
        catalog
            .claim_classification_work(&session, 10)
            .unwrap()
            .is_none()
    );
    assert_eq!(
        scalar(
            &catalog,
            "SELECT count(*) FROM classification_work_items WHERE kind = 'content'"
        ),
        1
    );
    assert_eq!(
        scalar(&catalog, "SELECT count(*) FROM classification_predictions"),
        1
    );
}

#[test]
fn manual_pause_survives_restart_and_fences_the_previous_claim() {
    let (directory, mut catalog) = fixture();
    let (_run, session) = start(&mut catalog);
    request(
        &mut catalog,
        &session,
        &["a"],
        ClassificationWorkPriority::Requested,
    );
    let original = catalog
        .claim_classification_work(&session, 10)
        .unwrap()
        .unwrap();
    let paused = change(&mut catalog, ClassificationDispatchCommand::Pause);
    assert!(!catalog.yield_classification_work(&original).unwrap());
    drop(catalog);
    let mut catalog = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
    assert_eq!(catalog.classification_dispatch_state().unwrap(), paused);
    let restarted = id("restarted");
    let current = catalog
        .open_classification_session(paused.revision, &restarted)
        .unwrap();
    assert_eq!(current.mode, ClassificationDispatchMode::Paused);
    assert!(
        catalog
            .claim_classification_work(&restarted, 20)
            .unwrap()
            .is_none()
    );
    change(&mut catalog, ClassificationDispatchCommand::Continue);
    let next = catalog
        .claim_classification_work(&restarted, 20)
        .unwrap()
        .unwrap();
    assert_eq!(next.work_id, original.work_id);
    assert!(next.lease_id > original.lease_id);
    assert!(!catalog.yield_classification_work(&original).unwrap());
    assert!(catalog.yield_classification_work(&next).unwrap());
}

#[test]
fn stopping_requires_an_explicit_new_run_and_keeps_compatible_work() {
    let (_directory, mut catalog) = fixture();
    let (run, session) = start(&mut catalog);
    request(
        &mut catalog,
        &session,
        &["a"],
        ClassificationWorkPriority::Requested,
    );
    let original = catalog
        .claim_classification_work(&session, 10)
        .unwrap()
        .unwrap();
    let stopped = change(&mut catalog, ClassificationDispatchCommand::Stop);
    assert!(
        catalog
            .change_classification_dispatch(
                stopped.revision,
                &ClassificationDispatchCommand::Enable {
                    run_id: run.id().clone(),
                }
            )
            .is_err()
    );
    assert!(
        catalog
            .claim_classification_work(&session, 20)
            .unwrap()
            .is_none()
    );
    assert!(!catalog.yield_classification_work(&original).unwrap());
    let replacement = ClassificationRun::new(
        id("explicit-new-run"),
        run.model(),
        run.primary_rules(),
        None,
    )
    .unwrap();
    catalog
        .begin_classification_run(&replacement, None)
        .unwrap();
    change(
        &mut catalog,
        ClassificationDispatchCommand::Enable {
            run_id: replacement.id().clone(),
        },
    );
    let next = catalog
        .claim_classification_work(&session, 20)
        .unwrap()
        .unwrap();
    assert_eq!(next.work_id, original.work_id);
    assert_eq!(next.run_id, *replacement.id());
}

#[test]
fn replacing_a_run_preserves_manual_pause() {
    let (_directory, mut catalog) = fixture();
    let (run, session) = start(&mut catalog);
    change(&mut catalog, ClassificationDispatchCommand::Pause);
    let next =
        ClassificationRun::new(id("replacement"), run.model(), run.primary_rules(), None).unwrap();
    catalog.begin_classification_run(&next, None).unwrap();
    let state = change(
        &mut catalog,
        ClassificationDispatchCommand::ReplaceRun {
            run_id: next.id().clone(),
        },
    );
    assert_eq!(state.mode, ClassificationDispatchMode::Paused);
    assert!(
        catalog
            .claim_classification_work(&session, 20)
            .unwrap()
            .is_none()
    );
}

#[test]
fn transient_failures_stop_after_three_attempts_and_require_explicit_retry() {
    let (_directory, mut catalog) = fixture();
    let (_run, session) = start(&mut catalog);
    request(
        &mut catalog,
        &session,
        &["a"],
        ClassificationWorkPriority::NewContent,
    );
    for attempt in 1..=3 {
        let lease = catalog
            .claim_classification_work(&session, attempt * 10)
            .unwrap()
            .unwrap();
        assert!(
            catalog
                .fail_classification_work(
                    &lease,
                    &ClassificationWorkIssue {
                        kind: ClassificationWorkFailure::Transient,
                        code: "worker_exited".into(),
                        retry_unix_ms: (attempt + 1) * 10,
                    }
                )
                .unwrap()
        );
        assert!(
            catalog
                .claim_classification_work(&session, attempt * 10 + 1)
                .unwrap()
                .is_none()
        );
    }
    assert!(
        catalog
            .claim_classification_work(&session, 100)
            .unwrap()
            .is_none()
    );
    assert_eq!(catalog.retry_classification_work(&session, 100).unwrap(), 1);
    assert!(
        catalog
            .claim_classification_work(&session, 100)
            .unwrap()
            .is_some()
    );
}

#[test]
fn each_priority_lane_receives_a_turn_under_continuous_higher_priority_work() {
    let (_directory, mut catalog) = fixture();
    let (_run, session) = start(&mut catalog);
    request(
        &mut catalog,
        &session,
        &["a"],
        ClassificationWorkPriority::Requested,
    );
    request(
        &mut catalog,
        &session,
        &["b"],
        ClassificationWorkPriority::NewContent,
    );
    request(
        &mut catalog,
        &session,
        &["c"],
        ClassificationWorkPriority::Backlog,
    );
    let mut priorities = Vec::new();
    for _ in 0..7 {
        let lease = catalog
            .claim_classification_work(&session, 10)
            .unwrap()
            .unwrap();
        priorities.push(lease.priority);
        assert!(catalog.yield_classification_work(&lease).unwrap());
    }
    assert_eq!(
        priorities
            .iter()
            .filter(|priority| **priority == ClassificationWorkPriority::Requested)
            .count(),
        4
    );
    assert_eq!(
        priorities
            .iter()
            .filter(|priority| **priority == ClassificationWorkPriority::NewContent)
            .count(),
        2
    );
    assert_eq!(
        priorities.last(),
        Some(&ClassificationWorkPriority::Backlog)
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
fn prediction_and_work_checkpoint_commit_atomically() {
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
    let content = catalog
        .claim_classification_work(&session, 10)
        .unwrap()
        .unwrap();
    let prediction = evidence(&run, &content);
    catalog.connection.execute_batch("CREATE TRIGGER reject_work_checkpoint BEFORE UPDATE OF phase ON classification_work_items
        WHEN OLD.kind = 'content' AND NEW.phase = 'done' BEGIN SELECT RAISE(ABORT, 'injected'); END;").unwrap();
    assert!(
        catalog
            .publish_classification_work(&content, &prediction)
            .is_err()
    );
    assert_eq!(
        scalar(&catalog, "SELECT count(*) FROM classification_predictions"),
        0
    );
    assert_eq!(
        scalar(
            &catalog,
            "SELECT count(*) FROM classification_work_items WHERE phase = 'running'"
        ),
        1
    );
    catalog
        .connection
        .execute_batch("DROP TRIGGER reject_work_checkpoint")
        .unwrap();
    assert!(
        catalog
            .publish_classification_work(&content, &prediction)
            .unwrap()
    );
    assert!(
        !catalog
            .publish_classification_work(&content, &prediction)
            .unwrap()
    );
    assert_eq!(
        scalar(&catalog, "SELECT count(*) FROM classification_predictions"),
        1
    );
}

#[test]
fn fingerprint_requires_committed_group_publication_before_content_work() {
    let (_directory, mut catalog) = fixture();
    let (_run, session) = start(&mut catalog);
    publish(&mut catalog, "a", 'a');
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
    assert!(!catalog.finish_classification_verification(&source).unwrap());
    assert_eq!(
        scalar(
            &catalog,
            "SELECT count(*) FROM classification_work_items WHERE kind = 'content'"
        ),
        0
    );
    catalog
        .publish_exact_analysis_batch(
            "organization-fixture",
            &[selected(&catalog, "a").source_generation],
        )
        .unwrap();
    let retry = catalog
        .claim_classification_work(&session, 10)
        .unwrap()
        .unwrap();
    assert!(catalog.finish_classification_verification(&retry).unwrap());
}
