use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::AtomicBool;

use crate::domain::ScanError;
use crate::domain::classification::discovery::{
    CategoryNaming, ConceptId, ConceptNamingScope, DiscoveryCohort,
};
use crate::domain::classification::{
    CategoryScore, ClassificationContent, ClassificationRevision, ModelCompatibility,
};

use super::super::classification_matching::CategoryMatcher;
use super::{
    ConceptAlternative, ConceptConsolidation, ConceptConsolidationLimits, ConceptSearchLimits,
    check_cancelled, competition,
};

mod policy;

pub use policy::{CategoryAdmissionCriteria, CategoryAdmissionPolicy};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MemberAdmission {
    Supports,
    DifferentPurpose,
    NoImprovement,
    CompetingContext,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CategoryMemberEvidence {
    pub content: ClassificationContent,
    pub current: CategoryScore,
    pub similarity: f32,
    pub improvement: f32,
    pub separation: f32,
    pub disposition: MemberAdmission,
    pub alternative_context: Option<ConceptAlternative>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CategoryAdmissionDisposition {
    Proposed,
    WeakContext,
    InsufficientMembers,
    InsufficientCoverage,
    NameConflict,
    ProposalLimit,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CategoryAdmissionCandidate {
    pub concept_id: ConceptId,
    pub disposition: CategoryAdmissionDisposition,
    pub supporting_members: usize,
    pub mean_supporting_improvement: f32,
    pub members: Vec<CategoryMemberEvidence>,
}

#[derive(Debug, PartialEq)]
pub struct CategoryAdmission {
    pub naming: CategoryNaming,
    pub model: ModelCompatibility,
    pub cohort_revision: ClassificationRevision,
    pub vocabulary_revision: ClassificationRevision,
    pub retrieval_revision: ClassificationRevision,
    pub retrieval_limits: ConceptSearchLimits,
    pub frontier_revision: ClassificationRevision,
    pub consolidation_revision: ClassificationRevision,
    pub consolidation_limits: ConceptConsolidationLimits,
    pub taxonomy_revision: ClassificationRevision,
    pub taxonomy_names_revision: ClassificationRevision,
    pub policy_revision: ClassificationRevision,
    pub minimum_context_relatedness: Option<f32>,
    pub candidates: Vec<CategoryAdmissionCandidate>,
}

pub fn assess_category_candidates(
    cohort: &DiscoveryCohort,
    discovery: &ConceptConsolidation,
    matcher: &CategoryMatcher,
    policy: &CategoryAdmissionPolicy,
    cancelled: &AtomicBool,
) -> Result<CategoryAdmission, ScanError> {
    check_cancelled(cancelled)?;
    if cohort.revision() != discovery.cohort_revision
        || cohort.model() != discovery.model
        || matcher.model() != discovery.model
    {
        return Err(invalid_evidence());
    }
    discovery
        .naming
        .validate(matcher.taxonomy())
        .map_err(|error| {
            ScanError::new("classification_naming_purpose_changed", error.to_string())
        })?;
    let scores = aligned_scores(cohort, discovery, cancelled)?;
    let minimum_context_relatedness = discovery.comparison_policy.relatedness()?;
    let mut baseline = Vec::with_capacity(cohort.member_samples().len());
    for sample in cohort.member_samples() {
        check_cancelled(cancelled)?;
        let ranked = matcher
            .rank(&sample.feature)
            .map_err(|_| invalid_evidence())?;
        baseline.push(ranked.leading().clone());
    }
    let criteria = policy.criteria();
    let mut candidates = Vec::with_capacity(discovery.candidates.len());
    for (index, concept) in discovery.candidates.iter().enumerate() {
        check_cancelled(cancelled)?;
        let alternatives = competition::aligned(cohort, concept, minimum_context_relatedness)?;
        let mut members = Vec::with_capacity(baseline.len());
        let mut supporting_members = 0;
        let mut improvement_total = 0.0_f64;
        for (member_index, current) in baseline.iter().enumerate() {
            let similarity = scores[index][member_index];
            let mut competing = scores
                .iter()
                .enumerate()
                .filter(|(other, _)| *other != index)
                .map(|(_, row)| row[member_index])
                .fold(current.similarity, f32::max);
            if let Some(alternative) = alternatives[member_index] {
                competing = competing.max(alternative.similarity);
            }
            let improvement = similarity - current.similarity;
            let separation = similarity - competing;
            let disposition = if !discovery.naming.includes(&current.category_id) {
                MemberAdmission::DifferentPurpose
            } else if improvement <= 0.0 || improvement < criteria.minimum_improvement {
                MemberAdmission::NoImprovement
            } else if separation <= 0.0 || separation < criteria.minimum_separation {
                MemberAdmission::CompetingContext
            } else {
                supporting_members += 1;
                improvement_total += f64::from(improvement);
                MemberAdmission::Supports
            };
            members.push(CategoryMemberEvidence {
                content: cohort.member_samples()[member_index].content.clone(),
                current: current.clone(),
                similarity,
                improvement,
                separation,
                disposition,
                alternative_context: alternatives[member_index],
            });
        }
        let contrast = concept.candidate.scores.contrast;
        let disposition = if contrast <= 0.0 || contrast < criteria.minimum_context_contrast {
            CategoryAdmissionDisposition::WeakContext
        } else if supporting_members < criteria.minimum_members {
            CategoryAdmissionDisposition::InsufficientMembers
        } else if supporting_members * 1000
            < baseline.len() * usize::from(criteria.coverage_per_thousand)
        {
            CategoryAdmissionDisposition::InsufficientCoverage
        } else {
            CategoryAdmissionDisposition::Proposed
        };
        candidates.push(CategoryAdmissionCandidate {
            concept_id: concept.candidate.concept.id(),
            disposition,
            supporting_members,
            mean_supporting_improvement: if supporting_members == 0 {
                0.0
            } else {
                (improvement_total / supporting_members as f64) as f32
            },
            members,
        });
    }
    candidates.sort_by(|left, right| {
        right
            .supporting_members
            .cmp(&left.supporting_members)
            .then_with(|| {
                right
                    .mean_supporting_improvement
                    .total_cmp(&left.mean_supporting_improvement)
            })
            .then_with(|| left.concept_id.cmp(&right.concept_id))
    });
    let labels: BTreeMap<_, _> = discovery
        .candidates
        .iter()
        .map(|candidate| {
            (
                candidate.candidate.concept.id(),
                candidate.candidate.concept.label().to_lowercase(),
            )
        })
        .collect();
    let mut used_labels: BTreeSet<_> = matcher
        .taxonomy()
        .categories()
        .iter()
        .map(|category| category.label().to_lowercase())
        .collect();
    let mut proposed = 0;
    for candidate in &mut candidates {
        check_cancelled(cancelled)?;
        if candidate.disposition != CategoryAdmissionDisposition::Proposed {
            continue;
        }
        let label = &labels[&candidate.concept_id];
        if used_labels.contains(label) {
            candidate.disposition = CategoryAdmissionDisposition::NameConflict;
        } else if proposed == criteria.max_proposals {
            candidate.disposition = CategoryAdmissionDisposition::ProposalLimit;
        } else {
            proposed += 1;
            used_labels.insert(label.clone());
        }
    }
    check_cancelled(cancelled)?;
    Ok(CategoryAdmission {
        naming: discovery.naming.clone(),
        model: discovery.model,
        cohort_revision: discovery.cohort_revision,
        vocabulary_revision: discovery.vocabulary_revision,
        retrieval_revision: discovery.retrieval_revision,
        retrieval_limits: discovery.retrieval_limits,
        frontier_revision: discovery.frontier_revision,
        consolidation_revision: discovery.consolidation_revision,
        consolidation_limits: discovery.limits,
        taxonomy_revision: matcher.taxonomy().revision(),
        taxonomy_names_revision: ClassificationRevision::derive(
            "ame.classification.category-admission-names.v1",
            matcher.taxonomy().categories().iter().flat_map(|category| {
                [
                    category.id().as_str().as_bytes(),
                    category.label().as_bytes(),
                ]
            }),
        ),
        policy_revision: policy.revision(),
        minimum_context_relatedness,
        candidates,
    })
}

fn aligned_scores(
    cohort: &DiscoveryCohort,
    discovery: &ConceptConsolidation,
    cancelled: &AtomicBool,
) -> Result<Vec<Vec<f32>>, ScanError> {
    if discovery.candidates.len() > 64 {
        return Err(invalid_evidence());
    }
    let indices: BTreeMap<_, _> = cohort
        .member_samples()
        .iter()
        .enumerate()
        .map(|(index, sample)| (content_key(&sample.content), index))
        .collect();
    let mut identities = BTreeSet::new();
    let mut scores = Vec::new();
    for candidate in &discovery.candidates {
        check_cancelled(cancelled)?;
        let aggregate = candidate.candidate.scores;
        if !identities.insert(candidate.candidate.concept.id())
            || candidate.candidate.concept.naming_scope() != ConceptNamingScope::Candidate
            || !discovery
                .naming
                .admits_description(&candidate.candidate.concept)
            || candidate.member_similarities.len() != indices.len()
            || !valid_cosine(aggregate.similarity)
            || !valid_cosine(aggregate.context_similarity)
            || !aggregate.contrast.is_finite()
            || !(-2.0..=2.0).contains(&aggregate.contrast)
            || (aggregate.similarity - aggregate.context_similarity - aggregate.contrast).abs()
                > 1e-6
        {
            return Err(invalid_evidence());
        }
        let mut row = vec![f32::NAN; indices.len()];
        for member in &candidate.member_similarities {
            let Some(&index) = indices.get(&content_key(&member.content)) else {
                return Err(invalid_evidence());
            };
            if !valid_cosine(member.similarity) || !row[index].is_nan() {
                return Err(invalid_evidence());
            }
            row[index] = member.similarity;
        }
        let mean = row.iter().map(|value| f64::from(*value)).sum::<f64>() / row.len() as f64;
        if (mean - f64::from(aggregate.similarity)).abs() > 1e-6 {
            return Err(invalid_evidence());
        }
        scores.push(row);
    }
    Ok(scores)
}

fn content_key(content: &ClassificationContent) -> (&str, [u8; 32], u64) {
    (content.scheme.as_str(), content.digest, content.byte_length)
}

fn valid_cosine(value: f32) -> bool {
    value.is_finite() && (-1.0..=1.0).contains(&value)
}

fn invalid_evidence() -> ScanError {
    ScanError::new(
        "classification_discovery_evidence_invalid",
        "Category admission needs compatible, complete per-content discovery evidence",
    )
}
