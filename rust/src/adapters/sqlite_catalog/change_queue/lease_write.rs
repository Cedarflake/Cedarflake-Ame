use std::time::Duration;

use rusqlite::{Connection, Transaction, TransactionBehavior};

use super::super::operation_diagnostics::measure;
use super::super::{SQLITE_BUSY_TIMEOUT, SqliteCatalog, database_error};
use super::{LeasedLibraryChange, LibraryChangeLane, ScanError, lease_matching::LeaseSelection};

const RECOVERY_LEASE_ADMISSION_TIMEOUT: Duration = Duration::from_millis(5);

pub(super) fn lease_in_transaction(
    catalog: &mut SqliteCatalog,
    selection: LeaseSelection,
    select_lane: impl FnOnce(&Connection) -> Result<Option<LibraryChangeLane>, ScanError>,
    lease: impl FnOnce(&Transaction<'_>) -> Result<Vec<LeasedLibraryChange>, ScanError>,
) -> Result<Vec<LeasedLibraryChange>, ScanError> {
    if matches!(selection, LeaseSelection::RecoveryAuthoritative) {
        return try_recovery_lease(catalog, select_lane, lease);
    }
    let Some(lane) = select_lane(&catalog.connection)? else {
        return Ok(Vec::new());
    };
    let transaction = catalog.begin_write_in_lane(lane)?;
    let leased = lease(&transaction)?;
    transaction.commit().map_err(database_error)?;
    Ok(leased)
}

fn try_recovery_lease(
    catalog: &mut SqliteCatalog,
    select_lane: impl FnOnce(&Connection) -> Result<Option<LibraryChangeLane>, ScanError>,
    lease: impl FnOnce(&Transaction<'_>) -> Result<Vec<LeasedLibraryChange>, ScanError>,
) -> Result<Vec<LeasedLibraryChange>, ScanError> {
    // Readiness reads share the zero-wait boundary; they can contend before writer admission.
    let restore = RestoreBusyTimeout {
        connection: &mut catalog.connection,
    };
    restore
        .connection
        .busy_timeout(Duration::ZERO)
        .map_err(database_error)?;
    let result: Result<Vec<LeasedLibraryChange>, ScanError> = (|| {
        let Some(lane) = measure("recovery_lease_selection", || {
            select_lane(restore.connection)
        })?
        else {
            return Ok(Vec::new());
        };
        // Retain FIFO ordering briefly without holding a status poll behind a long publication.
        let Some(_permit) = measure("recovery_lease_admission", || {
            catalog
                .write_admission
                .acquire_for(lane, RECOVERY_LEASE_ADMISSION_TIMEOUT)
        }) else {
            return Ok(Vec::new());
        };
        let transaction = measure("recovery_lease_begin", || {
            restore
                .connection
                .transaction_with_behavior(TransactionBehavior::Immediate)
        })
        .map_err(database_error)?;
        let leased = measure("recovery_lease_update", || lease(&transaction))?;
        measure("recovery_lease_commit", || transaction.commit()).map_err(database_error)?;
        Ok(leased)
    })();
    match result {
        Err(error)
            if matches!(
                error.code.as_str(),
                "catalog_database_busy" | "catalog_database_locked"
            ) =>
        {
            Ok(Vec::new())
        }
        result => result,
    }
}

struct RestoreBusyTimeout<'connection> {
    connection: &'connection mut Connection,
}

impl Drop for RestoreBusyTimeout<'_> {
    fn drop(&mut self) {
        let _ = self.connection.busy_timeout(SQLITE_BUSY_TIMEOUT);
    }
}
