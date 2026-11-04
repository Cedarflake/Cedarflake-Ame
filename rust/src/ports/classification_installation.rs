use std::path::Path;
use std::sync::atomic::AtomicBool;

use crate::domain::ScanError;
use crate::domain::classification::installation::{ClassificationInstallation, DownloadIntent};

pub(crate) trait ClassificationInstallationRepository {
    fn load(&self, path: &Path) -> Result<Option<ClassificationInstallation>, ScanError>;
    fn save(&self, path: &Path, installation: &ClassificationInstallation)
    -> Result<(), ScanError>;

    fn save_if_unchanged(
        &self,
        path: &Path,
        expected: Option<&ClassificationInstallation>,
        installation: &ClassificationInstallation,
        cancelled: &AtomicBool,
    ) -> Result<(), ScanError> {
        self.publish(path, expected, installation, None, cancelled)
    }

    /// Revalidates selection, cancellation and download authority in the publication transaction.
    fn publish(
        &self,
        path: &Path,
        expected: Option<&ClassificationInstallation>,
        installation: &ClassificationInstallation,
        intent: Option<&DownloadIntent>,
        cancelled: &AtomicBool,
    ) -> Result<(), ScanError>;

    fn pending_download(
        &self,
        path: &Path,
        package: &str,
    ) -> Result<Option<DownloadIntent>, ScanError>;
    fn authorize_download(&self, path: &Path, package: &str) -> Result<DownloadIntent, ScanError>;
    fn require_current_download(
        &self,
        path: &Path,
        intent: &DownloadIntent,
    ) -> Result<(), ScanError>;
    fn cancel_download(&self, path: &Path, intent: &DownloadIntent) -> Result<(), ScanError>;
}
