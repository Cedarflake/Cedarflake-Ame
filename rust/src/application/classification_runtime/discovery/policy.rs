use serde::Deserialize;

use crate::domain::ScanError;
use crate::domain::classification::discovery::{CategoryNaming, CategoryNamingPurpose};
use crate::domain::classification::{ClassificationRevision, Taxonomy};

use super::super::super::classification_candidate;
use super::super::super::classification_discovery::{
    CategoryAdmissionCriteria, CategoryAdmissionPolicy, ConceptConsolidationLimits,
    ConceptSearchLimits, DiscoveryCohortPolicy, DiscoveryPreparationPolicy,
};

const PROFILE: &str = include_str!("../../../../../resources/classification/discovery-policy.json");

#[derive(Deserialize)]
struct Profile {
    cohort_minimum_pair_cosine: f32,
    nomination: Nomination,
    admission: Admission,
    purposes: Vec<Purpose>,
}

#[derive(Deserialize)]
struct Nomination {
    max_candidates: usize,
    minimum_context_relatedness: f32,
}

#[derive(Deserialize)]
struct Admission {
    minimum_members: usize,
    coverage_per_thousand: u16,
    minimum_improvement: f32,
    minimum_separation: f32,
    minimum_context_contrast: f32,
    max_proposals: usize,
}

#[derive(Deserialize)]
struct Purpose {
    category_id: String,
    label_pattern: String,
    comparison_pattern: String,
}

pub(super) struct ManagedDiscoveryPolicy {
    profile: Profile,
    admission: CategoryAdmissionPolicy,
    naming: Vec<CategoryNaming>,
}

impl ManagedDiscoveryPolicy {
    pub(super) fn for_taxonomy(taxonomy: &Taxonomy) -> Result<Self, ScanError> {
        let mut profile: Profile = serde_json::from_str(PROFILE).map_err(invalid)?;
        let original = classification_candidate::initial_categories().map_err(invalid)?;
        let mut naming = Vec::new();
        for purpose in std::mem::take(&mut profile.purposes) {
            let current = taxonomy
                .categories()
                .iter()
                .find(|definition| definition.id().as_str() == purpose.category_id);
            let baseline = original
                .categories()
                .iter()
                .find(|definition| definition.id().as_str() == purpose.category_id);
            let (Some(current), Some(baseline)) = (current, baseline) else {
                continue;
            };
            if current.semantic_revision() != baseline.semantic_revision() {
                continue;
            }
            naming.push(CategoryNaming::WithinCategory(
                CategoryNamingPurpose::new(
                    current,
                    purpose.label_pattern,
                    purpose.comparison_pattern,
                )
                .map_err(invalid)?,
            ));
        }
        let criteria = &profile.admission;
        let admission = CategoryAdmissionPolicy::new(
            ClassificationRevision::derive(
                "ame.classification.managed-discovery.v1",
                [PROFILE.as_bytes()],
            ),
            CategoryAdmissionCriteria {
                minimum_members: criteria.minimum_members,
                coverage_per_thousand: criteria.coverage_per_thousand,
                minimum_improvement: criteria.minimum_improvement,
                minimum_separation: criteria.minimum_separation,
                minimum_context_contrast: criteria.minimum_context_contrast,
                max_proposals: criteria.max_proposals,
            },
        )?;
        Ok(Self {
            profile,
            admission,
            naming,
        })
    }

    pub(super) fn is_available(&self) -> bool {
        !self.naming.is_empty()
    }

    pub(super) fn preparation(&self) -> DiscoveryPreparationPolicy<'_> {
        DiscoveryPreparationPolicy {
            maximum_contents: 512,
            cohorts: DiscoveryCohortPolicy {
                minimum_pair_cosine: self.profile.cohort_minimum_pair_cosine,
                minimum_members: self.admission.criteria().minimum_members,
                maximum_cohorts: 64,
            },
            retrieval: ConceptSearchLimits {
                entry_points: 4096,
                expanded_concepts: 4096,
                edges: 65_536,
                hops: 2,
                seeds: 16,
                results: 64,
            },
            consolidation: ConceptConsolidationLimits {
                hops: 4,
                concepts: 4096,
                edges: 65_536,
            },
            maximum_candidates: self.profile.nomination.max_candidates,
            minimum_context_relatedness: self.profile.nomination.minimum_context_relatedness,
            admission: &self.admission,
            naming: &self.naming,
        }
    }
}

fn invalid(error: impl std::fmt::Display) -> ScanError {
    ScanError::new("classification_discovery_policy_invalid", error.to_string())
}

#[cfg(test)]
mod tests;
