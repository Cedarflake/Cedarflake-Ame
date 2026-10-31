use std::collections::BTreeSet;
use std::sync::atomic::AtomicBool;

use crate::domain::ScanError;
use crate::domain::classification::discovery::{ConceptId, DiscoveryCohort};
use crate::domain::classification::{ClassificationRevision, ModelCompatibility};
use crate::ports::classification_encoder::ClassificationEncoderFactory;
use crate::ports::classification_features::ClassificationFeatureRepository;
use crate::ports::classification_vocabulary::ClassificationVocabulary;

use super::super::classification_features::load_or_encode_descriptions;
use super::super::classification_residency::{ClassificationEncoderResidency, ResidencyClock};
use super::{
    ConceptMatch, ConceptSearchLimits, DiscoveryCandidates, check_cancelled, check_revision,
};

mod frontier;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ConceptConsolidationLimits {
    pub hops: usize,
    pub concepts: usize,
    pub edges: usize,
}

#[derive(Clone, Copy)]
pub struct ConceptConsolidationRequest<'a> {
    pub cohort: &'a DiscoveryCohort,
    pub retrieved: &'a DiscoveryCandidates,
    pub frontier: &'a BTreeSet<ConceptId>,
    pub limits: ConceptConsolidationLimits,
}

#[derive(Debug, PartialEq)]
pub struct ConsolidatedConcept {
    pub candidate: ConceptMatch,
    pub narrower_concepts: Vec<ConceptId>,
}

pub struct ConceptConsolidation {
    pub model: ModelCompatibility,
    pub cohort_revision: ClassificationRevision,
    pub vocabulary_revision: ClassificationRevision,
    pub retrieval_revision: ClassificationRevision,
    pub retrieval_limits: ConceptSearchLimits,
    pub frontier_revision: ClassificationRevision,
    pub consolidation_revision: ClassificationRevision,
    pub limits: ConceptConsolidationLimits,
    pub candidates: Vec<ConsolidatedConcept>,
    pub unresolved_concepts: Vec<ConceptId>,
    pub visited_edges: usize,
}

pub fn consolidate_classification_concepts<F: ClassificationEncoderFactory, C: ResidencyClock>(
    request: ConceptConsolidationRequest<'_>,
    vocabulary: &mut impl ClassificationVocabulary,
    residency: &mut ClassificationEncoderResidency<F, C>,
    cache: &mut impl ClassificationFeatureRepository,
    cancelled: &AtomicBool,
) -> Result<ConceptConsolidation, ScanError> {
    let result = consolidate(request, vocabulary, residency, cache, cancelled);
    residency.finish_descriptions()?;
    result
}

fn consolidate<F: ClassificationEncoderFactory, C: ResidencyClock>(
    request: ConceptConsolidationRequest<'_>,
    vocabulary: &mut impl ClassificationVocabulary,
    residency: &mut ClassificationEncoderResidency<F, C>,
    cache: &mut impl ClassificationFeatureRepository,
    cancelled: &AtomicBool,
) -> Result<ConceptConsolidation, ScanError> {
    let ConceptConsolidationRequest {
        cohort,
        retrieved,
        frontier,
        limits,
    } = request;
    check_cancelled(cancelled)?;
    if frontier.is_empty()
        || frontier.len() > 64
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
        || cohort.revision() != retrieved.cohort_revision
    {
        return Err(ScanError::new(
            "classification_feature_mismatch",
            "The consolidation evidence belongs to another cohort or encoder",
        ));
    }
    let revision = retrieved.vocabulary_revision;
    check_revision(vocabulary, revision)?;
    let resolved = frontier::resolve(request, vocabulary, cancelled)?;
    let descriptions: Vec<_> = resolved
        .groups
        .keys()
        .map(|id| resolved.concepts[id].description())
        .collect();
    let features = load_or_encode_descriptions(&descriptions, residency, cache, cancelled)?;
    let mut candidates = Vec::new();
    for (id, narrower) in resolved.groups {
        check_cancelled(cancelled)?;
        let concept = &resolved.concepts[&id];
        let feature = &features[&ClassificationRevision::description(concept.description())];
        let scores = cohort.score(feature).map_err(|error| {
            ScanError::new("classification_feature_mismatch", error.to_string())
        })?;
        candidates.push(ConsolidatedConcept {
            candidate: ConceptMatch {
                concept: concept.clone(),
                scores,
            },
            narrower_concepts: narrower.into_iter().collect(),
        });
    }
    candidates.sort_by(|left, right| {
        right
            .candidate
            .scores
            .contrast
            .total_cmp(&left.candidate.scores.contrast)
            .then_with(|| {
                right
                    .candidate
                    .scores
                    .similarity
                    .total_cmp(&left.candidate.scores.similarity)
            })
            .then_with(|| {
                left.candidate
                    .concept
                    .id()
                    .cmp(&right.candidate.concept.id())
            })
    });
    check_cancelled(cancelled)?;
    check_revision(vocabulary, revision)?;
    Ok(ConceptConsolidation {
        model: cohort.model(),
        cohort_revision: cohort.revision(),
        vocabulary_revision: revision,
        retrieval_revision: retrieved.retrieval_revision,
        retrieval_limits: retrieved.limits,
        frontier_revision: ClassificationRevision::derive(
            "ame.classification.naming-frontier.v1",
            frontier.iter().map(|id| id.bytes().as_slice()),
        ),
        consolidation_revision: ClassificationRevision::derive(
            "ame.classification.concept-consolidation.v1",
            std::iter::empty(),
        ),
        limits,
        candidates,
        unresolved_concepts: resolved.unresolved,
        visited_edges: resolved.visited_edges,
    })
}

fn limit_error() -> ScanError {
    ScanError::new(
        "classification_discovery_limit",
        "Concept consolidation requires complete retrieval within its resource limits",
    )
}
