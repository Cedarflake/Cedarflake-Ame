use crate::domain::ScanError;

#[flutter_rust_bridge::frb(opaque)]
pub struct PreparedClassificationMaintenance {
    #[cfg(all(windows, feature = "classification-worker"))]
    pub(crate) inner: crate::application::classification_desktop::ClassificationDesktopMaintenance,
}

impl PreparedClassificationMaintenance {
    pub fn commit(&self) -> Result<bool, ScanError> {
        #[cfg(all(windows, feature = "classification-worker"))]
        {
            self.inner.commit()
        }
        #[cfg(not(all(windows, feature = "classification-worker")))]
        {
            Err(super::classification::unsupported())
        }
    }

    #[flutter_rust_bridge::frb(sync)]
    pub fn cancel(&self) {
        #[cfg(all(windows, feature = "classification-worker"))]
        self.inner.cancel();
    }
}
