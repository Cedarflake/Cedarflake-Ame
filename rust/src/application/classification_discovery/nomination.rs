use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::AtomicBool;

use crate::domain::ScanError;
use crate::domain::classification::ClassificationRevision;
use crate::domain::classification::discovery::{CategoryNaming, DiscoveryCohort};
use crate::ports::classification_encoder::ClassificationEncoderFactory;
use crate::ports::classification_features::ClassificationFeatureRepository;
use crate::ports::classification_vocabulary::ClassificationVocabulary;

use super::super::classification_matching::CategoryMatcher;
use super::super::classification_residency::{ClassificationEncoderResidency, ResidencyClock};
use super::context_graph::{self, limit_error};
use super::{
    CategoryAdmissionPolicy, ConceptConsolidation, ConceptConsolidationLimits,
    ContextComparisonPolicy, DiscoveryCandidates, check_cancelled, check_revision,
};

mod evaluation;
mod selection;
use evaluation::ContextEvaluationRequest;

pub struct ConceptNominationRequest<'a> {
    pub cohort: &'a DiscoveryCohort,
    pub retrieved: &'a DiscoveryCandidates,
    pub matcher: &'a CategoryMatcher,
    pub policy: &'a CategoryAdmissionPolicy,
    pub limits: ConceptConsolidationLimits,
    pub max_candidates: usize,
    pub minimum_context_relatedness: f32,
    pub naming: &'a CategoryNaming,
}

pub struct ConceptNomination {
    pub contexts: ConceptConsolidation,
    pub considered_concepts: usize,
    pub naming_concepts: usize,
    pub eligible_concepts: usize,
    pub subsumed_concepts: usize,
    pub candidate_limit_reached: bool,
    pub ancestry_edges: usize,
}

pub fn nominate_classification_contexts<F: ClassificationEncoderFactory, C: ResidencyClock>(
    request: ConceptNominationRequest<'_>,
    vocabulary: &mut impl ClassificationVocabulary,
    residency: &mut ClassificationEncoderResidency<F, C>,
    cache: &mut impl ClassificationFeatureRepository,
    cancelled: &AtomicBool,
) -> Result<ConceptNomination, ScanError> {
    let result = nominate(request, vocabulary, residency, cache, cancelled);
    residency.finish_descriptions()?;
    result
}

