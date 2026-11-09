use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use crate::domain::file_operations::{
    FileOperationDirectory, FileOperationIssue, FileOperationRequest, FileOperationResult,
    FileOperationSource,
};

#[derive(Clone, Default)]
pub struct FileOperationControl {
    cancelled: Arc<AtomicBool>,
    copied_bytes: Arc<AtomicU64>,
    total_bytes: Arc<AtomicU64>,
}

impl FileOperationControl {
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }

    pub fn copy_progress(&self) -> (u64, u64) {
        (
            self.copied_bytes.load(Ordering::Acquire),
            self.total_bytes.load(Ordering::Acquire),
        )
    }

    pub(crate) fn publish_copy_progress(&self, copied: u64, total: u64) {
        self.total_bytes.store(total, Ordering::Release);
        self.copied_bytes.store(copied, Ordering::Release);
    }
}

pub trait NativeFileOperations: Send + Sync {
    fn inspect_source(&self, path: &str) -> Result<FileOperationSource, FileOperationIssue>;
    fn inspect_directory(&self, path: &str) -> Result<FileOperationDirectory, FileOperationIssue>;

    /// The caller records this exact authorized item before invoking the native operation.
    /// A different-volume move is rejected unchanged; its copy/verify/remove stages belong to
    /// the durable application workflow. Unknown outcomes must be reconciled before retry.
    fn execute(
        &self,
        request: &FileOperationRequest,
        control: &FileOperationControl,
    ) -> FileOperationResult;

    /// Complete a previously authorized move after its exact successful copy receipt is durable.
    /// Verify bytes with both identities held before removing only the original through its handle.
    fn complete_move(
        &self,
        source: &FileOperationSource,
        destination: &FileOperationSource,
        control: &FileOperationControl,
    ) -> FileOperationResult;
}
