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

fn shared_prefix_harness() -> Harness {
    let mut harness = purpose_harness();
    harness.vocabulary.concepts.insert(
        id(5),
        VocabularyConcept::new(id(5), "人物".into(), "a portrait of a person".into())
            .unwrap()
            .with_subject("people".into())
            .unwrap(),
    );
    harness
        .vocabulary
        .links
        .get_mut(&id(2))
        .unwrap()
        .push(id(5));
    harness.calls.features.borrow_mut().extend([
        ("a portrait of a person".into(), vec![0.0, 1.0]),
        (
            "a reaction sticker featuring people".into(),
            vec![0.9, 0.4358899],
        ),
    ]);
    let mut criteria = harness.policy.criteria();
    criteria.minimum_separation = 0.15;
    harness.policy = CategoryAdmissionPolicy::new(revision(10), criteria).unwrap();
    harness
}

#[test]
fn shared_prefix_does_not_erase_competition_between_different_topics() {
    let mut harness = shared_prefix_harness();
    let projected_relatedness = Embedding::normalized(vec![0.8, 0.6])
        .unwrap()
        .cosine(&Embedding::normalized(vec![0.9, 0.4358899]).unwrap())
        .unwrap();
    assert!(projected_relatedness > harness.minimum_relatedness);
    let nomination = harness
        .nominate(64, context_limits(), &AtomicBool::new(false))
        .unwrap();
    assert!(nomination.contexts.candidates.is_empty());
    let assessment = assess(&harness, &nomination).unwrap();
    assert!(assessment.candidates.is_empty());
    let encoded = harness.calls.encoded.borrow().len();
    let repeated = harness
        .nominate(64, context_limits(), &AtomicBool::new(false))
        .unwrap();
    assert_eq!(repeated.contexts.candidates, nomination.contexts.candidates);
    assert_eq!(harness.calls.encoded.borrow().len(), encoded);
    assert!(harness.residency.resident_kind().is_none());

    let mut criteria = harness.policy.criteria();
    criteria.minimum_separation = 0.05;
    harness.policy = CategoryAdmissionPolicy::new(revision(11), criteria).unwrap();
    let separated = harness
        .nominate(64, context_limits(), &AtomicBool::new(false))
        .unwrap();
    let candidate = &separated.contexts.candidates[0];
    assert_eq!(candidate.candidate.concept.id(), id(5));
    let alternative = candidate.comparison.as_ref().unwrap().members[0]
        .alternative
        .unwrap();
    assert_eq!(alternative.concept, id(3));
    assert_eq!(alternative.relatedness, 0.0);
    assert!((alternative.similarity - 0.8).abs() < 1e-6);
    let assessment = assess(&harness, &separated).unwrap();
    assert_eq!(
        assessment.candidates[0].disposition,
        CategoryAdmissionDisposition::Proposed
    );
    assert!((assessment.candidates[0].members[0].separation - 0.1).abs() < 1e-6);
    assert_eq!(harness.calls.encoded.borrow().len(), encoded);
}

#[test]
fn cancelled_topic_comparison_keeps_completed_qualified_features_and_retires() {
    let mut harness = shared_prefix_harness();
    harness.calls.cancel_at.set(Some(3));
    let error = harness
        .nominate(64, context_limits(), &AtomicBool::new(false))
        .err()
        .unwrap();
    assert_eq!(error.code, "classification_cancelled");
    assert_eq!(
        harness.calls.encoded.borrow().last().unwrap(),
        "a portrait of a person"
    );
    assert_eq!(harness.cache.feature_cache_stats().unwrap().entries, 2);
    assert_eq!(harness.calls.retires.get(), 1);
    assert!(harness.residency.resident_kind().is_none());
}

#[test]
fn retrieved_nouns_cannot_bypass_the_independent_context_name_and_description() {
    let mut harness = purpose_harness();
    let cancelled = AtomicBool::new(false);
    let source = std::mem::replace(&mut harness.vocabulary, vocabulary());
    let catalog = NamingContextCatalog::new(vec![NamingContext {
        concept: VocabularyConcept::new(id(20), "宠物猫".into(), "pet cats at home".into())
            .unwrap()
            .with_subject("pet cats".into())
            .unwrap(),
        cues: vec![id(3)],
        broader: vec![],
    }])
    .unwrap();
    let mut vocabulary = ContextualVocabulary::new(source, catalog, &cancelled);
    harness.calls.features.borrow_mut().insert(
        "a reaction sticker featuring pet cats".into(),
        vec![0.8, 0.6],
    );
    let retrieved = retrieve_classification_concepts(
        &harness.cohort,
        &mut vocabulary,
        limits(),
        &mut harness.residency,
        &mut harness.cache,
        &cancelled,
    )
    .unwrap();
    assert!(
        retrieved
            .nearest
            .iter()
            .any(|row| row.concept.id() == id(3))
    );
    assert!(
        retrieved
            .nearest
            .iter()
            .all(|row| row.concept.naming_scope() == ConceptNamingScope::TraversalOnly)
    );
    let result = nominate_classification_contexts(
        ConceptNominationRequest {
            cohort: &harness.cohort,
            retrieved: &retrieved,
            matcher: &harness.matcher,
            policy: &harness.policy,
            limits: context_limits(),
            max_candidates: 64,
            minimum_context_relatedness: 0.9,
            naming: &harness.naming,
        },
        &mut vocabulary,
        &mut harness.residency,
        &mut harness.cache,
        &cancelled,
    )
    .unwrap();
    assert_eq!(result.naming_concepts, 1);
    let candidate = &result.contexts.candidates[0];
    assert_eq!(candidate.candidate.concept.label(), "宠物猫表情");
    assert_eq!(
        candidate.candidate.concept.description(),
        "a reaction sticker featuring pet cats"
    );
    assert!((candidate.member_similarities[0].similarity - 0.8).abs() < 1e-6);
    let assessment = assess(&harness, &result).unwrap();
    assert_eq!(assessment.candidates[0].supporting_members, 2);
    assert_eq!(
        assessment.candidates[0].members[2].disposition,
        MemberAdmission::DifferentPurpose
    );
    assert!(harness.residency.resident_kind().is_none());
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
    assert_eq!(
        *harness.calls.encoded.borrow(),
        vec![DESCRIPTION, "a classroom"]
    );

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
    assert_eq!(
        *harness.calls.encoded.borrow(),
        vec![DESCRIPTION, "a classroom"]
    );
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
