use super::*;

use crate::domain::classification::{
    CategoryDefinition, CategoryId, Embedding, ModelCompatibility, ScreeningThresholds, Taxonomy,
    TextFeature,
};

fn revision(value: u8) -> ClassificationRevision {
    ClassificationRevision::from_bytes([value; 32])
}

fn model() -> ModelCompatibility {
    ModelCompatibility::new(revision(1), revision(2), revision(3), 2).unwrap()
}

fn matcher(rows: &[(&str, &str, [f32; 2])]) -> CategoryMatcher {
    let categories = rows
        .iter()
        .map(|(id, text, _)| {
            CategoryDefinition::new(
                CategoryId::new((*id).to_owned()).unwrap(),
                (*id).to_owned(),
                vec![(*text).to_owned()],
            )
            .unwrap()
        })
        .collect();
    let features = rows
        .iter()
        .map(|(_, text, values)| TextFeature {
            identity: model().text_identity(),
            description: ClassificationRevision::description(text),
            embedding: Embedding::normalized(values.to_vec()).unwrap(),
        })
        .collect();
    CategoryMatcher::new(Taxonomy::new(categories).unwrap(), model(), features).unwrap()
}

fn primary() -> CategoryMatcher {
    matcher(&[
        ("alpha", "first alpha", [1.0, 0.05]),
        ("beta", "first beta", [1.0, 0.0]),
    ])
}

fn refinement() -> CategoryMatcher {
    matcher(&[
        ("alpha", "refined alpha", [1.0, 0.0]),
        ("beta", "refined beta", [0.0, 1.0]),
    ])
}

fn profile(
    primary: &CategoryMatcher,
    refinement: &CategoryMatcher,
    rescreen_below: f32,
    review_below: f32,
) -> ScreeningProfile {
    ScreeningProfile::new(
        ScreeningIdentity {
            model: model(),
            primary_rules: primary.taxonomy().revision(),
            refinement_rules: refinement.taxonomy().revision(),
        },
        revision(4),
        ScreeningThresholds::new(rescreen_below, review_below).unwrap(),
    )
}

fn image(values: [f32; 2]) -> ImageFeature {
    ImageFeature {
        identity: model().image_identity(),
        embedding: Embedding::normalized(values.to_vec()).unwrap(),
    }
}

#[test]
fn retains_a_usable_suggestion_without_a_calibration_profile() {
    let result = ScreeningClassifier::uncalibrated(primary())
        .classify(&image([1.0, 0.0]))
        .unwrap();
    assert_eq!(result.suggestion().leading().category_id.as_str(), "beta");
    assert_eq!(
        result.review_recommendation(),
        ReviewRecommendation::Uncalibrated
    );
    assert!(result.refined().is_none());
    assert!(result.profile_revision().is_none());
}

#[test]
fn rescreens_a_boundary_once_with_the_same_cached_image() {
    let first = primary();
    let second = refinement();
    let policy = profile(&first, &second, 0.01, 0.02);
    let classifier = ScreeningClassifier::with_profile(first, second, policy).unwrap();
    let result = classifier.classify(&image([1.0, 0.0])).unwrap();
    assert_eq!(result.initial().leading().category_id.as_str(), "beta");
    assert_eq!(result.suggestion().leading().category_id.as_str(), "alpha");
    assert_eq!(result.initial().model(), result.suggestion().model());
    assert_ne!(
        result.initial().rules_revision(),
        result.suggestion().rules_revision()
    );
    assert_eq!(result.refined(), Some(result.suggestion()));
    assert_eq!(result.profile_revision(), Some(policy.revision()));
    assert_eq!(
        result.review_recommendation(),
        ReviewRecommendation::RoutineSampling
    );
}

