use rusqlite::Connection;

use crate::domain::{
    JournalFileReference, JournalIdentifier, JournalUsn, PersistentJournalCrossRootLineage,
    PersistentJournalLineageState, PersistentJournalPendingRename, PersistentJournalVolumeIdentity,
    ScanError, persistent_journal_canonical_lineage_entry,
};

use super::super::database_error;

pub(super) struct PersistentJournalLineagePayloadProof {
    pub(super) lineage: PersistentJournalCrossRootLineage,
    pub(super) previous_range_id: String,
    pub(super) current_range_id: String,
}

pub(super) fn load_lineage_proofs(
    connection: &Connection,
) -> Result<Vec<PersistentJournalLineagePayloadProof>, ScanError> {
    let mut statement = connection
        .prepare(
            "SELECT lineage.id, lineage.volume_guid, lineage.volume_serial,
                    lineage.file_reference_version, lineage.file_reference,
                    lineage.previous_root_id, lineage.previous_root_generation,
                    lineage.previous_relative_path, lineage.current_root_id,
                    lineage.current_root_generation, lineage.current_relative_path,
                    lineage.status, lineage.journal_id, lineage.old_usn,
                    lineage.new_usn, lineage.previous_carry_id,
                    previous_owner.source_range_id, current_owner.source_range_id
             FROM library_persistent_journal_cross_root_lineage AS lineage
             JOIN library_persistent_journal_cross_root_ranges AS previous_owner
               ON previous_owner.lineage_id = lineage.id
              AND previous_owner.participant_role = 'previous'
             JOIN library_persistent_journal_cross_root_ranges AS current_owner
               ON current_owner.lineage_id = lineage.id
              AND current_owner.participant_role = 'current'
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
                row.get::<_, String>(16)?,
                row.get::<_, String>(17)?,
            ))
        })
        .map_err(database_error)?;
    let mut proofs = Vec::new();
    for row in rows {
        let row = row.map_err(database_error)?;
        let file_reference = JournalFileReference::from_bytes(&row.4)
            .map_err(|_| super::values::unverifiable_contract())?;
        if i64::from(file_reference.record_version()) != row.3 {
            return Err(super::values::unverifiable_contract());
        }
        let lineage = PersistentJournalCrossRootLineage {
            lineage_id: row.0,
            owner_source_range_id: row.16.clone(),
            volume: PersistentJournalVolumeIdentity {
                volume_guid: row.1,
                volume_serial: super::values::parse_canonical_u64(&row.2)?,
            },
            journal_id: JournalIdentifier::parse_canonical(
                row.12
                    .as_deref()
                    .ok_or_else(super::values::unverifiable_contract)?,
            )
            .map_err(|_| super::values::unverifiable_contract())?,
            file_reference,
            old_usn: row
                .13
                .as_deref()
                .map(JournalUsn::parse_canonical)
                .transpose()
                .map_err(|_| super::values::unverifiable_contract())?,
            new_usn: row
                .14
                .as_deref()
                .map(JournalUsn::parse_canonical)
                .transpose()
                .map_err(|_| super::values::unverifiable_contract())?,
            previous_carry_id: row.15,
            previous_root_id: row.5,
            previous_root_generation: super::values::parse_root_generation(row.6)?,
            previous_relative_path: row.7,
            current_root_id: row.8,
            current_root_generation: super::values::parse_root_generation(row.9)?,
            current_relative_path: row.10,
            state: super::values::parse_lineage_state(&row.11)?,
        };
        lineage
            .validate()
            .map_err(|_| super::values::unverifiable_contract())?;
        proofs.push(PersistentJournalLineagePayloadProof {
            lineage,
            previous_range_id: row.16,
            current_range_id: row.17,
        });
    }
    Ok(proofs)
}

pub(super) fn load_pending_rename_proofs(
    connection: &Connection,
) -> Result<Vec<PersistentJournalPendingRename>, ScanError> {
    let mut statement = connection
        .prepare(
            "SELECT carry_id, source_range_id, volume_guid, volume_serial, journal_id,
                    file_reference_version, file_reference, old_usn,
                    previous_root_id, previous_root_generation, previous_relative_path,
                    is_directory, enrolled_unix_ms
             FROM library_persistent_journal_pending_renames ORDER BY carry_id",
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
    let mut pending_renames = Vec::new();
    for row in rows {
        let row = row.map_err(database_error)?;
        let file_reference = JournalFileReference::from_bytes(&row.6)
            .map_err(|_| super::values::unverifiable_contract())?;
        if i64::from(file_reference.record_version()) != row.5 {
            return Err(super::values::unverifiable_contract());
        }
        let pending = PersistentJournalPendingRename {
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
        };
        pending
            .validate()
            .map_err(|_| super::values::unverifiable_contract())?;
        pending_renames.push(pending);
    }
    Ok(pending_renames)
}

pub(super) fn lineage_payload_candidates(
    lineage: &PersistentJournalCrossRootLineage,
    owner_source_range_id: &str,
    payload_range_id: &str,
) -> Vec<String> {
    let mut candidate = lineage.clone();
    candidate.owner_source_range_id = owner_source_range_id.to_owned();
    let mut candidates = vec![persistent_journal_canonical_lineage_entry(
        &candidate,
        payload_range_id,
    )];
    if candidate.state != PersistentJournalLineageState::Pending {
        candidate.state = PersistentJournalLineageState::Pending;
        candidates.push(persistent_journal_canonical_lineage_entry(
            &candidate,
            payload_range_id,
        ));
    }
    candidates.sort();
    candidates.dedup();
    candidates
}

pub(super) fn pending_rename_matches_consumed_lineage(
    pending: &PersistentJournalPendingRename,
    proof: &PersistentJournalLineagePayloadProof,
) -> bool {
    proof.lineage.previous_carry_id.as_deref() == Some(pending.carry_id.as_str())
        && proof.previous_range_id == pending.source_range_id
        && proof.lineage.volume == pending.volume
        && proof.lineage.journal_id == pending.journal_id
        && proof.lineage.file_reference == pending.file_reference
        && proof.lineage.old_usn == Some(pending.old_usn)
        && proof.lineage.previous_root_id == pending.previous_root_id
        && proof.lineage.previous_root_generation == pending.previous_root_generation
        && proof.lineage.previous_relative_path == pending.previous_relative_path
}
