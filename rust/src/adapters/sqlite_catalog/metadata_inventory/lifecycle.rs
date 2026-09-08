use rusqlite::{Connection, TransactionBehavior, params};

use crate::domain::{
    MetadataInventoryCleanupReport, MetadataInventoryRun, MetadataInventoryRunRequest,
    MetadataInventoryRunStatus, MetadataInventoryStartRequest, ScanError,
};

use super::{
    MAX_CLEANUP_RUNS, MAX_PAGE_ENTRIES, SqliteCatalog, begin_metadata_inventory_transaction,
    database_error, load_run, sqlite_integer, sqlite_unsigned, validate_active_root,
    validate_start_request,
};

#[cfg(test)]
mod tests;

pub(super) fn begin_next(
    catalog: &mut SqliteCatalog,
    request: &MetadataInventoryStartRequest,
) -> Result<MetadataInventoryRun, ScanError> {
    validate_start_request(request)?;
    require_standalone_connection(&catalog.connection)?;
    let existing = {
        // Run, frontier, and active-root evidence must belong to the same read snapshot.
        let read = catalog
            .connection
            .transaction_with_behavior(TransactionBehavior::Deferred)
            .map_err(database_error)?;
        let existing = select_existing(&read, request)?;
        read.commit().map_err(database_error)?;
        existing
    };
    if let Some(existing) = existing {
        return Ok(existing);
    }
    // The read snapshot ends before admission; only this transaction may allocate an epoch.
    #[cfg(test)]
    tests::before_write(tests::WriteOperation::BeginNext);
    let transaction = catalog.begin_write()?;
    if let Some(existing) = select_existing(&transaction, request)? {
        transaction.commit().map_err(database_error)?;
        return Ok(existing);
    }
    let latest_epoch = transaction
        .query_row(
            "SELECT MAX(epoch)
             FROM library_metadata_inventory_runs
             WHERE root_id = ?1 AND root_generation = ?2",
            params![
                request.root_id,
                sqlite_integer(
                    request.root_generation.value(),
                    "metadata inventory root generation",
                )?,
            ],
            |row| row.get::<_, Option<i64>>(0),
        )
        .map_err(database_error)?
        .map(|epoch| sqlite_unsigned(epoch, "metadata inventory epoch"))
        .transpose()?
        .unwrap_or(0);
    let epoch = latest_epoch.checked_add(1).ok_or_else(|| {
        ScanError::new(
            "metadata_inventory_epoch_overflow",
            "The metadata inventory epoch exceeded the supported range",
        )
    })?;
    let run_request = MetadataInventoryRunRequest {
        run_id: request.run_id.clone(),
        root_id: request.root_id.clone(),
        root_generation: request.root_generation,
        epoch,
        scope: request.scope.clone(),
        started_unix_ms: request.started_unix_ms,
    };
    let run = begin_metadata_inventory_transaction(&transaction, &run_request)?;
    transaction.commit().map_err(database_error)?;
    Ok(run)
}

fn select_existing(
    connection: &Connection,
    request: &MetadataInventoryStartRequest,
) -> Result<Option<MetadataInventoryRun>, ScanError> {
    let Some(existing) = load_run(connection, &request.run_id)? else {
        return Ok(None);
    };
    if !matches!(
        existing.status,
        MetadataInventoryRunStatus::Running | MetadataInventoryRunStatus::Comparing
    ) || existing.request.root_id != request.root_id
        || existing.request.root_generation != request.root_generation
        || existing.request.scope != request.scope
    {
        return Err(ScanError::new(
            "metadata_inventory_run_duplicate",
            "The metadata inventory run identity already exists",
        ));
    }
    validate_active_root(connection, &existing.request)?;
    Ok(Some(existing))
}

