use rusqlite::{OptionalExtension, Transaction, params};

use crate::domain::{LibraryRootGeneration, ScanError};

use super::persistence::{retire_root_publication_namespace, retire_unconsumed_live_gap_claims};
use super::{database_error, sqlite_unsigned};

// Both normal removal and compatibility repair require the same durable proof. An inactive
// generation alone is insufficient: namespace replacement and cancelled imports retain roots.
pub(in crate::adapters::sqlite_catalog) const REMOVED_ROOT_AUTHORITY_IDS_SQL: &str =
    "SELECT authority.change_id
     FROM library_recovery_authorities AS authority
     JOIN library_change_queue AS queue ON queue.id = authority.change_id
       AND queue.root_id = authority.root_id
       AND queue.root_generation = authority.root_generation
     JOIN library_change_queue_lanes AS lanes ON lanes.change_id = queue.id
     JOIN library_change_root_state AS state ON state.root_id = authority.root_id
       AND state.generation = authority.root_generation
     WHERE authority.retired_unix_ms IS NULL AND state.is_active = 0
       AND lanes.lane = 'p2_recovery' AND queue.status = 'superseded'
       AND queue.next_retry_unix_ms IS NULL AND queue.lease_expires_unix_ms IS NULL
       AND queue.superseded_by_change_id IS NULL
       AND NOT EXISTS(SELECT 1 FROM library_roots WHERE id = authority.root_id)
       AND NOT EXISTS(SELECT 1 FROM library_metadata_inventory_runs WHERE id = authority.run_id)
       AND NOT EXISTS(
         SELECT 1 FROM library_root_publication_namespaces AS namespace
         WHERE namespace.root_id = authority.root_id
           AND namespace.root_generation = authority.root_generation
       )";

pub(in crate::adapters::sqlite_catalog) fn removed_root_authority_retirement_sql() -> String {
    format!(
        "UPDATE library_recovery_authorities
         SET retired_unix_ms = MAX(
           authorized_unix_ms,
           (SELECT updated_unix_ms FROM library_change_queue
            WHERE id = library_recovery_authorities.change_id),
           (SELECT updated_unix_ms FROM library_change_root_state
            WHERE root_id = library_recovery_authorities.root_id)
         )
         WHERE change_id IN ({REMOVED_ROOT_AUTHORITY_IDS_SQL})"
    )
}

pub(in crate::adapters::sqlite_catalog) fn retire_removed_root_recovery_authorities(
    transaction: &Transaction<'_>,
    root_id: &str,
) -> Result<(), ScanError> {
    // Run rows disappear through root removal before authorization retires. Retiring earlier
    // would leave a retired authority attached to a still-running inventory.
    transaction
        .execute(
            &format!(
                "{} AND root_id = ?1",
                removed_root_authority_retirement_sql()
            ),
            [root_id],
        )
        .map_err(database_error)?;
    Ok(())
}

pub(in crate::adapters::sqlite_catalog) fn retire_root_change_queue(
    transaction: &Transaction<'_>,
    root_id: &str,
    now_unix_ms: i64,
) -> Result<(), ScanError> {
    let current_generation = transaction
        .query_row(
            "SELECT generation FROM library_change_root_state WHERE root_id = ?1",
            [root_id],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .map_err(database_error)?;
    if let Some(current_generation) = current_generation {
        if current_generation <= 0 {
            return Err(ScanError::new(
                "change_queue_generation_invalid",
                "The retired root has an invalid stored generation",
            ));
        }
        let parsed_generation =
            LibraryRootGeneration::new(sqlite_unsigned(current_generation, "root generation")?)
                .ok_or_else(|| {
                    ScanError::new(
                        "change_queue_generation_invalid",
                        "The retired root has an invalid stored generation",
                    )
                })?;
        retire_unconsumed_live_gap_claims(transaction, root_id, parsed_generation)?;
        retire_root_publication_namespace(transaction, root_id, current_generation)?;
        transaction
            .execute(
                "UPDATE library_change_root_state
                 SET is_active = 0, updated_unix_ms = ?1 WHERE root_id = ?2",
                params![now_unix_ms, root_id],
            )
            .map_err(database_error)?;
    } else {
        return Err(ScanError::new(
            "change_queue_generation_missing",
            "The registered root has no durable generation authority to retire",
        ));
    }
    transaction
        .execute(
            "UPDATE library_change_queue
             SET status = 'superseded', next_retry_unix_ms = NULL,
                 lease_expires_unix_ms = NULL, superseded_by_change_id = NULL,
                 updated_unix_ms = ?1
             WHERE root_id = ?2 AND root_generation = ?3
               AND status IN ('pending', 'leased', 'retry_wait')",
            params![now_unix_ms, root_id, current_generation],
        )
        .map_err(database_error)?;
    Ok(())
}
