use std::time::{SystemTime, UNIX_EPOCH};

use crate::domain::ScanError;
use crate::domain::file_operation_exports::{
    FileExportBatch, FileExportSkip, FileOrganizationDestination, FileOrganizationRule,
};
use crate::domain::file_operation_journal::FILE_OPERATION_PAGE_SIZE;
use crate::domain::file_operation_selection::{
    FileOperationCommand, FileOperationSelectionSummary,
};
use crate::domain::file_operations::FileAction;
use crate::domain::organization::validate_identifier;
use crate::ports::file_operation_catalog::FileOperationCatalog;
use crate::ports::file_operation_exports::NativeFilePreparation;
use crate::ports::file_operation_journal::FileOperationJournal;
use crate::ports::file_operation_selection::FileOperationSelectionRepository;

use super::FileOperationSelectionSession;

pub(super) fn record(
    selection: &dyn FileOperationSelectionRepository,
    journal: &mut impl FileOperationCatalog,
    native: &dyn NativeFilePreparation,
    batch_id: &str,
    command: &FileOperationCommand,
    summary: &FileOperationSelectionSummary,
    session: &FileOperationSelectionSession,
) -> Result<u64, ScanError> {
    validate_identifier(batch_id)?;
    if journal.batch_count(batch_id)? != 0 || journal.load_export(batch_id)?.is_some() {
        return Err(ScanError::new(
            "file_operation_batch_exists",
            "This operation identifier is already recorded",
        ));
    }
    validate(command, summary)?;
    let directory = command
        .destination_directory
        .as_deref()
        .map(|path| native.inspect_directory(path))
        .transpose()
        .map_err(|error| ScanError::new("file_operation_destination_unavailable", error.message))?;
    let authorized_unix_ms = now();
    if let Some(root) = &directory {
        journal.begin_export(&FileExportBatch {
            batch_id: batch_id.to_owned(),
            authorized_unix_ms,
            action: command.action,
            rule: command.organization_rule,
            selected_files: summary.file_count,
            root: root.clone(),
        })?;
    }
    let mut after = None;
    let mut prepared = 0;
    let mut executable = 0;
    let result = (|| {
        loop {
            session.check()?;
            let page = selection.page(after)?;
            if page.is_empty() {
                break;
            }
            let mut authorized = Vec::with_capacity(page.len());
            let mut skipped = Vec::new();
            for candidate in page {
                session.check()?;
                session
                    .progress
                    .lock()
                    .unwrap_or_else(|error| error.into_inner())
                    .current_path = Some(candidate.location.display_path.clone());
                after = Some(candidate.ordinal);
                if let FileOrganizationDestination::Skip(reason) =
                    &candidate.organization_destination
                {
                    skipped.push(FileExportSkip {
                        ordinal: candidate.ordinal,
                        source: candidate.location.absolute_path.clone(),
                        reason: reason.clone(),
                    });
                } else {
                    match super::candidate::authorize(
                        &candidate,
                        command,
                        directory.as_ref(),
                        native,
                        batch_id,
                        authorized_unix_ms,
                    ) {
                        Ok(item) => authorized.push(item),
                        Err(error) if directory.is_some() => skipped.push(FileExportSkip {
                            ordinal: candidate.ordinal,
                            source: candidate.location.absolute_path.clone(),
                            reason: skip_reason(&error),
                        }),
                        Err(error) => return Err(error),
                    }
                }
            }
            if !authorized.is_empty() {
                journal.authorize(&authorized)?;
            }
            if !skipped.is_empty() {
                journal.record_export_skips(batch_id, &skipped)?;
            }
            executable += authorized.len() as u64;
            prepared += (authorized.len() + skipped.len()) as u64;
            let mut progress = session
                .progress
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            progress.total_files = summary.file_count;
            progress.prepared_files = prepared;
        }
        if prepared != summary.file_count {
            return Err(ScanError::new(
                "file_operation_selection_changed",
                "The physical selection changed during preparation",
            ));
        }
        Ok(executable)
    })();
    if let Err(error) = result {
        retire(journal, batch_id).map_err(|retirement| {
            ScanError::new(
                "file_operation_preparation_retirement_failed",
                format!(
                    "{}; unable to retire unstarted records: {}",
                    error.message, retirement.message
                ),
            )
        })?;
        return Err(error);
    }
    result
}

fn validate(
    command: &FileOperationCommand,
    summary: &FileOperationSelectionSummary,
) -> Result<(), ScanError> {
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
                && command.organization_rule == FileOrganizationRule::Direct
        }
        FileAction::Recycle | FileAction::DeletePermanently => {
            command.destination_directory.is_none()
                && command.new_name.is_none()
                && command.organization_rule == FileOrganizationRule::Direct
        }
    };
    if valid {
        Ok(())
    } else {
        Err(ScanError::new(
            "file_operation_command_invalid",
            "The file action and its destination do not match",
        ))
    }
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

fn skip_reason(error: &ScanError) -> String {
    let message = match error.code.as_str() {
        "file_operation_source_unavailable" => "源文件不可用，可能已移除、被占用或尚未下载",
        "file_operation_source_changed" => "源文件已发生变化，需要重新选择",
        "file_operation_same_destination" => "文件已在目标位置",
        "file_organization_name_invalid" | "file_operation_name_invalid" => {
            "目标名称不符合 Windows 文件命名规则"
        }
        _ => "目标文件夹不可用或无法写入",
    };
    // Native details remain in the durable explanation without allowing an unbounded item record.
    format!(
        "{message}：{}",
        error.message.chars().take(1000).collect::<String>()
    )
}

fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|value| i64::try_from(value.as_millis()).ok())
        .unwrap_or(0)
}
