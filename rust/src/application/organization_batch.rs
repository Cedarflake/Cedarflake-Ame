use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use crate::adapters::SqliteOrganizationBatch;
use crate::domain::organization::OrganizationWriteReceipt;
use crate::domain::organization_batch::{
    OrganizationBatchAction, OrganizationBatchPhase, OrganizationBatchProgress,
    OrganizationBatchSummary,
};
use crate::domain::{GalleryQuery, ScanError};
use crate::ports::organization_batch::{OrganizationBatchControl, OrganizationBatchRepository};

const MAX_OPEN_SELECTIONS: usize = 2;
static OPEN_SELECTIONS: AtomicUsize = AtomicUsize::new(0);

struct SelectionPermit;

impl SelectionPermit {
    fn acquire() -> Result<Self, ScanError> {
        OPEN_SELECTIONS
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |count| {
                (count < MAX_OPEN_SELECTIONS).then_some(count + 1)
            })
            .map_err(|_| {
                ScanError::new(
                    "organization_selection_busy",
                    "Other organization selections are still active",
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
    repository: Box<dyn OrganizationBatchRepository>,
    _permit: SelectionPermit,
}

pub(crate) struct OrganizationBatchSession {
    inner: Mutex<Option<ActiveSelection>>,
    control: Arc<OrganizationBatchControl>,
}

impl OrganizationBatchSession {
    pub(crate) fn open(
        query: GalleryQuery,
        revision: u64,
        all_matching: bool,
    ) -> Result<Self, ScanError> {
        let permit = SelectionPermit::acquire()?;
        let control = Arc::new(OrganizationBatchControl::new());
        let repository = SqliteOrganizationBatch::new(
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
        operation: impl FnOnce(&mut dyn OrganizationBatchRepository) -> Result<T, ScanError>,
    ) -> Result<T, ScanError> {
        self.control.check()?;
        let mut inner = self.inner.lock().map_err(|_| closed())?;
        self.control.check()?;
        let active = inner.as_mut().ok_or_else(closed)?;
        operation(active.repository.as_mut())
    }

    pub(crate) fn append_ids(&self, asset_ids: Vec<String>) -> Result<(), ScanError> {
        self.with_repository(|repository| repository.append_ids(&asset_ids))
    }

    pub(crate) fn prepare(&self) -> Result<OrganizationBatchSummary, ScanError> {
        self.with_repository(|repository| repository.prepare())
    }

    pub(crate) fn apply(
        &self,
        operation_id: String,
        action: OrganizationBatchAction,
    ) -> Result<OrganizationWriteReceipt, ScanError> {
        self.with_repository(|repository| repository.apply(&operation_id, &action))
    }

    pub(crate) fn progress(&self) -> OrganizationBatchProgress {
        self.control.snapshot()
    }

    pub(crate) fn close(&self) -> Result<(), ScanError> {
        self.control.cancelled.store(true, Ordering::Release);
        let active = self.inner.lock().map_err(|_| closed())?.take();
        drop(active);
        let progress = self.control.snapshot();
        if progress.phase != OrganizationBatchPhase::Completed {
            self.control.report(
                OrganizationBatchPhase::Cancelled,
                progress.completed_groups,
                progress.total_groups,
            );
        }
        Ok(())
    }
}

fn closed() -> ScanError {
    ScanError::new(
        "organization_selection_closed",
        "The organization selection is no longer available",
    )
}

#[cfg(test)]
mod tests;
