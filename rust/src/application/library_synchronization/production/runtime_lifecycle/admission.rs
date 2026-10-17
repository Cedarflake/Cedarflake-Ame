use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::domain::ScanError;

use super::super::ProductionSynchronization;
use super::retirement::stop_timeout_error;
use super::state::{
    AdmittedStopFence, AdmittedStopFenceUse, ConstructedRuntimeDisposition, RuntimePollClaim,
    RuntimeStartTicket, RuntimeStopFence, SynchronizationRuntimeClaim, SynchronizationRuntimeEntry,
    SynchronizationRuntimeRegistry, SynchronizationRuntimeState, lock_runtime_registry,
    registry_notify_all, runtime_busy_error,
};

pub(super) fn first_import_runtime_owner_for_poll(
    registry: &SynchronizationRuntimeRegistry,
) -> Result<Option<RuntimeStartTicket>, ScanError> {
    let registry = lock_runtime_registry(registry)?;
    match &registry.entry {
        SynchronizationRuntimeEntry::Empty
        | SynchronizationRuntimeEntry::Starting { .. }
        | SynchronizationRuntimeEntry::Polling { .. } => Ok(None),
        SynchronizationRuntimeEntry::Ready { owner_ticket, .. } => Ok(Some(*owner_ticket)),
        SynchronizationRuntimeEntry::Stopping { .. }
        | SynchronizationRuntimeEntry::Draining { .. } => Err(ScanError::new(
            "library_synchronization_stop_in_progress",
            "Library synchronization is stopping before first-import change capture",
        )),
    }
}

pub(super) fn reserve_runtime_start_ticket(
    registry: &SynchronizationRuntimeRegistry,
) -> Result<RuntimeStartTicket, ScanError> {
    let mut registry = lock_runtime_registry(registry)?;
    registry
        .allocate_lifecycle_ordinal()
        .map(RuntimeStartTicket::from_ordinal)
}

pub(super) fn reserve_runtime_stop_fence(
    registry: &SynchronizationRuntimeRegistry,
    timeout: Duration,
) -> Result<RuntimeStopFence, ScanError> {
    let deadline = Instant::now()
        .checked_add(timeout)
        .ok_or_else(stop_timeout_error)?;
    reserve_runtime_stop_fence_until(registry, deadline)
}

pub(super) fn reserve_runtime_stop_fence_until(
    registry: &SynchronizationRuntimeRegistry,
    deadline: Instant,
) -> Result<RuntimeStopFence, ScanError> {
    let mut registry_guard = lock_runtime_registry(registry)?;
    let next_ordinal = registry_guard.next_lifecycle_ordinal.checked_next()?;
    registry_guard.prepare_stop_fence_capacity()?;
    let cancellation_fence = RuntimeStopFence::from_ordinal(next_ordinal);
    let covered_owner = registry_guard.entry.owner_identity();
    registry_guard.next_lifecycle_ordinal = next_ordinal;
    registry_guard
        .admitted_stop_fences
        .push_back(AdmittedStopFence {
            fence: cancellation_fence,
            covered_owner,
            use_state: AdmittedStopFenceUse::Pending,
        });
    registry_guard.cancelled_through_ordinal = cancellation_fence.ordinal();
    let entry = std::mem::replace(
        &mut registry_guard.entry,
        SynchronizationRuntimeEntry::Empty,
    );
    registry_guard.entry = match entry {
        SynchronizationRuntimeEntry::Starting {
            epoch,
            owner_ticket,
            cancelled,
        } if owner_ticket.ordinal() <= cancellation_fence.ordinal() => {
            cancelled.store(true, Ordering::Release);
            SynchronizationRuntimeEntry::Stopping {
                epoch,
                owner_ticket,
                cancellation_fence,
                deadline,
            }
        }
        SynchronizationRuntimeEntry::Ready {
            epoch,
            owner_ticket,
            cancelled,
            runtime,
        }
        | SynchronizationRuntimeEntry::Polling {
            epoch,
            owner_ticket,
            cancelled,
            runtime,
        } if owner_ticket.ordinal() <= cancellation_fence.ordinal() => {
            cancelled.store(true, Ordering::Release);
            SynchronizationRuntimeEntry::Draining {
                epoch,
                owner_ticket,
                cancellation_fence: Some(cancellation_fence),
                deadline,
                runtime,
            }
        }
        SynchronizationRuntimeEntry::Stopping {
            epoch,
            owner_ticket,
            cancellation_fence: first_fence,
            deadline: first_deadline,
        } => SynchronizationRuntimeEntry::Stopping {
            epoch,
            owner_ticket,
            cancellation_fence: first_fence,
            deadline: first_deadline,
        },
        SynchronizationRuntimeEntry::Draining {
            epoch,
            owner_ticket,
            cancellation_fence: first_fence,
            deadline: first_deadline,
            runtime,
        } => SynchronizationRuntimeEntry::Draining {
            epoch,
            owner_ticket,
            cancellation_fence: first_fence.or(Some(cancellation_fence)),
            deadline: first_deadline,
            runtime,
        },
        entry => entry,
    };
    drop(registry_guard);
    registry_notify_all(registry);
    Ok(cancellation_fence)
}

