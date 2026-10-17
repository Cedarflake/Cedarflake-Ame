use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use crate::domain::{LibrarySynchronizationSnapshot, ScanError};

use super::super::{ProductionSynchronization, SYNCHRONIZATION_STOP_TIMEOUT, poll_runtime};
#[cfg(test)]
use super::admission::reserve_runtime_start_ticket;
use super::admission::{claim_runtime_for_start_with_ticket, promote_constructed_runtime};
use super::retirement::{finish_draining_runtime, stop_timeout_error};
use super::state::{
    ConstructedRuntimeDisposition, PanickedRuntimeRetention, RuntimeStartTicket,
    SynchronizationRuntimeClaim, SynchronizationRuntimeEntry, SynchronizationRuntimeRegistry,
    lock_runtime_registry, registry_notify_all,
};

#[cfg(test)]
pub(super) fn start_runtime_with(
    registry: &SynchronizationRuntimeRegistry,
    constructor: impl FnOnce() -> ProductionSynchronization,
) -> Result<LibrarySynchronizationSnapshot, ScanError> {
    let owner_ticket = reserve_runtime_start_ticket(registry)?;
    start_runtime_with_ticket(registry, owner_ticket, constructor)
}

pub(super) fn start_runtime_with_ticket(
    registry: &SynchronizationRuntimeRegistry,
    owner_ticket: RuntimeStartTicket,
    constructor: impl FnOnce() -> ProductionSynchronization,
) -> Result<LibrarySynchronizationSnapshot, ScanError> {
    let (epoch, cancelled, runtime) =
        match claim_runtime_for_start_with_ticket(registry, owner_ticket)? {
            SynchronizationRuntimeClaim::Poll {
                epoch,
                cancelled,
                runtime,
            } => (epoch, cancelled, runtime),
            SynchronizationRuntimeClaim::Construct { epoch, cancelled } => {
                // Service identity, hashing, WinTrust and platform construction stay outside the
                // registry mutex. The epoch decides whether their late result may be installed.
                let mut candidate = match catch_unwind(AssertUnwindSafe(constructor)) {
                    Ok(candidate) => candidate,
                    Err(_) => {
                        recover_starting_runtime_panic(registry, epoch, owner_ticket)?;
                        return Err(runtime_owner_panic_error());
                    }
                };
                candidate.stop_requested = Arc::clone(&cancelled);
                match promote_constructed_runtime(
                    registry,
                    epoch,
                    owner_ticket,
                    Arc::clone(&cancelled),
                    candidate,
                )? {
                    ConstructedRuntimeDisposition::Poll(runtime) => (epoch, cancelled, runtime),
                    ConstructedRuntimeDisposition::Drain { deadline, runtime } => {
                        finish_draining_runtime(registry, epoch, owner_ticket, deadline, &runtime)?;
                        return Err(ScanError::new(
                            "library_synchronization_start_cancelled",
                            "Synchronization startup completed after its epoch was stopped",
                        ));
                    }
                }
            }
        };
    execute_runtime_poll(registry, epoch, owner_ticket, cancelled, runtime)
}

pub(super) fn execute_runtime_poll(
    registry: &SynchronizationRuntimeRegistry,
    epoch: u64,
    owner_ticket: RuntimeStartTicket,
    cancelled: Arc<AtomicBool>,
    runtime: Arc<Mutex<ProductionSynchronization>>,
) -> Result<LibrarySynchronizationSnapshot, ScanError> {
    execute_runtime_operation(
        registry,
        epoch,
        owner_ticket,
        cancelled,
        runtime,
        poll_runtime,
    )
}

pub(super) fn execute_runtime_operation<T>(
    registry: &SynchronizationRuntimeRegistry,
    epoch: u64,
    owner_ticket: RuntimeStartTicket,
    cancelled: Arc<AtomicBool>,
    runtime: Arc<Mutex<ProductionSynchronization>>,
    operation: impl FnOnce(&mut ProductionSynchronization) -> Result<T, ScanError>,
) -> Result<T, ScanError> {
    let mut runtime_guard = runtime.lock().map_err(|_| {
        ScanError::new(
            "library_synchronization_state_unavailable",
            "The library synchronization runtime is unavailable",
        )
    })?;
    let result = catch_unwind(AssertUnwindSafe(|| operation(&mut runtime_guard)));
    drop(runtime_guard);
    match result {
        Ok(result) => {
            let drain_deadline =
                finish_runtime_operation(registry, epoch, owner_ticket, &cancelled, &runtime)?;
            let Some(deadline) = drain_deadline else {
                return result;
            };
            finish_draining_runtime(registry, epoch, owner_ticket, deadline, &runtime)?;
            Err(ScanError::new(
                "library_synchronization_poll_cancelled",
                "A stopped synchronization epoch discarded its late poll result",
            ))
        }
        Err(_) => {
            #[cfg(test)]
            wait_before_panicked_runtime_retention(registry);
            match retain_panicked_runtime(registry, epoch, owner_ticket, &cancelled, &runtime)? {
                PanickedRuntimeRetention::Retained(deadline) => {
                    let _ =
                        finish_draining_runtime(registry, epoch, owner_ticket, deadline, &runtime);
                }
                PanickedRuntimeRetention::AlreadyDrained => {}
            }
            Err(runtime_owner_panic_error())
        }
    }
}

#[cfg(test)]
pub(super) fn wait_before_panicked_runtime_retention(registry: &SynchronizationRuntimeRegistry) {
    let barrier = registry
        .panic_retention_barrier
        .lock()
        .expect("local panic retention barrier")
        .take();
    if let Some(barrier) = barrier {
        barrier.reached.wait();
        barrier.resume.wait();
    }
}

