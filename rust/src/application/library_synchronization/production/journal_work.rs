use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread::{self, JoinHandle};
use std::time::Instant;

use crate::adapters::SqliteCatalogSession;
use crate::application::{AuthoritativeRecoveryPolicy, JournalCaptureWindowReport};
use crate::domain::{
    IncrementalLibraryChangeReport, LibraryChangeLane, LibraryChangeQueuePolicy,
    LibraryRootGeneration, ScanError,
};
use crate::journal_broker::{CallerClaim, PersistentChangeJournalSession};

use super::super::{journal_baseline, journal_drain};
use super::finish_runtime_task_until;
use super::journal_volume::JournalVolumeWork;

pub(super) mod fallback;
mod page;

pub(super) enum JournalWork {
    Opening {
        work: journal_baseline::JournalBaselineOpeningWork,
        session: Arc<dyn PersistentChangeJournalSession>,
        caller: CallerClaim,
    },
    Closing {
        failures: Arc<fallback::JournalReadFailures>,
        work: journal_baseline::JournalBaselineClosingWork,
        session: Arc<dyn PersistentChangeJournalSession>,
        caller: CallerClaim,
    },
    Page {
        work: JournalVolumeWork,
        failures: Arc<fallback::JournalReadFailures>,
        session: Arc<dyn PersistentChangeJournalSession>,
        caller: CallerClaim,
    },
    Drain {
        root_id: String,
        root_generation: LibraryRootGeneration,
    },
}

pub(super) enum JournalTaskOutcome {
    Page(JournalCaptureWindowReport),
    Drain(IncrementalLibraryChangeReport),
    BaselineBoundary,
}

pub(super) struct JournalTask {
    cancelled: Arc<AtomicBool>,
    receiver: Receiver<Result<JournalTaskOutcome, ScanError>>,
    worker: Option<JoinHandle<()>>,
}

impl JournalTask {
    pub(super) fn start(
        work: JournalWork,
        catalog_session: Arc<SqliteCatalogSession>,
        queue_policy: LibraryChangeQueuePolicy,
        recovery_policy: AuthoritativeRecoveryPolicy,
        observed_unix_ms: i64,
    ) -> Result<Self, ScanError> {
        match work {
            JournalWork::Opening {
                work,
                session,
                caller,
            } => Self::start_baseline_opening(
                work,
                session,
                caller,
                observed_unix_ms,
                catalog_session,
                queue_policy,
            ),
            JournalWork::Closing {
                failures,
                work,
                session,
                caller,
            } => Self::start_baseline_closing(
                work,
                session,
                caller,
                observed_unix_ms,
                catalog_session,
                (failures, queue_policy),
            ),
            JournalWork::Page {
                work,
                failures,
                session,
                caller,
            } => Self::start_journal_page(
                work,
                failures,
                session,
                caller,
                observed_unix_ms,
                catalog_session,
                queue_policy,
            ),
            JournalWork::Drain {
                root_id,
                root_generation,
            } => Self::start_journal_drain(
                root_id,
                root_generation,
                observed_unix_ms,
                catalog_session,
                queue_policy,
                recovery_policy,
            ),
        }
    }

    pub(super) fn request_stop(&self) {
        self.cancelled.store(true, Ordering::Release);
    }

    pub(super) fn poll(&mut self) -> Option<Result<JournalTaskOutcome, ScanError>> {
        let result = match self.receiver.try_recv() {
            Ok(result) => result,
            Err(TryRecvError::Empty) => return None,
            Err(TryRecvError::Disconnected) => Err(ScanError::new(
                "persistent_journal_worker_disconnected",
                "The persistent journal worker stopped without a result",
            )),
        };
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
        Some(result)
    }

    pub(super) fn finish_stopping_until(&mut self, deadline: Instant) -> Result<(), ScanError> {
        finish_runtime_task_until(
            &self.receiver,
            &mut self.worker,
            deadline,
            "persistent_journal_stop_timeout",
            "Persistent journal work did not stop within the bounded shutdown window",
        )
    }

