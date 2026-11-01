use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, atomic::AtomicBool};

use crate::domain::ScanError;
use crate::domain::classification::maintenance::{
    CategoryCoverageSummary, CategoryCoverageSupport,
};
use crate::domain::classification::{ClassificationContent, ClassificationRevision};
use crate::ports::classification_configuration::CategoryCoverageRepository;

use super::check_cancelled;

pub fn survey_category_coverage(
    repository: &impl CategoryCoverageRepository,
    page_size: usize,
    retained_per_category: usize,
    cancelled: &Arc<AtomicBool>,
) -> Result<CategoryCoverageSummary, ScanError> {
    check_cancelled(cancelled)?;
    if !(1..=64).contains(&page_size) || !(1..=64).contains(&retained_per_category) {
        return Err(invalid());
    }
    let snapshot = repository.begin_category_coverage(cancelled)?;
    let taxonomy = snapshot
        .configuration
        .catalog
        .taxonomy
        .as_ref()
        .ok_or_else(invalid)?;
    let configuration = snapshot
        .configuration
        .catalog
        .configuration
        .ok_or_else(invalid)?;
    let identities: BTreeSet<_> = taxonomy
        .categories()
        .iter()
        .map(|category| category.id().clone())
        .collect();
    let mut categories: BTreeMap<_, _> = identities
        .iter()
        .cloned()
        .map(|id| {
            (
                id,
                CategoryCoverageSupport {
                    contents: 0,
                    retained_contents: Vec::new(),
                },
            )
        })
        .collect();
    let mut hash = blake3::Hasher::new_derive_key("ame.classification.category-coverage.v1");
    hash.update(&snapshot.catalog_revision.to_le_bytes());
    hash.update(configuration.bytes());
    hash.update(&(retained_per_category as u64).to_le_bytes());
    hash.update(&[u8::from(snapshot.unverified_sources)]);
    let mut after = None;
    let mut classified_contents = 0;
    let mut unclassified_contents = 0;
    loop {
        check_cancelled(cancelled)?;
        let page =
            repository.read_category_coverage(&snapshot, after.as_ref(), page_size, cancelled)?;
        if page.entries.len() > page_size || (page.has_more && page.entries.len() != page_size) {
            return Err(invalid());
        }
        for entry in page.entries {
            check_cancelled(cancelled)?;
            if after
                .as_ref()
                .is_some_and(|previous| key(previous) >= key(&entry.content))
            {
                return Err(invalid());
            }
            field(&mut hash, entry.content.scheme.as_str().as_bytes());
            hash.update(&entry.content.digest);
            hash.update(&entry.content.byte_length.to_le_bytes());
            if let Some(prediction) = entry.prediction {
                let predicted_ids: BTreeSet<_> = prediction
                    .initial()
                    .categories()
                    .iter()
                    .map(|score| score.category_id.clone())
                    .collect();
                if prediction.content() != &entry.content
                    || prediction.run().configuration() != configuration
                    || prediction.run().model() != snapshot.model
                    || predicted_ids != identities
                {
                    return Err(invalid());
                }
                hash.update(&[1]);
                field(&mut hash, prediction.id().as_str().as_bytes());
                field(&mut hash, prediction.run().id().as_str().as_bytes());
                for ranking in [Some(prediction.initial()), prediction.refined()] {
                    hash.update(&[u8::from(ranking.is_some())]);
                    if let Some(ranking) = ranking {
                        hash.update(ranking.rules_revision().bytes());
                        for score in ranking.categories() {
                            field(&mut hash, score.category_id.as_str().as_bytes());
                            hash.update(&score.similarity.to_le_bytes());
                        }
                    }
                }
                let category = categories
                    .get_mut(&prediction.suggestion().leading().category_id)
                    .ok_or_else(invalid)?;
                category.contents += 1;
                if category.retained_contents.len() < retained_per_category {
                    category.retained_contents.push(entry.content.clone());
                }
                classified_contents += 1;
            } else {
                hash.update(&[0]);
                unclassified_contents += 1;
            }
            after = Some(entry.content);
        }
        if !page.has_more {
            break;
        }
    }
    check_cancelled(cancelled)?;
    repository.validate_category_coverage(&snapshot, cancelled)?;
    check_cancelled(cancelled)?;
    Ok(CategoryCoverageSummary {
        snapshot,
        categories,
        classified_contents,
        unclassified_contents,
        evidence: ClassificationRevision::from_bytes(*hash.finalize().as_bytes()),
    })
}

fn key(content: &ClassificationContent) -> (&str, &[u8; 32], u64) {
    (
        content.scheme.as_str(),
        &content.digest,
        content.byte_length,
    )
}

fn field(hash: &mut blake3::Hasher, value: &[u8]) {
    hash.update(&(value.len() as u64).to_le_bytes());
    hash.update(value);
}

fn invalid() -> ScanError {
    ScanError::new(
        "classification_coverage_invalid",
        "Category coverage requires complete ordered current-content evidence",
    )
}

#[cfg(test)]
mod tests;
