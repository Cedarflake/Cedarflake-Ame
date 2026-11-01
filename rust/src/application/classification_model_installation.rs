use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use crate::adapters::classification_runtime::installation::{
    VerifiedModelInstallation, check_cancelled, prepare,
};
use crate::adapters::{ClassificationInstallation, ClassificationInstallationStore};
use crate::domain::ScanError;
use crate::domain::classification::installation::ModelInstallationProgress;

use super::classification_desktop::installation_path;
use super::classification_runtime::while_retired;

static INSTALLING: AtomicBool = AtomicBool::new(false);

struct InstallationOwnership;

impl InstallationOwnership {
    fn acquire() -> Result<Self, ScanError> {
        INSTALLING
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| {
                ScanError::new(
                    "classification_installation_busy",
                    "A model installation is already in progress",
                )
            })?;
        Ok(Self)
    }
}

impl Drop for InstallationOwnership {
    fn drop(&mut self) {
        INSTALLING.store(false, Ordering::Release);
    }
}

pub struct PreparedModelInstallation {
    settings: PathBuf,
    expected: Option<ClassificationInstallation>,
    verified: VerifiedModelInstallation,
    _ownership: InstallationOwnership,
}

pub fn prepare_managed_classification_installation(
    cancelled: &AtomicBool,
    progress: &mut dyn FnMut(ModelInstallationProgress),
) -> Result<PreparedModelInstallation, ScanError> {
    check_cancelled(cancelled)?;
    let ownership = InstallationOwnership::acquire()?;
    let settings = installation_path(&super::storage_paths()?);
    let expected = ClassificationInstallationStore::load(&settings)?;
    let executable = std::env::current_exe().map_err(path_error)?;
    let runtime = executable
        .parent()
        .ok_or_else(|| path_error("Missing application directory"))?
        .join("data")
        .join("classification-runtime");
    let verified = prepare(&model_storage()?, &runtime, cancelled, progress)?;
    Ok(PreparedModelInstallation {
        settings,
        expected,
        verified,
        _ownership: ownership,
    })
}

impl PreparedModelInstallation {
    pub fn commit(self, cancelled: &AtomicBool) -> Result<(), ScanError> {
        let installation = ClassificationInstallation {
            models: self.verified.models.clone(),
            runtime: self.verified.runtime.clone(),
        };
        publish_installation(
            &self.settings,
            self.expected.as_ref(),
            &installation,
            cancelled,
        )
    }
}

fn publish_installation(
    settings: &Path,
    expected: Option<&ClassificationInstallation>,
    installation: &ClassificationInstallation,
    cancelled: &AtomicBool,
) -> Result<(), ScanError> {
    while_retired(|| {
        check_cancelled(cancelled)?;
        ClassificationInstallationStore::save_if_unchanged(
            settings,
            expected,
            installation,
            cancelled,
        )
    })
}

fn model_storage() -> Result<PathBuf, ScanError> {
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

fn path_error(error: impl std::fmt::Display) -> ScanError {
    ScanError::new(
        "classification_model_storage_unavailable",
        error.to_string(),
    )
}

#[cfg(test)]
mod tests;
