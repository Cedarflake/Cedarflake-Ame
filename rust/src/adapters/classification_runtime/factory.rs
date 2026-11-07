use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::domain::ScanError;
use crate::domain::classification::{ImageFeature, ModelCompatibility, TextFeature};
use crate::domain::classification_acceleration::{
    ClassificationComputeDevice, ClassificationDeviceStatus,
};
use crate::ports::classification_acceleration::ClassificationDeviceObserver;
use crate::ports::classification_encoder::{
    ClassificationEncoder, ClassificationEncoderFactory, ClassificationEncoderKind,
};

use super::{
    EncoderBackend, EncoderKind, RuntimeError, WorkerConfiguration, WorkerSession, manifest,
};

#[derive(Default)]
pub struct LocalClassificationEncoderFactory {
    pub executable: PathBuf,
    pub model_directory: PathBuf,
    pub runtime_directory: PathBuf,
    pub backend: EncoderBackend,
    pub observer: Option<Arc<dyn ClassificationDeviceObserver>>,
    pub device_name: Option<String>,
}

impl LocalClassificationEncoderFactory {
    pub fn recognizes_model(&self, model: ModelCompatibility) -> bool {
        model == manifest::compatibility()
            || model == manifest::compatibility_for(EncoderBackend::DirectMl { device_id: 0 })
    }

    pub fn with_gpu_preference(mut self, enabled: bool) -> Self {
        self.backend = EncoderBackend::Cpu;
        self.device_name = None;
        let runtime = self
            .runtime_directory
            .with_file_name("classification-accelerator");
        if enabled
            && runtime.join(super::backend::GPU_RUNTIME.name).is_file()
            && runtime.join(super::backend::DIRECTML.name).is_file()
            && let Some(device) = super::device::preferred_gpu()
        {
            self.backend = EncoderBackend::DirectMl {
                device_id: device.index,
            };
            self.device_name = Some(device.name);
        }
        if let Some(observer) = &self.observer {
            observer.device_changed(ClassificationDeviceStatus {
                device: self.device(),
                active: false,
            });
        }
        self
    }

    fn device(&self) -> ClassificationComputeDevice {
        match self.backend {
            EncoderBackend::Cpu => ClassificationComputeDevice::Cpu,
            EncoderBackend::DirectMl { .. } => ClassificationComputeDevice::Gpu {
                name: self.device_name.clone().unwrap_or_else(|| "GPU".into()),
            },
        }
    }
}

impl ClassificationEncoderFactory for LocalClassificationEncoderFactory {
    type Encoder = LocalClassificationEncoder;

    fn compatibility(&self) -> ModelCompatibility {
        manifest::compatibility_for(self.backend)
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
                backend: if kind == ClassificationEncoderKind::Image {
                    self.backend
                } else {
                    EncoderBackend::Cpu
                },
                encoder: match kind {
                    ClassificationEncoderKind::Image => EncoderKind::Image,
                    ClassificationEncoderKind::Text => EncoderKind::Text,
                },
            }),
            session: None,
            observer: if kind == ClassificationEncoderKind::Image {
                self.observer.clone()
            } else {
                None
            },
            device: self.device(),
        })
    }
}

pub struct LocalClassificationEncoder {
    configuration: Option<WorkerConfiguration>,
    session: Option<WorkerSession>,
    observer: Option<Arc<dyn ClassificationDeviceObserver>>,
    device: ClassificationComputeDevice,
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
            .map_err(runtime_error)?;
        if let Some(observer) = &self.observer {
            observer.device_changed(ClassificationDeviceStatus {
                device: self.device.clone(),
                active: true,
            });
        }
        Ok(())
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
        if let Some(observer) = &self.observer {
            observer.device_changed(ClassificationDeviceStatus {
                device: self.device.clone(),
                active: false,
            });
        }
        Ok(())
    }

    fn is_retired(&self) -> bool {
        self.session.as_ref().is_none_or(WorkerSession::is_retired)
    }
}

pub(super) fn runtime_error(error: RuntimeError) -> ScanError {
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
