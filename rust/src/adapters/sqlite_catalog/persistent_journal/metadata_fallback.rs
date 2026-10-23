use rusqlite::{Connection, Transaction, params};

use crate::domain::{LibraryChangeId, LibraryRootGeneration, ScanError};

use super::super::{SqliteCatalog, database_error, sqlite_integer};

use crate::domain::persistent_journal::METADATA_FALLBACK_CODE;

pub(super) fn admit(
    transaction: &Transaction<'_>,
    change_id: LibraryChangeId,
    root_id: &str,
    generation: LibraryRootGeneration,
    now: i64,
) -> Result<(), ScanError> {
    transaction
        .execute(
            "INSERT INTO library_metadata_fallbacks(change_id, admitted_unix_ms)
         VALUES (?1, ?2) ON CONFLICT(change_id) DO NOTHING",
            params![
                sqlite_integer(change_id.value(), "fallback change ID")?,
                now
            ],
        )
        .map_err(database_error)?;
    // Keep the original cursor, ranges and failed window as evidence. Inventory owns
    // this recovery's completion; none of these rows is a successful journal receipt.
    transaction.execute(
        "UPDATE library_persistent_journal_root_state
         SET capability_state = 'live_only', continuity_state = 'live_only',
             last_failure_code = ?3, last_failure_message = 'Metadata verification owns this recovery',
             updated_unix_ms = MAX(updated_unix_ms, ?4)
         WHERE root_id = ?1 AND root_generation = ?2",
        params![root_id, sqlite_integer(generation.value(), "root generation")?, METADATA_FALLBACK_CODE, now],
    ).map_err(database_error)?;
    transaction
        .execute(
            "UPDATE library_persistent_journal_checkpoints
         SET continuity_state = 'live_only', updated_unix_ms = MAX(updated_unix_ms, ?3)
         WHERE root_id = ?1 AND root_generation = ?2",
            params![
                root_id,
                sqlite_integer(generation.value(), "root generation")?,
                now
            ],
        )
        .map_err(database_error)?;
    Ok(())
}

pub(in crate::adapters::sqlite_catalog) fn owns(
    connection: &Connection,
    change_id: LibraryChangeId,
) -> Result<bool, ScanError> {
    connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM library_metadata_fallbacks WHERE change_id = ?1)",
            [sqlite_integer(change_id.value(), "fallback change ID")?],
            |row| row.get(0),
        )
        .map_err(database_error)
}

impl SqliteCatalog {
    pub(crate) fn has_retained_recovery_inventory(
        &self,
        root_id: &str,
        generation: LibraryRootGeneration,
    ) -> Result<bool, ScanError> {
        self.connection
            .query_row(
                "SELECT EXISTS (SELECT 1 FROM library_recovery_authorities AS authority
             JOIN library_metadata_inventory_runs AS run ON run.id = authority.run_id
             WHERE authority.root_id = ?1 AND authority.root_generation = ?2
               AND authority.retired_unix_ms IS NULL AND run.status IN ('running', 'comparing'))",
                params![
                    root_id,
                    sqlite_integer(generation.value(), "root generation")?
                ],
                |row| row.get(0),
            )
            .map_err(database_error)
    }

    pub(crate) fn resume_journal_after_metadata_fallback(
        &mut self,
        root_id: &str,
        generation: LibraryRootGeneration,
        now: i64,
    ) -> Result<bool, ScanError> {
        let transaction = self.begin_write()?;
        let pending: bool = transaction
            .query_row(
                "SELECT EXISTS (
               SELECT 1 FROM library_recovery_authorities
               WHERE root_id = ?1 AND root_generation = ?2 AND retired_unix_ms IS NULL
             )",
                params![
                    root_id,
                    sqlite_integer(generation.value(), "root generation")?
                ],
                |row| row.get(0),
            )
            .map_err(database_error)?;
        if pending {
            return Ok(false);
        }
        let updated = transaction
            .execute(
                "UPDATE library_persistent_journal_root_state
             SET capability_state = 'supported', continuity_state = 'catching_up',
                 last_failure_code = NULL, last_failure_message = NULL,
                 updated_unix_ms = MAX(updated_unix_ms, ?3)
             WHERE root_id = ?1 AND root_generation = ?2 AND capability_state = 'live_only'
               AND last_failure_code = ?4
               AND EXISTS (SELECT 1 FROM library_change_root_state AS active
                 WHERE active.root_id = ?1 AND active.generation = ?2 AND active.is_active = 1)",
                params![
                    root_id,
                    sqlite_integer(generation.value(), "root generation")?,
                    now,
                    METADATA_FALLBACK_CODE
                ],
            )
            .map_err(database_error)?;
        if updated == 1 {
            transaction
                .execute(
                    "UPDATE library_persistent_journal_checkpoints
                 SET continuity_state = 'catching_up', last_failure_code = NULL,
                     last_failure_message = NULL, updated_unix_ms = MAX(updated_unix_ms, ?3)
                 WHERE root_id = ?1 AND root_generation = ?2 AND continuity_state = 'live_only'",
                    params![
                        root_id,
                        sqlite_integer(generation.value(), "root generation")?,
                        now
                    ],
                )
                .map_err(database_error)?;
        }
        transaction.commit().map_err(database_error)?;
        Ok(updated == 1)
    }
}

