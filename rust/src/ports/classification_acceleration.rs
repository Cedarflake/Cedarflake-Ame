use std::path::Path;

use crate::domain::ScanError;
#[cfg(all(windows, feature = "classification-worker"))]
use crate::domain::classification_acceleration::ClassificationDeviceStatus;

pub(crate) trait ClassificationAccelerationRepository {
    fn gpu_enabled(&self, path: &Path) -> Result<bool, ScanError>;
    fn set_gpu_enabled(&self, path: &Path, enabled: bool) -> Result<(), ScanError>;
}

#[cfg(all(windows, feature = "classification-worker"))]
pub trait ClassificationDeviceObserver: Send + Sync {
    fn device_changed(&self, status: ClassificationDeviceStatus);
}
