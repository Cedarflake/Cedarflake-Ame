use std::sync::atomic::AtomicBool;

use crate::domain::ScanError;
use crate::domain::classification::discovery::{CategoryNaming, ConceptId, DiscoveryCohort};
use crate::domain::classification::maintenance::CategoryExpansionPlan;
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
    for category in proposed {
        if snapshot.known_categories.contains(category.id()) {
            result.already_known.push(category.id().clone());
        } else if categories.len() == 256 {
            result.capacity_deferred.push(category.id().clone());
        } else {
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
    });
    Ok(result)
}

fn managed_category_id(
    concept: ConceptId,
    naming: &CategoryNaming,
) -> Result<CategoryId, ScanError> {
    let purpose = match naming {
        CategoryNaming::Unconditioned => "",
        CategoryNaming::WithinCategory(purpose) => purpose.category().as_str(),
    };
    let identity = ClassificationRevision::derive(
        "ame.classification.managed-category.v1",
        [concept.bytes().as_slice(), purpose.as_bytes()],
    );
    CategoryId::new(
        blake3::Hash::from_bytes(*identity.bytes())
            .to_hex()
            .to_string(),
    )
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
