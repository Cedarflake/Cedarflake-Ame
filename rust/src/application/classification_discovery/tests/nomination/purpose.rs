use crate::domain::classification::discovery::CategoryNamingPurpose;

use super::*;

const DESCRIPTION: &str = "a reaction sticker featuring cat";

fn purpose_harness() -> Harness {
    let mut harness = Harness::new();
    let categories = vec![
        CategoryDefinition::new(
            CategoryId::new("reactions".into()).unwrap(),
            "表情包".into(),
            vec!["a reaction image".into()],
        )
        .unwrap(),
        CategoryDefinition::new(
            CategoryId::new("photos".into()).unwrap(),
            "实拍".into(),
            vec!["a photograph".into()],
        )
        .unwrap(),
    ];
    harness.naming = CategoryNaming::WithinCategory(
        CategoryNamingPurpose::new(
            &categories[0],
            "{subject}表情".into(),
            "a reaction sticker featuring {subject}".into(),
        )
        .unwrap(),
    );
    harness.matcher = CategoryMatcher::new(
        Taxonomy::new(categories).unwrap(),
        model(),
        [
            ("a reaction image", vec![0.2, -0.98]),
            ("a photograph", vec![-0.98, 0.2]),
        ]
        .into_iter()
        .map(|(text, values)| TextFeature {
            identity: model().text_identity(),
            description: ClassificationRevision::description(text),
            embedding: Embedding::normalized(values).unwrap(),
        })
        .collect(),
    )
    .unwrap();
    harness.vocabulary.concepts.insert(
        id(3),
        VocabularyConcept::new(id(3), "猫".into(), "a classroom".into())
            .unwrap()
            .with_subject("cat".into())
            .unwrap(),
    );
    harness
        .calls
        .features
        .borrow_mut()
        .insert(DESCRIPTION.into(), vec![0.8, 0.6]);
    harness
}

fn assess(harness: &Harness, result: &ConceptNomination) -> Result<CategoryAdmission, ScanError> {
    assess_category_candidates(
        &harness.cohort,
        &result.contexts,
        &harness.matcher,
        &harness.policy,
        &AtomicBool::new(false),
    )
}

#[test]
fn composed_names_are_scored_independently_and_cannot_claim_a_different_image_purpose() {
    let mut harness = purpose_harness();
    let first = harness
        .nominate(64, context_limits(), &AtomicBool::new(false))
        .unwrap();
    assert_eq!(first.considered_concepts, 4);
    assert_eq!(first.naming_concepts, 1);
    assert_eq!(first.contexts.candidates.len(), 1);
    let candidate = &first.contexts.candidates[0];
    assert_eq!(candidate.candidate.concept.label(), "猫表情");
    assert_eq!(candidate.candidate.concept.description(), DESCRIPTION);
    assert!((candidate.member_similarities[0].similarity - 0.8).abs() < 1e-6);
    assert_eq!(candidate.narrower_concepts, vec![id(1), id(4)]);
    assert_eq!(
        first.contexts.vocabulary_revision,
        harness.retrieved.vocabulary_revision
    );
    assert_eq!(
        first.contexts.retrieval_revision,
        harness.retrieved.retrieval_revision
    );
    assert_eq!(*harness.calls.encoded.borrow(), vec![DESCRIPTION]);

    let assessment = assess(&harness, &first).unwrap();
    assert_eq!(assessment.naming, harness.naming);
    assert_eq!(assessment.candidates[0].supporting_members, 2);
    assert_eq!(
        assessment.candidates[0].disposition,
        CategoryAdmissionDisposition::Proposed
    );
    let different = &assessment.candidates[0].members[2];
    assert!(different.improvement > 0.0 && different.separation > 0.0);
    assert_eq!(different.current.category_id.as_str(), "photos");
    assert_eq!(different.disposition, MemberAdmission::DifferentPurpose);

    let repeated = harness
        .nominate(64, context_limits(), &AtomicBool::new(false))
        .unwrap();
    assert_eq!(repeated.contexts.candidates, first.contexts.candidates);
    assert_eq!(assess(&harness, &repeated).unwrap(), assessment);
    assert_eq!(*harness.calls.encoded.borrow(), vec![DESCRIPTION]);
    assert_eq!(harness.calls.retires.get(), 1);
    assert!(harness.residency.resident_kind().is_none());
}

#[test]
fn an_out_of_purpose_majority_cannot_satisfy_nomination_support() {
    let mut harness = purpose_harness();
    let photo = harness
        .matcher
        .taxonomy()
        .categories()
        .iter()
        .find(|category| category.id().as_str() == "photos")
        .unwrap();
    harness.naming = CategoryNaming::WithinCategory(
        CategoryNamingPurpose::new(
            photo,
            "{subject}照片".into(),
            "a reaction sticker featuring {subject}".into(),
        )
        .unwrap(),
    );
    let result = harness
        .nominate(64, context_limits(), &AtomicBool::new(false))
        .unwrap();
    assert_eq!(result.eligible_concepts, 0);
    assert!(result.contexts.candidates.is_empty());
    assert_eq!(result.contexts.unresolved_concepts, vec![id(1), id(4)]);
}

#[test]
fn stale_parent_or_raw_description_cannot_be_assessed_as_a_purpose_qualified_name() {
    let mut harness = purpose_harness();
    let mut result = harness
        .nominate(64, context_limits(), &AtomicBool::new(false))
        .unwrap();
    result.contexts.candidates[0].candidate.concept = harness.vocabulary.concepts[&id(3)].clone();
    assert_eq!(
        assess(&harness, &result).unwrap_err().code,
        "classification_discovery_evidence_invalid"
    );
    harness.matcher = matcher("a different meaning", vec![0.0, 1.0]);
    assert_eq!(
        assess(&harness, &result).unwrap_err().code,
        "classification_naming_purpose_changed"
    );
    let encoded = harness.calls.encoded.borrow().len();
    let error = harness
        .nominate(64, context_limits(), &AtomicBool::new(false))
        .err()
        .unwrap();
    assert_eq!(error.code, "classification_naming_purpose_changed");
    assert_eq!(harness.calls.encoded.borrow().len(), encoded);
}

#[test]
fn cancellation_of_composed_description_work_retires_its_worker() {
    let mut harness = purpose_harness();
    harness.calls.cancel_at.set(Some(1));
    let error = harness
        .nominate(64, context_limits(), &AtomicBool::new(false))
        .err()
        .unwrap();
    assert_eq!(error.code, "classification_cancelled");
    assert_eq!(harness.calls.retires.get(), 1);
    assert!(harness.residency.resident_kind().is_none());
    assert_eq!(harness.cache.feature_cache_stats().unwrap().entries, 0);
}
