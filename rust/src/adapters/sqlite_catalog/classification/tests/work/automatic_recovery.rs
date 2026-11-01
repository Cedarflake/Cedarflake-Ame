use super::*;

use crate::adapters::sqlite_catalog::tests::publish_gallery_fixture;

fn fail_source(
    catalog: &mut SqliteCatalog,
    session: &ClassificationRecordId,
) -> ClassificationWorkLease {
    request(
        catalog,
        session,
        &["a"],
        ClassificationWorkPriority::Requested,
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
                code: "invalid_image".into(),
                observed_unix_ms: 10,
            },
        )
        .unwrap();
    lease
}

#[test]
fn failed_input_waits_for_model_change_without_resetting_on_a_compatible_run() {
    let (_directory, mut catalog) = fixture();
    let (original, session) = start(&mut catalog);
    let failed = fail_source(&mut catalog, &session);
    for (index, model) in [
        original.model(),
        ModelCompatibility::new(
            ClassificationRevision::from_bytes([4; 32]),
            ClassificationRevision::from_bytes([2; 32]),
            ClassificationRevision::from_bytes([3; 32]),
            2,
        )
        .unwrap(),
    ]
    .into_iter()
    .enumerate()
    {
        let run = ClassificationRun::new(
            id(&format!("replacement-{index}")),
            model,
            original.primary_rules(),
            None,
        )
        .unwrap();
        catalog.begin_classification_run(&run, None).unwrap();
        change(
            &mut catalog,
            ClassificationDispatchCommand::ReplaceRun {
                run_id: run.id().clone(),
            },
        );
        request(
            &mut catalog,
            &session,
            &["a"],
            ClassificationWorkPriority::Requested,
        );
        let claim = catalog
            .claim_classification_work(&session, 1_000_000)
            .unwrap();
        if index == 0 {
            assert!(claim.is_none());
        } else {
            let claim = claim.unwrap();
            assert_ne!(claim.work_id, failed.work_id);
            assert_eq!(claim.target, failed.target);
            assert_eq!(claim.run_id, *run.id());
        }
    }
}

#[test]
fn transient_failure_leaves_other_sources_runnable_and_respects_manual_pause() {
    let (_directory, mut catalog) = fixture();
    let (_, session) = start(&mut catalog);
    request(
        &mut catalog,
        &session,
        &["a", "b"],
        ClassificationWorkPriority::Requested,
    );
    let first = catalog
        .claim_classification_work(&session, 10)
        .unwrap()
        .unwrap();
    catalog
        .fail_classification_work(
            &first,
            &ClassificationWorkIssue {
                kind: ClassificationWorkFailure::Transient,
                code: "worker_exited".into(),
                observed_unix_ms: 10,
            },
        )
        .unwrap();
    let second = catalog
        .claim_classification_work(&session, 11)
        .unwrap()
        .unwrap();
    assert_ne!(second.work_id, first.work_id);
    catalog
        .fail_classification_work(
            &second,
            &ClassificationWorkIssue {
                kind: ClassificationWorkFailure::Permanent,
                code: "invalid_image".into(),
                observed_unix_ms: 11,
            },
        )
        .unwrap();
    change(&mut catalog, ClassificationDispatchCommand::Pause);
    assert!(
        catalog
            .claim_classification_work(&session, 100_000)
            .unwrap()
            .is_none()
    );
    change(&mut catalog, ClassificationDispatchCommand::Continue);
    let resumed = catalog
        .claim_classification_work(&session, 100_000)
        .unwrap()
        .unwrap();
    assert_eq!(resumed.work_id, first.work_id);
    catalog
        .fail_classification_work(
            &resumed,
            &ClassificationWorkIssue {
                kind: ClassificationWorkFailure::Transient,
                code: "worker_exited".into(),
                observed_unix_ms: 100_000,
            },
        )
        .unwrap();
    assert_eq!(
        catalog.classification_work_ready_at(&session).unwrap(),
        Some(130_000)
    );
}

#[test]
fn changed_source_is_admitted_after_a_permanent_failure() {
    let (_directory, mut catalog) = fixture();
    let (_, session) = start(&mut catalog);
    let failed = fail_source(&mut catalog, &session);
    assert!(
        catalog
            .claim_classification_work(&session, 1_000_000)
            .unwrap()
            .is_none()
    );
    publish_gallery_fixture(
        &mut catalog,
        "changed-scan",
        "root-a",
        "C:\\Fixture",
        &[("a", None, 100), ("b", None, 2)],
    );
    request(
        &mut catalog,
        &session,
        &["a"],
        ClassificationWorkPriority::Requested,
    );
    let recovered = catalog
        .claim_classification_work(&session, 1_000_000)
        .unwrap()
        .unwrap();
    assert_ne!(recovered.work_id, failed.work_id);
    let ClassificationWorkTarget::VerifySource(original) = failed.target else {
        panic!("source required")
    };
    let ClassificationWorkTarget::VerifySource(current) = recovered.target else {
        panic!("source required")
    };
    assert_eq!(current.location_id, original.location_id);
    assert_ne!(current.source_generation, original.source_generation);
}
