use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::domain::ScanError;
use crate::domain::file_operation_journal::{
    AuthorizedFileOperation, FILE_OPERATION_PAGE_SIZE, FileOperationCatalogBinding,
};
use crate::domain::file_operation_selection::{
    FileOperationCommand, FileOperationSelectionSummary,
};
use crate::domain::file_operations::{FileAction, FileOperationDestination, FileOperationRequest};
use crate::domain::organization::validate_identifier;
use crate::ports::file_operation_journal::FileOperationJournal;
use crate::ports::file_operation_selection::FileOperationSelectionRepository;
use crate::ports::file_operations::NativeFileOperations;

use super::FileOperationSelectionSession;

pub(super) fn record(
    selection: &dyn FileOperationSelectionRepository,
    journal: &mut dyn FileOperationJournal,
    native: &dyn NativeFileOperations,
    batch_id: &str,
    command: &FileOperationCommand,
    summary: &FileOperationSelectionSummary,
    session: &FileOperationSelectionSession,
) -> Result<(), ScanError> {
    validate_identifier(batch_id)?;
    if journal.batch_count(batch_id)? != 0 {
        return Err(ScanError::new(
            "file_operation_batch_exists",
            "This operation identifier is already recorded",
        ));
    }
    let valid = match command.action {
        FileAction::Copy | FileAction::Move => {
            command
                .destination_directory
                .as_ref()
                .is_some_and(|path| !path.is_empty())
                && command.new_name.is_none()
        }
        FileAction::Rename => {
            command.destination_directory.is_none()
                && command
                    .new_name
                    .as_ref()
                    .is_some_and(|name| !name.is_empty())
                && summary.file_count == 1
        }
        FileAction::Recycle | FileAction::DeletePermanently => {
            command.destination_directory.is_none() && command.new_name.is_none()
        }
    };
    if !valid {
        return Err(ScanError::new(
            "file_operation_command_invalid",
            "The file action and its destination do not match",
        ));
    }
    let directory = command
        .destination_directory
        .as_deref()
        .map(|path| native.inspect_directory(path))
        .transpose()
        .map_err(|error| ScanError::new("file_operation_destination_unavailable", error.message))?;
    let authorized_unix_ms = now();
    let mut after = None;
    let mut completed = 0;
    let result = (|| {
        loop {
            session.check()?;
            let page = selection.page(after)?;
            if page.is_empty() {
                break;
            }
            let mut authorized = Vec::with_capacity(page.len());
            for candidate in page {
                session.check()?;
                let location = candidate.location;
                session
                    .progress
                    .lock()
                    .unwrap_or_else(|error| error.into_inner())
                    .current_path = Some(location.display_path.clone());
                let source = native
                    .inspect_source(&location.absolute_path)
                    .map_err(|error| {
                        ScanError::new("file_operation_source_unavailable", error.message)
                    })?;
                if source.state.file_identity != location.file_identity
                    || source.state.source_revision != location.source_revision
                    || source.state.file_size != location.file_size
                    || source.state.modified_unix_ms != location.modified_unix_ms
                {
                    return Err(ScanError::new(
                        "file_operation_source_changed",
                        "The selected file changed; refresh its selection before retrying",
                    ));
                }
                let destination = if matches!(
                    command.action,
                    FileAction::Copy | FileAction::Move | FileAction::Rename
                ) {
                    let name = match &command.new_name {
                        Some(name) => name.clone(),
                        None => Path::new(&location.absolute_path)
                            .file_name()
                            .and_then(|name| name.to_str())
                            .ok_or_else(|| {
                                ScanError::new(
                                    "file_operation_name_invalid",
                                    "The selected file has no supported filename",
                                )
                            })?
                            .to_owned(),
                    };
                    let destination = FileOperationDestination {
                        directory: directory.clone().unwrap_or_else(|| source.parent.clone()),
                        name,
                    };
                    if command.action != FileAction::Copy
                        && Path::new(&destination.directory.path).join(&destination.name)
                            == Path::new(&source.state.absolute_path)
                    {
                        return Err(ScanError::new(
                            "file_operation_same_destination",
                            "The source and destination are the same file",
                        ));
                    }
                    native
                        .destination_occupied(&destination)
                        .map_err(native_error)?;
                    Some(destination)
                } else {
                    None
                };
                after = Some(candidate.ordinal);
                authorized.push(AuthorizedFileOperation {
                    batch_id: batch_id.to_owned(),
                    ordinal: candidate.ordinal,
                    authorized_unix_ms,
                    catalog: FileOperationCatalogBinding {
                        asset_id: location.asset_id,
                        location_id: location.location_id,
                        source_generation: location.source_generation,
                        root_id: location.root_id,
                        root_generation: candidate.root_generation,
                    },
                    request: FileOperationRequest {
                        action: command.action,
                        source,
                        destination,
                    },
                });
            }
            journal.authorize(&authorized)?;
            completed += authorized.len() as u64;
            let mut progress = session
                .progress
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            progress.total_files = summary.file_count;
            progress.prepared_files = completed;
        }
        if completed != summary.file_count {
            return Err(ScanError::new(
                "file_operation_selection_changed",
                "The physical selection changed during preparation",
            ));
        }
        Ok(())
    })();
    if let Err(error) = result {
        if completed > 0 {
            retire(journal, batch_id).map_err(|retirement| {
                ScanError::new(
                    "file_operation_preparation_retirement_failed",
                    format!(
                        "{}; unable to retire unstarted records: {}",
                        error.message, retirement.message
                    ),
                )
            })?;
        }
        return Err(error);
    }
    Ok(())
}

pub(super) fn retire(
    journal: &mut dyn FileOperationJournal,
    batch_id: &str,
) -> Result<(), ScanError> {
    let mut after = None;
    loop {
        let page = journal.batch_page(batch_id, after, FILE_OPERATION_PAGE_SIZE)?;
        if page.is_empty() {
            return Ok(());
        }
        after = page.last().map(|record| record.authorization.ordinal);
        journal.cancel_unstarted(
            &page.iter().map(|record| record.id).collect::<Vec<_>>(),
            now(),
        )?;
    }
}

fn native_error(error: crate::domain::file_operations::FileOperationIssue) -> ScanError {
    let code = match error.kind {
        crate::domain::file_operations::FileOperationIssueKind::InvalidRequest => {
            "file_operation_name_invalid"
        }
        _ => "file_operation_destination_unavailable",
    };
    ScanError::new(code, error.message)
}

fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|value| i64::try_from(value.as_millis()).ok())
        .unwrap_or(0)
}
