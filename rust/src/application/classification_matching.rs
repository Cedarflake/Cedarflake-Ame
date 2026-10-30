use std::collections::BTreeMap;

use crate::domain::classification::{
    ClassificationError, ClassificationRevision, Embedding, ImageFeature, ModelCompatibility,
    Taxonomy, TextFeature,
};

pub use crate::domain::classification::{CategoryScore, RankedPrediction};

pub struct CategoryMatcher {
    taxonomy: Taxonomy,
    model: ModelCompatibility,
    descriptions: BTreeMap<ClassificationRevision, Embedding>,
}

impl CategoryMatcher {
    pub(crate) fn taxonomy(&self) -> &Taxonomy {
        &self.taxonomy
    }

    pub(crate) fn model(&self) -> ModelCompatibility {
        self.model
    }

    pub fn new(
        taxonomy: Taxonomy,
        model: ModelCompatibility,
        features: Vec<TextFeature>,
    ) -> Result<Self, ClassificationError> {
        let required = taxonomy.description_revisions();
        if features.len() > required.len() {
            return Err(ClassificationError::UnexpectedTextFeature);
        }
        let mut descriptions = BTreeMap::new();
        for feature in features {
            if feature.identity != model.text_identity() {
                return Err(ClassificationError::IncompatibleFeature);
            }
            if feature.embedding.values().len() != model.dimensions() {
                return Err(ClassificationError::InvalidEmbedding);
            }
            if !required.contains(&feature.description) {
                return Err(ClassificationError::UnexpectedTextFeature);
            }
            if descriptions
                .insert(feature.description, feature.embedding)
                .is_some()
            {
                return Err(ClassificationError::DuplicateTextFeature);
            }
        }
        if descriptions.len() != required.len() {
            return Err(ClassificationError::MissingTextFeature);
        }
        Ok(Self {
            taxonomy,
            model,
            descriptions,
        })
    }

    pub fn rank(&self, image: &ImageFeature) -> Result<RankedPrediction, ClassificationError> {
        if image.identity != self.model.image_identity() {
            return Err(ClassificationError::IncompatibleFeature);
        }
        if image.embedding.values().len() != self.model.dimensions() {
            return Err(ClassificationError::InvalidEmbedding);
        }
        let mut categories = Vec::with_capacity(self.taxonomy.categories().len());
        for category in self.taxonomy.categories() {
            let mut similarity = -1.0_f32;
            for description in category.descriptions() {
                let revision = ClassificationRevision::description(description);
                let feature = self
                    .descriptions
                    .get(&revision)
                    .ok_or(ClassificationError::MissingTextFeature)?;
                similarity = similarity.max(image.embedding.cosine(feature)?);
            }
            categories.push(CategoryScore {
                category_id: category.id().clone(),
                similarity,
            });
        }
        RankedPrediction::new(self.taxonomy.revision(), self.model, categories)
    }
}

#[cfg(test)]
mod tests;
