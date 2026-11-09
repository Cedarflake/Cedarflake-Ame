use super::ScanError;
use super::file_operation_journal::FILE_OPERATION_PAGE_SIZE;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileOperationTaskKind {
    Execute,
    Inspect,
    ContinueCopiedMove,
}

#[derive(Clone, Debug)]
pub struct FileOperationTaskRequest {
    pub kind: FileOperationTaskKind,
    pub operation_ids: Vec<u64>,
}

impl FileOperationTaskRequest {
    pub(crate) fn validate(&self) -> Result<(), ScanError> {
        if self.operation_ids.is_empty()
            || self.operation_ids.len() > FILE_OPERATION_PAGE_SIZE
            || self.operation_ids.contains(&0)
            || self
                .operation_ids
                .iter()
                .collect::<std::collections::HashSet<_>>()
                .len()
                != self.operation_ids.len()
        {
            return Err(ScanError::new(
                "file_operation_task_invalid",
                "A file-operation task needs 1 to 128 distinct durable item IDs",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileOperationTaskPhase {
    Starting,
    Running,
    Stopping,
    Finished,
    Cancelled,
    NeedsAttention,
}

#[derive(Clone, Debug)]
pub struct FileOperationTaskProgress {
    pub kind: FileOperationTaskKind,
    pub phase: FileOperationTaskPhase,
    pub total_items: u64,
    pub processed_items: u64,
    pub published_items: u64,
    pub failed_items: u64,
    pub cancelled_items: u64,
    pub current_operation_id: Option<u64>,
    pub current_path: Option<String>,
    pub copied_bytes: u64,
    pub total_bytes: u64,
    pub failure: Option<ScanError>,
}
