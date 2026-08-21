use super::{FileIdentityEvidence, LibraryRootGeneration};

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
    pub placeholder_state: MetadataInventoryPlaceholderState,
    pub is_reparse_point: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MetadataInventoryPage {
    pub page_index: u64,
    pub entries: Vec<MetadataInventoryEntry>,
    pub cursor: Option<String>,
    pub is_complete: bool,
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
    pub is_complete: bool,
    pub is_cancelled: bool,
}
