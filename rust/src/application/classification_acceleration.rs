use crate::domain::ScanError;
use crate::domain::classification_acceleration::ClassificationAccelerationSettings;
use crate::ports::classification_acceleration::ClassificationAccelerationRepository;

use super::classification_desktop::installation_path;
use super::classification_model_installation::production::installation_repository;
use super::{classification_desktop, classification_runtime, storage_paths};

pub(crate) fn load() -> Result<ClassificationAccelerationSettings, ScanError> {
    Ok(ClassificationAccelerationSettings {
        gpu_enabled: installation_repository()
            .gpu_enabled(&installation_path(&storage_paths()?))?,
        status: classification_desktop::current_device()?,
    })
}

pub(crate) fn save(enabled: bool) -> Result<(), ScanError> {
    classification_runtime::while_retired(|| {
        installation_repository().set_gpu_enabled(&installation_path(&storage_paths()?), enabled)
    })
}
