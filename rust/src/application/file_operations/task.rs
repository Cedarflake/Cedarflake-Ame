use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

use crate::domain::ScanError;
use crate::domain::file_operation_task::{
    FileOperationTaskPhase as TaskPhase, FileOperationTaskProgress, FileOperationTaskRequest,
    FileOperationTaskTarget,
};
use crate::ports::file_operation_catalog::FileOperationCatalog;
use crate::ports::file_operation_exports::NativeFileExports;
use crate::ports::file_operation_output::FileOperationOutputReader;
use crate::ports::file_operations::{FileOperationControl, NativeFileOperations};

mod execution;
mod export;

static ACTIVE_LANE: AtomicBool = AtomicBool::new(false);

struct LanePermit;

impl LanePermit {
    fn acquire() -> Result<Self, ScanError> {
        ACTIVE_LANE
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map(|_| Self)
            .map_err(|_| {
                ScanError::new(
                    "file_operation_busy",
                    "Another file-operation task is still running",
                )
            })
    }
}

impl Drop for LanePermit {
    fn drop(&mut self) {
        ACTIVE_LANE.store(false, Ordering::Release);
    }
}

struct TaskState {
    control: FileOperationControl,
    progress: Mutex<FileOperationTaskProgress>,
}

impl TaskState {
    fn update(&self, update: impl FnOnce(&mut FileOperationTaskProgress)) {
        update(
            &mut self
                .progress
                .lock()
                .unwrap_or_else(|error| error.into_inner()),
        );
    }
}

pub struct FileOperationTask {
    state: Arc<TaskState>,
    worker: Mutex<Option<JoinHandle<()>>>,
}

impl FileOperationTask {
    pub(crate) fn spawn<C, N>(
        request: FileOperationTaskRequest,
        open_catalog: impl FnOnce() -> Result<C, ScanError> + Send + 'static,
        native: N,
    ) -> Result<Self, ScanError>
    where
        C: FileOperationCatalog + 'static,
        N: NativeFileOperations + FileOperationOutputReader + NativeFileExports + 'static,
    {
        request.validate()?;
        let permit = LanePermit::acquire()?;
        let state = Arc::new(TaskState {
            control: FileOperationControl::default(),
            progress: Mutex::new(FileOperationTaskProgress {
                kind: request.kind,
                phase: TaskPhase::Starting,
                total_items: match &request.target {
                    FileOperationTaskTarget::Items(ids) => ids.len() as u64,
                    FileOperationTaskTarget::Batch { expected_items, .. } => *expected_items,
                    FileOperationTaskTarget::Export(_) => 0,
                },
                processed_items: 0,
                published_items: 0,
                failed_items: 0,
                cancelled_items: 0,
                skipped_items: 0,
                export: None,
                current_operation_id: None,
                current_path: None,
                copied_bytes: 0,
                total_bytes: 0,
                failure: None,
            }),
        });
        let running = Arc::clone(&state);
        let worker = thread::Builder::new()
            .name("ame-file-operation-task".into())
            .spawn(move || {
                let _permit = permit;
                let result = catch_unwind(AssertUnwindSafe(|| {
                    let mut catalog = open_catalog()?;
                    export::run(&request, &mut catalog, &native, &running)
                }))
                .unwrap_or_else(|_| Err(worker_failed()));
                running.update(|progress| match result {
                    Ok(phase) => progress.phase = phase,
                    Err(error) => {
                        progress.phase = TaskPhase::NeedsAttention;
                        progress.failure = Some(error);
                    }
                });
            })
            .map_err(|error| ScanError::new("file_operation_start_failed", error.to_string()))?;
        Ok(Self {
            state,
            worker: Mutex::new(Some(worker)),
        })
    }

    pub fn snapshot(&self) -> FileOperationTaskProgress {
        let mut progress = self
            .state
            .progress
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clone();
        if self.state.control.is_cancelled()
            && matches!(progress.phase, TaskPhase::Starting | TaskPhase::Running)
        {
            progress.phase = TaskPhase::Stopping;
        }
        let (copied, total) = self.state.control.copy_progress();
        progress.copied_bytes = copied.min(total);
        progress.total_bytes = total;
        progress
    }

    pub fn cancel(&self) {
        self.state.control.cancel();
    }

    pub fn close(&self) -> Result<(), ScanError> {
        self.cancel();
        // Concurrent close callers wait for the same owner; progress and cancellation use separate state.
        let mut worker = self
            .worker
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if let Some(worker) = worker.take() {
            worker.join().map_err(|_| worker_failed())?;
        }
        Ok(())
    }
}

impl Drop for FileOperationTask {
    fn drop(&mut self) {
        let _ = self.close();
    }
}

fn worker_failed() -> ScanError {
    ScanError::new(
        "file_operation_worker_failed",
        "The file-operation worker stopped; inspect its durable outcome before retrying",
    )
}
