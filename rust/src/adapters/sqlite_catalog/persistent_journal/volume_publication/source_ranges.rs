use rusqlite::{OptionalExtension, Transaction, params};

use crate::domain::{
    PersistentJournalCheckpoint, PersistentJournalEnrollmentBatch, ScanError,
    persistent_journal_batch_payload,
};

use super::super::super::{database_error, sqlite_integer};

use super::super::{
    compare_and_swap_checkpoint, invalid_batch, load_checkpoint, load_root_authority,
    require_active_root_generation,
};

pub(in crate::adapters::sqlite_catalog::persistent_journal) fn require_supported_root(
    transaction: &Transaction<'_>,
    batch: &PersistentJournalEnrollmentBatch,
) -> Result<(), ScanError> {
    let supported = transaction
        .query_row(
            "SELECT journal.capability_state = 'supported' AND active.is_active = 1
             FROM library_persistent_journal_root_state AS journal
             JOIN library_change_root_state AS active
               ON active.root_id = journal.root_id
              AND active.generation = journal.root_generation
             WHERE journal.root_id = ?1 AND journal.root_generation = ?2",
            params![
                batch.range.root_id,
                sqlite_integer(batch.range.root_generation.value(), "root generation")?,
            ],
            |row| row.get::<_, bool>(0),
        )
        .optional()
        .map_err(database_error)?
        .unwrap_or(false);
    if supported {
        Ok(())
    } else {
        Err(ScanError::new(
            "persistent_journal_root_authority_unavailable",
            "The root has no supported persistent journal authority for this generation",
        ))
    }
}

pub(super) fn reject_conflicting_range_coordinates(
    transaction: &Transaction<'_>,
    batch: &PersistentJournalEnrollmentBatch,
) -> Result<(), ScanError> {
    let conflicting = transaction
        .query_row(
            "SELECT EXISTS(
               SELECT 1 FROM library_persistent_journal_source_ranges
               WHERE root_id = ?1 AND root_generation = ?2
                 AND volume_guid = ?3 AND volume_serial = ?4 AND journal_id = ?5
                 AND requested_start_usn = ?6 AND requested_end_usn = ?7
                 AND covered_until_usn = ?8 AND protocol_version = ?9
                 AND contract_version = ?10 AND id <> ?11
             )",
            params![
                batch.range.root_id,
                sqlite_integer(batch.range.root_generation.value(), "root generation")?,
                batch.range.volume.volume_guid,
                batch.range.volume.canonical_serial(),
                batch.range.journal_id.to_canonical_text(),
                batch.range.requested_start_usn.to_canonical_text(),
                batch.range.requested_end_usn.to_canonical_text(),
                batch.range.covered_until_usn.to_canonical_text(),
                i64::from(batch.range.protocol_version),
                i64::from(batch.range.contract_version),
                batch.range.batch_id,
            ],
            |row| row.get::<_, bool>(0),
        )
        .map_err(database_error)?;
    if conflicting {
        Err(invalid_batch(
            "The journal range coordinates are already bound to different batch content",
        ))
    } else {
        Ok(())
    }
}

pub(in crate::adapters::sqlite_catalog::persistent_journal) fn source_range_exists(
    transaction: &Transaction<'_>,
    batch: &PersistentJournalEnrollmentBatch,
) -> Result<bool, ScanError> {
    let exact = transaction
        .query_row(
            "SELECT root_id = ?2 AND root_generation = ?3
                    AND volume_guid = ?4 AND volume_serial = ?5 AND journal_id = ?6
                    AND requested_start_usn = ?7 AND requested_end_usn = ?8
                    AND covered_until_usn = ?9 AND is_complete = ?10
                    AND protocol_version = ?11 AND contract_version = ?12
                    AND canonical_payload = ?13
             FROM library_persistent_journal_source_ranges WHERE id = ?1",
            params![
                batch.range.batch_id,
                batch.range.root_id,
                sqlite_integer(batch.range.root_generation.value(), "root generation")?,
                batch.range.volume.volume_guid,
                batch.range.volume.canonical_serial(),
                batch.range.journal_id.to_canonical_text(),
                batch.range.requested_start_usn.to_canonical_text(),
                batch.range.requested_end_usn.to_canonical_text(),
                batch.range.covered_until_usn.to_canonical_text(),
                batch.range.is_complete,
                i64::from(batch.range.protocol_version),
                i64::from(batch.range.contract_version),
                persistent_journal_batch_payload(batch),
            ],
            |row| row.get::<_, bool>(0),
        )
        .optional()
        .map_err(database_error)?;
    match exact {
        Some(true) => Ok(true),
        Some(false) => Err(invalid_batch(
            "The journal batch identifier is already owned by different source evidence",
        )),
        None => Ok(false),
    }
}

