use std::time::Duration;

use rusqlite::{Connection, Transaction, TransactionBehavior};

use super::super::{SQLITE_BUSY_TIMEOUT, SqliteCatalog, database_error};
use super::{LeasedLibraryChange, LibraryChangeLane, ScanError, lease_matching::LeaseSelection};

const RECOVERY_LEASE_ADMISSION_TIMEOUT: Duration = Duration::from_millis(5);

pub(super) fn lease_in_transaction(
    catalog: &mut SqliteCatalog,
    selection: LeaseSelection,
    lane: LibraryChangeLane,
    lease: impl FnOnce(&Transaction<'_>) -> Result<Vec<LeasedLibraryChange>, ScanError>,
) -> Result<Vec<LeasedLibraryChange>, ScanError> {
    if matches!(selection, LeaseSelection::RecoveryAuthoritative) {
        return try_recovery_lease(catalog, lane, lease);
    }
    let transaction = catalog.begin_write_in_lane(lane)?;
    let leased = lease(&transaction)?;
    transaction.commit().map_err(database_error)?;
    Ok(leased)
}

fn try_recovery_lease(
    catalog: &mut SqliteCatalog,
    lane: LibraryChangeLane,
    lease: impl FnOnce(&Transaction<'_>) -> Result<Vec<LeasedLibraryChange>, ScanError>,
) -> Result<Vec<LeasedLibraryChange>, ScanError> {
    // Keep a FIFO position briefly so repeated short writers cannot starve recovery dispatch.
    // A long publication still yields this status poll with the durable task ready to continue.
    let Some(_permit) = catalog
        .write_admission
        .acquire_for(lane, RECOVERY_LEASE_ADMISSION_TIMEOUT)
    else {
        return Ok(Vec::new());
    };
    let restore = RestoreBusyTimeout {
        connection: &mut catalog.connection,
    };
    restore
        .connection
        .busy_timeout(Duration::ZERO)
        .map_err(database_error)?;
    let result: Result<Vec<LeasedLibraryChange>, ScanError> = (|| {
        let transaction = restore
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(database_error)?;
        let leased = lease(&transaction)?;
        transaction.commit().map_err(database_error)?;
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
