use std::collections::BTreeSet;

use super::{CategoryId, ClassificationError, ClassificationRevision, ModelCompatibility};

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
    pub fn new(
        rules_revision: ClassificationRevision,
        model: ModelCompatibility,
        mut categories: Vec<CategoryScore>,
    ) -> Result<Self, ClassificationError> {
        if categories.is_empty() || categories.len() > super::taxonomy::MAX_CATEGORIES {
            return Err(ClassificationError::InvalidPrediction);
        }
        let mut identities = BTreeSet::new();
        for score in &mut categories {
            if !score.similarity.is_finite()
                || !(-1.0..=1.0).contains(&score.similarity)
                || !identities.insert(&score.category_id)
            {
                return Err(ClassificationError::InvalidPrediction);
            }
            if score.similarity == 0.0 {
                score.similarity = 0.0;
            }
        }
        categories.sort_by(|left, right| {
            right
                .similarity
                .total_cmp(&left.similarity)
                .then_with(|| left.category_id.cmp(&right.category_id))
        });
        Ok(Self {
            rules_revision,
            model,
            categories,
        })
    }

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
