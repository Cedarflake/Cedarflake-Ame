use std::path::PathBuf;

use crate::domain::{FileIdentityEvidence, FileVisit, ScanError, ScanIssue};

pub(crate) trait ScanFileDiscovery {
    type Entries<'a>: ScanDirectoryEntries
    where
        Self: 'a;

    fn canonical_root(&self) -> Result<PathBuf, ScanError>;
    fn metadata_inventory_root_identity(&self) -> Result<Option<FileIdentityEvidence>, ScanError>;
    fn entry_paths_in_directory(
        &self,
        relative_directory: &str,
    ) -> Result<Self::Entries<'_>, ScanIssue>;
    fn visit_relative_path(&self, relative_path: &str) -> FileVisit;
}

/// Retain the directory binding until complete enumeration and closing identity validation.
pub(crate) trait ScanDirectoryEntries: Iterator<Item = String> {
    fn finish(self) -> Result<Option<FileIdentityEvidence>, ScanIssue>;
}
