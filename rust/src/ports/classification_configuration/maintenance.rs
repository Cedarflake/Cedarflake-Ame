use std::sync::atomic::AtomicBool;

use crate::domain::ScanError;
use crate::domain::classification::CategoryId;
use crate::domain::classification::maintenance::{
    CategoryExpansionPlan, CategoryExpansionSnapshot, CategoryRetirementPlan,
    CategoryRetirementSnapshot,
};
use crate::domain::classification_configuration::ClassificationConfigurationSnapshot;

pub trait CategoryExpansionRepository {
    fn load_category_expansion_snapshot(
        &self,
        proposed: &[CategoryId],
    ) -> Result<CategoryExpansionSnapshot, ScanError>;

    fn apply_category_expansion(
        &mut self,
        plan: &CategoryExpansionPlan,
        cancelled: &AtomicBool,
    ) -> Result<ClassificationConfigurationSnapshot, ScanError>;
}

pub trait CategoryRetirementRepository {
    fn load_category_retirement_snapshot(&self) -> Result<CategoryRetirementSnapshot, ScanError>;

    fn apply_category_retirement(
        &mut self,
        plan: &CategoryRetirementPlan,
        cancelled: &AtomicBool,
    ) -> Result<ClassificationConfigurationSnapshot, ScanError>;
}
