use std::collections::BTreeMap;

use super::DiscoveryError;
use crate::domain::classification::{CategoryId, ClassificationRevision};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConceptPurposeLabels {
    labels: BTreeMap<CategoryId, String>,
}

impl ConceptPurposeLabels {
    pub fn new(labels: Vec<(CategoryId, String)>) -> Result<Self, DiscoveryError> {
        if !(1..=64).contains(&labels.len()) {
            return Err(DiscoveryError::InvalidConcept);
        }
        let mut admitted = BTreeMap::new();
        for (purpose, label) in labels {
            let label = label.trim();
            if label.is_empty()
                || label.chars().count() > 80
                || label.chars().any(char::is_control)
                || admitted.insert(purpose, label.to_owned()).is_some()
            {
                return Err(DiscoveryError::InvalidConcept);
            }
        }
        Ok(Self { labels: admitted })
    }

    pub fn label(&self, purpose: &CategoryId) -> Option<&str> {
        self.labels.get(purpose).map(String::as_str)
    }

    pub fn revision(&self) -> ClassificationRevision {
        ClassificationRevision::derive(
            "ame.classification.concept-purpose-labels.v1",
            self.labels
                .iter()
                .flat_map(|(purpose, label)| [purpose.as_str().as_bytes(), label.as_bytes()]),
        )
    }
}
