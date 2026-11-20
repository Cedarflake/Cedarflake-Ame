use std::path::PathBuf;

use crate::domain::ScanError;
use crate::domain::classification::installation::DownloadIntent;
use crate::ports::classification_installation::ClassificationInstallationRepository;
use crate::ports::classification_model_package::ClassificationModelPackage;

use super::super::classification_desktop::installation_path;
use super::path_error;
use super::production::{application_data, bundled_models, installation_repository, model_package};

#[derive(Clone)]
pub(super) enum InstallationSource {
    Download,
    Resume(PendingDownload),
    Bundled(PathBuf),
}

#[derive(Clone)]
pub(super) struct PendingDownload {
    pub settings: PathBuf,
    pub intent: DownloadIntent,
}

impl PendingDownload {
    pub fn cancel(&self) -> Result<(), ScanError> {
        installation_repository().cancel_download(&self.settings, &self.intent)
    }
}

pub(super) fn pending_source() -> Result<Option<InstallationSource>, ScanError> {
    let settings = installation_path(&super::super::storage_paths()?);
    if let Some(intent) = installation_repository()
        .pending_download(&settings, model_package().package_directory())?
    {
        return Ok(Some(InstallationSource::Resume(PendingDownload {
            settings,
            intent,
        })));
    }
    if installation_repository().load(&settings)?.is_some() {
        return Ok(None);
    }
    let models = bundled_models(&application_data()?);
    if models.try_exists().map_err(path_error)? {
        return Ok(Some(InstallationSource::Bundled(models)));
    }
    Ok(None)
}
