use rusqlite::Connection;

use crate::domain::{
    JournalFileReference, JournalIdentifier, JournalUsn, PersistentJournalCapability,
    PersistentJournalCheckpoint, PersistentJournalSourceRange, PersistentJournalVolumeIdentity,
    ScanError, persistent_journal_batch_id_from_payload,
    persistent_journal_batch_payload_matches_source_range,
};

use super::super::database_error;

pub(super) fn validate_roots(connection: &Connection) -> Result<(), ScanError> {
    let mut statement = connection
        .prepare(
            "SELECT root_id, root_generation, protocol_version, contract_version,
                    capability_state, continuity_state, last_failure_code,
                    last_failure_message, updated_unix_ms
             FROM library_persistent_journal_root_state",
        )
        .map_err(database_error)?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, Option<String>>(6)?,
                row.get::<_, Option<String>>(7)?,
                row.get::<_, i64>(8)?,
            ))
        })
        .map_err(database_error)?;
    for row in rows {
        let (
            root_id,
            root_generation,
            protocol_version,
            contract_version,
            capability_state,
            continuity_state,
            failure_code,
            failure_message,
            updated_unix_ms,
        ) = row.map_err(database_error)?;
        PersistentJournalCapability {
            root_id,
            root_generation: super::values::parse_root_generation(root_generation)?,
            protocol_version: u16::try_from(protocol_version)
                .map_err(|_| super::values::unverifiable_contract())?,
            contract_version: u16::try_from(contract_version)
                .map_err(|_| super::values::unverifiable_contract())?,
            state: super::values::parse_capability_state(&capability_state)?,
            continuity: super::values::parse_continuity_state(&continuity_state)?,
            failure: super::values::parse_failure(failure_code, failure_message)?,
            updated_unix_ms,
        }
        .validate()
        .map_err(|_| super::values::unverifiable_contract())?;
    }
    Ok(())
}

pub(super) fn validate_checkpoints(connection: &Connection) -> Result<(), ScanError> {
    let mut statement = connection
        .prepare(
            "SELECT root_id, root_generation, volume_guid, volume_serial,
                    root_reference_version, root_file_reference, journal_id,
                    next_unread_usn, captured_exclusive_end, covered_catalog_revision,
                    protocol_version, contract_version, continuity_state,
                    last_failure_code, last_failure_message, updated_unix_ms
             FROM library_persistent_journal_checkpoints",
        )
        .map_err(database_error)?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, i64>(4)?,
                row.get::<_, Vec<u8>>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, String>(7)?,
                row.get::<_, String>(8)?,
                row.get::<_, i64>(9)?,
                row.get::<_, i64>(10)?,
                row.get::<_, i64>(11)?,
                row.get::<_, String>(12)?,
                row.get::<_, Option<String>>(13)?,
                row.get::<_, Option<String>>(14)?,
                row.get::<_, i64>(15)?,
            ))
        })
        .map_err(database_error)?;
    for row in rows {
        let (
            root_id,
            root_generation,
            volume_guid,
            volume_serial,
            reference_version,
            reference,
            journal_id,
            next_unread_usn,
            captured_exclusive_end,
            covered_catalog_revision,
            protocol_version,
            contract_version,
            continuity,
            failure_code,
            failure_message,
            updated_unix_ms,
        ) = row.map_err(database_error)?;
        let root_file_reference = JournalFileReference::from_bytes(&reference)
            .map_err(|_| super::values::unverifiable_contract())?;
        if i64::from(root_file_reference.record_version()) != reference_version {
            return Err(super::values::unverifiable_contract());
        }
        PersistentJournalCheckpoint {
            root_id,
            root_generation: super::values::parse_root_generation(root_generation)?,
            volume: PersistentJournalVolumeIdentity {
                volume_guid,
                volume_serial: super::values::parse_canonical_u64(&volume_serial)?,
            },
            root_file_reference,
            journal_id: JournalIdentifier::parse_canonical(&journal_id)
                .map_err(|_| super::values::unverifiable_contract())?,
            next_unread_usn: JournalUsn::parse_canonical(&next_unread_usn)
                .map_err(|_| super::values::unverifiable_contract())?,
            captured_exclusive_end: JournalUsn::parse_canonical(&captured_exclusive_end)
                .map_err(|_| super::values::unverifiable_contract())?,
            covered_catalog_revision: u64::try_from(covered_catalog_revision)
                .map_err(|_| super::values::unverifiable_contract())?,
            protocol_version: u16::try_from(protocol_version)
                .map_err(|_| super::values::unverifiable_contract())?,
            contract_version: u16::try_from(contract_version)
                .map_err(|_| super::values::unverifiable_contract())?,
            continuity: super::values::parse_continuity_state(&continuity)?,
            failure: super::values::parse_failure(failure_code, failure_message)?,
            updated_unix_ms,
        }
        .validate()
        .map_err(|_| super::values::unverifiable_contract())?;
    }
    Ok(())
}

