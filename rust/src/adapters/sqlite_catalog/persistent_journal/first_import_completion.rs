use rusqlite::{Connection, Transaction, TransactionBehavior, params};

use crate::domain::{LibraryChangeId, LibraryChangeLane, ScanError};

use super::super::{
    SqliteCatalog, database_error, load_catalog_revision, sqlite_integer, sqlite_unsigned,
};
use super::{baseline_closing_is_covered, invalid_batch};

#[cfg(test)]
mod tests;

struct ReadyFirstImportBaseline {
    change_id: LibraryChangeId,
}

pub(super) fn finalize(
    catalog: &mut SqliteCatalog,
    completed_unix_ms: i64,
) -> Result<bool, ScanError> {
    if completed_unix_ms < 0 {
        return Err(invalid_batch(
            "The first-import baseline completion time is invalid",
        ));
    }
    if !catalog.connection.is_autocommit() {
        return Err(ScanError::new(
            "persistent_journal_first_import_completion_transaction_active",
            "First-import completion requires a standalone catalog connection",
        ));
    }
    // This read-only hint never requests writer admission or interrupts publication.
    // Its snapshot ends before admission; only the second selection can authorize completion.
    let has_candidate = {
        let read = catalog
            .connection
            .transaction_with_behavior(TransactionBehavior::Deferred)
            .map_err(database_error)?;
        let found = select_ready(&read)?.is_some();
        read.commit().map_err(database_error)?;
        found
    };
    if !has_candidate {
        return Ok(false);
    }
    let transaction = catalog.begin_write_in_lane(LibraryChangeLane::Journal)?;
    let Some(candidate) = select_ready(&transaction)? else {
        transaction.commit().map_err(database_error)?;
        return Ok(false);
    };
    complete(&transaction, candidate, completed_unix_ms)?;
    transaction.commit().map_err(database_error)?;
    Ok(true)
}

