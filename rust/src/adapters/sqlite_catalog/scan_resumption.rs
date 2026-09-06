use rusqlite::{Transaction, params};

use crate::domain::{ScanCheckpoint, ScanError};

use super::scan_lifecycle::{discard_first_import_boundary, discard_scan_staging};
use super::{ScanOwner, database_error};

enum ScanResumePlan {
    RetainPublishedCheckpoint,
    RebuildUnverifiedFirstImport,
}

pub(super) fn resume_checkpoint(
    transaction: &Transaction<'_>,
    scan_id: &str,
    owner: ScanOwner,
    checkpoint: ScanCheckpoint,
) -> Result<ScanCheckpoint, ScanError> {
    let has_published_baseline = transaction
        .query_row(
            "SELECT root.active_scan_id IS NOT NULL FROM scan_runs AS scan
         JOIN library_roots AS root ON root.id = scan.root_id WHERE scan.id = ?1",
            [scan_id],
            |row| row.get::<_, bool>(0),
        )
        .map_err(database_error)?;
    let plan = if owner == ScanOwner::Foreground && !has_published_baseline {
        ScanResumePlan::RebuildUnverifiedFirstImport
    } else {
        ScanResumePlan::RetainPublishedCheckpoint
    };
    match plan {
        ScanResumePlan::RetainPublishedCheckpoint => Ok(checkpoint),
        ScanResumePlan::RebuildUnverifiedFirstImport => {
            // A retained opening record cannot prove that its journal still covers the
            // detached interval. Only an explicit resume reaches this rebuilding path.
            discard_first_import_boundary(transaction, scan_id)?;
            discard_scan_staging(transaction, scan_id)?;
            for sql in [
                "DELETE FROM scan_directory_frontier WHERE scan_id = ?1",
                "DELETE FROM scan_directory_entries WHERE scan_id = ?1",
                "DELETE FROM scan_issues WHERE scan_id = ?1",
            ] {
                transaction
                    .execute(sql, [scan_id])
                    .map_err(database_error)?;
            }
            transaction
                .execute(
                    "UPDATE scan_runs SET last_visited_relative_path = NULL,
                   visited_entries = 0, accepted_items = 0, issue_count = 0,
                   requires_previous_snapshot = 0, current_directory_relative_path = NULL,
                   current_directory_enumerated = 0
                 WHERE id = ?1 AND status IN ('running', 'paused')",
                    [scan_id],
                )
                .map_err(database_error)?;
            transaction
                .execute(
                    "INSERT INTO scan_directory_frontier(scan_id, relative_path) VALUES (?1, '')",
                    [scan_id],
                )
                .map_err(database_error)?;
            transaction.execute(
                "UPDATE library_persistent_journal_root_state
                 SET protocol_version = 0, capability_state = 'unknown', continuity_state = 'baseline_required',
                   last_failure_code = NULL, last_failure_message = NULL
                 WHERE (root_id, root_generation) = (
                   SELECT root_id, root_generation_at_start FROM scan_runs WHERE id = ?1
                 )",
                params![scan_id],
            ).map_err(database_error)?;
            Ok(ScanCheckpoint::default())
        }
    }
}
