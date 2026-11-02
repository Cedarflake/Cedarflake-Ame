use crate::domain::ScanError;
use crate::domain::classification::installation::ModelInstallationProgress;

#[cfg(all(windows, feature = "classification-worker"))]
use crate::application::classification_model_installation::operation::ModelInstallationOperation;

#[flutter_rust_bridge::frb(opaque)]
pub struct DesktopModelInstallation {
    #[cfg(all(windows, feature = "classification-worker"))]
    inner: ModelInstallationOperation,
}

#[flutter_rust_bridge::frb(sync)]
pub fn create_model_installation() -> Result<DesktopModelInstallation, ScanError> {
    #[cfg(all(windows, feature = "classification-worker"))]
    {
        Ok(DesktopModelInstallation {
            inner: ModelInstallationOperation::new(),
        })
    }
    #[cfg(not(all(windows, feature = "classification-worker")))]
    {
        Err(super::classification::unsupported())
    }
}

pub fn recover_model_installation() -> Result<Option<DesktopModelInstallation>, ScanError> {
    #[cfg(all(windows, feature = "classification-worker"))]
    {
        Ok(ModelInstallationOperation::recover()?.map(|inner| DesktopModelInstallation { inner }))
    }
    #[cfg(not(all(windows, feature = "classification-worker")))]
    {
        Err(super::classification::unsupported())
    }
}

impl DesktopModelInstallation {
    pub fn discard(&self) -> Result<(), ScanError> {
        #[cfg(all(windows, feature = "classification-worker"))]
        {
            self.inner.discard()
        }
        #[cfg(not(all(windows, feature = "classification-worker")))]
        {
            Err(super::classification::unsupported())
        }
    }
    pub fn prepare(&self) -> Result<(), ScanError> {
        #[cfg(all(windows, feature = "classification-worker"))]
        {
            self.inner.prepare()
        }
        #[cfg(not(all(windows, feature = "classification-worker")))]
        {
            Err(super::classification::unsupported())
        }
    }

    #[flutter_rust_bridge::frb(sync)]
    pub fn progress(&self) -> Result<Option<ModelInstallationProgress>, ScanError> {
        #[cfg(all(windows, feature = "classification-worker"))]
        {
            self.inner.progress()
        }
        #[cfg(not(all(windows, feature = "classification-worker")))]
        {
            Err(super::classification::unsupported())
        }
    }

    #[flutter_rust_bridge::frb(sync)]
    pub fn cancel(&self) -> Result<(), ScanError> {
        #[cfg(all(windows, feature = "classification-worker"))]
        {
            self.inner.cancel()
        }
        #[cfg(not(all(windows, feature = "classification-worker")))]
        {
            Err(super::classification::unsupported())
        }
    }

    pub fn commit(&self) -> Result<(), ScanError> {
        #[cfg(all(windows, feature = "classification-worker"))]
        {
            self.inner.commit()
        }
        #[cfg(not(all(windows, feature = "classification-worker")))]
        {
            Err(super::classification::unsupported())
        }
    }
}
