use crate::domain::classification::{
    CategoryDefinition, ClassificationRecordId, Embedding, ImageFeature, Taxonomy, TextFeature,
};

use super::*;

type Definition<'a> = (&'a str, &'a str, [f32; 3]);

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
        .map(|(id, label, _)| {
            CategoryDefinition::new(
                CategoryId::new((*id).into()).unwrap(),
                (*label).into(),
                vec![format!("Description {id}")],
            )
            .unwrap()
        })
        .collect();
    let features = definitions
        .iter()
        .map(|(id, _, values)| TextFeature {
            identity: model().text_identity(),
            description: ClassificationRevision::description(&format!("Description {id}")),
            embedding: Embedding::normalized(values.to_vec()).unwrap(),
        })
        .collect();
    CategoryMatcher::new(Taxonomy::new(categories).unwrap(), model(), features).unwrap()
}

fn sample(id: u64, values: [f32; 3]) -> DiscoverySample {
    DiscoverySample {
        content: ClassificationContent {
            scheme: ClassificationRecordId::new("fixture".into()).unwrap(),
            digest: *blake3::hash(&id.to_le_bytes()).as_bytes(),
            byte_length: 10,
        },
        feature: ImageFeature {
            identity: model().image_identity(),
            embedding: Embedding::normalized(values.to_vec()).unwrap(),
        },
    }
}

#[test]
fn counterfactual_matches_independent_rematching_after_each_single_removal() {
    let definitions = [
        ("a", "A", [1.0, 0.0, 0.0]),
        ("b", "B", [0.0, 1.0, 0.0]),
        ("c", "C", [-1.0, 0.0, 0.0]),
        ("d", "D", [0.0, -1.0, 0.0]),
    ];
    let samples = [
        sample(1, [0.9, 0.2, 0.1]),
        sample(2, [0.2, 0.9, 0.1]),
        sample(3, [-0.9, 0.2, 0.1]),
    ];
    let original = matcher(&definitions);
    let assessment =
        assess_category_withdrawals(&original, &samples, &AtomicBool::new(false)).unwrap();
    assert_eq!(assessment.sampled_contents, 3);
    assert_eq!(assessment.categories.len(), 4);
    assert!(
        assessment
            .categories
            .iter()
            .find(|impact| impact.category_id.as_str() == "d")
            .unwrap()
            .affected
            .is_empty()
    );
    let mut affected_total = 0;
    for impact in &assessment.categories {
        let retained: Vec<_> = definitions
            .iter()
            .copied()
            .filter(|(id, _, _)| *id != impact.category_id.as_str())
            .collect();
        let reduced = matcher(&retained);
        for sample in &samples {
            let before = original.rank(&sample.feature).unwrap();
            let after = reduced.rank(&sample.feature).unwrap();
            let affected = impact
                .affected
                .iter()
                .find(|member| member.content == sample.content);
            if before.leading().category_id == impact.category_id {
                let affected = affected.unwrap();
                assert_eq!(&affected.alternative, after.leading());
                assert_eq!(affected.removed_similarity, before.leading().similarity);
                assert!(affected.similarity_loss() > 0.0);
                affected_total += 1;
            } else {
                assert!(affected.is_none());
                assert_eq!(before.leading(), after.leading());
            }
        }
    }
    assert_eq!(affected_total, samples.len());
}

#[test]
fn ties_keep_canonical_replacements_and_zero_loss_without_retirement_authority() {
    let definitions = [
        ("c", "C", [-1.0, 0.0, 0.0]),
        ("b", "B", [1.0, 0.0, 0.0]),
        ("a", "A", [1.0, 0.0, 0.0]),
    ];
    let result = assess_category_withdrawals(
        &matcher(&definitions),
        &[sample(1, [1.0, 0.0, 0.0])],
        &AtomicBool::new(false),
    )
    .unwrap();
    let first = &result.categories[0];
    assert_eq!(first.category_id.as_str(), "a");
    assert_eq!(first.affected[0].alternative.category_id.as_str(), "b");
    assert_eq!(first.affected[0].similarity_loss(), 0.0);
    assert!(
        result.categories[1..]
            .iter()
            .all(|impact| impact.affected.is_empty())
    );
}

#[test]
fn revisions_bind_all_scores_labels_and_identities_but_not_input_order() {
    let definitions = [
        ("a", "A", [1.0, 0.0, 0.0]),
        ("b", "B", [0.0, 1.0, 0.0]),
        ("c", "C", [-1.0, 0.0, 0.0]),
    ];
    let mut samples = vec![sample(1, [0.9, 0.2, 0.0]), sample(2, [0.8, 0.1, 0.0])];
    let current = matcher(&definitions);
    let before = assess_category_withdrawals(&current, &samples, &AtomicBool::new(false)).unwrap();
    samples.reverse();
    let mut reversed_definitions = definitions;
    reversed_definitions.reverse();
    assert_eq!(
        before,
        assess_category_withdrawals(
            &matcher(&reversed_definitions),
            &samples,
            &AtomicBool::new(false)
        )
        .unwrap()
    );
    let mut renamed = definitions;
    renamed[0].1 = "Renamed";
    let renamed =
        assess_category_withdrawals(&matcher(&renamed), &samples, &AtomicBool::new(false)).unwrap();
    assert_eq!(renamed.categories, before.categories);
    assert_ne!(renamed.revision, before.revision);
    let mut changed_score = definitions;
    changed_score[2].2 = [-0.9, -0.1, 0.0];
    let changed =
        assess_category_withdrawals(&matcher(&changed_score), &samples, &AtomicBool::new(false))
            .unwrap();
    assert_eq!(changed.categories, before.categories);
    assert_ne!(changed.revision, before.revision);
    samples[0].content.byte_length += 1;
    assert_ne!(
        assess_category_withdrawals(&current, &samples, &AtomicBool::new(false))
            .unwrap()
            .revision,
        before.revision
    );
}

#[test]
fn malformed_windows_and_incompatible_authority_are_rejected() {
    let definitions = [("a", "A", [1.0, 0.0, 0.0]), ("b", "B", [0.0, 1.0, 0.0])];
    let current = matcher(&definitions);
    let valid = sample(1, [1.0, 0.0, 0.0]);
    let mut incompatible = valid.clone();
    incompatible.feature.identity.pipeline = ClassificationRevision::from_bytes([9; 32]);
    let mut wrong_dimensions = valid.clone();
    wrong_dimensions.feature.embedding = Embedding::normalized(vec![1.0, 0.0]).unwrap();
    for samples in [
        Vec::new(),
        vec![valid.clone(), valid.clone()],
        vec![incompatible],
        vec![wrong_dimensions],
        (0..513).map(|id| sample(id, [1.0, 0.0, 0.0])).collect(),
    ] {
        assert_eq!(
            assess_category_withdrawals(&current, &samples, &AtomicBool::new(false))
                .unwrap_err()
                .code,
            "classification_withdrawal_invalid"
        );
    }
    assert!(
        assess_category_withdrawals(
            &matcher(&definitions[..1]),
            std::slice::from_ref(&valid),
            &AtomicBool::new(false)
        )
        .is_err()
    );
    assert_eq!(
        assess_category_withdrawals(&current, &[valid], &AtomicBool::new(true))
            .unwrap_err()
            .code,
        "classification_cancelled"
    );
}
