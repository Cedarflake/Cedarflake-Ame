use std::path::{Path, PathBuf};

use crate::domain::ScanError;
use crate::domain::classification::installation::ClassificationInstallation;
use crate::ports::classification_acceleration::ClassificationAccelerationRepository;
use crate::ports::classification_installation::ClassificationInstallationRepository;
use crate::ports::classification_model_package::ClassificationModelPackage;

use super::path_error;

pub(crate) fn installation_repository()
-> impl ClassificationInstallationRepository + ClassificationAccelerationRepository {
    crate::adapters::ClassificationInstallationStore
}

pub(crate) fn model_package() -> impl ClassificationModelPackage {
    crate::adapters::classification_runtime::LocalClassificationModelPackage
}

pub(in crate::application) fn application_data() -> Result<PathBuf, ScanError> {
    std::env::current_exe()
        .map_err(path_error)?
        .parent()
        .map(|parent| parent.join("data"))
        .ok_or_else(|| path_error("Missing application directory"))
}

pub(super) fn bundled_models(data: &Path) -> PathBuf {
    data.join("classification-models")
        .join(model_package().package_directory())
}

pub(in crate::application) fn installation_for_execution(
    settings: &Path,
    application_data: &Path,
) -> Result<Option<ClassificationInstallation>, ScanError> {
    let mut installation = installation_repository().load(settings)?;
    if let Some(selected) = &mut installation
        && selected.runtime.ends_with("data/classification-runtime")
        && let Some(previous_data) = selected.runtime.parent()
        && previous_data != application_data
    {
        if selected.models == bundled_models(previous_data) {
            let models = bundled_models(application_data);
            if models.try_exists().map_err(path_error)? {
                selected.models = models;
            }
        }
        selected.runtime = application_data.join("classification-runtime");
    }
    Ok(installation)
}

pub(super) fn model_storage() -> Result<PathBuf, ScanError> {
    #[cfg(debug_assertions)]
    if let Some(root) = std::env::var_os("CEDARFLAKE_AME_TEST_STORAGE_ROOT") {
        let root = PathBuf::from(root);
        if !root.is_absolute() {
            return Err(path_error("Test model storage must be absolute"));
        }
        return Ok(root.join("models"));
    }
    Ok(crate::adapters::application_data::directories()?
        .data
        .join("models"))
}
