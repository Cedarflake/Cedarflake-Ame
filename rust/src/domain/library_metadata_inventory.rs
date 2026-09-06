use super::{
    FileIdentityEvidence, JournalFileReference, JournalIdentifier, JournalUsn, LibraryChangeId,
    LibraryRootGeneration, PERSISTENT_JOURNAL_CONTRACT_VERSION, PersistentJournalVolumeIdentity,
    ScanError, SourceRevisionEvidence,
};

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum LibraryRecoveryAuthorityReason {
    ExistingRootBaseline,
    FirstImportBoundary,
    JournalGap,
    JournalReset,
    JournalTrim,
    JournalReconstructionFailure,
    ContainmentFailure,
    BrokerAfterCurrentFailure,
    WatcherUncoveredGap,
}

impl LibraryRecoveryAuthorityReason {
    pub const fn requires_opening_boundary(self) -> bool {
        matches!(self, Self::ExistingRootBaseline | Self::FirstImportBoundary)
    }

    pub const fn is_journal_continuity_failure(self) -> bool {
        matches!(
            self,
            Self::JournalGap
                | Self::JournalReset
                | Self::JournalTrim
                | Self::JournalReconstructionFailure
                | Self::ContainmentFailure
                | Self::BrokerAfterCurrentFailure
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LibraryRecoveryOpeningBoundary {
    pub volume: PersistentJournalVolumeIdentity,
    pub root_file_reference: JournalFileReference,
    pub journal_id: JournalIdentifier,
    pub next_usn: JournalUsn,
    pub protocol_version: u16,
    pub contract_version: u16,
}

impl LibraryRecoveryOpeningBoundary {
    pub fn validate(&self) -> Result<(), ScanError> {
        self.volume.validate()?;
        if self.protocol_version == 0
            || self.contract_version != PERSISTENT_JOURNAL_CONTRACT_VERSION
        {
            return Err(ScanError::new(
                "metadata_inventory_recovery_opening_boundary_invalid",
                "The recovery opening boundary identity is invalid",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LibraryRecoveryAuthority {
    pub change_id: LibraryChangeId,
    pub run_id: String,
    pub root_id: String,
    pub root_generation: LibraryRootGeneration,
    pub reason: LibraryRecoveryAuthorityReason,
    pub opening_boundary: Option<LibraryRecoveryOpeningBoundary>,
    pub authorized_unix_ms: i64,
    pub retired_unix_ms: Option<i64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MetadataInventoryScope {
    Root,
    Subtree { relative_path: String },
}

impl MetadataInventoryScope {
    pub fn relative_path(&self) -> &str {
        match self {
            Self::Root => "",
            Self::Subtree { relative_path } => relative_path,
        }
    }
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum MetadataInventoryEntryKind {
    File,
    Directory,
    Other,
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum MetadataInventoryPlaceholderState {
    Available,
    Offline,
    RecallOnOpen,
    RecallOnDataAccess,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MetadataInventoryEntry {
    pub relative_path: String,
    pub kind: MetadataInventoryEntryKind,
    pub file_size: Option<u64>,
    pub modified_unix_ms: i64,
    pub file_identity: Option<FileIdentityEvidence>,
    pub source_revision: Option<SourceRevisionEvidence>,
    pub placeholder_state: MetadataInventoryPlaceholderState,
    pub is_reparse_point: bool,
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum MetadataInventoryFrontierState {
    Pending,
    Enumerating,
    Completed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MetadataInventoryFrontierEntry {
    pub ordinal: u64,
    pub relative_directory: String,
    pub state: MetadataInventoryFrontierState,
    pub directory_identity: Option<FileIdentityEvidence>,
    pub resume_after_relative_path: Option<String>,
    pub enumerated_entry_count: u64,
}

impl MetadataInventoryFrontierEntry {
    pub fn enumerating(
        ordinal: u64,
        relative_directory: impl Into<String>,
        directory_identity: Option<FileIdentityEvidence>,
        resume_after_relative_path: Option<String>,
        enumerated_entry_count: u64,
    ) -> Self {
        Self {
            ordinal,
            relative_directory: relative_directory.into(),
            state: MetadataInventoryFrontierState::Enumerating,
            directory_identity,
            resume_after_relative_path,
            enumerated_entry_count,
        }
    }

    pub fn completed(
        relative_directory: impl Into<String>,
        directory_identity: Option<FileIdentityEvidence>,
    ) -> Self {
        Self {
            ordinal: 0,
            relative_directory: relative_directory.into(),
            state: MetadataInventoryFrontierState::Completed,
            directory_identity,
            resume_after_relative_path: None,
            enumerated_entry_count: 0,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MetadataInventoryPage {
    pub page_index: u64,
    pub entries: Vec<MetadataInventoryEntry>,
    pub cursor: Option<String>,
    pub is_complete: bool,
    pub frontier: Vec<MetadataInventoryFrontierEntry>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MetadataInventoryRunRequest {
    pub run_id: String,
    pub root_id: String,
    pub root_generation: LibraryRootGeneration,
    pub epoch: u64,
    pub scope: MetadataInventoryScope,
    pub started_unix_ms: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MetadataInventoryStartRequest {
    pub run_id: String,
    pub root_id: String,
    pub root_generation: LibraryRootGeneration,
    pub scope: MetadataInventoryScope,
    pub started_unix_ms: i64,
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum MetadataInventoryRunStatus {
    Running,
    Comparing,
    Completed,
    Failed,
    Cancelled,
    Superseded,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MetadataInventoryRun {
    pub request: MetadataInventoryRunRequest,
    pub status: MetadataInventoryRunStatus,
    pub next_page_index: u64,
    pub enumeration_cursor: Option<String>,
    pub comparison_cursor: Option<String>,
    pub absence_cursor: Option<String>,
    pub staged_entry_count: u64,
    pub candidate_count: u64,
    pub enumeration_complete: bool,
    pub absence_authority: bool,
    pub updated_unix_ms: i64,
    pub completed_unix_ms: Option<i64>,
    pub last_issue_code: Option<String>,
    pub last_issue_message: Option<String>,
    pub frontier: Vec<MetadataInventoryFrontierEntry>,
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum MetadataInventoryComparisonStatus {
    Unchanged,
    Enqueued,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MetadataInventoryComparisonUpdate {
    pub relative_path: String,
    pub status: MetadataInventoryComparisonStatus,
    pub candidate_previous_relative_path: Option<String>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MetadataInventoryCleanupReport {
    pub removed_entry_count: u32,
    pub removed_run_count: u32,
    pub has_more: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MetadataInventoryReport {
    pub staged_entry_count: u64,
    pub candidate_count: u64,
    pub unchanged_count: u64,
    pub absence_candidate_count: u64,
    pub enqueued_count: u64,
    pub coalesced_count: u64,
    pub superseded_count: u64,
    pub cleanup_pending: bool,
    pub awaiting_closing_boundary: bool,
    pub is_complete: bool,
    pub is_cancelled: bool,
    pub is_backpressured: bool,
}
