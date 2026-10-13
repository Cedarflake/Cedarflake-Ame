use rusqlite::params;

use crate::domain::{MetadataInventoryRun, MetadataInventoryRunStatus, ScanError};

use super::{
    SqliteCatalog, database_error, directory_snapshot, load_absence_candidates, require_run,
    validate_active_root,
};

pub(super) fn complete(
    catalog: &mut SqliteCatalog,
    run_id: &str,
    completed_unix_ms: i64,
) -> Result<MetadataInventoryRun, ScanError> {
    let transaction = catalog.begin_write()?;
    let run = require_run(&transaction, run_id)?;
    if run.status != MetadataInventoryRunStatus::Comparing || !run.absence_authority {
        return Err(ScanError::new(
            "metadata_inventory_completion_not_authoritative",
            "Metadata inventory completion requires complete absence authority",
        ));
    }
    validate_active_root(&transaction, &run.request)?;
    let has_pending = transaction
        .query_row(
            "SELECT EXISTS(
               SELECT 1 FROM library_metadata_inventory_entries
               WHERE run_id = ?1 AND comparison_status = 'pending'
             )",
            [run_id],
            |row| row.get::<_, bool>(0),
        )
        .map_err(database_error)?;
    let remaining_absence =
        load_absence_candidates(&transaction, &run, run.absence_cursor.as_deref(), 1)?;
    if has_pending || !remaining_absence.is_empty() {
        return Err(ScanError::new(
            "metadata_inventory_comparison_incomplete",
            "Metadata inventory completion requires every staged and absent path to be compared",
        ));
    }
    let has_recovery_authority = transaction
        .query_row(
            "SELECT EXISTS(
               SELECT 1 FROM library_recovery_authorities
               WHERE run_id = ?1 AND retired_unix_ms IS NULL
             )",
            [run_id],
            |row| row.get::<_, bool>(0),
        )
        .map_err(database_error)?;
    if !has_recovery_authority {
        directory_snapshot::publish(&transaction, &run)?;
        transaction
            .execute(
                "UPDATE library_metadata_inventory_runs
                 SET status = 'completed', completed_unix_ms = ?2, updated_unix_ms = ?2
                 WHERE id = ?1 AND status = 'comparing' AND absence_authority = 1",
                params![run_id, completed_unix_ms],
            )
            .map_err(database_error)?;
    }
    let run = require_run(&transaction, run_id)?;
    transaction.commit().map_err(database_error)?;
    Ok(run)
}
