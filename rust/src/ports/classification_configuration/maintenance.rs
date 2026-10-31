use std::sync::atomic::AtomicBool;

use crate::domain::ScanError;
use crate::domain::classification::CategoryId;
use crate::domain::classification::maintenance::{
    CategoryExpansionPlan, CategoryExpansionSnapshot,
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
