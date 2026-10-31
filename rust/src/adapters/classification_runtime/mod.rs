#[cfg(windows)]
mod artifacts;
#[cfg(windows)]
mod dll_search;
#[cfg(windows)]
mod encoder;
#[cfg(windows)]
mod factory;
mod manifest;
#[cfg(windows)]
mod process;
mod protocol;
#[cfg(windows)]
mod transport;
#[cfg(windows)]
mod windows_job;
#[cfg(windows)]
mod worker;

use serde::{Deserialize, Serialize};
use std::process::ExitCode;

#[cfg(windows)]
pub use factory::LocalClassificationEncoderFactory;
#[cfg(windows)]
pub(crate) use manifest::compatibility as selected_model_compatibility;
#[cfg(windows)]
pub use process::{WorkerConfiguration, WorkerSession};

#[cfg(windows)]
pub(crate) fn verify_local_installation(
    models: &std::path::Path,
    runtime: &std::path::Path,
) -> Result<(), crate::domain::ScanError> {
    let _image = artifacts::ArtifactSet::open(models, runtime, EncoderKind::Image)
        .map_err(factory::runtime_error)?;
    let _text = artifacts::ArtifactSet::open(models, runtime, EncoderKind::Text)
        .map_err(factory::runtime_error)?;
    Ok(())
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EncoderKind {
    Image,
    Text,
}

#[derive(Debug, thiserror::Error)]
pub enum RuntimeError {
    #[error("Classification worker I/O failed")]
    Io(#[from] std::io::Error),
    #[error("Classification worker protocol was rejected")]
    Protocol,
    #[error("Classification artifact verification failed")]
    ArtifactMismatch,
    #[error("Classification artifacts must be local regular files")]
    UnsafeArtifact,
    #[error("Classification runtime configuration was rejected")]
    Configuration,
    #[error("Classification input was rejected")]
    InvalidInput,
    #[error("Classification runtime could not load")]
    RuntimeUnavailable,
    #[error("Classification inference failed")]
    Inference,
    #[error("Classification was cancelled")]
    Cancelled,
    #[error("Classification worker timed out")]
    TimedOut,
    #[error("Classification worker has exited")]
    Exited,
}

pub const fn prepared_image_length() -> usize {
    manifest::IMAGE_VALUES
}

pub fn run_worker_process() -> ExitCode {
    #[cfg(windows)]
    {
        worker::run()
    }
    #[cfg(not(windows))]
    {
        ExitCode::FAILURE
    }
}
