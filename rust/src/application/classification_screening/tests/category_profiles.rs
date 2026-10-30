use super::*;
use crate::domain::classification::CategoryScreeningThresholds;

fn category_rule(id: &str, rescreen: f32, review: Option<f32>) -> CategoryScreeningThresholds {
    let thresholds = match review {
        Some(value) => ScreeningThresholds::new(rescreen, value),
        None => ScreeningThresholds::without_review_calibration(rescreen),
    }
    .unwrap();
    CategoryScreeningThresholds {
        category_id: CategoryId::new(id.to_owned()).unwrap(),
        thresholds,
    }
}

fn category_profile(rules: Vec<CategoryScreeningThresholds>) -> ScreeningProfile {
    let uniform = profile(&primary(), &refinement(), 0.01, 0.02);
    ScreeningProfile::for_categories(uniform.identity(), uniform.calibration(), rules).unwrap()
}

#[test]
fn initial_category_owns_refinement_and_final_category_owns_review() {
    let policy = category_profile(vec![
        category_rule("alpha", 0.0, Some(1.1)),
        category_rule("beta", 0.01, Some(0.0)),
    ]);
    let result = ScreeningClassifier::with_profile(primary(), refinement(), policy)
        .unwrap()
        .classify(&image([1.0, 0.0]))
        .unwrap();
    assert_eq!(result.initial().leading().category_id.as_str(), "beta");
    assert_eq!(result.suggestion().leading().category_id.as_str(), "alpha");
    assert_eq!(result.suggestion().margin(), Some(1.0));
    assert!(result.refined().is_some());
    assert_eq!(
        result.review_recommendation(),
        ReviewRecommendation::LowSeparation
    );
}

#[test]
fn an_unrelated_category_cutoff_does_not_trigger_refinement() {
    let policy = category_profile(vec![
        category_rule("alpha", 2.0, Some(2.0)),
        category_rule("beta", 0.0, Some(0.0)),
    ]);
    let result = ScreeningClassifier::with_profile(primary(), refinement(), policy)
        .unwrap()
        .classify(&image([1.0, 0.0]))
        .unwrap();
    assert_eq!(result.suggestion().leading().category_id.as_str(), "beta");
    assert!(result.refined().is_none());
    assert_eq!(
        result.review_recommendation(),
        ReviewRecommendation::RoutineSampling
    );
}

#[test]
fn an_unmeasured_category_keeps_its_suggestion_without_a_confidence_band() {
    let policy = category_profile(vec![
        category_rule("alpha", 0.0, None),
        category_rule("beta", 0.01, Some(0.0)),
    ]);
    let result = ScreeningClassifier::with_profile(primary(), refinement(), policy)
        .unwrap()
        .classify(&image([1.0, 0.0]))
        .unwrap();
    assert_eq!(result.suggestion().leading().category_id.as_str(), "alpha");
    assert!(result.refined().is_some());
    assert_eq!(
        result.review_recommendation(),
        ReviewRecommendation::Uncalibrated
    );
}

#[test]
fn missing_or_extra_category_rules_cannot_silently_use_a_global_cutoff() {
    for rules in [
        vec![category_rule("alpha", 0.01, Some(0.02))],
        vec![
            category_rule("alpha", 0.01, Some(0.02)),
            category_rule("beta", 0.01, Some(0.02)),
            category_rule("gamma", 0.01, Some(0.02)),
        ],
        vec![
            category_rule("alpha", 0.01, Some(0.02)),
            category_rule("gamma", 0.01, Some(0.02)),
        ],
    ] {
        let policy = category_profile(rules);
        assert!(matches!(
            ScreeningClassifier::with_profile(primary(), refinement(), policy),
            Err(ClassificationError::IncompatibleScreeningProfile)
        ));
    }
}

#[test]
fn category_rule_order_is_canonical_and_missing_calibration_is_distinct_from_zero() {
    let alpha = category_rule("alpha", 0.0, None);
    let beta = category_rule("beta", 0.01, Some(0.02));
    let forward = category_profile(vec![alpha.clone(), beta.clone()]);
    let reversed = category_profile(vec![beta.clone(), alpha]);
    let calibrated_zero = category_profile(vec![category_rule("alpha", 0.0, Some(0.0)), beta]);
    assert_eq!(forward.revision(), reversed.revision());
    assert_ne!(forward.revision(), calibrated_zero.revision());
    assert_eq!(
        forward.thresholds_for(&CategoryId::new("gamma".to_owned()).unwrap()),
        None
    );
}

#[test]
fn category_policy_identity_includes_each_threshold_and_the_policy_form() {
    let alpha = category_rule("alpha", 0.01, Some(0.02));
    let beta = category_rule("beta", 0.01, Some(0.02));
    let original = category_profile(vec![alpha.clone(), beta.clone()]);
    let changed = category_profile(vec![alpha, category_rule("beta", 0.01, Some(0.03))]);
    let uniform = profile(&primary(), &refinement(), 0.01, 0.02);
    assert_ne!(original.revision(), changed.revision());
    assert_ne!(original.revision(), uniform.revision());
    assert_eq!(original.identity(), changed.identity());
    assert_eq!(
        original.thresholds_for(&beta.category_id),
        Some(beta.thresholds)
    );
}

#[test]
fn empty_duplicate_and_unbounded_rule_sets_are_rejected() {
    let uniform = profile(&primary(), &refinement(), 0.01, 0.02);
    for rules in [
        vec![],
        vec![category_rule("alpha", 0.0, None); 2],
        (0..257)
            .map(|index| category_rule(&format!("category-{index}"), 0.0, None))
            .collect(),
    ] {
        assert_eq!(
            ScreeningProfile::for_categories(uniform.identity(), uniform.calibration(), rules),
            Err(ClassificationError::InvalidScreeningCategories)
        );
    }
}
