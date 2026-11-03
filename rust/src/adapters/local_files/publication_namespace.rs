use std::fs::File;
#[cfg(not(windows))]
use std::path::Path;

use crate::domain::{
    DiscoveredFile, ExpectedFileState, FileIdentityEvidence, FileVisit, MetadataInventoryEntry,
    ScanError, ScanIssue,
};
use crate::ports::media_inspection::MediaSource;
use crate::ports::source_publication::{
    PublicationDirectorySource, PublicationFileSource, PublicationNamespace,
    PublicationNamespaceFactory,
};

use super::directory_enumeration::PublicationGuardedFileVisits;
use super::{FileDiscovery, PublicationGuardedFileDiscovery};

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

impl PublicationFileSource for PublicationGuardedFileDiscovery {
    fn metadata_inventory_entry(
        &self,
        relative_path: &str,
    ) -> Result<MetadataInventoryEntry, ScanIssue> {
        self.metadata_inventory_entry(relative_path)
    }

    fn visit_relative_path(&self, relative_path: &str) -> FileVisit {
        self.visit_relative_path(relative_path)
    }
}

impl PublicationDirectorySource for PublicationGuardedFileDiscovery {
    type Entries<'a> = PublicationGuardedFileVisits<'a>;

    fn file_visits_in_directory(
        &self,
        relative_directory: &str,
    ) -> Result<Self::Entries<'_>, ScanIssue> {
        self.file_visits_in_directory(relative_directory)
    }
}

impl MediaSource for PublicationGuardedFileDiscovery {
    type Reader = File;

    fn open_media_reader(&self, file: &DiscoveredFile) -> std::io::Result<File> {
        self.discovery.open_media_reader(file)
    }
}

impl MediaSource for FileDiscovery {
    type Reader = File;

    fn open_media_reader(&self, file: &DiscoveredFile) -> std::io::Result<File> {
        #[cfg(windows)]
        {
            self.open_pinned_source_file(&file.relative_path)
        }
        #[cfg(not(windows))]
        {
            super::open_source_file(
                Path::new(&file.absolute_path),
                Path::new(&file.source_root_path),
            )
        }
    }
}
