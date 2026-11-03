use super::*;

#[test]
fn a_broader_context_rejected_by_competition_cannot_erase_its_supported_child() {
    let mut harness = Harness::new();
    harness.calls.features.borrow_mut().extend([
        ("a workshop".into(), vec![1.0, 0.0]),
        ("a classroom".into(), vec![0.6, 0.8]),
    ]);

    let nomination = harness
        .nominate(1, context_limits(), &AtomicBool::new(false))
        .unwrap();
    assert_eq!(nomination.contexts.candidates.len(), 1);
    assert_eq!(
        nomination.contexts.candidates[0].candidate.concept.id(),
        id(1)
    );
    let assessment = assess_category_candidates(
        &harness.cohort,
        &nomination.contexts,
        &harness.matcher,
        &harness.policy,
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(assessment.candidates[0].supporting_members, 2);
    assert_eq!(
        assessment.candidates[0].disposition,
        CategoryAdmissionDisposition::Proposed
    );
    assert_eq!(nomination.subsumed_concepts, 0);
    assert!(harness.residency.resident_kind().is_none());
}

#[test]
fn bounded_evaluation_reports_omitted_candidates_without_growing_the_published_frontier() {
    let mut harness = Harness::new();
    harness.vocabulary.concepts.clear();
    harness.vocabulary.links.clear();
    for identity in 1..=67 {
        let description = format!("context {identity}");
        harness
            .calls
            .features
            .borrow_mut()
            .insert(description.clone(), vec![1.0, 0.0]);
        harness.vocabulary.concepts.insert(
            id(identity),
            VocabularyConcept::new(id(identity), description.clone(), description).unwrap(),
        );
    }
    harness
        .vocabulary
        .links
        .insert(id(1), (2..=67).map(id).collect());
    harness.retrieved.nearest = vec![ConceptMatch {
        concept: harness.vocabulary.concepts[&id(1)].clone(),
        scores: harness.retrieved.nearest[0].scores,
    }];
    harness.retrieved.distinctive = harness.retrieved.nearest.clone();
    let result = harness
        .nominate(
            1,
            ConceptConsolidationLimits {
                concepts: 128,
                edges: 256,
                hops: 4,
            },
            &AtomicBool::new(false),
        )
        .unwrap();
    assert_eq!(result.eligible_concepts, 67);
    assert!(result.candidate_limit_reached);
    assert_eq!(result.contexts.candidates.len(), 1);
    assert_eq!(result.contexts.candidates[0].candidate.concept.id(), id(2));
    assert!(harness.residency.resident_kind().is_none());
}
