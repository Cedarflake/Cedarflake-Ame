use rusqlite::{Transaction, params};

use super::super::{database_error, validate_active_root};
use crate::domain::{MetadataInventoryRun, MetadataInventoryScope, ScanError};

pub(in crate::adapters::sqlite_catalog::metadata_inventory) fn initialize_recovery(
    transaction: &Transaction<'_>,
    run: &MetadataInventoryRun,
) -> Result<(), ScanError> {
    if run.request.scope != MetadataInventoryScope::Root || !run.enumeration_complete {
        return Ok(());
    }
    let initializing: bool = transaction
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM library_recovery_authorities AS authority
         WHERE authority.run_id = ?1 AND authority.retired_unix_ms IS NULL
           AND authority.reason IN ('existing_root_baseline', 'first_import_boundary',
             'journal_gap', 'journal_reset', 'journal_trim', 'journal_reconstruction_failure',
             'containment_failure', 'broker_after_current_failure'))",
            [&run.request.run_id],
            |row| row.get(0),
        )
        .map_err(database_error)?;
    if !initializing {
        return Ok(());
    }
    validate_active_root(transaction, &run.request)?;
    transaction
        .execute(
            "DELETE FROM scan_directory_journal_authority
         WHERE scan_id = (SELECT active_scan_id FROM library_roots WHERE id = ?1)",
            params![run.request.root_id],
        )
        .map_err(database_error)?;
    // Install the completed baseline input before closing-range replay. Later journal deltas
    // remain authoritative when inventory comparison and candidate publication finish.
    super::publish(transaction, run)
}
