use crate::application::{
    poll_production_library_synchronization,
    reserve_production_library_synchronization_start_ticket,
    reserve_production_library_synchronization_stop_fence,
    start_production_library_synchronization, stop_production_library_synchronization,
};
use crate::domain::{LibrarySynchronizationSnapshot, ScanError};

#[flutter_rust_bridge::frb(sync)]
pub fn reserve_library_synchronization_start_ticket() -> Result<u64, ScanError> {
    reserve_production_library_synchronization_start_ticket()
}

#[flutter_rust_bridge::frb(sync)]
pub fn reserve_library_synchronization_stop_fence() -> Result<u64, ScanError> {
    reserve_production_library_synchronization_stop_fence()
}

pub fn start_library_synchronization(
    owner_ticket: u64,
) -> Result<LibrarySynchronizationSnapshot, ScanError> {
    start_production_library_synchronization(owner_ticket)
}

pub fn poll_library_synchronization(
    owner_ticket: u64,
) -> Result<LibrarySynchronizationSnapshot, ScanError> {
    poll_production_library_synchronization(owner_ticket)
}

pub fn stop_library_synchronization(cancellation_fence: u64) -> Result<(), ScanError> {
    stop_production_library_synchronization(cancellation_fence)
}
