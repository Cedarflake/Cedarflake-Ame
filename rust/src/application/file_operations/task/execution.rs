use std::time::{SystemTime, UNIX_EPOCH};

use crate::domain::ScanError;
use crate::domain::file_operation_journal::{
    FILE_OPERATION_PAGE_SIZE, FileOperationPhase as Phase, FileOperationRecord,
};
use crate::domain::file_operation_task::{
    FileOperationTaskKind as Kind, FileOperationTaskPhase, FileOperationTaskRequest,
    FileOperationTaskTarget,
};
use crate::ports::file_operation_catalog::FileOperationCatalog;
use crate::ports::file_operation_output::FileOperationOutputReader;
use crate::ports::file_operations::NativeFileOperations;

use super::super::{FileOperationExecutor, publish_file_operation};
use super::TaskState;

enum ItemPages {
    Items(Option<Vec<u64>>),
    Batch {
        id: String,
        after: Option<u64>,
        remaining: u64,
    },
}

impl ItemPages {
    fn open(
        target: &FileOperationTaskTarget,
        catalog: &impl FileOperationCatalog,
    ) -> Result<Self, ScanError> {
        match target {
            FileOperationTaskTarget::Export(_) => Err(ScanError::new(
                "file_operation_task_invalid",
                "A report task cannot execute media operations",
            )),
            FileOperationTaskTarget::Items(ids) => Ok(Self::Items(Some(ids.clone()))),
            FileOperationTaskTarget::Batch {
                batch_id,
                expected_items,
            } => {
                if catalog.batch_count(batch_id)? != *expected_items {
                    return Err(ScanError::new(
                        "file_operation_batch_changed",
                        "The recorded batch no longer matches its authorized item count",
                    ));
                }
                Ok(Self::Batch {
                    id: batch_id.clone(),
                    after: None,
                    remaining: *expected_items,
                })
            }
        }
    }

    fn next(
        &mut self,
        catalog: &impl FileOperationCatalog,
    ) -> Result<Vec<FileOperationRecord>, ScanError> {
        match self {
            Self::Items(ids) => ids
                .take()
                .unwrap_or_default()
                .into_iter()
                .map(|id| catalog.load(id))
                .collect(),
            Self::Batch {
                id,
                after,
                remaining,
            } => {
                if *remaining == 0 {
                    return Ok(Vec::new());
                }
                let limit = (*remaining).min(FILE_OPERATION_PAGE_SIZE as u64) as usize;
                let page = catalog.batch_page(id, *after, limit)?;
                if page.is_empty() {
                    return Err(ScanError::new(
                        "file_operation_batch_changed",
                        "The recorded batch ended before its authorized item count",
                    ));
                }
                *remaining -= page.len() as u64;
                if let Some(last) = page.last() {
                    *after = Some(last.authorization.ordinal);
                }
                Ok(page)
            }
        }
    }
}

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
    let mut pages = ItemPages::open(&request.target, catalog)?;
    state.update(|progress| progress.phase = FileOperationTaskPhase::Running);
    loop {
        let page = pages.next(catalog)?;
        if page.is_empty() {
            break;
        }
        let mut remaining = page.into_iter();
        while let Some(record) = remaining.next() {
            if state.control.is_cancelled() {
                if request.kind != Kind::Execute {
                    return Ok(FileOperationTaskPhase::Cancelled);
                }
                let ids = std::iter::once(record.id)
                    .chain(remaining.map(|record| record.id))
                    .collect::<Vec<_>>();
                for current in catalog.cancel_unstarted(&ids, now())? {
                    record_progress(state, &current);
                }
                break;
            }
            state.control.publish_copy_progress(0, 0);
            state.update(|progress| {
                progress.current_operation_id = Some(record.id);
                progress.current_path = Some(
                    record
                        .authorization
                        .request
                        .source
                        .state
                        .absolute_path
                        .clone(),
                );
            });
            let mut executor = FileOperationExecutor::new(catalog, native, &state.control, &now);
            let record = match request.kind {
                Kind::Execute
                    if matches!(
                        record.checkpoint.phase,
                        Phase::Published | Phase::Failed | Phase::Cancelled
                    ) =>
                {
                    record
                }
                Kind::Execute => executor.execute(record.id)?,
                Kind::Inspect => executor.reconcile(record.id)?,
                Kind::ContinueCopiedMove => executor.resume_copied_move(record.id)?,
                Kind::WriteReport => {
                    return Err(ScanError::new(
                        "file_operation_task_invalid",
                        "Report generation cannot execute a file action",
                    ));
                }
            };
            let record = if record.checkpoint.phase == Phase::NativeCompleted {
                publish_file_operation(catalog, native, record.id, now())?
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
    }
    let cancelled = state.control.is_cancelled()
        && state
            .progress
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .cancelled_items
            > 0;
    Ok(if cancelled {
        FileOperationTaskPhase::Cancelled
    } else {
        FileOperationTaskPhase::Finished
    })
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