pub(super) fn validate_runtime_owner_ticket(
    registry: &SynchronizationRuntimeState,
    owner_ticket: RuntimeStartTicket,
) -> Result<(), ScanError> {
    if owner_ticket.ordinal() > registry.next_lifecycle_ordinal {
        return Err(ScanError::new(
            "library_synchronization_lifecycle_ticket_invalid",
            "The synchronization lifecycle ticket was not issued by this process",
        ));
    }
    if owner_ticket.ordinal() <= registry.cancelled_through_ordinal {
        return Err(ScanError::new(
            "library_synchronization_start_cancelled",
            "The synchronization start request was cancelled before admission",
        ));
    }
    Ok(())
}

pub(super) fn claim_runtime_for_start_with_ticket(
    registry: &SynchronizationRuntimeRegistry,
    owner_ticket: RuntimeStartTicket,
) -> Result<SynchronizationRuntimeClaim, ScanError> {
    let mut registry = lock_runtime_registry(registry)?;
    validate_runtime_owner_ticket(&registry, owner_ticket)?;
    match std::mem::replace(&mut registry.entry, SynchronizationRuntimeEntry::Empty) {
        SynchronizationRuntimeEntry::Empty => {
            let epoch = registry.allocate_epoch()?;
            let cancelled = Arc::new(AtomicBool::new(false));
            registry.entry = SynchronizationRuntimeEntry::Starting {
                epoch,
                owner_ticket,
                cancelled: Arc::clone(&cancelled),
            };
            Ok(SynchronizationRuntimeClaim::Construct { epoch, cancelled })
        }
        SynchronizationRuntimeEntry::Ready {
            epoch,
            owner_ticket: active_owner_ticket,
            cancelled,
            runtime,
        } if active_owner_ticket == owner_ticket => {
            registry.entry = SynchronizationRuntimeEntry::Polling {
                epoch,
                owner_ticket,
                cancelled: Arc::clone(&cancelled),
                runtime: Arc::clone(&runtime),
            };
            Ok(SynchronizationRuntimeClaim::Poll {
                epoch,
                cancelled,
                runtime,
            })
        }
        entry => {
            let error = runtime_busy_error(&entry);
            registry.entry = entry;
            Err(error)
        }
    }
}

#[cfg(test)]
pub(super) fn claim_runtime_for_start(
    registry: &SynchronizationRuntimeRegistry,
) -> Result<SynchronizationRuntimeClaim, ScanError> {
    let owner_ticket = reserve_runtime_start_ticket(registry)?;
    claim_runtime_for_start_with_ticket(registry, owner_ticket)
}

