use std::path::Path;

use crate::domain::ScanError;
use crate::domain::file_operation_journal::{
    FileOperationEvent as Event, FileOperationPhase as Phase, FileOperationRecord,
};
use crate::domain::file_operations::{
    FileAction, FileOperationDestination, FileOperationIssue, FileOperationIssueKind as Kind,
    FileOperationReceipt, FileOperationRecovery, FileOperationSource,
};
use crate::ports::file_operations::NativeFileOperations;

#[derive(Clone, Debug)]
pub enum FileOperationInspection {
    NotStarted,
    CopyRetained,
    UnverifiedTransfer(FileOperationSource),
    Completed(FileOperationReceipt),
    NeedsAttention(FileOperationIssue),
    Settled,
}

impl super::FileOperationExecutor<'_> {
    pub fn reconcile(&mut self, id: u64) -> Result<FileOperationRecord, ScanError> {
        let record = self.journal.load(id)?;
        let inspection = match inspect_file_operation(&record, self.native) {
            FileOperationInspection::UnverifiedTransfer(destination) => {
                self.verify_transfer(&record, destination)?
            }
            inspection => inspection,
        };
        match inspection {
            FileOperationInspection::NotStarted => self.advance(&record, Event::Cancelled),
            FileOperationInspection::Completed(receipt)
                if record.checkpoint.phase != Phase::NativeCompleted =>
            {
                self.advance(&record, Event::NativeCompleted(receipt))
            }
            FileOperationInspection::CopyRetained
                if matches!(
                    record.checkpoint.phase,
                    Phase::RemovingSource | Phase::Uncertain
                ) =>
            {
                let copied = record.checkpoint.copied.clone().ok_or_else(|| {
                    ScanError::new(
                        "file_operation_missing_receipt",
                        "Recovery requires its recorded copy receipt",
                    )
                })?;
                self.advance(&record, Event::Copied(copied))
            }
            FileOperationInspection::NeedsAttention(issue)
                if record.checkpoint.phase != Phase::Uncertain =>
            {
                self.advance(&record, Event::Uncertain(issue))
            }
            _ => Ok(record),
        }
    }

    fn verify_transfer(
        &self,
        record: &FileOperationRecord,
        destination: FileOperationSource,
    ) -> Result<FileOperationInspection, ScanError> {
        let Some(proof) = self.journal.content_proof(&record.authorization)? else {
            return Ok(FileOperationInspection::NeedsAttention(unknown(
                "The interrupted transfer has no reliable original content evidence",
            )));
        };
        Ok(
            match self
                .native
                .verify_content(&destination, &proof, self.control)
            {
                Ok(()) => FileOperationInspection::Completed(FileOperationReceipt {
                    destination: Some(destination),
                    recovery: FileOperationRecovery::None,
                }),
                Err(issue) => FileOperationInspection::NeedsAttention(issue),
            },
        )
    }
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
    let source = if request.action == FileAction::Copy && checkpoint.receipt.is_some() {
        None
    } else {
        native.inspect_location(&FileOperationDestination {
            directory: request.source.parent.clone(),
            name: source_name.to_owned(),
        })?
    };
    let destination = checkpoint
        .destination
        .as_ref()
        .map(|location| native.inspect_location(location))
        .transpose()?
        .flatten();
    let source_retired = source.is_none()
        || (request.action == FileAction::Rename
            && source.as_ref().is_some_and(|source| {
                source.state.absolute_path != request.source.state.absolute_path
                    && destination
                        .as_ref()
                        .is_some_and(|destination| same_file(source, destination))
            }));
    if let Some(receipt) = &checkpoint.receipt {
        let target_matches = match (&receipt.destination, &destination) {
            (Some(expected), Some(actual)) => same_file(expected, actual),
            (None, None) => true,
            _ => false,
        };
        let source_matches = request.action == FileAction::Copy || source_retired;
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
        && source_retired
        && let Some(actual) = destination
        && actual.state.file_identity == request.source.state.file_identity
        && actual.state.file_size == request.source.state.file_size
        && actual.state.modified_unix_ms == request.source.state.modified_unix_ms
    {
        return Ok(FileOperationInspection::UnverifiedTransfer(actual));
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
