use super::*;
use crate::domain::classification::{ClassificationRecordId, Embedding};

fn model() -> ModelCompatibility {
    ModelCompatibility::new(
        ClassificationRevision::from_bytes([1; 32]),
        ClassificationRevision::from_bytes([2; 32]),
        ClassificationRevision::from_bytes([3; 32]),
        2,
    )
    .unwrap()
}

fn sample(id: u8, values: Vec<f32>) -> DiscoverySample {
    DiscoverySample {
        content: ClassificationContent {
            scheme: ClassificationRecordId::new("fixture-v1".into()).unwrap(),
            digest: [id; 32],
            byte_length: 100,
        },
        feature: ImageFeature {
            identity: model().image_identity(),
            embedding: Embedding::normalized(values).unwrap(),
        },
    }
}

#[test]
fn cohort_scores_mean_similarity_without_promoting_a_diffuse_group_to_unit_length() {
    let members = [sample(1, vec![1.0, 0.0]), sample(2, vec![0.0, 1.0])];
    let comparison = [sample(3, vec![-1.0, 0.0])];
    let cohort = DiscoveryCohort::new(model(), &members, &comparison).unwrap();
    let text = TextFeature {
        identity: model().text_identity(),
        description: ClassificationRevision::description("a scene"),
        embedding: Embedding::normalized(vec![1.0, 0.0]).unwrap(),
    };
    assert_eq!(
        cohort.score(&text).unwrap(),
        ConceptSimilarity {
            similarity: 0.5,
            context_similarity: -1.0,
            contrast: 1.5,
        }
    );
}

#[test]
fn repeated_content_cannot_inflate_support_or_leak_into_comparison() {
    let repeated = [sample(1, vec![1.0, 0.0]), sample(1, vec![1.0, 0.0])];
    let comparison = [sample(2, vec![0.0, 1.0])];
    assert!(matches!(
        DiscoveryCohort::new(model(), &repeated, &comparison),
        Err(DiscoveryError::DuplicateContent)
    ));
    assert!(matches!(
        DiscoveryCohort::new(model(), &repeated[..1], &repeated[1..]),
        Err(DiscoveryError::DuplicateContent)
    ));
}

#[test]
fn cohort_identity_preserves_roles_and_ignores_input_order() {
    let first = sample(1, vec![1.0, 0.0]);
    let second = sample(2, vec![0.0, 1.0]);
    let context = [sample(3, vec![-1.0, 0.0])];
    let members = [first, second];
    let original = DiscoveryCohort::new(model(), &members, &context).unwrap();
    let reversed = [sample(2, vec![0.0, 1.0]), sample(1, vec![1.0, 0.0])];
    assert_eq!(
        original.revision(),
        DiscoveryCohort::new(model(), &reversed, &context)
            .unwrap()
            .revision()
    );
    assert_ne!(
        original.revision(),
        DiscoveryCohort::new(model(), &context, &members)
            .unwrap()
            .revision()
    );
}

#[test]
fn invalid_cohorts_and_incompatible_features_fail_before_ranking() {
    let members = [sample(1, vec![1.0, 0.0])];
    assert!(matches!(
        DiscoveryCohort::new(model(), &members, &[]),
        Err(DiscoveryError::InvalidCohort)
    ));
    let mut wrong = sample(2, vec![0.0, 1.0]);
    wrong.feature.identity.pipeline = ClassificationRevision::from_bytes([9; 32]);
    assert!(matches!(
        DiscoveryCohort::new(model(), &members, &[wrong]),
        Err(DiscoveryError::IncompatibleFeature)
    ));
    let too_many: Vec<_> = (0..512).map(|_| sample(3, vec![1.0, 0.0])).collect();
    assert!(matches!(
        DiscoveryCohort::new(model(), &members, &too_many),
        Err(DiscoveryError::InvalidCohort)
    ));
}
