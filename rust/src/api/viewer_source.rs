use std::sync::Mutex;

use crate::application::{self, ViewerSourceRequest};
use crate::domain::ScanError;

#[flutter_rust_bridge::frb(opaque)]
pub struct ViewerSourceReadLease {
    inner: Mutex<Option<application::ViewerSourceLease>>,
}

pub fn acquire_viewer_source(
    request: ViewerSourceRequest,
) -> Result<ViewerSourceReadLease, ScanError> {
    application::acquire_viewer_source(request).map(|lease| ViewerSourceReadLease {
        inner: Mutex::new(Some(lease)),
    })
}

impl ViewerSourceReadLease {
    pub fn source_path(&self) -> Result<String, ScanError> {
        let inner = self.inner.lock().map_err(|_| lease_error())?;
        inner
            .as_ref()
            .map(|lease| lease.path().to_owned())
            .ok_or_else(lease_error)
    }

    pub fn close(&self) -> Result<(), ScanError> {
        let lease = self.inner.lock().map_err(|_| lease_error())?.take();
        drop(lease);
        Ok(())
    }
}

fn lease_error() -> ScanError {
    ScanError::new(
        "viewer_source_lease_closed",
        "The original-image read lease is no longer available",
    )
}
