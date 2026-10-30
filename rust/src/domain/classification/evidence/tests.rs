use super::*;
use crate::domain::classification::CategoryScore;

fn revision(value: u8) -> ClassificationRevision {
    ClassificationRevision::from_bytes([value; 32])
}

fn id(value: &str) -> ClassificationRecordId {
    ClassificationRecordId::new(value.into()).unwrap()
}

fn model() -> ModelCompatibility {
    ModelCompatibility::new(revision(1), revision(2), revision(3), 2).unwrap()
}

fn scores(values: &[(&str, f32)]) -> Vec<CategoryScore> {
    values
        .iter()
        .map(|(category, score)| CategoryScore {
            category_id: CategoryId::new((*category).into()).unwrap(),
            similarity: *score,
        })
        .collect()
}

fn ranking(rules: u8, values: &[(&str, f32)]) -> RankedPrediction {
    RankedPrediction::new(revision(rules), model(), scores(values)).unwrap()
}

fn content() -> ClassificationContent {
    ClassificationContent {
        scheme: id("blake3-256-full-v1"),
        digest: [20; 32],
        byte_length: 512,
    }
}

fn run(refine: bool) -> ClassificationRun {
    ClassificationRun::new(
        id("run-1"),
        model(),
        revision(4),
        refine.then_some(RefinementIdentity {
            rules: revision(5),
            profile: revision(6),
        }),
    )
    .unwrap()
}

#[test]
fn ranking_rejects_invalid_values_and_duplicate_categories() {
    for values in [
        vec![],
        scores(&[("photo", f32::NAN)]),
        scores(&[("photo", f32::INFINITY)]),
        scores(&[("photo", 1.01)]),
        scores(&[("photo", -1.01)]),
        scores(&[("photo", 0.3), ("photo", 0.4)]),
        (0..257)
            .map(|index| CategoryScore {
                category_id: CategoryId::new(format!("category-{index}")).unwrap(),
                similarity: 0.5,
            })
            .collect(),
    ] {
        assert_eq!(
            RankedPrediction::new(revision(4), model(), values),
            Err(ClassificationError::InvalidPrediction)
        );
    }
}

#[test]
fn ranking_keeps_stable_ties_and_top_two_separation() {
    let value = ranking(4, &[("z", -0.0), ("a", 0.0), ("b", -0.5)]);
    assert_eq!(value.leading().category_id.as_str(), "a");
    assert_eq!(value.margin(), Some(0.0));
    assert_eq!(
        value.categories()[1].similarity.to_bits(),
        0.0_f32.to_bits()
    );
    assert_eq!(ranking(4, &[("photo", 0.2)]).margin(), None);
}

#[test]
fn rejects_path_like_and_unbounded_record_identifiers() {
    for invalid in [
        "".to_owned(),
        "../result".into(),
        "run\n2".into(),
        "x".repeat(129),
    ] {
        assert_eq!(
            ClassificationRecordId::new(invalid),
            Err(ClassificationError::InvalidRecordId)
        );
    }
    assert_eq!(id("local_run-2.result").as_str(), "local_run-2.result");
}

#[test]
fn separate_runs_share_configuration_without_sharing_identity() {
    let first = run(false);
    let second = ClassificationRun::new(id("run-2"), model(), revision(4), None).unwrap();
    assert_eq!(first.configuration(), second.configuration());
    assert_ne!(first.id(), second.id());
    assert_ne!(first.configuration(), run(true).configuration());
}

