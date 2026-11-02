use std::collections::BTreeSet;

use super::CategoryRetirementSnapshot;
use crate::domain::ScanError;
use crate::domain::classification::{CategoryId, ClassificationRevision};
use crate::domain::classification_configuration::ClassificationConfigurationChange;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ManagedCategoryCatalog {
    pub(crate) revision: ClassificationRevision,
    pub(crate) categories: BTreeSet<CategoryId>,
}

impl ManagedCategoryCatalog {
    pub fn new(
        revision: ClassificationRevision,
        categories: BTreeSet<CategoryId>,
    ) -> Result<Self, ScanError> {
        if categories.is_empty() || categories.len() > 256 {
            return Err(ScanError::new(
                "classification_category_catalog_invalid",
                "The managed category catalog requires between 1 and 256 identities",
            ));
        }
        Ok(Self {
            revision,
            categories,
        })
    }
}

pub struct CategoryCatalogReconciliationPlan {
    pub(crate) before: CategoryRetirementSnapshot,
    pub(crate) catalog: ManagedCategoryCatalog,
    pub(crate) configuration: ClassificationConfigurationChange,
    pub(crate) retired: BTreeSet<CategoryId>,
}

impl CategoryCatalogReconciliationPlan {
    pub fn retired(&self) -> &BTreeSet<CategoryId> {
        &self.retired
    }
}
