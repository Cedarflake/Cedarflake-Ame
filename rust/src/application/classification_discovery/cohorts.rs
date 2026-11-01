use std::collections::BTreeSet;
use std::sync::atomic::AtomicBool;

use crate::domain::ScanError;
use crate::domain::classification::ClassificationContent;
use crate::domain::classification::discovery::{DiscoveryCohort, DiscoverySample};

use super::super::classification_matching::CategoryMatcher;
use super::check_cancelled;

mod linkage;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DiscoveryCohortPolicy {
    pub minimum_pair_cosine: f32,
    pub minimum_members: usize,
    pub maximum_cohorts: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiscoveryCohortDeferral {
    InsufficientMembers,
    MissingComparison,
    CohortLimit,
}

#[derive(Debug, PartialEq, Eq)]
pub struct DeferredDiscoveryCohort {
    pub members: Vec<ClassificationContent>,
    pub reason: DiscoveryCohortDeferral,
}

pub struct DiscoveryCohortBatch {
    pub cohorts: Vec<DiscoveryCohort>,
    pub deferred: Vec<DeferredDiscoveryCohort>,
}

pub fn form_discovery_cohorts(
    samples: &[DiscoverySample],
    matcher: &CategoryMatcher,
    policy: DiscoveryCohortPolicy,
    cancelled: &AtomicBool,
) -> Result<DiscoveryCohortBatch, ScanError> {
    check_cancelled(cancelled)?;
    if samples.len() > 512
        || !policy.minimum_pair_cosine.is_finite()
        || !(0.0..=1.0).contains(&policy.minimum_pair_cosine)
        || !(2..=512).contains(&policy.minimum_members)
        || !(1..=64).contains(&policy.maximum_cohorts)
    {
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
    let mut current = Vec::with_capacity(ordered.len());
    for sample in &ordered {
        check_cancelled(cancelled)?;
        current.push(
            matcher
                .rank(&sample.feature)
                .map_err(|_| invalid())?
                .leading()
                .category_id
                .clone(),
        );
    }
    let mut clusters = linkage::groups(&ordered, policy.minimum_pair_cosine, cancelled)?;
    clusters.sort_by(|left, right| right.len().cmp(&left.len()).then_with(|| left.cmp(right)));
    let mut result = DiscoveryCohortBatch {
        cohorts: Vec::new(),
        deferred: Vec::new(),
    };
    for cluster in clusters {
        check_cancelled(cancelled)?;
        let members: BTreeSet<_> = cluster.iter().copied().collect();
        let categories: BTreeSet<_> = cluster.iter().map(|index| &current[*index]).collect();
        let comparison: Vec<_> = (0..ordered.len())
            .filter(|index| !members.contains(index) && categories.contains(&current[*index]))
            .collect();
        let deferral = if cluster.len() < policy.minimum_members {
            Some(DiscoveryCohortDeferral::InsufficientMembers)
        } else if comparison.is_empty() {
            Some(DiscoveryCohortDeferral::MissingComparison)
        } else if result.cohorts.len() == policy.maximum_cohorts {
            Some(DiscoveryCohortDeferral::CohortLimit)
        } else {
            None
        };
        if let Some(reason) = deferral {
            result.deferred.push(DeferredDiscoveryCohort {
                members: cluster
                    .iter()
                    .map(|index| ordered[*index].content.clone())
                    .collect(),
                reason,
            });
            continue;
        }
        let members: Vec<_> = cluster
            .iter()
            .map(|index| ordered[*index].clone())
            .collect();
        let comparison: Vec<_> = comparison
            .iter()
            .map(|index| ordered[*index].clone())
            .collect();
        result.cohorts.push(
            DiscoveryCohort::new(matcher.model(), &members, &comparison).map_err(|_| invalid())?,
        );
    }
    check_cancelled(cancelled)?;
    Ok(result)
}

fn content_key(content: &ClassificationContent) -> (&str, &[u8; 32], u64) {
    (
        content.scheme.as_str(),
        &content.digest,
        content.byte_length,
    )
}

fn invalid() -> ScanError {
    ScanError::new(
        "classification_cohort_invalid",
        "Discovery grouping requires a bounded compatible window of distinct content features",
    )
}

#[cfg(test)]
mod tests;
