use std::path::Path;

use crate::adapters::SqliteFileOperationJournal;
use crate::domain::file_operation_task::FileOperationTaskRequest;
use crate::domain::{LibraryChangeLane, ScanError};
use crate::ports::file_operation_catalog::FileOperationCatalog;

use super::FileOperationTask;

pub(super) fn open_selection(
    path: &Path,
    scope: crate::domain::file_operation_selection::FileOperationSelectionScope,
    cancelled: std::sync::Arc<std::sync::atomic::AtomicBool>,
) -> Result<
    Box<dyn crate::ports::file_operation_selection::FileOperationSelectionRepository>,
    ScanError,
> {
    let catalog = crate::application::catalog_session::open_catalog(path, LibraryChangeLane::Live)?;
    Ok(Box::new(
        crate::adapters::SqliteFileOperationSelection::new(catalog, scope, cancelled)?,
    ))
}

pub(super) fn source_inspector()
-> Result<Box<dyn crate::ports::file_operation_exports::NativeFilePreparation>, ScanError> {
    #[cfg(windows)]
    {
        Ok(Box::new(crate::adapters::WindowsFileOperations::new(None)))
    }
    #[cfg(not(windows))]
    {
        Err(ScanError::new(
            "file_operation_platform_unavailable",
            "File operations require Windows",
        ))
    }
}

pub(super) fn open_journal(path: &Path) -> Result<impl FileOperationCatalog + use<>, ScanError> {
    let catalog = crate::application::catalog_session::open_catalog(path, LibraryChangeLane::Live)?;
    Ok(SqliteFileOperationJournal::from_catalog(catalog))
}

#[cfg(windows)]
pub fn start_file_operation_task(
    catalog_path: std::path::PathBuf,
    request: FileOperationTaskRequest,
    owner_window: Option<u64>,
) -> Result<FileOperationTask, ScanError> {
    use crate::adapters::WindowsFileOperations;

    let owner = owner_window.map(usize::try_from).transpose().map_err(|_| {
        ScanError::new(
            "file_operation_owner_invalid",
            "The dialog owner is not a valid window handle",
        )
    })?;
    let native = WindowsFileOperations::new(owner);
    FileOperationTask::spawn(request, move || open_journal(&catalog_path), native)
}

#[cfg(not(windows))]
pub fn start_file_operation_task(
    _catalog_path: std::path::PathBuf,
    _request: FileOperationTaskRequest,
    _owner_window: Option<u64>,
) -> Result<FileOperationTask, ScanError> {
    Err(ScanError::new(
        "file_operation_platform_unavailable",
        "File operations require Windows",
    ))
}