pub(in crate::adapters::sqlite_catalog::persistent_journal) fn insert_source_range(
    transaction: &Transaction<'_>,
    batch: &PersistentJournalEnrollmentBatch,
) -> Result<(), ScanError> {
    transaction
        .execute(
            "INSERT INTO library_persistent_journal_source_ranges(
               id, root_id, root_generation, volume_guid, volume_serial, journal_id,
               requested_start_usn, requested_end_usn, covered_until_usn, is_complete,
               protocol_version, contract_version, status, enrolled_unix_ms,
               checkpointed_unix_ms, canonical_payload
             ) VALUES (
               ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12,
               'enrolled', ?13, NULL, ?14
             )",
            params![
                batch.range.batch_id,
                batch.range.root_id,
                sqlite_integer(batch.range.root_generation.value(), "root generation")?,
                batch.range.volume.volume_guid,
                batch.range.volume.canonical_serial(),
                batch.range.journal_id.to_canonical_text(),
                batch.range.requested_start_usn.to_canonical_text(),
                batch.range.requested_end_usn.to_canonical_text(),
                batch.range.covered_until_usn.to_canonical_text(),
                batch.range.is_complete,
                i64::from(batch.range.protocol_version),
                i64::from(batch.range.contract_version),
                batch.range.enrolled_unix_ms,
                persistent_journal_batch_payload(batch),
            ],
        )
        .map_err(database_error)?;
    transaction
        .execute(
            "INSERT INTO library_persistent_journal_range_lifecycle(
               source_range_id, lifecycle_state, completed_unix_ms, updated_unix_ms
             ) VALUES (?1, 'pending', NULL, ?2)",
            params![batch.range.batch_id, batch.range.enrolled_unix_ms],
        )
        .map_err(database_error)?;
    Ok(())
}

pub(super) fn validate_checkpoint_advance(
    transaction: &Transaction<'_>,
    range: &crate::domain::PersistentJournalSourceRange,
    checkpoint: &PersistentJournalCheckpoint,
) -> Result<(), ScanError> {
    if range.root_id != checkpoint.root_id
        || range.root_generation != checkpoint.root_generation
        || range.volume != checkpoint.volume
        || range.journal_id != checkpoint.journal_id
        || range.protocol_version != checkpoint.protocol_version
        || range.contract_version != checkpoint.contract_version
        || range.covered_until_usn != checkpoint.next_unread_usn
        || range.requested_end_usn != checkpoint.captured_exclusive_end
    {
        return Err(invalid_batch(
            "The checkpoint does not exactly close its durable source range",
        ));
    }
    require_active_root_generation(transaction, &checkpoint.root_id, checkpoint.root_generation)?;
    let current = load_checkpoint(transaction, &checkpoint.root_id, checkpoint.root_generation)?
        .ok_or_else(|| {
            invalid_batch(
                "The source range cannot establish an implicit persistent journal baseline",
            )
        })?;
    let authority = load_root_authority(transaction, checkpoint)?;
    if current.volume != range.volume
        || current.root_file_reference != checkpoint.root_file_reference
        || current.journal_id != range.journal_id
        || current.next_unread_usn != range.requested_start_usn
        || current.captured_exclusive_end > range.requested_end_usn
        || current.covered_catalog_revision > checkpoint.covered_catalog_revision
        || current.protocol_version != range.protocol_version
        || current.contract_version != range.contract_version
        || checkpoint.next_unread_usn <= current.next_unread_usn
        || checkpoint.captured_exclusive_end < current.captured_exclusive_end
        || checkpoint.updated_unix_ms < current.updated_unix_ms
        || authority.protocol_version != range.protocol_version
        || authority.contract_version != range.contract_version
        || authority.capability_state != "supported"
    {
        return Err(invalid_batch(
            "The checkpoint does not continuously advance the exact durable journal state",
        ));
    }
    Ok(())
}

pub(super) fn apply_checkpoint_advance(
    transaction: &Transaction<'_>,
    range: &crate::domain::PersistentJournalSourceRange,
    checkpoint: &PersistentJournalCheckpoint,
) -> Result<(), ScanError> {
    let current = load_checkpoint(transaction, &checkpoint.root_id, checkpoint.root_generation)?
        .ok_or_else(|| invalid_batch("The checkpoint baseline disappeared"))?;
    let authority = load_root_authority(transaction, checkpoint)?;
    compare_and_swap_checkpoint(transaction, &current, checkpoint)?;
    let updated = transaction
        .execute(
            "UPDATE library_persistent_journal_source_ranges
             SET status = 'checkpointed', checkpointed_unix_ms = ?1
             WHERE id = ?2 AND status = 'enrolled'",
            params![checkpoint.updated_unix_ms, range.batch_id],
        )
        .map_err(database_error)?;
    if updated != 1 {
        return Err(invalid_batch(
            "The source range checkpoint transition raced",
        ));
    }
    let authority_updated = transaction
        .execute(
            "UPDATE library_persistent_journal_root_state
             SET protocol_version = ?1, contract_version = ?2,
                 capability_state = 'supported', continuity_state = 'catching_up',
                 last_failure_code = NULL, last_failure_message = NULL,
                 updated_unix_ms = ?3
             WHERE root_id = ?4 AND root_generation = ?5
               AND protocol_version = ?6 AND contract_version = ?7
               AND capability_state = ?8 AND continuity_state = ?9
               AND last_failure_code IS ?10 AND last_failure_message IS ?11
               AND updated_unix_ms = ?12
               AND EXISTS(
                 SELECT 1 FROM library_change_root_state AS active
                 WHERE active.root_id = ?4 AND active.generation = ?5 AND active.is_active = 1
               )",
            params![
                i64::from(checkpoint.protocol_version),
                i64::from(checkpoint.contract_version),
                checkpoint.updated_unix_ms,
                checkpoint.root_id,
                sqlite_integer(checkpoint.root_generation.value(), "root generation")?,
                i64::from(authority.protocol_version),
                i64::from(authority.contract_version),
                authority.capability_state,
                authority.continuity_state,
                authority.failure_code,
                authority.failure_message,
                authority.updated_unix_ms,
            ],
        )
        .map_err(database_error)?;
    if authority_updated != 1 {
        return Err(ScanError::new(
            "persistent_journal_root_authority_stale",
            "The checkpoint no longer owns the active root generation",
        ));
    }
    Ok(())
}
