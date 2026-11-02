use std::path::PathBuf;

use crate::adapters::ClassificationInstallationStore;
use crate::adapters::DownloadIntent;
use crate::adapters::classification_runtime::installation::PACKAGE_DIRECTORY;
use crate::domain::ScanError;

use super::super::classification_desktop::installation_path;
use super::path_error;

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
        self.intent.cancel(&self.settings)
    }
}

pub(super) fn application_data() -> Result<PathBuf, ScanError> {
    std::env::current_exe()
        .map_err(path_error)?
        .parent()
        .map(|parent| parent.join("data"))
        .ok_or_else(|| path_error("Missing application directory"))
}

pub(super) fn pending_source() -> Result<Option<InstallationSource>, ScanError> {
    let settings = installation_path(&super::super::storage_paths()?);
    if let Some(intent) = DownloadIntent::pending(&settings, PACKAGE_DIRECTORY)? {
        return Ok(Some(InstallationSource::Resume(PendingDownload {
            settings,
            intent,
        })));
    }
    if ClassificationInstallationStore::load(&settings)?.is_some() {
        return Ok(None);
    }
    let models = application_data()?
        .join("classification-models")
        .join(PACKAGE_DIRECTORY);
    if models.try_exists().map_err(path_error)? {
        return Ok(Some(InstallationSource::Bundled(models)));
    }
    Ok(None)
}
