use super::*;

fn id(value: &str) -> ClassificationRecordId {
    ClassificationRecordId::new(value.into()).unwrap()
}

fn content() -> ClassificationContent {
    ClassificationContent {
        scheme: id("blake3-256-full-v1"),
        digest: [20; 32],
        byte_length: 512,
    }
}

#[test]
fn classifier_result_publishes_both_passes_under_its_admitted_run() {
    let first = primary();
    let second = refinement();
    let policy = profile(&first, &second, 0.1, 0.01);
    let classifier = ScreeningClassifier::with_profile(first, second, policy).unwrap();
    let run = classifier.describe_run(id("run-1")).unwrap();
    let result = classifier.classify(&image([1.0, 0.0])).unwrap();
    assert!(result.refined().is_some());
    let selected = result.suggestion().leading().category_id.clone();
    let evidence = result
        .into_evidence(id("prediction-1"), run.clone(), content())
        .unwrap();
    assert_eq!(evidence.run(), &run);
    assert_eq!(evidence.content(), &content());
    assert_eq!(evidence.proposal().evidence.category, selected);
    assert_eq!(
        evidence.review_recommendation(),
        ReviewRecommendation::RoutineSampling
    );
}

#[test]
fn a_non_refined_result_still_binds_the_complete_screening_configuration() {
    let first = primary();
    let second = refinement();
    let policy = profile(&first, &second, 0.0, 0.0);
    let classifier = ScreeningClassifier::with_profile(first, second, policy).unwrap();
    let run = classifier.describe_run(id("run-1")).unwrap();
    let result = classifier.classify(&image([1.0, 0.0])).unwrap();
    assert!(result.refined().is_none());
    let identity = run.refinement().unwrap();
    for changed in [
        RefinementIdentity {
            rules: revision(30),
            ..identity
        },
        RefinementIdentity {
            profile: revision(30),
            ..identity
        },
    ] {
        let other_run =
            ClassificationRun::new(id("run-2"), run.model(), run.primary_rules(), Some(changed))
                .unwrap();
        assert_eq!(
            result
                .clone()
                .into_evidence(id("prediction-1"), other_run, content()),
            Err(ClassificationError::IncompatiblePrediction)
        );
    }
}

#[test]
fn an_uncalibrated_result_cannot_be_attached_to_a_calibrated_run() {
    let classifier = ScreeningClassifier::uncalibrated(primary());
    let run = classifier.describe_run(id("run-1")).unwrap();
    let result = classifier.classify(&image([1.0, 0.0])).unwrap();
    let calibrated = ClassificationRun::new(
        id("run-2"),
        run.model(),
        run.primary_rules(),
        Some(RefinementIdentity {
            rules: revision(30),
            profile: revision(31),
        }),
    )
    .unwrap();
    assert_eq!(
        result
            .clone()
            .into_evidence(id("prediction-1"), calibrated, content()),
        Err(ClassificationError::IncompatiblePrediction)
    );
    assert_eq!(
        result
            .into_evidence(id("prediction-1"), run, content())
            .unwrap()
            .review_recommendation(),
        ReviewRecommendation::Uncalibrated
    );
}
