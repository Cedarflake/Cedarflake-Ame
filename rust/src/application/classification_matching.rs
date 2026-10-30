use std::collections::BTreeMap;

use crate::domain::classification::{
    CategoryId, ClassificationError, ClassificationRevision, Embedding, ImageFeature,
    ModelCompatibility, Taxonomy, TextFeature,
};

pub struct CategoryMatcher {
    taxonomy: Taxonomy,
    model: ModelCompatibility,
    descriptions: BTreeMap<ClassificationRevision, Embedding>,
}

impl CategoryMatcher {
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
        categories.sort_by(|left, right| {
            right
                .similarity
                .total_cmp(&left.similarity)
                .then_with(|| left.category_id.cmp(&right.category_id))
        });
        Ok(RankedPrediction {
            rules_revision: self.taxonomy.revision(),
            model: self.model,
            categories,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CategoryScore {
    pub category_id: CategoryId,
    pub similarity: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RankedPrediction {
    rules_revision: ClassificationRevision,
    model: ModelCompatibility,
    categories: Vec<CategoryScore>,
}

impl RankedPrediction {
    pub fn rules_revision(&self) -> ClassificationRevision {
        self.rules_revision
    }

    pub fn model(&self) -> ModelCompatibility {
        self.model
    }

    pub fn categories(&self) -> &[CategoryScore] {
        &self.categories
    }

    pub fn leading(&self) -> &CategoryScore {
        &self.categories[0]
    }

    pub fn margin(&self) -> Option<f32> {
        self.categories
            .get(1)
            .map(|runner_up| self.leading().similarity - runner_up.similarity)
    }
}

#[cfg(test)]
mod tests;
