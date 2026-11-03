use std::collections::BTreeSet;
use std::sync::atomic::AtomicBool;

use super::{CategoryRetirementSnapshot, check_cancelled};
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

pub(crate) fn catalog_retirements(
    snapshot: &CategoryRetirementSnapshot,
    catalog: &ManagedCategoryCatalog,
    cancelled: &AtomicBool,
) -> Result<BTreeSet<CategoryId>, ScanError> {
    let mut retired = BTreeSet::new();
    if let Some(rules) = &snapshot.configuration.catalog.taxonomy {
        for category in rules.categories() {
            check_cancelled(cancelled)?;
            if !catalog.categories.contains(category.id()) && !snapshot.protects(category) {
                retired.insert(category.id().clone());
            }
        }
    }
    Ok(retired)
}
