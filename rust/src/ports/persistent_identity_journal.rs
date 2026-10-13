use std::sync::atomic::AtomicBool;

use crate::domain::{
    FileIdentityEvidence, JournalFileReference, LibraryRootGeneration,
    PersistentJournalVolumeIdentity, ScanError,
};

#[derive(Clone, Debug)]
pub(crate) struct IdentityJournalRecord {
    pub(crate) file_reference: JournalFileReference,
    pub(crate) parent_reference: JournalFileReference,
    pub(crate) usn: i64,
    pub(crate) is_directory: bool,
    pub(crate) is_deleted: bool,
}

pub(crate) struct IdentityJournalRead {
    pub(crate) volume: PersistentJournalVolumeIdentity,
    pub(crate) journal_id: u64,
    pub(crate) start_usn: i64,
    pub(crate) end_usn: i64,
    pub(crate) max_records: u32,
}

pub(crate) struct IdentityJournalPage {
    pub(crate) covered_until_usn: i64,
    pub(crate) records: Vec<IdentityJournalRecord>,
}

#[derive(Clone, Debug)]
pub(crate) struct IdentityJournalRoot {
    pub(crate) root_id: String,
    pub(crate) root_generation: LibraryRootGeneration,
}

#[derive(Clone, Debug)]
pub(crate) struct IdentityJournalLocation {
    pub(crate) identity: FileIdentityEvidence,
    pub(crate) relative_path: String,
    pub(crate) is_directory: bool,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct IdentityJournalResolution {
    pub(crate) locations: Vec<IdentityJournalLocation>,
    pub(crate) access_denied: Vec<FileIdentityEvidence>,
    pub(crate) unresolved: Vec<FileIdentityEvidence>,
}

pub(crate) trait PersistentIdentityJournalSource: Send + Sync {
    fn can_probe_baseline(&self, root: &IdentityJournalRoot) -> bool;
    fn volume_identity(
        &self,
        root: &IdentityJournalRoot,
    ) -> Result<PersistentJournalVolumeIdentity, ScanError>;

    fn read_volume(
        &self,
        request: &IdentityJournalRead,
        cancelled: &AtomicBool,
    ) -> Result<IdentityJournalPage, ScanError>;

    fn current_locations(
        &self,
        root: &IdentityJournalRoot,
        references: &[JournalFileReference],
        cancelled: &AtomicBool,
    ) -> Result<IdentityJournalResolution, ScanError>;
}

pub(crate) trait PersistentJournalNamespaceReader: Send + Sync {
    fn has_snapshot(&self, root: &IdentityJournalRoot) -> Result<bool, ScanError>;

    fn previous_locations(
        &self,
        root: &IdentityJournalRoot,
        identities: &[FileIdentityEvidence],
    ) -> Result<Vec<IdentityJournalLocation>, ScanError>;
}

pub(crate) fn identity_for_reference(
    volume_serial: u64,
    reference: &JournalFileReference,
) -> FileIdentityEvidence {
    let mut bytes = [0_u8; 16];
    bytes[..reference.as_bytes().len()].copy_from_slice(reference.as_bytes());
    FileIdentityEvidence {
        scheme: "windows-file-id-128-v1".to_owned(),
        value: format!("{volume_serial:016x}:{:032x}", u128::from_le_bytes(bytes)),
    }
}
