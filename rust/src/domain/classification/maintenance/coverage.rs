use std::collections::BTreeMap;

use crate::domain::classification::{
    CategoryId, ClassificationContent, ClassificationRevision, ModelCompatibility,
    PredictionEvidence,
};
use crate::domain::classification_configuration::ClassificationConfigurationSnapshot;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CategoryCoverageSnapshot {
    pub catalog_revision: u64,
    pub configuration: ClassificationConfigurationSnapshot,
    pub model: ModelCompatibility,
    pub unverified_sources: bool,
}

#[derive(Clone, Debug)]
pub struct CategoryCoverageEntry {
    pub content: ClassificationContent,
    pub prediction: Option<PredictionEvidence>,
}

#[derive(Debug)]
pub struct CategoryCoveragePage {
    pub entries: Vec<CategoryCoverageEntry>,
    pub has_more: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub struct CategoryCoverageSupport {
    pub contents: u64,
    pub retained_contents: Vec<ClassificationContent>,
}

#[derive(Debug)]
pub struct CategoryCoverageSummary {
    pub snapshot: CategoryCoverageSnapshot,
    pub categories: BTreeMap<CategoryId, CategoryCoverageSupport>,
    pub classified_contents: u64,
    pub unclassified_contents: u64,
    pub evidence: ClassificationRevision,
}

impl CategoryCoverageSummary {
    pub fn is_complete(&self) -> bool {
        !self.snapshot.unverified_sources && self.unclassified_contents == 0
    }
}