pub(super) fn cleanup_terminal(
    catalog: &mut SqliteCatalog,
    terminal_before_unix_ms: i64,
    entry_limit: u32,
    run_limit: u32,
) -> Result<MetadataInventoryCleanupReport, ScanError> {
    if entry_limit == 0
        || entry_limit > MAX_PAGE_ENTRIES
        || run_limit == 0
        || run_limit > MAX_CLEANUP_RUNS
    {
        return Err(ScanError::new(
            "metadata_inventory_cleanup_limit_invalid",
            "Metadata inventory cleanup limits exceed the bounded contract",
        ));
    }
    require_standalone_connection(&catalog.connection)?;
    // A single read statement is only an admission hint, never deletion authority.
    if !has_cleanup_candidates(&catalog.connection, terminal_before_unix_ms)? {
        return Ok(MetadataInventoryCleanupReport::default());
    }
    #[cfg(test)]
    tests::before_write(tests::WriteOperation::CleanupTerminal);
    let transaction = catalog.begin_write()?;
    if !has_cleanup_candidates(&transaction, terminal_before_unix_ms)? {
        transaction.commit().map_err(database_error)?;
        return Ok(MetadataInventoryCleanupReport::default());
    }
    let removed_entry_count = transaction
        .execute(
            "DELETE FROM library_metadata_inventory_entries
             WHERE rowid IN (
               SELECT entries.rowid
               FROM library_metadata_inventory_entries AS entries
               JOIN library_metadata_inventory_runs AS runs ON runs.id = entries.run_id
               WHERE runs.status IN ('completed', 'failed', 'cancelled', 'superseded')
                 AND NOT EXISTS (
                   SELECT 1 FROM library_metadata_inventory_spools AS spool
                   WHERE spool.run_id = runs.id
                 )
               ORDER BY runs.updated_unix_ms, runs.id, entries.relative_path
               LIMIT ?1
             )",
            [i64::from(entry_limit)],
        )
        .map_err(database_error)?;
    let removed_run_count = transaction
        .execute(
            "DELETE FROM library_metadata_inventory_runs
             WHERE id IN (
               SELECT runs.id
               FROM library_metadata_inventory_runs AS runs
               WHERE runs.status IN ('completed', 'failed', 'cancelled', 'superseded')
                 AND runs.updated_unix_ms < ?1
                 AND NOT EXISTS (
                   SELECT 1 FROM library_metadata_inventory_spools AS spool
                   WHERE spool.run_id = runs.id
                 )
                 AND NOT EXISTS(
                   SELECT 1 FROM library_metadata_inventory_entries AS entries
                   WHERE entries.run_id = runs.id
                 )
               ORDER BY runs.updated_unix_ms, runs.id
               LIMIT ?2
             )",
            params![terminal_before_unix_ms, i64::from(run_limit)],
        )
        .map_err(database_error)?;
    let has_more = has_cleanup_candidates(&transaction, terminal_before_unix_ms)?;
    transaction.commit().map_err(database_error)?;
    Ok(MetadataInventoryCleanupReport {
        removed_entry_count: u32::try_from(removed_entry_count).map_err(|_| {
            ScanError::new(
                "metadata_inventory_cleanup_count_overflow",
                "Metadata inventory entry cleanup count overflowed",
            )
        })?,
        removed_run_count: u32::try_from(removed_run_count).map_err(|_| {
            ScanError::new(
                "metadata_inventory_cleanup_count_overflow",
                "Metadata inventory run cleanup count overflowed",
            )
        })?,
        has_more,
    })
}

fn has_cleanup_candidates(
    connection: &Connection,
    terminal_before_unix_ms: i64,
) -> Result<bool, ScanError> {
    connection
        .query_row(
            "SELECT
               EXISTS(
                 SELECT 1
                 FROM library_metadata_inventory_entries AS entries
                 JOIN library_metadata_inventory_runs AS runs ON runs.id = entries.run_id
                 WHERE runs.status IN ('completed', 'failed', 'cancelled', 'superseded')
                   AND NOT EXISTS (
                     SELECT 1 FROM library_metadata_inventory_spools AS spool
                     WHERE spool.run_id = runs.id
                   )
               )
               OR EXISTS(
                 SELECT 1
                 FROM library_metadata_inventory_runs AS runs
                 WHERE runs.status IN ('completed', 'failed', 'cancelled', 'superseded')
                   AND runs.updated_unix_ms < ?1
                   AND NOT EXISTS (
                     SELECT 1 FROM library_metadata_inventory_spools AS spool
                     WHERE spool.run_id = runs.id
                   )
                   AND NOT EXISTS(
                     SELECT 1 FROM library_metadata_inventory_entries AS entries
                     WHERE entries.run_id = runs.id
                   )
               )",
            [terminal_before_unix_ms],
            |row| row.get(0),
        )
        .map_err(database_error)
}

fn require_standalone_connection(connection: &Connection) -> Result<(), ScanError> {
    if !connection.is_autocommit() {
        return Err(ScanError::new(
            "metadata_inventory_lifecycle_transaction_active",
            "Metadata inventory lifecycle operations require a standalone catalog connection",
        ));
    }
    Ok(())
}
