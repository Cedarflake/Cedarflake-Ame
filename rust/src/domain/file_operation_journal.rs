use serde::{Deserialize, Serialize};

use crate::domain::ScanError;
use crate::domain::file_operations::{
    FileAction, FileOperationDestination, FileOperationIssue, FileOperationReceipt,
    FileOperationRequest, FileOperationSource,
};

mod transitions;

pub const FILE_OPERATION_PAGE_SIZE: usize = 128;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FileOperationCatalogBinding {
    pub asset_id: String,
    pub location_id: String,
    pub source_generation: u64,
    pub root_id: String,
    pub root_generation: u64,
}

#[derive(Clone, Debug)]
pub struct AuthorizedFileOperation {
    pub batch_id: String,
    pub ordinal: u64,
    pub authorized_unix_ms: i64,
    pub catalog: FileOperationCatalogBinding,
    pub request: FileOperationRequest,
}

impl AuthorizedFileOperation {
    pub fn validate(&self) -> Result<(), ScanError> {
        for identifier in [
            &self.batch_id,
            &self.catalog.asset_id,
            &self.catalog.location_id,
            &self.catalog.root_id,
        ] {
            if identifier.is_empty() || identifier.len() > 256 || identifier.contains('\0') {
                return Err(invalid("An operation requires bounded catalog identifiers"));
            }
        }
        if self.catalog.source_generation == 0 || self.catalog.root_generation == 0 {
            return Err(invalid(
                "An operation requires a current catalog generation",
            ));
        }
        let requires_destination = matches!(
            self.request.action,
            FileAction::Copy | FileAction::Move | FileAction::Rename
        );
        if requires_destination != self.request.destination.is_some() {
            return Err(invalid(
                "The action and authorized destination do not match",
            ));
        }
        validate_source(&self.request.source)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FileOperationPhase {
    Prepared,
    Executing,
    NameOccupied,
    Copied,
    RemovingSource,
    NativeCompleted,
    Published,
    Failed,
    Cancelled,
    Uncertain,
}

#[derive(Clone, Debug)]
pub struct FileOperationCheckpoint {
    pub version: u64,
    pub phase: FileOperationPhase,
    pub destination: Option<FileOperationDestination>,
    pub copied: Option<FileOperationSource>,
    pub receipt: Option<FileOperationReceipt>,
    pub issue: Option<FileOperationIssue>,
    pub catalog_revision: Option<u64>,
}

impl Default for FileOperationCheckpoint {
    fn default() -> Self {
        Self {
            version: 0,
            phase: FileOperationPhase::Prepared,
            destination: None,
            copied: None,
            receipt: None,
            issue: None,
            catalog_revision: None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct FileOperationRecord {
    pub id: u64,
    pub authorization: AuthorizedFileOperation,
    pub checkpoint: FileOperationCheckpoint,
    pub updated_unix_ms: i64,
}

#[derive(Clone, Debug)]
pub enum FileOperationEvent {
    Started(Option<FileOperationDestination>),
    NameOccupied,
    Copied(FileOperationSource),
    RemovingSource,
    NativeCompleted(FileOperationReceipt),
    Published(u64),
    Failed(FileOperationIssue),
    Cancelled,
    Uncertain(FileOperationIssue),
}

pub(crate) fn invalid(message: impl Into<String>) -> ScanError {
    ScanError::new("file_operation_journal_invalid", message)
}

pub(crate) fn validate_source(source: &FileOperationSource) -> Result<(), ScanError> {
    let state = &source.state;
    if source.parent.path.is_empty()
        || source.parent.identity.scheme.is_empty()
        || source.parent.identity.value.is_empty()
        || state.absolute_path.is_empty()
        || state
            .file_identity
            .as_ref()
            .is_none_or(|identity| identity.scheme.is_empty() || identity.value.is_empty())
        || state
            .source_revision
            .as_ref()
            .is_none_or(|revision| revision.scheme.is_empty() || revision.value.is_empty())
    {
        return Err(invalid(
            "The operation requires complete source identity evidence",
        ));
    }
    Ok(())
}
