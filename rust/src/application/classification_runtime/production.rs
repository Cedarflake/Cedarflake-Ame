use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use super::ClassificationRuntimePaths;
use super::device::ClassificationDeviceState;
use crate::domain::ScanError;
use crate::ports::classification_vocabulary::ManagedClassificationVocabulary;

pub(super) fn encoder_factory(
    paths: &ClassificationRuntimePaths,
    observer: Arc<ClassificationDeviceState>,
) -> crate::adapters::classification_runtime::LocalClassificationEncoderFactory {
    crate::adapters::classification_runtime::LocalClassificationEncoderFactory {
        executable: paths.worker.clone(),
        model_directory: paths.models.clone(),
        runtime_directory: paths.runtime.clone(),
        observer: Some(observer),
        ..Default::default()
    }
    .with_gpu_preference(paths.gpu_enabled)
}

pub(super) fn vocabulary(
    cancelled: &AtomicBool,
) -> Result<impl ManagedClassificationVocabulary + use<>, ScanError> {
    crate::adapters::CuratedClassificationVocabulary::bundled(cancelled)
}
