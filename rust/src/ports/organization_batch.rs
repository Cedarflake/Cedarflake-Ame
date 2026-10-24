use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::domain::ScanError;
use crate::domain::organization::OrganizationWriteReceipt;
use crate::domain::organization_batch::{
    OrganizationBatchAction, OrganizationBatchPhase, OrganizationBatchProgress,
    OrganizationBatchSummary,
};

pub(crate) trait OrganizationBatchRepository: Send {
    fn append_ids(&mut self, asset_ids: &[String]) -> Result<(), ScanError>;
    fn prepare(&mut self) -> Result<OrganizationBatchSummary, ScanError>;
    fn refresh(&mut self) -> Result<OrganizationBatchSummary, ScanError>;
    fn apply(
        &mut self,
        operation_id: &str,
        action: &OrganizationBatchAction,
    ) -> Result<OrganizationWriteReceipt, ScanError>;
}

pub(crate) struct OrganizationBatchControl {
    pub cancelled: AtomicBool,
    progress: Mutex<OrganizationBatchProgress>,
}

impl OrganizationBatchControl {
    pub fn new() -> Self {
        Self {
            cancelled: AtomicBool::new(false),
            progress: Mutex::new(OrganizationBatchProgress {
                phase: OrganizationBatchPhase::Collecting,
                completed_groups: 0,
                total_groups: 0,
            }),
        }
    }

    pub fn check(&self) -> Result<(), ScanError> {
        if self.cancelled.load(Ordering::Acquire) {
            return Err(ScanError::new(
                "organization_selection_cancelled",
                "The organization selection was cancelled",
            ));
        }
        Ok(())
    }

    pub fn report(&self, phase: OrganizationBatchPhase, completed_groups: u64, total_groups: u64) {
        *self
            .progress
            .lock()
            .unwrap_or_else(|error| error.into_inner()) = OrganizationBatchProgress {
            phase,
            completed_groups,
            total_groups,
        };
    }

    pub fn snapshot(&self) -> OrganizationBatchProgress {
        self.progress
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clone()
    }
}
