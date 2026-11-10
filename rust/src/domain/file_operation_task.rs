use super::ScanError;
use super::file_operation_journal::FILE_OPERATION_PAGE_SIZE;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileOperationTaskKind {
    Execute,
    Inspect,
    ContinueCopiedMove,
    WriteReport,
}

#[derive(Clone, Debug)]
pub struct FileOperationTaskRequest {
    pub kind: FileOperationTaskKind,
    pub target: FileOperationTaskTarget,
}

#[derive(Clone, Debug)]
pub enum FileOperationTaskTarget {
    Items(Vec<u64>),
    Batch {
        batch_id: String,
        expected_items: u64,
    },
    Export(String),
}

impl FileOperationTaskRequest {
    pub(crate) fn validate(&self) -> Result<(), ScanError> {
        let valid = match &self.target {
            FileOperationTaskTarget::Items(ids) => {
                self.kind != FileOperationTaskKind::WriteReport
                    && !ids.is_empty()
                    && ids.len() <= FILE_OPERATION_PAGE_SIZE
                    && !ids.contains(&0)
                    && ids.iter().collect::<std::collections::HashSet<_>>().len() == ids.len()
            }
            FileOperationTaskTarget::Batch { batch_id, .. } => {
                self.kind == FileOperationTaskKind::Execute
                    && !batch_id.is_empty()
                    && batch_id.len() <= 256
                    && !batch_id.contains('\0')
            }
            FileOperationTaskTarget::Export(id) => {
                self.kind == FileOperationTaskKind::WriteReport
                    && !id.is_empty()
                    && id.len() <= 256
                    && !id.contains('\0')
            }
        };
        if !valid {
            return Err(ScanError::new(
                "file_operation_task_invalid",
                "A task needs a bounded item selection or an exact recorded batch",
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
    pub skipped_items: u64,
    pub export: Option<super::file_operation_exports::FileExportRecord>,
    pub current_operation_id: Option<u64>,
    pub current_path: Option<String>,
    pub copied_bytes: u64,
    pub total_bytes: u64,
    pub failure: Option<ScanError>,
}
