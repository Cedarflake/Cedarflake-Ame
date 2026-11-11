use crate::application::file_operations::{FileOperationSelectionSession, FileOperationTask};
use crate::domain::ScanError;
use crate::domain::file_operation_journal::FileOperationRecord;
use crate::domain::file_operation_selection::{
    DuplicateCleanupGroup, FileOperationCommand, FileOperationPreparationProgress,
    FileOperationSelectionScope, FileOperationSelectionSummary,
};
use crate::domain::file_operation_task::FileOperationTaskProgress;
use crate::domain::organization::OrganizationSelection;

#[flutter_rust_bridge::frb(opaque)]
pub struct FileSelectionDraft {
    inner: FileOperationSelectionSession,
}

#[flutter_rust_bridge::frb(opaque)]
pub struct FileActionTask {
    inner: FileOperationTask,
}

pub fn open_file_selection(
    scope: FileOperationSelectionScope,
) -> Result<FileSelectionDraft, ScanError> {
    FileOperationSelectionSession::open(scope).map(|inner| FileSelectionDraft { inner })
}

pub fn load_file_operation_history(
    before_id: Option<u64>,
    limit: u32,
) -> Result<Vec<FileOperationRecord>, ScanError> {
    crate::application::file_operations::history::history(before_id, limit)
}

pub fn load_file_export_history(
    before_id: Option<u64>,
    limit: u32,
) -> Result<Vec<crate::domain::file_operation_exports::FileExportRecord>, ScanError> {
    crate::application::file_operations::history::exports(before_id, limit)
}

pub fn write_file_export_report(batch_id: String) -> Result<FileActionTask, ScanError> {
    crate::application::file_operations::history::write_report(batch_id)
        .map(|inner| FileActionTask { inner })
}

pub fn load_unfinished_file_operations(
    after_id: u64,
    limit: u32,
) -> Result<Vec<FileOperationRecord>, ScanError> {
    crate::application::file_operations::history::unfinished(after_id, limit)
}

pub fn recover_file_operations(
    ids: Vec<u64>,
    continue_copied_move: bool,
    owner_window: Option<u64>,
) -> Result<FileActionTask, ScanError> {
    crate::application::file_operations::history::recover(ids, continue_copied_move, owner_window)
        .map(|inner| FileActionTask { inner })
}

impl FileSelectionDraft {
    pub fn append_groups(&self, groups: Vec<DuplicateCleanupGroup>) -> Result<(), ScanError> {
        self.inner.append_groups(groups)
    }
    pub fn append_ids(&self, asset_ids: Vec<String>) -> Result<(), ScanError> {
        self.inner.append_ids(asset_ids)
    }
    pub fn append_locations(&self, locations: Vec<OrganizationSelection>) -> Result<(), ScanError> {
        self.inner.append_locations(locations)
    }
    pub fn prepare(&self) -> Result<FileOperationSelectionSummary, ScanError> {
        self.inner.prepare()
    }
    pub fn execute(
        &self,
        batch_id: String,
        command: FileOperationCommand,
        owner_window: Option<u64>,
    ) -> Result<FileActionTask, ScanError> {
        self.inner
            .execute(batch_id, command, owner_window)
            .map(|inner| FileActionTask { inner })
    }
    pub fn progress(&self) -> FileOperationPreparationProgress {
        self.inner.progress()
    }
    pub fn cancel(&self) {
        self.inner.cancel();
    }
    pub fn close(&self) -> Result<(), ScanError> {
        self.inner.close()
    }
}

impl FileActionTask {
    pub fn progress(&self) -> FileOperationTaskProgress {
        self.inner.snapshot()
    }
    pub fn cancel(&self) {
        self.inner.cancel();
    }
    pub fn close(&self) -> Result<(), ScanError> {
        self.inner.close()
    }
}
