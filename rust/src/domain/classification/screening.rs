use super::taxonomy::MAX_CATEGORIES;
use super::{
    CategoryDefinition, CategoryId, ClassificationError, ClassificationRevision, ModelCompatibility,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScreeningIdentity {
    pub model: ModelCompatibility,
    pub primary_rules: ClassificationRevision,
    pub refinement_rules: ClassificationRevision,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScreeningThresholds {
    rescreen_below: f32,
    review_below: Option<f32>,
}

impl ScreeningThresholds {
    pub fn new(rescreen_below: f32, review_below: f32) -> Result<Self, ClassificationError> {
        Ok(Self {
            rescreen_below: validated_separation(rescreen_below)?,
            review_below: Some(validated_separation(review_below)?),
        })
    }

    pub fn without_review_calibration(rescreen_below: f32) -> Result<Self, ClassificationError> {
        Ok(Self {
            rescreen_below: validated_separation(rescreen_below)?,
            review_below: None,
        })
    }

    pub fn rescreen_below(self) -> f32 {
        self.rescreen_below
    }

    pub fn review_below(self) -> Option<f32> {
        self.review_below
    }

    fn canonical_bytes(self) -> Vec<u8> {
        let mut bytes = self.rescreen_below.to_bits().to_le_bytes().to_vec();
        match self.review_below {
            Some(value) => {
                bytes.push(1);
                bytes.extend(value.to_bits().to_le_bytes());
            }
            None => bytes.push(0),
        }
        bytes
    }
}

fn validated_separation(value: f32) -> Result<f32, ClassificationError> {
    if !value.is_finite() || !(0.0..=2.0).contains(&value) {
        return Err(ClassificationError::InvalidScreeningThresholds);
    }
    Ok(value.max(0.0))
}

#[derive(Clone, Debug, PartialEq)]
pub struct CategoryScreeningThresholds {
    pub category_id: CategoryId,
    pub thresholds: ScreeningThresholds,
}

#[derive(Clone, Debug, PartialEq)]
enum ScreeningPolicy {
    Uniform(ScreeningThresholds),
    Categories(Vec<CategoryScreeningThresholds>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct ScreeningProfile {
    identity: ScreeningIdentity,
    calibration: ClassificationRevision,
    policy: ScreeningPolicy,
    revision: ClassificationRevision,
}

impl ScreeningProfile {
    pub fn new(
        identity: ScreeningIdentity,
        calibration: ClassificationRevision,
        thresholds: ScreeningThresholds,
    ) -> Self {
        Self::build(identity, calibration, ScreeningPolicy::Uniform(thresholds))
    }

    pub fn for_categories(
        identity: ScreeningIdentity,
        calibration: ClassificationRevision,
        mut categories: Vec<CategoryScreeningThresholds>,
    ) -> Result<Self, ClassificationError> {
        if categories.is_empty() || categories.len() > MAX_CATEGORIES {
            return Err(ClassificationError::InvalidScreeningCategories);
        }
        categories.sort_by(|left, right| left.category_id.cmp(&right.category_id));
        if categories
            .windows(2)
            .any(|pair| pair[0].category_id == pair[1].category_id)
        {
            return Err(ClassificationError::InvalidScreeningCategories);
        }
        Ok(Self::build(
            identity,
            calibration,
            ScreeningPolicy::Categories(categories),
        ))
    }

    fn build(
        identity: ScreeningIdentity,
        calibration: ClassificationRevision,
        policy: ScreeningPolicy,
    ) -> Self {
        let model = identity.model;
        let mut fields = vec![
            model.image_identity().embedding_space.bytes().to_vec(),
            model.image_identity().pipeline.bytes().to_vec(),
            model.text_identity().pipeline.bytes().to_vec(),
            (model.dimensions() as u32).to_le_bytes().to_vec(),
            identity.primary_rules.bytes().to_vec(),
            identity.refinement_rules.bytes().to_vec(),
            calibration.bytes().to_vec(),
        ];
        match &policy {
            ScreeningPolicy::Uniform(thresholds) => {
                fields.push(b"uniform".to_vec());
                fields.push(thresholds.canonical_bytes());
            }
            ScreeningPolicy::Categories(categories) => {
                fields.push(b"categories".to_vec());
                for category in categories {
                    fields.push(category.category_id.as_str().as_bytes().to_vec());
                    fields.push(category.thresholds.canonical_bytes());
                }
            }
        }
        let revision = ClassificationRevision::derive(
            "ame.classification.single-rescreen.margin-policy.v2",
            fields.iter().map(Vec::as_slice),
        );
        Self {
            identity,
            calibration,
            policy,
            revision,
        }
    }

    pub fn identity(&self) -> ScreeningIdentity {
        self.identity
    }

    pub fn calibration(&self) -> ClassificationRevision {
        self.calibration
    }

    pub fn thresholds_for(&self, category: &CategoryId) -> Option<ScreeningThresholds> {
        match &self.policy {
            ScreeningPolicy::Uniform(thresholds) => Some(*thresholds),
            ScreeningPolicy::Categories(categories) => categories
                .binary_search_by(|entry| entry.category_id.cmp(category))
                .ok()
                .map(|index| categories[index].thresholds),
        }
    }

    pub(crate) fn matches_categories(&self, active: &[CategoryDefinition]) -> bool {
        match &self.policy {
            ScreeningPolicy::Uniform(_) => true,
            ScreeningPolicy::Categories(categories) => categories
                .iter()
                .map(|entry| &entry.category_id)
                .eq(active.iter().map(CategoryDefinition::id)),
        }
    }

    pub fn revision(&self) -> ClassificationRevision {
        self.revision
    }
}