    #[cfg(test)]
    pub(super) fn from_test_parts(
        cancelled: Arc<AtomicBool>,
        receiver: Receiver<Result<JournalTaskOutcome, ScanError>>,
        worker: JoinHandle<()>,
    ) -> Self {
        Self {
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

    fn start_baseline_opening(
        work: journal_baseline::JournalBaselineOpeningWork,
        session: Arc<dyn crate::journal_broker::PersistentChangeJournalSession>,
        caller: crate::journal_broker::CallerClaim,
        observed_unix_ms: i64,
        catalog_session: Arc<SqliteCatalogSession>,
        queue_policy: LibraryChangeQueuePolicy,
    ) -> Result<Self, ScanError> {
        let cancelled = Arc::new(AtomicBool::new(false));
        let worker_cancelled = Arc::clone(&cancelled);
        let (sender, receiver) = mpsc::sync_channel(1);
        let worker = thread::Builder::new()
            .name("ame-p1-baseline-opening".to_owned())
            .spawn(move || {
                let result = journal_baseline::capture_opening_boundary(
                    work,
                    session.as_ref(),
                    &caller,
                    observed_unix_ms,
                    catalog_session.as_ref(),
                    queue_policy,
                    worker_cancelled.as_ref(),
                )
                .map(|_| JournalTaskOutcome::BaselineBoundary);
                let _ = sender.send(result);
            })
            .map_err(|error| {
                ScanError::new(
                    "persistent_journal_worker_start_failed",
                    format!("Could not start the opening boundary worker: {error}"),
                )
            })?;
        Ok(Self {
            cancelled,
            receiver,
            worker: Some(worker),
        })
    }

    fn start_baseline_closing(
        work: journal_baseline::JournalBaselineClosingWork,
        session: Arc<dyn crate::journal_broker::PersistentChangeJournalSession>,
        caller: crate::journal_broker::CallerClaim,
        observed_unix_ms: i64,
        catalog_session: Arc<SqliteCatalogSession>,
        fallback: (Arc<fallback::JournalReadFailures>, LibraryChangeQueuePolicy),
    ) -> Result<Self, ScanError> {
        let cancelled = Arc::new(AtomicBool::new(false));
        let worker_cancelled = Arc::clone(&cancelled);
        let (sender, receiver) = mpsc::sync_channel(1);
        let worker = thread::Builder::new()
            .name("ame-p1-baseline-closing".to_owned())
            .spawn(move || {
                let root = (work.root_id().to_owned(), work.root_generation());
                let result = journal_baseline::capture_closing_boundary(
                    work,
                    session.as_ref(),
                    &caller,
                    observed_unix_ms,
                    catalog_session.as_ref(),
                    worker_cancelled.as_ref(),
                )
                .or_else(|error| {
                    fallback.0.handle_closing_failure(
                        error,
                        &root,
                        catalog_session.as_ref(),
                        observed_unix_ms,
                        fallback.1,
                        worker_cancelled.as_ref(),
                    )
                })
                .map(|()| JournalTaskOutcome::BaselineBoundary);
                let _ = sender.send(result);
            })
            .map_err(|error| {
                ScanError::new(
                    "persistent_journal_worker_start_failed",
                    format!("Could not start the closing boundary worker: {error}"),
                )
            })?;
        Ok(Self {
            cancelled,
            receiver,
            worker: Some(worker),
        })
    }

    fn start_journal_page(
        work: JournalVolumeWork,
        failures: Arc<fallback::JournalReadFailures>,
        session: Arc<dyn crate::journal_broker::PersistentChangeJournalSession>,
        caller: crate::journal_broker::CallerClaim,
        observed_unix_ms: i64,
        catalog_session: Arc<SqliteCatalogSession>,
        policy: LibraryChangeQueuePolicy,
    ) -> Result<Self, ScanError> {
        let cancelled = Arc::new(AtomicBool::new(false));
        let worker_cancelled = Arc::clone(&cancelled);
        let (sender, receiver) = mpsc::sync_channel(1);
        let worker = thread::Builder::new()
            .name("ame-p1-journal-page".to_owned())
            .spawn(move || {
                let result = page::run(
                    work,
                    session,
                    caller,
                    observed_unix_ms,
                    catalog_session,
                    policy,
                    page::JournalPageControl {
                        cancelled: worker_cancelled,
                        failures,
                    },
                );
                let _ = sender.send(result);
            })
            .map_err(|error| {
                ScanError::new(
                    "persistent_journal_worker_start_failed",
                    format!("Could not start the persistent journal page worker: {error}"),
                )
            })?;
        Ok(Self {
            cancelled,
            receiver,
            worker: Some(worker),
        })
    }

    fn start_journal_drain(
        root_id: String,
        root_generation: LibraryRootGeneration,
        now_unix_ms: i64,
        catalog_session: Arc<SqliteCatalogSession>,
        policy: LibraryChangeQueuePolicy,
        recovery_policy: AuthoritativeRecoveryPolicy,
    ) -> Result<Self, ScanError> {
        let cancelled = Arc::new(AtomicBool::new(false));
        let worker_cancelled = Arc::clone(&cancelled);
        let (sender, receiver) = mpsc::sync_channel(1);
        let worker = thread::Builder::new()
            .name("ame-p1-journal-drain".to_owned())
            .spawn(move || {
                let result = catalog_session
                    .open_in_lane(LibraryChangeLane::Journal)
                    .and_then(|mut catalog| {
                        journal_drain::process_ready(
                            &mut catalog,
                            &root_id,
                            root_generation,
                            now_unix_ms,
                            policy,
                            recovery_policy,
                            &worker_cancelled,
                        )
                        .map(JournalTaskOutcome::Drain)
                    });
                let _ = sender.send(result);
            })
            .map_err(|error| {
                ScanError::new(
                    "persistent_journal_worker_start_failed",
                    format!("Could not start the persistent journal drain worker: {error}"),
                )
            })?;
        Ok(Self {
            cancelled,
            receiver,
            worker: Some(worker),
        })
    }
}