#[test]
fn every_encoder_and_rule_component_participates_in_configuration() {
    let base = run(false).configuration();
    for candidate in [
        ModelCompatibility::new(revision(9), revision(2), revision(3), 2).unwrap(),
        ModelCompatibility::new(revision(1), revision(9), revision(3), 2).unwrap(),
        ModelCompatibility::new(revision(1), revision(2), revision(9), 2).unwrap(),
        ModelCompatibility::new(revision(1), revision(2), revision(3), 3).unwrap(),
    ] {
        assert_ne!(
            base,
            ClassificationRun::new(id("run-1"), candidate, revision(4), None)
                .unwrap()
                .configuration()
        );
    }
    assert_ne!(
        base,
        ClassificationRun::new(id("run-1"), model(), revision(9), None)
            .unwrap()
            .configuration()
    );
    for refinement in [
        RefinementIdentity {
            rules: revision(9),
            profile: revision(6),
        },
        RefinementIdentity {
            rules: revision(5),
            profile: revision(9),
        },
    ] {
        assert_ne!(
            run(true).configuration(),
            ClassificationRun::new(id("run-1"), model(), revision(4), Some(refinement))
                .unwrap()
                .configuration()
        );
    }
}

#[test]
fn retains_first_and_refined_evidence_with_the_selected_category() {
    let value = PredictionEvidence::new(
        id("prediction-1"),
        run(true),
        content(),
        ranking(4, &[("photo", 0.6), ("meme", 0.59)]),
        Some(ranking(5, &[("photo", 0.1), ("meme", 0.7)])),
        ReviewRecommendation::RoutineSampling,
    )
    .unwrap();
    assert_eq!(value.initial().leading().category_id.as_str(), "photo");
    assert_eq!(
        value.refined().unwrap().leading().category_id.as_str(),
        "meme"
    );
    let reference = value.proposal().evidence;
    assert_eq!(reference.prediction_id, id("prediction-1"));
    assert_eq!(reference.run_id, id("run-1"));
    assert_eq!(reference.content, content());
    assert_eq!(reference.primary_rules, revision(4));
    assert_eq!(reference.category.as_str(), "meme");
}

#[test]
fn uncalibrated_evidence_cannot_claim_a_calibrated_review_band() {
    for review in [
        ReviewRecommendation::LowSeparation,
        ReviewRecommendation::RoutineSampling,
    ] {
        assert_eq!(
            PredictionEvidence::new(
                id("prediction-1"),
                run(false),
                content(),
                ranking(4, &[("photo", 0.6)]),
                None,
                review
            ),
            Err(ClassificationError::IncompatiblePrediction)
        );
    }
}

#[test]
fn rejects_refinement_with_different_categories_or_rules() {
    for refined in [
        ranking(5, &[("photo", 0.5)]),
        ranking(5, &[("other", 0.5), ("meme", 0.4)]),
        ranking(9, &[("photo", 0.5), ("meme", 0.4)]),
    ] {
        assert_eq!(
            PredictionEvidence::new(
                id("prediction-1"),
                run(true),
                content(),
                ranking(4, &[("photo", 0.6), ("meme", 0.59)]),
                Some(refined),
                ReviewRecommendation::RoutineSampling
            ),
            Err(ClassificationError::IncompatiblePrediction)
        );
    }
}

#[test]
fn rejects_predictions_from_another_model_or_primary_rule_revision() {
    let different_model =
        ModelCompatibility::new(revision(11), revision(2), revision(3), 2).unwrap();
    for initial in [
        ranking(9, &[("photo", 0.5)]),
        RankedPrediction::new(revision(4), different_model, scores(&[("photo", 0.5)])).unwrap(),
    ] {
        assert_eq!(
            PredictionEvidence::new(
                id("prediction-1"),
                run(false),
                content(),
                initial,
                None,
                ReviewRecommendation::Uncalibrated
            ),
            Err(ClassificationError::IncompatiblePrediction)
        );
    }
}

#[test]
fn a_run_without_refinement_cannot_accept_a_second_ranking() {
    assert_eq!(
        PredictionEvidence::new(
            id("prediction-1"),
            run(false),
            content(),
            ranking(4, &[("photo", 0.6)]),
            Some(ranking(5, &[("photo", 0.6)])),
            ReviewRecommendation::Uncalibrated
        ),
        Err(ClassificationError::IncompatiblePrediction)
    );
}
