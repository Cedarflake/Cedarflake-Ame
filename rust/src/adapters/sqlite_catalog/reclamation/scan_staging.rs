use rusqlite::TransactionBehavior;

use crate::domain::ScanError;
use crate::ports::{CatalogMaintenanceAttempt, CatalogMaintenanceControl};

use super::super::scan_staging::discard_next_retired_batch;
use super::{SqliteCatalogSpaceMaintenance, database_error, is_busy};

pub(super) fn discard_retired_batch(
    maintenance: &SqliteCatalogSpaceMaintenance,
    control: &CatalogMaintenanceControl,
) -> Result<CatalogMaintenanceAttempt<usize>, ScanError> {
    let mut writer = match maintenance.try_open_write(control)? {
        CatalogMaintenanceAttempt::Completed(writer) => writer,
        CatalogMaintenanceAttempt::Busy => return Ok(CatalogMaintenanceAttempt::Busy),
        CatalogMaintenanceAttempt::Interrupted => {
            return Ok(CatalogMaintenanceAttempt::Interrupted);
        }
    };
    let transaction = match writer
        .connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
    {
        Ok(transaction) => transaction,
        Err(error) if is_busy(&error) => return Ok(CatalogMaintenanceAttempt::Busy),
        Err(error) => return Err(database_error(error)),
    };
    if control.is_interrupted() {
        return Ok(CatalogMaintenanceAttempt::Interrupted);
    }
    let discarded = discard_next_retired_batch(&transaction)?;
    // Each small batch owns its complete deletion and orphan cleanup. A foreground writer
    // can preempt between batches without committing a half-cleaned set of locations.
    if control.is_interrupted() {
        return Ok(CatalogMaintenanceAttempt::Interrupted);
    }
    transaction.commit().map_err(database_error)?;
    Ok(CatalogMaintenanceAttempt::Completed(discarded))
}
