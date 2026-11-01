use std::collections::BTreeMap;

use crate::domain::classification::discovery::{
    CategoryNamingPurpose, ConceptId, VocabularyConcept,
};
use crate::domain::classification::{CategoryId, ClassificationRevision};
use crate::domain::classification_configuration::ClassificationConfigurationChange;

use super::{CategoryCoverageSummary, CategoryRetirementSnapshot};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CategoryMergeBasis {
    pub naming: CategoryNamingPurpose,
    pub minimum_members: usize,
    pub evidence: ClassificationRevision,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CategoryMergeSnapshot {
    pub categories: CategoryRetirementSnapshot,
    pub bases: BTreeMap<CategoryId, CategoryMergeBasis>,
}

pub struct CategoryMergePlan {
    pub(crate) before: CategoryMergeSnapshot,
    pub(crate) coverage: CategoryCoverageSummary,
    pub(crate) configuration: ClassificationConfigurationChange,
    pub(crate) merged: PurposeCategoryMerge,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PurposeCategoryMerge {
    pub category_id: CategoryId,
    pub retained_id: CategoryId,
    pub context: Option<CategoryMergeContext>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CategoryMergeContext {
    pub catalog: ClassificationRevision,
    pub narrower: VocabularyConcept,
    pub broader: VocabularyConcept,
    pub path: Vec<ConceptId>,
}

impl CategoryMergePlan {
    pub fn category_id(&self) -> &CategoryId {
        &self.merged.category_id
    }

    pub fn retained_id(&self) -> &CategoryId {
        &self.merged.retained_id
    }

    pub fn configuration(&self) -> &ClassificationConfigurationChange {
        &self.configuration
    }
}
