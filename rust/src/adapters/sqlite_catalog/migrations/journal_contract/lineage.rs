use rusqlite::Connection;

use crate::domain::{
    JournalFileReference, JournalIdentifier, JournalUsn, PersistentJournalCrossRootLineage,
    PersistentJournalLineageState, PersistentJournalPendingRename, PersistentJournalVolumeIdentity,
    ScanError, persistent_journal_batch_payload_contains_lineage,
    persistent_journal_batch_payload_contains_pending_lineage_source,
};

use super::super::database_error;

pub(in crate::adapters::sqlite_catalog) fn validate_lineage_rows(
    connection: &Connection,
) -> Result<(), ScanError> {
    let mut statement = connection
        .prepare(
            "SELECT lineage.id, lineage.volume_guid, lineage.volume_serial,
                    lineage.file_reference_version, lineage.file_reference,
                    lineage.previous_root_id, lineage.previous_root_generation,
                    lineage.previous_relative_path, lineage.current_root_id,
                    lineage.current_root_generation, lineage.current_relative_path,
                    lineage.status, lineage.journal_id, lineage.old_usn,
                    lineage.new_usn, lineage.previous_carry_id
             FROM library_persistent_journal_cross_root_lineage AS lineage
             ORDER BY lineage.id",
        )
        .map_err(database_error)?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, Vec<u8>>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, i64>(6)?,
                row.get::<_, String>(7)?,
                row.get::<_, String>(8)?,
                row.get::<_, i64>(9)?,
                row.get::<_, String>(10)?,
                row.get::<_, String>(11)?,
                row.get::<_, Option<String>>(12)?,
                row.get::<_, Option<String>>(13)?,
                row.get::<_, Option<String>>(14)?,
                row.get::<_, Option<String>>(15)?,
            ))
        })
        .map_err(database_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(database_error)?;
    drop(statement);
    for row in rows {
        let (
            lineage_id,
            volume_guid,
            volume_serial,
            reference_version,
            reference,
            previous_root_id,
            previous_root_generation,
            previous_relative_path,
            current_root_id,
            current_root_generation,
            current_relative_path,
            state,
            journal_id,
            old_usn,
            new_usn,
            previous_carry_id,
        ) = row;
        let file_reference = JournalFileReference::from_bytes(&reference)
            .map_err(|_| super::values::unverifiable_contract())?;
        if i64::from(file_reference.record_version()) != reference_version {
            return Err(super::values::unverifiable_contract());
        }
        let mut lineage = PersistentJournalCrossRootLineage {
            lineage_id: lineage_id.clone(),
            owner_source_range_id: String::new(),
            volume: PersistentJournalVolumeIdentity {
                volume_guid,
                volume_serial: super::values::parse_canonical_u64(&volume_serial)?,
            },
            journal_id: JournalIdentifier::parse_canonical(
                journal_id
                    .as_deref()
                    .ok_or_else(super::values::unverifiable_contract)?,
            )
            .map_err(|_| super::values::unverifiable_contract())?,
            file_reference,
            old_usn: old_usn
                .as_deref()
                .map(JournalUsn::parse_canonical)
                .transpose()
                .map_err(|_| super::values::unverifiable_contract())?,
            new_usn: new_usn
                .as_deref()
                .map(JournalUsn::parse_canonical)
                .transpose()
                .map_err(|_| super::values::unverifiable_contract())?,
            previous_carry_id,
            previous_root_id,
            previous_root_generation: super::values::parse_root_generation(
                previous_root_generation,
            )?,
            previous_relative_path,
            current_root_id,
            current_root_generation: super::values::parse_root_generation(current_root_generation)?,
            current_relative_path,
            state: super::values::parse_lineage_state(&state)?,
        };
        let mut owner_statement = connection
            .prepare(
                "SELECT owners.source_range_id, owners.participant_role,
                        ranges.root_id, ranges.root_generation, ranges.volume_guid,
                        ranges.volume_serial, ranges.journal_id, ranges.canonical_payload
                 FROM library_persistent_journal_cross_root_ranges AS owners
                 JOIN library_persistent_journal_source_ranges AS ranges
                   ON ranges.id = owners.source_range_id
                 WHERE owners.lineage_id = ?1
                 ORDER BY owners.participant_role, owners.source_range_id",
            )
            .map_err(database_error)?;
        let owner_rows = owner_statement
            .query_map([&lineage_id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, Vec<u8>>(7)?,
                ))
            })
            .map_err(database_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(database_error)?;
        let [current_owner, previous_owner] = owner_rows.as_slice() else {
            return Err(super::values::unverifiable_contract());
        };
        if current_owner.1 != "current" || previous_owner.1 != "previous" {
            return Err(super::values::unverifiable_contract());
        }
        for owner in [previous_owner, current_owner] {
            let expected = if owner.1 == "previous" {
                (
                    lineage.previous_root_id.as_str(),
                    lineage.previous_root_generation,
                )
            } else {
                (
                    lineage.current_root_id.as_str(),
                    lineage.current_root_generation,
                )
            };
            if owner.2 != expected.0
                || super::values::parse_root_generation(owner.3)? != expected.1
                || owner.4 != lineage.volume.volume_guid
                || super::values::parse_canonical_u64(&owner.5)? != lineage.volume.volume_serial
                || JournalIdentifier::parse_canonical(&owner.6)
                    .map_err(|_| super::values::unverifiable_contract())?
                    != lineage.journal_id
            {
                return Err(super::values::unverifiable_contract());
            }
        }
        let owner_lifecycle = {
            let mut lifecycle_statement = connection
                .prepare(
                    "SELECT owners.participant_role, ranges.status,
                            lifecycle.lifecycle_state, ranges.root_generation,
                            COALESCE(active.is_active, 0), active.generation,
                            journal.continuity_state, journal.last_failure_code
                     FROM library_persistent_journal_cross_root_ranges AS owners
                     JOIN library_persistent_journal_source_ranges AS ranges
                       ON ranges.id = owners.source_range_id
                     JOIN library_persistent_journal_range_lifecycle AS lifecycle
                       ON lifecycle.source_range_id = ranges.id
                     LEFT JOIN library_change_root_state AS active
                       ON active.root_id = ranges.root_id
                     LEFT JOIN library_persistent_journal_root_state AS journal
                       ON journal.root_id = ranges.root_id
                      AND journal.root_generation = ranges.root_generation
                     WHERE owners.lineage_id = ?1
                     ORDER BY owners.participant_role",
                )
                .map_err(database_error)?;
            lifecycle_statement
                .query_map([&lineage_id], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, i64>(3)?,
                        row.get::<_, bool>(4)?,
                        row.get::<_, Option<i64>>(5)?,
                        row.get::<_, Option<String>>(6)?,
                        row.get::<_, Option<String>>(7)?,
                    ))
                })
                .map_err(database_error)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(database_error)?
        };
        let [current_lifecycle, previous_lifecycle] = owner_lifecycle.as_slice() else {
            return Err(super::values::unverifiable_contract());
        };
        if current_lifecycle.0 != "current" || previous_lifecycle.0 != "previous" {
            return Err(super::values::unverifiable_contract());
        }
        let mut completed_count = 0_u8;
        let mut superseded_count = 0_u8;
        for owner in [previous_lifecycle, current_lifecycle] {
            match owner.2.as_str() {
                "completed" if owner.1 == "checkpointed" => {
                    completed_count = completed_count.saturating_add(1);
                }
                "pending" if owner.1 != "superseded" => {}
                "superseded"
                    if owner.1 == "superseded"
                        && (!owner.4 || owner.5 != Some(owner.3))
                        && owner.6.as_deref() == Some("unavailable")
                        && owner.7.as_deref() == Some("root_generation_retired") =>
                {
                    superseded_count = superseded_count.saturating_add(1);
                }
                _ => return Err(super::values::unverifiable_contract()),
            }
        }
        let expected_state = if superseded_count > 0 {
            PersistentJournalLineageState::Superseded
        } else if completed_count == 2 {
            PersistentJournalLineageState::Completed
        } else {
            PersistentJournalLineageState::Pending
        };
        if lineage.state != expected_state {
            return Err(super::values::unverifiable_contract());
        }
        lineage.owner_source_range_id = previous_owner.0.clone();
        lineage
            .validate()
            .map_err(|_| super::values::unverifiable_contract())?;
        if lineage.previous_carry_id.is_some() {
            if !persistent_journal_batch_payload_contains_pending_lineage_source(
                &previous_owner.7,
                &lineage,
            ) || !persistent_journal_batch_payload_contains_lineage(
                &current_owner.7,
                &current_owner.0,
                &lineage,
            ) {
                return Err(super::values::unverifiable_contract());
            }
            lineage.owner_source_range_id = current_owner.0.clone();
            if !persistent_journal_batch_payload_contains_lineage(
                &current_owner.7,
                &current_owner.0,
                &lineage,
            ) {
                return Err(super::values::unverifiable_contract());
            }
        } else {
            if !persistent_journal_batch_payload_contains_lineage(
                &previous_owner.7,
                &previous_owner.0,
                &lineage,
            ) {
                return Err(super::values::unverifiable_contract());
            }
            lineage.owner_source_range_id = current_owner.0.clone();
            if !persistent_journal_batch_payload_contains_lineage(
                &current_owner.7,
                &current_owner.0,
                &lineage,
            ) {
                return Err(super::values::unverifiable_contract());
            }
        }
    }
    Ok(())
}

