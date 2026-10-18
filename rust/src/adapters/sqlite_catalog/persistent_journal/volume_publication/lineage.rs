use rusqlite::{Transaction, params};

use crate::domain::{
    JournalUsn, PersistentJournalCrossRootLineage, PersistentJournalLineageState,
    PersistentJournalVolumeBatch, ScanError,
};

use super::super::super::change_queue::PERSISTENT_JOURNAL_CATCH_UP_SOURCE;

use super::super::super::{database_error, sqlite_integer};

use super::super::{invalid_batch, lineage_state_text};

pub(super) fn validate_atomic_lineage(
    batch: &PersistentJournalVolumeBatch,
) -> Result<(), ScanError> {
    let mut owners =
        std::collections::BTreeMap::<&str, Vec<&PersistentJournalCrossRootLineage>>::new();
    for page in &batch.pages {
        for lineage in page
            .enrollment
            .cross_root_lineage
            .iter()
            .chain(&page.enrollment.carried_cross_root_lineage)
        {
            owners
                .entry(lineage.lineage_id.as_str())
                .or_default()
                .push(lineage);
        }
    }
    for endpoints in owners.values() {
        let [left, right] = endpoints.as_slice() else {
            return Err(invalid_batch(
                "A volume handoff must publish exactly two source-range owners",
            ));
        };
        if left.owner_source_range_id == right.owner_source_range_id
            || left.volume != right.volume
            || left.journal_id != right.journal_id
            || left.file_reference != right.file_reference
            || left.old_usn != right.old_usn
            || left.new_usn != right.new_usn
            || left.previous_carry_id != right.previous_carry_id
            || left.previous_root_id != right.previous_root_id
            || left.previous_root_generation != right.previous_root_generation
            || left.previous_relative_path != right.previous_relative_path
            || left.current_root_id != right.current_root_id
            || left.current_root_generation != right.current_root_generation
            || left.current_relative_path != right.current_relative_path
            || left.state != right.state
        {
            return Err(invalid_batch(
                "A volume handoff contains inconsistent durable endpoint evidence",
            ));
        }
    }
    Ok(())
}

pub(in crate::adapters::sqlite_catalog::persistent_journal) fn enroll_cross_root_lineage(
    transaction: &Transaction<'_>,
    lineage: &PersistentJournalCrossRootLineage,
    enrolled_unix_ms: i64,
) -> Result<(), ScanError> {
    transaction
        .execute(
            "INSERT INTO library_persistent_journal_cross_root_lineage(
               id, volume_guid, volume_serial, file_reference_version, file_reference,
               previous_root_id, previous_root_generation, previous_relative_path,
               current_root_id, current_root_generation, current_relative_path,
               status, created_unix_ms, updated_unix_ms, journal_id, old_usn, new_usn,
               previous_carry_id
             ) VALUES (
               ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?13,
               ?14, ?15, ?16, ?17
             ) ON CONFLICT(id) DO NOTHING",
            params![
                lineage.lineage_id,
                lineage.volume.volume_guid,
                lineage.volume.canonical_serial(),
                i64::from(lineage.file_reference.record_version()),
                lineage.file_reference.as_bytes(),
                lineage.previous_root_id,
                sqlite_integer(lineage.previous_root_generation.value(), "root generation")?,
                lineage.previous_relative_path,
                lineage.current_root_id,
                sqlite_integer(lineage.current_root_generation.value(), "root generation")?,
                lineage.current_relative_path,
                lineage_state_text(lineage.state),
                enrolled_unix_ms,
                lineage.journal_id.to_canonical_text(),
                lineage.old_usn.map(JournalUsn::to_canonical_text),
                lineage.new_usn.map(JournalUsn::to_canonical_text),
                lineage.previous_carry_id,
            ],
        )
        .map_err(database_error)?;
    let exact = transaction
        .query_row(
            "SELECT volume_guid = ?2 AND volume_serial = ?3
                    AND file_reference_version = ?4 AND file_reference = ?5
                    AND previous_root_id = ?6 AND previous_root_generation = ?7
                    AND previous_relative_path = ?8 AND current_root_id = ?9
                    AND current_root_generation = ?10 AND current_relative_path = ?11
                    AND status = ?12 AND journal_id = ?13
                    AND old_usn IS ?14 AND new_usn IS ?15 AND previous_carry_id IS ?16
             FROM library_persistent_journal_cross_root_lineage WHERE id = ?1",
            params![
                lineage.lineage_id,
                lineage.volume.volume_guid,
                lineage.volume.canonical_serial(),
                i64::from(lineage.file_reference.record_version()),
                lineage.file_reference.as_bytes(),
                lineage.previous_root_id,
                sqlite_integer(lineage.previous_root_generation.value(), "root generation")?,
                lineage.previous_relative_path,
                lineage.current_root_id,
                sqlite_integer(lineage.current_root_generation.value(), "root generation")?,
                lineage.current_relative_path,
                lineage_state_text(lineage.state),
                lineage.journal_id.to_canonical_text(),
                lineage.old_usn.map(JournalUsn::to_canonical_text),
                lineage.new_usn.map(JournalUsn::to_canonical_text),
                lineage.previous_carry_id,
            ],
            |row| row.get::<_, bool>(0),
        )
        .map_err(database_error)?;
    if !exact {
        return Err(invalid_batch(
            "The cross-root lineage identifier conflicts with durable evidence",
        ));
    }
    let owner = transaction
        .query_row(
            "SELECT root_id, root_generation
             FROM library_persistent_journal_source_ranges WHERE id = ?1",
            [&lineage.owner_source_range_id],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)),
        )
        .map_err(database_error)?;
    let participant_role = if lineage.previous_root_id == owner.0
        && lineage.previous_root_generation.value()
            == u64::try_from(owner.1).map_err(|_| invalid_batch("Invalid owner generation"))?
    {
        "previous"
    } else if lineage.current_root_id == owner.0
        && lineage.current_root_generation.value()
            == u64::try_from(owner.1).map_err(|_| invalid_batch("Invalid owner generation"))?
    {
        "current"
    } else {
        return Err(invalid_batch(
            "The lineage participant does not match the owner root generation",
        ));
    };
    transaction
        .execute(
            "INSERT INTO library_persistent_journal_cross_root_ranges(
               lineage_id, source_range_id, participant_role, enrolled_unix_ms
             ) VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(lineage_id, source_range_id) DO NOTHING",
            params![
                lineage.lineage_id,
                lineage.owner_source_range_id,
                participant_role,
                enrolled_unix_ms,
            ],
        )
        .map_err(database_error)?;
    link_cross_root_queue_evidence(transaction, &lineage.lineage_id, enrolled_unix_ms)?;
    if lineage.state == PersistentJournalLineageState::Completed {
        let role_count = transaction
            .query_row(
                "SELECT COUNT(DISTINCT participant_role)
                 FROM library_persistent_journal_cross_root_ranges WHERE lineage_id = ?1",
                [&lineage.lineage_id],
                |row| row.get::<_, i64>(0),
            )
            .map_err(database_error)?;
        if role_count != 2 {
            return Err(invalid_batch(
                "Completed cross-root lineage requires both durable root ranges",
            ));
        }
    }
    Ok(())
}

