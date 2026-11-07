use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::AtomicBool;

use crate::domain::ScanError;
use crate::domain::classification::discovery::{ConceptId, DiscoveryCohort, VocabularyConcept};
use crate::domain::classification::{ClassificationContent, ClassificationRevision};
use crate::ports::classification_features::{ClassificationFeatureRepository, TEXT_FEATURE_WINDOW};
use crate::ports::image_feature_encoding::ImageFeatureEncoding;

use super::super::classification_features::load_or_encode_descriptions;
use super::{ConsolidatedConcept, check_cancelled};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ContextComparisonPolicy {
    SelectedContexts,
    DistinctTopics { minimum_relatedness: f32 },
}

impl ContextComparisonPolicy {
    pub(super) fn relatedness(self) -> Result<Option<f32>, ScanError> {
        match self {
            Self::SelectedContexts => Ok(None),
            Self::DistinctTopics {
                minimum_relatedness,
            } if minimum_relatedness.is_finite() && (0.0..=1.0).contains(&minimum_relatedness) => {
                Ok(Some(minimum_relatedness))
            }
            Self::DistinctTopics { .. } => Err(invalid()),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ConceptAlternative {
    pub concept: ConceptId,
    pub similarity: f32,
    pub relatedness: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ContextMemberAlternative {
    pub content: ClassificationContent,
    pub alternative: Option<ConceptAlternative>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ContextComparison {
    pub minimum_relatedness: f32,
    pub members: Vec<ContextMemberAlternative>,
}

pub(super) struct CompetitionRequest<'a> {
    pub cohort: &'a DiscoveryCohort,
    pub qualified: &'a BTreeMap<ConceptId, VocabularyConcept>,
    pub topics: &'a BTreeMap<ConceptId, VocabularyConcept>,
    pub eligible: &'a BTreeSet<ConceptId>,
    pub minimum_relatedness: f32,
}

pub(super) fn retain<E: ImageFeatureEncoding>(
    request: CompetitionRequest<'_>,
    candidates: &mut [ConsolidatedConcept],
    residency: &mut E,
    cache: &mut impl ClassificationFeatureRepository,
    cancelled: &AtomicBool,
) -> Result<(), ScanError> {
    if candidates.is_empty() {
        return Ok(());
    }
    let selected_descriptions: Vec<_> = candidates
        .iter()
        .map(|candidate| request.topics[&candidate.candidate.concept.id()].description())
        .collect();
    let selected_topics =
        load_or_encode_descriptions(&selected_descriptions, residency, cache, cancelled)?;
    for candidate in candidates.iter_mut() {
        candidate.comparison = Some(ContextComparison {
            minimum_relatedness: request.minimum_relatedness,
            members: request
                .cohort
                .member_samples()
                .iter()
                .map(|sample| ContextMemberAlternative {
                    content: sample.content.clone(),
                    alternative: None,
                })
                .collect(),
        });
    }
    let eligible: Vec<_> = request.eligible.iter().collect();
    for window in eligible.chunks(TEXT_FEATURE_WINDOW) {
        check_cancelled(cancelled)?;
        let descriptions: Vec<_> = window
            .iter()
            .map(|id| request.qualified[id].description())
            .collect();
        let features = load_or_encode_descriptions(&descriptions, residency, cache, cancelled)?;
        let topic_descriptions: Vec<_> = window
            .iter()
            .map(|id| request.topics[id].description())
            .collect();
        let topics = load_or_encode_descriptions(&topic_descriptions, residency, cache, cancelled)?;
        for &id in window {
            check_cancelled(cancelled)?;
            let revision = ClassificationRevision::description(request.qualified[id].description());
            let feature = &features[&revision];
            let topic_revision =
                ClassificationRevision::description(request.topics[id].description());
            let topic = &topics[&topic_revision];
            let scores = request
                .cohort
                .member_similarities(feature)
                .map_err(|error| {
                    ScanError::new("classification_feature_mismatch", error.to_string())
                })?;
            for candidate in candidates.iter_mut() {
                check_cancelled(cancelled)?;
                let selected = &selected_topics[&ClassificationRevision::description(
                    request.topics[&candidate.candidate.concept.id()].description(),
                )];
                if candidate.candidate.concept.id() == *id || selected.description == topic_revision
                {
                    continue;
                }
                let relatedness = selected
                    .embedding
                    .cosine(&topic.embedding)
                    .map_err(|error| {
                        ScanError::new("classification_feature_mismatch", error.to_string())
                    })?;
                if relatedness >= request.minimum_relatedness {
                    continue;
                }
                let comparison = candidate
                    .comparison
                    .as_mut()
                    .expect("initialized comparison");
                for (member, score) in comparison.members.iter_mut().zip(&scores) {
                    if member
                        .alternative
                        .is_none_or(|previous| score.similarity > previous.similarity)
                    {
                        member.alternative = Some(ConceptAlternative {
                            concept: *id,
                            similarity: score.similarity,
                            relatedness,
                        });
                    }
                }
            }
        }
    }
    Ok(())
}

pub(super) fn aligned(
    cohort: &DiscoveryCohort,
    candidate: &ConsolidatedConcept,
    expected_relatedness: Option<f32>,
) -> Result<Vec<Option<ConceptAlternative>>, ScanError> {
    let comparison = match (&candidate.comparison, expected_relatedness) {
        (None, None) => return Ok(vec![None; cohort.member_samples().len()]),
        (Some(comparison), Some(expected)) if comparison.minimum_relatedness == expected => {
            comparison
        }
        _ => return Err(invalid()),
    };
    if !comparison.minimum_relatedness.is_finite()
        || !(0.0..=1.0).contains(&comparison.minimum_relatedness)
        || comparison.members.len() != cohort.member_samples().len()
    {
        return Err(invalid());
    }
    let mut alternatives = BTreeMap::new();
    for member in &comparison.members {
        if let Some(alternative) = member.alternative
            && (alternative.concept == candidate.candidate.concept.id()
                || !alternative.similarity.is_finite()
                || !(-1.0..=1.0).contains(&alternative.similarity)
                || !alternative.relatedness.is_finite()
                || !(-1.0..comparison.minimum_relatedness).contains(&alternative.relatedness))
        {
            return Err(invalid());
        }
        let content = &member.content;
        if alternatives
            .insert(
                (&content.scheme, content.digest, content.byte_length),
                member.alternative,
            )
            .is_some()
        {
            return Err(invalid());
        }
    }
    cohort
        .member_samples()
        .iter()
        .map(|sample| {
            let content = &sample.content;
            alternatives
                .get(&(&content.scheme, content.digest, content.byte_length))
                .copied()
                .ok_or_else(invalid)
        })
        .collect()
}

fn invalid() -> ScanError {
    ScanError::new(
        "classification_discovery_evidence_invalid",
        "Context comparison needs complete compatible member evidence",
    )
}
