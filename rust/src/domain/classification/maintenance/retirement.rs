use std::collections::{BTreeMap, BTreeSet};

use super::ManagedCategoryOrigin;
use crate::domain::classification::{CategoryDefinition, CategoryId, ModelCompatibility};
use crate::domain::classification_configuration::{
    ClassificationConfigurationChange, ClassificationConfigurationSnapshot,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CategoryRetirementSnapshot {
    pub configuration: ClassificationConfigurationSnapshot,
    pub model: Option<ModelCompatibility>,
    pub origins: BTreeMap<CategoryId, ManagedCategoryOrigin>,
    pub human_categories: BTreeSet<CategoryId>,
}

impl CategoryRetirementSnapshot {
    pub(crate) fn protects(&self, category: &CategoryDefinition) -> bool {
        self.human_categories.contains(category.id())
            || self
                .origins
                .values()
                .any(|origin| origin.purpose.as_ref() == Some(category.id()))
            || self.origins.get(category.id()).is_none_or(|origin| {
                origin.protected
                    || origin.admitted_label != category.label()
                    || origin.admitted_semantic_revision != category.semantic_revision()
            })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EquivalentCategoryRetirement {
    pub category_id: CategoryId,
    pub retained_id: CategoryId,
}

pub struct CategoryRetirementPlan {
    pub(crate) before: CategoryRetirementSnapshot,
    pub(crate) configuration: ClassificationConfigurationChange,
    pub(crate) retirements: Vec<EquivalentCategoryRetirement>,
}

impl CategoryRetirementPlan {
    pub fn retirements(&self) -> &[EquivalentCategoryRetirement] {
        &self.retirements
    }

    pub fn configuration(&self) -> &ClassificationConfigurationChange {
        &self.configuration
    }
}