pub(super) fn finish_runtime_operation(
    registry: &SynchronizationRuntimeRegistry,
    epoch: u64,
    owner_ticket: RuntimeStartTicket,
    cancelled: &Arc<AtomicBool>,
    runtime: &Arc<Mutex<ProductionSynchronization>>,
) -> Result<Option<Instant>, ScanError> {
    let mut registry_guard = lock_runtime_registry(registry)?;
    match &registry_guard.entry {
        SynchronizationRuntimeEntry::Polling {
            epoch: active_epoch,
            owner_ticket: active_owner_ticket,
            runtime: active_runtime,
            ..
        } if *active_epoch == epoch
            && *active_owner_ticket == owner_ticket
            && Arc::ptr_eq(active_runtime, runtime)
            && !cancelled.load(Ordering::Acquire) =>
        {
            registry_guard.entry = SynchronizationRuntimeEntry::Ready {
                epoch,
                owner_ticket,
                cancelled: Arc::clone(cancelled),
                runtime: Arc::clone(runtime),
            };
            drop(registry_guard);
            registry_notify_all(registry);
            Ok(None)
        }
        SynchronizationRuntimeEntry::Draining {
            epoch: active_epoch,
            owner_ticket: active_owner_ticket,
            deadline,
            runtime: active_runtime,
            ..
        } if *active_epoch == epoch
            && *active_owner_ticket == owner_ticket
            && Arc::ptr_eq(active_runtime, runtime) =>
        {
            Ok(Some(*deadline))
        }
        _ => Err(ScanError::new(
            "library_synchronization_owner_state_invalid",
            "The synchronization runtime owner no longer matches its registry epoch",
        )),
    }
}

pub(super) fn retain_panicked_runtime(
    registry: &SynchronizationRuntimeRegistry,
    epoch: u64,
    owner_ticket: RuntimeStartTicket,
    cancelled: &Arc<AtomicBool>,
    runtime: &Arc<Mutex<ProductionSynchronization>>,
) -> Result<PanickedRuntimeRetention, ScanError> {
    cancelled.store(true, Ordering::Release);
    let mut registry_guard = lock_runtime_registry(registry)?;
    let deadline = match &registry_guard.entry {
        SynchronizationRuntimeEntry::Polling {
            epoch: active_epoch,
            owner_ticket: active_owner_ticket,
            cancelled: active_cancelled,
            runtime: active_runtime,
        } if *active_epoch == epoch
            && *active_owner_ticket == owner_ticket
            && Arc::ptr_eq(active_runtime, runtime) =>
        {
            active_cancelled.store(true, Ordering::Release);
            Instant::now()
                .checked_add(SYNCHRONIZATION_STOP_TIMEOUT)
                .ok_or_else(stop_timeout_error)?
        }
        SynchronizationRuntimeEntry::Draining {
            epoch: active_epoch,
            owner_ticket: active_owner_ticket,
            deadline,
            runtime: active_runtime,
            ..
        } if *active_epoch == epoch
            && *active_owner_ticket == owner_ticket
            && Arc::ptr_eq(active_runtime, runtime) =>
        {
            return Ok(PanickedRuntimeRetention::Retained(*deadline));
        }
        _ => {
            let active_epoch = match &registry_guard.entry {
                SynchronizationRuntimeEntry::Empty => None,
                SynchronizationRuntimeEntry::Starting { epoch, .. }
                | SynchronizationRuntimeEntry::Ready { epoch, .. }
                | SynchronizationRuntimeEntry::Polling { epoch, .. }
                | SynchronizationRuntimeEntry::Stopping { epoch, .. }
                | SynchronizationRuntimeEntry::Draining { epoch, .. } => Some(*epoch),
            };
            if active_epoch == Some(epoch) {
                return Err(ScanError::new(
                    "library_synchronization_owner_state_invalid",
                    "A panicked synchronization owner retained an impossible same-epoch state",
                ));
            }
            if registry_guard.retired_through_epoch >= epoch {
                return Ok(PanickedRuntimeRetention::AlreadyDrained);
            }
            return Err(ScanError::new(
                "library_synchronization_owner_state_invalid",
                "A panicked synchronization owner no longer matches its registry epoch",
            ));
        }
    };
    registry_guard.entry = SynchronizationRuntimeEntry::Draining {
        epoch,
        owner_ticket,
        cancellation_fence: None,
        deadline,
        runtime: Arc::clone(runtime),
    };
    drop(registry_guard);
    registry_notify_all(registry);
    Ok(PanickedRuntimeRetention::Retained(deadline))
}

pub(super) fn recover_starting_runtime_panic(
    registry: &SynchronizationRuntimeRegistry,
    epoch: u64,
    owner_ticket: RuntimeStartTicket,
) -> Result<(), ScanError> {
    let mut registry_guard = lock_runtime_registry(registry)?;
    if matches!(
        registry_guard.entry,
        SynchronizationRuntimeEntry::Starting {
            epoch: active_epoch,
            owner_ticket: active_owner_ticket,
            ..
        } | SynchronizationRuntimeEntry::Stopping {
            epoch: active_epoch,
            owner_ticket: active_owner_ticket,
            ..
        } if active_epoch == epoch && active_owner_ticket == owner_ticket
    ) {
        registry_guard.retire_epoch(epoch);
        registry_guard.entry = SynchronizationRuntimeEntry::Empty;
        drop(registry_guard);
        registry_notify_all(registry);
    }
    Ok(())
}

pub(super) fn runtime_owner_panic_error() -> ScanError {
    ScanError::new(
        "library_synchronization_owner_panicked",
        "The library synchronization owner panicked and entered bounded recovery",
    )
}
