use std::fs::File;
use std::os::windows::io::AsRawHandle;
use std::path::Path;

use windows_sys::Win32::Storage::FileSystem::{
    FILE_DISPOSITION_INFO, FileDispositionInfo, SetFileInformationByHandle,
};

use crate::domain::file_operations::{
    FileOperationIssue, FileOperationIssueKind as Kind, FileOperationReceipt,
    FileOperationRecovery, FileOperationRequest, FileOperationResult,
};
use crate::ports::file_operations::FileOperationControl;

use super::scope::OperationScope;

pub(super) fn permanently(
    request: &FileOperationRequest,
    scope: &OperationScope,
    control: &FileOperationControl,
) -> FileOperationResult {
    let source = scope
        .source
        .open_operation_source(&scope.source_name, &request.source.state, true)
        .map_err(|error| FileOperationIssue::new(Kind::SourceChanged, error.to_string()))?;
    if control.is_cancelled() {
        return Err(super::cancelled());
    }
    held_source(source, Path::new(&request.source.state.absolute_path))?;
    Ok(FileOperationReceipt {
        destination: None,
        recovery: FileOperationRecovery::None,
    })
}

pub(super) fn held_source(source: File, path: &Path) -> Result<(), FileOperationIssue> {
    let disposition = FILE_DISPOSITION_INFO { DeleteFile: true };
    // SAFETY: ADR 0042; callers retain the pinned parent and this revalidated DELETE-capable
    // handle without write/delete sharing. The deletion never reopens the source path.
    let removed = unsafe {
        SetFileInformationByHandle(
            source.as_raw_handle(),
            FileDispositionInfo,
            std::ptr::from_ref(&disposition).cast(),
            std::mem::size_of_val(&disposition) as u32,
        )
    };
    if removed == 0 {
        return Err(FileOperationIssue::new(
            Kind::NativeFailure,
            std::io::Error::last_os_error().to_string(),
        ));
    }
    drop(source);
    match path.symlink_metadata() {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        _ => Err(FileOperationIssue::new(
            Kind::OutcomeUnknown,
            "The removed path needs reconciliation",
        )),
    }
}
