use std::sync::{Arc, atomic::AtomicBool};

use crate::domain::ScanError;
use crate::domain::classification::maintenance::{
    CategoryCoveragePage, CategoryCoverageSnapshot, CategoryExpansionPlan,
    CategoryExpansionSnapshot, CategoryMergePlan, CategoryMergeSnapshot, CategoryRetirementPlan,
    CategoryRetirementSnapshot,
};
use crate::domain::classification::{CategoryId, ClassificationContent};
use crate::domain::classification_configuration::ClassificationConfigurationSnapshot;

pub trait CategoryExpansionRepository {
    fn load_category_expansion_snapshot(
        &self,
        proposed: &[CategoryId],
    ) -> Result<CategoryExpansionSnapshot, ScanError>;

    fn apply_category_expansion(
        &mut self,
        plan: &CategoryExpansionPlan,
        cancelled: &Arc<AtomicBool>,
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

pub trait CategoryCoverageRepository {
    fn begin_category_coverage(
        &self,
        cancelled: &Arc<AtomicBool>,
    ) -> Result<CategoryCoverageSnapshot, ScanError>;

    fn read_category_coverage(
        &self,
        snapshot: &CategoryCoverageSnapshot,
        after: Option<&ClassificationContent>,
        limit: usize,
        cancelled: &Arc<AtomicBool>,
    ) -> Result<CategoryCoveragePage, ScanError>;

    fn validate_category_coverage(
        &self,
        snapshot: &CategoryCoverageSnapshot,
        cancelled: &Arc<AtomicBool>,
    ) -> Result<(), ScanError>;
}

pub trait CategoryMergeRepository: CategoryCoverageRepository {
    fn load_category_merge_snapshot(
        &self,
        cancelled: &Arc<AtomicBool>,
    ) -> Result<CategoryMergeSnapshot, ScanError>;

    fn apply_category_merge(
        &mut self,
        plan: &CategoryMergePlan,
        cancelled: &Arc<AtomicBool>,
    ) -> Result<ClassificationConfigurationSnapshot, ScanError>;
}
