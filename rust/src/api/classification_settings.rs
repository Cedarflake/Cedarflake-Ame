use crate::domain::ScanError;
use crate::domain::classification_acceleration::ClassificationAccelerationSettings;
use crate::domain::classification_configuration::{
    ClassificationCategoryInput, ClassificationConfigurationAction, ClassificationControlAction,
    ClassificationSettings,
};

#[cfg(all(windows, feature = "classification-worker"))]
use crate::application::classification_configuration;

pub fn load_classification_acceleration() -> Result<ClassificationAccelerationSettings, ScanError> {
    #[cfg(all(windows, feature = "classification-worker"))]
    {
        crate::application::classification_acceleration::load()
    }
    #[cfg(not(all(windows, feature = "classification-worker")))]
    {
        Err(super::classification::unsupported())
    }
}

pub fn save_classification_acceleration(gpu_enabled: bool) -> Result<(), ScanError> {
    #[cfg(all(windows, feature = "classification-worker"))]
    {
        crate::application::classification_acceleration::save(gpu_enabled)
    }
    #[cfg(not(all(windows, feature = "classification-worker")))]
    {
        let _ = gpu_enabled;
        Err(super::classification::unsupported())
    }
}
pub fn load_classification_settings() -> Result<ClassificationSettings, ScanError> {
    #[cfg(all(windows, feature = "classification-worker"))]
    {
        classification_configuration::load()
    }
    #[cfg(not(all(windows, feature = "classification-worker")))]
    {
        Err(super::classification::unsupported())
    }
}

pub fn load_classification_navigation() -> Result<ClassificationSettings, ScanError> {
    #[cfg(all(windows, feature = "classification-worker"))]
    {
        classification_configuration::load_navigation()
    }
    #[cfg(not(all(windows, feature = "classification-worker")))]
    {
        Err(super::classification::unsupported())
    }
}

pub fn initial_classification_categories() -> Result<Vec<ClassificationCategoryInput>, ScanError> {
    #[cfg(all(windows, feature = "classification-worker"))]
    {
        classification_configuration::initial_categories()
    }
    #[cfg(not(all(windows, feature = "classification-worker")))]
    {
        Err(super::classification::unsupported())
    }
}

pub fn save_classification_settings(
    operation_id: String,
    expected_catalog_revision: u64,
    expected_dispatch_revision: u64,
    categories: Vec<ClassificationCategoryInput>,
    action: ClassificationConfigurationAction,
) -> Result<ClassificationSettings, ScanError> {
    #[cfg(all(windows, feature = "classification-worker"))]
    {
        classification_configuration::save(
            operation_id,
            expected_catalog_revision,
            expected_dispatch_revision,
            categories,
            action,
        )
    }
    #[cfg(not(all(windows, feature = "classification-worker")))]
    {
        let _ = (
            operation_id,
            expected_catalog_revision,
            expected_dispatch_revision,
            categories,
            action,
        );
        Err(super::classification::unsupported())
    }
}

pub fn change_classification_control(
    expected_revision: u64,
    action: ClassificationControlAction,
) -> Result<ClassificationSettings, ScanError> {
    #[cfg(all(windows, feature = "classification-worker"))]
    {
        classification_configuration::change_control(expected_revision, action)
    }
    #[cfg(not(all(windows, feature = "classification-worker")))]
    {
        let _ = (expected_revision, action);
        Err(super::classification::unsupported())
    }
}
