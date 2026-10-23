use rusqlite::{Connection, OptionalExtension, params};

use crate::domain::{
    JournalIdentifier, JournalUsn, LibraryChangeId, LibraryRootGeneration,
    PersistentJournalBaseline, PersistentJournalBaselinePhase, PersistentJournalVolumeIdentity,
    ScanError,
};

use super::super::{database_error, sqlite_integer, sqlite_unsigned};

use super::{
    invalid_storage, load_checkpoint, parse_canonical_u64, parse_file_reference,
    parse_root_generation, parse_u16,
};

pub(super) fn load_active_baselines(
    connection: &Connection,
) -> Result<Vec<PersistentJournalBaseline>, ScanError> {
    let mut statement = connection
        .prepare(
            "SELECT baseline.change_id
             FROM library_persistent_journal_baselines AS baseline
             JOIN library_recovery_authorities AS authority
               ON authority.change_id = baseline.change_id
             WHERE baseline.phase <> 'completed' AND authority.retired_unix_ms IS NULL
               AND NOT EXISTS (SELECT 1 FROM library_metadata_fallbacks AS fallback
                 WHERE fallback.change_id = baseline.change_id)
             ORDER BY baseline.root_id, baseline.root_generation",
        )
        .map_err(database_error)?;
    let ids = statement
        .query_map([], |row| row.get::<_, i64>(0))
        .map_err(database_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(database_error)?;
    ids.into_iter()
        .map(|id| {
            let id = LibraryChangeId::new(sqlite_unsigned(id, "baseline change ID")?)
                .ok_or_else(|| invalid_storage("baseline change ID"))?;
            load_baseline(connection, id)?
                .ok_or_else(|| invalid_storage("persistent journal baseline"))
        })
        .collect()
}

pub(in crate::adapters::sqlite_catalog) fn load_baseline_for_root(
    connection: &Connection,
    root_id: &str,
    root_generation: LibraryRootGeneration,
) -> Result<Option<PersistentJournalBaseline>, ScanError> {
    let change_id = connection
        .query_row(
            "SELECT change_id FROM library_persistent_journal_baselines AS baseline
             WHERE root_id = ?1 AND root_generation = ?2
               AND NOT EXISTS (SELECT 1 FROM library_metadata_fallbacks AS fallback
                 WHERE fallback.change_id = baseline.change_id)",
            params![
                root_id,
                sqlite_integer(root_generation.value(), "root generation")?,
            ],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .map_err(database_error)?;
    change_id
        .map(|value| {
            LibraryChangeId::new(sqlite_unsigned(value, "baseline change ID")?)
                .ok_or_else(|| invalid_storage("baseline change ID"))
        })
        .transpose()?
        .map(|change_id| load_baseline(connection, change_id))
        .transpose()
        .map(Option::flatten)
}

pub(super) fn load_baseline(
    connection: &Connection,
    change_id: LibraryChangeId,
) -> Result<Option<PersistentJournalBaseline>, ScanError> {
    let stored = connection
        .query_row(
            "SELECT authority.run_id, baseline.root_id, baseline.root_generation,
                    baseline.volume_guid, baseline.volume_serial,
                    baseline.root_reference_version, baseline.root_file_reference,
                    baseline.journal_id, baseline.opening_next_usn,
                    baseline.closing_next_usn, baseline.protocol_version,
                    baseline.contract_version, baseline.phase,
                    baseline.authorized_unix_ms, baseline.updated_unix_ms,
                    baseline.completed_unix_ms
             FROM library_persistent_journal_baselines AS baseline
             JOIN library_recovery_authorities AS authority
               ON authority.change_id = baseline.change_id
             WHERE baseline.change_id = ?1",
            [sqlite_integer(change_id.value(), "baseline change ID")?],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, i64>(5)?,
                    row.get::<_, Vec<u8>>(6)?,
                    row.get::<_, String>(7)?,
                    row.get::<_, String>(8)?,
                    row.get::<_, Option<String>>(9)?,
                    row.get::<_, i64>(10)?,
                    row.get::<_, i64>(11)?,
                    row.get::<_, String>(12)?,
                    row.get::<_, i64>(13)?,
                    row.get::<_, i64>(14)?,
                    row.get::<_, Option<i64>>(15)?,
                ))
            },
        )
        .optional()
        .map_err(database_error)?;
    let Some(stored) = stored else {
        return Ok(None);
    };
    let baseline = PersistentJournalBaseline {
        change_id,
        run_id: stored.0,
        root_id: stored.1,
        root_generation: parse_root_generation(stored.2)?,
        volume: PersistentJournalVolumeIdentity {
            volume_guid: stored.3,
            volume_serial: parse_canonical_u64(&stored.4, "baseline volume serial")?,
        },
        root_file_reference: parse_file_reference(stored.5, &stored.6)?,
        journal_id: JournalIdentifier::parse_canonical(&stored.7)?,
        opening_next_usn: JournalUsn::parse_canonical(&stored.8)?,
        closing_next_usn: stored
            .9
            .as_deref()
            .map(JournalUsn::parse_canonical)
            .transpose()?,
        protocol_version: parse_u16(stored.10, "baseline protocol version")?,
        contract_version: parse_u16(stored.11, "baseline contract version")?,
        phase: parse_baseline_phase(&stored.12)?,
        authorized_unix_ms: stored.13,
        updated_unix_ms: stored.14,
        completed_unix_ms: stored.15,
    };
    baseline.validate()?;
    Ok(Some(baseline))
}