pub(super) fn validate_ranges(connection: &Connection) -> Result<(), ScanError> {
    let mut statement = connection
        .prepare(
            "SELECT id, root_id, root_generation, volume_guid, volume_serial, journal_id,
                    requested_start_usn, requested_end_usn, covered_until_usn, is_complete,
                    protocol_version, contract_version, status, enrolled_unix_ms,
                    checkpointed_unix_ms, canonical_payload
             FROM library_persistent_journal_source_ranges",
        )
        .map_err(database_error)?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, String>(7)?,
                row.get::<_, String>(8)?,
                row.get::<_, bool>(9)?,
                row.get::<_, i64>(10)?,
                row.get::<_, i64>(11)?,
                row.get::<_, String>(12)?,
                row.get::<_, i64>(13)?,
                row.get::<_, Option<i64>>(14)?,
                row.get::<_, Vec<u8>>(15)?,
            ))
        })
        .map_err(database_error)?;
    for row in rows {
        let (
            batch_id,
            root_id,
            root_generation,
            volume_guid,
            volume_serial,
            journal_id,
            requested_start_usn,
            requested_end_usn,
            covered_until_usn,
            is_complete,
            protocol_version,
            contract_version,
            state,
            enrolled_unix_ms,
            checkpointed_unix_ms,
            canonical_payload,
        ) = row.map_err(database_error)?;
        if persistent_journal_batch_id_from_payload(&canonical_payload) != batch_id {
            return Err(super::values::unverifiable_contract());
        }
        let source_range = PersistentJournalSourceRange {
            batch_id,
            root_id,
            root_generation: super::values::parse_root_generation(root_generation)?,
            volume: PersistentJournalVolumeIdentity {
                volume_guid,
                volume_serial: super::values::parse_canonical_u64(&volume_serial)?,
            },
            journal_id: JournalIdentifier::parse_canonical(&journal_id)
                .map_err(|_| super::values::unverifiable_contract())?,
            requested_start_usn: JournalUsn::parse_canonical(&requested_start_usn)
                .map_err(|_| super::values::unverifiable_contract())?,
            requested_end_usn: JournalUsn::parse_canonical(&requested_end_usn)
                .map_err(|_| super::values::unverifiable_contract())?,
            covered_until_usn: JournalUsn::parse_canonical(&covered_until_usn)
                .map_err(|_| super::values::unverifiable_contract())?,
            is_complete,
            protocol_version: u16::try_from(protocol_version)
                .map_err(|_| super::values::unverifiable_contract())?,
            contract_version: u16::try_from(contract_version)
                .map_err(|_| super::values::unverifiable_contract())?,
            state: super::values::parse_range_state(&state)?,
            enrolled_unix_ms,
            checkpointed_unix_ms,
        };
        source_range
            .validate()
            .map_err(|_| super::values::unverifiable_contract())?;
        if !persistent_journal_batch_payload_matches_source_range(&canonical_payload, &source_range)
        {
            return Err(super::values::unverifiable_contract());
        }
    }
    Ok(())
}
