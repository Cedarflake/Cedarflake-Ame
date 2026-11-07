#[cfg(windows)]
mod artifacts;
mod backend;
pub use backend::EncoderBackend;
#[cfg(windows)]
mod device;
#[cfg(test)]
mod distribution_tests;
#[cfg(windows)]
mod dll_search;
#[cfg(windows)]
mod encoder;
#[cfg(windows)]
mod factory;
#[cfg(windows)]
pub(crate) mod installation;
mod manifest;
#[cfg(windows)]
mod package;
#[cfg(windows)]
pub(crate) use package::LocalClassificationModelPackage;
#[cfg(windows)]
mod precision;
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
#[cfg(all(windows, test))]
pub(crate) use manifest::compatibility as selected_model_compatibility;
#[cfg(windows)]
pub use process::{WorkerConfiguration, WorkerSession};

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
