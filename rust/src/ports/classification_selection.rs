use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::domain::ScanError;
use crate::domain::classification::{ClassificationAction, ClassificationRecordId};
use crate::domain::classification_catalog::ClassificationWriteReceipt;
use crate::domain::classification_selection::{
    ClassificationSelectionPhase, ClassificationSelectionProgress, ClassificationSelectionSummary,
};

pub(crate) trait ClassificationSelectionRepository: Send {
    fn append_targets(
        &mut self,
        targets: &[crate::domain::organization::OrganizationSelection],
    ) -> Result<(), ScanError>;
    fn append_ids(&mut self, asset_ids: &[String]) -> Result<(), ScanError>;
    fn prepare(&mut self) -> Result<ClassificationSelectionSummary, ScanError>;
    fn apply(
        &mut self,
        id: &ClassificationRecordId,
        action: &ClassificationAction,
    ) -> Result<ClassificationWriteReceipt, ScanError>;
}

pub(crate) struct ClassificationSelectionControl {
    pub cancelled: AtomicBool,
    progress: Mutex<ClassificationSelectionProgress>,
}

impl ClassificationSelectionControl {
    pub fn new() -> Self {
        Self {
            cancelled: AtomicBool::new(false),
            progress: Mutex::new(ClassificationSelectionProgress {
                phase: ClassificationSelectionPhase::Collecting,
                completed_groups: 0,
                total_groups: 0,
            }),
        }
    }

    pub fn check(&self) -> Result<(), ScanError> {
        if self.cancelled.load(Ordering::Acquire) {
            return Err(ScanError::new(
                "classification_selection_cancelled",
                "The classification selection was cancelled",
            ));
        }
        Ok(())
    }

    pub fn report(
        &self,
        phase: ClassificationSelectionPhase,
        completed_groups: u64,
        total_groups: u64,
    ) {
        *self
            .progress
            .lock()
            .unwrap_or_else(|error| error.into_inner()) = ClassificationSelectionProgress {
            phase,
            completed_groups,
            total_groups,
        };
    }

    pub fn snapshot(&self) -> ClassificationSelectionProgress {
        self.progress
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clone()
    }
}
