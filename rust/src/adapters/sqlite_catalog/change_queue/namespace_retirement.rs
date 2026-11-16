use rusqlite::{Transaction, params};

use crate::domain::ScanError;

use super::database_error;

// A queue may receive authority before its inventory starts. Namespace retirement must
// release that authority even when there is no run for inventory cleanup to discover.
pub(in crate::adapters::sqlite_catalog) const UNSTARTED_AUTHORITY_IDS: &str =
    "SELECT authority.change_id
     FROM library_recovery_authorities AS authority
     JOIN library_change_queue AS queue ON queue.id = authority.change_id
       AND queue.root_id = authority.root_id
       AND queue.root_generation = authority.root_generation
     JOIN library_change_queue_lanes AS lane ON lane.change_id = queue.id
     WHERE authority.retired_unix_ms IS NULL AND lane.lane = 'p2_recovery'
       AND queue.status = 'superseded' AND queue.next_retry_unix_ms IS NULL
       AND queue.lease_expires_unix_ms IS NULL AND queue.superseded_by_change_id IS NULL
       AND NOT EXISTS (SELECT 1 FROM library_metadata_inventory_runs AS run
                       WHERE run.id = authority.run_id)
       AND NOT EXISTS (SELECT 1 FROM library_root_publication_namespaces AS namespace
                       WHERE namespace.root_id = authority.root_id
                         AND namespace.root_generation = authority.root_generation)";

pub(super) fn retire_unstarted_authorities(
    transaction: &Transaction<'_>,
    root_id: &str,
    generation: i64,
    now: i64,
) -> Result<(), ScanError> {
    transaction
        .execute(
            &format!(
                "UPDATE library_recovery_authorities
                 SET retired_unix_ms = MAX(authorized_unix_ms, ?3)
                 WHERE change_id IN ({UNSTARTED_AUTHORITY_IDS})
                   AND root_id = ?1 AND root_generation = ?2"
            ),
            params![root_id, generation, now],
        )
        .map_err(database_error)?;
    Ok(())
}
