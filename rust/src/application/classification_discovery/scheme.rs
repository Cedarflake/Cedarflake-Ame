use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::AtomicBool;

use crate::domain::ScanError;
use crate::domain::classification::discovery::DiscoverySample;
use crate::domain::classification::{
    CategoryDefinition, CategoryId, CategoryScore, ClassificationContent, ClassificationRevision,
    ModelCompatibility,
};

use super::super::classification_matching::CategoryMatcher;
use super::{CategoryAdmissionPolicy, check_cancelled};

pub(super) mod selection;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CategorySchemeDisposition {
    Admitted,
    ExistingDefinition(CategoryId),
    NameConflict,
    InsufficientMembers,
    OverlappingProposal,
    AdditionBudget,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CategorySchemeCandidate {
    pub category_id: CategoryId,
    pub disposition: CategorySchemeDisposition,
    pub unique_members: usize,
    pub mean_marginal_gain: f32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CategorySchemeMember {
    pub content: ClassificationContent,
    pub previous_category: CategoryId,
    pub proposed_category: CategoryId,
}

#[derive(Debug, PartialEq)]
pub struct CategorySchemeAssessment {
    pub revision: ClassificationRevision,
    pub model: ModelCompatibility,
    pub baseline_revision: ClassificationRevision,
    pub expanded_revision: ClassificationRevision,
    pub candidates: Vec<CategorySchemeCandidate>,
    pub members: Vec<CategorySchemeMember>,
}

pub(super) struct ScoredSample {
    pub content: ClassificationContent,
    pub baseline: CategoryScore,
    pub proposed: Vec<f32>,
    pub eligible: Vec<bool>,
}

pub fn assess_category_scheme(
    baseline: &CategoryMatcher,
    expanded: &CategoryMatcher,
    samples: &[DiscoverySample],
    policy: &CategoryAdmissionPolicy,
    cancelled: &AtomicBool,
) -> Result<CategorySchemeAssessment, ScanError> {
    check_cancelled(cancelled)?;
    let proposed = proposals(baseline, expanded)?;
    if samples.is_empty() || samples.len() > 512 {
        return Err(invalid());
    }
    let mut ordered: Vec<_> = samples.iter().collect();
    ordered.sort_by_key(|sample| content_key(&sample.content));
    if ordered
        .windows(2)
        .any(|pair| content_key(&pair[0].content) == content_key(&pair[1].content))
    {
        return Err(invalid());
    }
    let mut scored = Vec::with_capacity(ordered.len());
    for sample in ordered {
        check_cancelled(cancelled)?;
        let current = baseline.rank(&sample.feature).map_err(|_| invalid())?;
        let all = expanded.rank(&sample.feature).map_err(|_| invalid())?;
        let by_id: BTreeMap<_, _> = all
            .categories()
            .iter()
            .map(|score| (&score.category_id, score.similarity))
            .collect();
        if current
            .categories()
            .iter()
            .any(|score| by_id.get(&score.category_id) != Some(&score.similarity))
        {
            return Err(invalid());
        }
        scored.push(ScoredSample {
            content: sample.content.clone(),
            baseline: current.leading().clone(),
            eligible: vec![true; proposed.len()],
            proposed: proposed
                .iter()
                .map(|category| by_id[category.id()])
                .collect(),
        });
    }
    let candidates = selection::select(
        baseline.taxonomy().categories(),
        &proposed,
        &scored,
        policy.criteria(),
        cancelled,
    )?;
    let admitted: BTreeSet<_> = candidates
        .iter()
        .filter(|candidate| candidate.disposition == CategorySchemeDisposition::Admitted)
        .map(|candidate| &candidate.category_id)
        .collect();
    let members = scored
        .iter()
        .map(|sample| {
            let mut category = &sample.baseline.category_id;
            let mut best = sample.baseline.similarity;
            for (index, candidate) in proposed.iter().enumerate() {
                let score = sample.proposed[index];
                if admitted.contains(candidate.id())
                    && (score > best || (score == best && candidate.id() < category))
                {
                    category = candidate.id();
                    best = sample.proposed[index];
                }
            }
            CategorySchemeMember {
                content: sample.content.clone(),
                previous_category: sample.baseline.category_id.clone(),
                proposed_category: category.clone(),
            }
        })
        .collect();
    check_cancelled(cancelled)?;
    Ok(CategorySchemeAssessment {
        revision: revision(baseline, expanded, &scored, policy),
        model: baseline.model(),
        baseline_revision: baseline.taxonomy().revision(),
        expanded_revision: expanded.taxonomy().revision(),
        candidates,
        members,
    })
}

fn proposals<'a>(
    baseline: &CategoryMatcher,
    expanded: &'a CategoryMatcher,
) -> Result<Vec<&'a CategoryDefinition>, ScanError> {
    if baseline.model() != expanded.model() {
        return Err(invalid());
    }
    let current: BTreeMap<_, _> = baseline
        .taxonomy()
        .categories()
        .iter()
        .map(|category| (category.id(), category))
        .collect();
    let after: BTreeMap<_, _> = expanded
        .taxonomy()
        .categories()
        .iter()
        .map(|category| (category.id(), category))
        .collect();
    if current
        .iter()
        .any(|(id, category)| after.get(id) != Some(category))
    {
        return Err(invalid());
    }
    let proposed: Vec<_> = expanded
        .taxonomy()
        .categories()
        .iter()
        .filter(|category| !current.contains_key(category.id()))
        .collect();
    if proposed.len() > 64 {
        return Err(invalid());
    }
    Ok(proposed)
}

fn content_key(content: &ClassificationContent) -> (&str, [u8; 32], u64) {
    (content.scheme.as_str(), content.digest, content.byte_length)
}

fn revision(
    baseline: &CategoryMatcher,
    expanded: &CategoryMatcher,
    samples: &[ScoredSample],
    policy: &CategoryAdmissionPolicy,
) -> ClassificationRevision {
    let mut hash = blake3::Hasher::new_derive_key("ame.classification.category-scheme.v1");
    let model = baseline.model();
    for revision in [
        baseline.taxonomy().revision(),
        expanded.taxonomy().revision(),
        model.image_identity().embedding_space,
        model.image_identity().pipeline,
        model.text_identity().pipeline,
        policy.revision(),
    ] {
        hash.update(revision.bytes());
    }
    hash.update(&(model.dimensions() as u64).to_le_bytes());
    for category in expanded.taxonomy().categories() {
        for value in [category.id().as_str(), category.label()] {
            hash.update(&(value.len() as u64).to_le_bytes());
            hash.update(value.as_bytes());
        }
    }
    for sample in samples {
        hash.update(&(sample.content.scheme.as_str().len() as u64).to_le_bytes());
        hash.update(sample.content.scheme.as_str().as_bytes());
        hash.update(&sample.content.digest);
        hash.update(&sample.content.byte_length.to_le_bytes());
        let baseline_id = sample.baseline.category_id.as_str();
        hash.update(&(baseline_id.len() as u64).to_le_bytes());
        hash.update(baseline_id.as_bytes());
        hash.update(&sample.baseline.similarity.to_le_bytes());
        for score in &sample.proposed {
            hash.update(&score.to_le_bytes());
        }
    }
    ClassificationRevision::from_bytes(*hash.finalize().as_bytes())
}

fn invalid() -> ScanError {
    ScanError::new(
        "classification_scheme_invalid",
        "Category scheme assessment requires compatible bounded samples and unchanged existing definitions",
    )
}

#[cfg(test)]
mod tests;
