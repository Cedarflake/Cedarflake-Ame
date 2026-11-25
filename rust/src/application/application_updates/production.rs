use crate::domain::application_update::ApplicationUpdateError;
use crate::ports::application_updates::ApplicationUpdateSource;

pub(super) fn open(
    version: String,
    build: u64,
) -> Result<Box<dyn ApplicationUpdateSource>, ApplicationUpdateError> {
    #[cfg(windows)]
    {
        Ok(Box::new(
            crate::adapters::application_updates::WindowsApplicationUpdates::new(version, build)?,
        ))
    }
    #[cfg(not(windows))]
    {
        let _ = (version, build);
        Err(ApplicationUpdateError::new(
            crate::domain::application_update::ApplicationUpdateFailureCode::Unavailable,
            "Application updates are supported on Windows",
        ))
    }
}
