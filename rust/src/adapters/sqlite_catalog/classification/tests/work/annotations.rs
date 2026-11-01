use crate::domain::classification_selection::ClassificationReviewReason;

use super::*;

#[test]
fn terminal_failure_and_retry_publish_coherent_card_status() {
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
    let current_revision =
        |catalog: &SqliteCatalog| scalar(catalog, "SELECT revision FROM catalog_state") as u64;
    let revision = current_revision(&catalog);
    for attempt in 0..3 {
        let claim = catalog
            .claim_classification_work(&session, 10 + attempt)
            .unwrap()
            .unwrap();
        catalog
            .fail_classification_work(
                &claim,
                &ClassificationWorkIssue {
                    kind: ClassificationWorkFailure::Transient,
                    code: "worker_failed".into(),
                    retry_unix_ms: 11 + attempt,
                },
            )
            .unwrap();
        assert_eq!(
            current_revision(&catalog),
            revision + u64::from(attempt == 2)
        );
    }
    let revision = current_revision(&catalog);
    let targets = [selected(&catalog, "a"), selected(&catalog, "c")];
    let failed = catalog
        .classification_annotations(revision, &targets)
        .unwrap();
    assert_eq!(failed.len(), 2);
    assert!(
        failed
            .iter()
            .all(|item| item.reason == ClassificationReviewReason::AnalysisFailed)
    );
    assert_eq!(catalog.retry_classification_work(&session, 20).unwrap(), 1);
    assert_eq!(current_revision(&catalog), revision + 1);
    assert!(
        catalog
            .classification_annotations(revision + 1, &targets)
            .unwrap()
            .is_empty()
    );
    let claim = catalog
        .claim_classification_work(&session, 20)
        .unwrap()
        .unwrap();
    catalog
        .publish_classification_work(&claim, &evidence(&run, &claim))
        .unwrap();
    assert!(
        catalog
            .classification_annotations(current_revision(&catalog), &targets)
            .unwrap()
            .iter()
            .all(|item| item.reason == ClassificationReviewReason::Uncalibrated)
    );
}

#[test]
fn failed_source_does_not_override_a_human_classification() {
    let (_directory, mut catalog) = fixture();
    let (_, session) = start(&mut catalog);
    request(
        &mut catalog,
        &session,
        &["b"],
        ClassificationWorkPriority::Requested,
    );
    let claim = catalog
        .claim_classification_work(&session, 10)
        .unwrap()
        .unwrap();
    catalog
        .fail_classification_work(
            &claim,
            &ClassificationWorkIssue {
                kind: ClassificationWorkFailure::Permanent,
                code: "corrupt_image".into(),
                retry_unix_ms: 20,
            },
        )
        .unwrap();
    let target = selected(&catalog, "b");
    let revision = scalar(&catalog, "SELECT revision FROM catalog_state") as u64;
    assert_eq!(
        catalog
            .classification_annotations(revision, std::slice::from_ref(&target))
            .unwrap()[0]
            .reason,
        ClassificationReviewReason::AnalysisFailed
    );
    let prepared = prepare(&catalog, &["b"]);
    catalog
        .change_classification(
            &id("human"),
            &prepared,
            &ClassificationAction::Correct(category("photo")),
        )
        .unwrap();
    let revision = scalar(&catalog, "SELECT revision FROM catalog_state") as u64;
    assert!(
        catalog
            .classification_annotations(revision, &[target])
            .unwrap()
            .is_empty()
    );
}
