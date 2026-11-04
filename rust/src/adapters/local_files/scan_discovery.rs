use std::path::PathBuf;

use crate::domain::{FileIdentityEvidence, FileVisit, ScanError, ScanIssue};
use crate::ports::scan_discovery::ScanFileDiscovery;

use super::FileDiscovery;
use super::scan_enumeration::DirectoryEntryPaths;

impl ScanFileDiscovery for FileDiscovery {
    type Entries<'a> = DirectoryEntryPaths<'a>;

    fn canonical_root(&self) -> Result<PathBuf, ScanError> {
        self.canonical_root()
    }

    fn metadata_inventory_root_identity(&self) -> Result<Option<FileIdentityEvidence>, ScanError> {
        self.metadata_inventory_root_identity()
    }

    fn entry_paths_in_directory(
        &self,
        relative_directory: &str,
    ) -> Result<Self::Entries<'_>, ScanIssue> {
        self.entry_paths_in_directory(relative_directory)
    }

    fn visit_relative_path(&self, relative_path: &str) -> FileVisit {
        self.visit_relative_path(relative_path)
    }
}
