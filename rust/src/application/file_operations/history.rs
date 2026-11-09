use crate::domain::ScanError;
use crate::domain::file_operation_journal::FileOperationRecord;
use crate::domain::file_operation_task::{
    FileOperationTaskKind, FileOperationTaskRequest, FileOperationTaskTarget,
};
use crate::ports::file_operation_journal::FileOperationJournal;

use super::{FileOperationTask, production};

pub(crate) fn history(
    before_id: Option<u64>,
    limit: u32,
) -> Result<Vec<FileOperationRecord>, ScanError> {
    let path = crate::application::storage_paths()?.catalog_path;
    production::open_journal(&path)?.history(before_id, limit as usize)
}

pub(crate) fn unfinished(after_id: u64, limit: u32) -> Result<Vec<FileOperationRecord>, ScanError> {
    let path = crate::application::storage_paths()?.catalog_path;
    production::open_journal(&path)?.unfinished(after_id, limit as usize)
}

pub(crate) fn recover(
    ids: Vec<u64>,
    continue_copied_move: bool,
    owner_window: Option<u64>,
) -> Result<FileOperationTask, ScanError> {
    let path = crate::application::storage_paths()?.catalog_path;
    let kind = if continue_copied_move {
        FileOperationTaskKind::ContinueCopiedMove
    } else {
        FileOperationTaskKind::Inspect
    };
    super::start_file_operation_task(
        path,
        FileOperationTaskRequest {
            kind,
            target: FileOperationTaskTarget::Items(ids),
        },
        owner_window,
    )
}
