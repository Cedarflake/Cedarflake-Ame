use rusqlite::{OptionalExtension, Transaction, params};

use crate::domain::{LibraryRootGeneration, ScanError};

use super::super::super::{database_error, sqlite_integer};

pub(super) fn retire_superseded(
    transaction: &Transaction<'_>,
    root_id: &str,
    generation: LibraryRootGeneration,
    now: i64,
) -> Result<(), ScanError> {
    let previous = transaction
        .query_row(
            "SELECT baseline.change_id
         FROM library_persistent_journal_baselines AS baseline
         JOIN library_recovery_authorities AS old ON old.change_id = baseline.change_id
         JOIN library_change_queue AS control ON control.id = old.change_id
         JOIN scan_runs AS scan ON scan.id = old.run_id
         JOIN library_roots AS root ON root.id = old.root_id AND root.active_scan_id = scan.id
         WHERE old.root_id = ?1 AND old.root_generation = ?2
           AND old.reason = 'first_import_boundary' AND old.retired_unix_ms IS NULL
           AND baseline.phase <> 'completed' AND control.status IN ('pending', 'retry_wait')
           AND scan.root_id = old.root_id AND scan.root_generation_at_start = old.root_generation
           AND scan.scan_owner = 'foreground' AND scan.status = 'completed'
           AND EXISTS (
             SELECT 1 FROM library_metadata_fallbacks AS fallback
             JOIN library_recovery_authorities AS current ON current.change_id = fallback.change_id
             JOIN library_change_queue AS replacement ON replacement.id = current.change_id
             WHERE current.root_id = old.root_id AND current.root_generation = old.root_generation
               AND current.change_id <> old.change_id AND current.retired_unix_ms IS NULL
               AND replacement.status IN ('pending', 'leased', 'retry_wait')
               AND fallback.admitted_unix_ms >= baseline.authorized_unix_ms
           )",
            params![
                root_id,
                sqlite_integer(generation.value(), "root generation")?
            ],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .map_err(database_error)?;
    let Some(change_id) = previous else {
        return Ok(());
    };
    transaction
        .execute(
            "UPDATE library_change_queue
         SET status = 'superseded', lease_expires_unix_ms = NULL,
             updated_unix_ms = MAX(updated_unix_ms, ?2),
             last_failure_code = 'persistent_journal_baseline_replaced',
             last_failure_message = 'Independent metadata verification owns the unfinished baseline'
         WHERE id = ?1 AND status IN ('pending', 'retry_wait')",
            params![change_id, now],
        )
        .map_err(database_error)?;
    transaction
        .execute(
            "UPDATE library_recovery_authorities
         SET retired_unix_ms = MAX(authorized_unix_ms, ?2)
         WHERE change_id = ?1 AND retired_unix_ms IS NULL",
            params![change_id, now],
        )
        .map_err(database_error)?;
    Ok(())
}
