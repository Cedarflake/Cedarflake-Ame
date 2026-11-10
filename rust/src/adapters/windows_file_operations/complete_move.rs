use std::io::Read;
use std::path::Path;

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
    super::remove::held_source(original, Path::new(&source.state.absolute_path))?;
    Ok(FileOperationReceipt {
        destination: Some(destination.clone()),
        recovery: FileOperationRecovery::None,
    })
}
