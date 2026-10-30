use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::domain::ScanError;
use crate::domain::classification::{ImageFeature, ModelCompatibility, TextFeature};
use crate::ports::classification_encoder::{
    ClassificationEncoder, ClassificationEncoderFactory, ClassificationEncoderKind,
};

use super::{EncoderKind, RuntimeError, WorkerConfiguration, WorkerSession, manifest};

pub struct LocalClassificationEncoderFactory {
    pub executable: PathBuf,
    pub model_directory: PathBuf,
    pub runtime_directory: PathBuf,
}

impl ClassificationEncoderFactory for LocalClassificationEncoderFactory {
    type Encoder = LocalClassificationEncoder;

    fn compatibility(&self) -> ModelCompatibility {
        manifest::compatibility()
    }

    fn create(
        &self,
        kind: ClassificationEncoderKind,
    ) -> Result<LocalClassificationEncoder, ScanError> {
        Ok(LocalClassificationEncoder {
            configuration: Some(WorkerConfiguration {
                executable: self.executable.clone(),
                model_directory: self.model_directory.clone(),
                runtime_directory: self.runtime_directory.clone(),
                encoder: match kind {
                    ClassificationEncoderKind::Image => EncoderKind::Image,
                    ClassificationEncoderKind::Text => EncoderKind::Text,
                },
            }),
            session: None,
        })
    }
}

pub struct LocalClassificationEncoder {
    configuration: Option<WorkerConfiguration>,
    session: Option<WorkerSession>,
}

impl ClassificationEncoder for LocalClassificationEncoder {
    fn initialize(&mut self, cancelled: &AtomicBool) -> Result<(), ScanError> {
        if cancelled.load(Ordering::Acquire) {
            return Err(runtime_error(RuntimeError::Cancelled));
        }
        let configuration = self
            .configuration
            .take()
            .ok_or_else(|| runtime_error(RuntimeError::Configuration))?;
        self.session = Some(WorkerSession::prepare(&configuration).map_err(runtime_error)?);
        self.session
            .as_mut()
            .expect("the process is owned before initialization")
            .initialize(configuration, cancelled)
            .map_err(runtime_error)
    }

    fn encode_prepared_image(
        &mut self,
        pixels: &[f32],
        cancelled: &AtomicBool,
    ) -> Result<ImageFeature, ScanError> {
        self.session
            .as_mut()
            .ok_or_else(|| runtime_error(RuntimeError::RuntimeUnavailable))?
            .encode_prepared_image(pixels, cancelled)
            .map_err(runtime_error)
    }

    fn encode_description(
        &mut self,
        description: &str,
        cancelled: &AtomicBool,
    ) -> Result<TextFeature, ScanError> {
        self.session
            .as_mut()
            .ok_or_else(|| runtime_error(RuntimeError::RuntimeUnavailable))?
            .encode_description(description, cancelled)
            .map_err(runtime_error)
    }

    fn retire(&mut self) -> Result<(), ScanError> {
        if let Some(session) = &mut self.session {
            session.retire_process().map_err(runtime_error)?;
        }
        Ok(())
    }

    fn is_retired(&self) -> bool {
        self.session.as_ref().is_none_or(WorkerSession::is_retired)
    }
}

fn runtime_error(error: RuntimeError) -> ScanError {
    let code = match error {
        RuntimeError::Cancelled => "classification_cancelled",
        RuntimeError::ArtifactMismatch | RuntimeError::UnsafeArtifact => {
            "classification_artifact_rejected"
        }
        RuntimeError::Configuration | RuntimeError::InvalidInput => "classification_input_rejected",
        RuntimeError::RuntimeUnavailable => "classification_runtime_unavailable",
        RuntimeError::TimedOut => "classification_worker_timeout",
        RuntimeError::Io(_) | RuntimeError::Exited => "classification_worker_exited",
        RuntimeError::Inference | RuntimeError::Protocol => "classification_worker_failed",
    };
    ScanError::new(code, error.to_string())
}
