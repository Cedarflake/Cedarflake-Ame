use crate::domain::application_update::{
    ApplicationRelease, ApplicationUpdateChannel, ApplicationUpdateDelivery,
    ApplicationUpdateError, ApplicationUpdateFailureCode, ApplicationUpdateResult,
};

pub(crate) trait ApplicationUpdateSource: Send + Sync {
    fn delivery(&self) -> ApplicationUpdateDelivery;
    fn open_store(&self) -> Result<(), ApplicationUpdateError> {
        Err(ApplicationUpdateError::new(
            ApplicationUpdateFailureCode::Unavailable,
            "This installation does not use Microsoft Store updates",
        ))
    }
    fn take_result(&self) -> Result<Option<ApplicationUpdateResult>, ApplicationUpdateError>;

    fn check(
        &self,
        channel: ApplicationUpdateChannel,
    ) -> Result<Option<Box<dyn ApplicationUpdatePackage>>, ApplicationUpdateError>;
}

pub(crate) trait ApplicationUpdatePackage: Send + Sync {
    fn release(&self) -> ApplicationRelease;

    fn download(
        &self,
        observer: &dyn ApplicationDownloadObserver,
    ) -> Result<(), ApplicationUpdateError>;

    fn verify(
        &self,
        observer: &dyn ApplicationDownloadObserver,
    ) -> Result<(), ApplicationUpdateError>;

    fn prepare_handoff(&self) -> Result<(), ApplicationUpdateError>;

    fn apply_after_exit(&self) -> Result<(), ApplicationUpdateError>;
}

pub(crate) trait ApplicationDownloadObserver: Send + Sync {
    fn cancelled(&self) -> bool;

    fn received(&self, bytes: u64);

    fn verifying(&self);
}