fn nominate<F: ClassificationEncoderFactory, C: ResidencyClock>(
    request: ConceptNominationRequest<'_>,
    vocabulary: &mut impl ClassificationVocabulary,
    residency: &mut ClassificationEncoderResidency<F, C>,
    cache: &mut impl ClassificationFeatureRepository,
    cancelled: &AtomicBool,
) -> Result<ConceptNomination, ScanError> {
    let ConceptNominationRequest {
        cohort,
        retrieved,
        matcher,
        policy,
        limits,
        max_candidates,
        minimum_context_relatedness,
        naming,
    } = request;
    check_cancelled(cancelled)?;
    if !(1..=64).contains(&max_candidates)
        || !minimum_context_relatedness.is_finite()
        || !(0.0..=1.0).contains(&minimum_context_relatedness)
        || limits.hops > 4
        || !(1..=4096).contains(&limits.concepts)
        || !(1..=65_536).contains(&limits.edges)
        || retrieved.nearest.len() > 64
        || retrieved.distinctive.len() > 64
        || !retrieved.bounds_reached.is_empty()
    {
        return Err(limit_error());
    }
    if cohort.model() != retrieved.model
        || cohort.model() != residency.compatibility()
        || cohort.model() != matcher.model()
        || cohort.revision() != retrieved.cohort_revision
    {
        return Err(ScanError::new(
            "classification_feature_mismatch",
            "Context nomination needs compatible retrieval, cohort and category features",
        ));
    }
    let revision = retrieved.vocabulary_revision;
    check_revision(vocabulary, revision)?;
    naming.validate(matcher.taxonomy()).map_err(naming_error)?;
    let graph = context_graph::collect(retrieved, limits, vocabulary, cancelled)?;
    let mut projected = BTreeMap::new();
    for concept in graph.concepts.values() {
        check_cancelled(cancelled)?;
        projected.insert(concept.id(), naming.project(concept));
    }
    let mut baseline = Vec::with_capacity(cohort.member_samples().len());
    for sample in cohort.member_samples() {
        check_cancelled(cancelled)?;
        baseline.push(
            matcher
                .rank(&sample.feature)
                .map_err(feature_error)?
                .leading()
                .clone(),
        );
    }
    let evaluated = evaluation::evaluate(
        ContextEvaluationRequest {
            cohort,
            graph: &graph,
            projected: &projected,
            baseline: &baseline,
            criteria: policy.criteria(),
            naming,
            minimum_relatedness: minimum_context_relatedness,
        },
        residency,
        cache,
        cancelled,
    )?;
    check_cancelled(cancelled)?;
    check_revision(vocabulary, revision)?;
    let selection = selection::select(
        evaluated.supported,
        &graph.parents,
        policy.criteria().minimum_members,
        max_candidates,
        limits.edges,
    )?;
    check_cancelled(cancelled)?;
    let selected: BTreeSet<_> = selection.selected.iter().copied().collect();
    let candidates: Vec<_> = evaluated
        .candidates
        .into_iter()
        .filter(|candidate| selected.contains(&candidate.candidate.concept.id()))
        .collect();
    check_cancelled(cancelled)?;
    check_revision(vocabulary, revision)?;
    let covered: BTreeSet<_> = candidates
        .iter()
        .flat_map(|row| row.narrower_concepts.iter().copied())
        .collect();
    let seeds: BTreeSet<_> = retrieved
        .nearest
        .iter()
        .chain(&retrieved.distinctive)
        .map(|row| row.concept.id())
        .collect();
    let taxonomy_revision = matcher.taxonomy().revision();
    let policy_revision = policy.revision();
    let capacity = (max_candidates as u64).to_le_bytes();
    let relatedness = minimum_context_relatedness.to_le_bytes();
    let naming_revision = naming.revision();
    let frontier_revision = ClassificationRevision::derive(
        "ame.classification.automatic-context-frontier.v8",
        [
            taxonomy_revision.bytes().as_slice(),
            policy_revision.bytes().as_slice(),
            capacity.as_slice(),
            relatedness.as_slice(),
            naming_revision.bytes().as_slice(),
        ]
        .into_iter()
        .chain(selected.iter().map(|id| id.bytes().as_slice())),
    );
    Ok(ConceptNomination {
        considered_concepts: graph.concepts.len(),
        naming_concepts: evaluated.naming_concepts,
        eligible_concepts: evaluated.eligible_concepts,
        subsumed_concepts: selection.subsumed,
        candidate_limit_reached: evaluated.limit_reached || selection.candidate_limit_reached,
        ancestry_edges: selection.ancestry_edges,
        contexts: ConceptConsolidation {
            naming: naming.clone(),
            comparison_policy: ContextComparisonPolicy::DistinctTopics {
                minimum_relatedness: minimum_context_relatedness,
            },
            model: cohort.model(),
            cohort_revision: cohort.revision(),
            vocabulary_revision: revision,
            retrieval_revision: retrieved.retrieval_revision,
            retrieval_limits: retrieved.limits,
            frontier_revision,
            consolidation_revision: ClassificationRevision::derive(
                "ame.classification.automatic-context-nomination.v8",
                std::iter::empty(),
            ),
            limits,
            candidates,
            unresolved_concepts: seeds.difference(&covered).copied().collect(),
            visited_edges: graph.visited_edges,
        },
    })
}

fn feature_error(error: impl std::fmt::Display) -> ScanError {
    ScanError::new("classification_feature_mismatch", error.to_string())
}

fn naming_error(error: impl std::fmt::Display) -> ScanError {
    ScanError::new("classification_naming_purpose_changed", error.to_string())
}
