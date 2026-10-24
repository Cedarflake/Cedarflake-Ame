use std::sync::atomic::{AtomicBool, Ordering};

use crate::domain::ScanError;

static ARCHIVE_ACTIVE: AtomicBool = AtomicBool::new(false);

#[derive(Default)]
pub(crate) struct ArchiveTaskControl {
    started: AtomicBool,
    cancelled: AtomicBool,
}

impl ArchiveTaskControl {
    pub(crate) fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
    }

    pub(crate) fn should_continue(&self) -> bool {
        !self.cancelled.load(Ordering::Acquire)
    }

    pub(super) fn admit(&self) -> Result<ArchiveLease<'static>, ScanError> {
        self.admit_with(&ARCHIVE_ACTIVE)
    }

    fn admit_with<'a>(&self, active: &'a AtomicBool) -> Result<ArchiveLease<'a>, ScanError> {
        self.started
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| {
                ScanError::new(
                    "organization_archive_task_used",
                    "This backup task has already started",
                )
            })?;
        if !self.should_continue() {
            return Err(ScanError::new(
                "organization_archive_cancelled",
                "The backup task was cancelled",
            ));
        }
        active
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| {
                ScanError::new(
                    "organization_archive_busy",
                    "An organization backup operation is already running",
                )
            })?;
        Ok(ArchiveLease(active))
    }
}

pub(super) struct ArchiveLease<'a>(&'a AtomicBool);

impl Drop for ArchiveLease<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

#[cfg(test)]
mod tests;
