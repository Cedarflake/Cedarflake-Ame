use crate::domain::ScanError;
use crate::domain::image_features::{ImageFeature, ModelCompatibility};
use crate::domain::semantic_discovery::{
    FeaturePublication, ImageContentIdentity, SemanticFeatureStorage,
};

pub trait SemanticFeatureRepository {
    fn load_images(
        &self,
        model: ModelCompatibility,
        content: &[ImageContentIdentity],
        generation: u64,
    ) -> Result<Vec<Option<ImageFeature>>, ScanError>;

    fn load_image(
        &self,
        model: ModelCompatibility,
        content: &ImageContentIdentity,
    ) -> Result<Option<ImageFeature>, ScanError>;

    fn publish_image(
        &mut self,
        model: ModelCompatibility,
        content: &ImageContentIdentity,
        feature: &ImageFeature,
    ) -> Result<FeaturePublication, ScanError>;

    fn storage(&self) -> Result<SemanticFeatureStorage, ScanError>;
}
