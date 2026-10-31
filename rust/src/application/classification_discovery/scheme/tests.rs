use crate::domain::classification::{
    ClassificationRecordId, Embedding, ImageFeature, Taxonomy, TextFeature,
};

use super::super::CategoryAdmissionCriteria;
use super::*;

type Definition<'a> = (&'a str, &'a str, [f32; 3]);
const BASE: Definition<'_> = ("base", "broad content", [0.0, 0.0, 1.0]);
const CAT: Definition<'_> = ("cat", "cats", [1.0, 0.0, 0.0]);
const DOG: Definition<'_> = ("dog", "dogs", [0.0, 1.0, 0.0]);

fn model() -> ModelCompatibility {
    ModelCompatibility::new(
        ClassificationRevision::from_bytes([1; 32]),
        ClassificationRevision::from_bytes([2; 32]),
        ClassificationRevision::from_bytes([3; 32]),
        3,
    )
    .unwrap()
}

fn matcher(definitions: &[Definition<'_>]) -> CategoryMatcher {
    let categories = definitions
        .iter()
        .map(|(id, description, _)| {
            CategoryDefinition::new(
                CategoryId::new((*id).into()).unwrap(),
                (*id).into(),
                vec![(*description).into()],
            )
            .unwrap()
        })
        .collect();
    let features: BTreeMap<_, _> = definitions
        .iter()
        .map(|(_, description, values)| {
            let revision = ClassificationRevision::description(description);
            (
                revision,
                TextFeature {
                    identity: model().text_identity(),
                    description: revision,
                    embedding: Embedding::normalized(values.to_vec()).unwrap(),
                },
            )
        })
        .collect();
    CategoryMatcher::new(
        Taxonomy::new(categories).unwrap(),
        model(),
        features.into_values().collect(),
    )
    .unwrap()
}

fn samples(cats: usize, dogs: usize) -> Vec<DiscoverySample> {
    (0..cats + dogs)
        .map(|index| DiscoverySample {
            content: ClassificationContent {
                scheme: ClassificationRecordId::new("fixture".into()).unwrap(),
                digest: *blake3::hash(&(index as u64).to_le_bytes()).as_bytes(),
                byte_length: 100,
            },
            feature: ImageFeature {
                identity: model().image_identity(),
                embedding: Embedding::normalized(if index < cats {
                    CAT.2.to_vec()
                } else {
                    DOG.2.to_vec()
                })
                .unwrap(),
            },
        })
        .collect()
}

fn policy(budget: usize) -> CategoryAdmissionPolicy {
    CategoryAdmissionPolicy::new(
        ClassificationRevision::from_bytes([4; 32]),
        CategoryAdmissionCriteria {
            minimum_members: 3,
            coverage_per_thousand: 500,
            minimum_improvement: 0.0,
            minimum_separation: 0.0,
            minimum_context_contrast: 0.0,
            max_proposals: budget,
        },
    )
    .unwrap()
}

fn assess(
    baseline: &[Definition<'_>],
    expanded: &[Definition<'_>],
    samples: &[DiscoverySample],
    budget: usize,
) -> CategorySchemeAssessment {
    assess_category_scheme(
        &matcher(baseline),
        &matcher(expanded),
        samples,
        &policy(budget),
        &AtomicBool::new(false),
    )
    .unwrap()
}

#[test]
fn joint_competition_keeps_distinctions_and_defers_a_redundant_broad_proposal() {
    let combined = ("combined", "cats and dogs", [1.0, 1.0, 0.0]);
    let result = assess(&[BASE], &[BASE, CAT, DOG, combined], &samples(3, 3), 8);
    let actual: Vec<_> = result
        .candidates
        .iter()
        .map(|candidate| (candidate.category_id.as_str(), &candidate.disposition))
        .collect();
    assert_eq!(
        actual,
        vec![
            ("cat", &CategorySchemeDisposition::Admitted),
            ("combined", &CategorySchemeDisposition::OverlappingProposal),
            ("dog", &CategorySchemeDisposition::Admitted),
        ]
    );
    assert!(
        result
            .members
            .iter()
            .all(|member| member.proposed_category.as_str() != "base")
    );
}

#[test]
fn sparse_candidates_wait_and_existing_categories_remain_in_the_baseline() {
    let result = assess(&[BASE], &[BASE, CAT, DOG], &samples(2, 3), 8);
    assert_eq!(
        result.candidates[0].disposition,
        CategorySchemeDisposition::InsufficientMembers
    );
    assert_eq!(
        result.candidates[1].disposition,
        CategorySchemeDisposition::Admitted
    );
    assert_eq!(
        result
            .members
            .iter()
            .filter(|member| member.proposed_category.as_str() == "base")
            .count(),
        2
    );
    let repeated = assess(
        &[BASE, DOG],
        &[BASE, DOG, ("another_dog", "dogs", DOG.2)],
        &samples(2, 3),
        8,
    );
    assert_eq!(
        repeated.candidates[0].disposition,
        CategorySchemeDisposition::ExistingDefinition(CategoryId::new("dog".into()).unwrap())
    );
    assert!(
        repeated
            .members
            .iter()
            .all(|member| member.proposed_category == member.previous_category)
    );
}

#[test]
fn duplicate_descriptions_share_support_and_do_not_consume_two_slots() {
    let result = assess(
        &[BASE],
        &[BASE, CAT, ("second_cat", "cats", CAT.2)],
        &samples(3, 0),
        8,
    );
    assert_eq!(
        result.candidates[0].disposition,
        CategorySchemeDisposition::Admitted
    );
    assert_eq!(result.candidates[0].unique_members, 3);
    assert_eq!(
        result.candidates[1].disposition,
        CategorySchemeDisposition::OverlappingProposal
    );
}

#[test]
fn budget_defers_the_lowest_marginal_value_without_retiring_existing_definitions() {
    let result = assess(&[BASE], &[BASE, CAT, DOG], &samples(4, 3), 1);
    assert_eq!(
        result.candidates[0].disposition,
        CategorySchemeDisposition::Admitted
    );
    assert_eq!(
        result.candidates[1].disposition,
        CategorySchemeDisposition::AdditionBudget
    );
    assert_eq!(
        result
            .members
            .iter()
            .filter(|member| member.proposed_category.as_str() == "cat")
            .count(),
        4
    );
    assert_eq!(
        result
            .members
            .iter()
            .filter(|member| member.proposed_category.as_str() == "base")
            .count(),
        3
    );
}

#[test]
fn ordering_is_stable_and_duplicate_content_cannot_inflate_support() {
    let mut input = samples(3, 3);
    let expected = assess(&[BASE], &[BASE, CAT, DOG], &input, 8);
    input.reverse();
    assert_eq!(assess(&[BASE], &[DOG, CAT, BASE], &input, 8), expected);
    input[1] = input[0].clone();
    assert_eq!(
        assess_category_scheme(
            &matcher(&[BASE]),
            &matcher(&[BASE, CAT, DOG]),
            &input,
            &policy(8),
            &AtomicBool::new(false)
        )
        .unwrap_err()
        .code,
        "classification_scheme_invalid"
    );
}

#[test]
fn assessment_revision_binds_the_previous_category_even_when_scores_match() {
    let mut input = samples(3, 0);
    let cats = assess(&[CAT, DOG], &[CAT, DOG, BASE], &input, 8);
    for sample in &mut input {
        sample.feature.embedding = Embedding::normalized(DOG.2.to_vec()).unwrap();
    }
    let dogs = assess(&[CAT, DOG], &[CAT, DOG, BASE], &input, 8);
    assert_eq!(cats.baseline_revision, dogs.baseline_revision);
    assert_eq!(cats.expanded_revision, dogs.expanded_revision);
    assert_eq!(cats.candidates, dogs.candidates);
    assert_ne!(cats.members, dogs.members);
    assert_ne!(cats.revision, dogs.revision);
}

#[test]
fn reserved_names_and_final_ties_follow_the_existing_matching_contract() {
    let reserved = assess(
        &[BASE, CAT],
        &[BASE, CAT, ("CAT", "different cats", DOG.2)],
        &samples(3, 3),
        8,
    );
    assert_eq!(
        reserved.candidates[0].disposition,
        CategorySchemeDisposition::NameConflict
    );
    let base = ("z_base", BASE.1, BASE.2);
    let mut input = samples(4, 0);
    input[3].feature.embedding = Embedding::normalized(vec![1.0, 0.0, 1.0]).unwrap();
    let result = assess(&[base], &[base, CAT], &input, 8);
    assert_eq!(result.candidates[0].unique_members, 3);
    let final_matcher = matcher(&[base, CAT]);
    for member in result.members {
        let sample = input
            .iter()
            .find(|sample| sample.content == member.content)
            .unwrap();
        assert_eq!(
            member.proposed_category,
            final_matcher
                .rank(&sample.feature)
                .unwrap()
                .leading()
                .category_id
        );
    }
}

#[test]
fn mismatched_definitions_features_empty_windows_and_cancellation_are_rejected() {
    let baseline = matcher(&[BASE]);
    let input = samples(3, 0);
    for changed in [
        ("base", "new meaning", BASE.2),
        ("base", "broad content", CAT.2),
    ] {
        assert_eq!(
            assess_category_scheme(
                &baseline,
                &matcher(&[changed, CAT]),
                &input,
                &policy(8),
                &AtomicBool::new(false)
            )
            .unwrap_err()
            .code,
            "classification_scheme_invalid"
        );
    }
    assert!(
        assess_category_scheme(
            &baseline,
            &matcher(&[BASE, CAT]),
            &[],
            &policy(8),
            &AtomicBool::new(false)
        )
        .is_err()
    );
    assert_eq!(
        assess_category_scheme(
            &baseline,
            &matcher(&[BASE, CAT]),
            &input,
            &policy(8),
            &AtomicBool::new(true)
        )
        .unwrap_err()
        .code,
        "classification_cancelled"
    );
}
