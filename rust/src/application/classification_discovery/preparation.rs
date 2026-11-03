use std::sync::{Arc, atomic::AtomicBool};

use crate::domain::ScanError;
use crate::domain::classification::ClassificationRecordId;
use crate::domain::classification::discovery::CategoryNaming;
use crate::domain::classification::maintenance::CategoryExpansionPlan;
use crate::ports::classification_configuration::{
    CategoryCoverageRepository, CategoryExpansionRepository,
};
use crate::ports::classification_encoder::ClassificationEncoderFactory;
use crate::ports::classification_features::ClassificationFeatureRepository;
use crate::ports::classification_vocabulary::ClassificationVocabulary;

use super::super::classification_features::prepare_classification_matcher;
use super::super::classification_residency::{ClassificationEncoderResidency, ResidencyClock};
use super::{
    CategoryAdmissionPolicy, CategoryExpansionBudget, CategoryExpansionRequest,
    ConceptConsolidationLimits, ConceptNominationRequest, ConceptSearchLimits,
    DiscoveryCohortPolicy, DiscoverySampleCursor, check_cancelled, form_discovery_cohorts,
    nominate_classification_contexts, prepare_category_expansion, read_discovery_samples,
    retrieve_classification_concepts, survey_category_coverage,
};

pub struct DiscoveryPreparationPolicy<'a> {
    pub maximum_contents: usize,
    pub cohorts: DiscoveryCohortPolicy,
    pub retrieval: ConceptSearchLimits,
    pub consolidation: ConceptConsolidationLimits,
    pub maximum_candidates: usize,
    pub minimum_context_relatedness: f32,
    pub admission: &'a CategoryAdmissionPolicy,
    pub naming: &'a [CategoryNaming],
}

pub struct DiscoveryPreparationRequest<'a> {
    pub operation_id: ClassificationRecordId,
    pub cursor: Option<&'a DiscoverySampleCursor>,
    pub policy: &'a DiscoveryPreparationPolicy<'a>,
}

pub struct PreparedDiscoveryWindow {
    pub examined_contents: usize,
    pub missing_features: usize,
    pub considered_cohorts: usize,
    pub deferred_cohorts: usize,
    pub bounded_cohorts: usize,
    pub next: Option<DiscoverySampleCursor>,
    pub expansion: Option<CategoryExpansionPlan>,
}

pub fn prepare_discovery_window<F: ClassificationEncoderFactory, C: ResidencyClock>(
    request: DiscoveryPreparationRequest<'_>,
    repository: &(impl CategoryCoverageRepository + CategoryExpansionRepository),
    vocabulary: &mut impl ClassificationVocabulary,
    residency: &mut ClassificationEncoderResidency<F, C>,
    cache: &mut impl ClassificationFeatureRepository,
    cancelled: &Arc<AtomicBool>,
) -> Result<PreparedDiscoveryWindow, ScanError> {
    let policy = request.policy;
    validate_policy(policy)?;
    let window = read_discovery_samples(
        repository,
        cache,
        request.cursor,
        policy.maximum_contents,
        cancelled,
    )?;
    let mut result = PreparedDiscoveryWindow {
        examined_contents: window.examined_contents,
        missing_features: window.missing_features,
        considered_cohorts: 0,
        deferred_cohorts: 0,
        bounded_cohorts: 0,
        next: window.next,
        expansion: None,
    };
    if window.snapshot.model != residency.compatibility() {
        return Err(changed());
    }
    let taxonomy = window
        .snapshot
        .configuration
        .catalog
        .taxonomy
        .as_ref()
        .ok_or_else(changed)?;
    for naming in policy.naming {
        naming.validate(taxonomy).map_err(|_| changed())?;
    }
    if window.samples.len() < policy.cohorts.minimum_members {
        return Ok(result);
    }
    let matcher = prepare_classification_matcher(taxonomy.clone(), residency, cache, cancelled)?;
    let groups = form_discovery_cohorts(&window.samples, &matcher, policy.cohorts, cancelled)?;
    result.deferred_cohorts = groups.deferred.len();
    if groups.cohorts.is_empty() {
        repository.validate_category_coverage(&window.snapshot, cancelled)?;
        check_cancelled(cancelled)?;
        return Ok(result);
    }
    let coverage = survey_category_coverage(repository, 64, 1, cancelled)?;
    if coverage.snapshot != window.snapshot {
        return Err(changed());
    }
    'cohorts: for cohort in &groups.cohorts {
        check_cancelled(cancelled)?;
        result.considered_cohorts += 1;
        let retrieved = retrieve_classification_concepts(
            cohort,
            vocabulary,
            policy.retrieval,
            residency,
            cache,
            cancelled,
        )?;
        if !retrieved.bounds_reached.is_empty() {
            result.deferred_cohorts += 1;
            result.bounded_cohorts += 1;
            continue;
        }
        for naming in policy.naming {
            let nominated = nominate_classification_contexts(
                ConceptNominationRequest {
                    cohort,
                    retrieved: &retrieved,
                    matcher: &matcher,
                    policy: policy.admission,
                    limits: policy.consolidation,
                    max_candidates: policy.maximum_candidates,
                    minimum_context_relatedness: policy.minimum_context_relatedness,
                    naming,
                },
                vocabulary,
                residency,
                cache,
                cancelled,
            );
            let nominated = match nominated {
                Ok(nominated) => nominated,
                Err(error) if error.code == "classification_discovery_limit" => {
                    result.deferred_cohorts += 1;
                    result.bounded_cohorts += 1;
                    continue 'cohorts;
                }
                Err(error) => return Err(error),
            };
            let assessment = prepare_category_expansion(
                CategoryExpansionRequest {
                    operation_id: request.operation_id.clone(),
                    cohort,
                    discovery: &nominated.contexts,
                    matcher: &matcher,
                    policy: policy.admission,
                    budget: CategoryExpansionBudget::VerifiedContents(&coverage),
                },
                repository,
                cancelled,
            )?;
            if let Some(plan) = assessment.plan {
                result.expansion = Some(plan);
                break 'cohorts;
            }
        }
    }
    repository.validate_category_coverage(&window.snapshot, cancelled)?;
    check_cancelled(cancelled)?;
    Ok(result)
}

fn validate_policy(policy: &DiscoveryPreparationPolicy<'_>) -> Result<(), ScanError> {
    policy.retrieval.validate()?;
    if !(1..=512).contains(&policy.maximum_contents)
        || !(2..=512).contains(&policy.cohorts.minimum_members)
        || !(1..=64).contains(&policy.cohorts.maximum_cohorts)
        || !policy.cohorts.minimum_pair_cosine.is_finite()
        || !(0.0..=1.0).contains(&policy.cohorts.minimum_pair_cosine)
        || !(1..=64).contains(&policy.naming.len())
        || !(1..=64).contains(&policy.maximum_candidates)
        || !policy.minimum_context_relatedness.is_finite()
        || !(0.0..=1.0).contains(&policy.minimum_context_relatedness)
        || policy.consolidation.hops > 4
        || !(1..=4096).contains(&policy.consolidation.concepts)
        || !(1..=65_536).contains(&policy.consolidation.edges)
    {
        return Err(ScanError::new(
            "classification_discovery_policy_invalid",
            "Discovery preparation requires bounded compatible policies",
        ));
    }
    Ok(())
}

fn changed() -> ScanError {
    ScanError::new(
        "classification_discovery_changed",
        "The classification settings or contents changed during discovery preparation",
    )
}
