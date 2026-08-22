use crate::application::{
    poll_production_library_synchronization, start_production_library_synchronization,
    stop_production_library_synchronization,
};
use crate::domain::{LibrarySynchronizationSnapshot, ScanError};

pub fn start_library_synchronization() -> Result<LibrarySynchronizationSnapshot, ScanError> {
    start_production_library_synchronization()
}

pub fn poll_library_synchronization() -> Result<LibrarySynchronizationSnapshot, ScanError> {
    poll_production_library_synchronization()
}

pub fn stop_library_synchronization() -> Result<(), ScanError> {
    stop_production_library_synchronization()
}
