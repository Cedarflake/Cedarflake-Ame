use super::*;

pub(super) fn provenance(
    cohort: &DiscoveryCohort,
    admission: &CategoryAdmission,
    policy: &CategoryAdmissionPolicy,
    candidates: Vec<CategoryExpansionCandidate>,
    capacity: CategoryExpansionCapacity,
) -> CategoryExpansionProvenance {
    let criteria = policy.criteria();
    let retrieval = admission.retrieval_limits;
    let consolidation = admission.consolidation_limits;
    CategoryExpansionProvenance {
        naming: admission.naming.clone(),
        vocabulary: admission.vocabulary_revision,
        cohort: admission.cohort_revision,
        retrieval: admission.retrieval_revision,
        frontier: admission.frontier_revision,
        consolidation: admission.consolidation_revision,
        policy: admission.policy_revision,
        comparison_rules: admission.taxonomy_revision,
        comparison_names: admission.taxonomy_names_revision,
        criteria: CategoryExpansionCriteria {
            minimum_members: criteria.minimum_members,
            coverage_per_thousand: criteria.coverage_per_thousand,
            minimum_improvement: criteria.minimum_improvement,
            minimum_separation: criteria.minimum_separation,
            minimum_context_contrast: criteria.minimum_context_contrast,
            max_proposals: criteria.max_proposals,
            minimum_context_relatedness: admission.minimum_context_relatedness,
        },
        bounds: CategoryExpansionBounds {
            entry_points: retrieval.entry_points,
            expanded_concepts: retrieval.expanded_concepts,
            retrieval_edges: retrieval.edges,
            retrieval_hops: retrieval.hops,
            seeds: retrieval.seeds,
            results: retrieval.results,
            consolidation_hops: consolidation.hops,
            consolidation_concepts: consolidation.concepts,
            consolidation_edges: consolidation.edges,
        },
        members: cohort
            .member_samples()
            .iter()
            .map(|sample| sample.content.clone())
            .collect(),
        candidates,
        capacity,
    }
}

pub(super) fn evidence_identity(
    admission: &CategoryAdmission,
    capacity: &CategoryExpansionCapacity,
) -> ClassificationRevision {
    let identities = [
        admission.naming.revision(),
        admission.cohort_revision,
        admission.vocabulary_revision,
        admission.retrieval_revision,
        admission.frontier_revision,
        admission.consolidation_revision,
        admission.taxonomy_revision,
        admission.taxonomy_names_revision,
        admission.policy_revision,
    ];
    let limits = admission.retrieval_limits;
    let consolidation = admission.consolidation_limits;
    let mut parameters = Vec::new();
    for value in [
        limits.entry_points,
        limits.expanded_concepts,
        limits.edges,
        limits.hops,
        limits.seeds,
        limits.results,
        consolidation.hops,
        consolidation.concepts,
        consolidation.edges,
    ] {
        parameters.extend_from_slice(&(value as u64).to_le_bytes());
    }
    if let Some(relatedness) = admission.minimum_context_relatedness {
        parameters.push(1);
        parameters.extend_from_slice(&relatedness.to_le_bytes());
    } else {
        parameters.push(0);
    }
    parameters.extend_from_slice(&(capacity.maximum_generated as u64).to_le_bytes());
    for category in &capacity.active_generated {
        parameters.extend_from_slice(&(category.as_str().len() as u64).to_le_bytes());
        parameters.extend_from_slice(category.as_str().as_bytes());
    }
    ClassificationRevision::derive(
        "ame.classification.category-expansion-evidence.v2",
        identities
            .iter()
            .map(|identity| identity.bytes().as_slice())
            .chain(std::iter::once(parameters.as_slice())),
    )
}
