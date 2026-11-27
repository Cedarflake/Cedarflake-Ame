use crate::application::application_updates::{ApplicationUpdateDownload, ApplicationUpdates};
use crate::domain::application_update::{
    ApplicationDownloadProgress, ApplicationRelease, ApplicationUpdateChannel,
    ApplicationUpdateDelivery, ApplicationUpdateError,
};

#[flutter_rust_bridge::frb(opaque)]
pub struct DesktopApplicationUpdates {
    inner: ApplicationUpdates,
}

#[flutter_rust_bridge::frb(opaque)]
pub struct DesktopApplicationDownload {
    inner: ApplicationUpdateDownload,
}

pub fn open_application_updates(
    version: String,
    build: u64,
) -> Result<DesktopApplicationUpdates, ApplicationUpdateError> {
    Ok(DesktopApplicationUpdates {
        inner: ApplicationUpdates::open(version, build)?,
    })
}

impl DesktopApplicationUpdates {
    pub fn take_result(
        &self,
    ) -> Result<
        Option<crate::domain::application_update::ApplicationUpdateResult>,
        ApplicationUpdateError,
    > {
        self.inner.take_result()
    }
    #[flutter_rust_bridge::frb(sync)]
    pub fn delivery(&self) -> ApplicationUpdateDelivery {
        self.inner.delivery()
    }
    pub fn open_store(&self) -> Result<(), ApplicationUpdateError> {
        self.inner.open_store()
    }
    pub fn check(
        &self,
        channel: ApplicationUpdateChannel,
    ) -> Result<Option<DesktopApplicationDownload>, ApplicationUpdateError> {
        self.inner
            .check(channel)
            .map(|download| download.map(|inner| DesktopApplicationDownload { inner }))
    }
}

impl DesktopApplicationDownload {
    pub fn prepare_handoff(&self) -> Result<(), ApplicationUpdateError> {
        self.inner.prepare_handoff()
    }
    #[flutter_rust_bridge::frb(sync)]
    pub fn prepare_download(&self) -> Result<(), ApplicationUpdateError> {
        self.inner.prepare_download()
    }
    #[flutter_rust_bridge::frb(sync)]
    pub fn release(&self) -> ApplicationRelease {
        self.inner.release()
    }
    #[flutter_rust_bridge::frb(sync)]
    pub fn progress(&self) -> ApplicationDownloadProgress {
        self.inner.progress()
    }
    pub fn download(&self) -> Result<(), ApplicationUpdateError> {
        self.inner.download()
    }
    #[flutter_rust_bridge::frb(sync)]
    pub fn cancel(&self) {
        self.inner.cancel();
    }
    pub fn verify(&self) -> Result<(), ApplicationUpdateError> {
        self.inner.verify()
    }
    pub fn apply_after_exit(&self) -> Result<(), ApplicationUpdateError> {
        self.inner.apply_after_exit()
    }
}
