use serde::{Deserialize, Serialize};

use crate::domain::{ExpectedFileState, FileIdentityEvidence};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FileAction {
    Copy,
    Move,
    Rename,
    Recycle,
    DeletePermanently,
}

#[derive(Clone, Debug)]
pub struct FileOperationDirectory {
    pub path: String,
    pub identity: FileIdentityEvidence,
}

#[derive(Clone, Debug)]
pub struct FileOperationSource {
    pub parent: FileOperationDirectory,
    pub state: ExpectedFileState,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileOperationContentProof {
    pub scheme: String,
    pub digest: String,
    pub byte_length: u64,
}

#[derive(Clone, Debug)]
pub struct FileOperationDestination {
    pub directory: FileOperationDirectory,
    pub name: String,
}

#[derive(Clone, Debug)]
pub struct FileOperationRequest {
    pub action: FileAction,
    pub source: FileOperationSource,
    pub destination: Option<FileOperationDestination>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FileOperationIssueKind {
    InvalidRequest,
    SourceChanged,
    DestinationChanged,
    Collision,
    Unavailable,
    DifferentVolume,
    Cancelled,
    NativeFailure,
    OutcomeUnknown,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FileOperationIssue {
    pub kind: FileOperationIssueKind,
    pub message: String,
}

impl std::fmt::Display for FileOperationIssue {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{:?}: {}", self.kind, self.message)
    }
}

impl std::error::Error for FileOperationIssue {}

impl FileOperationIssue {
    pub(crate) fn new(kind: FileOperationIssueKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct FileOperationReceipt {
    pub destination: Option<FileOperationSource>,
    pub recovery: FileOperationRecovery,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FileOperationRecovery {
    None,
    SystemRecycleBin,
}

pub type FileOperationResult = Result<FileOperationReceipt, FileOperationIssue>;
