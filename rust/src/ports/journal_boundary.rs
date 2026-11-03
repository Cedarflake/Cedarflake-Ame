use std::path::Path;

use crate::domain::{
    JournalFileReference, JournalIdentifier, JournalUsn, LibraryRootGeneration,
    PersistentJournalVolumeIdentity, ScanError,
};

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum JournalBoundaryProbe {
    Supported(SupportedJournalBoundary),
    LiveOnly { protocol_version: u16 },
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct SupportedJournalBoundary {
    pub(crate) protocol_version: u16,
    pub(crate) volume: PersistentJournalVolumeIdentity,
    pub(crate) root_file_reference: JournalFileReference,
    pub(crate) journal_id: JournalIdentifier,
    pub(crate) first_usn: JournalUsn,
    pub(crate) next_usn: JournalUsn,
}

pub(crate) trait JournalBoundaryReader {
    fn read_boundary(&self) -> Result<JournalBoundaryProbe, ScanError>;
}

pub(crate) trait JournalBoundarySource {
    /// Retains the root identity handle through the caller's verification or publication.
    type Root: JournalBoundaryReader;

    fn open_root(
        &self,
        root_id: &str,
        root_generation: LibraryRootGeneration,
        root_path: &Path,
    ) -> Result<Self::Root, ScanError>;
}

/// An independently owned connection, closed after verification even when the read fails.
pub(crate) trait JournalBoundarySession: JournalBoundaryReader {
    fn close(&self) -> Result<(), ScanError>;
}
