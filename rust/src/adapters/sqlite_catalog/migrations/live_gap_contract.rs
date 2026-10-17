use rusqlite::{Connection, OptionalExtension};

use crate::domain::ScanError;

use super::current_schema::{ContractValidationDepth, record_current_schema_row_audit};
use super::schema_shape::{schema_object_sql_matches, table_columns_match};
use super::{
    LIVE_GAP_RECOVERY_CLAIM_TABLE_DDL, LIVE_GAP_RECOVERY_CONTRACT_TABLE_DDL,
    LIVE_GAP_RECOVERY_IDENTITY_UPDATE_GUARD_DDL, LIVE_GAP_RECOVERY_INSERT_GUARD_DDL,
    LIVE_GAP_RECOVERY_ROOT_INDEX_DDL, database_error,
};

pub(super) fn validate(connection: &Connection) -> Result<(), ScanError> {
    validate_with_depth(connection, ContractValidationDepth::Full)
}

pub(super) fn validate_with_depth(
    connection: &Connection,
    depth: ContractValidationDepth,
) -> Result<(), ScanError> {
    let columns_match = table_columns_match(
        connection,
        "library_live_gap_recovery_contract",
        &[
            ("singleton", "INTEGER", false, 1),
            ("contract_version", "INTEGER", true, 0),
            ("complete", "INTEGER", true, 0),
        ],
    )? && table_columns_match(
        connection,
        "library_live_gap_recovery_claims",
        &[
            ("gap_change_id", "INTEGER", false, 1),
            ("root_id", "TEXT", true, 0),
            ("root_generation", "INTEGER", true, 0),
            ("consumer_kind", "TEXT", true, 0),
            ("opening_volume_guid", "TEXT", false, 0),
            ("opening_volume_serial", "TEXT", false, 0),
            ("opening_root_reference_version", "INTEGER", false, 0),
            ("opening_root_file_reference", "BLOB", false, 0),
            ("opening_journal_id", "TEXT", false, 0),
            ("opening_next_usn", "TEXT", false, 0),
            ("protocol_version", "INTEGER", false, 0),
            ("contract_version", "INTEGER", false, 0),
            ("source_range_id", "TEXT", false, 0),
            ("recovery_change_id", "INTEGER", false, 0),
            ("foreground_scan_id", "TEXT", false, 0),
            ("created_unix_ms", "INTEGER", true, 0),
            ("consumed_unix_ms", "INTEGER", false, 0),
        ],
    )?;
    let schema_matches = schema_object_sql_matches(
        connection,
        "table",
        "library_live_gap_recovery_contract",
        LIVE_GAP_RECOVERY_CONTRACT_TABLE_DDL,
    )? && schema_object_sql_matches(
        connection,
        "table",
        "library_live_gap_recovery_claims",
        LIVE_GAP_RECOVERY_CLAIM_TABLE_DDL,
    )? && schema_object_sql_matches(
        connection,
        "index",
        "library_live_gap_recovery_claims_root",
        LIVE_GAP_RECOVERY_ROOT_INDEX_DDL,
    )? && schema_object_sql_matches(
        connection,
        "trigger",
        "library_live_gap_recovery_claim_insert_guard",
        LIVE_GAP_RECOVERY_INSERT_GUARD_DDL,
    )? && schema_object_sql_matches(
        connection,
        "trigger",
        "library_live_gap_recovery_claim_identity_update_guard",
        LIVE_GAP_RECOVERY_IDENTITY_UPDATE_GUARD_DDL,
    )?;
    let marker_complete = connection
        .query_row(
            "SELECT contract_version = 1 AND complete = 1
             FROM library_live_gap_recovery_contract WHERE singleton = 1",
            [],
            |row| row.get::<_, bool>(0),
        )
        .optional()
        .map_err(database_error)?
        .unwrap_or(false);
    if !columns_match || !schema_matches || !marker_complete {
        return Err(ScanError::new(
            "catalog_live_gap_recovery_contract_unverifiable",
            "The catalog cannot prove live-gap lineage and consumer ownership",
        ));
    }
    if !depth.includes_active_rows() {
        return Ok(());
    }
    if depth.includes_rows() {
        record_current_schema_row_audit();
    }
    let include_terminal_claims = depth.includes_rows();
    let invalid_relations = connection
        .query_row(
            "SELECT EXISTS(
               SELECT 1
               FROM (
                 SELECT * FROM library_live_gap_recovery_claims
                 WHERE ?1 OR consumed_unix_ms IS NULL
               ) AS claim
               LEFT JOIN library_change_queue AS gap ON gap.id = claim.gap_change_id
               LEFT JOIN library_change_queue_lanes AS gap_lane
                 ON gap_lane.change_id = gap.id
               LEFT JOIN library_persistent_journal_source_ranges AS ranges
                 ON ranges.id = claim.source_range_id
               LEFT JOIN library_persistent_journal_range_lifecycle AS lifecycle
                 ON lifecycle.source_range_id = ranges.id
               LEFT JOIN library_change_queue AS recovery
                 ON recovery.id = claim.recovery_change_id
               LEFT JOIN library_change_queue_lanes AS recovery_lane
                 ON recovery_lane.change_id = recovery.id
                LEFT JOIN library_recovery_authorities AS authority
                  ON authority.change_id = recovery.id
                LEFT JOIN scan_runs AS foreground_scan
                  ON foreground_scan.id = claim.foreground_scan_id
                WHERE gap.id IS NULL
                  OR gap.root_id <> claim.root_id
                  OR gap.root_generation <> claim.root_generation
                  OR gap.intent_kind <> 'freshness_unknown'
                  OR gap.scope <> 'root' OR gap.relative_path <> ''
                  OR gap.previous_relative_path IS NOT NULL
                  OR (claim.consumer_kind = 'pending_journal' AND (
                    gap.origin <> 'live_notification' OR gap_lane.lane <> 'p0_live'
                     OR gap.status <> 'retry_wait'
                     OR gap.last_failure_code <> 'live_gap_waiting_for_journal_range'
                     OR gap.authoritative_scan_id IS NOT NULL
                    OR NOT EXISTS(
                      SELECT 1
                      FROM library_persistent_journal_checkpoints AS checkpoint
                      JOIN library_persistent_journal_root_state AS root
                        ON root.root_id = checkpoint.root_id
                       AND root.root_generation = checkpoint.root_generation
                      WHERE checkpoint.root_id = claim.root_id
                        AND checkpoint.root_generation = claim.root_generation
                        AND checkpoint.volume_guid = claim.opening_volume_guid
                        AND checkpoint.volume_serial = claim.opening_volume_serial
                        AND checkpoint.root_reference_version =
                              claim.opening_root_reference_version
                        AND checkpoint.root_file_reference =
                              claim.opening_root_file_reference
                        AND checkpoint.journal_id = claim.opening_journal_id
                        AND checkpoint.next_unread_usn = claim.opening_next_usn
                        AND checkpoint.captured_exclusive_end = claim.opening_next_usn
                        AND checkpoint.protocol_version = claim.protocol_version
                        AND checkpoint.contract_version = claim.contract_version
                        AND checkpoint.continuity_state = 'current'
                        AND checkpoint.last_failure_code IS NULL
                        AND root.capability_state = 'supported'
                        AND root.continuity_state = 'current'
                    )
                  ))
                  OR (claim.consumer_kind = 'journal_source_range' AND (
                    gap.origin <> 'live_notification' OR gap_lane.lane <> 'p0_live'
                    OR gap.status <> 'superseded'
                     OR ranges.id IS NULL OR ranges.status <> 'checkpointed'
                     OR gap.authoritative_scan_id IS NOT NULL
                    OR lifecycle.lifecycle_state NOT IN ('pending', 'completed')
                    OR ranges.root_id <> claim.root_id
                    OR ranges.root_generation <> claim.root_generation
                    OR ranges.volume_guid <> claim.opening_volume_guid
                    OR ranges.volume_serial <> claim.opening_volume_serial
                    OR ranges.journal_id <> claim.opening_journal_id
                    OR ranges.requested_start_usn <> claim.opening_next_usn
                    OR ranges.protocol_version <> claim.protocol_version
                    OR ranges.contract_version <> claim.contract_version
                    OR CAST(ranges.covered_until_usn AS INTEGER)
                         < CAST(claim.opening_next_usn AS INTEGER)
                  ))
                  OR (claim.consumer_kind = 'metadata_inventory_control' AND (
                    gap.origin <> 'live_notification' OR gap_lane.lane <> 'p0_live'
                    OR gap.status <> 'superseded'
                    OR gap.superseded_by_change_id <> recovery.id
                    OR recovery.root_id <> claim.root_id
                     OR recovery.root_generation <> claim.root_generation
                     OR gap.authoritative_scan_id IS NOT NULL
                    OR recovery.origin <> 'metadata_inventory'
                    OR recovery.intent_kind <> 'freshness_unknown'
                    OR recovery.scope <> 'root' OR recovery.relative_path <> ''
                    OR recovery_lane.lane <> 'p2_recovery'
                    OR authority.reason NOT IN (
                      'journal_gap', 'journal_reset', 'journal_trim',
                      'journal_reconstruction_failure', 'containment_failure',
                      'broker_after_current_failure', 'watcher_uncovered_gap'
                    )
                    OR authority.root_id <> claim.root_id
                    OR authority.root_generation <> claim.root_generation
                  ))
                  OR (claim.consumer_kind = 'explicit_recovery_required' AND (
                    gap.origin <> 'startup_catch_up' OR gap_lane.lane <> 'p1_journal'
                     OR gap.status <> 'retry_wait' OR gap.next_retry_unix_ms IS NOT NULL
                     OR gap.authoritative_scan_id IS NOT NULL
                     OR gap.last_failure_code <>
                          'live_gap_v30_explicit_recovery_required'
                   ))
                   OR (claim.consumer_kind = 'foreground_scan' AND (
                     gap.origin <> 'startup_catch_up' OR gap_lane.lane <> 'p1_journal'
                     OR foreground_scan.id IS NULL
                     OR foreground_scan.scan_owner <> 'foreground'
                     OR foreground_scan.root_id <> claim.root_id
                     OR foreground_scan.root_generation_at_start <> claim.root_generation
                     OR (claim.consumed_unix_ms IS NULL AND (
                       foreground_scan.status NOT IN ('running', 'paused')
                       OR gap.status <> 'leased'
                       OR gap.next_retry_unix_ms IS NOT NULL
                       OR gap.authoritative_scan_id IS NOT claim.foreground_scan_id
                       OR gap.last_failure_code <>
                            'live_gap_v30_explicit_recovery_in_progress'
                       OR gap.catalog_revision_at_success IS NOT NULL
                     ))
                     OR (claim.consumed_unix_ms IS NOT NULL AND (
                       foreground_scan.status <> 'completed'
                       OR foreground_scan.completed_unix_ms IS NOT claim.consumed_unix_ms
                       OR gap.status <> 'completed'
                       OR gap.authoritative_scan_id IS NOT NULL
                       OR gap.last_failure_code IS NOT NULL
                       OR gap.last_failure_message IS NOT NULL
                       OR gap.catalog_revision_at_success IS NULL
                     ))
                   ))
             ) OR (?1 AND EXISTS(
               SELECT 1 FROM pragma_foreign_key_check(
                 'library_live_gap_recovery_claims'
               )
             ))",
            [include_terminal_claims],
            |row| row.get::<_, bool>(0),
        )
        .map_err(database_error)?;
    if invalid_relations {
        return Err(ScanError::new(
            "catalog_live_gap_recovery_contract_unverifiable",
            "The catalog cannot prove live-gap lineage and consumer ownership",
        ));
    }
    Ok(())
}
