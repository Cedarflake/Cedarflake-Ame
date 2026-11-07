use crate::domain::organization::OrganizationSelection;
use crate::domain::semantic_discovery::ImageContentIdentity;
use crate::domain::{GalleryQuery, ScanError};

pub(crate) struct SemanticCandidate {
    pub source: OrganizationSelection,
    pub content: Option<ImageContentIdentity>,
}

pub(crate) struct SemanticCandidateWindow {
    pub revision: u64,
    pub candidates: Vec<SemanticCandidate>,
}

pub(crate) trait SemanticCatalogSnapshot {
    fn candidates(
        &self,
        query: &GalleryQuery,
        after_location: &str,
    ) -> Result<SemanticCandidateWindow, ScanError>;
}

pub(crate) trait SemanticCatalog {
    fn publish_semantic_content(
        &mut self,
        source: &OrganizationSelection,
    ) -> Result<bool, ScanError>;

    fn semantic_source(
        &self,
        source: &OrganizationSelection,
    ) -> Result<Option<crate::domain::exact_duplicates::FingerprintCandidate>, ScanError>;

    fn semantic_content(
        &self,
        source: &OrganizationSelection,
    ) -> Result<Option<ImageContentIdentity>, ScanError>;

    fn semantic_snapshot(&self) -> Result<Box<dyn SemanticCatalogSnapshot + '_>, ScanError>;

    fn semantic_candidates(
        &self,
        query: &GalleryQuery,
        after_location: &str,
    ) -> Result<SemanticCandidateWindow, ScanError> {
        self.semantic_snapshot()?.candidates(query, after_location)
    }
}
