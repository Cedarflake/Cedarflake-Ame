use std::collections::BTreeSet;

use super::{CategoryId, ClassificationContent, ClassificationRevision, ModelCompatibility};
use crate::domain::classification_configuration::{
    ClassificationConfigurationChange, ClassificationConfigurationSnapshot,
};

pub struct CategoryExpansionSnapshot {
    pub configuration: ClassificationConfigurationSnapshot,
    pub model: Option<ModelCompatibility>,
    pub known_categories: BTreeSet<CategoryId>,
}

pub struct CategoryExpansionPlan {
    pub(crate) configuration: ClassificationConfigurationChange,
    pub(crate) additions: Vec<CategoryId>,
    pub(crate) sources: Vec<ClassificationContent>,
    pub(crate) evidence: ClassificationRevision,
}

impl CategoryExpansionPlan {
    pub fn additions(&self) -> &[CategoryId] {
        &self.additions
    }

    pub fn configuration(&self) -> &ClassificationConfigurationChange {
        &self.configuration
    }
}
