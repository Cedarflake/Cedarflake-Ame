use rusqlite::{OptionalExtension, Transaction, params};

use crate::domain::{ScanCheckpoint, ScanError};

use super::scan_staging::persist_pending_locations;
use super::{SqliteCatalog, database_error, sqlite_integer};

pub(super) enum ProgressBoundary {
    Checkpoint,
    DirectoryCompletion,
}

pub(super) fn persist_progress(
    catalog: &mut SqliteCatalog,
    scan_id: &str,
    checkpoint: &ScanCheckpoint,
    boundary: ProgressBoundary,
) -> Result<(), ScanError> {
    let pending = catalog.pending_locations.clone();
    let transaction = catalog.begin_write()?;
    persist_pending_locations(&transaction, &pending)?;
    match boundary {
        ProgressBoundary::Checkpoint => write_checkpoint(&transaction, scan_id, checkpoint)?,
        ProgressBoundary::DirectoryCompletion => {
            complete_directory(&transaction, scan_id, checkpoint)?;
        }
    }
    transaction.commit().map_err(database_error)?;
    catalog.pending_locations.clear();
    Ok(())
}

fn write_checkpoint(
    transaction: &Transaction<'_>,
    scan_id: &str,
    checkpoint: &ScanCheckpoint,
) -> Result<(), ScanError> {
    let visited_entries = sqlite_integer(checkpoint.visited_entries, "visited entry count")?;
    let accepted_items = sqlite_integer(checkpoint.accepted_items, "accepted item count")?;
    let issue_count = sqlite_integer(checkpoint.issue_count, "issue count")?;
    let updated = transaction
        .execute(
            "UPDATE scan_runs
             SET last_visited_relative_path = ?2, visited_entries = ?3,
                 accepted_items = ?4, issue_count = ?5,
                 requires_previous_snapshot = ?6
             WHERE id = ?1 AND status IN ('running', 'paused')",
            params![
                scan_id,
                checkpoint.last_visited_relative_path,
                visited_entries,
                accepted_items,
                issue_count,
                checkpoint.requires_previous_snapshot,
            ],
        )
        .map_err(database_error)?;
    if updated != 1 {
        return Err(ScanError::new(
            "catalog_scan_not_checkpointable",
            "The scan is no longer in a checkpointable running state",
        ));
    }
    Ok(())
}

fn complete_directory(
    transaction: &Transaction<'_>,
    scan_id: &str,
    checkpoint: &ScanCheckpoint,
) -> Result<(), ScanError> {
    let visited_entries = sqlite_integer(checkpoint.visited_entries, "visited entry count")?;
    let accepted_items = sqlite_integer(checkpoint.accepted_items, "accepted item count")?;
    let issue_count = sqlite_integer(checkpoint.issue_count, "issue count")?;
    let current_directory = transaction
        .query_row(
            "SELECT current_directory_relative_path FROM scan_runs
             WHERE id = ?1 AND status = 'running'",
            [scan_id],
            |row| row.get::<_, Option<String>>(0),
        )
        .optional()
        .map_err(database_error)?
        .flatten()
        .ok_or_else(|| {
            ScanError::new(
                "catalog_scan_directory_not_completable",
                "The current scan directory is missing",
            )
        })?;
    transaction
        .execute(
            "DELETE FROM scan_directory_entries
             WHERE scan_id = ?1 AND directory_relative_path = ?2",
            params![scan_id, current_directory],
        )
        .map_err(database_error)?;
    let updated = transaction
        .execute(
            "UPDATE scan_runs
             SET current_directory_relative_path = NULL,
                 current_directory_enumerated = 0,
                 last_visited_relative_path = NULL,
                 visited_entries = ?2, accepted_items = ?3, issue_count = ?4,
                 requires_previous_snapshot = ?5
             WHERE id = ?1 AND status = 'running'",
            params![
                scan_id,
                visited_entries,
                accepted_items,
                issue_count,
                checkpoint.requires_previous_snapshot,
            ],
        )
        .map_err(database_error)?;
    if updated != 1 {
        return Err(ScanError::new(
            "catalog_scan_directory_not_completable",
            "The current scan directory could not be completed",
        ));
    }
    Ok(())
}
