use std::sync::{Arc, atomic::AtomicBool};

use crate::domain::ScanError;
use crate::domain::classification::ClassificationContent;
use crate::domain::classification::discovery::DiscoverySample;
use crate::domain::classification::maintenance::CategoryCoverageSnapshot;
use crate::domain::classification::work::ClassificationDispatchMode;
use crate::ports::classification_configuration::CategoryCoverageRepository;
use crate::ports::classification_features::ClassificationFeatureRepository;

use super::check_cancelled;

pub struct DiscoverySampleCursor {
    snapshot: CategoryCoverageSnapshot,
    after: ClassificationContent,
}

pub struct DiscoverySampleWindow {
    pub snapshot: CategoryCoverageSnapshot,
    pub samples: Vec<DiscoverySample>,
    pub examined_contents: usize,
    pub missing_features: usize,
    pub next: Option<DiscoverySampleCursor>,
}

pub fn read_discovery_samples(
    repository: &impl CategoryCoverageRepository,
    cache: &mut impl ClassificationFeatureRepository,
    cursor: Option<&DiscoverySampleCursor>,
    max_contents: usize,
    cancelled: &Arc<AtomicBool>,
) -> Result<DiscoverySampleWindow, ScanError> {
    check_cancelled(cancelled)?;
    if !(1..=512).contains(&max_contents) {
        return Err(invalid());
    }
    let snapshot = repository.begin_category_coverage(cancelled)?;
    if snapshot.configuration.dispatch.mode != ClassificationDispatchMode::Automatic
        || snapshot.configuration.catalog.taxonomy.is_none()
        || snapshot.configuration.catalog.configuration.is_none()
        || snapshot.configuration.dispatch.run_id.is_none()
        || snapshot.configuration.catalog.active_run_id.as_deref()
            != snapshot
                .configuration
                .dispatch
                .run_id
                .as_ref()
                .map(|id| id.as_str())
        || cursor.is_some_and(|cursor| cursor.snapshot != snapshot)
    {
        return Err(stale());
    }
    let mut after = cursor.map(|cursor| cursor.after.clone());
    let mut result = DiscoverySampleWindow {
        snapshot,
        samples: Vec::new(),
        examined_contents: 0,
        missing_features: 0,
        next: None,
    };
    let has_more = loop {
        check_cancelled(cancelled)?;
        let limit = (max_contents - result.examined_contents).min(64);
        let page = repository.read_category_coverage(
            &result.snapshot,
            after.as_ref(),
            limit,
            cancelled,
        )?;
        if page.entries.len() > limit || (page.has_more && page.entries.len() != limit) {
            return Err(invalid());
        }
        for entry in page.entries {
            check_cancelled(cancelled)?;
            if after
                .as_ref()
                .is_some_and(|last| key(last) >= key(&entry.content))
            {
                return Err(invalid());
            }
            let feature = cache.load_image_feature(result.snapshot.model, &entry.content)?;
            check_cancelled(cancelled)?;
            after = Some(entry.content.clone());
            result.examined_contents += 1;
            if let Some(feature) = feature {
                if feature.identity != result.snapshot.model.image_identity()
                    || feature.embedding.values().len() != result.snapshot.model.dimensions()
                {
                    return Err(invalid());
                }
                result.samples.push(DiscoverySample {
                    content: entry.content,
                    feature,
                });
            } else {
                result.missing_features += 1;
            }
        }
        if !page.has_more || result.examined_contents == max_contents {
            break page.has_more;
        }
    };
    repository.validate_category_coverage(&result.snapshot, cancelled)?;
    check_cancelled(cancelled)?;
    if has_more {
        result.next = Some(DiscoverySampleCursor {
            snapshot: result.snapshot.clone(),
            after: after.ok_or_else(invalid)?,
        });
    }
    Ok(result)
}

fn key(content: &ClassificationContent) -> (&str, &[u8; 32], u64) {
    (
        content.scheme.as_str(),
        &content.digest,
        content.byte_length,
    )
}

fn invalid() -> ScanError {
    ScanError::new(
        "classification_samples_invalid",
        "Discovery requires bounded ordered content and compatible cached features",
    )
}

fn stale() -> ScanError {
    ScanError::new(
        "classification_samples_changed",
        "The classification settings or current contents changed during discovery",
    )
}

#[cfg(test)]
mod tests;