fn select_ready(connection: &Connection) -> Result<Option<ReadyFirstImportBaseline>, ScanError> {
    let candidates = {
        let mut statement = connection
            .prepare(
                "SELECT baseline.change_id
                 FROM library_persistent_journal_baselines AS baseline
                 JOIN library_recovery_authorities AS authority
                   ON authority.change_id = baseline.change_id
                 JOIN library_change_queue AS control ON control.id = baseline.change_id
                 JOIN library_change_root_state AS root_state
                   ON root_state.root_id = baseline.root_id
                  AND root_state.generation = baseline.root_generation
                 JOIN library_roots AS root ON root.id = baseline.root_id
                 JOIN scan_runs AS scan ON scan.id = authority.run_id
                 WHERE authority.reason = 'first_import_boundary'
                   AND authority.retired_unix_ms IS NULL
                   AND baseline.phase = 'replay'
                   AND baseline.closing_next_usn IS NOT NULL
                   AND control.status IN ('pending', 'retry_wait')
                   AND root_state.is_active = 1
                   AND root.active_scan_id = scan.id
                   AND scan.root_id = baseline.root_id
                   AND scan.root_generation_at_start = baseline.root_generation
                   AND scan.scan_owner = 'foreground'
                   AND scan.status = 'completed'
                   AND NOT EXISTS (
                     SELECT 1
                     FROM library_change_queue AS pending
                     JOIN library_change_queue_lanes AS lane
                       ON lane.change_id = pending.id
                     WHERE pending.root_id = baseline.root_id
                       AND pending.root_generation = baseline.root_generation
                       AND (
                         lane.lane IN ('p0_live', 'p1_journal')
                         OR (lane.lane = 'p2_recovery'
                           AND pending.id <> baseline.change_id)
                       )
                       AND pending.status IN ('pending', 'leased', 'retry_wait')
                   )
                 ORDER BY baseline.change_id
                 LIMIT 16",
            )
            .map_err(database_error)?;
        statement
            .query_map([], |row| row.get::<_, i64>(0))
            .map_err(database_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(database_error)?
    };
    for stored_change_id in candidates {
        let change_id = LibraryChangeId::new(sqlite_unsigned(
            stored_change_id,
            "first-import baseline change ID",
        )?)
        .ok_or_else(|| invalid_batch("The first-import baseline change ID is invalid"))?;
        if baseline_closing_is_covered(connection, change_id)? {
            return Ok(Some(ReadyFirstImportBaseline { change_id }));
        }
    }
    Ok(None)
}

fn complete(
    transaction: &Transaction<'_>,
    candidate: ReadyFirstImportBaseline,
    completed_unix_ms: i64,
) -> Result<(), ScanError> {
    let stored_change_id = sqlite_integer(
        candidate.change_id.value(),
        "first-import baseline change ID",
    )?;
    let (root_id, root_generation, updated_unix_ms) = transaction
        .query_row(
            "SELECT root_id, root_generation, updated_unix_ms
             FROM library_persistent_journal_baselines
             WHERE change_id = ?1 AND phase = 'replay'",
            [stored_change_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            },
        )
        .map_err(database_error)?;
    if completed_unix_ms < updated_unix_ms {
        return Err(ScanError::new(
            "persistent_journal_first_import_completion_time_invalid",
            "The first-import baseline cannot complete before its closing replay",
        ));
    }
    let catalog_revision = load_catalog_revision(transaction)?;
    let completed = transaction
        .execute(
            "UPDATE library_change_queue
             SET status = 'completed', next_retry_unix_ms = NULL,
                 lease_expires_unix_ms = NULL,
                 catalog_revision_at_success = ?2, updated_unix_ms = ?3
             WHERE id = ?1 AND status IN ('pending', 'retry_wait')",
            params![
                stored_change_id,
                sqlite_integer(catalog_revision, "catalog revision")?,
                completed_unix_ms,
            ],
        )
        .map_err(database_error)?;
    if completed != 1 {
        return Err(ScanError::new(
            "persistent_journal_first_import_completion_raced",
            "The first-import baseline control changed during completion",
        ));
    }
    let baseline_updated = transaction
        .execute(
            "UPDATE library_persistent_journal_baselines
             SET phase = 'completed', completed_unix_ms = ?2, updated_unix_ms = ?2
             WHERE change_id = ?1 AND phase = 'replay'
               AND completed_unix_ms IS NULL",
            params![stored_change_id, completed_unix_ms],
        )
        .map_err(database_error)?;
    let checkpoint_updated = transaction
        .execute(
            "UPDATE library_persistent_journal_checkpoints
             SET continuity_state = 'current', updated_unix_ms = ?3
             WHERE root_id = ?1 AND root_generation = ?2
               AND continuity_state = 'catching_up'
               AND next_unread_usn = captured_exclusive_end
               AND last_failure_code IS NULL",
            params![root_id, root_generation, completed_unix_ms],
        )
        .map_err(database_error)?;
    let root_state_updated = transaction
        .execute(
            "UPDATE library_persistent_journal_root_state
             SET continuity_state = 'current', updated_unix_ms = ?3
             WHERE root_id = ?1 AND root_generation = ?2
               AND capability_state = 'supported'
               AND continuity_state = 'catching_up'
               AND last_failure_code IS NULL",
            params![root_id, root_generation, completed_unix_ms],
        )
        .map_err(database_error)?;
    let authority_retired = transaction
        .execute(
            "UPDATE library_recovery_authorities
             SET retired_unix_ms = ?2
             WHERE change_id = ?1 AND reason = 'first_import_boundary'
               AND retired_unix_ms IS NULL",
            params![stored_change_id, completed_unix_ms],
        )
        .map_err(database_error)?;
    if baseline_updated != 1
        || checkpoint_updated != 1
        || root_state_updated != 1
        || authority_retired != 1
    {
        return Err(ScanError::new(
            "persistent_journal_first_import_completion_raced",
            "The first-import baseline authority changed during atomic completion",
        ));
    }
    Ok(())
}
