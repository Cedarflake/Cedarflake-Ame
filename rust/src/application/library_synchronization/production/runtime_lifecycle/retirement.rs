use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread;
use std::time::{Duration, Instant};

use crate::domain::ScanError;

use super::super::ProductionSynchronization;
#[cfg(test)]
use super::super::SYNCHRONIZATION_STOP_TIMEOUT;
#[cfg(test)]
use super::admission::{reserve_runtime_stop_fence, reserve_runtime_stop_fence_until};
use super::operation::runtime_owner_panic_error;
use super::state::{
    RuntimeStartTicket, RuntimeStopFence, SynchronizationRuntimeEntry,
    SynchronizationRuntimeRegistry, invalid_stop_fence_error, lock_runtime_registry,
    registry_notify_all,
};

pub(super) fn finish_draining_runtime(
    registry: &SynchronizationRuntimeRegistry,
    epoch: u64,
    owner_ticket: RuntimeStartTicket,
    deadline: Instant,
    runtime: &Arc<Mutex<ProductionSynchronization>>,
) -> Result<(), ScanError> {
    {
        let registry_guard = lock_runtime_registry(registry)?;
        if !matches!(
            &registry_guard.entry,
            SynchronizationRuntimeEntry::Draining {
                epoch: active_epoch,
                owner_ticket: active_owner_ticket,
                deadline: active_deadline,
                runtime: active_runtime,
                ..
            } if *active_epoch == epoch
                && *active_owner_ticket == owner_ticket
                && *active_deadline == deadline
                && Arc::ptr_eq(active_runtime, runtime)
        ) {
            return Ok(());
        }
    }
    finish_draining_runtime_after_precheck(registry, epoch, owner_ticket, deadline, runtime)
}

pub(super) fn finish_draining_runtime_after_precheck(
    registry: &SynchronizationRuntimeRegistry,
    epoch: u64,
    owner_ticket: RuntimeStartTicket,
    deadline: Instant,
    runtime: &Arc<Mutex<ProductionSynchronization>>,
) -> Result<(), ScanError> {
    let mut runtime_guard = lock_draining_runtime_until(runtime, deadline)?;
    {
        let registry_guard = lock_runtime_registry(registry)?;
        if !matches!(
            &registry_guard.entry,
            SynchronizationRuntimeEntry::Draining {
                epoch: active_epoch,
                owner_ticket: active_owner_ticket,
                deadline: active_deadline,
                runtime: active_runtime,
                ..
            } if *active_epoch == epoch
                && *active_owner_ticket == owner_ticket
                && *active_deadline == deadline
                && Arc::ptr_eq(active_runtime, runtime)
        ) {
            return Ok(());
        }
    }
    let result = catch_unwind(AssertUnwindSafe(|| {
        runtime_guard.finish_stopping_until(deadline)
    }));
    match result {
        Ok(Ok(())) => {
            let mut registry_guard = lock_runtime_registry(registry)?;
            if matches!(
                &registry_guard.entry,
                SynchronizationRuntimeEntry::Draining {
                    epoch: active_epoch,
                    owner_ticket: active_owner_ticket,
                    deadline: active_deadline,
                    runtime: active_runtime,
                    ..
                } if *active_epoch == epoch
                    && *active_owner_ticket == owner_ticket
                    && *active_deadline == deadline
                    && Arc::ptr_eq(active_runtime, runtime)
            ) {
                registry_guard.retire_epoch(epoch);
                registry_guard.entry = SynchronizationRuntimeEntry::Empty;
            }
            drop(registry_guard);
            drop(runtime_guard);
            registry_notify_all(registry);
            Ok(())
        }
        Ok(Err(error)) => {
            drop(runtime_guard);
            registry_notify_all(registry);
            Err(error)
        }
        Err(_) => {
            drop(runtime_guard);
            registry_notify_all(registry);
            Err(runtime_owner_panic_error())
        }
    }
}

pub(super) fn lock_draining_runtime_until<'a>(
    runtime: &'a Mutex<ProductionSynchronization>,
    deadline: Instant,
) -> Result<MutexGuard<'a, ProductionSynchronization>, ScanError> {
    loop {
        match runtime.try_lock() {
            Ok(runtime) => return Ok(runtime),
            Err(std::sync::TryLockError::Poisoned(_)) => {
                return Err(ScanError::new(
                    "library_synchronization_state_unavailable",
                    "The draining synchronization runtime is unavailable",
                ));
            }
            Err(std::sync::TryLockError::WouldBlock) => {
                let remaining = deadline
                    .checked_duration_since(Instant::now())
                    .ok_or_else(stop_timeout_error)?;
                thread::sleep(remaining.min(Duration::from_millis(2)));
            }
        }
    }
}

