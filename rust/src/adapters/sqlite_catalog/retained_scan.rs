use rusqlite::OptionalExtension;

use crate::domain::ScanError;
use crate::ports::RetainedScanRepository;

use super::{SqliteCatalog, abandon_scan_transaction, database_error, unix_time_ms};

impl RetainedScanRepository for SqliteCatalog {
    fn cancel_retained_scan(&mut self, scan_id: &str) -> Result<(), ScanError> {
        let transaction = self.begin_user_interactive_write()?;
        let state = transaction
            .query_row(
                "SELECT scan.status, scan.scan_owner, scan.issue_count,
                    EXISTS(SELECT 1 FROM library_roots AS root
                           JOIN library_change_root_state AS state ON state.root_id = root.id
                           WHERE root.id = scan.root_id AND root.active_scan_id IS NULL
                             AND state.generation = scan.root_generation_at_start
                             AND state.is_active = 1)
             FROM scan_runs AS scan WHERE scan.id = ?1",
                [scan_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, i64>(2)?,
                        row.get::<_, bool>(3)?,
                    ))
                },
            )
            .optional()
            .map_err(database_error)?;
        let Some((status, owner, issue_count, owns_unpublished_root)) = state else {
            return Err(ScanError::new(
                "scan_retained_not_found",
                "The retained scan no longer exists",
            ));
        };
        if owner != "foreground" {
            return Err(ScanError::new(
                "scan_retained_owner_mismatch",
                "Only a retained foreground import can be cancelled by this operation",
            ));
        }
        if status == "cancelled" {
            return Ok(());
        }
        if !matches!(status.as_str(), "running" | "paused") || !owns_unpublished_root {
            return Err(ScanError::new(
                "scan_retained_not_cancellable",
                "The scan is not an unfinished import of its current unpublished root",
            ));
        }
        abandon_scan_transaction(
            &transaction,
            scan_id,
            "cancelled",
            issue_count,
            unix_time_ms(),
        )?;
        transaction.commit().map_err(database_error)
    }
}
