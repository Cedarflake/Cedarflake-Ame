use crate::domain::ScanError;
use crate::domain::classification_configuration::{
    ClassificationConfigurationChange, ClassificationConfigurationSnapshot,
};

mod maintenance;
pub use maintenance::{
    CategoryCoverageRepository, CategoryExpansionRepository, CategoryMergeRepository,
    CategoryRetirementRepository,
};

pub trait ClassificationConfigurationRepository {
    fn load_classification_configuration(
        &self,
    ) -> Result<ClassificationConfigurationSnapshot, ScanError>;

    fn save_classification_configuration(
        &mut self,
        change: &ClassificationConfigurationChange,
    ) -> Result<ClassificationConfigurationSnapshot, ScanError>;
}
