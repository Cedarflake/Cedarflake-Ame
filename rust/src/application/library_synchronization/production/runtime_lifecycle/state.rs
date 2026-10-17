use std::collections::VecDeque;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Condvar, Mutex, MutexGuard, OnceLock};
use std::time::Instant;

use crate::domain::ScanError;

use super::super::ProductionSynchronization;

pub(super) const MAX_ADMITTED_STOP_FENCES: usize = 64;

static SYNCHRONIZATION_RUNTIME: OnceLock<SynchronizationRuntimeRegistry> = OnceLock::new();

pub(super) struct SynchronizationRuntimeRegistry {
    pub(super) state: Mutex<SynchronizationRuntimeState>,
    pub(super) changed: Condvar,
    #[cfg(test)]
    pub(super) panic_retention_barrier: Mutex<Option<PanicRetentionBarrier>>,
}

#[cfg(test)]
#[derive(Clone)]
pub(super) struct PanicRetentionBarrier {
    pub(super) reached: Arc<std::sync::Barrier>,
    pub(super) resume: Arc<std::sync::Barrier>,
}

pub(super) struct SynchronizationRuntimeState {
    pub(super) next_epoch: u64,
    pub(super) next_lifecycle_ordinal: LifecycleOrdinal,
    pub(super) cancelled_through_ordinal: LifecycleOrdinal,
    pub(super) admitted_stop_fences: VecDeque<AdmittedStopFence>,
    pub(super) retired_through_epoch: u64,
    pub(super) entry: SynchronizationRuntimeEntry,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct LifecycleOrdinal(pub(super) u64);

impl LifecycleOrdinal {
    pub(super) const MAX: u64 = u64::MAX >> 1;

