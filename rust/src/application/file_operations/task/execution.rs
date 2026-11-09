use std::time::{SystemTime, UNIX_EPOCH};

use crate::domain::ScanError;
use crate::domain::file_operation_journal::{
    FileOperationEvent, FileOperationPhase as Phase, FileOperationRecord,
};
use crate::domain::file_operation_task::{
    FileOperationTaskKind as Kind, FileOperationTaskPhase, FileOperationTaskRequest,
};
use crate::ports::file_operation_catalog::FileOperationCatalog;
use crate::ports::file_operation_output::FileOperationOutputReader;
use crate::ports::file_operations::NativeFileOperations;

use super::super::{FileOperationExecutor, publish_file_operation};
use super::TaskState;

pub(super) fn run<C, N>(
    request: &FileOperationTaskRequest,
    catalog: &mut C,
    native: &N,
    state: &TaskState,
) -> Result<FileOperationTaskPhase, ScanError>
where
    C: FileOperationCatalog,
    N: NativeFileOperations + FileOperationOutputReader,
{
    state.update(|progress| progress.phase = FileOperationTaskPhase::Running);
    for (index, id) in request.operation_ids.iter().copied().enumerate() {
        if state.control.is_cancelled() {
            if request.kind == Kind::Execute {
                cancel_unstarted(catalog, &request.operation_ids[index..], state)?;
            }
            return Ok(FileOperationTaskPhase::Cancelled);
        }
        let record = catalog.load(id)?;
        state.control.publish_copy_progress(0, 0);
        state.update(|progress| {
            progress.current_operation_id = Some(id);
            progress.current_path = Some(record.authorization.request.source.state.absolute_path);
        });
        let mut executor = FileOperationExecutor::new(catalog, native, &state.control, &now);
        let record = match request.kind {
            Kind::Execute => executor.execute(id)?,
            Kind::Inspect => executor.reconcile(id)?,
            Kind::ContinueCopiedMove => executor.resume_copied_move(id)?,
        };
        let record = if record.checkpoint.phase == Phase::NativeCompleted {
            publish_file_operation(catalog, native, id, now())?
        } else {
            record
        };
        record_progress(state, &record);
        if record.checkpoint.phase == Phase::Uncertain {
            return Err(ScanError::new(
                "file_operation_inspection_required",
                record
                    .checkpoint
                    .issue
                    .as_ref()
                    .map_or("The native result requires inspection", |issue| {
                        issue.message.as_str()
                    }),
            ));
        }
    }
    Ok(FileOperationTaskPhase::Finished)
}

fn cancel_unstarted(
    catalog: &mut impl FileOperationCatalog,
    ids: &[u64],
    state: &TaskState,
) -> Result<(), ScanError> {
    for id in ids {
        let record = catalog.load(*id)?;
        if matches!(
            record.checkpoint.phase,
            Phase::Prepared | Phase::NameOccupied
        ) {
            let record = catalog.advance(
                *id,
                record.checkpoint.version,
                FileOperationEvent::Cancelled,
                now(),
            )?;
            record_progress(state, &record);
        }
    }
    Ok(())
}

fn record_progress(state: &TaskState, record: &FileOperationRecord) {
    state.update(|progress| {
        progress.processed_items += 1;
        match record.checkpoint.phase {
            Phase::Published => progress.published_items += 1,
            Phase::Failed | Phase::Uncertain => progress.failed_items += 1,
            Phase::Cancelled => progress.cancelled_items += 1,
            _ => {}
        }
    });
}

fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|value| i64::try_from(value.as_millis()).ok())
        .unwrap_or(0)
}
