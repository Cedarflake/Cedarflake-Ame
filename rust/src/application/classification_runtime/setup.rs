use crate::domain::ScanError;
use crate::domain::classification::ClassificationAnalysisSetup;
use crate::ports::classification_features::ClassificationFeatureRepository;
use crate::ports::image_feature_encoding::ImageFeatureEncoding;
use std::sync::atomic::AtomicBool;

use super::super::classification_features::prepare_classification_matcher;
use super::super::classification_screening::ScreeningClassifier;

pub(super) fn prepare<E: ImageFeatureEncoding>(
    setup: &ClassificationAnalysisSetup,
    residency: &mut E,
    cache: &mut impl ClassificationFeatureRepository,
    cancelled: &AtomicBool,
) -> Result<ScreeningClassifier, ScanError> {
    if residency.compatibility() != setup.run().model() {
        return Err(mismatch());
    }
    let primary =
        prepare_classification_matcher(setup.primary().clone(), residency, cache, cancelled)?;
    let classifier = match setup.refinement() {
        Some((rules, profile)) => {
            let refinement =
                prepare_classification_matcher(rules.clone(), residency, cache, cancelled)?;
            ScreeningClassifier::with_profile(primary, refinement, profile.clone())
                .map_err(|_| mismatch())?
        }
        None => ScreeningClassifier::uncalibrated(primary),
    };
    if classifier
        .describe_run(setup.run().id().clone())
        .map_err(|_| mismatch())?
        != *setup.run()
    {
        return Err(mismatch());
    }
    Ok(classifier)
}

pub(super) fn mismatch() -> ScanError {
    ScanError::new(
        "classification_analysis_mismatch",
        "The analysis setup does not match its stored run",
    )
}
