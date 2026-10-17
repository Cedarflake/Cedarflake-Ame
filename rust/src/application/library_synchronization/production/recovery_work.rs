use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(test)]
use std::sync::mpsc::RecvTimeoutError;
use std::sync::mpsc::{Receiver, TryRecvError};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Instant;

use crate::application::{
    AuthoritativeLibraryChangeReport, AuthoritativeRecoveryPolicy, MetadataInventoryRecoveryPage,
};
use crate::domain::{
    IncrementalLibraryChangeReport, LibraryChangeId, LibraryChangeQueuePolicy,
    LibraryRootGeneration, LibrarySynchronizationPhase, ScanError,
};

use super::finish_runtime_task_until;

mod automatic;
mod drain;
#[cfg(test)]
pub(super) use automatic::fail_next_recovery_worker_spawn;
pub(super) use automatic::start as start_automatic;
pub(super) use drain::{start_candidates, start_legacy_unowned};

pub(super) struct RecoveryWorkRoot {
    pub root_id: String,
    pub root_generation: LibraryRootGeneration,
    pub continuity_revision: u64,
}

pub(super) struct RecoveryWorkPolicy {
    pub queue: LibraryChangeQueuePolicy,
    pub recovery: AuthoritativeRecoveryPolicy,
    pub metadata_inventory_page_entries: u32,
}

pub(super) struct RecoveryTask {
    root_id: String,
    kind: RecoveryTaskKind,
    phase: Arc<Mutex<LibrarySynchronizationPhase>>,
    cancelled: Arc<AtomicBool>,
    receiver: Receiver<Result<RecoveryTaskOutcome, ScanError>>,
    worker: Option<JoinHandle<()>>,
}

pub(super) enum RecoveryTaskKind {
    BoundedAuthoritative {
        continuity_revision: u64,
    },
    MetadataInventory {
        continuity_revision: u64,
        change_id: LibraryChangeId,
    },
    CandidateDrain {
        continuity_revision: u64,
    },
    LegacyUnownedDrain {
        continuity_revision: u64,
    },
}

pub(super) enum RecoveryTaskOutcome {
    Authoritative(AuthoritativeLibraryChangeReport),
    MetadataInventory(Box<MetadataInventoryRecoveryPage>),
    CandidateDrain(IncrementalLibraryChangeReport),
    LegacyUnownedDrain(IncrementalLibraryChangeReport),
}

impl RecoveryTask {
    pub(super) fn root_id(&self) -> &str {
        &self.root_id
    }

    pub(super) fn kind(&self) -> &RecoveryTaskKind {
        &self.kind
    }

    pub(super) fn phase(&self) -> LibrarySynchronizationPhase {
        self.phase
            .lock()
            .map_or_else(|_| self.default_phase(), |phase| *phase)
    }

    pub(super) fn request_stop(&self) {
        self.cancelled.store(true, Ordering::Release);
    }

    pub(super) fn finish_stopping_until(&mut self, deadline: Instant) -> Result<(), ScanError> {
        finish_runtime_task_until(
            &self.receiver,
            &mut self.worker,
            deadline,
            "authoritative_recovery_stop_timeout",
            "Authoritative recovery did not stop within the bounded shutdown window",
        )
    }

    #[cfg(test)]
    pub(super) fn from_test_parts(
        root_id: String,
        kind: RecoveryTaskKind,
        phase: Arc<Mutex<LibrarySynchronizationPhase>>,
        cancelled: Arc<AtomicBool>,
        receiver: Receiver<Result<RecoveryTaskOutcome, ScanError>>,
        worker: Option<JoinHandle<()>>,
    ) -> Self {
        Self {
            root_id,
            kind,
            phase,
            cancelled,
            receiver,
            worker,
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

    pub(super) fn poll(&mut self) -> Option<Result<RecoveryTaskOutcome, ScanError>> {
        let result = match self.receiver.try_recv() {
            Ok(result) => result,
            Err(TryRecvError::Empty) => return None,
            Err(TryRecvError::Disconnected) => Err(ScanError::new(
                "authoritative_recovery_worker_disconnected",
                "The authoritative recovery worker stopped without a result",
            )),
        };
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
        Some(result)
    }

    pub(super) fn continuity_revision(&self) -> u64 {
        match self.kind {
            RecoveryTaskKind::BoundedAuthoritative {
                continuity_revision,
            }
            | RecoveryTaskKind::MetadataInventory {
                continuity_revision,
                ..
            }
            | RecoveryTaskKind::CandidateDrain {
                continuity_revision,
            }
            | RecoveryTaskKind::LegacyUnownedDrain {
                continuity_revision,
            } => continuity_revision,
        }
    }

    const fn default_phase(&self) -> LibrarySynchronizationPhase {
        match &self.kind {
            RecoveryTaskKind::BoundedAuthoritative { .. } => {
                LibrarySynchronizationPhase::Reconciliation
            }
            RecoveryTaskKind::MetadataInventory { .. } => {
                LibrarySynchronizationPhase::InventoryEnumeration
            }
            RecoveryTaskKind::CandidateDrain { .. } => LibrarySynchronizationPhase::Reconciliation,
            RecoveryTaskKind::LegacyUnownedDrain { .. } => {
                LibrarySynchronizationPhase::Reconciliation
            }
        }
    }

    #[cfg(test)]
    pub(super) fn stop_with_timeout(
        &mut self,
        timeout: std::time::Duration,
    ) -> Result<(), ScanError> {
        self.cancelled.store(true, Ordering::Release);
        match self.receiver.recv_timeout(timeout) {
            Ok(_) | Err(RecvTimeoutError::Disconnected) => {
                if let Some(worker) = self.worker.take() {
                    let _ = worker.join();
                }
                Ok(())
            }
            Err(RecvTimeoutError::Timeout) => Err(ScanError::new(
                "authoritative_recovery_stop_timeout",
                "Authoritative recovery did not stop within the bounded shutdown window",
            )),
        }
    }
}

#[cfg(test)]
mod tests;
