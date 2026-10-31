mod decisions;
pub mod discovery;
mod evidence;
mod features;
mod ranking;
mod screening;
mod setup;
mod taxonomy;
pub mod work;

pub use decisions::{
    ClassificationAction, ClassificationChange, ClassificationFieldAuthority,
    ClassificationObservation, ClassificationReview, ClassificationSubject, ClassificationTarget,
    EffectiveClassification, HumanCategoryChoice, HumanClassification,
    HumanClassificationConsensus, HumanDecisionOrigin, ProvisionalClassificationContent,
};
pub use evidence::{
    ClassificationContent, ClassificationProposal, ClassificationRecordId, ClassificationRun,
    PredictionEvidence, PredictionReference, RefinementIdentity, ReviewRecommendation,
};
pub use features::{Embedding, FeatureIdentity, ImageFeature, ModelCompatibility, TextFeature};
pub use ranking::{CategoryScore, RankedPrediction};
pub use screening::{
    CategoryScreeningThresholds, ScreeningIdentity, ScreeningPolicy, ScreeningProfile,
    ScreeningThresholds,
};
pub use setup::{ClassificationAnalysisSetup, ClassificationSetupActivation};
pub use taxonomy::{CategoryDefinition, CategoryId, ClassificationRevision, Taxonomy};

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum ClassificationError {
    #[error("A classification scheme needs at least one active category")]
    NoCategories,
    #[error("The classification scheme exceeds the active category limit")]
    TooManyCategories,
    #[error("Category identity is invalid")]
    InvalidCategoryId,
    #[error("Category label is empty or too long")]
    InvalidLabel,
    #[error("Category descriptions are empty, repeated, or exceed their limits")]
    InvalidDescriptions,
    #[error("Category identity occurs more than once")]
    DuplicateCategory,
    #[error("Feature dimensions, values, or norm are invalid")]
    InvalidEmbedding,
    #[error("Feature identity does not match the admitted model contract")]
    IncompatibleFeature,
    #[error("A required description feature is missing")]
    MissingTextFeature,
    #[error("A description feature occurs more than once")]
    DuplicateTextFeature,
    #[error("A description feature is outside the active classification scheme")]
    UnexpectedTextFeature,
    #[error("Screening thresholds must be finite cosine separations between zero and two")]
    InvalidScreeningThresholds,
    #[error("Screening categories must have one to 256 unique identities")]
    InvalidScreeningCategories,
    #[error("Screening profile does not match the active model and category rules")]
    IncompatibleScreeningProfile,
    #[error("Refinement must use the same model and active category identities")]
    IncompatibleRefinement,
    #[error("The analysis setup does not match its immutable run")]
    IncompatibleAnalysisSetup,
    #[error("Prediction rankings require unique categories and finite cosine scores")]
    InvalidPrediction,
    #[error("Classification record identity is invalid")]
    InvalidRecordId,
    #[error("Prediction evidence does not match its analysis run")]
    IncompatiblePrediction,
    #[error("The classification target has no valid content version")]
    InvalidClassificationTarget,
    #[error("The image group or its classification decision changed; refresh before saving")]
    ClassificationTargetChanged,
    #[error("Category definitions changed; refresh before saving")]
    ClassificationRulesChanged,
    #[error("The displayed prediction changed; refresh before confirming")]
    ClassificationPredictionChanged,
    #[error("The selected group has no current compatible prediction to confirm")]
    NoCompatiblePrediction,
    #[error("Choose a category to resolve the group's conflicting human decisions")]
    ClassificationConflictUnresolved,
    #[error("Use an explicit category correction to replace the saved human choice")]
    HumanChoiceRequiresCorrection,
    #[error("The selected category is no longer active")]
    CategoryRetired,
}

#[cfg(test)]
mod tests;