pub(super) fn promote_constructed_runtime(
    registry: &SynchronizationRuntimeRegistry,
    epoch: u64,
    owner_ticket: RuntimeStartTicket,
    cancelled: Arc<AtomicBool>,
    runtime: ProductionSynchronization,
) -> Result<ConstructedRuntimeDisposition, ScanError> {
    let runtime = Arc::new(Mutex::new(runtime));
    let mut registry_guard = lock_runtime_registry(registry)?;
    match &registry_guard.entry {
        SynchronizationRuntimeEntry::Starting {
            epoch: active_epoch,
            owner_ticket: active_owner_ticket,
            ..
        } if *active_epoch == epoch
            && *active_owner_ticket == owner_ticket
            && !cancelled.load(Ordering::Acquire) =>
        {
            registry_guard.entry = SynchronizationRuntimeEntry::Polling {
                epoch,
                owner_ticket,
                cancelled,
                runtime: Arc::clone(&runtime),
            };
            Ok(ConstructedRuntimeDisposition::Poll(runtime))
        }
        SynchronizationRuntimeEntry::Stopping {
            epoch: active_epoch,
            owner_ticket: active_owner_ticket,
            cancellation_fence,
            deadline,
        } if *active_epoch == epoch && *active_owner_ticket == owner_ticket => {
            let deadline = *deadline;
            let cancellation_fence = *cancellation_fence;
            registry_guard.entry = SynchronizationRuntimeEntry::Draining {
                epoch,
                owner_ticket,
                cancellation_fence: Some(cancellation_fence),
                deadline,
                runtime: Arc::clone(&runtime),
            };
            drop(registry_guard);
            registry_notify_all(registry);
            Ok(ConstructedRuntimeDisposition::Drain { deadline, runtime })
        }
        _ => Err(ScanError::new(
            "library_synchronization_owner_state_invalid",
            "A constructed synchronization runtime no longer matches its registry epoch",
        )),
    }
}

pub(super) fn claim_runtime_for_poll_with_ticket(
    registry: &SynchronizationRuntimeRegistry,
    owner_ticket: RuntimeStartTicket,
) -> Result<RuntimePollClaim, ScanError> {
    let mut registry = lock_runtime_registry(registry)?;
    match std::mem::replace(&mut registry.entry, SynchronizationRuntimeEntry::Empty) {
        SynchronizationRuntimeEntry::Ready {
            epoch,
            owner_ticket: active_owner_ticket,
            cancelled,
            runtime,
        } if active_owner_ticket == owner_ticket
            && owner_ticket.ordinal() > registry.cancelled_through_ordinal =>
        {
            registry.entry = SynchronizationRuntimeEntry::Polling {
                epoch,
                owner_ticket,
                cancelled: Arc::clone(&cancelled),
                runtime: Arc::clone(&runtime),
            };
            Ok((epoch, cancelled, runtime))
        }
        SynchronizationRuntimeEntry::Empty => {
            registry.entry = SynchronizationRuntimeEntry::Empty;
            Err(ScanError::new(
                "library_synchronization_not_started",
                "Library synchronization must start before it can be polled",
            ))
        }
        entry => {
            if entry.owner_ticket() != Some(owner_ticket) {
                registry.entry = entry;
                return Err(ScanError::new(
                    "library_synchronization_lifecycle_ticket_stale",
                    "The synchronization poll request does not own the active runtime",
                ));
            }
            let error = runtime_busy_error(&entry);
            registry.entry = entry;
            Err(error)
        }
    }
}

#[cfg(test)]
pub(super) fn claim_runtime_for_poll(
    registry: &SynchronizationRuntimeRegistry,
) -> Result<RuntimePollClaim, ScanError> {
    let owner_ticket = lock_runtime_registry(registry)?
        .entry
        .owner_ticket()
        .ok_or_else(|| {
            ScanError::new(
                "library_synchronization_not_started",
                "Library synchronization must start before it can be polled",
            )
        })?;
    claim_runtime_for_poll_with_ticket(registry, owner_ticket)
}
