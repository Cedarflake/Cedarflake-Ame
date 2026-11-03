use crate::domain::{ExpectedFileState, FileIdentityEvidence, ScanError, ScanIssue};
use crate::ports::source_publication::{PublicationNamespace, PublicationNamespaceFactory};

use super::PublicationGuardedFileDiscovery;

pub(crate) struct LocalPublicationNamespaces;

impl PublicationNamespaceFactory for LocalPublicationNamespaces {
    type Namespace = PublicationGuardedFileDiscovery;

    fn open(
        &self,
        root_path: &str,
        expected_identity: Option<&FileIdentityEvidence>,
    ) -> Result<Self::Namespace, ScanError> {
        PublicationGuardedFileDiscovery::new_metadata_inventory_source_guard(
            root_path,
            expected_identity,
        )
    }
}

impl PublicationNamespace for PublicationGuardedFileDiscovery {
    fn metadata_inventory_root_identity(&self) -> Result<Option<FileIdentityEvidence>, ScanError> {
        self.metadata_inventory_root_identity()
    }

    fn require_metadata_inventory_root_identity(
        &self,
        expected: &FileIdentityEvidence,
    ) -> Result<(), ScanError> {
        self.require_metadata_inventory_root_identity(expected)
    }

    fn contains_directory_identity(
        &self,
        relative_path: &str,
        expected: &FileIdentityEvidence,
    ) -> Result<bool, ScanIssue> {
        self.contains_directory_identity(relative_path, expected)
    }

    fn revalidate_relative_file_state(
        &self,
        relative_path: &str,
        expected: &ExpectedFileState,
    ) -> Result<(), ScanIssue> {
        self.revalidate_relative_file_state(relative_path, expected)
    }
}
