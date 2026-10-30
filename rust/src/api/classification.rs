use crate::domain::ScanError;

#[cfg(all(windows, feature = "classification-worker"))]
use crate::application::{
    classification_desktop, classification_dispatch::ClassificationForegroundState,
};

#[flutter_rust_bridge::frb(opaque)]
pub struct DesktopClassificationSession {
    #[cfg(all(windows, feature = "classification-worker"))]
    inner: classification_desktop::ClassificationDesktopSession,
}

pub fn open_classification_session() -> Result<Option<DesktopClassificationSession>, ScanError> {
    #[cfg(all(windows, feature = "classification-worker"))]
    {
        classification_desktop::open()
            .map(|session| session.map(|inner| DesktopClassificationSession { inner }))
    }
    #[cfg(not(all(windows, feature = "classification-worker")))]
    {
        Ok(None)
    }
}

pub fn select_classification_installation(
    models: String,
    runtime: String,
) -> Result<(), ScanError> {
    #[cfg(all(windows, feature = "classification-worker"))]
    {
        classification_desktop::save_installation(models, runtime)
    }
    #[cfg(not(all(windows, feature = "classification-worker")))]
    {
        let _ = (models, runtime);
        Err(unsupported())
    }
}

impl DesktopClassificationSession {
    pub fn run(&self) -> Result<(), ScanError> {
        #[cfg(all(windows, feature = "classification-worker"))]
        {
            self.inner.run()
        }
        #[cfg(not(all(windows, feature = "classification-worker")))]
        {
            Err(unsupported())
        }
    }

    #[flutter_rust_bridge::frb(sync)]
    pub fn foreground_changed(
        &self,
        reading_sources: bool,
        importing: bool,
        suspended: bool,
        memory_pressure: bool,
    ) -> Result<(), ScanError> {
        #[cfg(all(windows, feature = "classification-worker"))]
        {
            self.inner
                .foreground_changed(ClassificationForegroundState {
                    reading_sources,
                    importing,
                    suspended,
                    memory_pressure,
                })
        }
        #[cfg(not(all(windows, feature = "classification-worker")))]
        {
            let _ = (reading_sources, importing, suspended, memory_pressure);
            Err(unsupported())
        }
    }

    #[flutter_rust_bridge::frb(sync)]
    pub fn browsing_activity(&self) -> Result<(), ScanError> {
        #[cfg(all(windows, feature = "classification-worker"))]
        {
            self.inner.browsing_activity()
        }
        #[cfg(not(all(windows, feature = "classification-worker")))]
        {
            Err(unsupported())
        }
    }

    #[flutter_rust_bridge::frb(sync)]
    pub fn catalog_changed(&self) -> Result<(), ScanError> {
        #[cfg(all(windows, feature = "classification-worker"))]
        {
            self.inner.catalog_changed()
        }
        #[cfg(not(all(windows, feature = "classification-worker")))]
        {
            Err(unsupported())
        }
    }

    pub fn root_available(&self, root_id: String, generation: u64) -> Result<(), ScanError> {
        #[cfg(all(windows, feature = "classification-worker"))]
        {
            self.inner.root_available(root_id, generation)
        }
        #[cfg(not(all(windows, feature = "classification-worker")))]
        {
            let _ = (root_id, generation);
            Err(unsupported())
        }
    }

    #[flutter_rust_bridge::frb(sync)]
    pub fn request_close(&self) -> Result<(), ScanError> {
        #[cfg(all(windows, feature = "classification-worker"))]
        {
            self.inner.request_close()
        }
        #[cfg(not(all(windows, feature = "classification-worker")))]
        {
            Err(unsupported())
        }
    }

    pub fn close(&self) -> Result<(), ScanError> {
        #[cfg(all(windows, feature = "classification-worker"))]
        {
            self.inner.close()
        }
        #[cfg(not(all(windows, feature = "classification-worker")))]
        {
            Err(unsupported())
        }
    }
}

#[cfg(not(all(windows, feature = "classification-worker")))]
pub(super) fn unsupported() -> ScanError {
    ScanError::new(
        "classification_unsupported",
        "Local classification requires the Windows worker",
    )
}
