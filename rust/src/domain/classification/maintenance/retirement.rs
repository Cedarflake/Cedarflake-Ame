use std::collections::{BTreeMap, BTreeSet};

use super::ManagedCategoryOrigin;
use crate::domain::classification::{CategoryId, ModelCompatibility};
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
