use crate::domain::{ExpectedFileState, FileIdentityEvidence, ScanError, ScanIssue};

pub(crate) trait PublicationNamespaceFactory {
    type Namespace: PublicationNamespace;

    fn open(
        &self,
        root_path: &str,
        expected_identity: Option<&FileIdentityEvidence>,
    ) -> Result<Self::Namespace, ScanError>;
}

/// Retain the source root and ancestor bindings until publication completes or the owner drops.
pub(crate) trait PublicationNamespace {
    fn metadata_inventory_root_identity(&self) -> Result<Option<FileIdentityEvidence>, ScanError>;
    fn require_metadata_inventory_root_identity(
        &self,
        expected: &FileIdentityEvidence,
    ) -> Result<(), ScanError>;
    fn contains_directory_identity(
        &self,
        relative_path: &str,
        expected: &FileIdentityEvidence,
    ) -> Result<bool, ScanIssue>;
    fn revalidate_relative_file_state(
        &self,
        relative_path: &str,
        expected: &ExpectedFileState,
    ) -> Result<(), ScanIssue>;
}
