use std::collections::{BTreeMap, BTreeSet};

use super::discovery::{CategoryNaming, ConceptId};
use super::{
    CategoryId, ClassificationContent, ClassificationRecordId, ClassificationRevision,
    ModelCompatibility, Taxonomy,
};
use crate::domain::classification_configuration::{
    ClassificationConfigurationChange, ClassificationConfigurationSnapshot,
};

mod activation;
pub(crate) use activation::CategoryActivation;
mod coverage;
pub use coverage::{
    CategoryCoverageEntry, CategoryCoverageMember, CategoryCoveragePage, CategoryCoverageSnapshot,
    CategoryCoverageSummary, CategoryCoverageSupport,
};
mod merging;
pub(crate) use merging::PurposeCategoryMerge;
pub use merging::{CategoryMergeBasis, CategoryMergePlan, CategoryMergeSnapshot};
mod retirement;
pub use retirement::{
    CategoryRetirementPlan, CategoryRetirementSnapshot, EquivalentCategoryRetirement,
};

pub struct CategoryExpansionSnapshot {
    pub configuration: ClassificationConfigurationSnapshot,
    pub model: Option<ModelCompatibility>,
    pub known_categories: BTreeMap<CategoryId, String>,
    pub managed_categories: BTreeMap<CategoryId, ManagedCategoryOrigin>,
}

impl CategoryExpansionSnapshot {
    pub(crate) fn active_generated_categories(&self) -> BTreeSet<CategoryId> {
        self.configuration
            .catalog
            .taxonomy
            .as_ref()
            .into_iter()
            .flat_map(Taxonomy::categories)
            .filter(|category| self.managed_categories.contains_key(category.id()))
            .map(|category| category.id().clone())
            .collect()
    }
}

pub struct CategoryExpansionPlan {
    pub(crate) configuration: ClassificationConfigurationChange,
    pub(crate) additions: Vec<CategoryId>,
    pub(crate) sources: Vec<ClassificationContent>,
    pub(crate) evidence: ClassificationRevision,
    pub(crate) provenance: CategoryExpansionProvenance,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ManagedCategoryOrigin {
    pub category_id: CategoryId,
    pub concept_id: ConceptId,
    pub purpose: Option<CategoryId>,
    pub operation_id: ClassificationRecordId,
    pub admitted_label: String,
    pub admitted_semantic_revision: ClassificationRevision,
    pub protected: bool,
}

impl ManagedCategoryOrigin {
    pub(crate) fn validate(&self) -> Result<(), super::ClassificationError> {
        if managed_category_identity(self.concept_id, self.purpose.as_ref())? != self.category_id {
            return Err(super::ClassificationError::InvalidCategoryId);
        }
        Ok(())
    }
}

pub(crate) fn managed_category_identity(
    concept: ConceptId,
    purpose: Option<&CategoryId>,
) -> Result<CategoryId, super::ClassificationError> {
    let identity = ClassificationRevision::derive(
        "ame.classification.managed-category.v1",
        [
            concept.bytes().as_slice(),
            purpose.map_or("", CategoryId::as_str).as_bytes(),
        ],
    );
    CategoryId::new(
        blake3::Hash::from_bytes(*identity.bytes())
            .to_hex()
            .to_string(),
    )
}

pub(crate) struct CategoryExpansionProvenance {
    pub naming: CategoryNaming,
    pub vocabulary: ClassificationRevision,
    pub cohort: ClassificationRevision,
    pub retrieval: ClassificationRevision,
    pub frontier: ClassificationRevision,
    pub consolidation: ClassificationRevision,
    pub policy: ClassificationRevision,
    pub comparison_rules: ClassificationRevision,
    pub comparison_names: ClassificationRevision,
    pub criteria: CategoryExpansionCriteria,
    pub bounds: CategoryExpansionBounds,
    pub members: Vec<ClassificationContent>,
    pub candidates: Vec<CategoryExpansionCandidate>,
    pub capacity: CategoryExpansionCapacity,
}

pub(crate) struct CategoryExpansionCapacity {
    pub maximum_generated: usize,
    pub active_generated: BTreeSet<CategoryId>,
    pub catalog: Option<CategoryCapacityEvidence>,
}

pub(crate) struct CategoryCapacityEvidence {
    pub snapshot: CategoryCoverageSnapshot,
    pub coverage: ClassificationRevision,
    pub verified_contents: u64,
    pub minimum_members: usize,
}

pub(crate) struct CategoryExpansionCriteria {
    pub minimum_members: usize,
    pub coverage_per_thousand: u16,
    pub minimum_improvement: f32,
    pub minimum_separation: f32,
    pub minimum_context_contrast: f32,
    pub max_proposals: usize,
    pub minimum_context_relatedness: Option<f32>,
}

pub(crate) struct CategoryExpansionBounds {
    pub entry_points: usize,
    pub expanded_concepts: usize,
    pub retrieval_edges: usize,
    pub retrieval_hops: usize,
    pub seeds: usize,
    pub results: usize,
    pub consolidation_hops: usize,
    pub consolidation_concepts: usize,
    pub consolidation_edges: usize,
}

pub(crate) struct CategoryExpansionCandidate {
    pub category_id: CategoryId,
    pub activation: CategoryActivation,
    pub concept_id: ConceptId,
    pub supporting_contents: Vec<ClassificationContent>,
    pub mean_supporting_improvement: f32,
}

impl CategoryExpansionPlan {
    pub fn additions(&self) -> &[CategoryId] {
        &self.additions
    }

    pub fn configuration(&self) -> &ClassificationConfigurationChange {
        &self.configuration
    }
}
