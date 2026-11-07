use std::path::PathBuf;

use crate::domain::ScanError;
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

pub(super) fn model_storage() -> Result<PathBuf, ScanError> {
    #[cfg(debug_assertions)]
    if let Some(root) = std::env::var_os("CEDARFLAKE_AME_TEST_STORAGE_ROOT") {
        let root = PathBuf::from(root);
        if !root.is_absolute() {
            return Err(path_error("Test model storage must be absolute"));
        }
        return Ok(root.join("models"));
    }
    directories::ProjectDirs::from("com", "Cedarflake", "Ame")
        .map(|directories| directories.data_local_dir().join("models"))
        .ok_or_else(|| path_error("The operating system did not provide model storage"))
}
