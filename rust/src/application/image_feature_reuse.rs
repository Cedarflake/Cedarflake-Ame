use crate::domain::ScanError;
use crate::domain::classification::{ClassificationContent, ClassificationRevision};
use crate::domain::image_features::{ImageFeature, ModelCompatibility, TextFeature};
use crate::domain::semantic_discovery::ImageContentIdentity;
use crate::ports::classification_features::{
    ClassificationFeatureCacheStats, ClassificationFeatureRepository,
};
use crate::ports::semantic_features::SemanticFeatureRepository;

pub(crate) struct SharedImageFeatures<C, S> {
    cache: C,
    semantic: S,
}

impl<C, S> SharedImageFeatures<C, S> {
    pub(crate) fn new(cache: C, semantic: S) -> Self {
        Self { cache, semantic }
    }

    pub(crate) fn semantic(&self) -> &S {
        &self.semantic
    }
}

impl<C: ClassificationFeatureRepository, S: SemanticFeatureRepository>
    ClassificationFeatureRepository for SharedImageFeatures<C, S>
{
    fn load_image_feature(
        &mut self,
        model: ModelCompatibility,
        content: &ClassificationContent,
    ) -> Result<Option<ImageFeature>, ScanError> {
        let identity = content_identity(content)?;
        if let Some(feature) = self.semantic.load_image(model, &identity)? {
            return Ok(Some(feature));
        }
        let feature = self.cache.load_image_feature(model, content)?;
        if let Some(feature) = &feature {
            self.semantic.publish_image(model, &identity, feature)?;
        }
        Ok(feature)
    }

    fn store_image_feature(
        &mut self,
        model: ModelCompatibility,
        content: &ClassificationContent,
        feature: &ImageFeature,
    ) -> Result<(), ScanError> {
        self.cache.store_image_feature(model, content, feature)?;
        self.semantic
            .publish_image(model, &content_identity(content)?, feature)?;
        Ok(())
    }

    fn load_text_feature(
        &mut self,
        model: ModelCompatibility,
        description: ClassificationRevision,
    ) -> Result<Option<TextFeature>, ScanError> {
        self.cache.load_text_feature(model, description)
    }

    fn load_text_features(
        &mut self,
        model: ModelCompatibility,
        descriptions: &[ClassificationRevision],
    ) -> Result<Vec<Option<TextFeature>>, ScanError> {
        self.cache.load_text_features(model, descriptions)
    }

    fn store_text_feature(
        &mut self,
        model: ModelCompatibility,
        feature: &TextFeature,
    ) -> Result<(), ScanError> {
        self.cache.store_text_feature(model, feature)
    }

    fn feature_cache_stats(&self) -> Result<ClassificationFeatureCacheStats, ScanError> {
        self.cache.feature_cache_stats()
    }

    fn clear_feature_cache_batch(&mut self) -> Result<ClassificationFeatureCacheStats, ScanError> {
        self.cache.clear_feature_cache_batch()
    }
}

fn content_identity(content: &ClassificationContent) -> Result<ImageContentIdentity, ScanError> {
    ImageContentIdentity::new(
        content.scheme.as_str().to_owned(),
        content.digest,
        content.byte_length,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::{SqliteClassificationFeatureCache, SqliteSemanticFeatureStore};
    use crate::domain::classification::{ClassificationRecordId, Embedding};

    #[test]
    fn promotes_cached_images_and_preserves_them_when_the_lru_is_cleared() {
        let root = tempfile::tempdir().unwrap();
        let model = ModelCompatibility::new(
            ClassificationRevision::from_bytes([1; 32]),
            ClassificationRevision::from_bytes([2; 32]),
            ClassificationRevision::from_bytes([3; 32]),
            2,
        )
        .unwrap();
        let content = ClassificationContent {
            scheme: ClassificationRecordId::new("blake3-256-full-v1".to_owned()).unwrap(),
            digest: [7; 32],
            byte_length: 100,
        };
        let image = ImageFeature {
            identity: model.image_identity(),
            embedding: Embedding::normalized(vec![1.0, 2.0]).unwrap(),
        };
        let mut cache = SqliteClassificationFeatureCache::open(&root.path().join("lru")).unwrap();
        cache.store_image_feature(model, &content, &image).unwrap();
        let semantic = SqliteSemanticFeatureStore::open(&root.path().join("semantic")).unwrap();
        let mut features = SharedImageFeatures::new(cache, semantic);
        assert_eq!(
            features.load_image_feature(model, &content).unwrap(),
            Some(image.clone())
        );
        assert_eq!(features.semantic.storage().unwrap().features, 1);
        assert_eq!(features.clear_feature_cache_batch().unwrap().entries, 0);
        assert_eq!(
            features.load_image_feature(model, &content).unwrap(),
            Some(image)
        );
        assert_eq!(features.semantic.storage().unwrap().generation, 1);
    }
}
