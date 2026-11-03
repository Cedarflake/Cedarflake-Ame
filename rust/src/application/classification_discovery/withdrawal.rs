use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::AtomicBool;

use crate::domain::ScanError;
use crate::domain::classification::discovery::DiscoverySample;
use crate::domain::classification::{
    CategoryId, CategoryScore, ClassificationContent, ClassificationRevision, ModelCompatibility,
};

use super::super::classification_matching::CategoryMatcher;
use super::check_cancelled;

#[derive(Clone, Debug, PartialEq)]
pub struct CategoryWithdrawalMember {
    pub content: ClassificationContent,
    pub removed_similarity: f32,
    pub alternative: CategoryScore,
}

impl CategoryWithdrawalMember {
    pub fn similarity_loss(&self) -> f32 {
        self.removed_similarity - self.alternative.similarity
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CategoryWithdrawalImpact {
    pub category_id: CategoryId,
    pub affected: Vec<CategoryWithdrawalMember>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CategoryWithdrawalAssessment {
    pub revision: ClassificationRevision,
    pub model: ModelCompatibility,
    pub taxonomy_revision: ClassificationRevision,
    pub sampled_contents: usize,
    pub categories: Vec<CategoryWithdrawalImpact>,
}

/// Each impact removes one category from the unchanged scheme, independently of the other impacts.
pub fn assess_category_withdrawals(
    matcher: &CategoryMatcher,
    samples: &[DiscoverySample],
    cancelled: &AtomicBool,
) -> Result<CategoryWithdrawalAssessment, ScanError> {
    check_cancelled(cancelled)?;
    if samples.is_empty() || samples.len() > 512 || matcher.taxonomy().categories().len() < 2 {
        return Err(invalid());
    }
    let mut ordered: Vec<_> = samples.iter().collect();
    ordered.sort_by_key(|sample| content_key(&sample.content));
    let mut seen = BTreeSet::new();
    let mut affected: BTreeMap<_, Vec<_>> = matcher
        .taxonomy()
        .categories()
        .iter()
        .map(|category| (category.id().clone(), Vec::new()))
        .collect();
    let mut evidence = evidence_hasher(matcher);
    for sample in ordered {
        check_cancelled(cancelled)?;
        if !seen.insert(content_key(&sample.content)) {
            return Err(invalid());
        }
        let ranked = matcher.rank(&sample.feature).map_err(|_| invalid())?;
        let leader = ranked.leading();
        let alternative = ranked.categories().get(1).ok_or_else(invalid)?;
        affected
            .get_mut(&leader.category_id)
            .ok_or_else(invalid)?
            .push(CategoryWithdrawalMember {
                content: sample.content.clone(),
                removed_similarity: leader.similarity,
                alternative: alternative.clone(),
            });
        hash_content(&mut evidence, &sample.content);
        for score in ranked.categories() {
            hash_string(&mut evidence, score.category_id.as_str());
            evidence.update(&score.similarity.to_le_bytes());
        }
    }
    check_cancelled(cancelled)?;
    Ok(CategoryWithdrawalAssessment {
        revision: ClassificationRevision::from_bytes(*evidence.finalize().as_bytes()),
        model: matcher.model(),
        taxonomy_revision: matcher.taxonomy().revision(),
        sampled_contents: samples.len(),
        categories: affected
            .into_iter()
            .map(|(category_id, affected)| CategoryWithdrawalImpact {
                category_id,
                affected,
            })
            .collect(),
    })
}

fn evidence_hasher(matcher: &CategoryMatcher) -> blake3::Hasher {
    let mut hash = blake3::Hasher::new_derive_key("ame.classification.category-withdrawal.v1");
    let model = matcher.model();
    for revision in [
        matcher.taxonomy().revision(),
        model.image_identity().embedding_space,
        model.image_identity().pipeline,
        model.text_identity().pipeline,
    ] {
        hash.update(revision.bytes());
    }
    hash.update(&(model.dimensions() as u64).to_le_bytes());
    for category in matcher.taxonomy().categories() {
        hash_string(&mut hash, category.id().as_str());
        hash_string(&mut hash, category.label());
    }
    hash
}

fn content_key(content: &ClassificationContent) -> (&str, [u8; 32], u64) {
    (content.scheme.as_str(), content.digest, content.byte_length)
}

fn hash_content(hash: &mut blake3::Hasher, content: &ClassificationContent) {
    hash_string(hash, content.scheme.as_str());
    hash.update(&content.digest);
    hash.update(&content.byte_length.to_le_bytes());
}

fn hash_string(hash: &mut blake3::Hasher, value: &str) {
    hash.update(&(value.len() as u64).to_le_bytes());
    hash.update(value.as_bytes());
}

fn invalid() -> ScanError {
    ScanError::new(
        "classification_withdrawal_invalid",
        "Category withdrawal comparison requires distinct compatible cached contents and at least two categories",
    )
}

#[cfg(test)]
mod tests;
