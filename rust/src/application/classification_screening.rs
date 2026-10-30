use crate::domain::classification::{
    ClassificationError, ClassificationRevision, ImageFeature, ScreeningIdentity, ScreeningProfile,
};

use super::classification_matching::{CategoryMatcher, RankedPrediction};

pub struct ScreeningClassifier {
    primary: CategoryMatcher,
    refinement: Option<AdmittedRefinement>,
}

struct AdmittedRefinement {
    matcher: CategoryMatcher,
    profile: ScreeningProfile,
}

impl ScreeningClassifier {
    pub fn uncalibrated(primary: CategoryMatcher) -> Self {
        Self {
            primary,
            refinement: None,
        }
    }

    pub fn with_profile(
        primary: CategoryMatcher,
        refinement: CategoryMatcher,
        profile: ScreeningProfile,
    ) -> Result<Self, ClassificationError> {
        let primary_categories = primary.taxonomy().categories().iter().map(|item| item.id());
        let refinement_categories = refinement
            .taxonomy()
            .categories()
            .iter()
            .map(|item| item.id());
        if primary.model() != refinement.model()
            || !primary_categories.eq(refinement_categories)
            || primary.taxonomy().revision() == refinement.taxonomy().revision()
        {
            return Err(ClassificationError::IncompatibleRefinement);
        }
        let identity = ScreeningIdentity {
            model: primary.model(),
            primary_rules: primary.taxonomy().revision(),
            refinement_rules: refinement.taxonomy().revision(),
        };
        if profile.identity() != identity
            || !profile.matches_categories(primary.taxonomy().categories())
        {
            return Err(ClassificationError::IncompatibleScreeningProfile);
        }
        Ok(Self {
            primary,
            refinement: Some(AdmittedRefinement {
                matcher: refinement,
                profile,
            }),
        })
    }

    pub fn classify(&self, image: &ImageFeature) -> Result<ScreeningResult, ClassificationError> {
        let initial = self.primary.rank(image)?;
        let Some(refinement) = &self.refinement else {
            return Ok(ScreeningResult {
                initial,
                refined: None,
                profile: None,
                review: ReviewRecommendation::Uncalibrated,
            });
        };
        let thresholds = refinement
            .profile
            .thresholds_for(&initial.leading().category_id)
            .ok_or(ClassificationError::IncompatibleScreeningProfile)?;
        let refined = match initial.margin() {
            Some(margin) if margin == 0.0 || margin < thresholds.rescreen_below() => {
                Some(refinement.matcher.rank(image)?)
            }
            _ => None,
        };
        let suggestion = refined.as_ref().unwrap_or(&initial);
        let review_threshold = refinement
            .profile
            .thresholds_for(&suggestion.leading().category_id)
            .ok_or(ClassificationError::IncompatibleScreeningProfile)?
            .review_below();
        let review = match (suggestion.margin(), review_threshold) {
            (None, _) => ReviewRecommendation::Uncalibrated,
            (Some(0.0), _) => ReviewRecommendation::LowSeparation,
            (Some(_), None) => ReviewRecommendation::Uncalibrated,
            (Some(margin), Some(cutoff)) if margin < cutoff => ReviewRecommendation::LowSeparation,
            (Some(_), Some(_)) => ReviewRecommendation::RoutineSampling,
        };
        Ok(ScreeningResult {
            initial,
            refined,
            profile: Some(refinement.profile.revision()),
            review,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReviewRecommendation {
    Uncalibrated,
    LowSeparation,
    RoutineSampling,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ScreeningResult {
    initial: RankedPrediction,
    refined: Option<RankedPrediction>,
    profile: Option<ClassificationRevision>,
    review: ReviewRecommendation,
}

impl ScreeningResult {
    pub fn initial(&self) -> &RankedPrediction {
        &self.initial
    }

    pub fn refined(&self) -> Option<&RankedPrediction> {
        self.refined.as_ref()
    }

    pub fn suggestion(&self) -> &RankedPrediction {
        self.refined.as_ref().unwrap_or(&self.initial)
    }

    pub fn profile_revision(&self) -> Option<ClassificationRevision> {
        self.profile
    }

    pub fn review_recommendation(&self) -> ReviewRecommendation {
        self.review
    }
}

#[cfg(test)]
mod tests;
