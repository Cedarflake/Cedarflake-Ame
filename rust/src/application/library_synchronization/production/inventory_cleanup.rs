use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use crate::adapters::SqliteCatalogSession;
use crate::application::metadata_inventory::cleanup::cleanup_terminal_inventory_batch;
use crate::domain::{LibraryChangeLane, MetadataInventoryCleanupReport, ScanError};
use crate::ports::InventoryCleanupControl;

use super::{finish_runtime_task_until, now_unix_ms};

const IDLE_CHECK_INTERVAL: Duration = Duration::from_secs(5);
const RETRY_INITIAL_DELAY: Duration = Duration::from_millis(250);
const RETRY_MAX_DELAY: Duration = Duration::from_secs(30);

#[derive(Default)]
pub(super) struct InventoryCleanupOwner {
    task: Option<InventoryCleanupTask>,
    next_check: Option<Instant>,
    retry_delay: Option<Duration>,
    stopping: bool,
}

struct InventoryCleanupTask {
    cancelled: Arc<AtomicBool>,
    receiver: Receiver<Result<InventoryCleanupOutcome, ScanError>>,
    worker: Option<JoinHandle<()>>,
}

enum InventoryCleanupOutcome {
    Completed(MetadataInventoryCleanupReport),
    Cancelled,
}

impl InventoryCleanupOwner {
    pub(super) fn poll(
        &mut self,
        session: Arc<SqliteCatalogSession>,
        cancelled: Arc<AtomicBool>,
    ) -> Result<(), ScanError> {
        self.poll_with(Instant::now(), cancelled, move |cancelled| {
            if cancelled.load(Ordering::Acquire) {
                return Ok(InventoryCleanupOutcome::Cancelled);
            }
            let mut catalog = match session.open_in_lane(LibraryChangeLane::Recovery) {
                Ok(catalog) => catalog,
                Err(error) if error.code == "catalog_validated_session_stale" => {
                    crate::application::catalog_session::refresh_stale_catalog_session(
                        session.path(),
                        &session,
                    )?
                    .open_in_lane(LibraryChangeLane::Recovery)?
                }
                Err(error) => return Err(error),
            };
            if cancelled.load(Ordering::Acquire) {
                return Ok(InventoryCleanupOutcome::Cancelled);
            }
            match cleanup_terminal_inventory_batch(
                &mut catalog,
                now_unix_ms()?,
                InventoryCleanupControl::new(Arc::clone(cancelled)),
            ) {
                Ok(report) => Ok(InventoryCleanupOutcome::Completed(report)),
                Err(error) if error.code == "metadata_inventory_cleanup_cancelled" => {
                    Ok(InventoryCleanupOutcome::Cancelled)
                }
                Err(error) => Err(error),
            }
        })
    }

    fn poll_with(
        &mut self,
        now: Instant,
        cancelled: Arc<AtomicBool>,
        work: impl FnOnce(&Arc<AtomicBool>) -> Result<InventoryCleanupOutcome, ScanError>
        + Send
        + 'static,
    ) -> Result<(), ScanError> {
        self.reap_finished(now)?;
        if self.stopping
            || cancelled.load(Ordering::Acquire)
            || self.task.is_some()
            || self.next_check.is_some_and(|deadline| now < deadline)
        {
            return Ok(());
        }
        let worker_cancelled = Arc::clone(&cancelled);
        let (sender, receiver) = mpsc::sync_channel(1);
        let worker = match thread::Builder::new()
            .name("ame-inventory-cleanup".to_owned())
            .spawn(move || {
                let result = catch_unwind(AssertUnwindSafe(|| work(&worker_cancelled)))
                    .unwrap_or_else(|_| Err(worker_panic()));
                let _ = sender.send(result);
            }) {
            Ok(worker) => worker,
            Err(error) => {
                self.defer_retry(now);
                return Err(ScanError::new(
                    "metadata_inventory_cleanup_worker_unavailable",
                    format!("Could not start bounded inventory cleanup: {error}"),
                ));
            }
        };
        self.task = Some(InventoryCleanupTask {
            cancelled,
            receiver,
            worker: Some(worker),
        });
        Ok(())
    }

    fn reap_finished(&mut self, now: Instant) -> Result<(), ScanError> {
        let Some(task) = self.task.as_mut() else {
            return Ok(());
        };
        if task
            .worker
            .as_ref()
            .is_some_and(|worker| !worker.is_finished())
        {
            return Ok(());
        }
        let joined = task.worker.take().map(JoinHandle::join);
        let result = if joined.is_some_and(|result| result.is_err()) {
            Err(worker_panic())
        } else {
            task.receiver.try_recv().unwrap_or_else(|_| {
                Err(ScanError::new(
                    "metadata_inventory_cleanup_worker_disconnected",
                    "The bounded inventory cleanup worker stopped without its result",
                ))
            })
        };
        self.task = None;
        match result {
            Ok(InventoryCleanupOutcome::Completed(report)) => {
                self.retry_delay = None;
                self.next_check = (!report.has_more).then_some(now + IDLE_CHECK_INTERVAL);
                Ok(())
            }
            Ok(InventoryCleanupOutcome::Cancelled) => Ok(()),
            Err(error) => {
                self.defer_retry(now);
                if matches!(
                    error.code.as_str(),
                    "catalog_database_busy" | "catalog_database_locked"
                ) {
                    Ok(())
                } else {
                    Err(error)
                }
            }
        }
    }

    fn defer_retry(&mut self, now: Instant) {
        let delay = self.retry_delay.unwrap_or(RETRY_INITIAL_DELAY);
        self.next_check = Some(now + delay);
        self.retry_delay = Some(delay.saturating_mul(2).min(RETRY_MAX_DELAY));
    }

    pub(super) fn request_stop(&mut self) {
        self.stopping = true;
        if let Some(task) = &self.task {
            task.cancelled.store(true, Ordering::Release);
        }
    }

    pub(super) fn finish_stopping_until(&mut self, deadline: Instant) -> Result<(), ScanError> {
        self.request_stop();
        if let Some(task) = self.task.as_mut() {
            finish_runtime_task_until(
                &task.receiver,
                &mut task.worker,
                deadline,
                "metadata_inventory_cleanup_stop_timeout",
                "Bounded inventory cleanup did not stop within the shutdown window",
            )?;
            self.task = None;
        }
        Ok(())
    }
}

fn worker_panic() -> ScanError {
    ScanError::new(
        "metadata_inventory_cleanup_worker_panicked",
        "The bounded inventory cleanup worker panicked",
    )
}

#[cfg(test)]
mod tests;
