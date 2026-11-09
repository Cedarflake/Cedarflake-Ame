use std::path::Path;

use crate::domain::file_operation_journal::{FileOperationPhase as Phase, FileOperationRecord};
use crate::domain::file_operations::{
    FileAction, FileOperationDestination, FileOperationIssue, FileOperationIssueKind as Kind,
    FileOperationReceipt, FileOperationRecovery, FileOperationSource,
};
use crate::ports::file_operations::NativeFileOperations;

#[derive(Clone, Debug)]
pub enum FileOperationInspection {
    NotStarted,
    CopyRetained,
    Completed(FileOperationReceipt),
    NeedsAttention(FileOperationIssue),
    Settled,
}

pub fn inspect_file_operation(
    record: &FileOperationRecord,
    native: &dyn NativeFileOperations,
) -> FileOperationInspection {
    match inspect(record, native) {
        Ok(inspection) => inspection,
        Err(issue) => FileOperationInspection::NeedsAttention(issue),
    }
}

fn inspect(
    record: &FileOperationRecord,
    native: &dyn NativeFileOperations,
) -> Result<FileOperationInspection, FileOperationIssue> {
    let checkpoint = &record.checkpoint;
    match checkpoint.phase {
        Phase::Prepared | Phase::NameOccupied => return Ok(FileOperationInspection::NotStarted),
        Phase::Published => return Ok(FileOperationInspection::Settled),
        Phase::Failed | Phase::Cancelled if checkpoint.copied.is_none() => {
            return Ok(FileOperationInspection::Settled);
        }
        _ => {}
    }
    let request = &record.authorization.request;
    let source_name = Path::new(&request.source.state.absolute_path)
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| unknown("The recorded source has no filename"))?;
    let source = native.inspect_location(&FileOperationDestination {
        directory: request.source.parent.clone(),
        name: source_name.to_owned(),
    })?;
    let destination = checkpoint
        .destination
        .as_ref()
        .map(|location| native.inspect_location(location))
        .transpose()?
        .flatten();
    if let Some(receipt) = &checkpoint.receipt {
        let target_matches = match (&receipt.destination, &destination) {
            (Some(expected), Some(actual)) => same_file(expected, actual),
            (None, None) => true,
            _ => false,
        };
        let source_matches = if request.action == FileAction::Copy {
            source
                .as_ref()
                .is_some_and(|actual| same_file(&request.source, actual))
        } else {
            source.is_none()
        };
        if target_matches && source_matches {
            return Ok(FileOperationInspection::Completed(receipt.clone()));
        }
        return Err(unknown("Files changed after the recorded native result"));
    }
    if let Some(copied) = &checkpoint.copied {
        if !destination
            .as_ref()
            .is_some_and(|actual| same_file(copied, actual))
        {
            return Err(unknown("The recorded copied file is missing or changed"));
        }
        if source
            .as_ref()
            .is_some_and(|actual| same_file(&request.source, actual))
        {
            return Ok(FileOperationInspection::CopyRetained);
        }
        if source.is_none() && matches!(checkpoint.phase, Phase::RemovingSource | Phase::Uncertain)
        {
            return Ok(FileOperationInspection::Completed(FileOperationReceipt {
                destination,
                recovery: FileOperationRecovery::None,
            }));
        }
        return Err(unknown(
            "The copied move's source no longer matches its authorization",
        ));
    }
    if matches!(request.action, FileAction::Move | FileAction::Rename)
        && source.is_none()
        && destination.as_ref().is_some_and(|actual| {
            actual.state.file_identity == request.source.state.file_identity
                && actual.state.file_size == request.source.state.file_size
                && actual.state.modified_unix_ms == request.source.state.modified_unix_ms
        })
    {
        return Ok(FileOperationInspection::Completed(FileOperationReceipt {
            destination,
            recovery: FileOperationRecovery::None,
        }));
    }
    Err(unknown(
        "The interrupted native result requires inspection; no operation was replayed",
    ))
}

fn same_file(expected: &FileOperationSource, actual: &FileOperationSource) -> bool {
    expected.parent.identity == actual.parent.identity
        && expected.state.absolute_path == actual.state.absolute_path
        && expected.state.file_identity == actual.state.file_identity
        && expected.state.source_revision == actual.state.source_revision
        && expected.state.file_size == actual.state.file_size
        && expected.state.modified_unix_ms == actual.state.modified_unix_ms
}

fn unknown(message: &str) -> FileOperationIssue {
    FileOperationIssue::new(Kind::OutcomeUnknown, message)
}
