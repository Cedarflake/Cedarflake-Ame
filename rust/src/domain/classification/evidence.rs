use std::collections::BTreeSet;

use super::{
    CategoryId, ClassificationError, ClassificationRevision, ModelCompatibility, RankedPrediction,
};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ClassificationRecordId(String);

impl ClassificationRecordId {
    pub fn new(value: String) -> Result<Self, ClassificationError> {
        if value.is_empty()
            || value.len() > 128
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
        {
            return Err(ClassificationError::InvalidRecordId);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClassificationContent {
    pub scheme: ClassificationRecordId,
    pub digest: [u8; 32],
    pub byte_length: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClassificationRun {
    id: ClassificationRecordId,
    model: ModelCompatibility,
    primary_rules: ClassificationRevision,
    refinement: Option<RefinementIdentity>,
    configuration: ClassificationRevision,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RefinementIdentity {
    pub rules: ClassificationRevision,
    pub profile: ClassificationRevision,
}

impl ClassificationRun {
    pub fn new(
        id: ClassificationRecordId,
        model: ModelCompatibility,
        primary_rules: ClassificationRevision,
        refinement: Option<RefinementIdentity>,
    ) -> Result<Self, ClassificationError> {
        if refinement.is_some_and(|identity| identity.rules == primary_rules) {
            return Err(ClassificationError::IncompatibleRefinement);
        }
        let image = model.image_identity();
        let text = model.text_identity();
        let dimensions = (model.dimensions() as u64).to_le_bytes();
        let mut fields = vec![
            image.embedding_space.bytes().as_slice(),
            image.pipeline.bytes().as_slice(),
            text.pipeline.bytes().as_slice(),
            dimensions.as_slice(),
            primary_rules.bytes().as_slice(),
        ];
        if let Some(identity) = &refinement {
            fields.push(identity.rules.bytes());
            fields.push(identity.profile.bytes());
        }
        let configuration =
            ClassificationRevision::derive("ame.classification.configuration.v1", fields);
        Ok(Self {
            id,
            model,
            primary_rules,
            refinement,
            configuration,
        })
    }

    pub fn id(&self) -> &ClassificationRecordId {
        &self.id
    }

    pub fn model(&self) -> ModelCompatibility {
        self.model
    }

    pub fn primary_rules(&self) -> ClassificationRevision {
        self.primary_rules
    }

    pub fn refinement(&self) -> Option<RefinementIdentity> {
        self.refinement
    }

    pub fn configuration(&self) -> ClassificationRevision {
        self.configuration
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReviewRecommendation {
    Uncalibrated,
    LowSeparation,
    RoutineSampling,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PredictionEvidence {
    id: ClassificationRecordId,
    run: ClassificationRun,
    content: ClassificationContent,
    initial: RankedPrediction,
    refined: Option<RankedPrediction>,
    review: ReviewRecommendation,
}

impl PredictionEvidence {
    pub fn new(
        id: ClassificationRecordId,
        run: ClassificationRun,
        content: ClassificationContent,
        initial: RankedPrediction,
        refined: Option<RankedPrediction>,
        review: ReviewRecommendation,
    ) -> Result<Self, ClassificationError> {
        if initial.model() != run.model || initial.rules_revision() != run.primary_rules {
            return Err(ClassificationError::IncompatiblePrediction);
        }
        if run.refinement.is_none() && review != ReviewRecommendation::Uncalibrated {
            return Err(ClassificationError::IncompatiblePrediction);
        }
        if let Some(result) = &refined {
            let identity = run
                .refinement
                .ok_or(ClassificationError::IncompatiblePrediction)?;
            let original_ids: BTreeSet<_> = initial
                .categories()
                .iter()
                .map(|score| &score.category_id)
                .collect();
            let refined_ids: BTreeSet<_> = result
                .categories()
                .iter()
                .map(|score| &score.category_id)
                .collect();
            if result.model() != run.model
                || result.rules_revision() != identity.rules
                || original_ids != refined_ids
            {
                return Err(ClassificationError::IncompatiblePrediction);
            }
        }
        Ok(Self {
            id,
            run,
            content,
            initial,
            refined,
            review,
        })
    }

    pub fn id(&self) -> &ClassificationRecordId {
        &self.id
    }
    pub fn run(&self) -> &ClassificationRun {
        &self.run
    }
    pub fn content(&self) -> &ClassificationContent {
        &self.content
    }
    pub fn initial(&self) -> &RankedPrediction {
        &self.initial
    }
    pub fn refined(&self) -> Option<&RankedPrediction> {
        self.refined.as_ref()
    }
    pub fn suggestion(&self) -> &RankedPrediction {
        self.refined.as_ref().unwrap_or(&self.initial)
    }
    pub fn review_recommendation(&self) -> ReviewRecommendation {
        self.review
    }

    pub fn proposal(&self) -> ClassificationProposal {
        ClassificationProposal {
            evidence: PredictionReference {
                prediction_id: self.id.clone(),
                run_id: self.run.id.clone(),
                configuration: self.run.configuration,
                primary_rules: self.run.primary_rules,
                content: self.content.clone(),
                category: self.suggestion().leading().category_id.clone(),
            },
            review: self.review,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PredictionReference {
    pub prediction_id: ClassificationRecordId,
    pub run_id: ClassificationRecordId,
    pub configuration: ClassificationRevision,
    pub primary_rules: ClassificationRevision,
    pub content: ClassificationContent,
    pub category: CategoryId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClassificationProposal {
    pub evidence: PredictionReference,
    pub review: ReviewRecommendation,
}

#[cfg(test)]
mod tests;
