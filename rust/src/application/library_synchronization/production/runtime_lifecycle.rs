use crate::domain::{LibrarySynchronizationSnapshot, ScanError};

use super::{SYNCHRONIZATION_STOP_TIMEOUT, new_production_synchronization};
use state::{RuntimeStartTicket, RuntimeStopFence, runtime_registry};

mod admission;
mod operation;
mod retirement;
mod state;

pub(super) fn reserve_start_ticket() -> Result<u64, ScanError> {
    admission::reserve_runtime_start_ticket(runtime_registry()).map(RuntimeStartTicket::raw)
}

pub(super) fn reserve_stop_fence() -> Result<u64, ScanError> {
    admission::reserve_runtime_stop_fence(runtime_registry(), SYNCHRONIZATION_STOP_TIMEOUT)
        .map(RuntimeStopFence::raw)
}

pub(super) fn start(owner_ticket: u64) -> Result<LibrarySynchronizationSnapshot, ScanError> {
    let owner_ticket = RuntimeStartTicket::decode(owner_ticket)?;
    operation::start_runtime_with_ticket(
        runtime_registry(),
        owner_ticket,
        new_production_synchronization,
    )
}

pub(super) fn poll(owner_ticket: u64) -> Result<LibrarySynchronizationSnapshot, ScanError> {
    let owner_ticket = RuntimeStartTicket::decode(owner_ticket)?;
    let (epoch, cancelled, runtime) =
        admission::claim_runtime_for_poll_with_ticket(runtime_registry(), owner_ticket)?;
    operation::execute_runtime_poll(runtime_registry(), epoch, owner_ticket, cancelled, runtime)
}

#[cfg(not(test))]
pub(super) fn first_import_owner_ticket() -> Result<Option<u64>, ScanError> {
    admission::first_import_runtime_owner_for_poll(runtime_registry())
        .map(|ticket| ticket.map(RuntimeStartTicket::raw))
}

pub(super) fn stop(cancellation_fence: u64) -> Result<(), ScanError> {
    let cancellation_fence = RuntimeStopFence::decode(cancellation_fence)?;
    retirement::stop_runtime_with_fence(runtime_registry(), cancellation_fence)
}

#[cfg(test)]
mod tests;
