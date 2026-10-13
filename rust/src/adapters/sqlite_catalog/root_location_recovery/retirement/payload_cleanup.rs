use std::sync::atomic::AtomicBool;

use rusqlite::{Transaction, params};

use crate::domain::ScanError;

use super::super::super::database_error;
use super::super::{check_cancelled, write_attempt::LocationWriteAttempt};

const PAYLOAD_PAGE_LIMIT: i64 = 1_024;

pub(super) fn remove_pages(
    transaction: &Transaction<'_>,
    root_id: &str,
    generation: i64,
    cancelled: &AtomicBool,
    attempt: &LocationWriteAttempt,
) -> Result<(), ScanError> {
    // The retirement owner bounds this control roster before any payload work. Pages stay in
    // the same transaction as binding and authority retirement, so interruption restores them all.
    let mut statement = transaction
        .prepare(
            "SELECT id FROM library_metadata_inventory_runs
         WHERE root_id = ?1 AND root_generation = ?2 AND status <> 'completed'
         ORDER BY epoch LIMIT 256",
        )
        .map_err(database_error)?;
    let run_ids = statement
        .query_map(params![root_id, generation], |row| row.get::<_, String>(0))
        .map_err(database_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(database_error)?;
    for run_id in run_ids {
        for deletion in [
            "DELETE FROM library_metadata_inventory_entries WHERE rowid IN (
               SELECT rowid FROM library_metadata_inventory_entries
               WHERE run_id = ?1 ORDER BY relative_path LIMIT ?2
             )",
            "DELETE FROM library_metadata_inventory_candidate_owners WHERE rowid IN (
               SELECT rowid FROM library_metadata_inventory_candidate_owners
               WHERE run_id = ?1 ORDER BY candidate_key LIMIT ?2
             )",
            "DELETE FROM library_metadata_inventory_frontier WHERE rowid IN (
               SELECT rowid FROM library_metadata_inventory_frontier
               WHERE run_id = ?1 ORDER BY ordinal LIMIT ?2
             )",
        ] {
            let mut remove = transaction.prepare(deletion).map_err(database_error)?;
            loop {
                check_cancelled(cancelled)?;
                attempt.ensure_running()?;
                let removed = remove
                    .execute(params![run_id, PAYLOAD_PAGE_LIMIT])
                    .map_err(database_error)?;
                if removed == 0 {
                    break;
                }
                #[cfg(test)]
                super::super::tests::cost::after_recovery_write_page(root_id);
            }
        }
    }
    Ok(())
}
