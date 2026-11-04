use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use crate::domain::ScanError;
use crate::domain::classification::installation::{
    ClassificationInstallation, DownloadIntent, ModelInstallationProgress, check_cancelled,
};
use crate::ports::classification_installation::ClassificationInstallationRepository;
use crate::ports::classification_model_package::{
    ClassificationModelPackage, PreparedClassificationModel,
};

use super::classification_desktop::installation_path;
use super::classification_runtime::while_retired;

static INSTALLING: AtomicBool = AtomicBool::new(false);

pub(crate) mod operation;
pub(super) mod production;
use production::{installation_repository, model_package, model_storage};
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
    verified: Box<dyn PreparedClassificationModel>,
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
        let expected = installation_repository().load(&settings)?;
        let intent = match &source {
            InstallationSource::Download => Some(
                installation_repository()
                    .authorize_download(&settings, model_package().package_directory())?,
            ),
            InstallationSource::Resume(pending) => {
                if pending.settings != settings {
                    return Err(path_error(
                        "The pending download belongs to another profile",
                    ));
                }
                installation_repository().require_current_download(&settings, &pending.intent)?;
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
                model_package().prepare_bundled(models, &runtime, cancelled, progress)?
            }
            InstallationSource::Download | InstallationSource::Resume(_) => model_package()
                .prepare_download(&model_storage()?, &runtime, cancelled, progress)?,
        };
        Ok(PreparedModelInstallation {
            settings: self.settings,
            expected: self.expected,
            verified: Box::new(verified),
            pending: self.pending,
            _ownership: self.ownership,
        })
    }
}

impl PreparedModelInstallation {
    pub fn commit(self, cancelled: &AtomicBool) -> Result<(), ScanError> {
        let installation = self.verified.installation();
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
            return installation_repository().save_if_unchanged(
                settings,
                expected,
                installation,
                cancelled,
            );
        }
        installation_repository().publish(settings, expected, installation, intent, cancelled)
    })
}

fn path_error(error: impl std::fmt::Display) -> ScanError {
    ScanError::new(
        "classification_model_storage_unavailable",
        error.to_string(),
    )
}

#[cfg(test)]
mod tests;
