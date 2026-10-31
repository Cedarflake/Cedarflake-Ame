use crate::domain::ScanError;
use crate::domain::classification::{
    ClassificationContent, ClassificationRevision, ImageFeature, ModelCompatibility, TextFeature,
};

pub(crate) const TEXT_FEATURE_WINDOW: usize = 64;

pub(crate) fn validate_text_feature_window(length: usize) -> Result<(), ScanError> {
    if length > TEXT_FEATURE_WINDOW {
        return Err(ScanError::new(
            "classification_feature_window_exceeded",
            "The text-feature window exceeds its resource bound",
        ));
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClassificationFeatureCacheStats {
    pub entries: u64,
    pub accounted_bytes: u64,
}

pub trait ClassificationFeatureRepository {
    fn load_image_feature(
        &mut self,
        model: ModelCompatibility,
        content: &ClassificationContent,
    ) -> Result<Option<ImageFeature>, ScanError>;

    fn store_image_feature(
        &mut self,
        model: ModelCompatibility,
        content: &ClassificationContent,
        feature: &ImageFeature,
    ) -> Result<(), ScanError>;

    fn load_text_feature(
        &mut self,
        model: ModelCompatibility,
        description: ClassificationRevision,
    ) -> Result<Option<TextFeature>, ScanError>;

    fn load_text_features(
        &mut self,
        model: ModelCompatibility,
        descriptions: &[ClassificationRevision],
    ) -> Result<Vec<Option<TextFeature>>, ScanError> {
        validate_text_feature_window(descriptions.len())?;
        descriptions
            .iter()
            .map(|description| self.load_text_feature(model, *description))
            .collect()
    }

    fn store_text_feature(
        &mut self,
        model: ModelCompatibility,
        feature: &TextFeature,
    ) -> Result<(), ScanError>;

    fn feature_cache_stats(&self) -> Result<ClassificationFeatureCacheStats, ScanError>;

    fn clear_feature_cache_batch(&mut self) -> Result<ClassificationFeatureCacheStats, ScanError>;
}
