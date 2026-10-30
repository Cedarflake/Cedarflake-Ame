use super::{ClassificationError, ClassificationRevision, ModelCompatibility};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScreeningIdentity {
    pub model: ModelCompatibility,
    pub primary_rules: ClassificationRevision,
    pub refinement_rules: ClassificationRevision,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScreeningThresholds {
    rescreen_below: f32,
    review_below: f32,
}

impl ScreeningThresholds {
    pub fn new(rescreen_below: f32, review_below: f32) -> Result<Self, ClassificationError> {
        if ![rescreen_below, review_below]
            .iter()
            .all(|value| value.is_finite() && (0.0..=2.0).contains(value))
        {
            return Err(ClassificationError::InvalidScreeningThresholds);
        }
        Ok(Self {
            rescreen_below: rescreen_below.max(0.0),
            review_below: review_below.max(0.0),
        })
    }

    pub fn rescreen_below(self) -> f32 {
        self.rescreen_below
    }

    pub fn review_below(self) -> f32 {
        self.review_below
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScreeningProfile {
    identity: ScreeningIdentity,
    calibration: ClassificationRevision,
    thresholds: ScreeningThresholds,
    revision: ClassificationRevision,
}

impl ScreeningProfile {
    pub fn new(
        identity: ScreeningIdentity,
        calibration: ClassificationRevision,
        thresholds: ScreeningThresholds,
    ) -> Self {
        let model = identity.model;
        let revision = ClassificationRevision::derive(
            "ame.classification.single-rescreen.margin-policy.v1",
            [
                model.image_identity().embedding_space.bytes().as_slice(),
                model.image_identity().pipeline.bytes().as_slice(),
                model.text_identity().pipeline.bytes().as_slice(),
                &(model.dimensions() as u32).to_le_bytes(),
                identity.primary_rules.bytes().as_slice(),
                identity.refinement_rules.bytes().as_slice(),
                calibration.bytes().as_slice(),
                &thresholds.rescreen_below.to_bits().to_le_bytes(),
                &thresholds.review_below.to_bits().to_le_bytes(),
            ],
        );
        Self {
            identity,
            calibration,
            thresholds,
            revision,
        }
    }

    pub fn identity(self) -> ScreeningIdentity {
        self.identity
    }

    pub fn calibration(self) -> ClassificationRevision {
        self.calibration
    }

    pub fn thresholds(self) -> ScreeningThresholds {
        self.thresholds
    }

    pub fn revision(self) -> ClassificationRevision {
        self.revision
    }
}
