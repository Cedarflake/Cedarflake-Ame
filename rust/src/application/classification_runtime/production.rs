use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use super::super::analysis_scheduling::AnalysisScheduler;
use super::ClassificationDispatchSignals;
use super::ClassificationRuntimePaths;
use super::device::ClassificationDeviceState;
use crate::domain::ScanError;
use crate::domain::analysis_capacity::AnalysisCapacity;
use crate::ports::analysis_capacity::AnalysisCapacityProvider;
use crate::ports::classification_vocabulary::ManagedClassificationVocabulary;

struct NativeCapacity(ClassificationDispatchSignals);

impl AnalysisCapacityProvider for NativeCapacity {
    fn capacity(&self) -> Option<AnalysisCapacity> {
        crate::adapters::analysis_capacity::current_capacity()
    }

    fn memory_pressure(&self) -> bool {
        self.0.memory_pressure().unwrap_or(true)
    }
}

pub(super) fn analysis_scheduler(
    factory: crate::adapters::classification_runtime::LocalClassificationEncoderFactory,
    signals: ClassificationDispatchSignals,
) -> Arc<
    AnalysisScheduler<crate::adapters::classification_runtime::LocalClassificationEncoderFactory>,
> {
    let gpu = matches!(
        factory.backend,
        crate::adapters::classification_runtime::EncoderBackend::DirectMl { .. }
    );
    AnalysisScheduler::new(factory, Box::new(NativeCapacity(signals)), gpu)
}

pub(super) fn semantic_readers() -> (
    impl crate::ports::exact_duplicates::ExactContentReader,
    impl crate::ports::classification_analysis::ClassificationImageReader,
) {
    (
        crate::adapters::GuardedExactContentReader,
        crate::adapters::classification_image_input::GuardedClassificationImageReader,
    )
}

pub(super) fn recover_semantic_features(directory: &std::path::Path) -> Result<(), ScanError> {
    crate::adapters::SqliteSemanticFeatureStore::recover(directory)
}

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
