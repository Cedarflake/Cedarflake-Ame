use crate::domain::classification::{
    CategoryDefinition, CategoryId, ClassificationRecordId, ClassificationRevision, Embedding,
    ImageFeature, ModelCompatibility, Taxonomy, TextFeature,
};

use super::*;

fn model() -> ModelCompatibility {
    ModelCompatibility::new(
        ClassificationRevision::from_bytes([1; 32]),
        ClassificationRevision::from_bytes([2; 32]),
        ClassificationRevision::from_bytes([3; 32]),
        3,
    )
    .unwrap()
}

fn matcher(purposes: bool) -> CategoryMatcher {
    let definitions = if purposes {
        vec![("a", vec![1.0, 0.0, 0.0]), ("b", vec![0.0, 1.0, 0.0])]
    } else {
        vec![("base", vec![0.0, 0.0, 1.0])]
    };
    let rules = Taxonomy::new(
        definitions
            .iter()
            .map(|(id, _)| {
                CategoryDefinition::new(
                    CategoryId::new((*id).into()).unwrap(),
                    (*id).into(),
                    vec![(*id).into()],
                )
                .unwrap()
            })
            .collect(),
    )
    .unwrap();
    let features = definitions
        .into_iter()
        .map(|(description, values)| TextFeature {
            identity: model().text_identity(),
            description: ClassificationRevision::description(description),
            embedding: Embedding::normalized(values).unwrap(),
        })
        .collect();
    CategoryMatcher::new(rules, model(), features).unwrap()
}

fn sample(index: u64, values: [f32; 3]) -> DiscoverySample {
    let mut digest = [0; 32];
    digest[..8].copy_from_slice(&index.to_le_bytes());
    DiscoverySample {
        content: ClassificationContent {
            scheme: ClassificationRecordId::new("fixture".into()).unwrap(),
            digest,
            byte_length: 100,
        },
        feature: ImageFeature {
            identity: model().image_identity(),
            embedding: Embedding::normalized(values.to_vec()).unwrap(),
        },
    }
}

fn policy() -> DiscoveryCohortPolicy {
    DiscoveryCohortPolicy {
        minimum_pair_cosine: 0.75,
        minimum_members: 3,
        maximum_cohorts: 64,
    }
}

fn signature(
    batch: &DiscoveryCohortBatch,
) -> Vec<(Vec<ClassificationContent>, Vec<ClassificationContent>)> {
    batch
        .cohorts
        .iter()
        .map(|cohort| {
            (
                cohort.member_contents().cloned().collect(),
                cohort.comparison_contents().to_vec(),
            )
        })
        .collect()
}

#[test]
fn formation_is_deterministic_and_retains_sparse_contents() {
    let mut samples: Vec<_> = (0..7)
        .map(|index| {
            sample(
                index,
                match index {
                    0..=2 => [1.0, 0.0, 0.0],
                    3..=5 => [0.0, 1.0, 0.0],
                    _ => [-1.0, 0.0, 0.0],
                },
            )
        })
        .collect();
    let first =
        form_discovery_cohorts(&samples, &matcher(false), policy(), &AtomicBool::new(false))
            .unwrap();
    assert_eq!(first.cohorts.len(), 2);
    assert_eq!(
        first.deferred,
        vec![DeferredDiscoveryCohort {
            members: vec![samples[6].content.clone()],
            reason: DiscoveryCohortDeferral::InsufficientMembers
        }]
    );
    assert!(
        first
            .cohorts
            .iter()
            .all(|cohort| cohort.member_contents().count() == 3
                && cohort.comparison_contents().len() == 4)
    );
    samples.reverse();
    let reversed =
        form_discovery_cohorts(&samples, &matcher(false), policy(), &AtomicBool::new(false))
            .unwrap();
    assert_eq!(signature(&first), signature(&reversed));
    assert_eq!(first.deferred, reversed.deferred);
    assert_eq!(
        first
            .cohorts
            .iter()
            .map(DiscoveryCohort::revision)
            .collect::<Vec<_>>(),
        reversed
            .cohorts
            .iter()
            .map(DiscoveryCohort::revision)
            .collect::<Vec<_>>()
    );
}

