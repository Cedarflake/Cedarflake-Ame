use std::sync::atomic::AtomicBool;

use crate::domain::ScanError;
use crate::domain::classification::discovery::{CategoryNaming, ConceptId, DiscoveryCohort};
use crate::domain::classification::maintenance::{
    CategoryActivation, CategoryExpansionBounds, CategoryExpansionCandidate,
    CategoryExpansionCapacity, CategoryExpansionCriteria, CategoryExpansionPlan,
    CategoryExpansionProvenance,
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
    CategoryAdmission, CategoryAdmissionDisposition, CategoryAdmissionPolicy,
    CategorySchemeCandidate, CategorySchemeDisposition, ConceptConsolidation,
    assess_category_candidates, check_cancelled,
};

mod provenance;
mod selection;

pub struct CategoryExpansionRequest<'a> {
    pub operation_id: ClassificationRecordId,
    pub cohort: &'a DiscoveryCohort,
    pub discovery: &'a ConceptConsolidation,
    pub matcher: &'a CategoryMatcher,
    pub policy: &'a CategoryAdmissionPolicy,
    pub maximum_generated_categories: usize,
}

pub struct CategoryExpansionAssessment {
    pub admission: CategoryAdmission,
    pub scheme_candidates: Vec<CategorySchemeCandidate>,
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
        maximum_generated_categories,
    } = request;
    check_cancelled(cancelled)?;
    if !(1..=256).contains(&maximum_generated_categories) {
        return Err(ScanError::new(
            "classification_category_limit",
            "The cumulative generated-category budget must be between 1 and 256",
        ));
    }
    let admission = assess_category_candidates(cohort, discovery, matcher, policy, cancelled)?;
    let mut proposed = selection::proposals(&admission, discovery)?;
    let mut result = CategoryExpansionAssessment {
        admission,
        scheme_candidates: Vec::new(),
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
    let before = &snapshot.configuration;
    if before.catalog.taxonomy.as_ref() != Some(matcher.taxonomy())
        || snapshot.model != Some(cohort.model())
        || before.dispatch.mode != ClassificationDispatchMode::Automatic
    {
        return Err(stale());
    }
    proposed.retain(|category| {
        if snapshot.known_categories.contains_key(category.id())
            && snapshot.reactivation_origin(category).is_none()
        {
            result.already_known.push(category.id().clone());
            false
        } else {
            true
        }
    });
    if proposed.is_empty() {
        return Ok(result);
    }
    let capacity = CategoryExpansionCapacity {
        maximum_generated: maximum_generated_categories,
        active_generated: snapshot.active_generated_categories(),
    };
    let mut selected = selection::select(
        &result.admission,
        &proposed,
        matcher.taxonomy(),
        &capacity,
        policy,
        cancelled,
    )?;
    result.capacity_deferred = selected
        .outcomes
        .iter()
        .filter(|entry| entry.disposition == CategorySchemeDisposition::AdditionBudget)
        .map(|entry| entry.category_id.clone())
        .collect();
    result.scheme_candidates = selected.outcomes;
    if selected.categories.is_empty() {
        return Ok(result);
    }
    for (candidate, category) in selected.candidates.iter_mut().zip(&selected.categories) {
        if let Some(origin) = snapshot.reactivation_origin(category) {
            candidate.activation = CategoryActivation::Reactivate(origin.clone());
        }
    }
    let evidence =
        provenance::evidence_identity(&result.admission, &capacity, &selected.candidates);
    let additions = selected
        .categories
        .iter()
        .map(|category| category.id().clone())
        .collect();
    let mut categories = matcher.taxonomy().categories().to_vec();
    categories.extend(selected.categories);
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
        evidence,
        provenance: provenance::provenance(
            cohort,
            &result.admission,
            policy,
            selected.candidates,
            capacity,
        ),
    });
    Ok(result)
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

fn stale() -> ScanError {
    ScanError::new(
        "classification_changed",
        "The category evidence or configuration changed",
    )
}

#[cfg(test)]
mod tests;
