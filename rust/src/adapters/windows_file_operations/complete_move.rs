use std::io::Read;
use std::os::windows::io::AsRawHandle;
use std::path::Path;

use windows_sys::Win32::Storage::FileSystem::{
    FILE_DISPOSITION_INFO, FileDispositionInfo, SetFileInformationByHandle,
};

use crate::domain::file_operations::{
    FileOperationIssue, FileOperationIssueKind as Kind, FileOperationReceipt,
    FileOperationRecovery, FileOperationResult, FileOperationSource,
};
use crate::ports::file_operations::FileOperationControl;

use super::scope;

pub(super) fn complete_move(
    source: &FileOperationSource,
    destination: &FileOperationSource,
    control: &FileOperationControl,
) -> FileOperationResult {
    if source.state.file_identity.is_none()
        || destination.state.file_identity.is_none()
        || source.state.file_identity == destination.state.file_identity
        || source.state.file_size != destination.state.file_size
    {
        return Err(FileOperationIssue::new(
            Kind::InvalidRequest,
            "Move completion requires two distinct files of the same size",
        ));
    }
    let (source_guard, source_name) = scope::open_source(source)?;
    let (destination_guard, destination_name) = scope::open_source(destination)?;
    let mut original = source_guard
        .open_operation_source(&source_name, &source.state, true)
        .map_err(|error| FileOperationIssue::new(Kind::SourceChanged, error.to_string()))?;
    let mut copy = destination_guard
        .open_operation_source(&destination_name, &destination.state, false)
        .map_err(|error| FileOperationIssue::new(Kind::DestinationChanged, error.to_string()))?;
    let mut original_bytes = vec![0u8; 1024 * 1024];
    let mut copied_bytes = vec![0u8; original_bytes.len()];
    let mut remaining = source.state.file_size;
    while remaining != 0 {
        if control.is_cancelled() {
            return Err(super::cancelled());
        }
        let length = remaining.min(original_bytes.len() as u64) as usize;
        original
            .read_exact(&mut original_bytes[..length])
            .and_then(|()| copy.read_exact(&mut copied_bytes[..length]))
            .map_err(|error| FileOperationIssue::new(Kind::Unavailable, error.to_string()))?;
        if original_bytes[..length] != copied_bytes[..length] {
            return Err(FileOperationIssue::new(
                Kind::DestinationChanged,
                "The completed copy does not preserve the source content",
            ));
        }
        remaining -= length as u64;
    }
    if control.is_cancelled() {
        return Err(super::cancelled());
    }
    let disposition = FILE_DISPOSITION_INFO { DeleteFile: true };
    // SAFETY: ADR 0042; this exact DELETE-capable source handle and verified destination
    // stay open without write/delete sharing through the call. No path is reopened for deletion.
    let removed = unsafe {
        SetFileInformationByHandle(
            original.as_raw_handle(),
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
    drop(original);
    match Path::new(&source.state.absolute_path).symlink_metadata() {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(FileOperationReceipt {
            destination: Some(destination.clone()),
            recovery: FileOperationRecovery::None,
        }),
        _ => Err(FileOperationIssue::new(
            Kind::OutcomeUnknown,
            "Move source removal needs reconciliation",
        )),
    }
}
