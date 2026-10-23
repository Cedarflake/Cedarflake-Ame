use rusqlite::{OptionalExtension, params};

use crate::domain::{
    LibraryChangeId, LibraryChangeLane, PersistentJournalBaselinePhase,
    PersistentJournalCheckpoint, PersistentJournalContinuityState, ScanError,
};

use super::super::{SqliteCatalog, database_error, sqlite_integer, sqlite_unsigned};
use super::{invalid_batch, load_baseline, load_checkpoint};

pub(super) fn record(
    catalog: &mut SqliteCatalog,
    expected: &PersistentJournalCheckpoint,
    observed_unix_ms: i64,
) -> Result<bool, ScanError> {
    expected.validate()?;
    if expected.next_unread_usn != expected.captured_exclusive_end
        || observed_unix_ms < expected.updated_unix_ms
    {
        return Err(invalid_batch(
            "An empty replay must prove its exact closing cursor",
        ));
    }
    let transaction = catalog.begin_write_in_lane(LibraryChangeLane::Journal)?;
    let stored_id = transaction
        .query_row(
            "SELECT baseline.change_id FROM library_persistent_journal_baselines AS baseline
         JOIN library_recovery_authorities AS authority ON authority.change_id = baseline.change_id
         JOIN library_persistent_journal_root_state AS root
           ON root.root_id = baseline.root_id AND root.root_generation = baseline.root_generation
         JOIN library_change_root_state AS binding
           ON binding.root_id = baseline.root_id AND binding.generation = baseline.root_generation
         WHERE baseline.root_id = ?1 AND baseline.root_generation = ?2
           AND baseline.phase IN ('replay', 'absence')
           AND baseline.opening_next_usn = baseline.closing_next_usn
           AND authority.retired_unix_ms IS NULL AND binding.is_active = 1
           AND root.capability_state = 'supported' AND root.continuity_state = 'catching_up'",
            params![
                expected.root_id,
                sqlite_integer(expected.root_generation.value(), "root generation")?
            ],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .map_err(database_error)?;
    let Some(stored_id) = stored_id else {
        let completed = super::metadata_fallback::complete_empty_retry(
            &transaction,
            expected,
            observed_unix_ms,
        )?;
        transaction.commit().map_err(database_error)?;
        return Ok(completed);
    };
    let change_id = LibraryChangeId::new(sqlite_unsigned(stored_id, "baseline change ID")?)
        .ok_or_else(|| invalid_batch("The empty replay baseline identity is invalid"))?;
    let baseline = load_baseline(&transaction, change_id)?
        .ok_or_else(|| invalid_batch("The empty replay baseline disappeared"))?;
    let current = load_checkpoint(&transaction, &expected.root_id, expected.root_generation)?
        .ok_or_else(|| invalid_batch("The empty replay checkpoint disappeared"))?;
    if current.continuity != PersistentJournalContinuityState::CatchingUp
        || current.updated_unix_ms > observed_unix_ms
        || current.volume != expected.volume
        || current.root_file_reference != expected.root_file_reference
        || current.journal_id != expected.journal_id
        || current.next_unread_usn != expected.next_unread_usn
        || current.captured_exclusive_end != expected.captured_exclusive_end
        || current.protocol_version != expected.protocol_version
        || current.contract_version != expected.contract_version
        || baseline.closing_next_usn != Some(expected.next_unread_usn)
        || baseline.validate_replay_checkpoint(&current).is_err()
    {
        transaction.commit().map_err(database_error)?;
        return Ok(false);
    }
    if baseline.phase == PersistentJournalBaselinePhase::Replay {
        transaction
            .execute(
                "UPDATE library_persistent_journal_baselines
             SET phase = 'absence', updated_unix_ms = MAX(updated_unix_ms, ?2)
             WHERE change_id = ?1 AND phase = 'replay' AND closing_next_usn = opening_next_usn",
                params![stored_id, observed_unix_ms],
            )
            .map_err(database_error)?;
    }
    transaction
        .execute(
            "UPDATE library_persistent_journal_checkpoints SET last_failure_code = NULL,
           last_failure_message = NULL, updated_unix_ms = MAX(updated_unix_ms, ?3)
         WHERE root_id = ?1 AND root_generation = ?2 AND continuity_state = 'catching_up'",
            params![
                expected.root_id,
                sqlite_integer(expected.root_generation.value(), "root generation")?,
                observed_unix_ms
            ],
        )
        .map_err(database_error)?;
    transaction
        .execute(
            "UPDATE library_persistent_journal_root_state SET last_failure_code = NULL,
           last_failure_message = NULL, updated_unix_ms = MAX(updated_unix_ms, ?3)
         WHERE root_id = ?1 AND root_generation = ?2
           AND capability_state = 'supported' AND continuity_state = 'catching_up'",
            params![
                expected.root_id,
                sqlite_integer(expected.root_generation.value(), "root generation")?,
                observed_unix_ms
            ],
        )
        .map_err(database_error)?;
    transaction.commit().map_err(database_error)?;
    Ok(true)
}