#[test]
fn preserves_a_separated_first_pass_without_an_unnecessary_refinement() {
    let first = primary();
    let second = refinement();
    let policy = profile(&first, &second, 0.01, 0.02);
    let result = ScreeningClassifier::with_profile(first, second, policy)
        .unwrap()
        .classify(&image([0.0, 1.0]))
        .unwrap();
    assert!(result.refined().is_none());
    assert_eq!(result.suggestion(), result.initial());
    assert_eq!(
        result.review_recommendation(),
        ReviewRecommendation::RoutineSampling
    );
}

#[test]
fn a_refined_tie_remains_usable_and_uncertain_without_another_pass() {
    let first = primary();
    let second = matcher(&[
        ("alpha", "second alpha", [1.0, 0.0]),
        ("beta", "second beta", [1.0, 0.0]),
    ]);
    let policy = profile(&first, &second, 0.01, 0.0);
    let result = ScreeningClassifier::with_profile(first, second, policy)
        .unwrap()
        .classify(&image([1.0, 0.0]))
        .unwrap();
    assert_eq!(result.suggestion().margin(), Some(0.0));
    assert_eq!(result.suggestion().leading().category_id.as_str(), "alpha");
    assert_eq!(
        result.review_recommendation(),
        ReviewRecommendation::LowSeparation
    );
    assert!(result.refined().is_some());
}

#[test]
fn threshold_equality_uses_the_separated_side() {
    let first = primary();
    let second = refinement();
    let image = image([1.0, 0.0]);
    let boundary = first.rank(&image).unwrap().margin().unwrap();
    let policy = profile(&first, &second, boundary, boundary);
    let result = ScreeningClassifier::with_profile(first, second, policy)
        .unwrap()
        .classify(&image)
        .unwrap();
    assert!(result.refined().is_none());
    assert_eq!(
        result.review_recommendation(),
        ReviewRecommendation::RoutineSampling
    );
}

#[test]
fn first_pass_ties_receive_one_refinement_even_at_a_zero_cutoff() {
    let first = matcher(&[
        ("alpha", "tied alpha", [1.0, 0.0]),
        ("beta", "tied beta", [1.0, 0.0]),
    ]);
    let second = refinement();
    let policy = profile(&first, &second, 0.0, 0.0);
    let result = ScreeningClassifier::with_profile(first, second, policy)
        .unwrap()
        .classify(&image([1.0, 0.0]))
        .unwrap();
    assert_eq!(result.initial().margin(), Some(0.0));
    assert_eq!(result.suggestion().margin(), Some(1.0));
    assert!(result.refined().is_some());
}

#[test]
fn a_narrow_first_pass_stays_suggested_when_it_does_not_need_rescreening() {
    let first = primary();
    let second = refinement();
    let policy = profile(&first, &second, 0.0, 0.02);
    let result = ScreeningClassifier::with_profile(first, second, policy)
        .unwrap()
        .classify(&image([1.0, 0.0]))
        .unwrap();
    assert!(result.refined().is_none());
    assert_eq!(result.suggestion().leading().category_id.as_str(), "beta");
    assert_eq!(
        result.review_recommendation(),
        ReviewRecommendation::LowSeparation
    );
}

#[test]
fn a_single_category_does_not_invent_a_separation_band() {
    let first = matcher(&[("custom", "original", [1.0, 0.0])]);
    let second = matcher(&[("custom", "specific", [0.0, 1.0])]);
    let policy = profile(&first, &second, 0.1, 0.2);
    let result = ScreeningClassifier::with_profile(first, second, policy)
        .unwrap()
        .classify(&image([1.0, 0.0]))
        .unwrap();
    assert_eq!(result.suggestion().leading().category_id.as_str(), "custom");
    assert!(result.refined().is_none());
    assert_eq!(
        result.review_recommendation(),
        ReviewRecommendation::Uncalibrated
    );
}

