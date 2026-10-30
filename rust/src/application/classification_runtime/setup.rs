use crate::domain::ScanError;
use crate::domain::classification::{ClassificationRun, ScreeningProfile, Taxonomy};
use crate::ports::classification_encoder::ClassificationEncoderFactory;
use crate::ports::classification_features::ClassificationFeatureRepository;
use std::sync::atomic::AtomicBool;

use super::super::classification_features::prepare_classification_matcher;
use super::super::classification_residency::{ClassificationEncoderResidency, ResidencyClock};
use super::super::classification_screening::ScreeningClassifier;

pub struct ClassificationAnalysisSetup {
    pub(super) run: ClassificationRun,
    primary: Taxonomy,
    refinement: Option<(Taxonomy, ScreeningProfile)>,
}

impl ClassificationAnalysisSetup {
    pub fn new(
        run: ClassificationRun,
        primary: Taxonomy,
        refinement: Option<(Taxonomy, ScreeningProfile)>,
    ) -> Result<Self, ScanError> {
        if primary.revision() != run.primary_rules() {
            return Err(mismatch());
        }
        match (&refinement, run.refinement()) {
            (None, None) => {}
            (Some((rules, profile)), Some(identity)) => {
                if rules.revision() != identity.rules
                    || profile.revision() != identity.profile
                    || profile.identity().model != run.model()
                    || profile.identity().primary_rules != primary.revision()
                    || profile.identity().refinement_rules != rules.revision()
                    || primary.revision() == rules.revision()
                    || !profile.matches_categories(primary.categories())
                    || !primary
                        .categories()
                        .iter()
                        .map(|category| category.id())
                        .eq(rules.categories().iter().map(|category| category.id()))
                {
                    return Err(mismatch());
                }
            }
            _ => return Err(mismatch()),
        }
        Ok(Self {
            run,
            primary,
            refinement,
        })
    }

    pub(super) fn prepare<F: ClassificationEncoderFactory, C: ResidencyClock>(
        &self,
        residency: &mut ClassificationEncoderResidency<F, C>,
        cache: &mut impl ClassificationFeatureRepository,
        cancelled: &AtomicBool,
    ) -> Result<ScreeningClassifier, ScanError> {
        if residency.compatibility() != self.run.model() {
            return Err(mismatch());
        }
        let primary =
            prepare_classification_matcher(self.primary.clone(), residency, cache, cancelled)?;
        let classifier = match &self.refinement {
            Some((rules, profile)) => {
                let refinement =
                    prepare_classification_matcher(rules.clone(), residency, cache, cancelled)?;
                ScreeningClassifier::with_profile(primary, refinement, profile.clone())
                    .map_err(|_| mismatch())?
            }
            None => ScreeningClassifier::uncalibrated(primary),
        };
        if classifier
            .describe_run(self.run.id().clone())
            .map_err(|_| mismatch())?
            != self.run
        {
            return Err(mismatch());
        }
        Ok(classifier)
    }
}

pub(super) fn mismatch() -> ScanError {
    ScanError::new(
        "classification_analysis_mismatch",
        "The analysis setup does not match its stored run",
    )
}
