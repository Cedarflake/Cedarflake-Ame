use crate::domain::ScanError;
use crate::domain::file_operation_exports::FileExportReport;
use crate::domain::file_operation_task::{
    FileOperationTaskKind, FileOperationTaskPhase, FileOperationTaskRequest,
    FileOperationTaskTarget,
};
use crate::ports::file_operation_catalog::FileOperationCatalog;
use crate::ports::file_operation_exports::NativeFileExports;
use crate::ports::file_operation_output::FileOperationOutputReader;
use crate::ports::file_operations::NativeFileOperations;

use super::TaskState;

pub(super) fn run<C, N>(
    request: &FileOperationTaskRequest,
    catalog: &mut C,
    native: &N,
    state: &TaskState,
) -> Result<FileOperationTaskPhase, ScanError>
where
    C: FileOperationCatalog,
    N: NativeFileOperations + FileOperationOutputReader + NativeFileExports,
{
    let batch_id = match &request.target {
        FileOperationTaskTarget::Batch { batch_id, .. }
        | FileOperationTaskTarget::Export(batch_id) => Some(batch_id),
        FileOperationTaskTarget::Items(_) => None,
    };
    if let Some(id) = batch_id
        && let Some(record) = catalog.load_export(id)?
    {
        if let FileOperationTaskTarget::Batch { expected_items, .. } = &request.target
            && expected_items.checked_add(record.skipped_files) != Some(record.batch.selected_files)
        {
            return Err(ScanError::new(
                "file_operation_batch_changed",
                "The export no longer accounts for its full selected scope",
            ));
        }
        state.update(|progress| {
            progress.skipped_items = record.skipped_files;
            progress.total_items += record.skipped_files;
        });
    } else if matches!(
        &request.target,
        FileOperationTaskTarget::Export(_)
            | FileOperationTaskTarget::Batch {
                expected_items: 0,
                ..
            }
    ) {
        return Err(ScanError::new(
            "file_operation_batch_changed",
            "The export authorization is missing",
        ));
    }
    let result = if request.kind == FileOperationTaskKind::WriteReport {
        Ok(FileOperationTaskPhase::Finished)
    } else {
        super::execution::run(request, catalog, native, state)
    };
    if let Some(id) = batch_id {
        let report = match super::super::exports::generate(catalog, native, id) {
            Ok(report) => report,
            Err(error) => {
                return result.and(Err(ScanError::new(
                    "file_export_report_failed",
                    format!("Could not record the result report: {}", error.message),
                )));
            }
        };
        let failure = report.as_ref().and_then(|record| match &record.report {
            FileExportReport::Failed(error) => Some(error.clone()),
            _ => None,
        });
        state.update(|progress| progress.export = report);
        if let Some(error) = failure {
            return result.and(Err(error));
        }
    }
    result
}
