#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AnalysisRevision([u8; 32]);

impl AnalysisRevision {
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub fn bytes(&self) -> &[u8; 32] {
        &self.0
    }

    pub(crate) fn derive<'a>(context: &str, fields: impl IntoIterator<Item = &'a [u8]>) -> Self {
        let mut hasher = blake3::Hasher::new_derive_key(context);
        for field in fields {
            hasher.update(&(field.len() as u64).to_le_bytes());
            hasher.update(field);
        }
        Self(*hasher.finalize().as_bytes())
    }

    pub fn description(text: &str) -> Self {
        Self::derive("ame.classification.description.v1", [text.as_bytes()])
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[error("Feature dimensions, values, or norm are invalid")]
pub struct InvalidEmbedding;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FeatureIdentity {
    pub embedding_space: AnalysisRevision,
    pub pipeline: AnalysisRevision,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ModelCompatibility {
    embedding_space: AnalysisRevision,
    image_pipeline: AnalysisRevision,
    text_pipeline: AnalysisRevision,
    dimensions: usize,
}

impl ModelCompatibility {
    pub fn new(
        embedding_space: AnalysisRevision,
        image_pipeline: AnalysisRevision,
        text_pipeline: AnalysisRevision,
        dimensions: usize,
    ) -> Result<Self, InvalidEmbedding> {
        if !(1..=4096).contains(&dimensions) {
            return Err(InvalidEmbedding);
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

pub(crate) fn image_feature_key(
    model: ModelCompatibility,
    scheme: &str,
    digest: &[u8; 32],
    byte_length: u64,
) -> AnalysisRevision {
    let image = model.image_identity();
    AnalysisRevision::derive(
        "ame.classification.image-cache.v1",
        [
            image.embedding_space.bytes().as_slice(),
            image.pipeline.bytes(),
            &(model.dimensions() as u64).to_le_bytes(),
            scheme.as_bytes(),
            digest,
            &byte_length.to_le_bytes(),
        ],
    )
}

#[derive(Clone, Debug, PartialEq)]
pub struct Embedding(Box<[f32]>);

impl Embedding {
    pub fn normalized(values: Vec<f32>) -> Result<Self, InvalidEmbedding> {
        if values.is_empty() || values.len() > 4096 || values.iter().any(|value| !value.is_finite())
        {
            return Err(InvalidEmbedding);
        }
        let norm = values
            .iter()
            .map(|value| f64::from(*value).powi(2))
            .sum::<f64>()
            .sqrt();
        if norm == 0.0 {
            return Err(InvalidEmbedding);
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

    pub(crate) fn from_normalized_values(values: Vec<f32>) -> Result<Self, InvalidEmbedding> {
        if values.is_empty() || values.len() > 4096 || values.iter().any(|value| !value.is_finite())
        {
            return Err(InvalidEmbedding);
        }
        let squared_norm = values
            .iter()
            .map(|value| f64::from(*value).powi(2))
            .sum::<f64>();
        if (squared_norm - 1.0).abs() > 0.00001 {
            return Err(InvalidEmbedding);
        }
        Ok(Self(values.into_boxed_slice()))
    }

    pub(crate) fn cosine(&self, other: &Self) -> Result<f32, InvalidEmbedding> {
        if self.0.len() != other.0.len() {
            return Err(InvalidEmbedding);
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
    pub description: AnalysisRevision,
    pub embedding: Embedding,
}
