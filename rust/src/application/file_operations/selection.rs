use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};

use crate::domain::ScanError;
use crate::domain::file_operation_selection::{
    FileOperationCommand, FileOperationPreparationProgress, FileOperationSelectionScope,
    FileOperationSelectionSummary,
};
use crate::domain::file_operation_task::{
    FileOperationTaskKind, FileOperationTaskRequest, FileOperationTaskTarget,
};
use crate::domain::organization::OrganizationSelection;
use crate::ports::file_operation_selection::FileOperationSelectionRepository;

use super::{FileOperationTask, production};

mod authorization;

static OPEN_SELECTIONS: AtomicUsize = AtomicUsize::new(0);

struct SelectionPermit;

impl SelectionPermit {
    fn acquire() -> Result<Self, ScanError> {
        OPEN_SELECTIONS
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |count| {
                (count < 2).then_some(count + 1)
            })
            .map(|_| Self)
            .map_err(|_| {
                ScanError::new(
                    "file_operation_selection_busy",
                    "Other physical file selections are still open",
                )
            })
    }
}

impl Drop for SelectionPermit {
    fn drop(&mut self) {
        OPEN_SELECTIONS.fetch_sub(1, Ordering::AcqRel);
    }
}

struct SelectedFiles {
    repository: Box<dyn FileOperationSelectionRepository>,
    submitted: bool,
    _permit: SelectionPermit,
}

pub(crate) struct FileOperationSelectionSession {
    selected: Mutex<Option<SelectedFiles>>,
    cancelled: Arc<AtomicBool>,
    progress: Mutex<FileOperationPreparationProgress>,
    catalog_path: std::path::PathBuf,
}

impl FileOperationSelectionSession {
    pub(crate) fn open(scope: FileOperationSelectionScope) -> Result<Self, ScanError> {
        Self::for_catalog(crate::application::storage_paths()?.catalog_path, scope)
    }

    pub(super) fn for_catalog(
        catalog_path: std::path::PathBuf,
        scope: FileOperationSelectionScope,
    ) -> Result<Self, ScanError> {
        let permit = SelectionPermit::acquire()?;
        let cancelled = Arc::new(AtomicBool::new(false));
        let repository = production::open_selection(&catalog_path, scope, Arc::clone(&cancelled))?;
        Ok(Self {
            selected: Mutex::new(Some(SelectedFiles {
                repository,
                submitted: false,
                _permit: permit,
            })),
            cancelled,
            progress: Mutex::new(FileOperationPreparationProgress {
                prepared_files: 0,
                total_files: 0,
                current_path: None,
            }),
            catalog_path,
        })
    }

    fn with_selection<T>(
        &self,
        action: impl FnOnce(&mut SelectedFiles) -> Result<T, ScanError>,
    ) -> Result<T, ScanError> {
        self.check()?;
        let mut selected = self.selected.lock().map_err(|_| closed())?;
        self.check()?;
        let selected = selected.as_mut().ok_or_else(closed)?;
        if selected.submitted {
            return Err(closed());
        }
        action(selected)
    }

    pub(crate) fn append_ids(&self, ids: Vec<String>) -> Result<(), ScanError> {
        self.with_selection(|selected| selected.repository.append_ids(&ids))
    }

    pub(crate) fn append_locations(
        &self,
        locations: Vec<OrganizationSelection>,
    ) -> Result<(), ScanError> {
        self.with_selection(|selected| selected.repository.append_locations(&locations))
    }

    pub(crate) fn prepare(&self) -> Result<FileOperationSelectionSummary, ScanError> {
        self.with_selection(|selected| {
            let summary = selected.repository.prepare()?;
            self.progress
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .total_files = summary.file_count;
            Ok(summary)
        })
    }

    pub(crate) fn execute(
        &self,
        batch_id: String,
        command: FileOperationCommand,
        owner_window: Option<u64>,
    ) -> Result<FileOperationTask, ScanError> {
        self.with_selection(|selected| {
            let summary = selected.repository.prepare()?;
            let mut journal = production::open_journal(&self.catalog_path)?;
            let native = production::source_inspector()?;
            authorization::record(
                selected.repository.as_ref(),
                &mut journal,
                native.as_ref(),
                &batch_id,
                &command,
                &summary,
                self,
            )?;
            let result = (|| {
                self.check()?;
                super::start_file_operation_task(
                    self.catalog_path.clone(),
                    FileOperationTaskRequest {
                        kind: FileOperationTaskKind::Execute,
                        target: FileOperationTaskTarget::Batch {
                            batch_id: batch_id.clone(),
                            expected_items: summary.file_count,
                        },
                    },
                    owner_window,
                )
            })();
            match result {
                Ok(task) => {
                    selected.submitted = true;
                    if self.cancelled.load(Ordering::Acquire) {
                        task.cancel();
                    }
                    Ok(task)
                }
                Err(error) => {
                    authorization::retire(&mut journal, &batch_id).map_err(|retirement| {
                        ScanError::new(
                            "file_operation_preparation_retirement_failed",
                            format!(
                                "{}; unable to retire unstarted records: {}",
                                error.message, retirement.message
                            ),
                        )
                    })?;
                    Err(error)
                }
            }
        })
    }

    pub(crate) fn progress(&self) -> FileOperationPreparationProgress {
        self.progress
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clone()
    }

    pub(crate) fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
    }

    pub(crate) fn close(&self) -> Result<(), ScanError> {
        self.cancel();
        let selected = self.selected.lock().map_err(|_| closed())?.take();
        drop(selected);
        Ok(())
    }

    fn check(&self) -> Result<(), ScanError> {
        if self.cancelled.load(Ordering::Acquire) {
            Err(closed())
        } else {
            Ok(())
        }
    }
}

impl Drop for FileOperationSelectionSession {
    fn drop(&mut self) {
        let _ = self.close();
    }
}

fn closed() -> ScanError {
    ScanError::new(
        "file_operation_selection_closed",
        "The physical file selection is no longer active",
    )
}