#[cfg(test)]
pub(super) fn stop_runtime(registry: &SynchronizationRuntimeRegistry) -> Result<(), ScanError> {
    let cancellation_fence = reserve_runtime_stop_fence(registry, SYNCHRONIZATION_STOP_TIMEOUT)?;
    stop_runtime_with_fence(registry, cancellation_fence)
}

#[cfg(test)]
pub(super) fn stop_runtime_with_timeout(
    registry: &SynchronizationRuntimeRegistry,
    timeout: Duration,
) -> Result<(), ScanError> {
    let cancellation_fence = reserve_runtime_stop_fence(registry, timeout)?;
    stop_runtime_with_fence(registry, cancellation_fence)
}

#[cfg(test)]
pub(super) fn stop_runtime_until(
    registry: &SynchronizationRuntimeRegistry,
    requested_deadline: Instant,
) -> Result<(), ScanError> {
    let cancellation_fence = reserve_runtime_stop_fence_until(registry, requested_deadline)?;
    stop_runtime_with_fence(registry, cancellation_fence)
}

pub(super) fn stop_runtime_with_fence(
    registry: &SynchronizationRuntimeRegistry,
    cancellation_fence: RuntimeStopFence,
) -> Result<(), ScanError> {
    lock_runtime_registry(registry)?.consume_admitted_stop_fence(cancellation_fence)?;
    loop {
        let draining = {
            let registry_guard = lock_runtime_registry(registry)?;
            if cancellation_fence.ordinal() > registry_guard.next_lifecycle_ordinal
                || cancellation_fence.ordinal() > registry_guard.cancelled_through_ordinal
            {
                return Err(invalid_stop_fence_error());
            }
            match &registry_guard.entry {
                SynchronizationRuntimeEntry::Empty => return Ok(()),
                entry
                    if entry
                        .owner_ticket()
                        .is_some_and(|owner| owner.ordinal() > cancellation_fence.ordinal()) =>
                {
                    return Ok(());
                }
                SynchronizationRuntimeEntry::Draining {
                    epoch,
                    owner_ticket,
                    deadline,
                    runtime,
                    ..
                } => Some((*epoch, *owner_ticket, *deadline, Arc::clone(runtime))),
                SynchronizationRuntimeEntry::Stopping { .. } => None,
                SynchronizationRuntimeEntry::Starting { .. }
                | SynchronizationRuntimeEntry::Ready { .. }
                | SynchronizationRuntimeEntry::Polling { .. } => {
                    return Err(ScanError::new(
                        "library_synchronization_owner_state_invalid",
                        "The synchronization stop fence did not linearize its matching owner",
                    ));
                }
            }
        };
        if let Some((epoch, owner_ticket, deadline, runtime)) = draining {
            return finish_draining_runtime(registry, epoch, owner_ticket, deadline, &runtime);
        }

        let registry_guard = lock_runtime_registry(registry)?;
        if matches!(registry_guard.entry, SynchronizationRuntimeEntry::Empty)
            || registry_guard
                .entry
                .owner_ticket()
                .is_some_and(|owner| owner.ordinal() > cancellation_fence.ordinal())
        {
            return Ok(());
        }
        let deadline = match registry_guard.entry {
            SynchronizationRuntimeEntry::Stopping { deadline, .. }
            | SynchronizationRuntimeEntry::Draining { deadline, .. } => deadline,
            SynchronizationRuntimeEntry::Empty => return Ok(()),
            _ => {
                return Err(ScanError::new(
                    "library_synchronization_owner_state_invalid",
                    "The synchronization stop fence lost its admitted owner",
                ));
            }
        };
        let remaining = deadline
            .checked_duration_since(Instant::now())
            .ok_or_else(stop_timeout_error)?;
        let (registry_guard, wait) = registry
            .changed
            .wait_timeout(registry_guard, remaining)
            .map_err(|_| {
                ScanError::new(
                    "library_synchronization_state_unavailable",
                    "The library synchronization runtime state is unavailable",
                )
            })?;
        if wait.timed_out()
            && !matches!(registry_guard.entry, SynchronizationRuntimeEntry::Empty)
            && !registry_guard
                .entry
                .owner_ticket()
                .is_some_and(|owner| owner.ordinal() > cancellation_fence.ordinal())
        {
            return Err(stop_timeout_error());
        }
    }
}

pub(super) fn stop_timeout_error() -> ScanError {
    ScanError::new(
        "library_synchronization_stop_timeout",
        "Synchronization workers did not stop within the bounded shutdown window",
    )
}
