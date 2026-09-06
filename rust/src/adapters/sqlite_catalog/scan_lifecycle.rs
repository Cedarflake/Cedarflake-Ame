use rusqlite::{OptionalExtension, Transaction, params};

use crate::domain::{IncrementalCatalogRoot, LibraryRootGeneration, ScanError};

use super::{
    SqliteCatalog, database_error, restore_explicit_recovery_claims_from_foreground_scan,
    retire_root_change_queue, sqlite_unsigned,
};

#[cfg(test)]
mod tests;

pub(super) fn abandon_scan_transaction(
    transaction: &Transaction<'_>,
    scan_id: &str,
    status: &str,
    issue_count: i64,
    completed_unix_ms: i64,
) -> Result<(), ScanError> {
    let abandoned = transaction
        .execute(
            "UPDATE scan_runs
             SET status = ?2, completed_unix_ms = ?3, issue_count = ?4,
                 current_directory_relative_path = NULL,
                 current_directory_enumerated = 0,
                 last_visited_relative_path = NULL
             WHERE id = ?1 AND status IN ('running', 'paused')",
            params![scan_id, status, completed_unix_ms, issue_count],
        )
        .map_err(database_error)?;
    if abandoned == 0 {
        // Only the transaction that retires a live scan owns its staging. A completed
        // scan may still own the active projection or durable cross-root handoffs.
        return Ok(());
    }
    restore_explicit_recovery_claims_from_foreground_scan(transaction, scan_id, completed_unix_ms)?;
    retire_unpublished_capture(transaction, scan_id, completed_unix_ms)?;
    transaction
        .execute(
            "UPDATE library_change_queue
                 SET status = 'pending', ready_unix_ms = ?2,
                     next_retry_unix_ms = NULL, lease_expires_unix_ms = NULL,
                     authoritative_scan_id = NULL, updated_unix_ms = ?2
                  WHERE authoritative_scan_id = ?1 AND status = 'leased'
                    AND NOT EXISTS(
                      SELECT 1 FROM library_live_gap_recovery_claims AS claim
                      WHERE claim.gap_change_id = library_change_queue.id
                    )",
            params![scan_id, completed_unix_ms],
        )
        .map_err(database_error)?;
    transaction
        .execute(
            "UPDATE library_change_queue
                 SET authoritative_scan_id = NULL
                 WHERE authoritative_scan_id = ?1",
            [scan_id],
        )
        .map_err(database_error)?;
    for sql in [
        "DELETE FROM scan_run_catch_up_lineage WHERE scan_id = ?1",
        "DELETE FROM scan_directory_frontier WHERE scan_id = ?1",
        "DELETE FROM scan_directory_entries WHERE scan_id = ?1",
        "DELETE FROM library_scan_publication_namespace_bindings WHERE scan_id = ?1",
    ] {
        transaction
            .execute(sql, [scan_id])
            .map_err(database_error)?;
    }
    discard_scan_staging(transaction, scan_id)?;
    Ok(())
}

