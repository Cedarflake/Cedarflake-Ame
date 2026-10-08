use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use crate::domain::ScanError;

use super::super::SqliteWritePreemptCallback;

#[derive(Clone)]
pub(super) struct LocationWriteAttempt {
    preempted: Arc<AtomicBool>,
    active: Arc<Mutex<bool>>,
}

impl LocationWriteAttempt {
    pub(super) fn new() -> Self {
        Self {
            preempted: Arc::new(AtomicBool::new(false)),
            active: Arc::new(Mutex::new(true)),
        }
    }

    pub(super) fn callback(
        &self,
        interrupt: impl Fn() + Send + Sync + 'static,
    ) -> SqliteWritePreemptCallback {
        let attempt = self.clone();
        Arc::new(move || {
            let active = attempt
                .active
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            if *active {
                attempt.preempted.store(true, Ordering::Release);
                interrupt();
            }
        })
    }

    pub(super) fn is_interrupted(&self) -> bool {
        *self
            .active
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            && self.preempted.load(Ordering::Acquire)
    }

    pub(super) fn ensure_running(&self) -> Result<(), ScanError> {
        if self.preempted.load(Ordering::Acquire) {
            Err(ScanError::new(
                "root_location_recovery_preempted",
                "A higher-priority catalog operation preempted directory recovery",
            ))
        } else {
            Ok(())
        }
    }

    pub(super) fn classify(&self, error: ScanError) -> ScanError {
        if error.code == "catalog_database_interrupted"
            && let Err(preempted) = self.ensure_running()
        {
            return preempted;
        }
        error
    }

    pub(super) fn retire(&self) {
        *self
            .active
            .lock()
            .unwrap_or_else(|error| error.into_inner()) = false;
    }

    pub(super) fn retirement_guard(&self) -> AttemptRetirement {
        AttemptRetirement(self.clone())
    }
}

pub(super) struct AttemptRetirement(LocationWriteAttempt);

impl Drop for AttemptRetirement {
    fn drop(&mut self) {
        self.0.retire();
    }
}

#[cfg(test)]
mod tests;