pub(super) fn validate_pending_renames(connection: &Connection) -> Result<(), ScanError> {
    let mut statement = connection
        .prepare(
            "SELECT carry_id, source_range_id, volume_guid, volume_serial, journal_id,
                    file_reference_version, file_reference, old_usn,
                    previous_root_id, previous_root_generation, previous_relative_path,
                    is_directory, enrolled_unix_ms
             FROM library_persistent_journal_pending_renames",
        )
        .map_err(database_error)?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, i64>(5)?,
                row.get::<_, Vec<u8>>(6)?,
                row.get::<_, String>(7)?,
                row.get::<_, String>(8)?,
                row.get::<_, i64>(9)?,
                row.get::<_, String>(10)?,
                row.get::<_, bool>(11)?,
                row.get::<_, i64>(12)?,
            ))
        })
        .map_err(database_error)?;
    for row in rows {
        let row = row.map_err(database_error)?;
        let file_reference = JournalFileReference::from_bytes(&row.6)
            .map_err(|_| super::values::unverifiable_contract())?;
        if i64::from(file_reference.record_version()) != row.5 {
            return Err(super::values::unverifiable_contract());
        }
        PersistentJournalPendingRename {
            carry_id: row.0,
            source_range_id: row.1,
            volume: PersistentJournalVolumeIdentity {
                volume_guid: row.2,
                volume_serial: super::values::parse_canonical_u64(&row.3)?,
            },
            journal_id: JournalIdentifier::parse_canonical(&row.4)
                .map_err(|_| super::values::unverifiable_contract())?,
            file_reference,
            old_usn: JournalUsn::parse_canonical(&row.7)
                .map_err(|_| super::values::unverifiable_contract())?,
            previous_root_id: row.8,
            previous_root_generation: super::values::parse_root_generation(row.9)?,
            previous_relative_path: row.10,
            is_directory: row.11,
            enrolled_unix_ms: row.12,
        }
        .validate()
        .map_err(|_| super::values::unverifiable_contract())?;
    }
    Ok(())
}
