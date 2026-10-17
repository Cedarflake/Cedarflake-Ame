use rusqlite::{Connection, OptionalExtension, TransactionBehavior};

use crate::domain::ScanError;

use super::database_error;

pub(super) fn validate_recovery_marker(connection: &Connection) -> Result<(), ScanError> {
    validate_recovery_base_marker(connection)?;
    let has_scan_ownership_contract = connection
        .query_row(
            "SELECT EXISTS(
               SELECT 1 FROM pragma_table_info('library_change_queue_contract')
               WHERE name = 'scan_ownership_complete'
             )",
            [],
            |row| row.get::<_, bool>(0),
        )
        .map_err(database_error)?;
    if !has_scan_ownership_contract {
        return Err(unverifiable_recovery_contract());
    }
    let ownership_complete = connection
        .query_row(
            "SELECT scan_ownership_complete = 1
             FROM library_change_queue_contract WHERE singleton = 1",
            [],
            |row| row.get::<_, bool>(0),
        )
        .optional()
        .map_err(database_error)?
        .unwrap_or(false);
    if !ownership_complete {
        return Err(unverifiable_recovery_contract());
    }
    Ok(())
}

pub(super) fn validate_recovery_base_marker(connection: &Connection) -> Result<(), ScanError> {
    let has_recovery_contract = connection
        .query_row(
            "SELECT EXISTS(
               SELECT 1 FROM pragma_table_info('library_change_queue_contract')
               WHERE name = 'authoritative_recovery_complete'
             )",
            [],
            |row| row.get::<_, bool>(0),
        )
        .map_err(database_error)?;
    if !has_recovery_contract {
        return Err(unverifiable_recovery_contract());
    }
    let recovery_complete = connection
        .query_row(
            "SELECT authoritative_recovery_complete = 1
             FROM library_change_queue_contract WHERE singleton = 1",
            [],
            |row| row.get::<_, bool>(0),
        )
        .optional()
        .map_err(database_error)?
        .unwrap_or(false);
    if !recovery_complete {
        return Err(unverifiable_recovery_contract());
    }
    Ok(())
}

pub(super) fn repair(connection: &mut Connection) -> Result<(), ScanError> {
    validate_queue_authority(connection)?;
    validate_recovery_base_marker(connection)?;
    let (has_scan_runs, has_single_scan_owner_index, has_scan_owner, has_ownership_marker) =
        connection
            .query_row(
                "SELECT
               EXISTS(SELECT 1 FROM sqlite_master
                 WHERE type = 'table' AND name = 'scan_runs'),
               EXISTS(SELECT 1 FROM pragma_index_list('scan_runs')
                 WHERE name = 'scan_runs_one_active_root'
                   AND \"unique\" = 1 AND partial = 1),
               EXISTS(SELECT 1 FROM pragma_table_info('scan_runs')
                 WHERE name = 'scan_owner'),
               EXISTS(SELECT 1 FROM pragma_table_info('library_change_queue_contract')
                 WHERE name = 'scan_ownership_complete')",
                [],
                |row| {
                    Ok((
                        row.get::<_, bool>(0)?,
                        row.get::<_, bool>(1)?,
                        row.get::<_, bool>(2)?,
                        row.get::<_, bool>(3)?,
                    ))
                },
            )
            .map_err(database_error)?;
    if (!has_scan_runs || has_single_scan_owner_index && has_scan_owner) && has_ownership_marker {
        return Ok(());
    }
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(database_error)?;
    let (has_single_scan_owner_index, has_scan_owner, has_ownership_marker) = transaction
        .query_row(
            "SELECT
               EXISTS(SELECT 1 FROM pragma_index_list('scan_runs')
                 WHERE name = 'scan_runs_one_active_root'
                   AND \"unique\" = 1 AND partial = 1),
               EXISTS(SELECT 1 FROM pragma_table_info('scan_runs')
                 WHERE name = 'scan_owner'),
               EXISTS(SELECT 1 FROM pragma_table_info('library_change_queue_contract')
                 WHERE name = 'scan_ownership_complete')",
            [],
            |row| {
                Ok((
                    row.get::<_, bool>(0)?,
                    row.get::<_, bool>(1)?,
                    row.get::<_, bool>(2)?,
                ))
            },
        )
        .map_err(database_error)?;
    if has_single_scan_owner_index && has_scan_owner && has_ownership_marker {
        return transaction.commit().map_err(database_error);
    }
    let has_conflicting_scan = if has_scan_runs {
        transaction
            .query_row(
                "SELECT EXISTS(
                   SELECT 1 FROM scan_runs
                   WHERE status IN ('running', 'paused')
                   GROUP BY root_id HAVING COUNT(*) > 1
                 )",
                [],
                |row| row.get::<_, bool>(0),
            )
            .map_err(database_error)?
    } else {
        false
    };
    if has_conflicting_scan {
        return Err(unverifiable_recovery_contract());
    }
    if has_scan_runs && !has_scan_owner {
        transaction
            .execute_batch(
                "ALTER TABLE scan_runs ADD COLUMN scan_owner TEXT NOT NULL DEFAULT 'foreground'
                   CHECK(scan_owner IN ('foreground', 'authoritative_recovery'));
                 UPDATE scan_runs SET scan_owner = 'authoritative_recovery'
                 WHERE id LIKE 'sync-recovery-%';",
            )
            .map_err(database_error)?;
    }
    if !has_ownership_marker {
        transaction
            .execute_batch(
                "ALTER TABLE library_change_queue_contract
                   ADD COLUMN scan_ownership_complete INTEGER NOT NULL DEFAULT 1
                   CHECK(scan_ownership_complete = 1);",
            )
            .map_err(database_error)?;
    }
    if has_scan_runs && !has_single_scan_owner_index {
        transaction
            .execute(
                "CREATE UNIQUE INDEX scan_runs_one_active_root
                 ON scan_runs(root_id) WHERE status IN ('running', 'paused')",
                [],
            )
            .map_err(database_error)?;
    }
    transaction.commit().map_err(database_error)
}

pub(super) fn validate_queue_authority(connection: &Connection) -> Result<(), ScanError> {
    let has_contract = connection
        .query_row(
            "SELECT EXISTS(
               SELECT 1 FROM sqlite_master
               WHERE type = 'table' AND name = 'library_change_queue_contract'
             )",
            [],
            |row| row.get::<_, bool>(0),
        )
        .map_err(database_error)?;
    if !has_contract {
        return Err(unverifiable_queue_authority());
    }
    let authority_complete = connection
        .query_row(
            "SELECT root_authority_complete = 1
             FROM library_change_queue_contract WHERE singleton = 1",
            [],
            |row| row.get::<_, bool>(0),
        )
        .optional()
        .map_err(database_error)?
        .unwrap_or(false);
    if !authority_complete {
        return Err(unverifiable_queue_authority());
    }
    Ok(())
}

pub(super) fn unverifiable_queue_authority() -> ScanError {
    ScanError::new(
        "catalog_change_queue_authority_unverifiable",
        "This prerelease schema 17 catalog cannot prove its highest root generations and must not be opened",
    )
}

pub(super) fn unverifiable_recovery_contract() -> ScanError {
    ScanError::new(
        "catalog_authoritative_recovery_contract_unverifiable",
        "This prerelease schema 18 catalog cannot prove its authoritative recovery contract",
    )
}

#[cfg(test)]
mod tests;
