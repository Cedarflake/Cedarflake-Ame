use std::path::Path;

use crate::domain::{LibraryChangeLane, ScanError};
use crate::ports::RetainedScanRepository;

use super::execution_registry::reserve_retained_cancellation;

#[cfg(test)]
mod tests;

pub fn cancel_retained_scan(scan_id: &str) -> Result<(), ScanError> {
    let storage = super::storage_paths()?;
    cancel_retained_scan_at_path(scan_id, &storage.catalog_path)
}

fn cancel_retained_scan_at_path(scan_id: &str, catalog_path: &Path) -> Result<(), ScanError> {
    let _reservation = reserve_retained_cancellation(scan_id, catalog_path)?;
    let mut catalog = crate::application::catalog_session::open_catalog_for_foreground_scan(
        catalog_path,
        LibraryChangeLane::Recovery,
        scan_id,
    )?;
    catalog.cancel_retained_scan(scan_id)
}
