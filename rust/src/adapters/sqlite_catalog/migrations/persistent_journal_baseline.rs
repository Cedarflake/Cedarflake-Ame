use rusqlite::Connection;

use crate::domain::{JournalFileReference, JournalIdentifier, JournalUsn, ScanError};

use super::{
    ContractValidationDepth, PERSISTENT_JOURNAL_BASELINE_INSERT_GUARD_DDL,
    PERSISTENT_JOURNAL_BASELINE_ROOT_INDEX_DDL, PERSISTENT_JOURNAL_BASELINE_TABLE_DDL,
    PERSISTENT_JOURNAL_BASELINE_UPDATE_GUARD_DDL, database_error, record_current_schema_row_audit,
    schema_object_sql_matches, table_columns_match,
};

#[cfg(test)]
mod tests;

pub(super) fn validate_persistent_journal_baseline_contract(
    connection: &Connection,
) -> Result<(), ScanError> {
    validate_persistent_journal_baseline_contract_with_depth(
        connection,
        ContractValidationDepth::Full,
    )
}

pub(super) fn validate_persistent_journal_baseline_contract_with_depth(
    connection: &Connection,
    depth: ContractValidationDepth,
) -> Result<(), ScanError> {
    let columns_match = table_columns_match(
        connection,
        "library_persistent_journal_baselines",
        &[
            ("change_id", "INTEGER", false, 1),
            ("root_id", "TEXT", true, 0),
            ("root_generation", "INTEGER", true, 0),
            ("volume_guid", "TEXT", true, 0),
            ("volume_serial", "TEXT", true, 0),
            ("root_reference_version", "INTEGER", true, 0),
            ("root_file_reference", "BLOB", true, 0),
            ("journal_id", "TEXT", true, 0),
            ("opening_next_usn", "TEXT", true, 0),
            ("closing_next_usn", "TEXT", false, 0),
            ("protocol_version", "INTEGER", true, 0),
            ("contract_version", "INTEGER", true, 0),
            ("phase", "TEXT", true, 0),
            ("authorized_unix_ms", "INTEGER", true, 0),
            ("updated_unix_ms", "INTEGER", true, 0),
            ("completed_unix_ms", "INTEGER", false, 0),
        ],
    )?;
    let schema_matches = schema_object_sql_matches(
        connection,
        "table",
        "library_persistent_journal_baselines",
        PERSISTENT_JOURNAL_BASELINE_TABLE_DDL,
    )? && schema_object_sql_matches(
        connection,
        "index",
        "library_persistent_journal_baselines_root",
        PERSISTENT_JOURNAL_BASELINE_ROOT_INDEX_DDL,
    )? && schema_object_sql_matches(
        connection,
        "trigger",
        "library_persistent_journal_baseline_insert_guard",
        PERSISTENT_JOURNAL_BASELINE_INSERT_GUARD_DDL,
    )? && schema_object_sql_matches(
        connection,
        "trigger",
        "library_persistent_journal_baseline_update_guard",
        PERSISTENT_JOURNAL_BASELINE_UPDATE_GUARD_DDL,
    )?;
    let foreign_keys_match = connection
        .query_row(
            "SELECT COUNT(*) = 3
               AND EXISTS(
                 SELECT 1 FROM pragma_foreign_key_list('library_persistent_journal_baselines')
                 WHERE \"table\" = 'library_recovery_authorities'
                   AND \"from\" = 'change_id' AND \"to\" = 'change_id'
                   AND on_update = 'NO ACTION' AND on_delete = 'CASCADE'
                   AND \"match\" = 'NONE'
               )
               AND EXISTS(
                 SELECT 1
                 FROM pragma_foreign_key_list('library_persistent_journal_baselines') AS root_id
                 JOIN pragma_foreign_key_list('library_persistent_journal_baselines') AS generation
                   ON generation.id = root_id.id AND generation.seq = 1
                 WHERE root_id.seq = 0
                   AND root_id.\"table\" = 'library_persistent_journal_root_state'
                   AND generation.\"table\" = 'library_persistent_journal_root_state'
                   AND root_id.\"from\" = 'root_id' AND root_id.\"to\" = 'root_id'
                   AND generation.\"from\" = 'root_generation'
                   AND generation.\"to\" = 'root_generation'
                   AND root_id.on_update = 'NO ACTION' AND root_id.on_delete = 'CASCADE'
                   AND generation.on_update = 'NO ACTION'
                   AND generation.on_delete = 'CASCADE'
                   AND root_id.\"match\" = 'NONE' AND generation.\"match\" = 'NONE'
               )
             FROM pragma_foreign_key_list('library_persistent_journal_baselines')",
            [],
            |row| row.get::<_, bool>(0),
        )
        .map_err(database_error)?;
    if !columns_match || !schema_matches || !foreign_keys_match {
        return Err(ScanError::new(
            "catalog_persistent_journal_baseline_contract_unverifiable",
            "The catalog cannot prove its one-time persistent journal baseline authority",
        ));
    }
    if !depth.includes_rows() {
        return Ok(());
    }
    record_current_schema_row_audit();
    // Each historical window proves its immediate successor; only the last window
    // binds the mutable checkpoint. The row audit below also validates every successor.
    let invalid_rows = connection
        .query_row(
            "WITH ordered_baselines AS (
               SELECT *, LEAD(change_id) OVER (
                 PARTITION BY root_id, root_generation
                 ORDER BY authorized_unix_ms, change_id
               ) AS successor_change_id
               FROM library_persistent_journal_baselines
             )
             SELECT EXISTS(
               SELECT 1 FROM ordered_baselines AS baseline
                LEFT JOIN library_persistent_journal_baselines AS successor
                  ON successor.change_id = baseline.successor_change_id
                LEFT JOIN library_recovery_authorities AS successor_authority
                  ON successor_authority.change_id = successor.change_id
                LEFT JOIN library_recovery_authorities AS authority
                  ON authority.change_id = baseline.change_id
                LEFT JOIN library_change_queue AS queue ON queue.id = baseline.change_id
                LEFT JOIN library_persistent_journal_root_state AS root
                 ON root.root_id = baseline.root_id
                AND root.root_generation = baseline.root_generation
               LEFT JOIN library_persistent_journal_checkpoints AS checkpoint
                 ON checkpoint.root_id = baseline.root_id
                AND checkpoint.root_generation = baseline.root_generation
               WHERE authority.change_id IS NULL OR root.root_id IS NULL
                  OR authority.root_id <> baseline.root_id
                  OR authority.root_generation <> baseline.root_generation
                  OR authority.reason NOT IN (
                    'existing_root_baseline', 'first_import_boundary',
                    'watcher_uncovered_gap', 'journal_gap', 'journal_reset', 'journal_trim',
                    'journal_reconstruction_failure', 'containment_failure',
                    'broker_after_current_failure'
                  )
                  OR (authority.reason IN (
                        'existing_root_baseline', 'first_import_boundary'
                      ) AND (
                        authority.opening_journal_id <> baseline.journal_id
                        OR authority.opening_next_usn <> baseline.opening_next_usn
                      ))
                  OR (authority.reason IN (
                        'watcher_uncovered_gap', 'journal_gap', 'journal_reset', 'journal_trim',
                        'journal_reconstruction_failure', 'containment_failure',
                        'broker_after_current_failure'
                      ) AND (
                        authority.opening_journal_id IS NOT NULL
                        OR authority.opening_next_usn IS NOT NULL
                      ))
                   OR (baseline.phase = 'completed' AND NOT (
                         (
                           authority.retired_unix_ms IS NOT NULL
                           AND queue.id IS NOT NULL
                           AND queue.status IN ('completed', 'superseded')
                           AND (
                             (
                               successor.change_id IS NULL
                               AND checkpoint.root_id IS NOT NULL
                               AND root.continuity_state = checkpoint.continuity_state
                               AND checkpoint.continuity_state IN ('current', 'catching_up')
                               AND checkpoint.volume_guid = baseline.volume_guid
                               AND checkpoint.volume_serial = baseline.volume_serial
                               AND checkpoint.root_reference_version = baseline.root_reference_version
                               AND checkpoint.root_file_reference = baseline.root_file_reference
                               AND checkpoint.journal_id = baseline.journal_id
                               AND checkpoint.protocol_version = baseline.protocol_version
                               AND checkpoint.contract_version = baseline.contract_version
                               AND CAST(checkpoint.next_unread_usn AS INTEGER)
                                   >= CAST(baseline.closing_next_usn AS INTEGER)
                               AND CAST(checkpoint.captured_exclusive_end AS INTEGER)
                                   >= CAST(checkpoint.next_unread_usn AS INTEGER)
                               AND (
                                 checkpoint.continuity_state <> 'current'
                                 OR checkpoint.next_unread_usn = checkpoint.captured_exclusive_end
                               )
                               AND (
                                 (checkpoint.continuity_state = 'current'
                                   AND checkpoint.next_unread_usn = baseline.closing_next_usn)
                                 OR EXISTS (
                                   SELECT 1
                                   FROM library_persistent_journal_source_ranges AS ranges
                                   JOIN library_persistent_journal_range_lifecycle AS lifecycle
                                     ON lifecycle.source_range_id = ranges.id
                                   WHERE ranges.root_id = baseline.root_id
                                     AND ranges.root_generation = baseline.root_generation
                                     AND ranges.volume_guid = checkpoint.volume_guid
                                     AND ranges.volume_serial = checkpoint.volume_serial
                                     AND ranges.journal_id = checkpoint.journal_id
                                     AND ranges.protocol_version = checkpoint.protocol_version
                                     AND ranges.contract_version = checkpoint.contract_version
                                     AND ranges.requested_end_usn = checkpoint.captured_exclusive_end
                                     AND CAST(ranges.requested_start_usn AS INTEGER)
                                         >= CAST(baseline.closing_next_usn AS INTEGER)
                                     AND (
                                       (ranges.status = 'checkpointed'
                                         AND ranges.covered_until_usn = checkpoint.next_unread_usn
                                         AND lifecycle.lifecycle_state IN ('pending', 'completed')
                                         AND (checkpoint.continuity_state <> 'current'
                                           OR lifecycle.lifecycle_state = 'completed'))
                                       OR (ranges.status = 'enrolled'
                                         AND lifecycle.lifecycle_state = 'pending'
                                         AND checkpoint.continuity_state = 'catching_up'
                                         AND ranges.requested_start_usn = checkpoint.next_unread_usn)
                                     )
                                 )
                               )
                             ) OR (
                               successor.change_id IS NOT NULL
                               AND successor_authority.change_id IS NOT NULL
                               AND successor.change_id > baseline.change_id
                               AND successor_authority.change_id = successor.change_id
                               AND successor_authority.root_id = baseline.root_id
                               AND successor_authority.root_generation = baseline.root_generation
                               AND successor_authority.reason IN (
                                 'watcher_uncovered_gap', 'journal_gap', 'journal_reset',
                                 'journal_trim', 'journal_reconstruction_failure',
                                 'containment_failure', 'broker_after_current_failure'
                               )
                               AND successor_authority.authorized_unix_ms
                                   >= baseline.completed_unix_ms
                               AND successor_authority.authorized_unix_ms
                                   >= authority.retired_unix_ms
                               AND successor_authority.authorized_unix_ms >= queue.updated_unix_ms
                               AND successor.authorized_unix_ms
                                   >= successor_authority.authorized_unix_ms
                               AND successor.volume_guid = baseline.volume_guid
                               AND successor.volume_serial = baseline.volume_serial
                               AND successor.root_reference_version = baseline.root_reference_version
                               AND successor.root_file_reference = baseline.root_file_reference
                               AND successor.protocol_version = baseline.protocol_version
                               AND successor.contract_version = baseline.contract_version
                               AND (
                                 (successor_authority.reason = 'journal_reset'
                                   AND successor.journal_id <> baseline.journal_id
                                   AND (successor.phase <> 'inventory'
                                     OR checkpoint.journal_id = baseline.journal_id))
                                 OR (successor_authority.reason <> 'journal_reset'
                                   AND successor.journal_id = baseline.journal_id
                                   AND CAST(successor.opening_next_usn AS INTEGER)
                                       >= CAST(baseline.closing_next_usn AS INTEGER))
                               )
                             )
                           )
                         ) OR (
                           authority.retired_unix_ms IS NOT NULL
                           AND queue.status IN ('completed', 'superseded')
                           AND queue.last_failure_code =
                             'metadata_inventory_v28_recapture_required'
                           AND root.continuity_state = 'recovery_required'
                           AND checkpoint.root_id IS NOT NULL
                           AND checkpoint.continuity_state = 'recovery_required'
                           AND checkpoint.last_failure_code =
                             'metadata_inventory_v28_recapture_required'
                           AND checkpoint.volume_guid = baseline.volume_guid
                           AND checkpoint.volume_serial = baseline.volume_serial
                           AND checkpoint.root_reference_version = baseline.root_reference_version
                           AND checkpoint.root_file_reference = baseline.root_file_reference
                           AND checkpoint.journal_id = baseline.journal_id
                           AND checkpoint.next_unread_usn = baseline.closing_next_usn
                           AND checkpoint.captured_exclusive_end = baseline.closing_next_usn
                         )
                       ))
                  OR (baseline.phase <> 'completed' AND (
                        authority.retired_unix_ms IS NOT NULL
                        OR root.continuity_state = 'current'
                        OR checkpoint.continuity_state = 'current'
                      ))
                  OR (baseline.phase = 'inventory'
                      AND authority.reason IN (
                        'existing_root_baseline', 'first_import_boundary'
                      ) AND (
                        root.continuity_state <> 'baseline_required'
                        OR checkpoint.root_id IS NOT NULL
                      ))
                  OR (baseline.phase = 'inventory'
                      AND authority.reason IN (
                        'watcher_uncovered_gap', 'journal_gap', 'journal_reset', 'journal_trim',
                        'journal_reconstruction_failure', 'containment_failure',
                        'broker_after_current_failure'
                      ) AND (
                        root.continuity_state <> 'recovery_required'
                        OR checkpoint.root_id IS NULL
                        OR checkpoint.continuity_state <> 'recovery_required'
                        OR checkpoint.volume_guid <> baseline.volume_guid
                        OR checkpoint.volume_serial <> baseline.volume_serial
                        OR checkpoint.root_reference_version <> baseline.root_reference_version
                        OR checkpoint.root_file_reference <> baseline.root_file_reference
                        OR checkpoint.captured_exclusive_end <> checkpoint.next_unread_usn
                        OR (authority.reason = 'journal_reset'
                          AND checkpoint.journal_id = baseline.journal_id)
                        OR (authority.reason <> 'journal_reset' AND (
                          checkpoint.journal_id <> baseline.journal_id
                          OR CAST(checkpoint.next_unread_usn AS INTEGER)
                            > CAST(baseline.opening_next_usn AS INTEGER)
                        ))
                      ))
                  OR (baseline.phase IN ('replay', 'absence') AND (
                        root.continuity_state <> 'catching_up'
                        OR checkpoint.root_id IS NULL
                        OR checkpoint.continuity_state <> 'catching_up'
                        OR checkpoint.volume_guid <> baseline.volume_guid
                        OR checkpoint.volume_serial <> baseline.volume_serial
                        OR checkpoint.root_reference_version <> baseline.root_reference_version
                        OR checkpoint.root_file_reference <> baseline.root_file_reference
                        OR checkpoint.journal_id <> baseline.journal_id
                        OR checkpoint.captured_exclusive_end <> baseline.closing_next_usn
                        OR CAST(checkpoint.next_unread_usn AS INTEGER)
                          > CAST(baseline.closing_next_usn AS INTEGER)
                        OR (baseline.phase = 'absence'
                          AND checkpoint.next_unread_usn <> baseline.closing_next_usn)
                      ))
             ) OR EXISTS(
               SELECT 1 FROM pragma_foreign_key_check('library_persistent_journal_baselines')
             ) OR EXISTS(
               SELECT 1
               FROM library_persistent_journal_baselines AS baseline
               JOIN library_metadata_inventory_runs AS run
                 ON run.root_id = baseline.root_id
                AND run.root_generation = baseline.root_generation
                AND run.status IN ('running', 'comparing')
               LEFT JOIN library_recovery_authorities AS authority
                 ON authority.run_id = run.id
                AND authority.retired_unix_ms IS NULL
               WHERE baseline.phase <> 'completed'
                 AND (authority.change_id IS NULL
                   OR authority.change_id <> baseline.change_id)
             ) OR EXISTS(
               SELECT 1
               FROM library_persistent_journal_baselines AS baseline
               JOIN library_recovery_authorities AS authority
                 ON authority.change_id = baseline.change_id
               JOIN library_metadata_inventory_runs AS run
                 ON run.id = authority.run_id
               WHERE baseline.phase <> 'completed'
                 AND run.status NOT IN ('running', 'comparing')
             )",
            [],
            |row| row.get::<_, bool>(0),
        )
        .map_err(database_error)?;
    let mut rows = connection
        .prepare(
            "SELECT volume_guid, volume_serial, root_reference_version,
                    root_file_reference, journal_id, opening_next_usn,
                    closing_next_usn, protocol_version, contract_version,
                    phase, authorized_unix_ms, updated_unix_ms, completed_unix_ms
             FROM library_persistent_journal_baselines",
        )
        .map_err(database_error)?;
    let canonical_rows = rows
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, Vec<u8>>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, Option<String>>(6)?,
                row.get::<_, i64>(7)?,
                row.get::<_, i64>(8)?,
                row.get::<_, String>(9)?,
                row.get::<_, i64>(10)?,
                row.get::<_, i64>(11)?,
                row.get::<_, Option<i64>>(12)?,
            ))
        })
        .map_err(database_error)?
        .all(|row| {
            row.is_ok_and(
                |(
                    volume_guid,
                    volume_serial,
                    reference_version,
                    reference,
                    journal_id,
                    opening,
                    closing,
                    protocol_version,
                    contract_version,
                    phase,
                    authorized,
                    updated,
                    completed,
                )| {
                    let volume = crate::domain::PersistentJournalVolumeIdentity {
                        volume_guid,
                        volume_serial: match volume_serial.parse() {
                            Ok(value) => value,
                            Err(_) => return false,
                        },
                    };
                    let reference = match JournalFileReference::from_bytes(&reference) {
                        Ok(value) => value,
                        Err(_) => return false,
                    };
                    volume.validate().is_ok()
                        && i64::from(reference.record_version()) == reference_version
                        && JournalIdentifier::parse_canonical(&journal_id).is_ok()
                        && JournalUsn::parse_canonical(&opening).is_ok()
                        && closing
                            .as_deref()
                            .is_none_or(|value| JournalUsn::parse_canonical(value).is_ok())
                        && u16::try_from(protocol_version).is_ok_and(|value| value > 0)
                        && contract_version
                            == i64::from(crate::domain::PERSISTENT_JOURNAL_CONTRACT_VERSION)
                        && matches!(
                            phase.as_str(),
                            "inventory" | "replay" | "absence" | "completed"
                        )
                        && authorized >= 0
                        && updated >= authorized
                        && completed.is_none_or(|value| value >= updated)
                },
            )
        });
    if invalid_rows || !canonical_rows {
        return Err(ScanError::new(
            "catalog_persistent_journal_baseline_contract_unverifiable",
            "The catalog cannot prove its one-time persistent journal baseline authority",
        ));
    }
    Ok(())
}