    pub(super) fn checked_next(self) -> Result<Self, ScanError> {
        let next = self.0.checked_add(1).filter(|next| *next <= Self::MAX);
        next.map(Self).ok_or_else(|| {
            ScanError::new(
                "library_synchronization_lifecycle_ticket_exhausted",
                "The synchronization lifecycle ticket space is exhausted",
            )
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct RuntimeStartTicket(LifecycleOrdinal);

impl RuntimeStartTicket {
    const KIND: u64 = 0;

    pub(super) const fn from_ordinal(ordinal: LifecycleOrdinal) -> Self {
        Self(ordinal)
    }

    pub(super) const fn ordinal(self) -> LifecycleOrdinal {
        self.0
    }

    pub(super) const fn raw(self) -> u64 {
        (self.0.0 << 1) | Self::KIND
    }

    pub(super) fn decode(raw: u64) -> Result<Self, ScanError> {
        let ordinal = raw >> 1;
        if raw & 1 != Self::KIND || ordinal == 0 {
            return Err(ScanError::new(
                "library_synchronization_lifecycle_ticket_invalid",
                "The synchronization lifecycle ticket was not issued by this process",
            ));
        }
        Ok(Self(LifecycleOrdinal(ordinal)))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct RuntimeStopFence(LifecycleOrdinal);

impl RuntimeStopFence {
    pub(super) const KIND: u64 = 1;

    pub(super) const fn from_ordinal(ordinal: LifecycleOrdinal) -> Self {
        Self(ordinal)
    }

    pub(super) const fn ordinal(self) -> LifecycleOrdinal {
        self.0
    }

    pub(super) const fn raw(self) -> u64 {
        (self.0.0 << 1) | Self::KIND
    }

    pub(super) fn decode(raw: u64) -> Result<Self, ScanError> {
        let ordinal = raw >> 1;
        if raw & 1 != Self::KIND || ordinal == 0 {
            return Err(ScanError::new(
                "library_synchronization_lifecycle_fence_invalid",
                "The synchronization stop fence was not admitted by this process",
            ));
        }
        Ok(Self(LifecycleOrdinal(ordinal)))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct RuntimeOwnerIdentity {
    pub(super) epoch: u64,
    pub(super) owner_ticket: RuntimeStartTicket,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum AdmittedStopFenceUse {
    Pending,
    Consumed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct AdmittedStopFence {
    pub(super) fence: RuntimeStopFence,
    pub(super) covered_owner: Option<RuntimeOwnerIdentity>,
    pub(super) use_state: AdmittedStopFenceUse,
}

pub(super) enum SynchronizationRuntimeEntry {
    Empty,
    Starting {
        epoch: u64,
        owner_ticket: RuntimeStartTicket,
        cancelled: Arc<AtomicBool>,
    },
    Ready {
        epoch: u64,
        owner_ticket: RuntimeStartTicket,
        cancelled: Arc<AtomicBool>,
        runtime: Arc<Mutex<ProductionSynchronization>>,
    },
    Polling {
        epoch: u64,
        owner_ticket: RuntimeStartTicket,
        cancelled: Arc<AtomicBool>,
        runtime: Arc<Mutex<ProductionSynchronization>>,
    },
    Stopping {
        epoch: u64,
        owner_ticket: RuntimeStartTicket,
        cancellation_fence: RuntimeStopFence,
        deadline: Instant,
    },
    Draining {
        epoch: u64,
        owner_ticket: RuntimeStartTicket,
        cancellation_fence: Option<RuntimeStopFence>,
        deadline: Instant,
        runtime: Arc<Mutex<ProductionSynchronization>>,
    },
}

impl SynchronizationRuntimeEntry {
    pub(super) const fn owner_ticket(&self) -> Option<RuntimeStartTicket> {
        match self {
            Self::Empty => None,
            Self::Starting { owner_ticket, .. }
            | Self::Ready { owner_ticket, .. }
            | Self::Polling { owner_ticket, .. }
            | Self::Stopping { owner_ticket, .. }
            | Self::Draining { owner_ticket, .. } => Some(*owner_ticket),
        }
    }

    pub(super) const fn owner_identity(&self) -> Option<RuntimeOwnerIdentity> {
        match self {
            Self::Empty => None,
            Self::Starting {
                epoch,
                owner_ticket,
                ..
            }
            | Self::Ready {
                epoch,
                owner_ticket,
                ..
            }
            | Self::Polling {
                epoch,
                owner_ticket,
                ..
            }
            | Self::Stopping {
                epoch,
                owner_ticket,
                ..
            }
            | Self::Draining {
                epoch,
                owner_ticket,
                ..
            } => Some(RuntimeOwnerIdentity {
                epoch: *epoch,
                owner_ticket: *owner_ticket,
            }),
        }
    }
}

pub(super) enum SynchronizationRuntimeClaim {
    Construct {
        epoch: u64,
        cancelled: Arc<AtomicBool>,
    },
    Poll {
        epoch: u64,
        cancelled: Arc<AtomicBool>,
        runtime: Arc<Mutex<ProductionSynchronization>>,
    },
}

pub(super) enum ConstructedRuntimeDisposition {
    Poll(Arc<Mutex<ProductionSynchronization>>),
    Drain {
        deadline: Instant,
        runtime: Arc<Mutex<ProductionSynchronization>>,
    },
}

pub(super) enum PanickedRuntimeRetention {
    Retained(Instant),
    AlreadyDrained,
}

pub(super) type RuntimePollClaim = (u64, Arc<AtomicBool>, Arc<Mutex<ProductionSynchronization>>);

impl Default for SynchronizationRuntimeRegistry {
    fn default() -> Self {
        Self {
            state: Mutex::new(SynchronizationRuntimeState {
                next_epoch: 0,
                next_lifecycle_ordinal: LifecycleOrdinal(0),
                cancelled_through_ordinal: LifecycleOrdinal(0),
                admitted_stop_fences: VecDeque::new(),
                retired_through_epoch: 0,
                entry: SynchronizationRuntimeEntry::Empty,
            }),
            changed: Condvar::new(),
            #[cfg(test)]
            panic_retention_barrier: Mutex::new(None),
        }
    }
}

impl SynchronizationRuntimeState {
    pub(super) fn allocate_lifecycle_ordinal(&mut self) -> Result<LifecycleOrdinal, ScanError> {
        let ordinal = self.next_lifecycle_ordinal.checked_next()?;
        self.next_lifecycle_ordinal = ordinal;
        Ok(ordinal)
    }

    pub(super) fn allocate_epoch(&mut self) -> Result<u64, ScanError> {
        self.next_epoch = self.next_epoch.checked_add(1).ok_or_else(|| {
            ScanError::new(
                "library_synchronization_epoch_exhausted",
                "The synchronization runtime epoch space is exhausted",
            )
        })?;
        Ok(self.next_epoch)
    }

    pub(super) fn retire_epoch(&mut self, epoch: u64) {
        self.retired_through_epoch = self.retired_through_epoch.max(epoch);
    }

    pub(super) fn prepare_stop_fence_capacity(&mut self) -> Result<(), ScanError> {
        if self.admitted_stop_fences.len() < MAX_ADMITTED_STOP_FENCES {
            return Ok(());
        }

        let active_owner = self.entry.owner_identity();
        let reclaimable = self.admitted_stop_fences.iter().position(|entry| {
            entry.use_state == AdmittedStopFenceUse::Consumed
                && !entry
                    .covered_owner
                    .is_some_and(|owner| Some(owner) == active_owner)
        });
        let Some(reclaimable) = reclaimable else {
            return Err(ScanError::new(
                "library_synchronization_lifecycle_fence_capacity_exhausted",
                "The bounded synchronization stop-fence ledger is full",
            ));
        };
        self.admitted_stop_fences.remove(reclaimable);
        Ok(())
    }

    pub(super) fn consume_admitted_stop_fence(
        &mut self,
        cancellation_fence: RuntimeStopFence,
    ) -> Result<(), ScanError> {
        if cancellation_fence.ordinal() > self.next_lifecycle_ordinal
            || cancellation_fence.ordinal() > self.cancelled_through_ordinal
        {
            return Err(invalid_stop_fence_error());
        }
        let Some(entry) = self
            .admitted_stop_fences
            .iter_mut()
            .find(|entry| entry.fence == cancellation_fence)
        else {
            return Err(invalid_stop_fence_error());
        };
        entry.use_state = AdmittedStopFenceUse::Consumed;
        Ok(())
    }
}

pub(super) fn runtime_registry() -> &'static SynchronizationRuntimeRegistry {
    SYNCHRONIZATION_RUNTIME.get_or_init(SynchronizationRuntimeRegistry::default)
}

pub(super) fn lock_runtime_registry(
    registry: &SynchronizationRuntimeRegistry,
) -> Result<MutexGuard<'_, SynchronizationRuntimeState>, ScanError> {
    registry.state.lock().map_err(|_| {
        ScanError::new(
            "library_synchronization_state_unavailable",
            "The library synchronization runtime state is unavailable",
        )
    })
}

pub(super) fn invalid_stop_fence_error() -> ScanError {
    ScanError::new(
        "library_synchronization_lifecycle_fence_invalid",
        "The synchronization stop fence was not admitted by this process",
    )
}

pub(super) fn registry_notify_all(registry: &SynchronizationRuntimeRegistry) {
    registry.changed.notify_all();
}

pub(super) fn runtime_busy_error(entry: &SynchronizationRuntimeEntry) -> ScanError {
    let (code, message) = match entry {
        SynchronizationRuntimeEntry::Starting { .. } => (
            "library_synchronization_start_in_progress",
            "Synchronization startup is already in progress",
        ),
        SynchronizationRuntimeEntry::Polling { .. } => (
            "library_synchronization_poll_in_progress",
            "Synchronization polling is already in progress",
        ),
        SynchronizationRuntimeEntry::Stopping { .. }
        | SynchronizationRuntimeEntry::Draining { .. } => (
            "library_synchronization_stop_in_progress",
            "The prior synchronization epoch is still stopping",
        ),
        SynchronizationRuntimeEntry::Empty | SynchronizationRuntimeEntry::Ready { .. } => (
            "library_synchronization_state_raced",
            "The synchronization runtime changed while acquiring ownership",
        ),
    };
    ScanError::new(code, message)
}
