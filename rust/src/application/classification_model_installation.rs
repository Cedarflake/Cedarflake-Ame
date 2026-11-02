use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use crate::adapters::DownloadIntent;
use crate::adapters::classification_runtime::installation::{
    PACKAGE_DIRECTORY, VerifiedModelInstallation, check_cancelled, prepare, prepare_bundled,
};
use crate::adapters::{ClassificationInstallation, ClassificationInstallationStore};
use crate::domain::ScanError;
use crate::domain::classification::installation::ModelInstallationProgress;

use super::classification_desktop::installation_path;
use super::classification_runtime::while_retired;

static INSTALLING: AtomicBool = AtomicBool::new(false);

pub(crate) mod operation;
mod recovery;
use recovery::{InstallationSource, PendingDownload, application_data};

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
    pending: Option<PendingDownload>,
    _ownership: InstallationOwnership,
}

pub fn prepare_managed_classification_installation(
    cancelled: &AtomicBool,
    progress: &mut dyn FnMut(ModelInstallationProgress),
) -> Result<PreparedModelInstallation, ScanError> {
    ModelPreparation::begin(InstallationSource::Download, cancelled)?.prepare(cancelled, progress)
}

struct ModelPreparation {
    settings: PathBuf,
    expected: Option<ClassificationInstallation>,
    source: InstallationSource,
    pending: Option<PendingDownload>,
    ownership: InstallationOwnership,
}

impl ModelPreparation {
    fn begin(source: InstallationSource, cancelled: &AtomicBool) -> Result<Self, ScanError> {
        check_cancelled(cancelled)?;
        let ownership = InstallationOwnership::acquire()?;
        let settings = installation_path(&super::storage_paths()?);
        let expected = ClassificationInstallationStore::load(&settings)?;
        let intent = match &source {
            InstallationSource::Download => {
                Some(DownloadIntent::authorize(&settings, PACKAGE_DIRECTORY)?)
            }
            InstallationSource::Resume(pending) => {
                if pending.settings != settings {
                    return Err(path_error(
                        "The pending download belongs to another profile",
                    ));
                }
                pending.intent.require_current(&settings)?;
                Some(pending.intent.clone())
            }
            InstallationSource::Bundled(_) => None,
        };
        let pending = intent.map(|intent| PendingDownload {
            settings: settings.clone(),
            intent,
        });
        Ok(Self {
            settings,
            expected,
            source,
            pending,
            ownership,
        })
    }

    fn prepare(
        self,
        cancelled: &AtomicBool,
        progress: &mut dyn FnMut(ModelInstallationProgress),
    ) -> Result<PreparedModelInstallation, ScanError> {
        let runtime = application_data()?.join("classification-runtime");
        let verified = match &self.source {
            InstallationSource::Bundled(models) => {
                prepare_bundled(models, &runtime, cancelled, progress)?
            }
            InstallationSource::Download | InstallationSource::Resume(_) => {
                prepare(&model_storage()?, &runtime, cancelled, progress)?
            }
        };
        Ok(PreparedModelInstallation {
            settings: self.settings,
            expected: self.expected,
            verified,
            pending: self.pending,
            _ownership: self.ownership,
        })
    }
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
            self.pending.as_ref().map(|pending| &pending.intent),
            cancelled,
        )
    }
}

fn publish_installation(
    settings: &Path,
    expected: Option<&ClassificationInstallation>,
    installation: &ClassificationInstallation,
    intent: Option<&DownloadIntent>,
    cancelled: &AtomicBool,
) -> Result<(), ScanError> {
    while_retired(|| {
        check_cancelled(cancelled)?;
        if intent.is_none() {
            return ClassificationInstallationStore::save_if_unchanged(
                settings,
                expected,
                installation,
                cancelled,
            );
        }
        ClassificationInstallationStore::publish(
            settings,
            expected,
            installation,
            intent,
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