fn link_cross_root_queue_evidence(
    transaction: &Transaction<'_>,
    lineage_id: &str,
    enrolled_unix_ms: i64,
) -> Result<(), ScanError> {
    transaction
        .execute(
            "INSERT INTO library_change_queue_catch_up_lineage(
               change_id, catch_up_source, catch_up_watermark, enrolled_unix_ms
             )
             SELECT owner_changes.change_id, ?2, peer_ranges.source_range_id, ?3
             FROM library_persistent_journal_cross_root_ranges AS owner_ranges
             JOIN library_persistent_journal_queue_lineage AS owner_changes
               ON owner_changes.source_range_id = owner_ranges.source_range_id
             JOIN library_persistent_journal_cross_root_ranges AS peer_ranges
               ON peer_ranges.lineage_id = owner_ranges.lineage_id
             WHERE owner_ranges.lineage_id = ?1
             ON CONFLICT(change_id, catch_up_source, catch_up_watermark) DO NOTHING",
            params![
                lineage_id,
                PERSISTENT_JOURNAL_CATCH_UP_SOURCE,
                enrolled_unix_ms,
            ],
        )
        .map_err(database_error)?;
    transaction
        .execute(
            "INSERT INTO library_change_queue_catch_up_lineage(
               change_id, catch_up_source, catch_up_watermark, enrolled_unix_ms
             )
             SELECT recovery_changes.id, ?2, owner_ranges.source_range_id, ?3
             FROM library_persistent_journal_cross_root_lineage AS lineage
             JOIN library_persistent_journal_cross_root_ranges AS owner_ranges
               ON owner_ranges.lineage_id = lineage.id
             JOIN library_change_queue AS recovery_changes
               ON recovery_changes.root_id = lineage.previous_root_id
              AND recovery_changes.root_generation = lineage.previous_root_generation
              AND recovery_changes.relative_path = lineage.previous_relative_path
              AND recovery_changes.first_sequence = lineage.new_usn
              AND recovery_changes.most_recent_sequence = lineage.new_usn
             JOIN library_persistent_journal_queue_lineage AS recovery_ownership
               ON recovery_ownership.change_id = recovery_changes.id
             JOIN library_persistent_journal_source_ranges AS recovery_ranges
               ON recovery_ranges.id = recovery_ownership.source_range_id
              AND recovery_ranges.volume_guid = lineage.volume_guid
              AND recovery_ranges.volume_serial = lineage.volume_serial
              AND recovery_ranges.journal_id = lineage.journal_id
              AND CAST(recovery_ranges.requested_start_usn AS INTEGER)
                    <= CAST(lineage.new_usn AS INTEGER)
              AND CAST(lineage.new_usn AS INTEGER)
                    < CAST(recovery_ranges.covered_until_usn AS INTEGER)
             WHERE lineage.id = ?1
               AND lineage.previous_carry_id IS NOT NULL
               AND lineage.new_usn IS NOT NULL
             ON CONFLICT(change_id, catch_up_source, catch_up_watermark) DO NOTHING",
            params![
                lineage_id,
                PERSISTENT_JOURNAL_CATCH_UP_SOURCE,
                enrolled_unix_ms,
            ],
        )
        .map_err(database_error)?;
    Ok(())
}