#[test]
fn rejects_a_profile_after_semantic_or_model_changes() {
    let first = primary();
    let second = refinement();
    let original = profile(&first, &second, 0.01, 0.02);
    for changed_identity in [
        ScreeningIdentity {
            primary_rules: revision(5),
            ..original.identity()
        },
        ScreeningIdentity {
            refinement_rules: revision(6),
            ..original.identity()
        },
        ScreeningIdentity {
            model: ModelCompatibility::new(revision(1), revision(2), revision(7), 2).unwrap(),
            ..original.identity()
        },
    ] {
        let changed = ScreeningProfile::new(
            changed_identity,
            original.calibration(),
            original.thresholds(),
        );
        assert!(matches!(
            ScreeningClassifier::with_profile(primary(), refinement(), changed),
            Err(ClassificationError::IncompatibleScreeningProfile)
        ));
    }
}

#[test]
fn rejects_added_or_retired_categories_and_unchanged_refinement() {
    let first = primary();
    let second = refinement();
    let policy = profile(&first, &second, 0.01, 0.02);
    for invalid in [
        matcher(&[("alpha", "only alpha", [1.0, 0.0])]),
        matcher(&[
            ("alpha", "alpha", [1.0, 0.0]),
            ("gamma", "gamma", [0.0, 1.0]),
        ]),
        primary(),
    ] {
        assert!(matches!(
            ScreeningClassifier::with_profile(primary(), invalid, policy),
            Err(ClassificationError::IncompatibleRefinement)
        ));
    }
}

#[test]
fn rejects_incompatible_cached_features_before_publishing_a_suggestion() {
    let mut cached = image([1.0, 0.0]);
    cached.identity.pipeline = revision(9);
    assert_eq!(
        ScreeningClassifier::uncalibrated(primary()).classify(&cached),
        Err(ClassificationError::IncompatibleFeature)
    );
}

#[test]
fn display_only_renaming_keeps_the_profile_compatible() {
    let first = primary();
    let second = refinement();
    let policy = profile(&first, &second, 0.01, 0.02);
    let categories = first
        .taxonomy()
        .categories()
        .iter()
        .map(|category| {
            category
                .renamed(format!("Renamed {}", category.label()))
                .unwrap()
        })
        .collect();
    let first = CategoryMatcher::new(
        Taxonomy::new(categories).unwrap(),
        first.model(),
        vec![
            TextFeature {
                identity: model().text_identity(),
                description: ClassificationRevision::description("first alpha"),
                embedding: Embedding::normalized(vec![1.0, 0.05]).unwrap(),
            },
            TextFeature {
                identity: model().text_identity(),
                description: ClassificationRevision::description("first beta"),
                embedding: Embedding::normalized(vec![1.0, 0.0]).unwrap(),
            },
        ],
    )
    .unwrap();
    assert!(ScreeningClassifier::with_profile(first, second, policy).is_ok());
}

#[test]
fn versions_thresholds_and_reference_evidence_independently_of_features() {
    let original = profile(&primary(), &refinement(), 0.01, 0.02);
    let changed_evidence =
        ScreeningProfile::new(original.identity(), revision(8), original.thresholds());
    let changed_threshold = ScreeningProfile::new(
        original.identity(),
        original.calibration(),
        ScreeningThresholds::new(0.01, 0.03).unwrap(),
    );
    assert_ne!(original.revision(), changed_evidence.revision());
    assert_ne!(original.revision(), changed_threshold.revision());
    assert_eq!(original.identity(), changed_threshold.identity());
}

#[test]
fn rejects_non_finite_or_impossible_separation_thresholds() {
    for invalid in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, -0.1, 2.1] {
        assert_eq!(
            ScreeningThresholds::new(invalid, 0.1),
            Err(ClassificationError::InvalidScreeningThresholds)
        );
        assert_eq!(
            ScreeningThresholds::new(0.1, invalid),
            Err(ClassificationError::InvalidScreeningThresholds)
        );
    }
    assert_eq!(
        ScreeningThresholds::new(-0.0, 0.0),
        ScreeningThresholds::new(0.0, 0.0)
    );
}
