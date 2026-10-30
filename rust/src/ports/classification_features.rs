use crate::domain::ScanError;
use crate::domain::classification::{
    ClassificationContent, ClassificationRevision, ImageFeature, ModelCompatibility, TextFeature,
};

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

    fn store_text_feature(
        &mut self,
        model: ModelCompatibility,
        feature: &TextFeature,
    ) -> Result<(), ScanError>;

    fn feature_cache_stats(&self) -> Result<ClassificationFeatureCacheStats, ScanError>;

    fn clear_feature_cache_batch(&mut self) -> Result<ClassificationFeatureCacheStats, ScanError>;
}
