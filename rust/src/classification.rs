pub use crate::application::classification_matching::{
    CategoryMatcher, CategoryScore, RankedPrediction,
};
pub use crate::application::classification_screening::{
    ReviewRecommendation, ScreeningClassifier, ScreeningResult,
};
pub use crate::domain::classification::{
    CategoryDefinition, CategoryId, CategoryScreeningThresholds, ClassificationError,
    ClassificationRevision, Embedding, FeatureIdentity, ImageFeature, ModelCompatibility,
    ScreeningIdentity, ScreeningProfile, ScreeningThresholds, Taxonomy, TextFeature,
};
