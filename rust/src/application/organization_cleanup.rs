use std::sync::atomic::{AtomicBool, Ordering};

use crate::domain::ScanError;
use crate::domain::organization_cleanup::{
    OrganizationCleanupInspection, OrganizationCleanupProgress, OrganizationCleanupReceipt,
    OrganizationCleanupRequest,
};
use crate::ports::organization_cleanup::OrganizationCleanupRepository;

#[derive(Default)]
pub(crate) struct CleanupTaskControl {
    started: AtomicBool,
    cancelled: AtomicBool,
}

impl CleanupTaskControl {
    pub(crate) fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
    }

    fn should_continue(&self) -> bool {
        !self.cancelled.load(Ordering::Acquire)
    }

    fn start(&self) -> Result<(), ScanError> {
        if self.started.swap(true, Ordering::AcqRel) {
            return Err(ScanError::new(
                "organization_cleanup_task_used",
                "This cleanup task already started",
            ));
        }
        if !self.should_continue() {
            return Err(ScanError::new(
                "organization_cleanup_cancelled",
                "Cleanup was cancelled",
            ));
        }
        Ok(())
    }
}

pub(crate) fn inspect(
    cutoff_unix_ms: Option<i64>,
    control: &CleanupTaskControl,
    progress: &mut dyn FnMut(OrganizationCleanupProgress) -> bool,
) -> Result<OrganizationCleanupInspection, ScanError> {
    control.start()?;
    super::organization::open_catalog()?
        .inspect_organization_cleanup(cutoff_unix_ms, &mut |update| {
            control.should_continue() && progress(update)
        })
}

pub(crate) fn apply(
    request: &OrganizationCleanupRequest,
    control: &CleanupTaskControl,
    progress: &mut dyn FnMut(OrganizationCleanupProgress) -> bool,
) -> Result<OrganizationCleanupReceipt, ScanError> {
    request.validate()?;
    control.start()?;
    super::organization::open_catalog()?.apply_organization_cleanup(request, &mut |update| {
        control.should_continue() && progress(update)
    })
}
