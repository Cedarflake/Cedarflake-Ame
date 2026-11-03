use super::{
    CategoryId, ClassificationContent, ClassificationError, ClassificationProposal,
    ClassificationRevision, PredictionReference, ReviewRecommendation,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HumanCategoryChoice {
    Confirmed(PredictionReference),
    Corrected {
        category: CategoryId,
        rules: ClassificationRevision,
    },
}

impl HumanCategoryChoice {
    pub fn category(&self) -> &CategoryId {
        match self {
            Self::Confirmed(evidence) => &evidence.category,
            Self::Corrected { category, .. } => category,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HumanDecisionOrigin {
    Local,
    Restored,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum HumanClassification {
    #[default]
    Unreviewed,
    Chosen {
        category: CategoryId,
        origin: HumanDecisionOrigin,
    },
    Conflict,
}

#[derive(Default)]
enum ChoiceAgreement {
    #[default]
    Unreviewed,
    Chosen(CategoryId),
    Conflict,
}

impl ChoiceAgreement {
    fn include(&mut self, category: &CategoryId) {
        match self {
            Self::Unreviewed => *self = Self::Chosen(category.clone()),
            Self::Chosen(previous) if previous != category => *self = Self::Conflict,
            Self::Chosen(_) | Self::Conflict => {}
        }
    }

    fn resolve(self, origin: HumanDecisionOrigin) -> HumanClassification {
        match self {
            Self::Unreviewed => HumanClassification::Unreviewed,
            Self::Chosen(category) => HumanClassification::Chosen { category, origin },
            Self::Conflict => HumanClassification::Conflict,
        }
    }
}

#[derive(Default)]
pub struct HumanClassificationConsensus {
    local: ChoiceAgreement,
    restored: ChoiceAgreement,
}

impl HumanClassificationConsensus {
    pub fn include(&mut self, choice: &HumanCategoryChoice, origin: HumanDecisionOrigin) {
        match origin {
            HumanDecisionOrigin::Local => self.local.include(choice.category()),
            HumanDecisionOrigin::Restored => self.restored.include(choice.category()),
        }
    }

    pub fn resolve(self) -> HumanClassification {
        match self.local {
            ChoiceAgreement::Unreviewed => self.restored.resolve(HumanDecisionOrigin::Restored),
            local => local.resolve(HumanDecisionOrigin::Local),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProvisionalClassificationContent {
    asset_id: String,
    source_generation: u64,
}

impl ProvisionalClassificationContent {
    pub fn new(asset_id: String, source_generation: u64) -> Result<Self, ClassificationError> {
        if asset_id.is_empty()
            || asset_id.len() > 256
            || asset_id.chars().any(char::is_control)
            || source_generation == 0
            || source_generation > i64::MAX as u64
        {
            return Err(ClassificationError::InvalidClassificationTarget);
        }
        Ok(Self {
            asset_id,
            source_generation,
        })
    }

    pub fn asset_id(&self) -> &str {
        &self.asset_id
    }
    pub fn source_generation(&self) -> u64 {
        self.source_generation
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ClassificationSubject {
    Exact(ClassificationContent),
    Provisional(ProvisionalClassificationContent),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClassificationTarget {
    pub subject: ClassificationSubject,
    pub membership: ClassificationRevision,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClassificationFieldAuthority {
    pub category: ClassificationRevision,
    pub review: ClassificationRevision,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClassificationObservation {
    pub target: ClassificationTarget,
    pub authority: ClassificationFieldAuthority,
    pub rules: ClassificationRevision,
    pub configuration: Option<ClassificationRevision>,
    pub human: HumanClassification,
    pub deferred: bool,
    pub prediction: Option<ClassificationProposal>,
}

impl ClassificationObservation {
    pub fn compatible_prediction(&self) -> Option<&ClassificationProposal> {
        let prediction = self.prediction.as_ref()?;
        let evidence = &prediction.evidence;
        if self.configuration != Some(evidence.configuration)
            || self.rules != evidence.primary_rules
        {
            return None;
        }
        match &self.target.subject {
            ClassificationSubject::Exact(content) if content == &evidence.content => {
                Some(prediction)
            }
            ClassificationSubject::Exact(_) | ClassificationSubject::Provisional(_) => None,
        }
    }

    pub fn effective(&self) -> EffectiveClassification {
        match &self.human {
            HumanClassification::Chosen { category, .. } => {
                return EffectiveClassification {
                    category: Some(category.clone()),
                    review: ClassificationReview::Confirmed,
                };
            }
            HumanClassification::Conflict => {
                return EffectiveClassification {
                    category: None,
                    review: ClassificationReview::Conflict,
                };
            }
            HumanClassification::Unreviewed => {}
        }
        let prediction = self.compatible_prediction();
        let review = if self.deferred {
            ClassificationReview::Deferred
        } else {
            match prediction.map(|value| value.review) {
                None => ClassificationReview::Unclassified,
                Some(ReviewRecommendation::RoutineSampling) => ClassificationReview::Suggested,
                Some(ReviewRecommendation::Uncalibrated | ReviewRecommendation::LowSeparation) => {
                    ClassificationReview::NeedsReview
                }
            }
        };
        EffectiveClassification {
            category: prediction.map(|value| value.evidence.category.clone()),
            review,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClassificationReview {
    Unclassified,
    Suggested,
    NeedsReview,
    Deferred,
    Confirmed,
    Conflict,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EffectiveClassification {
    pub category: Option<CategoryId>,
    pub review: ClassificationReview,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ClassificationAction {
    Confirm,
    Correct(CategoryId),
    Defer,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ClassificationChange {
    Choose(HumanCategoryChoice),
    Defer,
    Unchanged,
}
