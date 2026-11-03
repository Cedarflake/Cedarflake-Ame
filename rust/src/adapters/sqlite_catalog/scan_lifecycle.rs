use rusqlite::{OptionalExtension, Transaction, params};

use crate::domain::{IncrementalCatalogRoot, LibraryRootGeneration, ScanError};

use crate::ports::scan_admission::FirstImportReadiness;

use super::scan_staging::retire_scan_staging;

use super::{
    SqliteCatalog, database_error, restore_explicit_recovery_claims_from_foreground_scan,
    retire_root_change_queue, sqlite_integer, sqlite_unsigned,
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
        "DELETE FROM scan_directory_identities WHERE scan_id = ?1",
        "DELETE FROM library_scan_publication_namespace_bindings WHERE scan_id = ?1",
        "DELETE FROM scan_resume_pending_locations WHERE scan_id = ?1",
    ] {
        transaction
            .execute(sql, [scan_id])
            .map_err(database_error)?;
    }
    retire_scan_staging(transaction, scan_id)?;
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

impl FirstImportReadiness for SqliteCatalog {
    fn pristine_first_import_scan(
        &self,
        root_id: &str,
        root_generation: LibraryRootGeneration,
    ) -> Result<Option<(String, i64)>, ScanError> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT scan.id, scan.started_unix_ms
                 FROM scan_runs AS scan
                 JOIN library_roots AS root ON root.id = scan.root_id
                 JOIN library_change_root_state AS active
                   ON active.root_id = scan.root_id
                  AND active.generation = scan.root_generation_at_start
                 WHERE scan.root_id = ?1 AND scan.root_generation_at_start = ?2
                   AND scan.scan_owner = 'foreground' AND scan.status = 'running'
                   AND scan.visited_entries = 0 AND scan.accepted_items = 0
                   AND root.active_scan_id IS NULL AND active.is_active = 1
                 ORDER BY scan.started_unix_ms, scan.id
                 LIMIT 2",
            )
            .map_err(database_error)?;
        let scan_ids = statement
            .query_map(
                params![
                    root_id,
                    sqlite_integer(root_generation.value(), "root generation")?,
                ],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)),
            )
            .map_err(database_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(database_error)?;
        match scan_ids.as_slice() {
            [] => Ok(None),
            [scan] => Ok(Some(scan.clone())),
            _ => Err(ScanError::new(
                "catalog_first_import_scan_ambiguous",
                "The root has more than one pristine first-import scan",
            )),
        }
    }

    fn load_inactive_first_import_roots(&self) -> Result<Vec<IncrementalCatalogRoot>, ScanError> {
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
