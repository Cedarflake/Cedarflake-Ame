use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::Duration;

use rusqlite::{Connection, Transaction, TransactionBehavior};

use crate::domain::{MetadataInventoryCleanupReport, ScanError};
use crate::ports::InventoryCleanupControl;

use super::super::{SQLITE_BUSY_TIMEOUT, SqliteCatalog, database_error};

#[cfg(test)]
mod tests;

struct RestoreBusyTimeout<'connection> {
    connection: &'connection mut Connection,
}

impl Drop for RestoreBusyTimeout<'_> {
    fn drop(&mut self) {
        let _ = self.connection.busy_timeout(SQLITE_BUSY_TIMEOUT);
    }
}

struct RestoreProgress<'connection> {
    connection: &'connection Connection,
}

impl Drop for RestoreProgress<'_> {
    fn drop(&mut self) {
        let _ = self.connection.progress_handler(0, None::<fn() -> bool>);
    }
}

pub(super) fn try_cleanup_write(
    catalog: &mut SqliteCatalog,
    control: InventoryCleanupControl,
    cleanup: impl FnOnce(&Transaction<'_>) -> Result<MetadataInventoryCleanupReport, ScanError>,
) -> Result<MetadataInventoryCleanupReport, ScanError> {
    if control.is_cancelled() {
        return Err(cancelled());
    }
    let preempted = Arc::new(AtomicBool::new(false));
    let signal = preempted.clone();
    let _permit = catalog
        .write_admission
        .try_acquire_preemptible_maintenance(Arc::new(move || {
            signal.store(true, Ordering::Release)
        }))
        .ok_or_else(deferred)?;
    let restore = RestoreBusyTimeout {
        connection: &mut catalog.connection,
    };
    restore
        .connection
        .busy_timeout(Duration::ZERO)
        .map_err(database_error)?;
    if let Some(error) = interruption(&control, &preempted) {
        return Err(error);
    }
    let transaction = restore
        .connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(database_error)?;
    // Declared after the transaction: even unwind clears interruption before rollback.
    let progress = RestoreProgress {
        connection: &transaction,
    };
    let signal = preempted.clone();
    let cancellation = control.clone();
    transaction
        .progress_handler(
            1_000,
            Some(move || cancellation.is_cancelled() || signal.load(Ordering::Acquire)),
        )
        .map_err(database_error)?;
    let result = cleanup(&transaction);
    drop(progress);
    match result {
        Err(error) if error.code == "catalog_database_interrupted" => {
            Err(interruption(&control, &preempted).unwrap_or(error))
        }
        Err(error) => Err(error),
        Ok(report) => {
            if let Some(error) = interruption(&control, &preempted) {
                return Err(error);
            }
            transaction.commit().map_err(database_error)?;
            Ok(report)
        }
    }
}

pub(super) fn read_candidates(
    connection: &Connection,
    control: &InventoryCleanupControl,
    read: impl FnOnce(&Connection) -> Result<bool, ScanError>,
) -> Result<bool, ScanError> {
    if control.is_cancelled() {
        return Err(cancelled());
    }
    let _progress = RestoreProgress { connection };
    let cancellation = control.clone();
    connection
        .progress_handler(1_000, Some(move || cancellation.is_cancelled()))
        .map_err(database_error)?;
    match read(connection) {
        Err(error) if error.code == "catalog_database_interrupted" && control.is_cancelled() => {
            Err(cancelled())
        }
        Err(error) => Err(error),
        Ok(_) if control.is_cancelled() => Err(cancelled()),
        Ok(result) => Ok(result),
    }
}

fn interruption(control: &InventoryCleanupControl, preempted: &AtomicBool) -> Option<ScanError> {
    if control.is_cancelled() {
        Some(cancelled())
    } else {
        preempted.load(Ordering::Acquire).then(deferred)
    }
}

fn cancelled() -> ScanError {
    ScanError::new(
        "metadata_inventory_cleanup_cancelled",
        "Inventory cleanup was cancelled",
    )
}

fn deferred() -> ScanError {
    ScanError::new(
        "catalog_database_busy",
        "Inventory cleanup yielded to active catalog work",
    )
}
