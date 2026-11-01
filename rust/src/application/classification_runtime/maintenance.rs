use std::path::Path;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use crate::domain::classification::maintenance::{
    CategoryExpansionPlan, CategoryMaintenancePlan, CategoryMergePlan, CategoryRetirementPlan,
};
use crate::domain::classification_configuration::ClassificationConfigurationSnapshot;
use crate::domain::{LibraryChangeLane, ScanError};
use crate::ports::classification_configuration::{
    CategoryExpansionRepository, CategoryMergeRepository, CategoryRetirementRepository,
};

use super::super::catalog_session::open_catalog;
use super::while_retired;

pub enum PreparedCategoryMaintenance<'a> {
    Expansion(&'a CategoryExpansionPlan),
    Retirement(&'a CategoryRetirementPlan),
    Merge(&'a CategoryMergePlan),
}

impl<'a> From<&'a CategoryMaintenancePlan> for PreparedCategoryMaintenance<'a> {
    fn from(plan: &'a CategoryMaintenancePlan) -> Self {
        match plan {
            CategoryMaintenancePlan::Expansion(plan) => Self::Expansion(plan),
            CategoryMaintenancePlan::Retirement(plan) => Self::Retirement(plan),
            CategoryMaintenancePlan::Merge(plan) => Self::Merge(plan),
        }
    }
}

pub fn commit_category_maintenance(
    database: &Path,
    plan: PreparedCategoryMaintenance<'_>,
    cancelled: &Arc<AtomicBool>,
) -> Result<ClassificationConfigurationSnapshot, ScanError> {
    if cancelled.load(Ordering::Acquire) {
        return Err(ScanError::new(
            "classification_cancelled",
            "Classification was cancelled",
        ));
    }
    while_retired(|| {
        let mut catalog = open_catalog(database, LibraryChangeLane::Recovery)?;
        match plan {
            PreparedCategoryMaintenance::Expansion(plan) => {
                catalog.apply_category_expansion(plan, cancelled)
            }
            PreparedCategoryMaintenance::Retirement(plan) => {
                catalog.apply_category_retirement(plan, cancelled)
            }
            PreparedCategoryMaintenance::Merge(plan) => {
                catalog.apply_category_merge(plan, cancelled)
            }
        }
    })
}
