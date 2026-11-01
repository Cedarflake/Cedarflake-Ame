use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use crate::adapters::SqliteClassificationSelection;
use crate::domain::GalleryQuery;
use crate::domain::ScanError;
use crate::domain::classification::{ClassificationAction, ClassificationRecordId};
use crate::domain::classification_catalog::ClassificationWriteReceipt;
use crate::domain::classification_selection::ClassificationItemState;
use crate::domain::classification_selection::{
    ClassificationSelectionPhase, ClassificationSelectionProgress, ClassificationSelectionSummary,
};
use crate::domain::organization::OrganizationSelection;
use crate::ports::classification::ClassificationRepository;
use crate::ports::classification_selection::{
    ClassificationSelectionControl, ClassificationSelectionRepository,
};

static OPEN_SELECTIONS: AtomicUsize = AtomicUsize::new(0);

pub(crate) fn load_items(
    selections: &[OrganizationSelection],
) -> Result<Vec<ClassificationItemState>, ScanError> {
    super::organization::open_catalog()?.classification_items(selections)
}

pub(crate) fn undo(
    operation_id: &ClassificationRecordId,
    original_id: &ClassificationRecordId,
) -> Result<ClassificationWriteReceipt, ScanError> {
    super::organization::open_catalog()?.undo_classification(operation_id, original_id)
}

struct SelectionPermit;

impl SelectionPermit {
    fn acquire() -> Result<Self, ScanError> {
        OPEN_SELECTIONS
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |count| {
                (count < 2).then_some(count + 1)
            })
            .map_err(|_| {
                ScanError::new(
                    "classification_selection_busy",
                    "Other classification selections are still active",
                )
            })?;
        Ok(Self)
    }
}

impl Drop for SelectionPermit {
    fn drop(&mut self) {
        OPEN_SELECTIONS.fetch_sub(1, Ordering::AcqRel);
    }
}

struct ActiveSelection {
    repository: Box<dyn ClassificationSelectionRepository>,
    _permit: SelectionPermit,
}

pub(crate) struct ClassificationSelectionSession {
    inner: Mutex<Option<ActiveSelection>>,
    control: Arc<ClassificationSelectionControl>,
}

impl ClassificationSelectionSession {
    pub(crate) fn open(
        query: GalleryQuery,
        revision: u64,
        all_matching: bool,
    ) -> Result<Self, ScanError> {
        let permit = SelectionPermit::acquire()?;
        let control = Arc::new(ClassificationSelectionControl::new());
        let repository = SqliteClassificationSelection::new(
            super::organization::open_catalog()?,
            query,
            revision,
            all_matching,
            Arc::clone(&control),
        )?;
        Ok(Self {
            inner: Mutex::new(Some(ActiveSelection {
                repository: Box::new(repository),
                _permit: permit,
            })),
            control,
        })
    }

    fn with_repository<T>(
        &self,
        operation: impl FnOnce(&mut dyn ClassificationSelectionRepository) -> Result<T, ScanError>,
    ) -> Result<T, ScanError> {
        self.control.check()?;
        let mut inner = self.inner.lock().map_err(|_| closed())?;
        self.control.check()?;
        operation(inner.as_mut().ok_or_else(closed)?.repository.as_mut())
    }

    pub(crate) fn append_ids(&self, ids: Vec<String>) -> Result<(), ScanError> {
        self.with_repository(|repository| repository.append_ids(&ids))
    }

    pub(crate) fn append_targets(
        &self,
        targets: Vec<OrganizationSelection>,
    ) -> Result<(), ScanError> {
        self.with_repository(|repository| repository.append_targets(&targets))
    }

    pub(crate) fn prepare(&self) -> Result<ClassificationSelectionSummary, ScanError> {
        self.with_repository(|repository| repository.prepare())
    }

    pub(crate) fn apply(
        &self,
        id: ClassificationRecordId,
        action: ClassificationAction,
    ) -> Result<ClassificationWriteReceipt, ScanError> {
        self.with_repository(|repository| repository.apply(&id, &action))
    }

    pub(crate) fn progress(&self) -> ClassificationSelectionProgress {
        self.control.snapshot()
    }

    pub(crate) fn close(&self) -> Result<(), ScanError> {
        self.control.cancelled.store(true, Ordering::Release);
        let active = self.inner.lock().map_err(|_| closed())?.take();
        drop(active);
        let progress = self.control.snapshot();
        if progress.phase != ClassificationSelectionPhase::Completed {
            self.control.report(
                ClassificationSelectionPhase::Cancelled,
                progress.completed_groups,
                progress.total_groups,
            );
        }
        Ok(())
    }
}

fn closed() -> ScanError {
    ScanError::new(
        "classification_selection_closed",
        "The classification selection is no longer available",
    )
}
