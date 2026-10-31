use std::collections::BTreeSet;
use std::sync::atomic::AtomicBool;

use crate::domain::ScanError;
use crate::domain::classification::discovery::{
    ConceptId, ConceptMemberSimilarity, ConceptNamingScope, DiscoveryCohort,
};
use crate::domain::classification::{CategoryScore, ClassificationRevision};
use crate::ports::classification_encoder::ClassificationEncoderFactory;
use crate::ports::classification_features::{ClassificationFeatureRepository, TEXT_FEATURE_WINDOW};
use crate::ports::classification_vocabulary::ClassificationVocabulary;

use super::super::classification_features::load_or_encode_descriptions;
use super::super::classification_matching::CategoryMatcher;
use super::super::classification_residency::{ClassificationEncoderResidency, ResidencyClock};
use super::context_graph::{self, limit_error};
use super::{
    CategoryAdmissionCriteria, CategoryAdmissionPolicy, ConceptConsolidation,
    ConceptConsolidationLimits, ConceptMatch, ConsolidatedConcept, DiscoveryCandidates,
    check_cancelled, check_revision,
};

mod selection;
use selection::{ContextSupport, MemberCoverage};

pub struct ConceptNominationRequest<'a> {
    pub cohort: &'a DiscoveryCohort,
    pub retrieved: &'a DiscoveryCandidates,
    pub matcher: &'a CategoryMatcher,
    pub policy: &'a CategoryAdmissionPolicy,
    pub limits: ConceptConsolidationLimits,
    pub max_candidates: usize,
}

pub struct ConceptNomination {
    pub contexts: ConceptConsolidation,
    pub considered_concepts: usize,
    pub naming_concepts: usize,
    pub eligible_concepts: usize,
    pub subsumed_concepts: usize,
    pub candidate_limit_reached: bool,
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
    } = request;
    check_cancelled(cancelled)?;
    if !(1..=64).contains(&max_candidates)
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
    let graph = context_graph::collect(retrieved, limits, vocabulary, cancelled)?;
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
    let mut eligible = Vec::new();
    let concepts: Vec<_> = graph
        .concepts
        .values()
        .filter(|concept| concept.naming_scope() == ConceptNamingScope::Candidate)
        .collect();
    for window in concepts.chunks(TEXT_FEATURE_WINDOW) {
        check_cancelled(cancelled)?;
        check_revision(vocabulary, revision)?;
        let descriptions: Vec<_> = window.iter().map(|concept| concept.description()).collect();
        let features = load_or_encode_descriptions(&descriptions, residency, cache, cancelled)?;
        for concept in window {
            check_cancelled(cancelled)?;
            let feature = &features[&ClassificationRevision::description(concept.description())];
            let scores = cohort.score(feature).map_err(feature_error)?;
            let members = cohort.member_similarities(feature).map_err(feature_error)?;
            if let Some(context) = qualify(
                concept.id(),
                scores.contrast,
                &members,
                &baseline,
                policy.criteria(),
            ) {
                eligible.push(context);
            }
        }
    }
    check_cancelled(cancelled)?;
    check_revision(vocabulary, revision)?;
    let eligible_concepts = eligible.len();
    let selection = selection::select(
        eligible,
        &graph.parents,
        policy.criteria().minimum_members,
        max_candidates,
    );
    check_cancelled(cancelled)?;
    let selected: BTreeSet<_> = selection.selected.iter().copied().collect();
    let descriptions: Vec<_> = selection
        .selected
        .iter()
        .map(|id| graph.concepts[id].description())
        .collect();
    let features = load_or_encode_descriptions(&descriptions, residency, cache, cancelled)?;
    let mut candidates = Vec::with_capacity(selected.len());
    for id in &selection.selected {
        check_cancelled(cancelled)?;
        let concept = &graph.concepts[id];
        let feature = &features[&ClassificationRevision::description(concept.description())];
        candidates.push(ConsolidatedConcept {
            candidate: ConceptMatch {
                concept: concept.clone(),
                scores: cohort.score(feature).map_err(feature_error)?,
            },
            member_similarities: cohort.member_similarities(feature).map_err(feature_error)?,
            narrower_concepts: graph.groups[id].iter().copied().collect(),
        });
    }
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
    let frontier_revision = ClassificationRevision::derive(
        "ame.classification.automatic-context-frontier.v3",
        [
            taxonomy_revision.bytes().as_slice(),
            policy_revision.bytes().as_slice(),
            capacity.as_slice(),
        ]
        .into_iter()
        .chain(selected.iter().map(|id| id.bytes().as_slice())),
    );
    Ok(ConceptNomination {
        considered_concepts: graph.concepts.len(),
        naming_concepts: concepts.len(),
        eligible_concepts,
        subsumed_concepts: selection.subsumed,
        candidate_limit_reached: selection.candidate_limit_reached,
        contexts: ConceptConsolidation {
            model: cohort.model(),
            cohort_revision: cohort.revision(),
            vocabulary_revision: revision,
            retrieval_revision: retrieved.retrieval_revision,
            retrieval_limits: retrieved.limits,
            frontier_revision,
            consolidation_revision: ClassificationRevision::derive(
                "ame.classification.automatic-context-nomination.v3",
                std::iter::empty(),
            ),
            limits,
            candidates,
            unresolved_concepts: seeds.difference(&covered).copied().collect(),
            visited_edges: graph.visited_edges,
        },
    })
}

fn qualify(
    id: ConceptId,
    contrast: f32,
    members: &[ConceptMemberSimilarity],
    baseline: &[CategoryScore],
    criteria: CategoryAdmissionCriteria,
) -> Option<ContextSupport> {
    if contrast <= 0.0 || contrast < criteria.minimum_context_contrast {
        return None;
    }
    let mut coverage = MemberCoverage::default();
    let mut improvement = 0.0_f64;
    for (index, (member, current)) in members.iter().zip(baseline).enumerate() {
        let gain = member.similarity - current.similarity;
        if gain > 0.0 && gain >= criteria.minimum_improvement {
            coverage.insert(index);
            improvement += f64::from(gain);
        }
    }
    let supporting_members = coverage.count();
    if supporting_members < criteria.minimum_members
        || supporting_members * 1000 < baseline.len() * usize::from(criteria.coverage_per_thousand)
    {
        return None;
    }
    Some(ContextSupport {
        id,
        contrast,
        coverage,
        mean_improvement: (improvement / supporting_members as f64) as f32,
    })
}

fn feature_error(error: impl std::fmt::Display) -> ScanError {
    ScanError::new("classification_feature_mismatch", error.to_string())
}
