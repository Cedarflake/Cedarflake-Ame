use crate::domain::ScanError;
use crate::domain::classification_configuration::{
    ClassificationConfigurationChange, ClassificationConfigurationSnapshot,
    ClassificationNavigationSnapshot,
};

mod maintenance;
pub use maintenance::{
    CategoryCatalogRepository, CategoryCoverageRepository, CategoryExpansionRepository,
    CategoryMergeRepository, CategoryRetirementRepository,
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

pub trait ClassificationNavigationRepository {
    fn load_classification_navigation(&self)
    -> Result<ClassificationNavigationSnapshot, ScanError>;
}