fn retire_unpublished_capture(
    transaction: &Transaction<'_>,
    scan_id: &str,
    now_unix_ms: i64,
) -> Result<(), ScanError> {
    let root_id = transaction
        .query_row(
            "SELECT scan.root_id FROM scan_runs AS scan
         JOIN library_roots AS root ON root.id = scan.root_id
         JOIN library_change_root_state AS state ON state.root_id = root.id
         WHERE scan.id = ?1 AND scan.scan_owner = 'foreground'
           AND root.active_scan_id IS NULL
           AND state.generation = scan.root_generation_at_start AND state.is_active = 1",
            [scan_id],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(database_error)?;
    let Some(root_id) = root_id else {
        return Ok(());
    };
    discard_first_import_boundary(transaction, scan_id)?;
    retire_root_change_queue(transaction, &root_id, now_unix_ms)
}

pub(super) fn discard_first_import_boundary(
    transaction: &Transaction<'_>,
    scan_id: &str,
) -> Result<(), ScanError> {
    let unsafe_capture = transaction.query_row(
        "SELECT EXISTS(
           SELECT 1 FROM library_recovery_authorities AS authority
           JOIN library_persistent_journal_baselines AS baseline ON baseline.change_id = authority.change_id
           WHERE authority.run_id = ?1 AND (
             authority.reason <> 'first_import_boundary' OR baseline.phase <> 'inventory'
             OR EXISTS(SELECT 1 FROM library_metadata_inventory_runs WHERE id = ?1)
             OR EXISTS(SELECT 1 FROM library_persistent_journal_checkpoints
                       WHERE root_id = baseline.root_id AND root_generation = baseline.root_generation)
           )
         )",
        [scan_id], |row| row.get::<_, bool>(0),
    ).map_err(database_error)?;
    if unsafe_capture {
        return Err(ScanError::new(
            "scan_first_import_retirement_conflict",
            "The unpublished scan no longer owns an unconsumed first-import boundary",
        ));
    }
    transaction
        .execute(
            "DELETE FROM library_change_queue
         WHERE id IN (
           SELECT authority.change_id FROM library_recovery_authorities AS authority
           WHERE authority.run_id = ?1 AND authority.reason = 'first_import_boundary'
         )",
            [scan_id],
        )
        .map_err(database_error)?;
    Ok(())
}

impl SqliteCatalog {
    pub(crate) fn load_inactive_first_import_roots(
        &self,
    ) -> Result<Vec<IncrementalCatalogRoot>, ScanError> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT root.id, root.path, state.generation, catalog.revision
             FROM library_roots AS root
             JOIN library_change_root_state AS state ON state.root_id = root.id
             CROSS JOIN catalog_state AS catalog
             WHERE root.active_scan_id IS NULL AND state.is_active = 0
             ORDER BY root.id",
            )
            .map_err(database_error)?;
        let rows = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                ))
            })
            .map_err(database_error)?;
        rows.map(|row| {
            let (root_id, root_path, generation, revision) = row.map_err(database_error)?;
            let root_generation =
                LibraryRootGeneration::new(sqlite_unsigned(generation, "root generation")?)
                    .ok_or_else(|| {
                        ScanError::new(
                            "catalog_root_generation_invalid",
                            "The inactive first import has an invalid generation",
                        )
                    })?;
            Ok(IncrementalCatalogRoot {
                root_id,
                root_path,
                root_generation,
                active_scan_id: None,
                has_running_scan: false,
                catalog_revision: sqlite_unsigned(revision, "catalog revision")?,
                last_consistency_audit_unix_ms: None,
                publication_root_identity: None,
            })
        })
        .collect()
    }
}

pub(super) fn discard_scan_staging(
    transaction: &Transaction<'_>,
    scan_id: &str,
) -> Result<(), ScanError> {
    transaction.execute_batch(
        "CREATE TEMP TABLE IF NOT EXISTS ame_scan_discard_assets(asset_id TEXT PRIMARY KEY) WITHOUT ROWID;
         DELETE FROM temp.ame_scan_discard_assets;",
    ).map_err(database_error)?;
    transaction
        .execute(
            "INSERT OR IGNORE INTO temp.ame_scan_discard_assets(asset_id)
         SELECT asset_id FROM asset_locations WHERE scan_id = ?1",
            [scan_id],
        )
        .map_err(database_error)?;
    transaction
        .execute("DELETE FROM asset_locations WHERE scan_id = ?1", [scan_id])
        .map_err(database_error)?;
    transaction.execute(
        "DELETE FROM assets WHERE id IN (SELECT asset_id FROM temp.ame_scan_discard_assets)
           AND NOT EXISTS(SELECT 1 FROM asset_locations WHERE asset_id = assets.id)
           AND NOT EXISTS(SELECT 1 FROM library_change_catch_up_handoffs WHERE asset_id = assets.id)
           AND NOT EXISTS(SELECT 1 FROM library_change_scan_handoff_items WHERE asset_id = assets.id)", [],
    ).map_err(database_error)?;
    transaction
        .execute("DELETE FROM temp.ame_scan_discard_assets", [])
        .map_err(database_error)?;
    Ok(())
}
