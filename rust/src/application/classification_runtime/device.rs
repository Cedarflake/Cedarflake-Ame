use std::sync::Mutex;

use crate::domain::ScanError;
use crate::domain::classification_acceleration::ClassificationDeviceStatus;
use crate::ports::classification_acceleration::ClassificationDeviceObserver;

#[derive(Default)]
pub(super) struct ClassificationDeviceState(Mutex<Option<ClassificationDeviceStatus>>);

impl ClassificationDeviceState {
    pub(super) fn snapshot(&self) -> Result<Option<ClassificationDeviceStatus>, ScanError> {
        self.0.lock().map(|status| status.clone()).map_err(|_| {
            ScanError::new(
                "classification_device_state_unavailable",
                "The analysis device status is unavailable",
            )
        })
    }
}

impl ClassificationDeviceObserver for ClassificationDeviceState {
    fn device_changed(&self, status: ClassificationDeviceStatus) {
        // The callback stores a value only; a poisoned observer cannot authorize model work.
        if let Ok(mut current) = self.0.lock() {
            *current = Some(status);
        }
    }
}
