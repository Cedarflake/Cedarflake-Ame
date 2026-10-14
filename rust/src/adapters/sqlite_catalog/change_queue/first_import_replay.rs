use rusqlite::{Connection, OptionalExtension, params};

use crate::domain::{LibraryChangeId, LibraryRootGeneration, ScanError};

use super::super::persistent_journal::baseline_closing_is_covered;
use super::{database_error, sqlite_integer, sqlite_unsigned};

#[cfg(test)]
mod tests;

pub(in crate::adapters::sqlite_catalog) fn blocks_unlineaged_live_changes(
    connection: &Connection,
    root_id: &str,
    generation: LibraryRootGeneration,
) -> Result<bool, ScanError> {
    let change_id = connection
        .query_row(
            "SELECT baseline.change_id
             FROM library_persistent_journal_baselines AS baseline
             JOIN library_recovery_authorities AS authority
               ON authority.change_id = baseline.change_id
             JOIN library_roots AS root ON root.id = baseline.root_id
             JOIN scan_runs AS scan ON scan.id = authority.run_id
             JOIN library_change_root_state AS state ON state.root_id = root.id
             WHERE baseline.root_id = ?1 AND baseline.root_generation = ?2
               AND baseline.phase <> 'completed'
               AND authority.reason = 'first_import_boundary'
               AND authority.retired_unix_ms IS NULL
               AND authority.root_id = baseline.root_id
               AND authority.root_generation = baseline.root_generation
               AND state.generation = baseline.root_generation AND state.is_active = 1
               AND root.active_scan_id = scan.id
               AND scan.root_id = baseline.root_id
               AND scan.root_generation_at_start = baseline.root_generation
               AND scan.scan_owner = 'foreground' AND scan.status = 'completed'",
            params![
                root_id,
                sqlite_integer(generation.value(), "root generation")?
            ],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .map_err(database_error)?;
    let Some(change_id) = change_id else {
        return Ok(false);
    };
    let change_id = LibraryChangeId::new(sqlite_unsigned(change_id, "baseline change ID")?)
        .ok_or_else(|| {
            ScanError::new(
                "persistent_journal_first_import_change_invalid",
                "The first-import replay requires a valid baseline change ID",
            )
        })?;
    // A live removal has no journal handoff owner until enrollment. Let the closing
    // replay consume the retained identity before an unrelated live lease can erase it.
    // Release at replay coverage: baseline completion itself waits for these live leases.
    Ok(!baseline_closing_is_covered(connection, change_id)?)
}
