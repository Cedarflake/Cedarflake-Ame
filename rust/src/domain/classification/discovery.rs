use std::collections::BTreeSet;

use super::{
    ClassificationContent, ClassificationRevision, ImageFeature, ModelCompatibility, TextFeature,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ConceptId([u8; 32]);

impl ConceptId {
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub fn bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VocabularyConcept {
    id: ConceptId,
    label: String,
    description: String,
}

impl VocabularyConcept {
    pub fn new(id: ConceptId, label: String, description: String) -> Result<Self, DiscoveryError> {
        let label = label.trim();
        let description = description.trim();
        if label.is_empty()
            || label.chars().count() > 80
            || label.chars().any(char::is_control)
            || description.is_empty()
            || description.chars().count() > 1024
            || description.chars().any(char::is_control)
        {
            return Err(DiscoveryError::InvalidConcept);
        }
        Ok(Self {
            id,
            label: label.to_owned(),
            description: description.to_owned(),
        })
    }

    pub fn id(&self) -> ConceptId {
        self.id
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn revision(&self) -> ClassificationRevision {
        ClassificationRevision::derive(
            "ame.classification.discovery-concept.v1",
            [
                self.id.bytes().as_slice(),
                self.label.as_bytes(),
                self.description.as_bytes(),
            ],
        )
    }
}

pub struct DiscoverySample {
    pub content: ClassificationContent,
    pub feature: ImageFeature,
}

pub struct DiscoveryCohort {
    model: ModelCompatibility,
    revision: ClassificationRevision,
    members: Vec<f64>,
    comparison: Vec<f64>,
}

impl DiscoveryCohort {
    pub fn new(
        model: ModelCompatibility,
        members: &[DiscoverySample],
        comparison: &[DiscoverySample],
    ) -> Result<Self, DiscoveryError> {
        if members.is_empty() || comparison.is_empty() || members.len() + comparison.len() > 512 {
            return Err(DiscoveryError::InvalidCohort);
        }
        let mut seen = BTreeSet::new();
        let mut identities = BTreeSet::new();
        for (role, samples) in [(0_u8, members), (1, comparison)] {
            for sample in samples {
                let content = &sample.content;
                let identity = (&content.scheme, content.digest, content.byte_length);
                if !seen.insert(identity) {
                    return Err(DiscoveryError::DuplicateContent);
                }
                if sample.feature.identity != model.image_identity()
                    || sample.feature.embedding.values().len() != model.dimensions()
                {
                    return Err(DiscoveryError::IncompatibleFeature);
                }
                identities.insert(ClassificationRevision::derive(
                    "ame.classification.discovery-sample.v1",
                    [
                        &[role],
                        content.scheme.as_str().as_bytes(),
                        &content.digest,
                        &content.byte_length.to_le_bytes(),
                    ],
                ));
            }
        }
        let revision = ClassificationRevision::derive(
            "ame.classification.discovery-cohort.v1",
            identities
                .iter()
                .map(|identity| identity.bytes().as_slice()),
        );
        Ok(Self {
            model,
            revision,
            members: mean_feature(model.dimensions(), members),
            comparison: mean_feature(model.dimensions(), comparison),
        })
    }

    pub fn model(&self) -> ModelCompatibility {
        self.model
    }

    pub fn revision(&self) -> ClassificationRevision {
        self.revision
    }

    pub fn score(&self, feature: &TextFeature) -> Result<ConceptSimilarity, DiscoveryError> {
        if feature.identity != self.model.text_identity()
            || feature.embedding.values().len() != self.model.dimensions()
        {
            return Err(DiscoveryError::IncompatibleFeature);
        }
        let similarity = mean_cosine(&self.members, feature);
        let context_similarity = mean_cosine(&self.comparison, feature);
        Ok(ConceptSimilarity {
            similarity,
            context_similarity,
            contrast: similarity - context_similarity,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ConceptSimilarity {
    pub similarity: f32,
    pub context_similarity: f32,
    pub contrast: f32,
}

fn mean_feature(dimensions: usize, samples: &[DiscoverySample]) -> Vec<f64> {
    let mut mean = vec![0.0; dimensions];
    for sample in samples {
        for (total, value) in mean.iter_mut().zip(sample.feature.embedding.values()) {
            *total += f64::from(*value) / samples.len() as f64;
        }
    }
    mean
}

fn mean_cosine(mean: &[f64], feature: &TextFeature) -> f32 {
    mean.iter()
        .zip(feature.embedding.values())
        .map(|(left, right)| left * f64::from(*right))
        .sum::<f64>()
        .clamp(-1.0, 1.0) as f32
}

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum DiscoveryError {
    #[error("The discovery concept has an invalid label or description")]
    InvalidConcept,
    #[error("Discovery needs a nonempty cohort and comparison within the sample limit")]
    InvalidCohort,
    #[error("A content version occurs more than once in the discovery samples")]
    DuplicateContent,
    #[error("Discovery features do not match the admitted encoder contract")]
    IncompatibleFeature,
}

#[cfg(test)]
mod tests;
