use std::sync::atomic::AtomicBool;

use crate::domain::ScanError;
use crate::domain::classification::discovery::{CategoryNaming, ConceptId, DiscoveryCohort};
use crate::domain::classification::maintenance::{
    CategoryExpansionBounds, CategoryExpansionCandidate, CategoryExpansionCriteria,
    CategoryExpansionPlan, CategoryExpansionProvenance,
};
use crate::domain::classification::work::ClassificationDispatchMode;
use crate::domain::classification::{
    CategoryDefinition, CategoryId, ClassificationAnalysisSetup, ClassificationRecordId,
    ClassificationRevision, ClassificationRun, Taxonomy,
};
use crate::domain::classification_configuration::{
    ClassificationConfigurationAction, ClassificationConfigurationChange,
};
use crate::ports::classification_configuration::CategoryExpansionRepository;

use super::super::classification_matching::CategoryMatcher;
use super::{
    CategoryAdmission, CategoryAdmissionDisposition, CategoryAdmissionPolicy, ConceptConsolidation,
    assess_category_candidates, check_cancelled,
};

pub struct CategoryExpansionRequest<'a> {
    pub operation_id: ClassificationRecordId,
    pub cohort: &'a DiscoveryCohort,
    pub discovery: &'a ConceptConsolidation,
    pub matcher: &'a CategoryMatcher,
    pub policy: &'a CategoryAdmissionPolicy,
}

pub struct CategoryExpansionAssessment {
    pub admission: CategoryAdmission,
    pub plan: Option<CategoryExpansionPlan>,
    pub already_known: Vec<CategoryId>,
    pub capacity_deferred: Vec<CategoryId>,
}

pub fn prepare_category_expansion(
    request: CategoryExpansionRequest<'_>,
    repository: &impl CategoryExpansionRepository,
    cancelled: &AtomicBool,
) -> Result<CategoryExpansionAssessment, ScanError> {
    let CategoryExpansionRequest {
        operation_id,
        cohort,
        discovery,
        matcher,
        policy,
    } = request;
    let admission = assess_category_candidates(cohort, discovery, matcher, policy, cancelled)?;
    let mut proposed = Vec::new();
    for candidate in &admission.candidates {
        if candidate.disposition != CategoryAdmissionDisposition::Proposed {
            continue;
        }
        let concept = discovery
            .candidates
            .iter()
            .find(|entry| entry.candidate.concept.id() == candidate.concept_id)
            .ok_or_else(stale)?
            .candidate
            .concept
            .clone();
        proposed.push(
            CategoryDefinition::new(
                managed_category_id(concept.id(), &discovery.naming)?,
                concept.label().to_owned(),
                vec![concept.description().to_owned()],
            )
            .map_err(|error| {
                ScanError::new("classification_category_invalid", error.to_string())
            })?,
        );
    }
    let mut result = CategoryExpansionAssessment {
        admission,
        plan: None,
        already_known: Vec::new(),
        capacity_deferred: Vec::new(),
    };
    if proposed.is_empty() {
        return Ok(result);
    }
    check_cancelled(cancelled)?;
    let identities: Vec<_> = proposed
        .iter()
        .map(|category| category.id().clone())
        .collect();
    let snapshot = repository.load_category_expansion_snapshot(&identities)?;
    let before = snapshot.configuration;
    if before.catalog.taxonomy.as_ref() != Some(matcher.taxonomy())
        || snapshot.model != Some(cohort.model())
        || before.dispatch.mode != ClassificationDispatchMode::Automatic
    {
        return Err(stale());
    }
    let mut categories = matcher.taxonomy().categories().to_vec();
    let mut additions = Vec::new();
    let mut candidates = Vec::new();
    for category in proposed {
        if snapshot.known_categories.contains(category.id()) {
            result.already_known.push(category.id().clone());
        } else if categories.len() == 256 {
            result.capacity_deferred.push(category.id().clone());
        } else {
            let admitted = result
                .admission
                .candidates
                .iter()
                .find(|candidate| {
                    managed_category_id(candidate.concept_id, &discovery.naming)
                        .is_ok_and(|id| &id == category.id())
                })
                .ok_or_else(stale)?;
            candidates.push(CategoryExpansionCandidate {
                category_id: category.id().clone(),
                concept_id: admitted.concept_id,
                supporting_contents: admitted
                    .members
                    .iter()
                    .filter(|member| member.disposition == super::MemberAdmission::Supports)
                    .map(|member| member.content.clone())
                    .collect(),
                mean_supporting_improvement: admitted.mean_supporting_improvement,
            });
            additions.push(category.id().clone());
            categories.push(category);
        }
    }
    if additions.is_empty() {
        return Ok(result);
    }
    let taxonomy = Taxonomy::new(categories).map_err(|_| stale())?;
    let run = ClassificationRun::new(
        operation_id.clone(),
        cohort.model(),
        taxonomy.revision(),
        None,
    )
    .map_err(|_| stale())?;
    let setup = ClassificationAnalysisSetup::new(run, taxonomy, None).map_err(|_| stale())?;
    check_cancelled(cancelled)?;
    result.plan = Some(CategoryExpansionPlan {
        configuration: ClassificationConfigurationChange {
            operation_id,
            expected_catalog_revision: before.catalog.revision,
            expected_dispatch_revision: before.dispatch.revision,
            setup,
            action: ClassificationConfigurationAction::Save,
        },
        additions,
        sources: cohort.source_contents(),
        evidence: evidence_identity(&result.admission),
        provenance: provenance(cohort, &result.admission, policy, candidates),
    });
    Ok(result)
}

fn provenance(
    cohort: &DiscoveryCohort,
    admission: &CategoryAdmission,
    policy: &CategoryAdmissionPolicy,
    candidates: Vec<CategoryExpansionCandidate>,
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
    }
}

fn managed_category_id(
    concept: ConceptId,
    naming: &CategoryNaming,
) -> Result<CategoryId, ScanError> {
    let purpose = match naming {
        CategoryNaming::Unconditioned => None,
        CategoryNaming::WithinCategory(purpose) => Some(purpose.category()),
    };
    crate::domain::classification::maintenance::managed_category_identity(concept, purpose)
        .map_err(|_| stale())
}

fn evidence_identity(admission: &CategoryAdmission) -> ClassificationRevision {
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
    ClassificationRevision::derive(
        "ame.classification.category-expansion-evidence.v1",
        identities
            .iter()
            .map(|identity| identity.bytes().as_slice())
            .chain(std::iter::once(parameters.as_slice())),
    )
}

fn stale() -> ScanError {
    ScanError::new(
        "classification_changed",
        "The category evidence or configuration changed",
    )
}

#[cfg(test)]
mod tests;
