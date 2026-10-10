use rusqlite::{Transaction, params};

use crate::domain::ScanError;

use super::database_error;

pub(super) fn cleanup_batch(
    transaction: &Transaction<'_>,
    terminal_before_unix_ms: i64,
    owner_limit: u32,
    run_limit: u32,
) -> Result<(), ScanError> {
    // Candidate lineage outlives staged entries until the terminal retention deadline.
    // Retire it explicitly so deleting a run cannot cascade through an unbounded owner set.
    transaction
        .execute(
            "DELETE FROM library_metadata_inventory_candidate_owners WHERE rowid IN (
           SELECT owner.rowid FROM library_metadata_inventory_candidate_owners AS owner
           WHERE owner.run_id IN (
             SELECT run.id FROM library_metadata_inventory_runs AS run
             WHERE run.status IN ('completed', 'failed', 'cancelled', 'superseded')
             AND run.updated_unix_ms < ?1
             AND NOT EXISTS(SELECT 1 FROM library_metadata_inventory_spools AS spool
               WHERE spool.run_id = run.id)
             AND NOT EXISTS(SELECT 1 FROM library_metadata_inventory_entries AS entry
               WHERE entry.run_id = run.id)
             AND EXISTS(SELECT 1 FROM library_metadata_inventory_candidate_owners AS candidate
               WHERE candidate.run_id = run.id)
             ORDER BY run.status, run.updated_unix_ms, run.id LIMIT ?3
           ) ORDER BY owner.run_id, owner.candidate_key LIMIT ?2
         )",
            params![terminal_before_unix_ms, owner_limit, run_limit],
        )
        .map_err(database_error)?;
    Ok(())
}