pub(super) fn complete_empty_retry(
    transaction: &Transaction<'_>,
    expected: &crate::domain::PersistentJournalCheckpoint,
    now: i64,
) -> Result<bool, ScanError> {
    let Some(current) =
        super::load_checkpoint(transaction, &expected.root_id, expected.root_generation)?
    else {
        return Ok(false);
    };
    if current != *expected
        || current.failure.is_some()
        || current.continuity != crate::domain::PersistentJournalContinuityState::CatchingUp
        || current.next_unread_usn != current.captured_exclusive_end
        || current.updated_unix_ms > now
    {
        return Ok(false);
    }
    let verified: bool = transaction.query_row(
        "SELECT EXISTS (
           SELECT 1 FROM library_metadata_fallbacks AS fallback
           JOIN library_recovery_authorities AS authority ON authority.change_id = fallback.change_id
           JOIN library_metadata_inventory_runs AS inventory ON inventory.id = authority.run_id
           WHERE authority.root_id = ?1 AND authority.root_generation = ?2
             AND authority.retired_unix_ms IS NOT NULL AND inventory.status = 'completed'
             AND inventory.enumeration_complete = 1 AND inventory.absence_authority = 1
         ) AND NOT EXISTS (
           SELECT 1 FROM library_recovery_authorities
           WHERE root_id = ?1 AND root_generation = ?2 AND retired_unix_ms IS NULL
         )",
        params![expected.root_id, sqlite_integer(expected.root_generation.value(), "root generation")?],
        |row| row.get(0),
    ).map_err(database_error)?;
    if !verified {
        return Ok(false);
    }
    let root_updated = transaction.execute(
        "UPDATE library_persistent_journal_root_state SET continuity_state = 'current', updated_unix_ms = ?3
         WHERE root_id = ?1 AND root_generation = ?2 AND capability_state = 'supported'
           AND continuity_state = 'catching_up' AND last_failure_code IS NULL
           AND EXISTS (SELECT 1 FROM library_change_root_state AS active
             WHERE active.root_id = ?1 AND active.generation = ?2 AND active.is_active = 1)",
        params![expected.root_id, sqlite_integer(expected.root_generation.value(), "root generation")?, now],
    ).map_err(database_error)?;
    if root_updated != 1 {
        return Ok(false);
    }
    transaction.execute(
        "UPDATE library_persistent_journal_checkpoints SET continuity_state = 'current', updated_unix_ms = ?3
         WHERE root_id = ?1 AND root_generation = ?2",
        params![expected.root_id, sqlite_integer(expected.root_generation.value(), "root generation")?, now],
    ).map_err(database_error)?;
    Ok(true)
}
