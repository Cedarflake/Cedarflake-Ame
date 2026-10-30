mod features;
mod screening;
mod taxonomy;

pub use features::{Embedding, FeatureIdentity, ImageFeature, ModelCompatibility, TextFeature};
pub use screening::{
    CategoryScreeningThresholds, ScreeningIdentity, ScreeningProfile, ScreeningThresholds,
};
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
}

#[cfg(test)]
mod tests;