#[test]
fn complete_link_rejects_a_chain_with_dissimilar_endpoints() {
    let samples = [
        sample(0, [1.0, 0.0, 0.0]),
        sample(1, [0.8660254, 0.5, 0.0]),
        sample(2, [0.5, 0.8660254, 0.0]),
    ];
    let result = form_discovery_cohorts(
        &samples,
        &matcher(false),
        DiscoveryCohortPolicy {
            minimum_pair_cosine: 0.8,
            minimum_members: 2,
            ..policy()
        },
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(result.cohorts.len(), 1);
    let members = result.cohorts[0].member_samples();
    assert_eq!(members.len(), 2);
    assert!(
        members[0]
            .feature
            .embedding
            .cosine(&members[1].feature.embedding)
            .unwrap()
            >= 0.8
    );
    assert_eq!(result.deferred.len(), 1);
}

#[test]
fn comparison_uses_other_contents_with_the_same_current_purpose() {
    let samples: Vec<_> = (0..8)
        .map(|index| {
            sample(
                index,
                match index {
                    0..=2 => [1.0, 0.0, 0.0],
                    3..=5 => [0.0, 1.0, 0.0],
                    6 => [0.6, 0.5, 0.6],
                    _ => [0.5, 0.6, -0.6],
                },
            )
        })
        .collect();
    let result =
        form_discovery_cohorts(&samples, &matcher(true), policy(), &AtomicBool::new(false))
            .unwrap();
    assert_eq!(result.cohorts.len(), 2);
    assert_eq!(
        result.cohorts[0].comparison_contents(),
        &[samples[6].content.clone()]
    );
    assert_eq!(
        result.cohorts[1].comparison_contents(),
        &[samples[7].content.clone()]
    );
}

#[test]
fn missing_comparison_and_cohort_capacity_keep_explicit_deferrals() {
    let one: Vec<_> = (0..3).map(|index| sample(index, [1.0, 0.0, 0.0])).collect();
    let result =
        form_discovery_cohorts(&one, &matcher(false), policy(), &AtomicBool::new(false)).unwrap();
    assert!(result.cohorts.is_empty());
    assert_eq!(
        result.deferred[0].reason,
        DiscoveryCohortDeferral::MissingComparison
    );
    let all: Vec<_> = one
        .into_iter()
        .chain((3..6).map(|index| sample(index, [0.0, 1.0, 0.0])))
        .collect();
    let limited = form_discovery_cohorts(
        &all,
        &matcher(false),
        DiscoveryCohortPolicy {
            maximum_cohorts: 1,
            ..policy()
        },
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(limited.cohorts.len(), 1);
    assert_eq!(
        limited.deferred[0].reason,
        DiscoveryCohortDeferral::CohortLimit
    );
    assert_eq!(limited.deferred[0].members.len(), 3);
}

#[test]
fn empty_windows_are_idle_and_invalid_windows_are_rejected() {
    let empty =
        form_discovery_cohorts(&[], &matcher(false), policy(), &AtomicBool::new(false)).unwrap();
    assert!(empty.cohorts.is_empty() && empty.deferred.is_empty());
    let valid = sample(1, [1.0, 0.0, 0.0]);
    assert!(
        form_discovery_cohorts(
            &[valid.clone(), valid.clone()],
            &matcher(false),
            policy(),
            &AtomicBool::new(false)
        )
        .is_err()
    );
    for invalid in [
        DiscoveryCohortPolicy {
            minimum_pair_cosine: f32::NAN,
            ..policy()
        },
        DiscoveryCohortPolicy {
            minimum_pair_cosine: -0.1,
            ..policy()
        },
        DiscoveryCohortPolicy {
            minimum_members: 1,
            ..policy()
        },
        DiscoveryCohortPolicy {
            maximum_cohorts: 65,
            ..policy()
        },
    ] {
        assert!(
            form_discovery_cohorts(
                std::slice::from_ref(&valid),
                &matcher(false),
                invalid,
                &AtomicBool::new(false)
            )
            .is_err()
        );
    }
    let too_many: Vec<_> = (0..513)
        .map(|index| sample(index, [1.0, 0.0, 0.0]))
        .collect();
    assert!(
        form_discovery_cohorts(
            &too_many,
            &matcher(false),
            policy(),
            &AtomicBool::new(false)
        )
        .is_err()
    );
    let mut incompatible = valid;
    incompatible.feature.identity.pipeline = ClassificationRevision::from_bytes([9; 32]);
    assert!(
        form_discovery_cohorts(
            &[incompatible],
            &matcher(false),
            policy(),
            &AtomicBool::new(false)
        )
        .is_err()
    );
}

#[test]
fn cancellation_returns_no_partial_cohort_batch() {
    let samples: Vec<_> = (0..512)
        .map(|index| sample(index, [1.0, 0.0, 0.0]))
        .collect();
    let cancelled = std::sync::Arc::new(AtomicBool::new(false));
    let signal = cancelled.clone();
    let worker = std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(1));
        signal.store(true, std::sync::atomic::Ordering::Release);
    });
    let result = form_discovery_cohorts(&samples, &matcher(false), policy(), &cancelled);
    worker.join().unwrap();
    assert_eq!(result.err().unwrap().code, "classification_cancelled");
}
