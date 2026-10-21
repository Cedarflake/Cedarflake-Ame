use rusqlite::{Transaction, params};

use crate::domain::{
    LibraryChangeId, LibraryChangeLane, LibraryChangeLeaseUpdateOutcome, ScanError,
};

use super::super::{SqliteCatalog, database_error, sqlite_integer};
use super::{
    admission_lane_for_change_ids, classify_lease_update, load_change,
    wake_metadata_inventory_capacity_deferrals,
};

pub(super) fn complete(
    catalog: &mut SqliteCatalog,
    change_id: LibraryChangeId,
    lease_generation: u64,
    catalog_revision_at_success: u64,
    completed_unix_ms: i64,
) -> Result<LibraryChangeLeaseUpdateOutcome, ScanError> {
    let lane = admission_lane_for_change_ids(&catalog.connection, [change_id])?;
    let transaction = catalog.begin_write_in_lane(lane)?;
    let outcome = classify_lease_update(
        &transaction,
        change_id,
        lease_generation,
        Some(catalog_revision_at_success),
    )?;
    if outcome == LibraryChangeLeaseUpdateOutcome::Applied {
        let completed_change = load_change(
            &transaction,
            sqlite_integer(change_id.value(), "change ID")?,
        )?;
        transaction
            .execute(
                "UPDATE library_change_queue
                 SET status = 'completed', lease_expires_unix_ms = NULL,
                     catalog_revision_at_success = ?1, updated_unix_ms = ?2
                 WHERE id = ?3 AND status = 'leased' AND lease_generation = ?4",
                params![
                    sqlite_integer(catalog_revision_at_success, "catalog revision")?,
                    completed_unix_ms,
                    sqlite_integer(change_id.value(), "change ID")?,
                    sqlite_integer(lease_generation, "lease generation")?,
                ],
            )
            .map_err(database_error)?;
        if matches!(
            lane,
            LibraryChangeLane::Journal | LibraryChangeLane::Recovery
        ) {
            wake_metadata_inventory_capacity_deferrals(
                &transaction,
                &completed_change.intent.root_id,
                completed_change.intent.root_generation,
                completed_unix_ms,
            )?;
        }
        release_handoffs(&transaction, change_id)?;
    }
    transaction.commit().map_err(database_error)?;
    Ok(outcome)
}

pub(in super::super) fn release_handoffs(
    transaction: &Transaction<'_>,
    change_id: LibraryChangeId,
) -> Result<(), ScanError> {
    let evidence = {
        let mut statement = transaction
            .prepare_cached(
                "SELECT catch_up_source, catch_up_watermark
             FROM library_change_queue_catch_up_lineage WHERE change_id = ?1",
            )
            .map_err(database_error)?;
        statement
            .query_map([sqlite_integer(change_id.value(), "change ID")?], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(database_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(database_error)?
    };
    super::super::catalog_delta::cleanup_terminal_catch_up_handoffs_batch(transaction, &evidence)
}
