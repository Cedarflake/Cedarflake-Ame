use super::{ClassificationError, ClassificationRevision};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FeatureIdentity {
    pub embedding_space: ClassificationRevision,
    pub pipeline: ClassificationRevision,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ModelCompatibility {
    embedding_space: ClassificationRevision,
    image_pipeline: ClassificationRevision,
    text_pipeline: ClassificationRevision,
    dimensions: usize,
}

impl ModelCompatibility {
    pub fn new(
        embedding_space: ClassificationRevision,
        image_pipeline: ClassificationRevision,
        text_pipeline: ClassificationRevision,
        dimensions: usize,
    ) -> Result<Self, ClassificationError> {
        if !(1..=4096).contains(&dimensions) {
            return Err(ClassificationError::InvalidEmbedding);
        }
        Ok(Self {
            embedding_space,
            image_pipeline,
            text_pipeline,
            dimensions,
        })
    }

    pub fn image_identity(&self) -> FeatureIdentity {
        FeatureIdentity {
            embedding_space: self.embedding_space,
            pipeline: self.image_pipeline,
        }
    }

    pub fn text_identity(&self) -> FeatureIdentity {
        FeatureIdentity {
            embedding_space: self.embedding_space,
            pipeline: self.text_pipeline,
        }
    }

    pub fn dimensions(&self) -> usize {
        self.dimensions
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Embedding(Box<[f32]>);

impl Embedding {
    pub fn normalized(values: Vec<f32>) -> Result<Self, ClassificationError> {
        if values.is_empty() || values.len() > 4096 || values.iter().any(|value| !value.is_finite())
        {
            return Err(ClassificationError::InvalidEmbedding);
        }
        let norm = values
            .iter()
            .map(|value| f64::from(*value).powi(2))
            .sum::<f64>()
            .sqrt();
        if norm == 0.0 {
            return Err(ClassificationError::InvalidEmbedding);
        }
        Ok(Self(
            values
                .into_iter()
                .map(|value| (f64::from(value) / norm) as f32)
                .collect(),
        ))
    }

    pub fn values(&self) -> &[f32] {
        &self.0
    }

    pub(crate) fn cosine(&self, other: &Self) -> Result<f32, ClassificationError> {
        if self.0.len() != other.0.len() {
            return Err(ClassificationError::InvalidEmbedding);
        }
        let similarity = self
            .0
            .iter()
            .zip(other.0.iter())
            .map(|(left, right)| f64::from(*left) * f64::from(*right))
            .sum::<f64>();
        Ok(similarity.clamp(-1.0, 1.0) as f32)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ImageFeature {
    pub identity: FeatureIdentity,
    pub embedding: Embedding,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TextFeature {
    pub identity: FeatureIdentity,
    pub description: ClassificationRevision,
    pub embedding: Embedding,
}
