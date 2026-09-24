use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread::{self, JoinHandle};
use std::time::Instant;

use crate::adapters::SqliteCatalogSession;
use crate::application::AuthoritativeRecoveryPolicy;
use crate::domain::{LibraryChangeQueuePolicy, LibraryRootGeneration, ScanError};

use super::finish_runtime_task_until;

mod batch;

pub(super) use batch::LiveWorkOutcome;

pub(super) struct LiveTask {
    root_id: String,
    cancelled: Arc<AtomicBool>,
    receiver: Receiver<LiveWorkOutcome>,
    worker: Option<JoinHandle<()>>,
}

impl LiveTask {
    pub(super) fn start(
        session: Arc<SqliteCatalogSession>,
        root_id: String,
        root_generation: LibraryRootGeneration,
        queue_policy: LibraryChangeQueuePolicy,
        recovery_policy: AuthoritativeRecoveryPolicy,
    ) -> Result<Self, ScanError> {
        let cancelled = Arc::new(AtomicBool::new(false));
        let worker_cancelled = Arc::clone(&cancelled);
        let worker_root_id = root_id.clone();
        let (sender, receiver) = mpsc::sync_channel(1);
        let worker = thread::Builder::new()
            .name("ame-p0-live-reconciliation".to_owned())
            .spawn(move || {
                let result = batch::run(
                    session,
                    &worker_root_id,
                    root_generation,
                    queue_policy,
                    recovery_policy,
                    &worker_cancelled,
                );
                let _ = sender.send(result);
            })
            .map_err(|error| {
                ScanError::new(
                    "live_reconciliation_worker_start_failed",
                    format!("Could not start the reserved P0 worker: {error}"),
                )
            })?;
        Ok(Self {
            root_id,
            cancelled,
            receiver,
            worker: Some(worker),
        })
    }

    pub(super) fn root_id(&self) -> &str {
        &self.root_id
    }

    pub(super) fn request_stop(&self) {
        self.cancelled.store(true, Ordering::Release);
    }

    pub(super) fn poll(&mut self) -> Option<LiveWorkOutcome> {
        if self
            .worker
            .as_ref()
            .is_some_and(|worker| !worker.is_finished())
        {
            return None;
        }
        let outcome = match self.receiver.try_recv() {
            Ok(outcome) => outcome,
            Err(TryRecvError::Empty) => return None,
            Err(TryRecvError::Disconnected) => LiveWorkOutcome::failed(ScanError::new(
                "live_reconciliation_worker_disconnected",
                "The reserved P0 worker stopped without a result",
            )),
        };
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
        Some(outcome)
    }

    pub(super) fn finish_stopping_until(&mut self, deadline: Instant) -> Result<(), ScanError> {
        self.request_stop();
        finish_runtime_task_until(
            &self.receiver,
            &mut self.worker,
            deadline,
            "live_reconciliation_stop_timeout",
            "P0 reconciliation did not stop within the bounded shutdown window",
        )
    }

    #[cfg(test)]
    pub(super) fn from_test_parts(
        root_id: String,
        cancelled: Arc<AtomicBool>,
        receiver: Receiver<LiveWorkOutcome>,
        worker: JoinHandle<()>,
    ) -> Self {
        Self {
            root_id,
            cancelled,
            receiver,
            worker: Some(worker),
        }
    }

    #[cfg(test)]
    pub(super) fn progress(&self) -> (Option<&str>, Option<bool>, bool) {
        (
            self.worker
                .as_ref()
                .and_then(|worker| worker.thread().name()),
            self.worker.as_ref().map(JoinHandle::is_finished),
            self.cancelled.load(Ordering::Acquire),
        )
    }
}

#[cfg(test)]
mod tests;
