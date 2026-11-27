use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU64, Ordering};

mod production;

use crate::domain::application_update::{
    ApplicationDownloadPhase, ApplicationDownloadProgress, ApplicationRelease,
    ApplicationUpdateChannel, ApplicationUpdateDelivery, ApplicationUpdateError,
    ApplicationUpdateFailureCode,
};
use crate::ports::application_updates::{
    ApplicationDownloadObserver, ApplicationUpdatePackage, ApplicationUpdateSource,
};

pub(crate) struct ApplicationUpdates {
    source: Box<dyn ApplicationUpdateSource>,
}

impl ApplicationUpdates {
    pub fn take_result(
        &self,
    ) -> Result<
        Option<crate::domain::application_update::ApplicationUpdateResult>,
        ApplicationUpdateError,
    > {
        self.source.take_result()
    }
    pub fn open(version: String, build: u64) -> Result<Self, ApplicationUpdateError> {
        Ok(Self {
            source: production::open(version, build)?,
        })
    }
    pub fn delivery(&self) -> ApplicationUpdateDelivery {
        self.source.delivery()
    }
    pub fn open_store(&self) -> Result<(), ApplicationUpdateError> {
        self.source.open_store()
    }
    pub fn check(
        &self,
        channel: ApplicationUpdateChannel,
    ) -> Result<Option<ApplicationUpdateDownload>, ApplicationUpdateError> {
        self.source
            .check(channel)
            .map(|package| package.map(ApplicationUpdateDownload::new))
    }
}

pub(crate) struct ApplicationUpdateDownload {
    package: Box<dyn ApplicationUpdatePackage>,
    operation: Mutex<()>,
    progress: DownloadControl,
    verified: AtomicBool,
}

impl ApplicationUpdateDownload {
    pub(crate) fn new(package: Box<dyn ApplicationUpdatePackage>) -> Self {
        Self {
            package,
            operation: Mutex::new(()),
            progress: DownloadControl::default(),
            verified: AtomicBool::new(false),
        }
    }

    pub(crate) fn release(&self) -> ApplicationRelease {
        self.package.release()
    }

    pub(crate) fn progress(&self) -> ApplicationDownloadProgress {
        let phase = match self.progress.phase.load(Ordering::Acquire) {
            1 => ApplicationDownloadPhase::Verifying,
            2 => ApplicationDownloadPhase::Ready,
            3 => ApplicationDownloadPhase::Cancelled,
            _ => ApplicationDownloadPhase::Downloading,
        };
        ApplicationDownloadProgress {
            phase,
            received_bytes: self.progress.received.load(Ordering::Acquire),
            total_bytes: self.release().download_bytes,
        }
    }

    pub(crate) fn download(&self) -> Result<(), ApplicationUpdateError> {
        let _operation = self.operation.try_lock().map_err(busy)?;
        let result = self.package.download(&self.progress);
        if self.progress.cancelled() {
            self.progress.phase.store(3, Ordering::Release);
            return Ok(());
        }
        result?;
        self.verified.store(true, Ordering::Release);
        self.progress.phase.store(2, Ordering::Release);
        Ok(())
    }

    pub(crate) fn prepare_download(&self) -> Result<(), ApplicationUpdateError> {
        let _operation = self.operation.try_lock().map_err(busy)?;
        self.verified.store(false, Ordering::Release);
        self.progress.cancelled.store(false, Ordering::Release);
        self.progress.phase.store(0, Ordering::Release);
        self.progress.received.store(0, Ordering::Release);
        Ok(())
    }

    pub(crate) fn verify(&self) -> Result<(), ApplicationUpdateError> {
        let _operation = self.operation.try_lock().map_err(busy)?;
        self.verify_package()
    }

    fn verify_package(&self) -> Result<(), ApplicationUpdateError> {
        self.verified.store(false, Ordering::Release);
        self.progress.verifying();
        self.package.verify(&self.progress)?;
        if self.progress.cancelled() {
            return Err(ApplicationUpdateError::new(
                ApplicationUpdateFailureCode::Cancelled,
                "Application update verification was cancelled",
            ));
        }
        self.verified.store(true, Ordering::Release);
        self.progress.phase.store(2, Ordering::Release);
        Ok(())
    }

    pub(crate) fn cancel(&self) {
        self.progress.cancelled.store(true, Ordering::Release);
    }

    pub(crate) fn apply_after_exit(&self) -> Result<(), ApplicationUpdateError> {
        let _operation = self.operation.try_lock().map_err(busy)?;
        if !self.verified.load(Ordering::Acquire) || self.progress.cancelled() {
            return Err(ApplicationUpdateError::new(
                ApplicationUpdateFailureCode::DamagedDownload,
                "No verified application update is ready",
            ));
        }
        self.package.apply_after_exit()
    }

    pub(crate) fn prepare_handoff(&self) -> Result<(), ApplicationUpdateError> {
        let _operation = self.operation.try_lock().map_err(busy)?;
        if !self.verified.load(Ordering::Acquire) || self.progress.cancelled() {
            return Err(ApplicationUpdateError::new(
                ApplicationUpdateFailureCode::DamagedDownload,
                "No verified update can be prepared",
            ));
        }
        self.package.prepare_handoff()
    }
}

impl Drop for ApplicationUpdateDownload {
    fn drop(&mut self) {
        self.progress.cancelled.store(true, Ordering::Release);
    }
}

#[derive(Default)]
struct DownloadControl {
    cancelled: AtomicBool,
    received: AtomicU64,
    phase: AtomicU8,
}

impl ApplicationDownloadObserver for DownloadControl {
    fn cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }

    fn received(&self, bytes: u64) {
        self.received.store(bytes, Ordering::Release);
    }

    fn verifying(&self) {
        self.phase.store(1, Ordering::Release);
    }
}

fn busy<T>(_error: std::sync::TryLockError<T>) -> ApplicationUpdateError {
    ApplicationUpdateError::new(
        ApplicationUpdateFailureCode::Unavailable,
        "An operation is already using this application update",
    )
}

#[cfg(test)]
mod tests;
