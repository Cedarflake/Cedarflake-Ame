use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::AtomicBool;

use crate::domain::ScanError;
use crate::domain::classification::discovery::{
    CategoryNaming, ConceptId, ConceptNamingScope, DiscoveryCohort, VocabularyConcept,
};
use crate::domain::classification::{CategoryScore, ClassificationRevision};
use crate::ports::classification_features::{ClassificationFeatureRepository, TEXT_FEATURE_WINDOW};
use crate::ports::image_feature_encoding::ImageFeatureEncoding;

use super::super::super::classification_features::load_or_encode_descriptions;
use super::super::competition::{self, CompetitionRequest};
use super::super::context_graph::ResolvedContexts;
use super::super::{
    CategoryAdmissionCriteria, ConceptMatch, ConsolidatedConcept, MemberAdmission, check_cancelled,
};
use super::selection::{ContextSupport, MemberCoverage};

const EVALUATION_LIMIT: usize = 64;

pub(super) struct ContextEvaluationRequest<'a> {
    pub cohort: &'a DiscoveryCohort,
    pub graph: &'a ResolvedContexts,
    pub projected: &'a BTreeMap<ConceptId, VocabularyConcept>,
    pub baseline: &'a [CategoryScore],
    pub criteria: CategoryAdmissionCriteria,
    pub naming: &'a CategoryNaming,
    pub minimum_relatedness: f32,
}

pub(super) struct ContextEvaluation {
    pub candidates: Vec<ConsolidatedConcept>,
    pub supported: Vec<ContextSupport>,
    pub naming_concepts: usize,
    pub eligible_concepts: usize,
    pub limit_reached: bool,
}

pub(super) fn evaluate<E: ImageFeatureEncoding>(
    request: ContextEvaluationRequest<'_>,
    residency: &mut E,
    cache: &mut impl ClassificationFeatureRepository,
    cancelled: &AtomicBool,
) -> Result<ContextEvaluation, ScanError> {
    let concepts: Vec<_> = request
        .projected
        .values()
        .filter(|concept| concept.naming_scope() == ConceptNamingScope::Candidate)
        .collect();
    let naming_concepts = concepts
        .iter()
        .filter(|concept| request.naming.admits_description(concept))
        .count();
    if naming_concepts == 0 {
        return Ok(ContextEvaluation {
            candidates: Vec::new(),
            supported: Vec::new(),
            naming_concepts,
            eligible_concepts: 0,
            limit_reached: false,
        });
    }
    let mut eligible = Vec::new();
    for window in concepts.chunks(TEXT_FEATURE_WINDOW) {
        check_cancelled(cancelled)?;
        let descriptions: Vec<_> = window.iter().map(|concept| concept.description()).collect();
        let features = load_or_encode_descriptions(&descriptions, residency, cache, cancelled)?;
        for concept in window {
            check_cancelled(cancelled)?;
            let feature = &features[&ClassificationRevision::description(concept.description())];
            let scores = request.cohort.score(feature).map_err(feature_error)?;
            let members = request
                .cohort
                .member_similarities(feature)
                .map_err(feature_error)?;
            let mut coverage = MemberCoverage::default();
            let mut improvement = 0.0_f64;
            for (index, (member, current)) in members.iter().zip(request.baseline).enumerate() {
                let gain = member.similarity - current.similarity;
                if request.naming.includes(&current.category_id)
                    && gain > 0.0
                    && gain >= request.criteria.minimum_improvement
                {
                    coverage.insert(index);
                    improvement += f64::from(gain);
                }
            }
            if let Some(support) =
                request.support(concept.id(), scores.contrast, coverage, improvement)
            {
                eligible.push(support);
            }
        }
    }
    let eligible_ids: BTreeSet<_> = eligible.iter().map(|context| context.id).collect();
    eligible.retain(|context| {
        request
            .naming
            .admits_description(&request.projected[&context.id])
    });
    let eligible_concepts = eligible.len();
    eligible.sort_by(|left, right| {
        right
            .coverage
            .count()
            .cmp(&left.coverage.count())
            .then_with(|| right.mean_improvement.total_cmp(&left.mean_improvement))
            .then_with(|| right.contrast.total_cmp(&left.contrast))
            .then_with(|| left.id.cmp(&right.id))
    });
    eligible.truncate(EVALUATION_LIMIT);
    let descriptions: Vec<_> = eligible
        .iter()
        .map(|context| request.projected[&context.id].description())
        .collect();
    let features = load_or_encode_descriptions(&descriptions, residency, cache, cancelled)?;
    let mut candidates = Vec::with_capacity(eligible.len());
    for context in &eligible {
        check_cancelled(cancelled)?;
        let concept = &request.projected[&context.id];
        let feature = &features[&ClassificationRevision::description(concept.description())];
        candidates.push(ConsolidatedConcept {
            comparison: None,
            candidate: ConceptMatch {
                concept: concept.clone(),
                scores: request.cohort.score(feature).map_err(feature_error)?,
            },
            member_similarities: request
                .cohort
                .member_similarities(feature)
                .map_err(feature_error)?,
            narrower_concepts: request.graph.groups[&context.id].iter().copied().collect(),
        });
    }
    competition::retain(
        CompetitionRequest {
            cohort: request.cohort,
            qualified: request.projected,
            topics: &request.graph.concepts,
            eligible: &eligible_ids,
            minimum_relatedness: request.minimum_relatedness,
        },
        &mut candidates,
        residency,
        cache,
        cancelled,
    )?;
    let mut supported = Vec::new();
    for candidate in &candidates {
        check_cancelled(cancelled)?;
        let alternatives =
            competition::aligned(request.cohort, candidate, Some(request.minimum_relatedness))?;
        let mut coverage = MemberCoverage::default();
        let mut improvement = 0.0_f64;
        for (index, (member, current)) in candidate
            .member_similarities
            .iter()
            .zip(request.baseline)
            .enumerate()
        {
            let competing = alternatives[index].map_or(current.similarity, |alternative| {
                current.similarity.max(alternative.similarity)
            });
            if request.criteria.member_admission(
                request.naming,
                current,
                member.similarity,
                competing,
            ) == MemberAdmission::Supports
            {
                coverage.insert(index);
                improvement += f64::from(member.similarity - current.similarity);
            }
        }
        if let Some(support) = request.support(
            candidate.candidate.concept.id(),
            candidate.candidate.scores.contrast,
            coverage,
            improvement,
        ) {
            supported.push(support);
        }
    }
    Ok(ContextEvaluation {
        candidates,
        supported,
        naming_concepts,
        eligible_concepts,
        limit_reached: eligible_concepts > EVALUATION_LIMIT,
    })
}

impl ContextEvaluationRequest<'_> {
    fn support(
        &self,
        id: ConceptId,
        contrast: f32,
        coverage: MemberCoverage,
        improvement: f64,
    ) -> Option<ContextSupport> {
        let count = coverage.count();
        if contrast <= 0.0
            || contrast < self.criteria.minimum_context_contrast
            || count < self.criteria.minimum_members
            || count * 1000 < self.baseline.len() * usize::from(self.criteria.coverage_per_thousand)
        {
            return None;
        }
        Some(ContextSupport {
            id,
            contrast,
            coverage,
            mean_improvement: (improvement / count as f64) as f32,
        })
    }
}

fn feature_error(error: impl std::fmt::Display) -> ScanError {
    ScanError::new("classification_feature_mismatch", error.to_string())
}
