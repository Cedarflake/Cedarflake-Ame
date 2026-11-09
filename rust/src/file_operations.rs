#[cfg(windows)]
pub use crate::adapters::WindowsFileOperations;
pub use crate::domain::file_operations::{
    FileAction, FileOperationDestination, FileOperationDirectory, FileOperationIssue,
    FileOperationIssueKind, FileOperationReceipt, FileOperationRecovery, FileOperationRequest,
    FileOperationResult, FileOperationSource,
};
pub use crate::ports::file_operations::{FileOperationControl, NativeFileOperations};