pub(in crate::adapters::sqlite_catalog) fn baseline_closing_is_covered(
    connection: &Connection,
    change_id: LibraryChangeId,
) -> Result<bool, ScanError> {
    let Some(baseline) = load_baseline(connection, change_id)? else {
        return Ok(false);
    };
    let Some(closing) = baseline.closing_next_usn else {
        return Ok(false);
    };
    if !matches!(
        baseline.phase,
        PersistentJournalBaselinePhase::Replay | PersistentJournalBaselinePhase::Absence
    ) {
        return Ok(false);
    }
    let authority_is_active = connection
        .query_row(
            "SELECT retired_unix_ms IS NULL FROM library_recovery_authorities
             WHERE change_id = ?1",
            [sqlite_integer(change_id.value(), "baseline change ID")?],
            |row| row.get::<_, bool>(0),
        )
        .optional()
        .map_err(database_error)?
        .unwrap_or(false);
    if !authority_is_active {
        return Ok(false);
    }
    let Some(checkpoint) =
        load_checkpoint(connection, &baseline.root_id, baseline.root_generation)?
    else {
        return Ok(false);
    };
    let identity_matches = checkpoint.volume == baseline.volume
        && checkpoint.root_file_reference == baseline.root_file_reference
        && checkpoint.journal_id == baseline.journal_id
        && checkpoint.protocol_version == baseline.protocol_version
        && checkpoint.contract_version == baseline.contract_version
        && checkpoint.next_unread_usn >= closing;
    if !identity_matches {
        return Ok(false);
    }
    if closing == baseline.opening_next_usn {
        return Ok(baseline.phase == PersistentJournalBaselinePhase::Absence);
    }
    let has_unfinished_replay = connection
        .query_row(
            "SELECT EXISTS(
               SELECT 1
               FROM library_persistent_journal_source_ranges AS ranges
               JOIN library_persistent_journal_range_lifecycle AS lifecycle
                 ON lifecycle.source_range_id = ranges.id
               WHERE ranges.root_id = ?1
                 AND ranges.root_generation = ?2
                 AND ranges.volume_guid = ?3
                 AND ranges.volume_serial = ?4
                 AND ranges.journal_id = ?5
                 AND CAST(ranges.requested_start_usn AS INTEGER) < ?6
                 AND CAST(ranges.covered_until_usn AS INTEGER) > ?7
                 AND lifecycle.lifecycle_state <> 'completed'
             )",
            params![
                baseline.root_id,
                sqlite_integer(baseline.root_generation.value(), "root generation")?,
                baseline.volume.volume_guid,
                baseline.volume.volume_serial.to_string(),
                baseline.journal_id.to_canonical_text(),
                closing.value(),
                baseline.opening_next_usn.value(),
            ],
            |row| row.get::<_, bool>(0),
        )
        .map_err(database_error)?;
    Ok(!has_unfinished_replay)
}

fn parse_baseline_phase(value: &str) -> Result<PersistentJournalBaselinePhase, ScanError> {
    match value {
        "inventory" => Ok(PersistentJournalBaselinePhase::Inventory),
        "replay" => Ok(PersistentJournalBaselinePhase::Replay),
        "absence" => Ok(PersistentJournalBaselinePhase::Absence),
        "completed" => Ok(PersistentJournalBaselinePhase::Completed),
        _ => Err(invalid_storage("persistent journal baseline phase")),
    }
}
