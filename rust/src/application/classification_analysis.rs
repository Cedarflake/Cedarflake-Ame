use std::sync::atomic::{AtomicBool, Ordering};

use crate::domain::ScanError;
use crate::domain::classification::work::{
    ClassificationAnalysisResult, ClassificationWorkLease, ClassificationWorkTarget,
};
use crate::domain::classification::{ClassificationRecordId, ClassificationRevision};
use crate::ports::classification_analysis::{
    ClassificationAnalysisRepository, ClassificationImageLease, ClassificationImageReader,
};
use crate::ports::classification_encoder::ClassificationEncoderFactory;
use crate::ports::classification_features::ClassificationFeatureRepository;

use super::classification_residency::{ClassificationEncoderResidency, ResidencyClock};
use super::classification_screening::ScreeningClassifier;

#[cfg(all(windows, feature = "classification-worker"))]
pub fn analyze_classification_content<C: ResidencyClock>(
    database_path: &std::path::Path,
    lease: &ClassificationWorkLease,
    classifier: &ScreeningClassifier,
    residency: &mut ClassificationEncoderResidency<
        crate::adapters::classification_runtime::LocalClassificationEncoderFactory,
        C,
    >,
    cache: &mut impl ClassificationFeatureRepository,
    cancelled: &AtomicBool,
) -> Result<ClassificationAnalysisResult, ScanError> {
    if cancelled.load(Ordering::Acquire) {
        return Ok(ClassificationAnalysisResult::Cancelled);
    }
    let mut catalog = super::catalog_session::open_catalog(
        database_path,
        crate::domain::LibraryChangeLane::Recovery,
    )?;
    analyze(
        &mut catalog,
        &crate::adapters::classification_image_input::GuardedClassificationImageReader,
        lease,
        classifier,
        residency,
        cache,
        cancelled,
    )
}

pub(crate) fn analyze<F: ClassificationEncoderFactory, C: ResidencyClock>(
    repository: &mut impl ClassificationAnalysisRepository,
    reader: &impl ClassificationImageReader,
    lease: &ClassificationWorkLease,
    classifier: &ScreeningClassifier,
    residency: &mut ClassificationEncoderResidency<F, C>,
    cache: &mut impl ClassificationFeatureRepository,
    cancelled: &AtomicBool,
) -> Result<ClassificationAnalysisResult, ScanError> {
    if cancelled.load(Ordering::Acquire) {
        return Ok(ClassificationAnalysisResult::Cancelled);
    }
    let ClassificationWorkTarget::AnalyzeContent { content, .. } = &lease.target else {
        return Err(incompatible());
    };
    let Some(prepared) = repository.prepare_classification_analysis(lease, cancelled)? else {
        return Ok(interrupted(cancelled));
    };
    let run = classifier
        .describe_run(lease.run_id.clone())
        .map_err(|_| incompatible())?;
    if run != prepared.run || run.model() != residency.compatibility() {
        return Err(incompatible());
    }
    let mut source = match reader.open(&prepared.candidate, content, cancelled) {
        Ok(source) => source,
        Err(_) if cancelled.load(Ordering::Acquire) => {
            return Ok(ClassificationAnalysisResult::Cancelled);
        }
        Err(error) => return Err(error),
    };
    let cached = cache.load_image_feature(run.model(), content)?;
    let used_cached_feature = cached.is_some();
    let feature = match cached {
        Some(feature) => feature,
        None => {
            let pixels = match source.prepare_pixels(cancelled) {
                Ok(pixels) => pixels,
                Err(_) if cancelled.load(Ordering::Acquire) => {
                    return Ok(ClassificationAnalysisResult::Cancelled);
                }
                Err(error) => return Err(error),
            };
            if !repository.classification_analysis_is_current(lease, cancelled)? {
                return Ok(interrupted(cancelled));
            }
            match residency.encode_prepared_image(&pixels, cancelled) {
                Ok(feature) => feature,
                Err(_) if cancelled.load(Ordering::Acquire) => {
                    return Ok(ClassificationAnalysisResult::Cancelled);
                }
                Err(error) => return Err(error),
            }
        }
    };
    if cancelled.load(Ordering::Acquire) {
        return Ok(ClassificationAnalysisResult::Cancelled);
    }
    source.revalidate()?;
    if !repository.classification_analysis_is_current(lease, cancelled)? {
        return Ok(interrupted(cancelled));
    }
    let result = classifier.classify(&feature).map_err(|_| incompatible())?;
    let identity = ClassificationRevision::derive(
        "ame.classification.work-prediction.v1",
        [
            lease.run_id.as_str().as_bytes(),
            lease.session_id.as_str().as_bytes(),
            &lease.work_id.to_le_bytes(),
            &lease.lease_id.to_le_bytes(),
            &lease.epoch.to_le_bytes(),
        ],
    );
    let prediction_id = ClassificationRecordId::new(
        identity
            .bytes()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect(),
    )
    .map_err(|_| incompatible())?;
    let evidence = result
        .into_evidence(prediction_id, run.clone(), content.clone())
        .map_err(|_| incompatible())?;
    if !used_cached_feature {
        cache.store_image_feature(run.model(), content, &feature)?;
    }
    if cancelled.load(Ordering::Acquire) {
        return Ok(ClassificationAnalysisResult::Cancelled);
    }
    source.revalidate()?;
    let committed = repository.commit_classification_analysis(lease, &evidence, cancelled)?;
    // Keep the read-only file and namespace lease until publication has committed or rolled back.
    drop(source);
    if committed {
        Ok(ClassificationAnalysisResult::Completed {
            used_cached_feature,
        })
    } else {
        Ok(interrupted(cancelled))
    }
}

fn interrupted(cancelled: &AtomicBool) -> ClassificationAnalysisResult {
    if cancelled.load(Ordering::Acquire) {
        ClassificationAnalysisResult::Cancelled
    } else {
        ClassificationAnalysisResult::Superseded
    }
}

fn incompatible() -> ScanError {
    ScanError::new(
        "classification_analysis_mismatch",
        "The classification task and encoder configuration do not match",
    )
}
