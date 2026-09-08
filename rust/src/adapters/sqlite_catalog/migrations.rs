use rusqlite::{Connection, OptionalExtension, Transaction, TransactionBehavior, params};

use crate::domain::{
    JournalFileReference, JournalIdentifier, JournalUsn, LibraryChangeIntent,
    LibraryChangeIntentKind, LibraryChangeOrigin, LibraryChangeScope, LibraryRootGeneration,
    PERSISTENT_JOURNAL_CONTRACT_VERSION, PersistentJournalCapability,
    PersistentJournalCapabilityState, PersistentJournalCheckpoint,
    PersistentJournalContinuityState, PersistentJournalCrossRootLineage,
    PersistentJournalEnrollmentBatch, PersistentJournalFailure, PersistentJournalLineageState,
    PersistentJournalPendingRename, PersistentJournalRangeState, PersistentJournalSourceRange,
    PersistentJournalVolumeIdentity, ScanError, persistent_journal_batch_id_from_payload,
    persistent_journal_batch_payload, persistent_journal_batch_payload_children,
    persistent_journal_batch_payload_contains_lineage,
    persistent_journal_batch_payload_contains_pending_lineage_source,
    persistent_journal_batch_payload_matches_source_range,
    persistent_journal_canonical_intent_entry, persistent_journal_canonical_lineage_entry,
    persistent_journal_pending_rename_from_payload_entry,
};

use super::{
    MAX_SCAN_CATCH_UP_LINEAGE, SCHEMA_VERSION, SQLITE_APPLICATION_ID, database_error,
    natural_name_key, parent_relative_path, unix_time_ms,
};

mod current_schema;
mod retired_root_authority;

use current_schema::{
    ContractValidationDepth, validate_current_schema_contract,
    validate_current_schema_contract_with_source_revision_rows,
};

pub(super) fn prepare_fresh_catalog_auto_vacuum(connection: &Connection) -> Result<(), ScanError> {
    let user_schema_objects: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE name NOT LIKE 'sqlite_%'",
            [],
            |row| row.get(0),
        )
        .map_err(database_error)?;
    if user_schema_objects == 0 {
        connection
            .execute_batch("PRAGMA auto_vacuum = INCREMENTAL;")
            .map_err(database_error)?;
    }
    Ok(())
}

pub(super) fn migrate_schema(connection: &mut Connection) -> Result<(), ScanError> {
    let has_schema_info: bool = connection
        .query_row(
            "SELECT EXISTS(
               SELECT 1 FROM sqlite_master
               WHERE type = 'table' AND name = 'schema_info'
             )",
            [],
            |row| row.get(0),
        )
        .map_err(database_error)?;

    if !has_schema_info {
        prepare_fresh_catalog_auto_vacuum(connection)?;
        let transaction = connection.transaction().map_err(database_error)?;
        create_schema_v19(&transaction)?;
        migrate_v19_to_v20_transaction(&transaction)?;
        migrate_v20_to_v21_transaction(&transaction)?;
        migrate_v21_to_v22_transaction(&transaction)?;
        migrate_v22_to_v23_transaction(&transaction)?;
        migrate_v23_to_v24_transaction(&transaction)?;
        migrate_v24_to_v25_transaction(&transaction)?;
        migrate_v25_to_v26_transaction(&transaction)?;
        migrate_v26_to_v27_transaction(&transaction)?;
        migrate_v27_to_v28_transaction(&transaction)?;
        migrate_v28_to_v29_transaction(&transaction)?;
        migrate_v29_to_v30_transaction(&transaction)?;
        migrate_v30_to_v31_transaction(&transaction)?;
        return transaction.commit().map_err(database_error);
    }

    loop {
        let version: i64 = connection
            .query_row("SELECT version FROM schema_info LIMIT 1", [], |row| {
                row.get(0)
            })
            .map_err(database_error)?;
        match version {
            SCHEMA_VERSION => {
                repair_prerelease_v24_source_range_id_triggers(connection)?;
                repair_prerelease_v26_recovery_window_schema(connection)?;
                repair_and_validate_current_terminal_recovery_state(connection)?;
                repair_prerelease_v31_source_revision_metadata(connection)?;
                recover_interrupted_explicit_foreground_claims(connection)?;
                return Ok(());
            }
            1 => migrate_v1_to_v2(connection)?,
            2 => migrate_v2_to_v3(connection)?,
            3 => migrate_v3_to_v4(connection)?,
            4 => migrate_v4_to_v5(connection)?,
            5 => migrate_v5_to_v6(connection)?,
            6 => migrate_v6_to_v7(connection)?,
            7 => migrate_v7_to_v8(connection)?,
            8 => migrate_v8_to_v9(connection)?,
            9 => migrate_v9_to_v10(connection)?,
            10 => migrate_v10_to_v11(connection)?,
            11 => migrate_v11_to_v12(connection)?,
            12 => migrate_v12_to_v13(connection)?,
            13 => migrate_v13_to_v14(connection)?,
            14 => migrate_v14_to_v15(connection)?,
            15 => migrate_v15_to_v16(connection)?,
            16 => migrate_v16_to_v17(connection)?,
            17 => {
                validate_change_queue_authority(connection)?;
                migrate_v17_to_v18(connection)?;
            }
            18 => {
                repair_prerelease_v18_scan_owner_index(connection)?;
                migrate_v18_to_v19(connection)?;
            }
            19 => {
                validate_prerelease_v19_catch_up_authority(connection)?;
                repair_prerelease_v19_derived_indexes(connection)?;
                repair_prerelease_v19_scan_lineage(connection)?;
                repair_prerelease_v19_scan_handoff_batches(connection)?;
                repair_v19_preview_expectations(connection)?;
                validate_v19_schema_contract(connection)?;
                migrate_v19_to_v20(connection)?;
            }
            20 => {
                validate_metadata_inventory_contract(connection)?;
                migrate_v20_to_v21(connection)?;
            }
            21 => {
                validate_v19_schema_contract(connection)?;
                validate_metadata_inventory_contract(connection)?;
                validate_terminal_media_evidence_contract(connection)?;
                migrate_v21_to_v22(connection)?;
            }
            22 => migrate_v22_to_v23(connection)?,
            23 => migrate_v23_to_v24(connection)?,
            24 => migrate_v24_to_v25(connection)?,
            25 => migrate_v25_to_v26(connection)?,
            26 => migrate_v26_to_v27(connection)?,
            27 => migrate_v27_to_v28(connection)?,
            28 => migrate_v28_to_v29(connection)?,
            29 => migrate_v29_to_v30(connection)?,
            30 => migrate_v30_to_v31(connection)?,
            _ => {
                return Err(ScanError::new(
                    "catalog_schema_unsupported",
                    format!("Expected catalog schema {SCHEMA_VERSION}, found {version}"),
                ));
            }
        }
    }
}

fn create_schema_v19(transaction: &Transaction<'_>) -> Result<(), ScanError> {
    transaction
        .execute_batch(
            "CREATE TABLE schema_info (
               version INTEGER NOT NULL
             );
             INSERT INTO schema_info(version) VALUES (19);
             CREATE TABLE catalog_state (
               revision INTEGER NOT NULL CHECK(revision >= 0)
             );
             INSERT INTO catalog_state(revision) VALUES (0);
             CREATE TABLE library_roots (
               id TEXT PRIMARY KEY,
               path TEXT NOT NULL UNIQUE,
               active_scan_id TEXT,
               created_unix_ms INTEGER NOT NULL
             );
             CREATE TABLE scan_runs (
               id TEXT PRIMARY KEY,
               root_id TEXT NOT NULL,
               status TEXT NOT NULL,
               scan_owner TEXT NOT NULL DEFAULT 'foreground'
                 CHECK(scan_owner IN ('foreground', 'authoritative_recovery')),
               started_unix_ms INTEGER NOT NULL,
               completed_unix_ms INTEGER,
               asset_count INTEGER NOT NULL DEFAULT 0,
               issue_count INTEGER NOT NULL DEFAULT 0,
               max_items INTEGER,
               max_entries INTEGER,
               preview_edge INTEGER NOT NULL,
               current_directory_relative_path TEXT,
               current_directory_enumerated INTEGER NOT NULL DEFAULT 0
                 CHECK(current_directory_enumerated IN (0, 1)),
               last_visited_relative_path TEXT,
               visited_entries INTEGER NOT NULL DEFAULT 0,
               accepted_items INTEGER NOT NULL DEFAULT 0,
               requires_previous_snapshot INTEGER NOT NULL DEFAULT 0
                 CHECK(requires_previous_snapshot IN (0, 1)),
               root_generation_at_start INTEGER
                 CHECK(root_generation_at_start IS NULL OR root_generation_at_start > 0),
               change_queue_high_watermark INTEGER
                 CHECK(change_queue_high_watermark IS NULL OR change_queue_high_watermark > 0),
               FOREIGN KEY(root_id) REFERENCES library_roots(id)
             );
             CREATE UNIQUE INDEX scan_runs_one_active_root
               ON scan_runs(root_id) WHERE status IN ('running', 'paused');
             CREATE TABLE assets (
               id TEXT PRIMARY KEY,
               created_unix_ms INTEGER NOT NULL
             );
             CREATE TABLE asset_locations (
               scan_id TEXT NOT NULL,
               asset_id TEXT NOT NULL,
               location_id TEXT NOT NULL,
               root_id TEXT NOT NULL,
               absolute_path TEXT NOT NULL,
               relative_path TEXT NOT NULL,
               preview_path TEXT NOT NULL,
               file_size INTEGER NOT NULL,
               created_unix_ms INTEGER,
               modified_unix_ms INTEGER NOT NULL,
               file_local_time TEXT,
               parent_relative_path TEXT NOT NULL DEFAULT '',
               natural_name_key TEXT NOT NULL DEFAULT '',
               width INTEGER NOT NULL,
               height INTEGER NOT NULL,
               preview_status TEXT NOT NULL DEFAULT 'pending'
                 CHECK(preview_status IN ('pending', 'ready', 'failed')),
               preview_issue_code TEXT,
               preview_issue_message TEXT,
               metadata_engine_id TEXT NOT NULL DEFAULT 'unknown',
               metadata_engine_version TEXT NOT NULL DEFAULT '0',
               capture_local_time TEXT,
               capture_offset_minutes INTEGER,
               capture_time_source TEXT
                 CHECK(capture_time_source IS NULL OR capture_time_source IN (
                   'exif_original', 'exif_digitized', 'exif_datetime'
                 )),
               capture_raw_value TEXT,
               file_identity_scheme TEXT,
               file_identity_value TEXT,
               CHECK(
                 (capture_local_time IS NULL AND capture_time_source IS NULL
                   AND capture_raw_value IS NULL)
                 OR
                 (capture_local_time IS NOT NULL AND capture_time_source IS NOT NULL
                   AND capture_raw_value IS NOT NULL)
               ),
               CHECK(
                 (file_identity_scheme IS NULL AND file_identity_value IS NULL)
                 OR
                 (file_identity_scheme IS NOT NULL AND file_identity_value IS NOT NULL)
               ),
               PRIMARY KEY(scan_id, location_id),
               FOREIGN KEY(scan_id) REFERENCES scan_runs(id),
               FOREIGN KEY(asset_id) REFERENCES assets(id),
               FOREIGN KEY(root_id) REFERENCES library_roots(id)
             );
             CREATE INDEX asset_locations_active_root
               ON asset_locations(root_id, scan_id, relative_path, location_id);
             CREATE INDEX asset_locations_file_identity
               ON asset_locations(scan_id, file_identity_scheme, file_identity_value);
             CREATE INDEX asset_locations_active_file_identity
               ON asset_locations(file_identity_scheme, file_identity_value, scan_id, location_id);
             CREATE INDEX asset_locations_location_id
               ON asset_locations(location_id, scan_id);
             CREATE INDEX asset_locations_asset_id
               ON asset_locations(asset_id);
             CREATE INDEX asset_locations_gallery_time
               ON asset_locations(
                 (COALESCE(capture_local_time, file_local_time) IS NULL),
                 IFNULL(COALESCE(capture_local_time, file_local_time), '') DESC,
                 modified_unix_ms DESC, root_id, location_id, scan_id
               );
             CREATE INDEX asset_locations_gallery_created
               ON asset_locations(
                 (file_local_time IS NULL), IFNULL(file_local_time, ''),
                 modified_unix_ms,
                 root_id, location_id, scan_id
               );
             CREATE INDEX asset_locations_gallery_modified
               ON asset_locations(modified_unix_ms, root_id, location_id, scan_id);
             CREATE INDEX asset_locations_gallery_name
               ON asset_locations(natural_name_key, root_id, location_id, scan_id);
             CREATE INDEX asset_locations_parent_folder
               ON asset_locations(root_id, parent_relative_path, scan_id, location_id);
             CREATE TABLE scan_issues (
               id INTEGER PRIMARY KEY AUTOINCREMENT,
               scan_id TEXT NOT NULL,
               path TEXT,
               code TEXT NOT NULL,
               message TEXT NOT NULL,
               FOREIGN KEY(scan_id) REFERENCES scan_runs(id)
             );
             CREATE UNIQUE INDEX scan_issues_identity
               ON scan_issues(scan_id, IFNULL(path, ''), code, message);
             CREATE TABLE scan_directory_frontier (
               id INTEGER PRIMARY KEY AUTOINCREMENT,
               scan_id TEXT NOT NULL,
               relative_path TEXT NOT NULL,
               FOREIGN KEY(scan_id) REFERENCES scan_runs(id),
               UNIQUE(scan_id, relative_path)
             );
             CREATE INDEX scan_directory_frontier_order
               ON scan_directory_frontier(scan_id, id);
             CREATE TABLE scan_directory_entries (
               scan_id TEXT NOT NULL,
               directory_relative_path TEXT NOT NULL,
               relative_path TEXT NOT NULL,
               PRIMARY KEY(scan_id, directory_relative_path, relative_path),
               FOREIGN KEY(scan_id) REFERENCES scan_runs(id)
             );
             CREATE TABLE preview_artifacts (
               artifact_key TEXT PRIMARY KEY,
               source_file_size INTEGER NOT NULL CHECK(source_file_size >= 0),
               source_modified_unix_ms INTEGER NOT NULL,
               source_identity_scheme TEXT,
               source_identity_value TEXT,
               algorithm_id TEXT NOT NULL,
               algorithm_version INTEGER NOT NULL CHECK(algorithm_version >= 0),
               orientation_contract TEXT NOT NULL,
               size_bucket INTEGER NOT NULL CHECK(size_bucket > 0),
               encoded_width INTEGER NOT NULL CHECK(encoded_width > 0),
               encoded_height INTEGER NOT NULL CHECK(encoded_height > 0),
               artifact_path TEXT NOT NULL UNIQUE,
               byte_size INTEGER NOT NULL CHECK(byte_size >= 0),
               lifecycle_state TEXT NOT NULL
                 CHECK(lifecycle_state IN ('ready', 'stale', 'evictable')),
               created_unix_ms INTEGER NOT NULL,
               last_used_unix_ms INTEGER NOT NULL,
               CHECK(
                 (source_identity_scheme IS NULL AND source_identity_value IS NULL)
                 OR
                 (source_identity_scheme IS NOT NULL AND source_identity_value IS NOT NULL)
               )
             );
             CREATE TABLE preview_artifact_locations (
               artifact_key TEXT NOT NULL,
               location_id TEXT NOT NULL,
               PRIMARY KEY(artifact_key, location_id),
               FOREIGN KEY(artifact_key) REFERENCES preview_artifacts(artifact_key)
                 ON DELETE CASCADE
             );
             CREATE INDEX preview_artifact_locations_location
               ON preview_artifact_locations(location_id, artifact_key);
             CREATE INDEX preview_artifacts_reclamation
               ON preview_artifacts(lifecycle_state, last_used_unix_ms, artifact_key);
             CREATE INDEX preview_artifacts_compatibility
               ON preview_artifacts(
                 source_file_size, source_modified_unix_ms,
                 algorithm_id, algorithm_version, orientation_contract, size_bucket
             );",
        )
        .map_err(database_error)?;
    create_library_change_queue_schema(transaction)?;
    transaction
        .execute(
            "ALTER TABLE library_change_queue
             ADD COLUMN authoritative_scan_id TEXT",
            [],
        )
        .map_err(database_error)?;
    transaction
        .execute(
            "ALTER TABLE library_change_root_state
             ADD COLUMN last_consistency_audit_unix_ms INTEGER",
            [],
        )
        .map_err(database_error)?;
    add_authoritative_recovery_contract_marker(transaction)?;
    add_change_catch_up_contract(transaction)?;
    add_preview_expectation_repair_marker(transaction)?;
    Ok(())
}

fn create_library_change_queue_schema(transaction: &Transaction<'_>) -> Result<(), ScanError> {
    transaction
        .execute_batch(
            "CREATE TABLE library_change_queue_contract (
               singleton INTEGER PRIMARY KEY CHECK(singleton = 1),
               root_authority_complete INTEGER NOT NULL
                 CHECK(root_authority_complete = 1)
             );
             INSERT INTO library_change_queue_contract(singleton, root_authority_complete)
             VALUES (1, 1);
             CREATE TABLE library_change_root_state (
               root_id TEXT PRIMARY KEY,
               generation INTEGER NOT NULL CHECK(generation > 0),
               is_active INTEGER NOT NULL CHECK(is_active IN (0, 1)),
               updated_unix_ms INTEGER NOT NULL
             );
             CREATE TABLE library_change_queue (
               id INTEGER PRIMARY KEY AUTOINCREMENT,
               root_id TEXT NOT NULL,
               root_generation INTEGER NOT NULL CHECK(root_generation > 0),
               intent_kind TEXT NOT NULL CHECK(intent_kind IN (
                 'reconcile', 'rename_candidate', 'freshness_unknown'
               )),
               scope TEXT NOT NULL CHECK(scope IN ('path', 'subtree', 'root')),
               relative_path TEXT NOT NULL,
               previous_relative_path TEXT,
               origin TEXT NOT NULL CHECK(origin IN (
                 'live_notification', 'startup_catch_up', 'user_refresh', 'consistency_audit'
               )),
               first_observed_unix_ms INTEGER NOT NULL,
               most_recent_observed_unix_ms INTEGER NOT NULL,
               first_sequence TEXT NOT NULL CHECK(length(first_sequence) > 0),
               most_recent_sequence TEXT NOT NULL CHECK(length(most_recent_sequence) > 0),
               coalesced_observation_count INTEGER NOT NULL
                 CHECK(coalesced_observation_count > 0),
               status TEXT NOT NULL CHECK(status IN (
                 'pending', 'leased', 'retry_wait', 'completed', 'superseded'
               )),
               ready_unix_ms INTEGER NOT NULL,
               attempt_count INTEGER NOT NULL DEFAULT 0 CHECK(attempt_count >= 0),
               next_retry_unix_ms INTEGER,
               lease_generation INTEGER NOT NULL DEFAULT 0 CHECK(lease_generation >= 0),
               lease_expires_unix_ms INTEGER,
               last_failure_code TEXT,
               last_failure_message TEXT,
               catalog_revision_at_enqueue INTEGER NOT NULL
                 CHECK(catalog_revision_at_enqueue >= 0),
               catalog_revision_at_success INTEGER
                 CHECK(catalog_revision_at_success IS NULL OR catalog_revision_at_success >= 0),
               catch_up_source TEXT,
               catch_up_watermark TEXT,
               superseded_by_change_id INTEGER,
               created_unix_ms INTEGER NOT NULL,
               updated_unix_ms INTEGER NOT NULL,
               CHECK(first_observed_unix_ms <= most_recent_observed_unix_ms),
               CHECK(
                 (last_failure_code IS NULL AND last_failure_message IS NULL)
                 OR
                 (last_failure_code IS NOT NULL AND last_failure_message IS NOT NULL)
               ),
               CHECK(
                 (status = 'leased' AND lease_expires_unix_ms IS NOT NULL)
                 OR
                 (status <> 'leased' AND lease_expires_unix_ms IS NULL)
               ),
               CHECK(status = 'retry_wait' OR next_retry_unix_ms IS NULL),
               CHECK(
                 (status = 'completed' AND catalog_revision_at_success IS NOT NULL)
                 OR
                 (status <> 'completed' AND catalog_revision_at_success IS NULL)
               ),
               FOREIGN KEY(superseded_by_change_id) REFERENCES library_change_queue(id)
                 ON DELETE SET NULL
             );
             CREATE INDEX library_change_queue_eligible
               ON library_change_queue(
                 root_id, root_generation, status, ready_unix_ms, next_retry_unix_ms, id
               );
             CREATE INDEX library_change_queue_lease_expiry
               ON library_change_queue(status, lease_expires_unix_ms, id);
             CREATE INDEX library_change_queue_active_path
               ON library_change_queue(
                 root_id, root_generation, status, relative_path, scope, id
               );
             CREATE INDEX library_change_queue_cleanup
               ON library_change_queue(status, updated_unix_ms, id);",
        )
        .map_err(database_error)?;
    let has_library_roots = transaction
        .query_row(
            "SELECT EXISTS(
               SELECT 1 FROM sqlite_master
               WHERE type = 'table' AND name = 'library_roots'
             )",
            [],
            |row| row.get::<_, bool>(0),
        )
        .map_err(database_error)?;
    if has_library_roots {
        transaction
            .execute(
                "INSERT INTO library_change_root_state(
                   root_id, generation, is_active, updated_unix_ms
                 )
                 SELECT id, 1, 1, 0 FROM library_roots",
                [],
            )
            .map_err(database_error)?;
    }
    Ok(())
}

fn validate_v19_schema_contract(connection: &Connection) -> Result<(), ScanError> {
    validate_v19_schema_contract_with_depth(connection, ContractValidationDepth::Full)
}

fn validate_v19_schema_contract_with_depth(
    connection: &Connection,
    depth: ContractValidationDepth,
) -> Result<(), ScanError> {
    validate_change_queue_authority(connection)?;
    validate_authoritative_recovery_marker(connection)?;
    validate_change_catch_up_contract_with_depth(connection, depth)?;
    validate_scan_handoff_batch_contract_with_depth(connection, depth)?;
    let (has_scan_runs, has_single_scan_owner_index, has_scan_owner) = connection
        .query_row(
            "SELECT
               EXISTS(SELECT 1 FROM sqlite_master
                 WHERE type = 'table' AND name = 'scan_runs'),
               EXISTS(SELECT 1 FROM pragma_index_list('scan_runs')
                 WHERE name = 'scan_runs_one_active_root'
                   AND \"unique\" = 1 AND partial = 1),
               EXISTS(SELECT 1 FROM pragma_table_info('scan_runs')
                 WHERE name = 'scan_owner')",
            [],
            |row| {
                Ok((
                    row.get::<_, bool>(0)?,
                    row.get::<_, bool>(1)?,
                    row.get::<_, bool>(2)?,
                ))
            },
        )
        .map_err(database_error)?;
    if has_scan_runs && (!has_single_scan_owner_index || !has_scan_owner) {
        return Err(unverifiable_authoritative_recovery_contract());
    }
    if !preview_repair_marker_is_complete(connection)? {
        return Err(ScanError::new(
            "catalog_change_catch_up_contract_unverifiable",
            "The catalog preview repair authority is missing",
        ));
    }
    Ok(())
}

fn validate_prerelease_v19_catch_up_authority(connection: &Connection) -> Result<(), ScanError> {
    let has_marker = connection
        .query_row(
            "SELECT EXISTS(
               SELECT 1 FROM pragma_table_info('library_change_queue_contract')
               WHERE name = 'change_catch_up_complete'
             )",
            [],
            |row| row.get::<_, bool>(0),
        )
        .map_err(database_error)?;
    if !has_marker {
        return Err(ScanError::new(
            "catalog_change_catch_up_contract_unverifiable",
            "The catalog cannot prove its downtime catch-up checkpoint authority",
        ));
    }
    let marker_complete = connection
        .query_row(
            "SELECT change_catch_up_complete = 1
             FROM library_change_queue_contract WHERE singleton = 1",
            [],
            |row| row.get::<_, bool>(0),
        )
        .optional()
        .map_err(database_error)?
        .unwrap_or(false);
    if !marker_complete {
        return Err(ScanError::new(
            "catalog_change_catch_up_contract_unverifiable",
            "The catalog cannot prove its downtime catch-up checkpoint authority",
        ));
    }
    Ok(())
}

#[cfg(test)]
std::thread_local! {
    static CURRENT_SCHEMA_VALIDATION_COUNT: std::cell::Cell<usize> =
        const { std::cell::Cell::new(0) };
    static SOURCE_REVISION_ROW_AUDIT_COUNT: std::cell::Cell<usize> =
        const { std::cell::Cell::new(0) };
    static CURRENT_SCHEMA_ROW_AUDIT_COUNT: std::cell::Cell<usize> =
        const { std::cell::Cell::new(0) };
}

#[cfg(test)]
fn reset_current_schema_validation_count() {
    CURRENT_SCHEMA_VALIDATION_COUNT.with(|count| count.set(0));
}

#[cfg(test)]
fn current_schema_validation_count() -> usize {
    CURRENT_SCHEMA_VALIDATION_COUNT.with(std::cell::Cell::get)
}

#[cfg(test)]
fn reset_source_revision_row_audit_count() {
    SOURCE_REVISION_ROW_AUDIT_COUNT.with(|count| count.set(0));
}

#[cfg(test)]
fn source_revision_row_audit_count() -> usize {
    SOURCE_REVISION_ROW_AUDIT_COUNT.with(std::cell::Cell::get)
}

#[cfg(test)]
fn reset_current_schema_row_audit_count() {
    CURRENT_SCHEMA_ROW_AUDIT_COUNT.with(|count| count.set(0));
}

#[cfg(test)]
fn current_schema_row_audit_count() -> usize {
    CURRENT_SCHEMA_ROW_AUDIT_COUNT.with(std::cell::Cell::get)
}

fn record_current_schema_row_audit() {
    #[cfg(test)]
    CURRENT_SCHEMA_ROW_AUDIT_COUNT.with(|count| count.set(count.get() + 1));
}

fn source_revision_metadata_contract_is_complete(
    connection: &Connection,
) -> Result<bool, ScanError> {
    if !schema_object_sql_matches(
        connection,
        "table",
        "library_source_revision_metadata_contract",
        SOURCE_REVISION_METADATA_CONTRACT_TABLE_DDL,
    )? {
        return Ok(false);
    }
    connection
        .query_row(
            "SELECT contract_version = 1 AND complete = 1
             FROM library_source_revision_metadata_contract
             WHERE singleton = 1",
            [],
            |row| row.get::<_, bool>(0),
        )
        .optional()
        .map(|value| value.unwrap_or(false))
        .map_err(database_error)
}

fn invalidate_precontract_source_revision_metadata(
    transaction: &Transaction<'_>,
) -> Result<(), ScanError> {
    transaction
        .execute_batch(
            "DELETE FROM preview_artifact_locations;
             UPDATE preview_artifacts SET lifecycle_state = 'evictable';
             UPDATE asset_locations
             SET preview_path = '', preview_status = 'pending',
                 preview_issue_code = NULL, preview_issue_message = NULL,
                 metadata_engine_id = 'ame-invalidated-media-metadata',
                 metadata_engine_version = '0',
                 capture_local_time = NULL, capture_offset_minutes = NULL,
                 capture_time_source = NULL, capture_raw_value = NULL;",
        )
        .map_err(database_error)
}

fn create_source_revision_metadata_contract(
    transaction: &Transaction<'_>,
) -> Result<(), ScanError> {
    transaction
        .execute_batch(SOURCE_REVISION_METADATA_CONTRACT_TABLE_DDL)
        .map_err(database_error)?;
    transaction
        .execute(
            "INSERT INTO library_source_revision_metadata_contract(
               singleton, contract_version, complete
             ) VALUES (1, 1, 1)",
            [],
        )
        .map_err(database_error)?;
    Ok(())
}

fn repair_prerelease_v31_source_revision_metadata(
    connection: &mut Connection,
) -> Result<(), ScanError> {
    let marker_sql = connection
        .query_row(
            "SELECT sql FROM sqlite_master
             WHERE type = 'table'
               AND name = 'library_source_revision_metadata_contract'",
            [],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(database_error)?;
    if marker_sql.is_some() {
        if source_revision_metadata_contract_is_complete(connection)? {
            return Ok(());
        }
        return Err(ScanError::new(
            "catalog_source_revision_contract_unverifiable",
            "The catalog cannot prove that legacy source metadata was invalidated",
        ));
    }

    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(database_error)?;
    validate_current_schema_contract_with_source_revision_rows(&transaction)?;
    invalidate_precontract_source_revision_metadata(&transaction)?;
    create_source_revision_metadata_contract(&transaction)?;
    if !source_revision_metadata_contract_is_complete(&transaction)? {
        return Err(ScanError::new(
            "catalog_source_revision_contract_unverifiable",
            "The catalog could not record its source metadata invalidation",
        ));
    }
    validate_current_schema_contract_with_source_revision_rows(&transaction)?;
    transaction.commit().map_err(database_error)
}

fn legacy_terminal_metadata_inventory_repair_shape_matches(
    connection: &Connection,
    schema_version: i64,
) -> Result<bool, ScanError> {
    let spool_contract_version = match schema_version {
        25 | 26 => None,
        27 => Some(1),
        28..=30 => Some(2),
        31 => Some(3),
        _ => return Ok(false),
    };
    let core_matches = schema_object_sql_matches(
        connection,
        "table",
        "library_metadata_inventory_runs",
        METADATA_INVENTORY_RUN_TABLE_DDL,
    )? && schema_object_sql_matches(
        connection,
        "table",
        "library_recovery_authorities",
        RECOVERY_AUTHORITY_TABLE_DDL,
    )?;
    if !core_matches {
        return Ok(false);
    }
    if let Some(version) = spool_contract_version {
        metadata_inventory_spool_schema_matches(connection, version)
    } else {
        Ok(true)
    }
}

fn legacy_terminal_metadata_inventory_repair_needed(
    connection: &Connection,
    schema_version: i64,
) -> Result<bool, ScanError> {
    if !legacy_terminal_metadata_inventory_repair_shape_matches(connection, schema_version)? {
        return Ok(false);
    }
    let terminal_authority = connection
        .query_row(
            "SELECT EXISTS(
               SELECT 1 FROM library_metadata_inventory_runs AS run
               WHERE run.status IN ('failed', 'cancelled', 'superseded')
                 AND run.absence_authority <> 0
                 AND NOT EXISTS(
                   SELECT 1 FROM library_recovery_authorities AS authority
                   WHERE authority.run_id = run.id
                     AND authority.root_id = run.root_id
                     AND authority.root_generation = run.root_generation
                     AND authority.retired_unix_ms IS NULL
                 )
             )",
            [],
            |row| row.get::<_, bool>(0),
        )
        .map_err(database_error)?;
    let terminal_spool = if schema_version >= 27 {
        connection
            .query_row(
                "SELECT EXISTS(
                   SELECT 1 FROM library_metadata_inventory_spools AS spool
                   JOIN library_metadata_inventory_runs AS run ON run.id = spool.run_id
                   WHERE run.status IN ('failed', 'cancelled', 'superseded')
                 )",
                [],
                |row| row.get::<_, bool>(0),
            )
            .map_err(database_error)?
    } else {
        false
    };
    Ok(terminal_authority || terminal_spool)
}

fn repair_legacy_terminal_metadata_inventory_state_transaction(
    transaction: &Transaction<'_>,
    schema_version: i64,
) -> Result<(), ScanError> {
    if !legacy_terminal_metadata_inventory_repair_shape_matches(transaction, schema_version)? {
        return Ok(());
    }
    if schema_version >= 27 {
        super::spool_retirement::delete_owned_spools(
            transaction,
            super::spool_retirement::SpoolOwner::LegacyTerminalRuns,
        )?;
    }
    transaction
        .execute(
            "UPDATE library_metadata_inventory_runs AS run
             SET absence_authority = 0
             WHERE run.status IN ('failed', 'cancelled', 'superseded')
               AND run.absence_authority <> 0
               AND NOT EXISTS(
                 SELECT 1 FROM library_recovery_authorities AS authority
                 WHERE authority.run_id = run.id
                   AND authority.root_id = run.root_id
                   AND authority.root_generation = run.root_generation
                   AND authority.retired_unix_ms IS NULL
               )",
            [],
        )
        .map_err(database_error)?;
    Ok(())
}

fn retired_live_gap_claim_repair_shape_matches(
    connection: &Connection,
    schema_version: i64,
) -> Result<bool, ScanError> {
    if !matches!(schema_version, 30 | 31) {
        return Ok(false);
    }
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
    if !schema_matches {
        return Ok(false);
    }
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
    Ok(marker_complete)
}

fn repairable_retired_live_gap_claim_ids(
    connection: &Connection,
    schema_version: i64,
) -> Result<Vec<i64>, ScanError> {
    if !retired_live_gap_claim_repair_shape_matches(connection, schema_version)? {
        return Ok(Vec::new());
    }
    let mut statement = connection
        .prepare(
            "SELECT claim.gap_change_id
             FROM library_live_gap_recovery_claims AS claim
             JOIN library_change_queue AS gap ON gap.id = claim.gap_change_id
             JOIN library_change_queue_lanes AS lane ON lane.change_id = gap.id
             JOIN library_change_root_state AS root_state
               ON root_state.root_id = claim.root_id
             WHERE claim.consumed_unix_ms IS NULL
               AND claim.source_range_id IS NULL
               AND claim.recovery_change_id IS NULL
               AND claim.foreground_scan_id IS NULL
               AND gap.root_id = claim.root_id
               AND gap.root_generation = claim.root_generation
               AND gap.intent_kind = 'freshness_unknown'
               AND gap.scope = 'root' AND gap.relative_path = ''
               AND gap.previous_relative_path IS NULL
               AND gap.status = 'superseded'
               AND gap.next_retry_unix_ms IS NULL
               AND gap.lease_expires_unix_ms IS NULL
               AND gap.authoritative_scan_id IS NULL
               AND gap.catalog_revision_at_success IS NULL
               AND gap.catch_up_source IS NULL
               AND gap.catch_up_watermark IS NULL
               AND gap.superseded_by_change_id IS NULL
               AND gap.last_failure_message IS NOT NULL
               AND (
                 (claim.consumer_kind = 'explicit_recovery_required'
                   AND claim.opening_volume_guid IS NULL
                   AND claim.opening_volume_serial IS NULL
                   AND claim.opening_root_reference_version IS NULL
                   AND claim.opening_root_file_reference IS NULL
                   AND claim.opening_journal_id IS NULL
                   AND claim.opening_next_usn IS NULL
                   AND claim.protocol_version IS NULL
                   AND claim.contract_version IS NULL
                   AND gap.origin = 'startup_catch_up'
                   AND lane.lane = 'p1_journal'
                   AND gap.last_failure_code =
                         'live_gap_v30_explicit_recovery_required')
                 OR
                 (claim.consumer_kind = 'pending_journal'
                   AND claim.opening_volume_guid IS NOT NULL
                   AND length(claim.opening_volume_guid) BETWEEN 1 AND 512
                   AND claim.opening_volume_serial IS NOT NULL
                   AND length(claim.opening_volume_serial) BETWEEN 1 AND 20
                   AND claim.opening_volume_serial NOT GLOB '*[^0-9]*'
                   AND claim.opening_root_reference_version IN (2, 3)
                   AND claim.opening_root_file_reference IS NOT NULL
                   AND length(claim.opening_root_file_reference) = CASE
                     claim.opening_root_reference_version WHEN 2 THEN 8 ELSE 16 END
                   AND claim.opening_journal_id IS NOT NULL
                   AND length(claim.opening_journal_id) BETWEEN 1 AND 20
                   AND claim.opening_journal_id <> '0'
                   AND claim.opening_journal_id NOT GLOB '*[^0-9]*'
                   AND claim.opening_next_usn IS NOT NULL
                   AND length(claim.opening_next_usn) BETWEEN 1 AND 19
                   AND claim.opening_next_usn NOT GLOB '*[^0-9]*'
                   AND claim.protocol_version BETWEEN 1 AND 65535
                   AND claim.contract_version = 1
                   AND gap.origin = 'live_notification'
                   AND lane.lane = 'p0_live'
                   AND gap.last_failure_code = 'live_gap_waiting_for_journal_range')
               )
               AND (
                 (root_state.is_active = 0
                   AND root_state.generation >= claim.root_generation
                   AND NOT EXISTS(
                     SELECT 1 FROM library_roots AS root
                     WHERE root.id = claim.root_id
                   ))
                 OR
                 (root_state.is_active = 1
                   AND root_state.generation > claim.root_generation
                   AND EXISTS(
                     SELECT 1 FROM library_roots AS root
                     WHERE root.id = claim.root_id
                   ))
               )
             ORDER BY claim.gap_change_id",
        )
        .map_err(database_error)?;
    statement
        .query_map([], |row| row.get::<_, i64>(0))
        .map_err(database_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(database_error)
}

fn retired_live_gap_claim_repair_needed(
    connection: &Connection,
    schema_version: i64,
) -> Result<bool, ScanError> {
    Ok(!repairable_retired_live_gap_claim_ids(connection, schema_version)?.is_empty())
}

fn repair_retired_live_gap_claims_transaction(
    transaction: &Transaction<'_>,
    schema_version: i64,
) -> Result<(), ScanError> {
    for gap_change_id in repairable_retired_live_gap_claim_ids(transaction, schema_version)? {
        let deleted = transaction
            .execute(
                "DELETE FROM library_live_gap_recovery_claims
                 WHERE gap_change_id = ?1
                   AND consumer_kind IN (
                     'explicit_recovery_required', 'pending_journal'
                   )
                   AND consumed_unix_ms IS NULL",
                [gap_change_id],
            )
            .map_err(database_error)?;
        if deleted != 1 {
            return Err(ScanError::new(
                "catalog_live_gap_recovery_repair_conflict",
                "The retired live-gap claim changed during catalog repair",
            ));
        }
    }
    Ok(())
}

fn repair_and_validate_current_terminal_recovery_state(
    connection: &mut Connection,
) -> Result<(), ScanError> {
    let repair_terminal_inventory =
        legacy_terminal_metadata_inventory_repair_needed(connection, SCHEMA_VERSION)?;
    let repair_retired_live_gap = retired_live_gap_claim_repair_needed(connection, SCHEMA_VERSION)?;
    let repair_removed_root_authority = retired_root_authority::repair_needed(connection)?;
    if !repair_terminal_inventory && !repair_retired_live_gap && !repair_removed_root_authority {
        return validate_current_schema_contract(connection);
    }
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(database_error)?;
    repair_legacy_terminal_metadata_inventory_state_transaction(&transaction, SCHEMA_VERSION)?;
    repair_retired_live_gap_claims_transaction(&transaction, SCHEMA_VERSION)?;
    retired_root_authority::repair_in_transaction(&transaction)?;
    validate_current_schema_contract_with_source_revision_rows(&transaction)?;
    transaction.commit().map_err(database_error)
}

fn recover_interrupted_explicit_foreground_claims(
    connection: &mut Connection,
) -> Result<(), ScanError> {
    let has_interrupted_claim = connection
        .query_row(
            "SELECT EXISTS(
               SELECT 1
               FROM library_live_gap_recovery_claims AS claim
               JOIN scan_runs AS scans ON scans.id = claim.foreground_scan_id
               WHERE claim.consumer_kind = 'foreground_scan'
                 AND claim.consumed_unix_ms IS NULL
                 AND scans.scan_owner = 'foreground'
                 AND scans.status IN ('running', 'paused')
             )",
            [],
            |row| row.get::<_, bool>(0),
        )
        .map_err(database_error)?;
    if !has_interrupted_claim {
        return Ok(());
    }

    let recovered_unix_ms = unix_time_ms();
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(database_error)?;
    let scan_ids = {
        let mut statement = transaction
            .prepare(
                "SELECT DISTINCT claim.foreground_scan_id
                 FROM library_live_gap_recovery_claims AS claim
                 JOIN scan_runs AS scans ON scans.id = claim.foreground_scan_id
                 WHERE claim.consumer_kind = 'foreground_scan'
                   AND claim.consumed_unix_ms IS NULL
                   AND scans.scan_owner = 'foreground'
                   AND scans.status IN ('running', 'paused')
                 ORDER BY claim.foreground_scan_id",
            )
            .map_err(database_error)?;
        statement
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(database_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(database_error)?
    };
    for scan_id in scan_ids {
        let claim_count = transaction
            .query_row(
                "SELECT COUNT(*) FROM library_live_gap_recovery_claims
                 WHERE consumer_kind = 'foreground_scan'
                   AND consumed_unix_ms IS NULL AND foreground_scan_id = ?1",
                [&scan_id],
                |row| row.get::<_, i64>(0),
            )
            .map_err(database_error)?;
        let restored_gaps = transaction
            .execute(
                "UPDATE library_change_queue
                 SET status = 'retry_wait', next_retry_unix_ms = NULL,
                     lease_expires_unix_ms = NULL, authoritative_scan_id = NULL,
                     last_failure_code = 'live_gap_v30_explicit_recovery_required',
                     last_failure_message =
                       'The explicit library update was interrupted and must be started again',
                     updated_unix_ms = ?2
                 WHERE id IN (
                   SELECT gap_change_id FROM library_live_gap_recovery_claims
                   WHERE consumer_kind = 'foreground_scan'
                     AND consumed_unix_ms IS NULL AND foreground_scan_id = ?1
                 )
                   AND status = 'leased' AND authoritative_scan_id = ?1",
                params![scan_id, recovered_unix_ms],
            )
            .map_err(database_error)?;
        if i64::try_from(restored_gaps).ok() != Some(claim_count) {
            return Err(ScanError::new(
                "catalog_live_gap_foreground_recovery_conflict",
                "The interrupted foreground recovery no longer owns every explicit gap",
            ));
        }
        let restored_claims = transaction
            .execute(
                "UPDATE library_live_gap_recovery_claims
                 SET consumer_kind = 'explicit_recovery_required',
                     foreground_scan_id = NULL, consumed_unix_ms = NULL
                 WHERE consumer_kind = 'foreground_scan'
                   AND consumed_unix_ms IS NULL AND foreground_scan_id = ?1",
                [&scan_id],
            )
            .map_err(database_error)?;
        if i64::try_from(restored_claims).ok() != Some(claim_count) {
            return Err(ScanError::new(
                "catalog_live_gap_foreground_recovery_conflict",
                "The interrupted foreground consumer changed before recovery",
            ));
        }
        transaction
            .execute(
                "UPDATE library_change_queue
                 SET status = 'pending', ready_unix_ms = ?2,
                     next_retry_unix_ms = NULL, lease_expires_unix_ms = NULL,
                     authoritative_scan_id = NULL, updated_unix_ms = ?2
                 WHERE authoritative_scan_id = ?1 AND status = 'leased'
                   AND NOT EXISTS(
                     SELECT 1 FROM library_live_gap_recovery_claims AS claim
                     WHERE claim.gap_change_id = library_change_queue.id
                   )",
                params![scan_id, recovered_unix_ms],
            )
            .map_err(database_error)?;
        transaction
            .execute(
                "UPDATE library_change_queue SET authoritative_scan_id = NULL
                 WHERE authoritative_scan_id = ?1",
                [&scan_id],
            )
            .map_err(database_error)?;
        let terminalized = transaction
            .execute(
                "UPDATE scan_runs
                 SET status = 'failed', completed_unix_ms = ?2,
                     current_directory_relative_path = NULL,
                     current_directory_enumerated = 0,
                     last_visited_relative_path = NULL
                 WHERE id = ?1 AND scan_owner = 'foreground'
                   AND status IN ('running', 'paused')",
                params![scan_id, recovered_unix_ms],
            )
            .map_err(database_error)?;
        if terminalized != 1 {
            return Err(ScanError::new(
                "catalog_live_gap_foreground_recovery_conflict",
                "The interrupted foreground scan changed before recovery",
            ));
        }
        for sql in [
            "DELETE FROM scan_run_catch_up_lineage WHERE scan_id = ?1",
            "DELETE FROM scan_directory_frontier WHERE scan_id = ?1",
            "DELETE FROM scan_directory_entries WHERE scan_id = ?1",
            "DELETE FROM library_scan_publication_namespace_bindings WHERE scan_id = ?1",
            "DELETE FROM asset_locations WHERE scan_id = ?1",
        ] {
            transaction
                .execute(sql, [&scan_id])
                .map_err(database_error)?;
        }
    }
    super::delete_orphan_assets(&transaction)?;
    validate_current_schema_contract_with_source_revision_rows(&transaction)?;
    transaction.commit().map_err(database_error)
}

fn validate_pre_live_gap_schema_contract(
    connection: &Connection,
    schema_version: i64,
) -> Result<(), ScanError> {
    validate_pre_live_gap_schema_contract_with_depth(
        connection,
        schema_version,
        ContractValidationDepth::Full,
    )
}

fn validate_pre_live_gap_schema_contract_with_depth(
    connection: &Connection,
    schema_version: i64,
    depth: ContractValidationDepth,
) -> Result<(), ScanError> {
    validate_v19_schema_contract_with_depth(connection, depth)?;
    validate_metadata_inventory_contract_with_depth(connection, depth)?;
    validate_terminal_media_evidence_contract_with_depth(connection, depth)?;
    validate_persistent_journal_contract_with_depth(connection, depth)?;
    validate_change_lane_contract_with_depth(connection, depth)?;
    validate_recovery_authority_contract_with_depth(connection, depth)?;
    validate_persistent_journal_baseline_contract_with_depth(connection, depth)?;
    validate_recovery_execution_contract_with_depth(connection, depth)?;
    validate_metadata_inventory_spool_contract_version_with_depth(
        connection,
        schema_version,
        if schema_version >= 31 { 3 } else { 2 },
        depth,
    )?;
    validate_root_publication_namespace_contract_with_depth(connection, depth)
}

const CHANGE_LANE_INDEX_DDL: &str = "CREATE INDEX library_change_queue_lanes_eligible
       ON library_change_queue_lanes(lane, change_id)";
const CHANGE_LANE_CONTRACT_TABLE_DDL: &str = "CREATE TABLE library_change_lane_contract (
       singleton INTEGER PRIMARY KEY CHECK(singleton = 1),
       complete INTEGER NOT NULL CHECK(complete = 1)
     )";
const CHANGE_LANE_QUEUE_TABLE_DDL: &str = "CREATE TABLE library_change_queue_lanes (
       change_id INTEGER PRIMARY KEY,
       lane TEXT NOT NULL CHECK(lane IN ('p0_live', 'p1_journal', 'p2_recovery')),
       FOREIGN KEY(change_id) REFERENCES library_change_queue(id) ON DELETE CASCADE
     )";
const CHANGE_LANE_INSERT_TRIGGER_DDL: &str = "CREATE TRIGGER library_change_queue_lane_insert
       AFTER INSERT ON library_change_queue
       BEGIN
         INSERT INTO library_change_queue_lanes(change_id, lane)
         VALUES (
           NEW.id,
           CASE NEW.origin
             WHEN 'live_notification' THEN 'p0_live'
             WHEN 'startup_catch_up' THEN 'p1_journal'
             ELSE 'p2_recovery'
           END
         );
       END";
const CHANGE_LANE_ORIGIN_TRIGGER_DDL: &str =
    "CREATE TRIGGER library_change_queue_lane_origin_update
       AFTER UPDATE OF origin ON library_change_queue
       BEGIN
         UPDATE library_change_queue_lanes
         SET lane = CASE NEW.origin
           WHEN 'live_notification' THEN 'p0_live'
           WHEN 'startup_catch_up' THEN 'p1_journal'
           ELSE 'p2_recovery'
         END
         WHERE change_id = NEW.id;
       END";
const CHANGE_LANE_INSERT_GUARD_DDL: &str = "CREATE TRIGGER library_change_queue_lane_insert_guard
       BEFORE INSERT ON library_change_queue_lanes
       WHEN NEW.lane <> (
         SELECT CASE origin
           WHEN 'live_notification' THEN 'p0_live'
           WHEN 'startup_catch_up' THEN 'p1_journal'
           ELSE 'p2_recovery'
         END
         FROM library_change_queue WHERE id = NEW.change_id
       )
       BEGIN
         SELECT RAISE(ABORT, 'change queue lane does not match origin');
       END";
const CHANGE_LANE_UPDATE_GUARD_DDL: &str = "CREATE TRIGGER library_change_queue_lane_update_guard
       BEFORE UPDATE OF lane, change_id ON library_change_queue_lanes
       WHEN NEW.lane <> (
         SELECT CASE origin
           WHEN 'live_notification' THEN 'p0_live'
           WHEN 'startup_catch_up' THEN 'p1_journal'
           ELSE 'p2_recovery'
         END
         FROM library_change_queue WHERE id = NEW.change_id
       )
       BEGIN
         SELECT RAISE(ABORT, 'change queue lane does not match origin');
       END";
const RECOVERY_AUTHORITY_CONTRACT_TABLE_DDL: &str =
    "CREATE TABLE library_recovery_authority_contract (
       singleton INTEGER PRIMARY KEY CHECK(singleton = 1),
       complete INTEGER NOT NULL CHECK(complete = 1)
     )";
const RECOVERY_AUTHORITY_TABLE_DDL: &str = "CREATE TABLE library_recovery_authorities (
       change_id INTEGER PRIMARY KEY,
       run_id TEXT NOT NULL UNIQUE CHECK(length(run_id) BETWEEN 1 AND 512),
       root_id TEXT NOT NULL CHECK(length(root_id) BETWEEN 1 AND 512),
       root_generation INTEGER NOT NULL CHECK(root_generation > 0),
       reason TEXT NOT NULL CHECK(reason IN (
         'existing_root_baseline', 'first_import_boundary', 'journal_gap',
         'journal_reset', 'journal_trim', 'journal_reconstruction_failure',
         'containment_failure', 'broker_after_current_failure', 'watcher_uncovered_gap'
       )),
       opening_journal_id TEXT,
       opening_next_usn TEXT,
       authorized_unix_ms INTEGER NOT NULL,
       retired_unix_ms INTEGER,
       CHECK(
         (reason IN ('existing_root_baseline', 'first_import_boundary')
           AND opening_journal_id IS NOT NULL AND opening_next_usn IS NOT NULL)
         OR
         (reason NOT IN ('existing_root_baseline', 'first_import_boundary')
           AND opening_journal_id IS NULL AND opening_next_usn IS NULL)
       ),
       CHECK(retired_unix_ms IS NULL OR retired_unix_ms >= authorized_unix_ms),
       FOREIGN KEY(change_id) REFERENCES library_change_queue(id) ON DELETE CASCADE
     )";
const RECOVERY_AUTHORITY_ROOT_INDEX_DDL: &str = "CREATE INDEX library_recovery_authorities_root
       ON library_recovery_authorities(root_id, root_generation, retired_unix_ms, change_id)";
const RECOVERY_AUTHORITY_INSERT_GUARD_DDL: &str =
    "CREATE TRIGGER library_recovery_authority_insert_guard
       BEFORE INSERT ON library_recovery_authorities
       WHEN NOT EXISTS (
         SELECT 1
         FROM library_change_queue AS queue
         JOIN library_change_queue_lanes AS lanes ON lanes.change_id = queue.id
         WHERE queue.id = NEW.change_id
           AND queue.root_id = NEW.root_id
           AND queue.root_generation = NEW.root_generation
           AND lanes.lane = 'p2_recovery'
       )
       BEGIN
         SELECT RAISE(ABORT, 'recovery authority does not match P2 queue work');
       END";
const RECOVERY_AUTHORITY_UPDATE_GUARD_DDL: &str =
    "CREATE TRIGGER library_recovery_authority_update_guard
       BEFORE UPDATE OF change_id, root_id, root_generation, reason,
                        opening_journal_id, opening_next_usn, authorized_unix_ms
       ON library_recovery_authorities
       BEGIN
         SELECT RAISE(ABORT, 'recovery authority identity is immutable');
       END";
const PERSISTENT_JOURNAL_BASELINE_TABLE_DDL: &str =
    "CREATE TABLE library_persistent_journal_baselines (
       change_id INTEGER PRIMARY KEY,
       root_id TEXT NOT NULL CHECK(length(root_id) BETWEEN 1 AND 512),
       root_generation INTEGER NOT NULL CHECK(root_generation > 0),
       volume_guid TEXT NOT NULL CHECK(length(volume_guid) BETWEEN 1 AND 512),
       volume_serial TEXT NOT NULL CHECK(length(volume_serial) BETWEEN 1 AND 20),
       root_reference_version INTEGER NOT NULL CHECK(root_reference_version IN (2, 3)),
       root_file_reference BLOB NOT NULL,
       journal_id TEXT NOT NULL CHECK(length(journal_id) BETWEEN 1 AND 20),
       opening_next_usn TEXT NOT NULL CHECK(length(opening_next_usn) BETWEEN 1 AND 19),
       closing_next_usn TEXT CHECK(length(closing_next_usn) BETWEEN 1 AND 19),
       protocol_version INTEGER NOT NULL CHECK(protocol_version BETWEEN 1 AND 65535),
       contract_version INTEGER NOT NULL CHECK(contract_version = 1),
       phase TEXT NOT NULL CHECK(phase IN ('inventory', 'replay', 'absence', 'completed')),
       authorized_unix_ms INTEGER NOT NULL CHECK(authorized_unix_ms >= 0),
       updated_unix_ms INTEGER NOT NULL CHECK(updated_unix_ms >= authorized_unix_ms),
       completed_unix_ms INTEGER CHECK(completed_unix_ms >= updated_unix_ms),
       CHECK(
         (root_reference_version = 2 AND length(root_file_reference) = 8)
         OR
         (root_reference_version = 3 AND length(root_file_reference) = 16)
       ),
       CHECK(journal_id <> '0' AND journal_id NOT GLOB '*[^0-9]*'),
       CHECK(opening_next_usn NOT GLOB '*[^0-9]*'),
       CHECK(closing_next_usn IS NULL OR closing_next_usn NOT GLOB '*[^0-9]*'),
       CHECK(
         (phase = 'inventory' AND closing_next_usn IS NULL)
         OR
         (phase <> 'inventory' AND closing_next_usn IS NOT NULL)
       ),
       CHECK(
         closing_next_usn IS NULL
         OR CAST(closing_next_usn AS INTEGER) >= CAST(opening_next_usn AS INTEGER)
       ),
       CHECK((phase = 'completed') = (completed_unix_ms IS NOT NULL)),
       FOREIGN KEY(change_id)
         REFERENCES library_recovery_authorities(change_id) ON DELETE CASCADE,
       FOREIGN KEY(root_id, root_generation)
         REFERENCES library_persistent_journal_root_state(root_id, root_generation)
         ON DELETE CASCADE
     )";
const PERSISTENT_JOURNAL_BASELINE_ROOT_INDEX_DDL: &str =
    "CREATE UNIQUE INDEX library_persistent_journal_baselines_root
       ON library_persistent_journal_baselines(root_id, root_generation)
       WHERE phase <> 'completed'";
const LEGACY_V26_PERSISTENT_JOURNAL_BASELINE_ROOT_INDEX_DDL: &str =
    "CREATE UNIQUE INDEX library_persistent_journal_baselines_root
       ON library_persistent_journal_baselines(root_id, root_generation)";
const PERSISTENT_JOURNAL_BASELINE_INSERT_GUARD_DDL: &str =
    "CREATE TRIGGER library_persistent_journal_baseline_insert_guard
       BEFORE INSERT ON library_persistent_journal_baselines
       WHEN NOT EXISTS (
         SELECT 1 FROM library_recovery_authorities AS authority
         WHERE authority.change_id = NEW.change_id
           AND authority.root_id = NEW.root_id
           AND authority.root_generation = NEW.root_generation
           AND (
             (authority.reason IN ('existing_root_baseline', 'first_import_boundary')
               AND authority.opening_journal_id = NEW.journal_id
               AND authority.opening_next_usn = NEW.opening_next_usn)
             OR
             (authority.reason IN (
                'watcher_uncovered_gap', 'journal_gap', 'journal_reset', 'journal_trim',
                'journal_reconstruction_failure', 'containment_failure',
                'broker_after_current_failure'
              )
               AND authority.opening_journal_id IS NULL
               AND authority.opening_next_usn IS NULL)
           )
           AND authority.retired_unix_ms IS NULL
       )
       BEGIN
         SELECT RAISE(ABORT, 'journal baseline does not match recovery authority');
       END";
const LEGACY_V26_PERSISTENT_JOURNAL_BASELINE_INSERT_GUARD_DDL: &str =
    "CREATE TRIGGER library_persistent_journal_baseline_insert_guard
       BEFORE INSERT ON library_persistent_journal_baselines
       WHEN NOT EXISTS (
         SELECT 1 FROM library_recovery_authorities AS authority
         WHERE authority.change_id = NEW.change_id
           AND authority.root_id = NEW.root_id
           AND authority.root_generation = NEW.root_generation
           AND authority.reason IN ('existing_root_baseline', 'first_import_boundary')
           AND authority.opening_journal_id = NEW.journal_id
           AND authority.opening_next_usn = NEW.opening_next_usn
           AND authority.retired_unix_ms IS NULL
       )
       BEGIN
         SELECT RAISE(ABORT, 'journal baseline does not match recovery authority');
       END";
const PERSISTENT_JOURNAL_BASELINE_UPDATE_GUARD_DDL: &str =
    "CREATE TRIGGER library_persistent_journal_baseline_update_guard
       BEFORE UPDATE OF change_id, root_id, root_generation, volume_guid, volume_serial,
                        root_reference_version, root_file_reference, journal_id,
                        opening_next_usn, protocol_version, contract_version,
                        authorized_unix_ms
       ON library_persistent_journal_baselines
       BEGIN
         SELECT RAISE(ABORT, 'journal baseline identity is immutable');
       END";
const METADATA_INVENTORY_RUN_TABLE_DDL: &str = "CREATE TABLE library_metadata_inventory_runs (
       id TEXT NOT NULL PRIMARY KEY CHECK(length(id) BETWEEN 1 AND 256),
       root_id TEXT NOT NULL,
       root_generation INTEGER NOT NULL CHECK(root_generation > 0),
       epoch INTEGER NOT NULL CHECK(epoch > 0),
       scope_kind TEXT NOT NULL CHECK(scope_kind IN ('root', 'subtree')),
       scope_relative_path TEXT NOT NULL,
       status TEXT NOT NULL CHECK(status IN (
         'running', 'comparing', 'completed', 'failed', 'cancelled', 'superseded'
       )),
       next_page_index INTEGER NOT NULL CHECK(next_page_index > 0),
       enumeration_cursor TEXT,
       comparison_cursor TEXT,
       absence_cursor TEXT,
       staged_entry_count INTEGER NOT NULL DEFAULT 0 CHECK(staged_entry_count >= 0),
       candidate_count INTEGER NOT NULL DEFAULT 0 CHECK(candidate_count >= 0),
       enumeration_complete INTEGER NOT NULL DEFAULT 0
         CHECK(enumeration_complete IN (0, 1)),
       absence_authority INTEGER NOT NULL DEFAULT 0
         CHECK(absence_authority IN (0, 1)),
       started_unix_ms INTEGER NOT NULL,
       updated_unix_ms INTEGER NOT NULL,
       completed_unix_ms INTEGER,
       last_issue_code TEXT,
       last_issue_message TEXT,
       CHECK(
         (scope_kind = 'root' AND scope_relative_path = '')
         OR
         (scope_kind = 'subtree' AND length(scope_relative_path) > 0)
       ),
       CHECK(instr(scope_relative_path, char(92)) = 0),
       CHECK(
         (last_issue_code IS NULL AND last_issue_message IS NULL)
         OR
         (last_issue_code IS NOT NULL AND last_issue_message IS NOT NULL)
       ),
       CHECK(enumeration_complete = 1 OR absence_authority = 0),
       CHECK(
         (status = 'completed' AND completed_unix_ms IS NOT NULL
           AND enumeration_complete = 1 AND absence_authority = 1)
         OR
         (status <> 'completed' AND completed_unix_ms IS NULL)
       ),
       UNIQUE(root_id, root_generation, epoch),
       FOREIGN KEY(root_id) REFERENCES library_roots(id) ON DELETE CASCADE
     )";
const RECOVERY_EXECUTION_CONTRACT_TABLE_DDL: &str =
    "CREATE TABLE library_recovery_execution_contract (
       singleton INTEGER PRIMARY KEY CHECK(singleton = 1),
       contract_version INTEGER NOT NULL CHECK(contract_version = 1),
       complete INTEGER NOT NULL CHECK(complete = 1)
     )";
const METADATA_INVENTORY_CANDIDATE_OWNER_TABLE_DDL: &str =
    "CREATE TABLE library_metadata_inventory_candidate_owners (
       run_id TEXT NOT NULL,
       candidate_key TEXT NOT NULL CHECK(length(candidate_key) BETWEEN 1 AND 1024),
       change_id INTEGER NOT NULL,
       candidate_role TEXT NOT NULL CHECK(candidate_role IN ('present', 'absence')),
       relative_path TEXT NOT NULL CHECK(length(relative_path) BETWEEN 1 AND 32767),
       previous_relative_path TEXT,
       owned_unix_ms INTEGER NOT NULL CHECK(owned_unix_ms >= 0),
       CHECK(instr(relative_path, char(92)) = 0),
       CHECK(previous_relative_path IS NULL OR (
         length(previous_relative_path) BETWEEN 1 AND 32767
         AND instr(previous_relative_path, char(92)) = 0
       )),
       PRIMARY KEY(run_id, candidate_key),
       FOREIGN KEY(run_id) REFERENCES library_metadata_inventory_runs(id) ON DELETE CASCADE,
       FOREIGN KEY(change_id) REFERENCES library_change_queue(id) ON DELETE RESTRICT
     )";
const METADATA_INVENTORY_CANDIDATE_CHANGE_INDEX_DDL: &str =
    "CREATE INDEX library_metadata_inventory_candidate_owners_change
       ON library_metadata_inventory_candidate_owners(change_id, run_id, candidate_key)";
const METADATA_INVENTORY_CANDIDATE_INSERT_GUARD_DDL: &str =
    "CREATE TRIGGER library_metadata_inventory_candidate_owner_insert_guard
       BEFORE INSERT ON library_metadata_inventory_candidate_owners
       WHEN NOT EXISTS (
         SELECT 1
         FROM library_metadata_inventory_runs AS run
         JOIN library_change_queue AS queue ON queue.id = NEW.change_id
         WHERE run.id = NEW.run_id
           AND queue.root_id = run.root_id
           AND queue.root_generation = run.root_generation
           AND queue.scope = 'path'
           AND queue.relative_path = NEW.relative_path
           AND queue.previous_relative_path IS NEW.previous_relative_path
       )
       BEGIN
         SELECT RAISE(ABORT, 'inventory candidate owner does not match path work');
       END";
const METADATA_INVENTORY_CANDIDATE_UPDATE_GUARD_DDL: &str =
    "CREATE TRIGGER library_metadata_inventory_candidate_owner_update_guard
       BEFORE UPDATE OF run_id, candidate_key, candidate_role, relative_path,
                        previous_relative_path, owned_unix_ms
       ON library_metadata_inventory_candidate_owners
       BEGIN
         SELECT RAISE(ABORT, 'inventory candidate owner identity is immutable');
       END";
const METADATA_INVENTORY_FRONTIER_TABLE_DDL: &str =
    "CREATE TABLE library_metadata_inventory_frontier (
       run_id TEXT NOT NULL,
       ordinal INTEGER NOT NULL CHECK(ordinal >= 0),
       relative_directory TEXT NOT NULL CHECK(length(relative_directory) <= 32767),
       state TEXT NOT NULL CHECK(state IN ('pending', 'enumerating', 'completed')),
       directory_identity_scheme TEXT,
       directory_identity_value TEXT,
       resume_after_relative_path TEXT,
       enumerated_entry_count INTEGER NOT NULL DEFAULT 0 CHECK(enumerated_entry_count >= 0),
       updated_unix_ms INTEGER NOT NULL CHECK(updated_unix_ms >= 0),
       CHECK(instr(relative_directory, char(92)) = 0),
       CHECK(resume_after_relative_path IS NULL OR (
         length(resume_after_relative_path) BETWEEN 1 AND 32767
         AND instr(resume_after_relative_path, char(92)) = 0
       )),
       CHECK(
         (directory_identity_scheme IS NULL AND directory_identity_value IS NULL)
         OR
         (length(directory_identity_scheme) BETWEEN 1 AND 128
           AND length(directory_identity_value) BETWEEN 1 AND 512)
       ),
       PRIMARY KEY(run_id, ordinal),
       UNIQUE(run_id, relative_directory),
       FOREIGN KEY(run_id) REFERENCES library_metadata_inventory_runs(id) ON DELETE CASCADE
     )";
const METADATA_INVENTORY_FRONTIER_STATE_INDEX_DDL: &str =
    "CREATE INDEX library_metadata_inventory_frontier_state
       ON library_metadata_inventory_frontier(run_id, state, ordinal)";
const METADATA_INVENTORY_SPOOL_CONTRACT_TABLE_V27_DDL: &str =
    "CREATE TABLE library_metadata_inventory_spool_contract (
       singleton INTEGER PRIMARY KEY CHECK(singleton = 1),
       contract_version INTEGER NOT NULL CHECK(contract_version = 1),
       complete INTEGER NOT NULL CHECK(complete = 1)
     )";
const METADATA_INVENTORY_SPOOL_TABLE_V27_DDL: &str =
    "CREATE TABLE library_metadata_inventory_spools (
       run_id TEXT PRIMARY KEY,
       authority_change_id INTEGER NOT NULL UNIQUE,
       root_id TEXT NOT NULL CHECK(length(root_id) BETWEEN 1 AND 512),
       root_generation INTEGER NOT NULL CHECK(root_generation > 0),
       scope_kind TEXT NOT NULL CHECK(scope_kind IN ('root', 'subtree')),
       scope_relative_path TEXT NOT NULL CHECK(length(scope_relative_path) <= 32767),
       state TEXT NOT NULL CHECK(state IN ('enumerating', 'ready')),
       created_unix_ms INTEGER NOT NULL CHECK(created_unix_ms >= 0),
       updated_unix_ms INTEGER NOT NULL CHECK(updated_unix_ms >= created_unix_ms),
       CHECK(instr(scope_relative_path, char(92)) = 0),
       CHECK(scope_kind = 'root' OR length(scope_relative_path) > 0),
       FOREIGN KEY(run_id) REFERENCES library_metadata_inventory_runs(id) ON DELETE CASCADE,
       FOREIGN KEY(authority_change_id)
         REFERENCES library_recovery_authorities(change_id) ON DELETE CASCADE
     )";
const METADATA_INVENTORY_SPOOL_CONTRACT_TABLE_DDL: &str =
    "CREATE TABLE library_metadata_inventory_spool_contract (
       singleton INTEGER PRIMARY KEY CHECK(singleton = 1),
       contract_version INTEGER NOT NULL CHECK(contract_version = 2),
       complete INTEGER NOT NULL CHECK(complete = 1)
     )";
const METADATA_INVENTORY_SPOOL_CONTRACT_TABLE_V31_DDL: &str =
    "CREATE TABLE library_metadata_inventory_spool_contract (
       singleton INTEGER PRIMARY KEY CHECK(singleton = 1),
       contract_version INTEGER NOT NULL CHECK(contract_version = 3),
       complete INTEGER NOT NULL CHECK(complete = 1)
     )";
const SOURCE_REVISION_METADATA_CONTRACT_TABLE_DDL: &str =
    "CREATE TABLE library_source_revision_metadata_contract (
       singleton INTEGER PRIMARY KEY CHECK(singleton = 1),
       contract_version INTEGER NOT NULL CHECK(contract_version = 1),
       complete INTEGER NOT NULL CHECK(complete = 1)
     )";
const METADATA_INVENTORY_SPOOL_TABLE_DDL: &str = "CREATE TABLE library_metadata_inventory_spools (
       run_id TEXT PRIMARY KEY,
       authority_change_id INTEGER NOT NULL UNIQUE,
       root_id TEXT NOT NULL CHECK(length(root_id) BETWEEN 1 AND 512),
       root_generation INTEGER NOT NULL CHECK(root_generation > 0),
       root_identity_scheme TEXT NOT NULL
         CHECK(length(root_identity_scheme) BETWEEN 1 AND 128),
       root_identity_value TEXT NOT NULL
         CHECK(length(root_identity_value) BETWEEN 1 AND 512),
       scope_kind TEXT NOT NULL CHECK(scope_kind IN ('root', 'subtree')),
       scope_relative_path TEXT NOT NULL CHECK(length(scope_relative_path) <= 32767),
       state TEXT NOT NULL CHECK(state IN ('enumerating', 'ready')),
       created_unix_ms INTEGER NOT NULL CHECK(created_unix_ms >= 0),
       updated_unix_ms INTEGER NOT NULL CHECK(updated_unix_ms >= created_unix_ms),
       CHECK(instr(scope_relative_path, char(92)) = 0),
       CHECK(scope_kind = 'root' OR length(scope_relative_path) > 0),
       FOREIGN KEY(run_id) REFERENCES library_metadata_inventory_runs(id) ON DELETE CASCADE,
       FOREIGN KEY(authority_change_id)
         REFERENCES library_recovery_authorities(change_id) ON DELETE CASCADE
     )";
const METADATA_INVENTORY_SPOOL_DIRECTORY_TABLE_DDL: &str =
    "CREATE TABLE library_metadata_inventory_spool_directories (
       run_id TEXT NOT NULL,
       ordinal INTEGER NOT NULL CHECK(ordinal >= 0),
       relative_directory TEXT NOT NULL CHECK(length(relative_directory) <= 32767),
       state TEXT NOT NULL CHECK(state IN ('pending', 'enumerating', 'completed')),
       directory_identity_scheme TEXT,
       directory_identity_value TEXT,
       source_entry_count INTEGER NOT NULL DEFAULT 0 CHECK(source_entry_count >= 0),
       created_unix_ms INTEGER NOT NULL CHECK(created_unix_ms >= 0),
       updated_unix_ms INTEGER NOT NULL CHECK(updated_unix_ms >= created_unix_ms),
       CHECK(instr(relative_directory, char(92)) = 0),
       CHECK(
         (directory_identity_scheme IS NULL AND directory_identity_value IS NULL)
         OR
         (length(directory_identity_scheme) BETWEEN 1 AND 128
           AND length(directory_identity_value) BETWEEN 1 AND 512)
       ),
       PRIMARY KEY(run_id, relative_directory),
       UNIQUE(run_id, ordinal),
       FOREIGN KEY(run_id) REFERENCES library_metadata_inventory_spools(run_id)
         ON DELETE CASCADE
     )";
const METADATA_INVENTORY_SPOOL_DIRECTORY_STATE_INDEX_DDL: &str =
    "CREATE INDEX library_metadata_inventory_spool_directories_state
       ON library_metadata_inventory_spool_directories(run_id, state, ordinal)";
const METADATA_INVENTORY_SPOOL_ENTRY_TABLE_DDL: &str =
    "CREATE TABLE library_metadata_inventory_spool_entries (
       run_id TEXT NOT NULL,
       directory_relative_path TEXT,
       relative_path TEXT NOT NULL CHECK(length(relative_path) BETWEEN 1 AND 32767),
       entry_kind TEXT NOT NULL CHECK(entry_kind IN ('file', 'directory', 'other')),
       file_size INTEGER CHECK(file_size IS NULL OR file_size >= 0),
       modified_unix_ms INTEGER NOT NULL,
       file_identity_scheme TEXT,
       file_identity_value TEXT,
       placeholder_state TEXT NOT NULL CHECK(placeholder_state IN (
         'available', 'offline', 'recall_on_open', 'recall_on_data_access'
       )),
       is_reparse_point INTEGER NOT NULL CHECK(is_reparse_point IN (0, 1)),
       staged_unix_ms INTEGER NOT NULL CHECK(staged_unix_ms >= 0),
       CHECK(directory_relative_path IS NULL OR instr(directory_relative_path, char(92)) = 0),
       CHECK(instr(relative_path, char(92)) = 0),
       CHECK(
         (entry_kind = 'file' AND file_size IS NOT NULL)
         OR
         (entry_kind <> 'file' AND file_size IS NULL)
       ),
       CHECK(
         (file_identity_scheme IS NULL AND file_identity_value IS NULL)
         OR
         (length(file_identity_scheme) BETWEEN 1 AND 128
           AND length(file_identity_value) BETWEEN 1 AND 512)
       ),
       PRIMARY KEY(run_id, relative_path),
       FOREIGN KEY(run_id, directory_relative_path)
         REFERENCES library_metadata_inventory_spool_directories(run_id, relative_directory)
         ON DELETE CASCADE
     )";
const METADATA_INVENTORY_SPOOL_ENTRY_TABLE_V31_DDL: &str =
    "CREATE TABLE library_metadata_inventory_spool_entries (
       run_id TEXT NOT NULL,
       directory_relative_path TEXT,
       relative_path TEXT NOT NULL CHECK(length(relative_path) BETWEEN 1 AND 32767),
       entry_kind TEXT NOT NULL CHECK(entry_kind IN ('file', 'directory', 'other')),
       file_size INTEGER CHECK(file_size IS NULL OR file_size >= 0),
       modified_unix_ms INTEGER NOT NULL,
       file_identity_scheme TEXT,
       file_identity_value TEXT,
       placeholder_state TEXT NOT NULL CHECK(placeholder_state IN (
         'available', 'offline', 'recall_on_open', 'recall_on_data_access'
       )),
       is_reparse_point INTEGER NOT NULL CHECK(is_reparse_point IN (0, 1)),
       staged_unix_ms INTEGER NOT NULL CHECK(staged_unix_ms >= 0),
       source_revision_token TEXT,
       CHECK(directory_relative_path IS NULL OR instr(directory_relative_path, char(92)) = 0),
       CHECK(instr(relative_path, char(92)) = 0),
       CHECK(
         (entry_kind = 'file' AND file_size IS NOT NULL)
         OR
         (entry_kind <> 'file' AND file_size IS NULL)
       ),
       CHECK(
         (file_identity_scheme IS NULL AND file_identity_value IS NULL)
         OR
         (length(file_identity_scheme) BETWEEN 1 AND 128
           AND length(file_identity_value) BETWEEN 1 AND 512)
       ),
       PRIMARY KEY(run_id, relative_path),
       FOREIGN KEY(run_id, directory_relative_path)
         REFERENCES library_metadata_inventory_spool_directories(run_id, relative_directory)
         ON DELETE CASCADE
     )";
const METADATA_INVENTORY_SPOOL_ENTRY_ORDER_INDEX_DDL: &str =
    "CREATE INDEX library_metadata_inventory_spool_entries_order
       ON library_metadata_inventory_spool_entries(run_id, relative_path)";
const ROOT_PUBLICATION_NAMESPACE_CONTRACT_TABLE_DDL: &str =
    "CREATE TABLE library_root_publication_namespace_contract (
       singleton INTEGER PRIMARY KEY CHECK(singleton = 1),
       contract_version INTEGER NOT NULL CHECK(contract_version = 1),
       complete INTEGER NOT NULL CHECK(complete = 1)
     )";
const ROOT_PUBLICATION_NAMESPACE_TABLE_DDL: &str =
    "CREATE TABLE library_root_publication_namespaces (
       root_id TEXT PRIMARY KEY,
       root_generation INTEGER NOT NULL CHECK(root_generation > 0),
       identity_scheme TEXT NOT NULL CHECK(identity_scheme = 'windows-file-id-128-v1'),
       identity_value TEXT NOT NULL CHECK(length(identity_value) = 49),
       authority_kind TEXT NOT NULL CHECK(authority_kind IN (
         'journal_v3_migration', 'metadata_inventory', 'foreground_scan'
       )),
       established_catalog_revision INTEGER NOT NULL
         CHECK(established_catalog_revision >= 0),
       established_unix_ms INTEGER NOT NULL CHECK(established_unix_ms >= 0),
       updated_unix_ms INTEGER NOT NULL CHECK(updated_unix_ms >= established_unix_ms),
       FOREIGN KEY(root_id) REFERENCES library_roots(id) ON DELETE CASCADE
     )";
const SCAN_PUBLICATION_NAMESPACE_BINDING_TABLE_DDL: &str =
    "CREATE TABLE library_scan_publication_namespace_bindings (
       scan_id TEXT PRIMARY KEY,
       root_id TEXT NOT NULL,
       root_generation INTEGER NOT NULL CHECK(root_generation > 0),
       identity_scheme TEXT NOT NULL CHECK(identity_scheme = 'windows-file-id-128-v1'),
       identity_value TEXT NOT NULL CHECK(length(identity_value) = 49),
       bound_unix_ms INTEGER NOT NULL CHECK(bound_unix_ms >= 0),
       FOREIGN KEY(scan_id) REFERENCES scan_runs(id) ON DELETE CASCADE,
       FOREIGN KEY(root_id) REFERENCES library_roots(id) ON DELETE CASCADE
     )";
const SCAN_PUBLICATION_NAMESPACE_ROOT_INDEX_DDL: &str =
    "CREATE UNIQUE INDEX library_scan_publication_namespace_root
       ON library_scan_publication_namespace_bindings(root_id, root_generation)";
const LIVE_GAP_RECOVERY_CONTRACT_TABLE_DDL: &str =
    "CREATE TABLE library_live_gap_recovery_contract (
       singleton INTEGER PRIMARY KEY CHECK(singleton = 1),
       contract_version INTEGER NOT NULL CHECK(contract_version = 1),
       complete INTEGER NOT NULL CHECK(complete = 1)
     )";
const LIVE_GAP_RECOVERY_CLAIM_TABLE_DDL: &str = "CREATE TABLE library_live_gap_recovery_claims (
       gap_change_id INTEGER PRIMARY KEY,
       root_id TEXT NOT NULL CHECK(length(root_id) BETWEEN 1 AND 512),
       root_generation INTEGER NOT NULL CHECK(root_generation > 0),
       consumer_kind TEXT NOT NULL CHECK(consumer_kind IN (
          'pending_journal', 'journal_source_range',
          'metadata_inventory_control', 'explicit_recovery_required',
          'foreground_scan'
        )),
       opening_volume_guid TEXT,
       opening_volume_serial TEXT,
       opening_root_reference_version INTEGER,
       opening_root_file_reference BLOB,
       opening_journal_id TEXT,
       opening_next_usn TEXT,
       protocol_version INTEGER,
       contract_version INTEGER,
       source_range_id TEXT,
       recovery_change_id INTEGER,
       foreground_scan_id TEXT,
       created_unix_ms INTEGER NOT NULL CHECK(created_unix_ms >= 0),
       consumed_unix_ms INTEGER,
       CHECK(
         (consumer_kind IN ('pending_journal', 'journal_source_range')
           AND opening_volume_guid IS NOT NULL
           AND opening_volume_serial IS NOT NULL
           AND opening_root_reference_version IN (2, 3)
           AND opening_root_file_reference IS NOT NULL
           AND opening_journal_id IS NOT NULL
           AND opening_next_usn IS NOT NULL
           AND protocol_version BETWEEN 1 AND 65535
           AND contract_version = 1)
         OR
         (consumer_kind = 'metadata_inventory_control'
           AND (
             (opening_volume_guid IS NULL
               AND opening_volume_serial IS NULL
               AND opening_root_reference_version IS NULL
               AND opening_root_file_reference IS NULL
               AND opening_journal_id IS NULL
               AND opening_next_usn IS NULL
               AND protocol_version IS NULL
               AND contract_version IS NULL)
             OR
             (opening_volume_guid IS NOT NULL
               AND opening_volume_serial IS NOT NULL
               AND opening_root_reference_version IN (2, 3)
               AND opening_root_file_reference IS NOT NULL
               AND opening_journal_id IS NOT NULL
               AND opening_next_usn IS NOT NULL
               AND protocol_version BETWEEN 1 AND 65535
               AND contract_version = 1)
           ))
         OR
          (consumer_kind IN ('explicit_recovery_required', 'foreground_scan')
            AND opening_volume_guid IS NULL
           AND opening_volume_serial IS NULL
           AND opening_root_reference_version IS NULL
           AND opening_root_file_reference IS NULL
           AND opening_journal_id IS NULL
           AND opening_next_usn IS NULL
           AND protocol_version IS NULL
           AND contract_version IS NULL)
       ),
       CHECK(
         opening_root_reference_version IS NULL
         OR (opening_root_reference_version = 2 AND length(opening_root_file_reference) = 8)
         OR (opening_root_reference_version = 3 AND length(opening_root_file_reference) = 16)
       ),
       CHECK(opening_volume_serial IS NULL OR (
         length(opening_volume_serial) BETWEEN 1 AND 20
         AND opening_volume_serial NOT GLOB '*[^0-9]*'
       )),
       CHECK(opening_journal_id IS NULL OR (
         length(opening_journal_id) BETWEEN 1 AND 20
         AND opening_journal_id <> '0'
         AND opening_journal_id NOT GLOB '*[^0-9]*'
       )),
       CHECK(opening_next_usn IS NULL OR (
         length(opening_next_usn) BETWEEN 1 AND 19
         AND opening_next_usn NOT GLOB '*[^0-9]*'
       )),
       CHECK(
          (consumer_kind = 'pending_journal'
            AND source_range_id IS NULL AND recovery_change_id IS NULL
            AND foreground_scan_id IS NULL
            AND consumed_unix_ms IS NULL)
         OR
          (consumer_kind = 'journal_source_range'
            AND source_range_id IS NOT NULL AND recovery_change_id IS NULL
            AND foreground_scan_id IS NULL
            AND consumed_unix_ms IS NOT NULL)
         OR
          (consumer_kind = 'metadata_inventory_control'
            AND source_range_id IS NULL AND recovery_change_id IS NOT NULL
            AND foreground_scan_id IS NULL
            AND consumed_unix_ms IS NOT NULL)
         OR
          (consumer_kind = 'explicit_recovery_required'
            AND source_range_id IS NULL AND recovery_change_id IS NULL
            AND foreground_scan_id IS NULL
            AND consumed_unix_ms IS NULL)
          OR
          (consumer_kind = 'foreground_scan'
            AND source_range_id IS NULL AND recovery_change_id IS NULL
            AND foreground_scan_id IS NOT NULL)
       ),
       CHECK(consumed_unix_ms IS NULL OR consumed_unix_ms >= created_unix_ms),
       FOREIGN KEY(gap_change_id) REFERENCES library_change_queue(id) ON DELETE CASCADE,
       FOREIGN KEY(source_range_id)
         REFERENCES library_persistent_journal_source_ranges(id) ON DELETE RESTRICT,
       FOREIGN KEY(recovery_change_id) REFERENCES library_change_queue(id) ON DELETE RESTRICT,
       FOREIGN KEY(foreground_scan_id) REFERENCES scan_runs(id) ON DELETE RESTRICT
      )";
const LIVE_GAP_RECOVERY_ROOT_INDEX_DDL: &str = "CREATE INDEX library_live_gap_recovery_claims_root
       ON library_live_gap_recovery_claims(
         root_id, root_generation, consumer_kind, gap_change_id
       )";
const LIVE_GAP_RECOVERY_INSERT_GUARD_DDL: &str =
    "CREATE TRIGGER library_live_gap_recovery_claim_insert_guard
       BEFORE INSERT ON library_live_gap_recovery_claims
       WHEN NOT EXISTS (
         SELECT 1
         FROM library_change_queue AS gap
         JOIN library_change_queue_lanes AS lane ON lane.change_id = gap.id
         WHERE gap.id = NEW.gap_change_id
           AND gap.root_id = NEW.root_id
           AND gap.root_generation = NEW.root_generation
           AND gap.intent_kind = 'freshness_unknown'
           AND gap.scope = 'root' AND gap.relative_path = ''
           AND gap.previous_relative_path IS NULL
           AND (
              (NEW.consumer_kind IN (
                 'pending_journal', 'journal_source_range', 'metadata_inventory_control'
               )
                AND gap.origin = 'live_notification' AND lane.lane = 'p0_live')
             OR
             (NEW.consumer_kind = 'explicit_recovery_required'
               AND gap.origin = 'startup_catch_up' AND lane.lane = 'p1_journal'
                AND gap.last_failure_code = 'live_gap_v30_explicit_recovery_required')
            )
            AND NEW.consumer_kind <> 'foreground_scan'
        )
       BEGIN
         SELECT RAISE(ABORT, 'live gap claim does not match durable gap work');
       END";
const LIVE_GAP_RECOVERY_IDENTITY_UPDATE_GUARD_DDL: &str =
    "CREATE TRIGGER library_live_gap_recovery_claim_identity_update_guard
       BEFORE UPDATE OF gap_change_id, root_id, root_generation,
                        opening_volume_guid, opening_volume_serial,
                        opening_root_reference_version, opening_root_file_reference,
                        opening_journal_id, opening_next_usn,
                        protocol_version, contract_version, created_unix_ms
       ON library_live_gap_recovery_claims
       BEGIN
         SELECT RAISE(ABORT, 'live gap claim identity is immutable');
       END";
const METADATA_INVENTORY_SPOOL_BINDING_UPDATE_GUARD_DDL: &str =
    "CREATE TRIGGER library_metadata_inventory_spool_binding_update_guard
       BEFORE UPDATE OF run_id, authority_change_id, root_id, root_generation,
                        root_identity_scheme, root_identity_value,
                        scope_kind, scope_relative_path, created_unix_ms
       ON library_metadata_inventory_spools
       BEGIN
         SELECT RAISE(ABORT, 'inventory spool authority binding is immutable');
     END";
const METADATA_INVENTORY_SPOOL_BINDING_UPDATE_GUARD_V27_DDL: &str =
    "CREATE TRIGGER library_metadata_inventory_spool_binding_update_guard
       BEFORE UPDATE OF run_id, authority_change_id, root_id, root_generation,
                        scope_kind, scope_relative_path, created_unix_ms
       ON library_metadata_inventory_spools
       BEGIN
         SELECT RAISE(ABORT, 'inventory spool authority binding is immutable');
       END";
const METADATA_INVENTORY_SPOOL_DIRECTORY_COMPLETE_GUARD_DDL: &str =
    "CREATE TRIGGER library_metadata_inventory_spool_directory_complete_guard
       BEFORE UPDATE OF directory_identity_scheme, directory_identity_value,
                        source_entry_count
       ON library_metadata_inventory_spool_directories
       WHEN OLD.state = 'completed'
       BEGIN
         SELECT RAISE(ABORT, 'completed inventory spool directory is immutable');
       END";

fn validate_change_lane_contract(connection: &Connection) -> Result<(), ScanError> {
    validate_change_lane_contract_with_depth(connection, ContractValidationDepth::Full)
}

fn validate_change_lane_contract_with_depth(
    connection: &Connection,
    depth: ContractValidationDepth,
) -> Result<(), ScanError> {
    let columns_match = table_columns_match(
        connection,
        "library_change_lane_contract",
        &[
            ("singleton", "INTEGER", false, 1),
            ("complete", "INTEGER", true, 0),
        ],
    )? && table_columns_match(
        connection,
        "library_change_queue_lanes",
        &[
            ("change_id", "INTEGER", false, 1),
            ("lane", "TEXT", true, 0),
        ],
    )?;
    let marker_complete = connection
        .query_row(
            "SELECT singleton = 1 AND complete = 1
             FROM library_change_lane_contract",
            [],
            |row| row.get::<_, bool>(0),
        )
        .optional()
        .map_err(database_error)?
        .unwrap_or(false);
    let schema_matches = schema_object_sql_matches(
        connection,
        "table",
        "library_change_lane_contract",
        CHANGE_LANE_CONTRACT_TABLE_DDL,
    )? && schema_object_sql_matches(
        connection,
        "table",
        "library_change_queue_lanes",
        CHANGE_LANE_QUEUE_TABLE_DDL,
    )? && schema_object_sql_matches(
        connection,
        "index",
        "library_change_queue_lanes_eligible",
        CHANGE_LANE_INDEX_DDL,
    )? && schema_object_sql_matches(
        connection,
        "trigger",
        "library_change_queue_lane_insert",
        CHANGE_LANE_INSERT_TRIGGER_DDL,
    )? && schema_object_sql_matches(
        connection,
        "trigger",
        "library_change_queue_lane_origin_update",
        CHANGE_LANE_ORIGIN_TRIGGER_DDL,
    )? && schema_object_sql_matches(
        connection,
        "trigger",
        "library_change_queue_lane_insert_guard",
        CHANGE_LANE_INSERT_GUARD_DDL,
    )? && schema_object_sql_matches(
        connection,
        "trigger",
        "library_change_queue_lane_update_guard",
        CHANGE_LANE_UPDATE_GUARD_DDL,
    )?;
    let foreign_key_matches = cascade_foreign_key_matches(
        connection,
        "library_change_queue_lanes",
        "change_id",
        "library_change_queue",
        "id",
    )?;
    if !columns_match || !marker_complete || !schema_matches || !foreign_key_matches {
        return Err(unverifiable_change_lane_contract());
    }
    if !depth.includes_rows() {
        return Ok(());
    }
    record_current_schema_row_audit();
    let invalid_rows = connection
        .query_row(
            "SELECT EXISTS(
               SELECT 1
               FROM library_change_queue AS queue
               LEFT JOIN library_change_queue_lanes AS lanes ON lanes.change_id = queue.id
               WHERE lanes.change_id IS NULL
                  OR lanes.lane <> CASE queue.origin
                    WHEN 'live_notification' THEN 'p0_live'
                    WHEN 'startup_catch_up' THEN 'p1_journal'
                    ELSE 'p2_recovery'
                  END
             ) OR EXISTS(
               SELECT 1 FROM pragma_foreign_key_check('library_change_queue_lanes')
             )",
            [],
            |row| row.get::<_, bool>(0),
        )
        .map_err(database_error)?;
    if invalid_rows {
        return Err(unverifiable_change_lane_contract());
    }
    Ok(())
}

fn unverifiable_change_lane_contract() -> ScanError {
    ScanError::new(
        "catalog_change_lane_contract_unverifiable",
        "The catalog cannot prove its durable change-lane authority",
    )
}

fn validate_recovery_authority_contract(connection: &Connection) -> Result<(), ScanError> {
    validate_recovery_authority_contract_with_depth(connection, ContractValidationDepth::Full)
}

fn validate_recovery_authority_contract_with_depth(
    connection: &Connection,
    depth: ContractValidationDepth,
) -> Result<(), ScanError> {
    let columns_match = table_columns_match(
        connection,
        "library_recovery_authority_contract",
        &[
            ("singleton", "INTEGER", false, 1),
            ("complete", "INTEGER", true, 0),
        ],
    )? && table_columns_match(
        connection,
        "library_recovery_authorities",
        &[
            ("change_id", "INTEGER", false, 1),
            ("run_id", "TEXT", true, 0),
            ("root_id", "TEXT", true, 0),
            ("root_generation", "INTEGER", true, 0),
            ("reason", "TEXT", true, 0),
            ("opening_journal_id", "TEXT", false, 0),
            ("opening_next_usn", "TEXT", false, 0),
            ("authorized_unix_ms", "INTEGER", true, 0),
            ("retired_unix_ms", "INTEGER", false, 0),
        ],
    )?;
    let marker_complete = connection
        .query_row(
            "SELECT singleton = 1 AND complete = 1
             FROM library_recovery_authority_contract",
            [],
            |row| row.get::<_, bool>(0),
        )
        .optional()
        .map_err(database_error)?
        .unwrap_or(false);
    let schema_matches = schema_object_sql_matches(
        connection,
        "table",
        "library_recovery_authority_contract",
        RECOVERY_AUTHORITY_CONTRACT_TABLE_DDL,
    )? && schema_object_sql_matches(
        connection,
        "table",
        "library_recovery_authorities",
        RECOVERY_AUTHORITY_TABLE_DDL,
    )? && schema_object_sql_matches(
        connection,
        "index",
        "library_recovery_authorities_root",
        RECOVERY_AUTHORITY_ROOT_INDEX_DDL,
    )? && schema_object_sql_matches(
        connection,
        "trigger",
        "library_recovery_authority_insert_guard",
        RECOVERY_AUTHORITY_INSERT_GUARD_DDL,
    )? && schema_object_sql_matches(
        connection,
        "trigger",
        "library_recovery_authority_update_guard",
        RECOVERY_AUTHORITY_UPDATE_GUARD_DDL,
    )?;
    let foreign_key_matches = cascade_foreign_key_matches(
        connection,
        "library_recovery_authorities",
        "change_id",
        "library_change_queue",
        "id",
    )?;
    if !columns_match || !marker_complete || !schema_matches || !foreign_key_matches {
        return Err(unverifiable_recovery_authority_contract());
    }
    if !depth.includes_rows() {
        return Ok(());
    }
    record_current_schema_row_audit();
    let invalid_rows = connection
        .query_row(
            "SELECT EXISTS(
               SELECT 1
               FROM library_recovery_authorities AS authority
               LEFT JOIN library_change_queue AS queue ON queue.id = authority.change_id
               LEFT JOIN library_change_queue_lanes AS lanes
                 ON lanes.change_id = authority.change_id
               WHERE queue.id IS NULL
                  OR queue.root_id <> authority.root_id
                  OR queue.root_generation <> authority.root_generation
                  OR lanes.lane <> 'p2_recovery'
             ) OR EXISTS(
               SELECT 1 FROM pragma_foreign_key_check('library_recovery_authorities')
             )",
            [],
            |row| row.get::<_, bool>(0),
        )
        .map_err(database_error)?;
    let mut boundaries = connection
        .prepare(
            "SELECT opening_journal_id, opening_next_usn
             FROM library_recovery_authorities
             WHERE opening_journal_id IS NOT NULL OR opening_next_usn IS NOT NULL",
        )
        .map_err(database_error)?;
    let canonical_boundaries = boundaries
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(database_error)?
        .all(|row| {
            row.is_ok_and(|(journal_id, next_usn)| {
                JournalIdentifier::parse_canonical(&journal_id).is_ok()
                    && JournalUsn::parse_canonical(&next_usn).is_ok()
            })
        });
    if invalid_rows || !canonical_boundaries {
        return Err(unverifiable_recovery_authority_contract());
    }
    Ok(())
}

fn validate_persistent_journal_baseline_contract(connection: &Connection) -> Result<(), ScanError> {
    validate_persistent_journal_baseline_contract_with_depth(
        connection,
        ContractValidationDepth::Full,
    )
}

fn validate_persistent_journal_baseline_contract_with_depth(
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
    let invalid_rows = connection
        .query_row(
            "SELECT EXISTS(
               SELECT 1 FROM library_persistent_journal_baselines AS baseline
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
                           AND root.continuity_state = 'current'
                           AND checkpoint.root_id IS NOT NULL
                           AND checkpoint.continuity_state = 'current'
                           AND checkpoint.volume_guid = baseline.volume_guid
                           AND checkpoint.volume_serial = baseline.volume_serial
                           AND checkpoint.root_reference_version = baseline.root_reference_version
                           AND checkpoint.root_file_reference = baseline.root_file_reference
                           AND checkpoint.journal_id = baseline.journal_id
                           AND checkpoint.next_unread_usn = baseline.closing_next_usn
                           AND checkpoint.captured_exclusive_end = baseline.closing_next_usn
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

fn validate_recovery_execution_contract(connection: &Connection) -> Result<(), ScanError> {
    validate_recovery_execution_contract_with_depth(connection, ContractValidationDepth::Full)
}

fn validate_recovery_execution_contract_with_depth(
    connection: &Connection,
    depth: ContractValidationDepth,
) -> Result<(), ScanError> {
    let columns_match = table_columns_match(
        connection,
        "library_recovery_execution_contract",
        &[
            ("singleton", "INTEGER", false, 1),
            ("contract_version", "INTEGER", true, 0),
            ("complete", "INTEGER", true, 0),
        ],
    )? && table_columns_match(
        connection,
        "library_metadata_inventory_candidate_owners",
        &[
            ("run_id", "TEXT", true, 1),
            ("candidate_key", "TEXT", true, 2),
            ("change_id", "INTEGER", true, 0),
            ("candidate_role", "TEXT", true, 0),
            ("relative_path", "TEXT", true, 0),
            ("previous_relative_path", "TEXT", false, 0),
            ("owned_unix_ms", "INTEGER", true, 0),
        ],
    )? && table_columns_match(
        connection,
        "library_metadata_inventory_frontier",
        &[
            ("run_id", "TEXT", true, 1),
            ("ordinal", "INTEGER", true, 2),
            ("relative_directory", "TEXT", true, 0),
            ("state", "TEXT", true, 0),
            ("directory_identity_scheme", "TEXT", false, 0),
            ("directory_identity_value", "TEXT", false, 0),
            ("resume_after_relative_path", "TEXT", false, 0),
            ("enumerated_entry_count", "INTEGER", true, 0),
            ("updated_unix_ms", "INTEGER", true, 0),
        ],
    )?;
    let marker_complete = connection
        .query_row(
            "SELECT singleton = 1 AND contract_version = 1 AND complete = 1
             FROM library_recovery_execution_contract",
            [],
            |row| row.get::<_, bool>(0),
        )
        .optional()
        .map_err(database_error)?
        .unwrap_or(false);
    let schema_matches = schema_object_sql_matches(
        connection,
        "table",
        "library_recovery_execution_contract",
        RECOVERY_EXECUTION_CONTRACT_TABLE_DDL,
    )? && schema_object_sql_matches(
        connection,
        "table",
        "library_metadata_inventory_candidate_owners",
        METADATA_INVENTORY_CANDIDATE_OWNER_TABLE_DDL,
    )? && schema_object_sql_matches(
        connection,
        "index",
        "library_metadata_inventory_candidate_owners_change",
        METADATA_INVENTORY_CANDIDATE_CHANGE_INDEX_DDL,
    )? && schema_object_sql_matches(
        connection,
        "trigger",
        "library_metadata_inventory_candidate_owner_insert_guard",
        METADATA_INVENTORY_CANDIDATE_INSERT_GUARD_DDL,
    )? && schema_object_sql_matches(
        connection,
        "trigger",
        "library_metadata_inventory_candidate_owner_update_guard",
        METADATA_INVENTORY_CANDIDATE_UPDATE_GUARD_DDL,
    )? && schema_object_sql_matches(
        connection,
        "table",
        "library_metadata_inventory_frontier",
        METADATA_INVENTORY_FRONTIER_TABLE_DDL,
    )? && schema_object_sql_matches(
        connection,
        "index",
        "library_metadata_inventory_frontier_state",
        METADATA_INVENTORY_FRONTIER_STATE_INDEX_DDL,
    )?;
    if !columns_match || !marker_complete || !schema_matches {
        return Err(ScanError::new(
            "catalog_recovery_execution_contract_unverifiable",
            "The catalog cannot prove recovery candidate ownership and frontier lifecycle",
        ));
    }
    if !depth.includes_rows() {
        return Ok(());
    }
    record_current_schema_row_audit();
    let invalid_relations = connection
        .query_row(
            "SELECT EXISTS(
               SELECT 1
               FROM library_metadata_inventory_candidate_owners AS owner
               LEFT JOIN library_metadata_inventory_runs AS run ON run.id = owner.run_id
               LEFT JOIN library_change_queue AS queue ON queue.id = owner.change_id
               WHERE run.id IS NULL OR queue.id IS NULL
                  OR queue.root_id <> run.root_id
                  OR queue.root_generation <> run.root_generation
                  OR queue.scope <> 'path'
                  OR queue.relative_path <> owner.relative_path
                  OR queue.previous_relative_path IS NOT owner.previous_relative_path
             ) OR EXISTS(
               SELECT 1
               FROM library_metadata_inventory_frontier AS frontier
               LEFT JOIN library_metadata_inventory_runs AS run ON run.id = frontier.run_id
               WHERE run.id IS NULL
                  OR (run.status IN ('completed', 'failed', 'cancelled', 'superseded')
                    AND frontier.state <> 'completed')
                  OR frontier.ordinal <> (
                    SELECT COUNT(*)
                    FROM library_metadata_inventory_frontier AS earlier
                    WHERE earlier.run_id = frontier.run_id
                      AND earlier.ordinal < frontier.ordinal
                  )
                  OR (frontier.state = 'pending' AND (
                    frontier.ordinal <> (
                      SELECT MAX(last.ordinal)
                      FROM library_metadata_inventory_frontier AS last
                      WHERE last.run_id = frontier.run_id
                    )
                    OR frontier.resume_after_relative_path IS NOT NULL
                    OR frontier.enumerated_entry_count <> 0
                    OR NOT EXISTS(
                      SELECT 1
                      FROM library_metadata_inventory_entries AS entry
                      WHERE entry.run_id = frontier.run_id
                        AND entry.relative_path = frontier.relative_directory
                        AND entry.entry_kind = 'directory'
                    )
                  ))
                  OR (frontier.state = 'enumerating'
                    AND frontier.resume_after_relative_path IS NOT NULL
                    AND NOT EXISTS(
                      SELECT 1
                      FROM library_metadata_inventory_entries AS entry
                      WHERE entry.run_id = frontier.run_id
                        AND entry.relative_path = frontier.resume_after_relative_path
                    ))
             ) OR EXISTS(
               SELECT 1
               FROM library_metadata_inventory_runs AS run
               WHERE (run.status = 'running' AND (
                 run.enumeration_complete <> 0
                 OR (run.next_page_index = 1 AND (
                   run.staged_entry_count <> 0
                   OR EXISTS(
                     SELECT 1 FROM library_metadata_inventory_frontier AS frontier
                     WHERE frontier.run_id = run.id
                   )
                 ))
                 OR (run.next_page_index > 1 AND NOT EXISTS(
                   SELECT 1 FROM library_metadata_inventory_frontier AS frontier
                   WHERE frontier.run_id = run.id
                     AND frontier.state IN ('pending', 'enumerating')
                 ))
                 OR EXISTS(
                   SELECT 1 FROM library_metadata_inventory_frontier AS frontier
                   WHERE frontier.run_id = run.id AND frontier.state = 'completed'
                 )
               ))
               OR (run.status = 'comparing' AND (
                 run.enumeration_complete <> 1
                 OR 1 <> (
                   SELECT COUNT(*) FROM library_metadata_inventory_frontier AS frontier
                   WHERE frontier.run_id = run.id AND frontier.state = 'completed'
                 )
                 OR 1 <> (
                   SELECT COUNT(*) FROM library_metadata_inventory_frontier AS frontier
                   WHERE frontier.run_id = run.id
                 )
               ))
             ) OR EXISTS(
               SELECT 1
               FROM library_recovery_authorities AS authority
               LEFT JOIN library_change_queue AS queue ON queue.id = authority.change_id
               LEFT JOIN library_metadata_inventory_runs AS run ON run.id = authority.run_id
               WHERE queue.id IS NULL
                  OR (authority.retired_unix_ms IS NULL
                    AND queue.status NOT IN ('pending', 'leased', 'retry_wait'))
                  OR (authority.retired_unix_ms IS NOT NULL
                    AND queue.status NOT IN ('completed', 'superseded'))
                  OR (authority.retired_unix_ms IS NOT NULL
                    AND run.id IS NOT NULL AND run.status <> 'completed')
                  OR (run.status = 'completed' AND EXISTS(
                    SELECT 1
                    FROM library_metadata_inventory_candidate_owners AS owner
                    JOIN library_change_queue AS candidate ON candidate.id = owner.change_id
                    WHERE owner.run_id = run.id
                      AND candidate.status IN ('pending', 'leased', 'retry_wait')
                  ))
             ) OR EXISTS(
               SELECT 1 FROM pragma_foreign_key_check(
                 'library_metadata_inventory_candidate_owners'
               )
             ) OR EXISTS(
               SELECT 1 FROM pragma_foreign_key_check('library_metadata_inventory_frontier')
             )",
            [],
            |row| row.get::<_, bool>(0),
        )
        .map_err(database_error)?;
    if invalid_relations {
        return Err(ScanError::new(
            "catalog_recovery_execution_contract_unverifiable",
            "The catalog cannot prove recovery candidate ownership and frontier lifecycle",
        ));
    }
    Ok(())
}

fn validate_root_publication_namespace_contract(connection: &Connection) -> Result<(), ScanError> {
    validate_root_publication_namespace_contract_with_depth(
        connection,
        ContractValidationDepth::Full,
    )
}

fn validate_root_publication_namespace_contract_with_depth(
    connection: &Connection,
    depth: ContractValidationDepth,
) -> Result<(), ScanError> {
    let columns_match = table_columns_match(
        connection,
        "library_root_publication_namespace_contract",
        &[
            ("singleton", "INTEGER", false, 1),
            ("contract_version", "INTEGER", true, 0),
            ("complete", "INTEGER", true, 0),
        ],
    )? && table_columns_match(
        connection,
        "library_root_publication_namespaces",
        &[
            ("root_id", "TEXT", false, 1),
            ("root_generation", "INTEGER", true, 0),
            ("identity_scheme", "TEXT", true, 0),
            ("identity_value", "TEXT", true, 0),
            ("authority_kind", "TEXT", true, 0),
            ("established_catalog_revision", "INTEGER", true, 0),
            ("established_unix_ms", "INTEGER", true, 0),
            ("updated_unix_ms", "INTEGER", true, 0),
        ],
    )? && table_columns_match(
        connection,
        "library_scan_publication_namespace_bindings",
        &[
            ("scan_id", "TEXT", false, 1),
            ("root_id", "TEXT", true, 0),
            ("root_generation", "INTEGER", true, 0),
            ("identity_scheme", "TEXT", true, 0),
            ("identity_value", "TEXT", true, 0),
            ("bound_unix_ms", "INTEGER", true, 0),
        ],
    )?;
    let schema_matches = schema_object_sql_matches(
        connection,
        "table",
        "library_root_publication_namespace_contract",
        ROOT_PUBLICATION_NAMESPACE_CONTRACT_TABLE_DDL,
    )? && schema_object_sql_matches(
        connection,
        "table",
        "library_root_publication_namespaces",
        ROOT_PUBLICATION_NAMESPACE_TABLE_DDL,
    )? && schema_object_sql_matches(
        connection,
        "table",
        "library_scan_publication_namespace_bindings",
        SCAN_PUBLICATION_NAMESPACE_BINDING_TABLE_DDL,
    )? && schema_object_sql_matches(
        connection,
        "index",
        "library_scan_publication_namespace_root",
        SCAN_PUBLICATION_NAMESPACE_ROOT_INDEX_DDL,
    )?;
    let marker_complete = connection
        .query_row(
            "SELECT contract_version = 1 AND complete = 1
             FROM library_root_publication_namespace_contract WHERE singleton = 1",
            [],
            |row| row.get::<_, bool>(0),
        )
        .optional()
        .map_err(database_error)?
        .unwrap_or(false);
    if !columns_match || !schema_matches || !marker_complete {
        return Err(unverifiable_root_publication_namespace_contract());
    }
    if !depth.includes_rows() {
        return Ok(());
    }
    record_current_schema_row_audit();
    let invalid_relations = connection
        .query_row(
            "SELECT EXISTS(
               SELECT 1
               FROM library_root_publication_namespaces AS proof
               LEFT JOIN library_change_root_state AS active ON active.root_id = proof.root_id
               WHERE active.root_id IS NULL OR active.is_active <> 1
                  OR active.generation <> proof.root_generation
             ) OR EXISTS(
               SELECT 1
               FROM library_scan_publication_namespace_bindings AS binding
               LEFT JOIN scan_runs AS scan ON scan.id = binding.scan_id
               LEFT JOIN library_change_root_state AS active
                 ON active.root_id = binding.root_id
               WHERE scan.id IS NULL OR scan.root_id <> binding.root_id
                  OR scan.root_generation_at_start <> binding.root_generation
                  OR scan.status NOT IN ('running', 'paused')
                  OR active.is_active <> 1 OR active.generation <> binding.root_generation
             ) OR EXISTS(
               SELECT 1 FROM pragma_foreign_key_check(
                 'library_root_publication_namespaces'
               )
             ) OR EXISTS(
               SELECT 1 FROM pragma_foreign_key_check(
                 'library_scan_publication_namespace_bindings'
               )
             )",
            [],
            |row| row.get::<_, bool>(0),
        )
        .map_err(database_error)?;
    let mut identities = connection
        .prepare(
            "SELECT identity_scheme, identity_value
             FROM library_root_publication_namespaces
             UNION ALL
             SELECT identity_scheme, identity_value
             FROM library_scan_publication_namespace_bindings",
        )
        .map_err(database_error)?;
    let identities_valid = identities
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(database_error)?
        .all(|row| {
            row.is_ok_and(|(scheme, value)| validate_windows_root_identity(&scheme, &value).is_ok())
        });
    if invalid_relations || !identities_valid {
        return Err(unverifiable_root_publication_namespace_contract());
    }
    Ok(())
}

fn validate_live_gap_recovery_contract(connection: &Connection) -> Result<(), ScanError> {
    validate_live_gap_recovery_contract_with_depth(connection, ContractValidationDepth::Full)
}

fn validate_live_gap_recovery_contract_with_depth(
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

fn validate_windows_root_identity(scheme: &str, value: &str) -> Result<(), ScanError> {
    let valid = scheme == "windows-file-id-128-v1"
        && value.len() == 49
        && value.as_bytes().get(16) == Some(&b':')
        && value.bytes().enumerate().all(|(index, byte)| {
            index == 16 || byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)
        });
    if valid {
        Ok(())
    } else {
        Err(unverifiable_root_publication_namespace_contract())
    }
}

fn unverifiable_root_publication_namespace_contract() -> ScanError {
    ScanError::new(
        "catalog_root_publication_namespace_unverifiable",
        "The catalog cannot prove the configured-root publication namespace contract",
    )
}

fn metadata_inventory_spool_schema_matches(
    connection: &Connection,
    expected_contract_version: i64,
) -> Result<bool, ScanError> {
    let contract_ddl = match expected_contract_version {
        1 => METADATA_INVENTORY_SPOOL_CONTRACT_TABLE_V27_DDL,
        2 => METADATA_INVENTORY_SPOOL_CONTRACT_TABLE_DDL,
        3 => METADATA_INVENTORY_SPOOL_CONTRACT_TABLE_V31_DDL,
        _ => return Ok(false),
    };
    let spool_ddl = if expected_contract_version == 1 {
        METADATA_INVENTORY_SPOOL_TABLE_V27_DDL
    } else {
        METADATA_INVENTORY_SPOOL_TABLE_DDL
    };
    let binding_guard_ddl = if expected_contract_version == 1 {
        METADATA_INVENTORY_SPOOL_BINDING_UPDATE_GUARD_V27_DDL
    } else {
        METADATA_INVENTORY_SPOOL_BINDING_UPDATE_GUARD_DDL
    };
    Ok(schema_object_sql_matches(
        connection,
        "table",
        "library_metadata_inventory_spool_contract",
        contract_ddl,
    )? && schema_object_sql_matches(
        connection,
        "table",
        "library_metadata_inventory_spools",
        spool_ddl,
    )? && schema_object_sql_matches(
        connection,
        "table",
        "library_metadata_inventory_spool_directories",
        METADATA_INVENTORY_SPOOL_DIRECTORY_TABLE_DDL,
    )? && schema_object_sql_matches(
        connection,
        "index",
        "library_metadata_inventory_spool_directories_state",
        METADATA_INVENTORY_SPOOL_DIRECTORY_STATE_INDEX_DDL,
    )? && schema_object_sql_matches(
        connection,
        "table",
        "library_metadata_inventory_spool_entries",
        if expected_contract_version == 3 {
            METADATA_INVENTORY_SPOOL_ENTRY_TABLE_V31_DDL
        } else {
            METADATA_INVENTORY_SPOOL_ENTRY_TABLE_DDL
        },
    )? && schema_object_sql_matches(
        connection,
        "index",
        "library_metadata_inventory_spool_entries_order",
        METADATA_INVENTORY_SPOOL_ENTRY_ORDER_INDEX_DDL,
    )? && schema_object_sql_matches(
        connection,
        "trigger",
        "library_metadata_inventory_spool_binding_update_guard",
        binding_guard_ddl,
    )? && schema_object_sql_matches(
        connection,
        "trigger",
        "library_metadata_inventory_spool_directory_complete_guard",
        METADATA_INVENTORY_SPOOL_DIRECTORY_COMPLETE_GUARD_DDL,
    )?)
}

fn validate_metadata_inventory_spool_contract_version(
    connection: &Connection,
    expected_schema_version: i64,
    expected_contract_version: i64,
) -> Result<(), ScanError> {
    validate_metadata_inventory_spool_contract_version_with_depth(
        connection,
        expected_schema_version,
        expected_contract_version,
        ContractValidationDepth::Full,
    )
}

fn validate_metadata_inventory_spool_contract_version_with_depth(
    connection: &Connection,
    expected_schema_version: i64,
    expected_contract_version: i64,
    depth: ContractValidationDepth,
) -> Result<(), ScanError> {
    let application_id: i64 = connection
        .query_row("PRAGMA application_id", [], |row| row.get(0))
        .map_err(database_error)?;
    let user_version: i64 = connection
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .map_err(database_error)?;
    let spool_columns = if expected_contract_version == 1 {
        vec![
            ("run_id", "TEXT", false, 1),
            ("authority_change_id", "INTEGER", true, 0),
            ("root_id", "TEXT", true, 0),
            ("root_generation", "INTEGER", true, 0),
            ("scope_kind", "TEXT", true, 0),
            ("scope_relative_path", "TEXT", true, 0),
            ("state", "TEXT", true, 0),
            ("created_unix_ms", "INTEGER", true, 0),
            ("updated_unix_ms", "INTEGER", true, 0),
        ]
    } else {
        vec![
            ("run_id", "TEXT", false, 1),
            ("authority_change_id", "INTEGER", true, 0),
            ("root_id", "TEXT", true, 0),
            ("root_generation", "INTEGER", true, 0),
            ("root_identity_scheme", "TEXT", true, 0),
            ("root_identity_value", "TEXT", true, 0),
            ("scope_kind", "TEXT", true, 0),
            ("scope_relative_path", "TEXT", true, 0),
            ("state", "TEXT", true, 0),
            ("created_unix_ms", "INTEGER", true, 0),
            ("updated_unix_ms", "INTEGER", true, 0),
        ]
    };
    let mut spool_entry_columns = vec![
        ("run_id", "TEXT", true, 1),
        ("directory_relative_path", "TEXT", false, 0),
        ("relative_path", "TEXT", true, 2),
        ("entry_kind", "TEXT", true, 0),
        ("file_size", "INTEGER", false, 0),
        ("modified_unix_ms", "INTEGER", true, 0),
        ("file_identity_scheme", "TEXT", false, 0),
        ("file_identity_value", "TEXT", false, 0),
        ("placeholder_state", "TEXT", true, 0),
        ("is_reparse_point", "INTEGER", true, 0),
        ("staged_unix_ms", "INTEGER", true, 0),
    ];
    if expected_contract_version == 3 {
        spool_entry_columns.push(("source_revision_token", "TEXT", false, 0));
    }
    let columns_match = table_columns_match(
        connection,
        "library_metadata_inventory_spool_contract",
        &[
            ("singleton", "INTEGER", false, 1),
            ("contract_version", "INTEGER", true, 0),
            ("complete", "INTEGER", true, 0),
        ],
    )? && table_columns_match(
        connection,
        "library_metadata_inventory_spools",
        &spool_columns,
    )? && table_columns_match(
        connection,
        "library_metadata_inventory_spool_directories",
        &[
            ("run_id", "TEXT", true, 1),
            ("ordinal", "INTEGER", true, 0),
            ("relative_directory", "TEXT", true, 2),
            ("state", "TEXT", true, 0),
            ("directory_identity_scheme", "TEXT", false, 0),
            ("directory_identity_value", "TEXT", false, 0),
            ("source_entry_count", "INTEGER", true, 0),
            ("created_unix_ms", "INTEGER", true, 0),
            ("updated_unix_ms", "INTEGER", true, 0),
        ],
    )? && table_columns_match(
        connection,
        "library_metadata_inventory_spool_entries",
        &spool_entry_columns,
    )?;
    let marker_complete = connection
        .query_row(
            "SELECT singleton = 1 AND contract_version = ?1 AND complete = 1
             FROM library_metadata_inventory_spool_contract",
            [expected_contract_version],
            |row| row.get::<_, bool>(0),
        )
        .optional()
        .map_err(database_error)?
        .unwrap_or(false);
    let schema_matches =
        metadata_inventory_spool_schema_matches(connection, expected_contract_version)?;
    if application_id != SQLITE_APPLICATION_ID
        || user_version != expected_schema_version
        || !columns_match
        || !marker_complete
        || !schema_matches
    {
        return Err(ScanError::new(
            "catalog_metadata_inventory_spool_contract_unverifiable",
            "The catalog cannot prove its bounded metadata inventory source spool",
        ));
    }
    if !depth.includes_rows() {
        return Ok(());
    }
    record_current_schema_row_audit();
    let invalid_relations = connection
        .query_row(
            "SELECT EXISTS(
               SELECT 1
               FROM library_metadata_inventory_spools AS spool
               LEFT JOIN library_metadata_inventory_runs AS run ON run.id = spool.run_id
               LEFT JOIN library_recovery_authorities AS authority
                 ON authority.change_id = spool.authority_change_id
               WHERE run.id IS NULL OR authority.change_id IS NULL
                  OR (?1 = 1 AND run.status <> 'running')
                  OR (?1 >= 2 AND run.status NOT IN ('running', 'comparing', 'completed'))
                  OR authority.retired_unix_ms IS NOT NULL
                  OR authority.run_id <> spool.run_id
                  OR authority.root_id <> spool.root_id
                  OR authority.root_generation <> spool.root_generation
                  OR run.root_id <> spool.root_id
                  OR run.root_generation <> spool.root_generation
                  OR run.scope_kind <> spool.scope_kind
                  OR run.scope_relative_path <> spool.scope_relative_path
             ) OR EXISTS(
               SELECT 1
               FROM library_metadata_inventory_spool_directories AS directory
               WHERE directory.ordinal <> (
                 SELECT COUNT(*)
                 FROM library_metadata_inventory_spool_directories AS earlier
                 WHERE earlier.run_id = directory.run_id
                   AND earlier.ordinal < directory.ordinal
               )
                  OR (directory.state = 'completed' AND (
                    directory.directory_identity_scheme IS NULL
                    OR directory.source_entry_count <> (
                      SELECT COUNT(*)
                      FROM library_metadata_inventory_spool_entries AS entry
                      WHERE entry.run_id = directory.run_id
                        AND entry.directory_relative_path = directory.relative_directory
                    )
                  ))
             ) OR EXISTS(
               SELECT 1 FROM library_metadata_inventory_spools AS spool
               WHERE (spool.state = 'ready' AND EXISTS(
                 SELECT 1 FROM library_metadata_inventory_spool_directories AS directory
                 WHERE directory.run_id = spool.run_id AND directory.state <> 'completed'
               )) OR (spool.state = 'enumerating' AND NOT EXISTS(
                 SELECT 1 FROM library_metadata_inventory_spool_directories AS directory
                 WHERE directory.run_id = spool.run_id
                   AND directory.state IN ('pending', 'enumerating')
               ))
             ) OR EXISTS(
               SELECT 1 FROM pragma_foreign_key_check('library_metadata_inventory_spools')
             ) OR EXISTS(
               SELECT 1 FROM pragma_foreign_key_check(
                 'library_metadata_inventory_spool_directories'
               )
             ) OR EXISTS(
               SELECT 1 FROM pragma_foreign_key_check(
                 'library_metadata_inventory_spool_entries'
               )
             )",
            [expected_contract_version],
            |row| row.get::<_, bool>(0),
        )
        .map_err(database_error)?;
    if invalid_relations {
        return Err(ScanError::new(
            "catalog_metadata_inventory_spool_contract_unverifiable",
            "The catalog cannot prove its bounded metadata inventory source spool",
        ));
    }
    Ok(())
}

fn unverifiable_recovery_authority_contract() -> ScanError {
    ScanError::new(
        "catalog_recovery_authority_contract_unverifiable",
        "The catalog cannot prove its bounded recovery authority",
    )
}

fn validate_terminal_media_evidence_contract(connection: &Connection) -> Result<(), ScanError> {
    validate_terminal_media_evidence_contract_with_depth(connection, ContractValidationDepth::Full)
}

fn validate_terminal_media_evidence_contract_with_depth(
    connection: &Connection,
    depth: ContractValidationDepth,
) -> Result<(), ScanError> {
    let mut evidence_columns = vec![
        ("root_id", "TEXT", true, 1),
        ("relative_path", "TEXT", true, 2),
        ("file_size", "INTEGER", true, 0),
        ("modified_unix_ms", "INTEGER", true, 0),
        ("file_identity_scheme", "TEXT", false, 0),
        ("file_identity_value", "TEXT", false, 0),
        ("inspection_engine_id", "TEXT", true, 0),
        ("inspection_engine_version", "INTEGER", true, 0),
        ("issue_code", "TEXT", true, 0),
        ("issue_message", "TEXT", true, 0),
        ("updated_unix_ms", "INTEGER", true, 0),
    ];
    if schema_version(connection)? >= 31 {
        evidence_columns.push(("source_revision_token", "TEXT", false, 0));
        evidence_columns.push(("source_generation", "INTEGER", false, 0));
    }
    let structure_matches = table_columns_match(
        connection,
        "library_terminal_media_evidence_contract",
        &[
            ("singleton", "INTEGER", false, 1),
            ("complete", "INTEGER", true, 0),
        ],
    )? && table_columns_match(
        connection,
        "library_terminal_media_evidence",
        &evidence_columns,
    )?;
    let marker_complete = connection
        .query_row(
            "SELECT singleton = 1 AND complete = 1
             FROM library_terminal_media_evidence_contract",
            [],
            |row| row.get::<_, bool>(0),
        )
        .optional()
        .map_err(database_error)?
        .unwrap_or(false);
    let foreign_key_matches = cascade_foreign_key_matches(
        connection,
        "library_terminal_media_evidence",
        "root_id",
        "library_roots",
        "id",
    )?;
    if !structure_matches || !marker_complete || !foreign_key_matches {
        return Err(ScanError::new(
            "catalog_terminal_media_evidence_contract_unverifiable",
            "The catalog cannot prove its terminal media evidence authority",
        ));
    }
    if !depth.includes_rows() {
        return Ok(());
    }
    record_current_schema_row_audit();
    let invalid_relations = connection
        .query_row(
            "SELECT EXISTS(
               SELECT 1 FROM pragma_foreign_key_check('library_terminal_media_evidence')
             )",
            [],
            |row| row.get::<_, bool>(0),
        )
        .map_err(database_error)?;
    if invalid_relations {
        return Err(ScanError::new(
            "catalog_terminal_media_evidence_contract_unverifiable",
            "The catalog cannot prove its terminal media evidence authority",
        ));
    }
    Ok(())
}

fn schema_version(connection: &Connection) -> Result<i64, ScanError> {
    connection
        .query_row("SELECT version FROM schema_info LIMIT 1", [], |row| {
            row.get(0)
        })
        .map_err(database_error)
}

fn validate_metadata_inventory_contract(connection: &Connection) -> Result<(), ScanError> {
    validate_metadata_inventory_contract_with_depth(connection, ContractValidationDepth::Full)
}

fn validate_metadata_inventory_contract_with_depth(
    connection: &Connection,
    depth: ContractValidationDepth,
) -> Result<(), ScanError> {
    let mut entry_columns = vec![
        ("run_id", "TEXT", true, 1),
        ("relative_path", "TEXT", true, 2),
        ("entry_kind", "TEXT", true, 0),
        ("file_size", "INTEGER", false, 0),
        ("modified_unix_ms", "INTEGER", true, 0),
        ("file_identity_scheme", "TEXT", false, 0),
        ("file_identity_value", "TEXT", false, 0),
        ("placeholder_state", "TEXT", true, 0),
        ("is_reparse_point", "INTEGER", true, 0),
        ("staged_page_index", "INTEGER", true, 0),
        ("comparison_status", "TEXT", true, 0),
        ("candidate_previous_relative_path", "TEXT", false, 0),
        ("staged_unix_ms", "INTEGER", true, 0),
    ];
    if schema_version(connection)? >= 31 {
        entry_columns.push(("source_revision_token", "TEXT", false, 0));
    }
    let structure_matches = table_columns_match(
        connection,
        "library_metadata_inventory_contract",
        &[
            ("singleton", "INTEGER", false, 1),
            ("complete", "INTEGER", true, 0),
        ],
    )? && table_columns_match(
        connection,
        "library_metadata_inventory_runs",
        &[
            ("id", "TEXT", true, 1),
            ("root_id", "TEXT", true, 0),
            ("root_generation", "INTEGER", true, 0),
            ("epoch", "INTEGER", true, 0),
            ("scope_kind", "TEXT", true, 0),
            ("scope_relative_path", "TEXT", true, 0),
            ("status", "TEXT", true, 0),
            ("next_page_index", "INTEGER", true, 0),
            ("enumeration_cursor", "TEXT", false, 0),
            ("comparison_cursor", "TEXT", false, 0),
            ("absence_cursor", "TEXT", false, 0),
            ("staged_entry_count", "INTEGER", true, 0),
            ("candidate_count", "INTEGER", true, 0),
            ("enumeration_complete", "INTEGER", true, 0),
            ("absence_authority", "INTEGER", true, 0),
            ("started_unix_ms", "INTEGER", true, 0),
            ("updated_unix_ms", "INTEGER", true, 0),
            ("completed_unix_ms", "INTEGER", false, 0),
            ("last_issue_code", "TEXT", false, 0),
            ("last_issue_message", "TEXT", false, 0),
        ],
    )? && table_columns_match(
        connection,
        "library_metadata_inventory_entries",
        &entry_columns,
    )?;
    let indexes_match = named_index_matches(
        connection,
        "library_metadata_inventory_runs",
        "library_metadata_inventory_runs_cleanup",
        &["status", "updated_unix_ms", "id"],
    )? && named_index_matches(
        connection,
        "library_metadata_inventory_entries",
        "library_metadata_inventory_entries_compare",
        &["run_id", "comparison_status", "relative_path"],
    )? && named_index_matches(
        connection,
        "library_metadata_inventory_entries",
        "library_metadata_inventory_entries_identity",
        &[
            "run_id",
            "file_identity_scheme",
            "file_identity_value",
            "relative_path",
        ],
    )? && named_index_matches(
        connection,
        "library_metadata_inventory_entries",
        "library_metadata_inventory_entries_previous",
        &[
            "run_id",
            "candidate_previous_relative_path",
            "relative_path",
        ],
    )?;
    let active_index_matches = connection
        .query_row(
            "SELECT EXISTS(
               SELECT 1 FROM pragma_index_list('library_metadata_inventory_runs')
               WHERE name = 'library_metadata_inventory_runs_one_active_root'
                 AND \"unique\" = 1 AND partial = 1
             ) AND (SELECT group_concat(name, ',') FROM (
               SELECT name FROM pragma_index_info(
                 'library_metadata_inventory_runs_one_active_root'
               ) ORDER BY seqno
             )) = 'root_id'",
            [],
            |row| row.get::<_, bool>(0),
        )
        .map_err(database_error)?;
    let active_index_predicate_matches = schema_object_sql_matches(
        connection,
        "index",
        "library_metadata_inventory_runs_one_active_root",
        "CREATE UNIQUE INDEX library_metadata_inventory_runs_one_active_root
           ON library_metadata_inventory_runs(root_id)
           WHERE status IN ('running', 'comparing')",
    )?;
    let queue_accepts_inventory_origin = connection
        .query_row(
            "SELECT instr(
               replace(replace(replace(replace(lower(sql), ' ', ''), char(10), ''),
                 char(13), ''), char(9), ''),
               '''metadata_inventory'''
             ) > 0
             FROM sqlite_master
             WHERE type = 'table' AND name = 'library_change_queue'",
            [],
            |row| row.get::<_, bool>(0),
        )
        .optional()
        .map_err(database_error)?
        .unwrap_or(false);
    let foreign_keys_match = cascade_foreign_key_matches(
        connection,
        "library_metadata_inventory_runs",
        "root_id",
        "library_roots",
        "id",
    )? && cascade_foreign_key_matches(
        connection,
        "library_metadata_inventory_entries",
        "run_id",
        "library_metadata_inventory_runs",
        "id",
    )?;
    let marker_complete = connection
        .query_row(
            "SELECT singleton = 1 AND complete = 1
             FROM library_metadata_inventory_contract",
            [],
            |row| row.get::<_, bool>(0),
        )
        .optional()
        .map_err(database_error)?
        .unwrap_or(false);
    if !structure_matches
        || !indexes_match
        || !active_index_matches
        || !active_index_predicate_matches
        || !queue_accepts_inventory_origin
        || !foreign_keys_match
        || !marker_complete
    {
        return Err(unverifiable_metadata_inventory_contract());
    }
    if !depth.includes_rows() {
        return Ok(());
    }
    record_current_schema_row_audit();
    let invalid_relations = connection
        .query_row(
            "SELECT
               EXISTS(
                 SELECT 1
                 FROM library_metadata_inventory_runs AS runs
                 LEFT JOIN library_roots AS roots ON roots.id = runs.root_id
                 LEFT JOIN library_change_root_state AS state ON state.root_id = runs.root_id
                 WHERE roots.id IS NULL OR state.root_id IS NULL
                    OR (
                      runs.status IN ('running', 'comparing')
                      AND (
                        state.is_active <> 1 OR state.generation <> runs.root_generation
                        OR NOT EXISTS(
                          SELECT 1 FROM scan_runs AS scans
                          WHERE scans.id = roots.active_scan_id
                            AND scans.status = 'completed'
                        )
                      )
                    )
               )
               OR EXISTS(
                 SELECT 1
                 FROM library_metadata_inventory_runs AS runs
                 WHERE (
                   runs.status IN ('running', 'comparing')
                   AND runs.staged_entry_count <> (
                     SELECT COUNT(*) FROM library_metadata_inventory_entries AS entries
                     WHERE entries.run_id = runs.id
                   )
                 )
                    OR (
                      runs.status IN ('completed', 'failed', 'cancelled', 'superseded')
                      AND runs.staged_entry_count < (
                        SELECT COUNT(*) FROM library_metadata_inventory_entries AS entries
                        WHERE entries.run_id = runs.id
                      )
                    )
                    OR (runs.scope_kind = 'root' AND runs.scope_relative_path <> '')
                    OR (runs.scope_kind = 'subtree' AND runs.scope_relative_path = '')
               )
               OR EXISTS(
                 SELECT 1
                 FROM library_metadata_inventory_entries AS entries
                 JOIN library_metadata_inventory_runs AS runs ON runs.id = entries.run_id
                 WHERE entries.staged_page_index >= runs.next_page_index
                    OR (
                      runs.scope_kind = 'subtree'
                      AND entries.relative_path <> runs.scope_relative_path
                      AND substr(entries.relative_path, 1, length(runs.scope_relative_path) + 1)
                        <> runs.scope_relative_path || '/'
                    )
               )
               OR EXISTS(SELECT 1 FROM pragma_foreign_key_check(
                 'library_metadata_inventory_runs'
               ))
               OR EXISTS(SELECT 1 FROM pragma_foreign_key_check(
                 'library_metadata_inventory_entries'
               ))",
            [],
            |row| row.get::<_, bool>(0),
        )
        .map_err(database_error)?;
    if invalid_relations {
        return Err(unverifiable_metadata_inventory_contract());
    }
    Ok(())
}

fn validate_persistent_journal_contract(connection: &Connection) -> Result<(), ScanError> {
    validate_persistent_journal_contract_with_depth(connection, ContractValidationDepth::Full)
}

fn validate_persistent_journal_contract_with_depth(
    connection: &Connection,
    depth: ContractValidationDepth,
) -> Result<(), ScanError> {
    let columns_match = table_columns_match(
        connection,
        "library_persistent_journal_contract",
        &[
            ("singleton", "INTEGER", false, 1),
            ("contract_version", "INTEGER", true, 0),
            ("complete", "INTEGER", true, 0),
        ],
    )? && table_columns_match(
        connection,
        "library_persistent_journal_root_state",
        &[
            ("root_id", "TEXT", true, 1),
            ("root_generation", "INTEGER", true, 2),
            ("protocol_version", "INTEGER", true, 0),
            ("contract_version", "INTEGER", true, 0),
            ("capability_state", "TEXT", true, 0),
            ("continuity_state", "TEXT", true, 0),
            ("last_failure_code", "TEXT", false, 0),
            ("last_failure_message", "TEXT", false, 0),
            ("updated_unix_ms", "INTEGER", true, 0),
        ],
    )? && table_columns_match(
        connection,
        "library_persistent_journal_checkpoints",
        &[
            ("root_id", "TEXT", false, 1),
            ("root_generation", "INTEGER", true, 0),
            ("volume_guid", "TEXT", true, 0),
            ("volume_serial", "TEXT", true, 0),
            ("root_reference_version", "INTEGER", true, 0),
            ("root_file_reference", "BLOB", true, 0),
            ("journal_id", "TEXT", true, 0),
            ("next_unread_usn", "TEXT", true, 0),
            ("captured_exclusive_end", "TEXT", true, 0),
            ("covered_catalog_revision", "INTEGER", true, 0),
            ("protocol_version", "INTEGER", true, 0),
            ("contract_version", "INTEGER", true, 0),
            ("continuity_state", "TEXT", true, 0),
            ("last_failure_code", "TEXT", false, 0),
            ("last_failure_message", "TEXT", false, 0),
            ("updated_unix_ms", "INTEGER", true, 0),
        ],
    )? && table_columns_match(
        connection,
        "library_persistent_journal_source_ranges",
        &[
            ("id", "TEXT", false, 1),
            ("root_id", "TEXT", true, 0),
            ("root_generation", "INTEGER", true, 0),
            ("volume_guid", "TEXT", true, 0),
            ("volume_serial", "TEXT", true, 0),
            ("journal_id", "TEXT", true, 0),
            ("requested_start_usn", "TEXT", true, 0),
            ("requested_end_usn", "TEXT", true, 0),
            ("covered_until_usn", "TEXT", true, 0),
            ("is_complete", "INTEGER", true, 0),
            ("protocol_version", "INTEGER", true, 0),
            ("contract_version", "INTEGER", true, 0),
            ("status", "TEXT", true, 0),
            ("enrolled_unix_ms", "INTEGER", true, 0),
            ("checkpointed_unix_ms", "INTEGER", false, 0),
            ("canonical_payload", "BLOB", true, 0),
        ],
    )? && table_columns_match(
        connection,
        "library_persistent_journal_queue_lineage",
        &[
            ("source_range_id", "TEXT", true, 1),
            ("change_id", "INTEGER", true, 2),
            ("enrolled_unix_ms", "INTEGER", true, 0),
        ],
    )? && table_columns_match(
        connection,
        "library_persistent_journal_cross_root_lineage",
        &[
            ("id", "TEXT", false, 1),
            ("volume_guid", "TEXT", true, 0),
            ("volume_serial", "TEXT", true, 0),
            ("file_reference_version", "INTEGER", true, 0),
            ("file_reference", "BLOB", true, 0),
            ("previous_root_id", "TEXT", true, 0),
            ("previous_root_generation", "INTEGER", true, 0),
            ("previous_relative_path", "TEXT", true, 0),
            ("current_root_id", "TEXT", true, 0),
            ("current_root_generation", "INTEGER", true, 0),
            ("current_relative_path", "TEXT", true, 0),
            ("status", "TEXT", true, 0),
            ("created_unix_ms", "INTEGER", true, 0),
            ("updated_unix_ms", "INTEGER", true, 0),
            ("journal_id", "TEXT", false, 0),
            ("old_usn", "TEXT", false, 0),
            ("new_usn", "TEXT", false, 0),
            ("previous_carry_id", "TEXT", false, 0),
        ],
    )? && table_columns_match(
        connection,
        "library_persistent_journal_cross_root_ranges",
        &[
            ("lineage_id", "TEXT", true, 1),
            ("source_range_id", "TEXT", true, 2),
            ("participant_role", "TEXT", true, 0),
            ("enrolled_unix_ms", "INTEGER", true, 0),
        ],
    )? && table_columns_match(
        connection,
        "library_persistent_journal_range_lifecycle",
        &[
            ("source_range_id", "TEXT", false, 1),
            ("lifecycle_state", "TEXT", true, 0),
            ("completed_unix_ms", "INTEGER", false, 0),
            ("updated_unix_ms", "INTEGER", true, 0),
        ],
    )? && table_columns_match(
        connection,
        "library_persistent_journal_pending_renames",
        &[
            ("carry_id", "TEXT", false, 1),
            ("source_range_id", "TEXT", true, 0),
            ("volume_guid", "TEXT", true, 0),
            ("volume_serial", "TEXT", true, 0),
            ("journal_id", "TEXT", true, 0),
            ("file_reference_version", "INTEGER", true, 0),
            ("file_reference", "BLOB", true, 0),
            ("old_usn", "TEXT", true, 0),
            ("previous_root_id", "TEXT", true, 0),
            ("previous_root_generation", "INTEGER", true, 0),
            ("previous_relative_path", "TEXT", true, 0),
            ("is_directory", "INTEGER", true, 0),
            ("enrolled_unix_ms", "INTEGER", true, 0),
        ],
    )?;
    let marker_complete = connection
        .query_row(
            "SELECT singleton = 1 AND contract_version = ?1 AND complete = 1
             FROM library_persistent_journal_contract",
            [i64::from(PERSISTENT_JOURNAL_CONTRACT_VERSION)],
            |row| row.get::<_, bool>(0),
        )
        .optional()
        .map_err(database_error)?
        .unwrap_or(false);
    let indexes_match = schema_object_sql_matches(
        connection,
        "index",
        "library_change_root_state_generation_identity",
        "CREATE UNIQUE INDEX library_change_root_state_generation_identity
           ON library_change_root_state(root_id, generation)",
    )? && named_index_matches(
        connection,
        "library_persistent_journal_root_state",
        "library_persistent_journal_root_state_continuity",
        &[
            "continuity_state",
            "capability_state",
            "updated_unix_ms",
            "root_id",
        ],
    )? && named_index_matches(
        connection,
        "library_persistent_journal_checkpoints",
        "library_persistent_journal_checkpoints_volume",
        &["volume_guid", "journal_id", "continuity_state", "root_id"],
    )? && named_index_matches(
        connection,
        "library_persistent_journal_source_ranges",
        "library_persistent_journal_source_ranges_root",
        &[
            "root_id",
            "root_generation",
            "status",
            "enrolled_unix_ms",
            "id",
        ],
    )? && named_index_matches(
        connection,
        "library_persistent_journal_source_ranges",
        "library_persistent_journal_source_ranges_volume",
        &["volume_guid", "journal_id", "requested_start_usn", "id"],
    )? && named_index_matches(
        connection,
        "library_persistent_journal_queue_lineage",
        "library_persistent_journal_queue_lineage_change",
        &["change_id", "source_range_id"],
    )? && named_index_matches(
        connection,
        "library_persistent_journal_cross_root_lineage",
        "library_persistent_journal_cross_root_lineage_previous",
        &[
            "previous_root_id",
            "previous_root_generation",
            "status",
            "id",
        ],
    )? && named_index_matches(
        connection,
        "library_persistent_journal_cross_root_lineage",
        "library_persistent_journal_cross_root_lineage_current",
        &["current_root_id", "current_root_generation", "status", "id"],
    )? && named_index_matches(
        connection,
        "library_persistent_journal_cross_root_ranges",
        "library_persistent_journal_cross_root_ranges_source",
        &["source_range_id", "lineage_id"],
    )? && named_index_matches(
        connection,
        "library_persistent_journal_range_lifecycle",
        "library_persistent_journal_range_lifecycle_state",
        &["lifecycle_state", "updated_unix_ms", "source_range_id"],
    )? && named_index_matches(
        connection,
        "library_persistent_journal_pending_renames",
        "library_persistent_journal_pending_renames_volume",
        &["volume_guid", "journal_id", "old_usn", "carry_id"],
    )? && named_index_matches(
        connection,
        "library_persistent_journal_pending_renames",
        "library_persistent_journal_pending_renames_source",
        &["source_range_id", "carry_id"],
    )?;
    let checks_and_foreign_keys_match = persistent_journal_schema_sql_matches(connection)?;
    if !columns_match || !marker_complete || !indexes_match || !checks_and_foreign_keys_match {
        return Err(unverifiable_persistent_journal_contract());
    }
    if depth.includes_rows() {
        record_current_schema_row_audit();
        validate_persistent_journal_rows(connection)
    } else {
        Ok(())
    }
}

fn persistent_journal_schema_sql_matches(connection: &Connection) -> Result<bool, ScanError> {
    for (name, expected) in PERSISTENT_JOURNAL_CANONICAL_TABLE_DDL {
        let Some(sql) = connection
            .query_row(
                "SELECT sql FROM sqlite_master WHERE type = 'table' AND name = ?1",
                [name],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(database_error)?
        else {
            return Ok(false);
        };
        if normalize_schema_sql(&sql) != normalize_schema_sql(expected) {
            return Ok(false);
        }
    }
    for (name, expected) in PERSISTENT_JOURNAL_CANONICAL_TRIGGER_DDL {
        let Some(sql) = connection
            .query_row(
                "SELECT sql FROM sqlite_master WHERE type = 'trigger' AND name = ?1",
                [name],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(database_error)?
        else {
            return Ok(false);
        };
        if normalize_schema_sql(&sql) != normalize_schema_sql(expected) {
            return Ok(false);
        }
    }
    Ok(true)
}

const PERSISTENT_JOURNAL_CANONICAL_TABLE_DDL: &[(&str, &str)] = &[
    (
        "library_persistent_journal_contract",
        r#"CREATE TABLE library_persistent_journal_contract (
          singleton INTEGER PRIMARY KEY CHECK(singleton = 1),
          contract_version INTEGER NOT NULL CHECK(contract_version = 1),
          complete INTEGER NOT NULL CHECK(complete = 1)
        )"#,
    ),
    (
        "library_persistent_journal_root_state",
        r#"CREATE TABLE library_persistent_journal_root_state (
          root_id TEXT NOT NULL,
          root_generation INTEGER NOT NULL CHECK(root_generation > 0),
          protocol_version INTEGER NOT NULL CHECK(protocol_version BETWEEN 0 AND 65535),
          contract_version INTEGER NOT NULL CHECK(contract_version = 1),
          capability_state TEXT NOT NULL CHECK(capability_state IN (
            'unknown', 'supported', 'live_only'
          )),
          continuity_state TEXT NOT NULL CHECK(continuity_state IN (
            'baseline_required', 'catching_up', 'current', 'recovery_required',
            'live_only', 'unavailable'
          )),
          last_failure_code TEXT,
          last_failure_message TEXT,
          updated_unix_ms INTEGER NOT NULL CHECK(updated_unix_ms >= 0),
          CHECK(
            (last_failure_code IS NULL AND last_failure_message IS NULL)
            OR
            (last_failure_code IS NOT NULL AND last_failure_message IS NOT NULL)
          ),
          CHECK(
            (capability_state = 'unknown' AND protocol_version = 0)
            OR
            (capability_state <> 'unknown' AND protocol_version > 0)
          ),
          CHECK(capability_state <> 'supported' OR last_failure_code IS NULL),
          CHECK(
            continuity_state <> 'current'
            OR (capability_state = 'supported' AND last_failure_code IS NULL)
          ),
          PRIMARY KEY(root_id, root_generation),
          FOREIGN KEY(root_id) REFERENCES library_roots(id) ON DELETE CASCADE
        )"#,
    ),
    (
        "library_persistent_journal_checkpoints",
        r#"CREATE TABLE library_persistent_journal_checkpoints (
          root_id TEXT PRIMARY KEY,
          root_generation INTEGER NOT NULL CHECK(root_generation > 0),
          volume_guid TEXT NOT NULL CHECK(length(volume_guid) BETWEEN 1 AND 512),
          volume_serial TEXT NOT NULL CHECK(length(volume_serial) BETWEEN 1 AND 20),
          root_reference_version INTEGER NOT NULL CHECK(root_reference_version IN (2, 3)),
          root_file_reference BLOB NOT NULL,
          journal_id TEXT NOT NULL CHECK(length(journal_id) BETWEEN 1 AND 20),
          next_unread_usn TEXT NOT NULL CHECK(length(next_unread_usn) BETWEEN 1 AND 19),
          captured_exclusive_end TEXT NOT NULL
            CHECK(length(captured_exclusive_end) BETWEEN 1 AND 19),
          covered_catalog_revision INTEGER NOT NULL CHECK(covered_catalog_revision >= 0),
          protocol_version INTEGER NOT NULL CHECK(protocol_version BETWEEN 1 AND 65535),
          contract_version INTEGER NOT NULL CHECK(contract_version = 1),
          continuity_state TEXT NOT NULL CHECK(continuity_state IN (
            'catching_up', 'current', 'recovery_required', 'live_only', 'unavailable'
          )),
          last_failure_code TEXT,
          last_failure_message TEXT,
          updated_unix_ms INTEGER NOT NULL CHECK(updated_unix_ms >= 0),
          CHECK(
            (root_reference_version = 2 AND length(root_file_reference) = 8)
            OR
            (root_reference_version = 3 AND length(root_file_reference) = 16)
          ),
          CHECK(journal_id <> '0' AND journal_id NOT GLOB '*[^0-9]*'),
          CHECK(next_unread_usn NOT GLOB '*[^0-9]*'),
          CHECK(captured_exclusive_end NOT GLOB '*[^0-9]*'),
          CHECK(CAST(next_unread_usn AS INTEGER) <= CAST(captured_exclusive_end AS INTEGER)),
          CHECK(
            (last_failure_code IS NULL AND last_failure_message IS NULL)
            OR
            (last_failure_code IS NOT NULL AND last_failure_message IS NOT NULL)
          ),
          CHECK(
            continuity_state <> 'current'
            OR (next_unread_usn = captured_exclusive_end AND last_failure_code IS NULL)
          ),
          FOREIGN KEY(root_id, root_generation)
            REFERENCES library_persistent_journal_root_state(root_id, root_generation)
            ON DELETE CASCADE
        )"#,
    ),
    (
        "library_persistent_journal_source_ranges",
        r#"CREATE TABLE library_persistent_journal_source_ranges (
          id TEXT PRIMARY KEY CHECK(length(id) BETWEEN 1 AND 512),
          root_id TEXT NOT NULL,
          root_generation INTEGER NOT NULL CHECK(root_generation > 0),
          volume_guid TEXT NOT NULL CHECK(length(volume_guid) BETWEEN 1 AND 512),
          volume_serial TEXT NOT NULL CHECK(length(volume_serial) BETWEEN 1 AND 20),
          journal_id TEXT NOT NULL CHECK(length(journal_id) BETWEEN 1 AND 20),
          requested_start_usn TEXT NOT NULL CHECK(length(requested_start_usn) BETWEEN 1 AND 19),
          requested_end_usn TEXT NOT NULL CHECK(length(requested_end_usn) BETWEEN 1 AND 19),
          covered_until_usn TEXT NOT NULL CHECK(length(covered_until_usn) BETWEEN 1 AND 19),
          is_complete INTEGER NOT NULL CHECK(is_complete IN (0, 1)),
          protocol_version INTEGER NOT NULL CHECK(protocol_version BETWEEN 1 AND 65535),
          contract_version INTEGER NOT NULL CHECK(contract_version = 1),
          status TEXT NOT NULL CHECK(status IN ('enrolled', 'checkpointed', 'superseded')),
          enrolled_unix_ms INTEGER NOT NULL CHECK(enrolled_unix_ms >= 0),
          checkpointed_unix_ms INTEGER CHECK(checkpointed_unix_ms >= 0),
          canonical_payload BLOB NOT NULL DEFAULT X'',
          CHECK(journal_id <> '0' AND journal_id NOT GLOB '*[^0-9]*'),
          CHECK(requested_start_usn NOT GLOB '*[^0-9]*'),
          CHECK(requested_end_usn NOT GLOB '*[^0-9]*'),
          CHECK(covered_until_usn NOT GLOB '*[^0-9]*'),
          CHECK(
            CAST(requested_start_usn AS INTEGER) < CAST(requested_end_usn AS INTEGER)
            AND CAST(requested_start_usn AS INTEGER) < CAST(covered_until_usn AS INTEGER)
            AND CAST(covered_until_usn AS INTEGER) <= CAST(requested_end_usn AS INTEGER)
          ),
          CHECK(
            is_complete = (
              CAST(covered_until_usn AS INTEGER) = CAST(requested_end_usn AS INTEGER)
            )
          ),
          CHECK(
            (status = 'checkpointed' AND checkpointed_unix_ms IS NOT NULL)
            OR
            (status <> 'checkpointed' AND checkpointed_unix_ms IS NULL)
          ),
          FOREIGN KEY(root_id, root_generation)
            REFERENCES library_persistent_journal_root_state(root_id, root_generation)
            ON DELETE CASCADE
        )"#,
    ),
    (
        "library_persistent_journal_queue_lineage",
        r#"CREATE TABLE library_persistent_journal_queue_lineage (
          source_range_id TEXT NOT NULL,
          change_id INTEGER NOT NULL,
          enrolled_unix_ms INTEGER NOT NULL CHECK(enrolled_unix_ms >= 0),
          PRIMARY KEY(source_range_id, change_id),
          FOREIGN KEY(source_range_id)
            REFERENCES library_persistent_journal_source_ranges(id) ON DELETE CASCADE,
          FOREIGN KEY(change_id) REFERENCES library_change_queue(id) ON DELETE CASCADE
        )"#,
    ),
    (
        "library_persistent_journal_cross_root_lineage",
        r#"CREATE TABLE library_persistent_journal_cross_root_lineage (
          id TEXT PRIMARY KEY CHECK(length(id) BETWEEN 1 AND 512),
          volume_guid TEXT NOT NULL CHECK(length(volume_guid) BETWEEN 1 AND 512),
          volume_serial TEXT NOT NULL CHECK(length(volume_serial) BETWEEN 1 AND 20),
          file_reference_version INTEGER NOT NULL CHECK(file_reference_version IN (2, 3)),
          file_reference BLOB NOT NULL,
          previous_root_id TEXT NOT NULL,
          previous_root_generation INTEGER NOT NULL CHECK(previous_root_generation > 0),
          previous_relative_path TEXT NOT NULL,
          current_root_id TEXT NOT NULL,
          current_root_generation INTEGER NOT NULL CHECK(current_root_generation > 0),
          current_relative_path TEXT NOT NULL,
          status TEXT NOT NULL CHECK(status IN ('pending', 'completed', 'superseded')),
          created_unix_ms INTEGER NOT NULL CHECK(created_unix_ms >= 0),
          updated_unix_ms INTEGER NOT NULL CHECK(updated_unix_ms >= 0),
          journal_id TEXT,
          old_usn TEXT,
          new_usn TEXT,
          previous_carry_id TEXT,
          CHECK(previous_root_id <> current_root_id),
          CHECK(length(previous_relative_path) BETWEEN 1 AND 32767),
          CHECK(length(current_relative_path) BETWEEN 1 AND 32767),
          CHECK(instr(previous_relative_path, char(92)) = 0),
          CHECK(instr(current_relative_path, char(92)) = 0),
          CHECK(
            (file_reference_version = 2 AND length(file_reference) = 8)
            OR
            (file_reference_version = 3 AND length(file_reference) = 16)
          ),
          FOREIGN KEY(previous_root_id, previous_root_generation)
            REFERENCES library_persistent_journal_root_state(root_id, root_generation),
          FOREIGN KEY(current_root_id, current_root_generation)
            REFERENCES library_persistent_journal_root_state(root_id, root_generation)
        )"#,
    ),
    (
        "library_persistent_journal_cross_root_ranges",
        r#"CREATE TABLE library_persistent_journal_cross_root_ranges (
          lineage_id TEXT NOT NULL,
          source_range_id TEXT NOT NULL,
          participant_role TEXT NOT NULL CHECK(participant_role IN ('previous', 'current')),
          enrolled_unix_ms INTEGER NOT NULL CHECK(enrolled_unix_ms >= 0),
          PRIMARY KEY(lineage_id, source_range_id),
          FOREIGN KEY(lineage_id)
            REFERENCES library_persistent_journal_cross_root_lineage(id) ON DELETE CASCADE,
          FOREIGN KEY(source_range_id)
            REFERENCES library_persistent_journal_source_ranges(id) ON DELETE CASCADE
        )"#,
    ),
    (
        "library_persistent_journal_range_lifecycle",
        r#"CREATE TABLE library_persistent_journal_range_lifecycle (
          source_range_id TEXT PRIMARY KEY,
          lifecycle_state TEXT NOT NULL
            CHECK(lifecycle_state IN ('pending', 'completed', 'superseded')),
          completed_unix_ms INTEGER CHECK(completed_unix_ms >= 0),
          updated_unix_ms INTEGER NOT NULL CHECK(updated_unix_ms >= 0),
          CHECK(
            (lifecycle_state = 'completed' AND completed_unix_ms IS NOT NULL)
            OR
            (lifecycle_state <> 'completed' AND completed_unix_ms IS NULL)
          ),
          FOREIGN KEY(source_range_id)
            REFERENCES library_persistent_journal_source_ranges(id) ON DELETE CASCADE
        )"#,
    ),
    (
        "library_persistent_journal_pending_renames",
        r#"CREATE TABLE library_persistent_journal_pending_renames (
          carry_id TEXT PRIMARY KEY CHECK(length(carry_id) BETWEEN 1 AND 512),
          source_range_id TEXT NOT NULL,
          volume_guid TEXT NOT NULL CHECK(length(volume_guid) BETWEEN 1 AND 512),
          volume_serial TEXT NOT NULL CHECK(length(volume_serial) BETWEEN 1 AND 20),
          journal_id TEXT NOT NULL CHECK(length(journal_id) BETWEEN 1 AND 20),
          file_reference_version INTEGER NOT NULL CHECK(file_reference_version IN (2, 3)),
          file_reference BLOB NOT NULL,
          old_usn TEXT NOT NULL CHECK(length(old_usn) BETWEEN 1 AND 19),
          previous_root_id TEXT NOT NULL,
          previous_root_generation INTEGER NOT NULL CHECK(previous_root_generation > 0),
          previous_relative_path TEXT NOT NULL,
          is_directory INTEGER NOT NULL CHECK(is_directory IN (0, 1)),
          enrolled_unix_ms INTEGER NOT NULL CHECK(enrolled_unix_ms >= 0),
          CHECK(journal_id <> '0' AND journal_id NOT GLOB '*[^0-9]*'),
          CHECK(old_usn NOT GLOB '*[^0-9]*'),
          CHECK(length(previous_relative_path) BETWEEN 1 AND 32767),
          CHECK(instr(previous_relative_path, char(92)) = 0),
          CHECK(
            (file_reference_version = 2 AND length(file_reference) = 8)
            OR
            (file_reference_version = 3 AND length(file_reference) = 16)
          ),
          FOREIGN KEY(source_range_id)
            REFERENCES library_persistent_journal_source_ranges(id) ON DELETE CASCADE,
          FOREIGN KEY(previous_root_id, previous_root_generation)
            REFERENCES library_persistent_journal_root_state(root_id, root_generation)
        )"#,
    ),
];

const PERSISTENT_JOURNAL_CANONICAL_TRIGGER_DDL: &[(&str, &str)] = &[
    (
        "library_persistent_journal_source_range_id_insert",
        r#"CREATE TRIGGER library_persistent_journal_source_range_id_insert
          BEFORE INSERT ON library_persistent_journal_source_ranges
          WHEN NEW.id IS NULL OR typeof(NEW.id) <> 'text'
            OR length(NEW.id) <> 64 OR NEW.id GLOB '*[^0-9a-f]*'
          BEGIN
            SELECT RAISE(ABORT, 'invalid persistent journal source range id');
          END"#,
    ),
    (
        "library_persistent_journal_source_range_id_update",
        r#"CREATE TRIGGER library_persistent_journal_source_range_id_update
          BEFORE UPDATE OF id ON library_persistent_journal_source_ranges
          WHEN NEW.id IS NULL OR typeof(NEW.id) <> 'text'
            OR length(NEW.id) <> 64 OR NEW.id GLOB '*[^0-9a-f]*'
          BEGIN
            SELECT RAISE(ABORT, 'invalid persistent journal source range id');
          END"#,
    ),
];

const PERSISTENT_JOURNAL_LEGACY_V24_TRIGGER_DDL: &[(&str, &str)] = &[
    (
        "library_persistent_journal_source_range_id_insert",
        r#"CREATE TRIGGER library_persistent_journal_source_range_id_insert
          BEFORE INSERT ON library_persistent_journal_source_ranges
          WHEN length(NEW.id) <> 64 OR NEW.id GLOB '*[^0-9a-f]*'
          BEGIN
            SELECT RAISE(ABORT, 'invalid persistent journal source range id');
          END"#,
    ),
    (
        "library_persistent_journal_source_range_id_update",
        r#"CREATE TRIGGER library_persistent_journal_source_range_id_update
          BEFORE UPDATE OF id ON library_persistent_journal_source_ranges
          WHEN length(NEW.id) <> 64 OR NEW.id GLOB '*[^0-9a-f]*'
          BEGIN
            SELECT RAISE(ABORT, 'invalid persistent journal source range id');
          END"#,
    ),
];

fn validate_persistent_journal_rows(connection: &Connection) -> Result<(), ScanError> {
    let invalid_relations = connection
        .query_row(
            "SELECT
               EXISTS(
                 SELECT 1 FROM library_roots AS roots
                 JOIN library_change_root_state AS generations ON generations.root_id = roots.id
                 LEFT JOIN library_persistent_journal_root_state AS journal
                   ON journal.root_id = roots.id
                  AND journal.root_generation = generations.generation
                 WHERE generations.is_active = 1 AND journal.root_id IS NULL
               )
               OR EXISTS(
                 SELECT 1 FROM library_persistent_journal_root_state AS journal
                 JOIN library_change_root_state AS generations
                   ON generations.root_id = journal.root_id
                 WHERE journal.root_generation > generations.generation
               )
               OR EXISTS(
                 SELECT 1 FROM library_persistent_journal_root_state AS journal
                 LEFT JOIN library_persistent_journal_checkpoints AS checkpoints
                   ON checkpoints.root_id = journal.root_id
                  AND checkpoints.root_generation = journal.root_generation
                 WHERE journal.continuity_state = 'current' AND checkpoints.root_id IS NULL
               )
               OR EXISTS(
                 SELECT 1 FROM library_persistent_journal_source_ranges AS ranges
                 JOIN library_persistent_journal_checkpoints AS checkpoints
                   ON checkpoints.root_id = ranges.root_id
                  AND checkpoints.root_generation = ranges.root_generation
                 WHERE ranges.status = 'checkpointed'
                   AND (
                     checkpoints.volume_guid <> ranges.volume_guid
                     OR checkpoints.volume_serial <> ranges.volume_serial
                     OR checkpoints.journal_id <> ranges.journal_id
                     OR CAST(checkpoints.next_unread_usn AS INTEGER)
                       < CAST(ranges.covered_until_usn AS INTEGER)
                   )
               )
               OR EXISTS(
                 SELECT 1 FROM library_persistent_journal_queue_lineage AS lineage
                 JOIN library_persistent_journal_source_ranges AS ranges
                   ON ranges.id = lineage.source_range_id
                 WHERE NOT EXISTS(
                   SELECT 1 FROM library_change_queue_catch_up_lineage AS legacy_lineage
                   WHERE legacy_lineage.change_id = lineage.change_id
                     AND legacy_lineage.catch_up_source = 'persistent_journal_v1'
                     AND legacy_lineage.catch_up_watermark = ranges.id
                 )
               )
               OR EXISTS(
                 SELECT 1 FROM library_persistent_journal_queue_lineage
                 WHERE enrolled_unix_ms < 0
               )
               OR EXISTS(
                 SELECT 1 FROM library_persistent_journal_cross_root_lineage
                 WHERE created_unix_ms < 0 OR updated_unix_ms < 0
               )
               OR EXISTS(
                 SELECT 1 FROM library_persistent_journal_cross_root_ranges
                 WHERE enrolled_unix_ms < 0
               )
               OR EXISTS(
                 SELECT 1 FROM library_persistent_journal_cross_root_ranges AS owners
                 JOIN library_persistent_journal_source_ranges AS ranges
                   ON ranges.id = owners.source_range_id
                 JOIN library_persistent_journal_cross_root_lineage AS lineage
                   ON lineage.id = owners.lineage_id
                 WHERE ranges.volume_guid <> lineage.volume_guid
                    OR ranges.volume_serial <> lineage.volume_serial
                    OR ranges.journal_id <> lineage.journal_id
                    OR (
                      owners.participant_role = 'previous'
                      AND (
                        ranges.root_id <> lineage.previous_root_id
                        OR ranges.root_generation <> lineage.previous_root_generation
                      )
                    )
                    OR (
                      owners.participant_role = 'current'
                      AND (
                        ranges.root_id <> lineage.current_root_id
                        OR ranges.root_generation <> lineage.current_root_generation
                      )
                    )
               )
               OR EXISTS(
                 SELECT 1 FROM library_persistent_journal_cross_root_lineage AS lineage
                 WHERE 2 <> (
                   SELECT COUNT(*)
                   FROM library_persistent_journal_cross_root_ranges AS owners
                   WHERE owners.lineage_id = lineage.id
                 )
                    OR 1 <> (
                      SELECT COUNT(*)
                      FROM library_persistent_journal_cross_root_ranges AS owners
                      WHERE owners.lineage_id = lineage.id
                        AND owners.participant_role = 'previous'
                    )
                    OR 1 <> (
                      SELECT COUNT(*)
                      FROM library_persistent_journal_cross_root_ranges AS owners
                      WHERE owners.lineage_id = lineage.id
                        AND owners.participant_role = 'current'
                    )
               )
               OR EXISTS(
                 SELECT 1 FROM library_persistent_journal_source_ranges AS ranges
                 LEFT JOIN library_persistent_journal_range_lifecycle AS lifecycle
                   ON lifecycle.source_range_id = ranges.id
                 WHERE lifecycle.source_range_id IS NULL
                    OR (ranges.status = 'superseded'
                        AND lifecycle.lifecycle_state <> 'superseded')
               )
               OR EXISTS(
                 SELECT 1 FROM library_persistent_journal_range_lifecycle AS lifecycle
                 WHERE lifecycle.updated_unix_ms < 0
                    OR (lifecycle.lifecycle_state = 'completed'
                        AND EXISTS(
                          SELECT 1
                          FROM library_persistent_journal_queue_lineage AS ownership
                          JOIN library_change_queue AS changes
                            ON changes.id = ownership.change_id
                          WHERE ownership.source_range_id = lifecycle.source_range_id
                            AND changes.status NOT IN ('completed', 'superseded')
                        ))
               )
               OR EXISTS(
                 SELECT 1 FROM library_persistent_journal_pending_renames AS pending
                 JOIN library_persistent_journal_source_ranges AS ranges
                   ON ranges.id = pending.source_range_id
                 WHERE pending.volume_guid <> ranges.volume_guid
                    OR pending.volume_serial <> ranges.volume_serial
                    OR pending.journal_id <> ranges.journal_id
                    OR pending.previous_root_id <> ranges.root_id
                    OR pending.previous_root_generation <> ranges.root_generation
                    OR CAST(pending.old_usn AS INTEGER)
                       < CAST(ranges.requested_start_usn AS INTEGER)
                    OR CAST(pending.old_usn AS INTEGER)
                       >= CAST(ranges.covered_until_usn AS INTEGER)
               )
               OR EXISTS(SELECT 1 FROM pragma_foreign_key_check)",
            [],
            |row| row.get::<_, bool>(0),
        )
        .map_err(database_error)?;
    if invalid_relations {
        return Err(unverifiable_persistent_journal_contract());
    }
    let has_recovery_authority = connection
        .query_row(
            "SELECT EXISTS(
               SELECT 1 FROM sqlite_master
               WHERE type = 'table' AND name = 'library_recovery_authorities'
             )",
            [],
            |row| row.get::<_, bool>(0),
        )
        .map_err(database_error)?;
    let invalid_inventory = if has_recovery_authority {
        connection
            .query_row(
                "SELECT EXISTS(
                   SELECT 1
                   FROM library_metadata_inventory_runs AS runs
                   WHERE (
                     runs.status IN ('running', 'comparing')
                     OR (runs.absence_authority <> 0 AND runs.status <> 'completed')
                   )
                   AND NOT EXISTS(
                     SELECT 1
                     FROM library_recovery_authorities AS authority
                     WHERE authority.run_id = runs.id
                       AND authority.root_id = runs.root_id
                       AND authority.root_generation = runs.root_generation
                       AND authority.retired_unix_ms IS NULL
                   )
                 )",
                [],
                |row| row.get::<_, bool>(0),
            )
            .map_err(database_error)?
    } else {
        connection
            .query_row(
                "SELECT EXISTS(
                   SELECT 1 FROM library_metadata_inventory_runs
                   WHERE status IN ('running', 'comparing')
                      OR (absence_authority <> 0 AND status <> 'completed')
                 )",
                [],
                |row| row.get::<_, bool>(0),
            )
            .map_err(database_error)?
    };
    if invalid_inventory {
        return Err(unverifiable_persistent_journal_contract());
    }
    validate_persistent_journal_root_rows(connection)?;
    validate_persistent_journal_checkpoint_rows(connection)?;
    validate_persistent_journal_range_rows(connection)?;
    validate_persistent_journal_lineage_rows(connection)?;
    validate_persistent_journal_pending_rename_rows(connection)?;
    validate_persistent_journal_payload_children(connection)
}

fn validate_persistent_journal_root_rows(connection: &Connection) -> Result<(), ScanError> {
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
            root_generation: parse_root_generation(root_generation)?,
            protocol_version: u16::try_from(protocol_version)
                .map_err(|_| unverifiable_persistent_journal_contract())?,
            contract_version: u16::try_from(contract_version)
                .map_err(|_| unverifiable_persistent_journal_contract())?,
            state: parse_journal_capability_state(&capability_state)?,
            continuity: parse_journal_continuity_state(&continuity_state)?,
            failure: parse_journal_failure(failure_code, failure_message)?,
            updated_unix_ms,
        }
        .validate()
        .map_err(|_| unverifiable_persistent_journal_contract())?;
    }
    Ok(())
}

fn validate_persistent_journal_checkpoint_rows(connection: &Connection) -> Result<(), ScanError> {
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
            .map_err(|_| unverifiable_persistent_journal_contract())?;
        if i64::from(root_file_reference.record_version()) != reference_version {
            return Err(unverifiable_persistent_journal_contract());
        }
        PersistentJournalCheckpoint {
            root_id,
            root_generation: parse_root_generation(root_generation)?,
            volume: PersistentJournalVolumeIdentity {
                volume_guid,
                volume_serial: parse_canonical_u64(&volume_serial)?,
            },
            root_file_reference,
            journal_id: JournalIdentifier::parse_canonical(&journal_id)
                .map_err(|_| unverifiable_persistent_journal_contract())?,
            next_unread_usn: JournalUsn::parse_canonical(&next_unread_usn)
                .map_err(|_| unverifiable_persistent_journal_contract())?,
            captured_exclusive_end: JournalUsn::parse_canonical(&captured_exclusive_end)
                .map_err(|_| unverifiable_persistent_journal_contract())?,
            covered_catalog_revision: u64::try_from(covered_catalog_revision)
                .map_err(|_| unverifiable_persistent_journal_contract())?,
            protocol_version: u16::try_from(protocol_version)
                .map_err(|_| unverifiable_persistent_journal_contract())?,
            contract_version: u16::try_from(contract_version)
                .map_err(|_| unverifiable_persistent_journal_contract())?,
            continuity: parse_journal_continuity_state(&continuity)?,
            failure: parse_journal_failure(failure_code, failure_message)?,
            updated_unix_ms,
        }
        .validate()
        .map_err(|_| unverifiable_persistent_journal_contract())?;
    }
    Ok(())
}

fn validate_persistent_journal_range_rows(connection: &Connection) -> Result<(), ScanError> {
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
            return Err(unverifiable_persistent_journal_contract());
        }
        let source_range = PersistentJournalSourceRange {
            batch_id,
            root_id,
            root_generation: parse_root_generation(root_generation)?,
            volume: PersistentJournalVolumeIdentity {
                volume_guid,
                volume_serial: parse_canonical_u64(&volume_serial)?,
            },
            journal_id: JournalIdentifier::parse_canonical(&journal_id)
                .map_err(|_| unverifiable_persistent_journal_contract())?,
            requested_start_usn: JournalUsn::parse_canonical(&requested_start_usn)
                .map_err(|_| unverifiable_persistent_journal_contract())?,
            requested_end_usn: JournalUsn::parse_canonical(&requested_end_usn)
                .map_err(|_| unverifiable_persistent_journal_contract())?,
            covered_until_usn: JournalUsn::parse_canonical(&covered_until_usn)
                .map_err(|_| unverifiable_persistent_journal_contract())?,
            is_complete,
            protocol_version: u16::try_from(protocol_version)
                .map_err(|_| unverifiable_persistent_journal_contract())?,
            contract_version: u16::try_from(contract_version)
                .map_err(|_| unverifiable_persistent_journal_contract())?,
            state: parse_journal_range_state(&state)?,
            enrolled_unix_ms,
            checkpointed_unix_ms,
        };
        source_range
            .validate()
            .map_err(|_| unverifiable_persistent_journal_contract())?;
        if !persistent_journal_batch_payload_matches_source_range(&canonical_payload, &source_range)
        {
            return Err(unverifiable_persistent_journal_contract());
        }
    }
    Ok(())
}

pub(super) fn validate_persistent_journal_lineage_rows(
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
            .map_err(|_| unverifiable_persistent_journal_contract())?;
        if i64::from(file_reference.record_version()) != reference_version {
            return Err(unverifiable_persistent_journal_contract());
        }
        let mut lineage = PersistentJournalCrossRootLineage {
            lineage_id: lineage_id.clone(),
            owner_source_range_id: String::new(),
            volume: PersistentJournalVolumeIdentity {
                volume_guid,
                volume_serial: parse_canonical_u64(&volume_serial)?,
            },
            journal_id: JournalIdentifier::parse_canonical(
                journal_id
                    .as_deref()
                    .ok_or_else(unverifiable_persistent_journal_contract)?,
            )
            .map_err(|_| unverifiable_persistent_journal_contract())?,
            file_reference,
            old_usn: old_usn
                .as_deref()
                .map(JournalUsn::parse_canonical)
                .transpose()
                .map_err(|_| unverifiable_persistent_journal_contract())?,
            new_usn: new_usn
                .as_deref()
                .map(JournalUsn::parse_canonical)
                .transpose()
                .map_err(|_| unverifiable_persistent_journal_contract())?,
            previous_carry_id,
            previous_root_id,
            previous_root_generation: parse_root_generation(previous_root_generation)?,
            previous_relative_path,
            current_root_id,
            current_root_generation: parse_root_generation(current_root_generation)?,
            current_relative_path,
            state: parse_journal_lineage_state(&state)?,
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
            return Err(unverifiable_persistent_journal_contract());
        };
        if current_owner.1 != "current" || previous_owner.1 != "previous" {
            return Err(unverifiable_persistent_journal_contract());
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
                || parse_root_generation(owner.3)? != expected.1
                || owner.4 != lineage.volume.volume_guid
                || parse_canonical_u64(&owner.5)? != lineage.volume.volume_serial
                || JournalIdentifier::parse_canonical(&owner.6)
                    .map_err(|_| unverifiable_persistent_journal_contract())?
                    != lineage.journal_id
            {
                return Err(unverifiable_persistent_journal_contract());
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
            return Err(unverifiable_persistent_journal_contract());
        };
        if current_lifecycle.0 != "current" || previous_lifecycle.0 != "previous" {
            return Err(unverifiable_persistent_journal_contract());
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
                _ => return Err(unverifiable_persistent_journal_contract()),
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
            return Err(unverifiable_persistent_journal_contract());
        }
        lineage.owner_source_range_id = previous_owner.0.clone();
        lineage
            .validate()
            .map_err(|_| unverifiable_persistent_journal_contract())?;
        if lineage.previous_carry_id.is_some() {
            if !persistent_journal_batch_payload_contains_pending_lineage_source(
                &previous_owner.7,
                &lineage,
            ) || !persistent_journal_batch_payload_contains_lineage(
                &current_owner.7,
                &current_owner.0,
                &lineage,
            ) {
                return Err(unverifiable_persistent_journal_contract());
            }
            lineage.owner_source_range_id = current_owner.0.clone();
            if !persistent_journal_batch_payload_contains_lineage(
                &current_owner.7,
                &current_owner.0,
                &lineage,
            ) {
                return Err(unverifiable_persistent_journal_contract());
            }
        } else {
            if !persistent_journal_batch_payload_contains_lineage(
                &previous_owner.7,
                &previous_owner.0,
                &lineage,
            ) {
                return Err(unverifiable_persistent_journal_contract());
            }
            lineage.owner_source_range_id = current_owner.0.clone();
            if !persistent_journal_batch_payload_contains_lineage(
                &current_owner.7,
                &current_owner.0,
                &lineage,
            ) {
                return Err(unverifiable_persistent_journal_contract());
            }
        }
    }
    Ok(())
}

fn validate_persistent_journal_pending_rename_rows(
    connection: &Connection,
) -> Result<(), ScanError> {
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
            .map_err(|_| unverifiable_persistent_journal_contract())?;
        if i64::from(file_reference.record_version()) != row.5 {
            return Err(unverifiable_persistent_journal_contract());
        }
        PersistentJournalPendingRename {
            carry_id: row.0,
            source_range_id: row.1,
            volume: PersistentJournalVolumeIdentity {
                volume_guid: row.2,
                volume_serial: parse_canonical_u64(&row.3)?,
            },
            journal_id: JournalIdentifier::parse_canonical(&row.4)
                .map_err(|_| unverifiable_persistent_journal_contract())?,
            file_reference,
            old_usn: JournalUsn::parse_canonical(&row.7)
                .map_err(|_| unverifiable_persistent_journal_contract())?,
            previous_root_id: row.8,
            previous_root_generation: parse_root_generation(row.9)?,
            previous_relative_path: row.10,
            is_directory: row.11,
            enrolled_unix_ms: row.12,
        }
        .validate()
        .map_err(|_| unverifiable_persistent_journal_contract())?;
    }
    Ok(())
}

struct PersistentJournalLineagePayloadProof {
    lineage: PersistentJournalCrossRootLineage,
    previous_range_id: String,
    current_range_id: String,
}

pub(super) fn validate_persistent_journal_payload_children(
    connection: &Connection,
) -> Result<(), ScanError> {
    let payloads = load_persistent_journal_payload_children(connection)?;
    validate_persistent_journal_payload_intents(connection, &payloads)?;
    let lineage_proofs = load_persistent_journal_lineage_payload_proofs(connection)?;
    let pending_renames = load_persistent_journal_pending_rename_proofs(connection)?;
    let mut lineage_slots = std::collections::BTreeMap::<String, Vec<Vec<String>>>::new();
    let mut consumed_by_range = std::collections::BTreeMap::<String, Vec<String>>::new();
    let mut consumed_proofs =
        std::collections::BTreeMap::<String, &PersistentJournalLineagePayloadProof>::new();
    for proof in &lineage_proofs {
        let lineage = &proof.lineage;
        if let Some(carry_id) = &lineage.previous_carry_id {
            if consumed_proofs.insert(carry_id.clone(), proof).is_some() {
                return Err(unverifiable_persistent_journal_contract());
            }
            lineage_slots
                .entry(proof.current_range_id.clone())
                .or_default()
                .push(lineage_payload_candidates(
                    lineage,
                    &proof.previous_range_id,
                    &proof.current_range_id,
                ));
            lineage_slots
                .entry(proof.current_range_id.clone())
                .or_default()
                .push(lineage_payload_candidates(
                    lineage,
                    &proof.current_range_id,
                    &proof.current_range_id,
                ));
            consumed_by_range
                .entry(proof.current_range_id.clone())
                .or_default()
                .push(carry_id.clone());
        } else {
            lineage_slots
                .entry(proof.previous_range_id.clone())
                .or_default()
                .push(lineage_payload_candidates(
                    lineage,
                    &proof.previous_range_id,
                    &proof.previous_range_id,
                ));
            lineage_slots
                .entry(proof.current_range_id.clone())
                .or_default()
                .push(lineage_payload_candidates(
                    lineage,
                    &proof.current_range_id,
                    &proof.current_range_id,
                ));
        }
    }
    for expected in consumed_by_range.values_mut() {
        expected.sort();
    }

    let mut durable_pending = std::collections::BTreeMap::new();
    for pending in pending_renames {
        if consumed_proofs.contains_key(&pending.carry_id)
            || durable_pending
                .insert(pending.carry_id.clone(), pending)
                .is_some()
        {
            return Err(unverifiable_persistent_journal_contract());
        }
    }
    let mut payload_pending = std::collections::BTreeMap::new();
    for (range_id, children) in &payloads {
        let slots = lineage_slots
            .get(range_id)
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        if !lineage_payload_slots_match(&children.lineage, slots) {
            return Err(unverifiable_persistent_journal_contract());
        }
        let expected_consumed = consumed_by_range
            .get(range_id)
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        if children.consumed_pending_rename_ids != expected_consumed {
            return Err(unverifiable_persistent_journal_contract());
        }
        for entry in &children.pending_renames {
            let pending = persistent_journal_pending_rename_from_payload_entry(entry, range_id)
                .ok_or_else(unverifiable_persistent_journal_contract)?;
            if payload_pending
                .insert(pending.carry_id.clone(), pending)
                .is_some()
            {
                return Err(unverifiable_persistent_journal_contract());
            }
        }
    }

    for (carry_id, pending) in &payload_pending {
        if let Some(durable) = durable_pending.get(carry_id) {
            if pending != durable {
                return Err(unverifiable_persistent_journal_contract());
            }
            continue;
        }
        let proof = consumed_proofs
            .get(carry_id)
            .ok_or_else(unverifiable_persistent_journal_contract)?;
        if !pending_rename_matches_consumed_lineage(pending, proof) {
            return Err(unverifiable_persistent_journal_contract());
        }
    }
    for (carry_id, pending) in &durable_pending {
        if payload_pending.get(carry_id) != Some(pending) {
            return Err(unverifiable_persistent_journal_contract());
        }
    }
    for (carry_id, proof) in consumed_proofs {
        let pending = payload_pending
            .get(&carry_id)
            .ok_or_else(unverifiable_persistent_journal_contract)?;
        if !pending_rename_matches_consumed_lineage(pending, proof) {
            return Err(unverifiable_persistent_journal_contract());
        }
    }
    Ok(())
}

fn validate_persistent_journal_payload_intents(
    connection: &Connection,
    payloads: &std::collections::BTreeMap<
        String,
        crate::domain::PersistentJournalBatchPayloadChildren,
    >,
) -> Result<(), ScanError> {
    let orphaned_evidence = connection
        .query_row(
            "SELECT EXISTS(
               SELECT 1
               FROM library_change_queue_catch_up_lineage AS lineage
               LEFT JOIN library_persistent_journal_source_ranges AS ranges
                 ON ranges.id = lineage.catch_up_watermark
               WHERE lineage.catch_up_source = 'persistent_journal_v1'
                 AND (
                   ranges.id IS NULL
                   OR NOT EXISTS (
                     SELECT 1
                     FROM library_persistent_journal_queue_lineage AS direct
                     WHERE direct.source_range_id = lineage.catch_up_watermark
                       AND direct.change_id = lineage.change_id
                   ) AND NOT EXISTS (
                     SELECT 1
                     FROM library_persistent_journal_queue_lineage AS owner
                     JOIN library_persistent_journal_cross_root_ranges AS owner_lineage
                       ON owner_lineage.source_range_id = owner.source_range_id
                     JOIN library_persistent_journal_cross_root_ranges AS peer_lineage
                       ON peer_lineage.lineage_id = owner_lineage.lineage_id
                      AND peer_lineage.source_range_id = lineage.catch_up_watermark
                      AND peer_lineage.enrolled_unix_ms = lineage.enrolled_unix_ms
                     WHERE owner.change_id = lineage.change_id
                   ) AND NOT EXISTS (
                     SELECT 1
                     FROM library_persistent_journal_queue_lineage AS recovery_owner
                     JOIN library_persistent_journal_source_ranges AS recovery_range
                       ON recovery_range.id = recovery_owner.source_range_id
                     JOIN library_change_queue AS recovery_change
                       ON recovery_change.id = recovery_owner.change_id
                     JOIN library_persistent_journal_cross_root_lineage AS recovery_lineage
                       ON recovery_lineage.previous_carry_id IS NOT NULL
                      AND recovery_lineage.previous_root_id = recovery_change.root_id
                      AND recovery_lineage.previous_root_generation = recovery_change.root_generation
                      AND recovery_lineage.previous_relative_path = recovery_change.relative_path
                      AND recovery_lineage.new_usn = recovery_change.first_sequence
                      AND recovery_lineage.new_usn = recovery_change.most_recent_sequence
                      AND recovery_lineage.volume_guid = recovery_range.volume_guid
                      AND recovery_lineage.volume_serial = recovery_range.volume_serial
                      AND recovery_lineage.journal_id = recovery_range.journal_id
                      AND CAST(recovery_range.requested_start_usn AS INTEGER)
                            <= CAST(recovery_lineage.new_usn AS INTEGER)
                      AND CAST(recovery_lineage.new_usn AS INTEGER)
                            < CAST(recovery_range.covered_until_usn AS INTEGER)
                     JOIN library_persistent_journal_cross_root_ranges AS recovery_endpoint
                       ON recovery_endpoint.lineage_id = recovery_lineage.id
                      AND recovery_endpoint.source_range_id = lineage.catch_up_watermark
                      AND recovery_endpoint.enrolled_unix_ms = lineage.enrolled_unix_ms
                     WHERE recovery_owner.change_id = lineage.change_id
                   )
                 )
             )",
            [],
            |row| row.get::<_, bool>(0),
        )
        .map_err(database_error)?;
    if orphaned_evidence {
        return Err(unverifiable_persistent_journal_contract());
    }

    for (range_id, children) in payloads {
        let mut expected =
            super::change_queue::normalize_persistent_journal_intents(&children.intents)
                .map_err(|_| unverifiable_persistent_journal_contract())?
                .iter()
                .map(persistent_journal_canonical_intent_entry)
                .collect::<Vec<_>>();
        expected.sort();

        let range_enrolled_unix_ms = connection
            .query_row(
                "SELECT enrolled_unix_ms
                 FROM library_persistent_journal_source_ranges WHERE id = ?1",
                [range_id],
                |row| row.get::<_, i64>(0),
            )
            .map_err(database_error)?;
        let mut statement = connection
            .prepare(
                "SELECT changes.root_id, changes.root_generation, changes.intent_kind,
                        changes.scope, changes.relative_path, changes.previous_relative_path,
                        changes.origin, changes.first_observed_unix_ms,
                        changes.most_recent_observed_unix_ms, changes.first_sequence,
                        changes.most_recent_sequence, changes.coalesced_observation_count,
                        changes.status, changes.catch_up_source, changes.catch_up_watermark,
                        ownership.enrolled_unix_ms,
                        (SELECT COUNT(*)
                         FROM library_change_queue_catch_up_lineage AS lineage
                         WHERE lineage.change_id = changes.id
                           AND (
                             lineage.catch_up_source <> 'persistent_journal_v1'
                             OR NOT (
                               lineage.catch_up_watermark = ?1
                               OR EXISTS (
                                 SELECT 1
                                 FROM library_persistent_journal_cross_root_ranges AS owner_lineage
                                 JOIN library_persistent_journal_cross_root_ranges AS peer_lineage
                                   ON peer_lineage.lineage_id = owner_lineage.lineage_id
                                  AND peer_lineage.source_range_id = lineage.catch_up_watermark
                                  AND peer_lineage.enrolled_unix_ms = lineage.enrolled_unix_ms
                                 WHERE owner_lineage.source_range_id = ?1
                               ) OR EXISTS (
                                 SELECT 1
                                 FROM library_persistent_journal_source_ranges AS recovery_range
                                 JOIN library_persistent_journal_cross_root_lineage AS recovery_lineage
                                   ON recovery_lineage.previous_carry_id IS NOT NULL
                                  AND recovery_lineage.previous_root_id = changes.root_id
                                  AND recovery_lineage.previous_root_generation = changes.root_generation
                                  AND recovery_lineage.previous_relative_path = changes.relative_path
                                  AND recovery_lineage.new_usn = changes.first_sequence
                                  AND recovery_lineage.new_usn = changes.most_recent_sequence
                                  AND recovery_lineage.volume_guid = recovery_range.volume_guid
                                  AND recovery_lineage.volume_serial = recovery_range.volume_serial
                                  AND recovery_lineage.journal_id = recovery_range.journal_id
                                  AND CAST(recovery_range.requested_start_usn AS INTEGER)
                                        <= CAST(recovery_lineage.new_usn AS INTEGER)
                                  AND CAST(recovery_lineage.new_usn AS INTEGER)
                                        < CAST(recovery_range.covered_until_usn AS INTEGER)
                                 JOIN library_persistent_journal_cross_root_ranges AS recovery_endpoint
                                   ON recovery_endpoint.lineage_id = recovery_lineage.id
                                  AND recovery_endpoint.source_range_id = lineage.catch_up_watermark
                                  AND recovery_endpoint.enrolled_unix_ms = lineage.enrolled_unix_ms
                                 WHERE recovery_range.id = ?1
                               )
                             )
                           )),
                        (SELECT COUNT(*)
                         FROM library_change_queue_catch_up_lineage AS lineage
                         WHERE lineage.change_id = changes.id
                           AND lineage.catch_up_source = 'persistent_journal_v1'
                           AND lineage.catch_up_watermark = ?1
                           AND lineage.enrolled_unix_ms = ?2)
                 FROM library_persistent_journal_queue_lineage AS ownership
                 JOIN library_change_queue AS changes ON changes.id = ownership.change_id
                 WHERE ownership.source_range_id = ?1
                 ORDER BY changes.id",
            )
            .map_err(database_error)?;
        let rows = statement
            .query_map(params![range_id, range_enrolled_unix_ms], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, Option<String>>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, i64>(7)?,
                    row.get::<_, i64>(8)?,
                    row.get::<_, String>(9)?,
                    row.get::<_, String>(10)?,
                    row.get::<_, i64>(11)?,
                    row.get::<_, String>(12)?,
                    row.get::<_, Option<String>>(13)?,
                    row.get::<_, Option<String>>(14)?,
                    row.get::<_, i64>(15)?,
                    row.get::<_, i64>(16)?,
                    row.get::<_, i64>(17)?,
                ))
            })
            .map_err(database_error)?;
        let mut actual = Vec::new();
        for row in rows {
            let row = row.map_err(database_error)?;
            if !matches!(
                row.12.as_str(),
                "pending" | "leased" | "retry_wait" | "completed" | "superseded"
            ) || row.13.as_deref() != Some("persistent_journal_v1")
                || row.14.as_deref() != Some(range_id.as_str())
                || row.15 != range_enrolled_unix_ms
                || row.16 != 0
                || row.17 != 1
            {
                return Err(unverifiable_persistent_journal_contract());
            }
            let intent = LibraryChangeIntent {
                root_id: row.0,
                root_generation: parse_root_generation(row.1)?,
                kind: parse_legacy_intent_kind(&row.2)?,
                scope: parse_legacy_intent_scope(&row.3)?,
                relative_path: row.4,
                previous_relative_path: row.5,
                origin: parse_legacy_intent_origin(&row.6)?,
                first_observed_unix_ms: row.7,
                most_recent_observed_unix_ms: row.8,
                first_sequence: parse_canonical_u64(&row.9)?,
                most_recent_sequence: parse_canonical_u64(&row.10)?,
                coalesced_observation_count: u32::try_from(row.11)
                    .map_err(|_| unverifiable_persistent_journal_contract())?,
            };
            actual.push(persistent_journal_canonical_intent_entry(&intent));
        }
        actual.sort();
        if actual != expected {
            return Err(unverifiable_persistent_journal_contract());
        }
    }
    Ok(())
}

fn load_persistent_journal_payload_children(
    connection: &Connection,
) -> Result<
    std::collections::BTreeMap<String, crate::domain::PersistentJournalBatchPayloadChildren>,
    ScanError,
> {
    let mut statement = connection
        .prepare(
            "SELECT id, canonical_payload
             FROM library_persistent_journal_source_ranges ORDER BY id",
        )
        .map_err(database_error)?;
    let rows = statement
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, Vec<u8>>(1)?))
        })
        .map_err(database_error)?;
    let mut payloads = std::collections::BTreeMap::new();
    for row in rows {
        let (range_id, payload) = row.map_err(database_error)?;
        let children = persistent_journal_batch_payload_children(&payload)
            .ok_or_else(unverifiable_persistent_journal_contract)?;
        if payloads.insert(range_id, children).is_some() {
            return Err(unverifiable_persistent_journal_contract());
        }
    }
    Ok(payloads)
}

fn load_persistent_journal_lineage_payload_proofs(
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
            .map_err(|_| unverifiable_persistent_journal_contract())?;
        if i64::from(file_reference.record_version()) != row.3 {
            return Err(unverifiable_persistent_journal_contract());
        }
        let lineage = PersistentJournalCrossRootLineage {
            lineage_id: row.0,
            owner_source_range_id: row.16.clone(),
            volume: PersistentJournalVolumeIdentity {
                volume_guid: row.1,
                volume_serial: parse_canonical_u64(&row.2)?,
            },
            journal_id: JournalIdentifier::parse_canonical(
                row.12
                    .as_deref()
                    .ok_or_else(unverifiable_persistent_journal_contract)?,
            )
            .map_err(|_| unverifiable_persistent_journal_contract())?,
            file_reference,
            old_usn: row
                .13
                .as_deref()
                .map(JournalUsn::parse_canonical)
                .transpose()
                .map_err(|_| unverifiable_persistent_journal_contract())?,
            new_usn: row
                .14
                .as_deref()
                .map(JournalUsn::parse_canonical)
                .transpose()
                .map_err(|_| unverifiable_persistent_journal_contract())?,
            previous_carry_id: row.15,
            previous_root_id: row.5,
            previous_root_generation: parse_root_generation(row.6)?,
            previous_relative_path: row.7,
            current_root_id: row.8,
            current_root_generation: parse_root_generation(row.9)?,
            current_relative_path: row.10,
            state: parse_journal_lineage_state(&row.11)?,
        };
        lineage
            .validate()
            .map_err(|_| unverifiable_persistent_journal_contract())?;
        proofs.push(PersistentJournalLineagePayloadProof {
            lineage,
            previous_range_id: row.16,
            current_range_id: row.17,
        });
    }
    Ok(proofs)
}

fn load_persistent_journal_pending_rename_proofs(
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
            .map_err(|_| unverifiable_persistent_journal_contract())?;
        if i64::from(file_reference.record_version()) != row.5 {
            return Err(unverifiable_persistent_journal_contract());
        }
        let pending = PersistentJournalPendingRename {
            carry_id: row.0,
            source_range_id: row.1,
            volume: PersistentJournalVolumeIdentity {
                volume_guid: row.2,
                volume_serial: parse_canonical_u64(&row.3)?,
            },
            journal_id: JournalIdentifier::parse_canonical(&row.4)
                .map_err(|_| unverifiable_persistent_journal_contract())?,
            file_reference,
            old_usn: JournalUsn::parse_canonical(&row.7)
                .map_err(|_| unverifiable_persistent_journal_contract())?,
            previous_root_id: row.8,
            previous_root_generation: parse_root_generation(row.9)?,
            previous_relative_path: row.10,
            is_directory: row.11,
            enrolled_unix_ms: row.12,
        };
        pending
            .validate()
            .map_err(|_| unverifiable_persistent_journal_contract())?;
        pending_renames.push(pending);
    }
    Ok(pending_renames)
}

fn lineage_payload_candidates(
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

fn lineage_payload_slots_match(actual: &[String], expected: &[Vec<String>]) -> bool {
    if actual.len() != expected.len() {
        return false;
    }
    let mut matched = vec![false; expected.len()];
    for entry in actual {
        let candidates = expected
            .iter()
            .enumerate()
            .filter(|(index, slot)| !matched[*index] && slot.contains(entry))
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        let [index] = candidates.as_slice() else {
            return false;
        };
        matched[*index] = true;
    }
    matched.into_iter().all(|is_matched| is_matched)
}

fn pending_rename_matches_consumed_lineage(
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

fn parse_root_generation(value: i64) -> Result<LibraryRootGeneration, ScanError> {
    u64::try_from(value)
        .ok()
        .and_then(LibraryRootGeneration::new)
        .ok_or_else(unverifiable_persistent_journal_contract)
}

fn parse_canonical_u64(value: &str) -> Result<u64, ScanError> {
    let parsed = value
        .parse::<u64>()
        .map_err(|_| unverifiable_persistent_journal_contract())?;
    if parsed.to_string() != value {
        return Err(unverifiable_persistent_journal_contract());
    }
    Ok(parsed)
}

fn parse_journal_capability_state(
    value: &str,
) -> Result<PersistentJournalCapabilityState, ScanError> {
    match value {
        "unknown" => Ok(PersistentJournalCapabilityState::Unknown),
        "supported" => Ok(PersistentJournalCapabilityState::Supported),
        "live_only" => Ok(PersistentJournalCapabilityState::LiveOnly),
        _ => Err(unverifiable_persistent_journal_contract()),
    }
}

fn parse_journal_continuity_state(
    value: &str,
) -> Result<PersistentJournalContinuityState, ScanError> {
    match value {
        "baseline_required" => Ok(PersistentJournalContinuityState::BaselineRequired),
        "catching_up" => Ok(PersistentJournalContinuityState::CatchingUp),
        "current" => Ok(PersistentJournalContinuityState::Current),
        "recovery_required" => Ok(PersistentJournalContinuityState::RecoveryRequired),
        "live_only" => Ok(PersistentJournalContinuityState::LiveOnly),
        "unavailable" => Ok(PersistentJournalContinuityState::Unavailable),
        _ => Err(unverifiable_persistent_journal_contract()),
    }
}

fn parse_journal_range_state(value: &str) -> Result<PersistentJournalRangeState, ScanError> {
    match value {
        "enrolled" => Ok(PersistentJournalRangeState::Enrolled),
        "checkpointed" => Ok(PersistentJournalRangeState::Checkpointed),
        "superseded" => Ok(PersistentJournalRangeState::Superseded),
        _ => Err(unverifiable_persistent_journal_contract()),
    }
}

fn parse_journal_lineage_state(value: &str) -> Result<PersistentJournalLineageState, ScanError> {
    match value {
        "pending" => Ok(PersistentJournalLineageState::Pending),
        "completed" => Ok(PersistentJournalLineageState::Completed),
        "superseded" => Ok(PersistentJournalLineageState::Superseded),
        _ => Err(unverifiable_persistent_journal_contract()),
    }
}

fn parse_journal_failure(
    code: Option<String>,
    message: Option<String>,
) -> Result<Option<PersistentJournalFailure>, ScanError> {
    match (code, message) {
        (None, None) => Ok(None),
        (Some(code), Some(message)) => Ok(Some(PersistentJournalFailure { code, message })),
        _ => Err(unverifiable_persistent_journal_contract()),
    }
}

fn unverifiable_persistent_journal_contract() -> ScanError {
    ScanError::new(
        "catalog_persistent_journal_contract_unverifiable",
        "The catalog cannot prove its per-root persistent journal continuity authority",
    )
}

fn unverifiable_metadata_inventory_contract() -> ScanError {
    ScanError::new(
        "catalog_metadata_inventory_contract_unverifiable",
        "The catalog cannot prove its metadata-inventory staging and completion authority",
    )
}

fn repair_v19_preview_expectations(connection: &mut Connection) -> Result<(), ScanError> {
    if preview_repair_marker_is_complete(connection)? {
        return Ok(());
    }
    repair_missing_v19_preview_expectation_marker(connection)
}

fn preview_repair_marker_is_complete(connection: &Connection) -> Result<bool, ScanError> {
    let marker_type = connection
        .query_row(
            "SELECT type FROM sqlite_master
             WHERE name = 'library_change_preview_repair_contract'",
            [],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(database_error)?;
    if let Some(marker_type) = marker_type {
        let marker_valid = marker_type == "table"
            && table_columns_match(
                connection,
                "library_change_preview_repair_contract",
                &[
                    ("singleton", "INTEGER", false, 1),
                    ("complete", "INTEGER", true, 0),
                ],
            )?
            && connection
                .query_row(
                    "SELECT singleton = 1 AND complete = 1
                     FROM library_change_preview_repair_contract",
                    [],
                    |row| row.get::<_, bool>(0),
                )
                .optional()
                .map_err(database_error)?
                .unwrap_or(false);
        if !marker_valid {
            return Err(ScanError::new(
                "catalog_change_catch_up_contract_unverifiable",
                "The catalog preview repair marker is malformed",
            ));
        }
        return Ok(true);
    }
    Ok(false)
}

fn repair_missing_v19_preview_expectation_marker(
    connection: &mut Connection,
) -> Result<(), ScanError> {
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(database_error)?;
    if preview_repair_marker_is_complete(&transaction)? {
        return transaction.commit().map_err(database_error);
    }
    transaction
        .execute_batch(
            "UPDATE library_change_catch_up_handoffs
             SET preview_path = '', preview_status = 'pending',
                 preview_issue_code = NULL, preview_issue_message = NULL
             WHERE preview_status = 'ready'
               AND NOT EXISTS (
                 SELECT 1 FROM preview_artifacts AS artifacts
                 WHERE artifacts.lifecycle_state = 'ready'
                   AND artifacts.artifact_path =
                     library_change_catch_up_handoffs.preview_path
               );
             UPDATE library_change_scan_handoff_items
             SET preview_path = '', preview_status = 'pending',
                 preview_issue_code = NULL, preview_issue_message = NULL
             WHERE preview_status = 'ready'
               AND NOT EXISTS (
                 SELECT 1 FROM preview_artifacts AS artifacts
                 WHERE artifacts.lifecycle_state = 'ready'
                   AND artifacts.artifact_path =
                     library_change_scan_handoff_items.preview_path
               );
             DELETE FROM preview_artifact_locations
             WHERE location_id IN (
               SELECT locations.location_id
               FROM library_roots AS roots
               JOIN asset_locations AS locations
                 ON locations.root_id = roots.id
                AND locations.scan_id = roots.active_scan_id
               WHERE locations.preview_status = 'ready'
                 AND NOT EXISTS (
                   SELECT 1 FROM preview_artifacts AS artifacts
                   WHERE artifacts.lifecycle_state = 'ready'
                     AND artifacts.artifact_path = locations.preview_path
                 )
             );
             UPDATE asset_locations
             SET preview_path = '', preview_status = 'pending',
                 preview_issue_code = NULL, preview_issue_message = NULL
             WHERE preview_status = 'ready'
               AND scan_id = (
                 SELECT roots.active_scan_id
                 FROM library_roots AS roots
                 WHERE roots.id = asset_locations.root_id
               )
               AND NOT EXISTS (
                 SELECT 1 FROM preview_artifacts AS artifacts
                 WHERE artifacts.lifecycle_state = 'ready'
                   AND artifacts.artifact_path = asset_locations.preview_path
               );
             CREATE TABLE library_change_preview_repair_contract (
               singleton INTEGER PRIMARY KEY CHECK(singleton = 1),
               complete INTEGER NOT NULL CHECK(complete = 1)
             );
             INSERT INTO library_change_preview_repair_contract(singleton, complete)
             VALUES (1, 1);",
        )
        .map_err(database_error)?;
    transaction.commit().map_err(database_error)
}

fn add_preview_expectation_repair_marker(transaction: &Transaction<'_>) -> Result<(), ScanError> {
    transaction
        .execute_batch(
            "CREATE TABLE library_change_preview_repair_contract (
               singleton INTEGER PRIMARY KEY CHECK(singleton = 1),
               complete INTEGER NOT NULL CHECK(complete = 1)
             );
             INSERT INTO library_change_preview_repair_contract(singleton, complete)
             VALUES (1, 1);",
        )
        .map_err(database_error)
}

fn validate_change_catch_up_contract_with_depth(
    connection: &Connection,
    depth: ContractValidationDepth,
) -> Result<(), ScanError> {
    let state_columns_match = table_columns_match(
        connection,
        "library_change_catch_up_state",
        &[
            ("volume_id", "TEXT", false, 1),
            ("journal_id", "TEXT", true, 0),
            ("next_usn", "TEXT", true, 0),
            ("root_set_fingerprint", "TEXT", true, 0),
            ("catalog_revision", "INTEGER", true, 0),
            ("updated_unix_ms", "INTEGER", true, 0),
        ],
    )?;
    let mut handoff_columns = vec![
        ("catch_up_source", "TEXT", true, 1),
        ("catch_up_watermark", "TEXT", true, 2),
        ("file_identity_scheme", "TEXT", true, 3),
        ("file_identity_value", "TEXT", true, 4),
        ("asset_id", "TEXT", true, 0),
        ("source_location_id", "TEXT", true, 0),
        ("root_id", "TEXT", true, 0),
        ("absolute_path", "TEXT", true, 0),
        ("relative_path", "TEXT", true, 0),
        ("preview_path", "TEXT", true, 0),
        ("file_size", "INTEGER", true, 0),
        ("created_unix_ms", "INTEGER", false, 0),
        ("modified_unix_ms", "INTEGER", true, 0),
        ("width", "INTEGER", true, 0),
        ("height", "INTEGER", true, 0),
        ("preview_status", "TEXT", true, 0),
        ("preview_issue_code", "TEXT", false, 0),
        ("preview_issue_message", "TEXT", false, 0),
        ("metadata_engine_id", "TEXT", true, 0),
        ("metadata_engine_version", "TEXT", true, 0),
        ("capture_local_time", "TEXT", false, 0),
        ("capture_offset_minutes", "INTEGER", false, 0),
        ("capture_time_source", "TEXT", false, 0),
        ("capture_raw_value", "TEXT", false, 0),
        ("updated_unix_ms", "INTEGER", true, 0),
    ];
    if schema_version(connection)? >= 31 {
        handoff_columns.push(("source_revision_token", "TEXT", false, 0));
        handoff_columns.push(("source_generation", "INTEGER", false, 0));
    }
    let handoff_columns_match = table_columns_match(
        connection,
        "library_change_catch_up_handoffs",
        &handoff_columns,
    )?;
    let (
        has_table,
        has_marker,
        has_path_index,
        has_handoff_table,
        has_asset_index,
        has_preview_index,
        has_lineage_table,
        has_lineage_index,
        has_lineage_foreign_key,
        has_scan_lineage_marker,
        has_scan_lineage_table,
        has_scan_lineage_index,
        has_scan_lineage_foreign_key,
    ) = connection
        .query_row(
            "SELECT
               EXISTS(SELECT 1 FROM sqlite_master
                 WHERE type = 'table' AND name = 'library_change_catch_up_state'),
               EXISTS(SELECT 1 FROM pragma_table_info('library_change_queue_contract')
                 WHERE name = 'change_catch_up_complete'),
               EXISTS(SELECT 1 FROM pragma_index_list('asset_locations')
                 WHERE name = 'asset_locations_root_relative'
                   AND \"unique\" = 0 AND partial = 0)
               AND (SELECT group_concat(name, ',') FROM (
                 SELECT name FROM pragma_index_info('asset_locations_root_relative')
                 ORDER BY seqno
                )) = 'root_id,relative_path,scan_id,location_id',
               EXISTS(SELECT 1 FROM sqlite_master
                 WHERE type = 'table' AND name = 'library_change_catch_up_handoffs')
               AND (SELECT COUNT(*) FROM pragma_table_info('library_change_catch_up_handoffs')
                 WHERE name IN (
                   'catch_up_source', 'catch_up_watermark', 'file_identity_scheme',
                   'file_identity_value', 'asset_id', 'source_location_id', 'root_id',
                   'absolute_path', 'relative_path', 'preview_path', 'file_size',
                   'created_unix_ms', 'modified_unix_ms', 'width', 'height',
                   'preview_status', 'preview_issue_code', 'preview_issue_message',
                   'metadata_engine_id', 'metadata_engine_version', 'capture_local_time',
                   'capture_offset_minutes', 'capture_time_source', 'capture_raw_value',
                   'updated_unix_ms', 'source_revision_token', 'source_generation'
                 )) IN (25, 27)
               AND (SELECT group_concat(name, ',') FROM (
                 SELECT name FROM pragma_table_info('library_change_catch_up_handoffs')
                 WHERE pk > 0 ORDER BY pk
               )) = 'catch_up_source,catch_up_watermark,file_identity_scheme,file_identity_value',
               EXISTS(SELECT 1 FROM pragma_index_list('library_change_catch_up_handoffs')
                 WHERE name = 'library_change_catch_up_handoffs_asset'
                   AND \"unique\" = 0 AND partial = 0)
               AND (SELECT group_concat(name, ',') FROM (
                 SELECT name FROM pragma_index_info('library_change_catch_up_handoffs_asset')
                 ORDER BY seqno
               )) = 'asset_id,catch_up_source,catch_up_watermark',
               EXISTS(SELECT 1 FROM pragma_index_list('library_change_catch_up_handoffs')
                 WHERE name = 'library_change_catch_up_handoffs_preview'
                   AND \"unique\" = 0 AND partial = 0)
               AND (SELECT group_concat(name, ',') FROM (
                 SELECT name FROM pragma_index_info('library_change_catch_up_handoffs_preview')
                 ORDER BY seqno
               )) = 'preview_path,preview_status,catch_up_source,catch_up_watermark',
               EXISTS(SELECT 1 FROM sqlite_master
                 WHERE type = 'table' AND name = 'library_change_queue_catch_up_lineage')
               AND (SELECT COUNT(*) FROM pragma_table_info(
                 'library_change_queue_catch_up_lineage'
               )) = 4
               AND (SELECT COUNT(*) FROM pragma_table_info(
                 'library_change_queue_catch_up_lineage'
               ) WHERE name IN (
                 'change_id', 'catch_up_source', 'catch_up_watermark', 'enrolled_unix_ms'
               )) = 4
               AND EXISTS(SELECT 1 FROM pragma_table_info(
                 'library_change_queue_catch_up_lineage'
               ) WHERE name = 'change_id' AND upper(type) = 'INTEGER'
                   AND \"notnull\" = 1 AND dflt_value IS NULL AND pk = 1)
               AND EXISTS(SELECT 1 FROM pragma_table_info(
                 'library_change_queue_catch_up_lineage'
               ) WHERE name = 'catch_up_source' AND upper(type) = 'TEXT'
                   AND \"notnull\" = 1 AND dflt_value IS NULL AND pk = 2)
               AND EXISTS(SELECT 1 FROM pragma_table_info(
                 'library_change_queue_catch_up_lineage'
               ) WHERE name = 'catch_up_watermark' AND upper(type) = 'TEXT'
                   AND \"notnull\" = 1 AND dflt_value IS NULL AND pk = 3)
               AND EXISTS(SELECT 1 FROM pragma_table_info(
                 'library_change_queue_catch_up_lineage'
               ) WHERE name = 'enrolled_unix_ms' AND upper(type) = 'INTEGER'
                   AND \"notnull\" = 1 AND dflt_value IS NULL AND pk = 0)
               AND (SELECT group_concat(name, ',') FROM (
                 SELECT name FROM pragma_table_info('library_change_queue_catch_up_lineage')
                 WHERE pk > 0 ORDER BY pk
               )) = 'change_id,catch_up_source,catch_up_watermark',
               EXISTS(SELECT 1 FROM pragma_index_list(
                 'library_change_queue_catch_up_lineage'
               ) WHERE name = 'library_change_queue_catch_up_lineage_evidence'
                   AND \"unique\" = 0 AND partial = 0)
               AND (SELECT group_concat(name, ',') FROM (
                 SELECT name FROM pragma_index_info(
                   'library_change_queue_catch_up_lineage_evidence'
                 ) ORDER BY seqno
               )) = 'catch_up_source,catch_up_watermark,change_id',
               (SELECT COUNT(*) FROM pragma_foreign_key_list(
                 'library_change_queue_catch_up_lineage'
               )) = 1
               AND EXISTS(SELECT 1 FROM pragma_foreign_key_list(
                 'library_change_queue_catch_up_lineage'
               ) WHERE \"table\" = 'library_change_queue' AND \"from\" = 'change_id'
                   AND \"to\" = 'id' AND on_update = 'NO ACTION'
                   AND on_delete = 'CASCADE' AND \"match\" = 'NONE'),
               EXISTS(SELECT 1 FROM pragma_table_info('library_change_queue_contract')
                 WHERE name = 'scan_catch_up_lineage_complete'),
               EXISTS(SELECT 1 FROM sqlite_master
                 WHERE type = 'table' AND name = 'scan_run_catch_up_lineage')
               AND (SELECT COUNT(*) FROM pragma_table_info('scan_run_catch_up_lineage')) = 4
               AND (SELECT COUNT(*) FROM pragma_table_info('scan_run_catch_up_lineage')
                 WHERE name IN (
                   'scan_id', 'catch_up_source', 'catch_up_watermark', 'enrolled_unix_ms'
                 )) = 4
               AND EXISTS(SELECT 1 FROM pragma_table_info('scan_run_catch_up_lineage')
                 WHERE name = 'scan_id' AND upper(type) = 'TEXT'
                   AND \"notnull\" = 1 AND dflt_value IS NULL AND pk = 1)
               AND EXISTS(SELECT 1 FROM pragma_table_info('scan_run_catch_up_lineage')
                 WHERE name = 'catch_up_source' AND upper(type) = 'TEXT'
                   AND \"notnull\" = 1 AND dflt_value IS NULL AND pk = 2)
               AND EXISTS(SELECT 1 FROM pragma_table_info('scan_run_catch_up_lineage')
                 WHERE name = 'catch_up_watermark' AND upper(type) = 'TEXT'
                   AND \"notnull\" = 1 AND dflt_value IS NULL AND pk = 3)
               AND EXISTS(SELECT 1 FROM pragma_table_info('scan_run_catch_up_lineage')
                 WHERE name = 'enrolled_unix_ms' AND upper(type) = 'INTEGER'
                   AND \"notnull\" = 1 AND dflt_value IS NULL AND pk = 0)
               AND (SELECT group_concat(name, ',') FROM (
                 SELECT name FROM pragma_table_info('scan_run_catch_up_lineage')
                 WHERE pk > 0 ORDER BY pk
               )) = 'scan_id,catch_up_source,catch_up_watermark',
               EXISTS(SELECT 1 FROM pragma_index_list('scan_run_catch_up_lineage')
                 WHERE name = 'scan_run_catch_up_lineage_evidence'
                   AND \"unique\" = 0 AND partial = 0)
               AND (SELECT group_concat(name, ',') FROM (
                 SELECT name FROM pragma_index_info('scan_run_catch_up_lineage_evidence')
                 ORDER BY seqno
               )) = 'catch_up_source,catch_up_watermark,scan_id',
               (SELECT COUNT(*) FROM pragma_foreign_key_list('scan_run_catch_up_lineage')) = 1
               AND EXISTS(SELECT 1 FROM pragma_foreign_key_list(
                 'scan_run_catch_up_lineage'
               ) WHERE \"table\" = 'scan_runs' AND \"from\" = 'scan_id'
                   AND \"to\" = 'id' AND on_update = 'NO ACTION'
                   AND on_delete = 'CASCADE' AND \"match\" = 'NONE')",
            [],
            |row| {
                Ok((
                    row.get::<_, bool>(0)?,
                    row.get::<_, bool>(1)?,
                    row.get::<_, bool>(2)?,
                    row.get::<_, bool>(3)?,
                    row.get::<_, bool>(4)?,
                    row.get::<_, bool>(5)?,
                    row.get::<_, bool>(6)?,
                    row.get::<_, bool>(7)?,
                    row.get::<_, bool>(8)?,
                    row.get::<_, bool>(9)?,
                    row.get::<_, bool>(10)?,
                    row.get::<_, bool>(11)?,
                    row.get::<_, bool>(12)?,
                ))
            },
        )
        .map_err(database_error)?;
    if !state_columns_match
        || !handoff_columns_match
        || !has_table
        || !has_marker
        || !has_path_index
        || !has_handoff_table
        || !has_asset_index
        || !has_preview_index
        || !has_lineage_table
        || !has_lineage_index
        || !has_lineage_foreign_key
        || !has_scan_lineage_marker
        || !has_scan_lineage_table
        || !has_scan_lineage_index
        || !has_scan_lineage_foreign_key
    {
        return Err(ScanError::new(
            "catalog_change_catch_up_contract_unverifiable",
            "The catalog cannot prove its downtime catch-up checkpoint authority",
        ));
    }
    let has_scan_handoff_marker = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM pragma_table_info('library_change_queue_contract')
             WHERE name = 'scan_handoff_batch_complete')",
            [],
            |row| row.get::<_, bool>(0),
        )
        .map_err(database_error)?;
    if !has_scan_handoff_marker {
        return Err(ScanError::new(
            "catalog_change_catch_up_contract_unverifiable",
            "The catalog cannot prove its normalized scan handoff contract",
        ));
    }
    let marker_complete = connection
        .query_row(
            "SELECT change_catch_up_complete = 1
                    AND scan_catch_up_lineage_complete = 1
                    AND scan_handoff_batch_complete = 1
             FROM library_change_queue_contract WHERE singleton = 1",
            [],
            |row| row.get::<_, bool>(0),
        )
        .optional()
        .map_err(database_error)?
        .unwrap_or(false);
    if !marker_complete {
        return Err(ScanError::new(
            "catalog_change_catch_up_contract_unverifiable",
            "The catalog cannot prove its downtime catch-up checkpoint authority",
        ));
    }
    if !depth.includes_rows() {
        return Ok(());
    }
    record_current_schema_row_audit();
    let invalid_lineage = connection
        .query_row(
            "SELECT
               EXISTS(
                 SELECT 1 FROM library_change_queue_catch_up_lineage AS lineage
                 LEFT JOIN library_change_queue AS changes ON changes.id = lineage.change_id
                 WHERE changes.id IS NULL
               )
               OR EXISTS(
                 SELECT 1 FROM library_change_queue AS changes
                 WHERE (
                   (changes.catch_up_source IS NULL) <> (changes.catch_up_watermark IS NULL)
                 ) OR (
                   changes.catch_up_source IS NOT NULL
                   AND NOT EXISTS (
                     SELECT 1 FROM library_change_queue_catch_up_lineage AS lineage
                     WHERE lineage.change_id = changes.id
                       AND lineage.catch_up_source = changes.catch_up_source
                       AND lineage.catch_up_watermark = changes.catch_up_watermark
                   )
                 ) OR (
                   changes.catch_up_source IS NULL
                   AND EXISTS (
                     SELECT 1 FROM library_change_queue_catch_up_lineage AS lineage
                     WHERE lineage.change_id = changes.id
                   )
                 ) OR (
                   SELECT COUNT(*) FROM library_change_queue_catch_up_lineage AS lineage
                   WHERE lineage.change_id = changes.id
                 ) > 64
               )
               OR EXISTS(
                 SELECT 1 FROM scan_run_catch_up_lineage AS lineage
                 LEFT JOIN scan_runs AS scans ON scans.id = lineage.scan_id
                 WHERE scans.id IS NULL
                    OR scans.status NOT IN ('running', 'paused')
               )
               OR EXISTS(
                 SELECT 1 FROM scan_run_catch_up_lineage
                 GROUP BY scan_id HAVING COUNT(*) > ?1
               )
               OR EXISTS(
                 SELECT 1
                 FROM scan_runs AS scans
                 JOIN library_change_queue AS changes
                   ON changes.root_id = scans.root_id
                  AND changes.root_generation = scans.root_generation_at_start
                  AND changes.id <= scans.change_queue_high_watermark
                 JOIN library_change_queue_catch_up_lineage AS lineage
                   ON lineage.change_id = changes.id
                 WHERE scans.status IN ('running', 'paused')
                   AND changes.status IN ('pending', 'leased', 'retry_wait')
                   AND NOT EXISTS (
                     SELECT 1 FROM scan_run_catch_up_lineage AS frozen
                     WHERE frozen.scan_id = scans.id
                       AND frozen.catch_up_source = lineage.catch_up_source
                       AND frozen.catch_up_watermark = lineage.catch_up_watermark
                   )
               )
               OR EXISTS(SELECT 1 FROM pragma_foreign_key_check(
                 'library_change_queue_catch_up_lineage'
               ))
               OR EXISTS(SELECT 1 FROM pragma_foreign_key_check(
                 'scan_run_catch_up_lineage'
               ))",
            [MAX_SCAN_CATCH_UP_LINEAGE],
            |row| row.get::<_, bool>(0),
        )
        .map_err(database_error)?;
    if invalid_lineage {
        return Err(ScanError::new(
            "catalog_change_catch_up_contract_unverifiable",
            "The catalog cannot prove its bounded queue-to-watermark lineage",
        ));
    }
    Ok(())
}

fn validate_scan_handoff_batch_contract_with_depth(
    connection: &Connection,
    depth: ContractValidationDepth,
) -> Result<(), ScanError> {
    let marker_complete = connection
        .query_row(
            "SELECT scan_handoff_batch_complete = 1
             FROM library_change_queue_contract WHERE singleton = 1",
            [],
            |row| row.get::<_, bool>(0),
        )
        .optional()
        .map_err(database_error)?
        .unwrap_or(false);
    let structure_matches = marker_complete
        && table_columns_match(
            connection,
            "library_change_scan_handoff_batches",
            &[
                ("id", "TEXT", true, 1),
                ("source_root_id", "TEXT", true, 0),
                ("updated_unix_ms", "INTEGER", true, 0),
            ],
        )?
        && table_columns_match(
            connection,
            "library_change_scan_handoff_lineage",
            &[
                ("batch_id", "TEXT", true, 1),
                ("catch_up_source", "TEXT", true, 2),
                ("catch_up_watermark", "TEXT", true, 3),
                ("enrolled_unix_ms", "INTEGER", true, 0),
            ],
        )?
        && table_columns_match(
            connection,
            "library_change_scan_handoff_items",
            &[
                ("batch_id", "TEXT", true, 1),
                ("file_identity_scheme", "TEXT", true, 2),
                ("file_identity_value", "TEXT", true, 3),
                ("asset_id", "TEXT", true, 0),
                ("source_location_id", "TEXT", true, 0),
                ("root_id", "TEXT", true, 0),
                ("absolute_path", "TEXT", true, 0),
                ("relative_path", "TEXT", true, 0),
                ("preview_path", "TEXT", true, 0),
                ("file_size", "INTEGER", true, 0),
                ("created_unix_ms", "INTEGER", false, 0),
                ("modified_unix_ms", "INTEGER", true, 0),
                ("width", "INTEGER", true, 0),
                ("height", "INTEGER", true, 0),
                ("preview_status", "TEXT", true, 0),
                ("preview_issue_code", "TEXT", false, 0),
                ("preview_issue_message", "TEXT", false, 0),
                ("metadata_engine_id", "TEXT", true, 0),
                ("metadata_engine_version", "TEXT", true, 0),
                ("capture_local_time", "TEXT", false, 0),
                ("capture_offset_minutes", "INTEGER", false, 0),
                ("capture_time_source", "TEXT", false, 0),
                ("capture_raw_value", "TEXT", false, 0),
            ],
        )?
        && named_index_matches(
            connection,
            "library_change_scan_handoff_lineage",
            "library_change_scan_handoff_lineage_evidence",
            &["catch_up_source", "catch_up_watermark", "batch_id"],
        )?
        && named_index_matches(
            connection,
            "library_change_scan_handoff_items",
            "library_change_scan_handoff_items_identity",
            &["file_identity_scheme", "file_identity_value", "batch_id"],
        )?
        && named_index_matches(
            connection,
            "library_change_scan_handoff_items",
            "library_change_scan_handoff_items_asset",
            &["asset_id", "batch_id"],
        )?
        && named_index_matches(
            connection,
            "library_change_scan_handoff_items",
            "library_change_scan_handoff_items_preview",
            &["preview_path", "preview_status", "batch_id"],
        )?
        && cascade_foreign_key_matches(
            connection,
            "library_change_scan_handoff_lineage",
            "batch_id",
            "library_change_scan_handoff_batches",
            "id",
        )?
        && cascade_foreign_key_matches(
            connection,
            "library_change_scan_handoff_items",
            "batch_id",
            "library_change_scan_handoff_batches",
            "id",
        )?;
    if !structure_matches {
        return Err(ScanError::new(
            "catalog_change_catch_up_contract_unverifiable",
            "The catalog cannot prove its normalized scan handoff contract",
        ));
    }
    if !depth.includes_rows() {
        return Ok(());
    }
    record_current_schema_row_audit();

    let invalid_relations = connection
        .query_row(
            "SELECT EXISTS(
               SELECT 1 FROM library_change_catch_up_handoffs AS handoffs
               WHERE NOT EXISTS (
                 SELECT 1
                 FROM library_change_queue_catch_up_lineage AS lineage
                 JOIN library_change_queue AS changes ON changes.id = lineage.change_id
                 WHERE lineage.catch_up_source = handoffs.catch_up_source
                   AND lineage.catch_up_watermark = handoffs.catch_up_watermark
                   AND changes.status IN ('pending', 'leased', 'retry_wait')
               ) AND NOT EXISTS (
                 SELECT 1
                 FROM scan_run_catch_up_lineage AS lineage
                 JOIN scan_runs AS scans ON scans.id = lineage.scan_id
                 WHERE lineage.catch_up_source = handoffs.catch_up_source
                   AND lineage.catch_up_watermark = handoffs.catch_up_watermark
                   AND scans.status IN ('running', 'paused')
               )
             ) OR EXISTS(
               SELECT 1 FROM library_change_scan_handoff_lineage AS handoffs
               WHERE NOT EXISTS (
                 SELECT 1
                 FROM library_change_queue_catch_up_lineage AS lineage
                 JOIN library_change_queue AS changes ON changes.id = lineage.change_id
                 WHERE lineage.catch_up_source = handoffs.catch_up_source
                   AND lineage.catch_up_watermark = handoffs.catch_up_watermark
                   AND changes.status IN ('pending', 'leased', 'retry_wait')
               ) AND NOT EXISTS (
                 SELECT 1
                 FROM scan_run_catch_up_lineage AS lineage
                 JOIN scan_runs AS scans ON scans.id = lineage.scan_id
                 WHERE lineage.catch_up_source = handoffs.catch_up_source
                   AND lineage.catch_up_watermark = handoffs.catch_up_watermark
                   AND scans.status IN ('running', 'paused')
               )
             ) OR EXISTS(
               SELECT 1 FROM library_change_scan_handoff_batches AS batches
               WHERE NOT EXISTS (
                 SELECT 1 FROM library_change_scan_handoff_lineage AS lineage
                 WHERE lineage.batch_id = batches.id
               ) OR NOT EXISTS (
                 SELECT 1 FROM library_change_scan_handoff_items AS items
                 WHERE items.batch_id = batches.id
               )
             ) OR EXISTS(
               SELECT 1
               FROM scan_run_catch_up_lineage AS frozen
               JOIN scan_runs AS scans ON scans.id = frozen.scan_id
               WHERE scans.status IN ('running', 'paused')
                 AND NOT EXISTS (
                   SELECT 1
                   FROM library_change_queue AS changes
                   JOIN library_change_queue_catch_up_lineage AS lineage
                     ON lineage.change_id = changes.id
                   WHERE changes.root_id = scans.root_id
                     AND changes.root_generation = scans.root_generation_at_start
                     AND changes.id <= scans.change_queue_high_watermark
                    AND lineage.catch_up_source = frozen.catch_up_source
                     AND lineage.catch_up_watermark = frozen.catch_up_watermark
                 )
             ) OR EXISTS(SELECT 1 FROM pragma_foreign_key_check(
               'library_change_scan_handoff_lineage'
             )) OR EXISTS(SELECT 1 FROM pragma_foreign_key_check(
               'library_change_scan_handoff_items'
             ))",
            [],
            |row| row.get::<_, bool>(0),
        )
        .map_err(database_error)?;
    if invalid_relations {
        return Err(ScanError::new(
            "catalog_change_catch_up_contract_unverifiable",
            "The catalog cannot prove the ownership of its retained handoff evidence",
        ));
    }
    Ok(())
}

fn table_columns_match(
    connection: &Connection,
    table: &str,
    expected: &[(&str, &str, bool, i64)],
) -> Result<bool, ScanError> {
    let mut statement = connection
        .prepare(&format!("PRAGMA table_info('{table}')"))
        .map_err(database_error)?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, bool>(3)?,
                row.get::<_, i64>(5)?,
            ))
        })
        .map_err(database_error)?;
    let actual = rows
        .collect::<Result<Vec<_>, _>>()
        .map_err(database_error)?;
    let mut expected = expected.to_vec();
    if schema_version(connection)? >= 31
        && !expected
            .iter()
            .any(|column| column.0 == "source_revision_token")
        && table == "library_change_scan_handoff_items"
    {
        expected.push(("source_revision_token", "TEXT", false, 0));
        expected.push(("source_generation", "INTEGER", false, 0));
    }
    Ok(actual.len() == expected.len()
        && actual.iter().zip(&expected).all(|(actual, expected)| {
            actual.0 == expected.0
                && actual.1.eq_ignore_ascii_case(expected.1)
                && actual.2 == expected.2
                && actual.3 == expected.3
        }))
}

fn named_index_matches(
    connection: &Connection,
    table: &str,
    index: &str,
    expected_columns: &[&str],
) -> Result<bool, ScanError> {
    let index_is_plain = connection
        .query_row(
            &format!(
                "SELECT EXISTS(SELECT 1 FROM pragma_index_list('{table}')
                 WHERE name = ?1 AND \"unique\" = 0 AND partial = 0)"
            ),
            [index],
            |row| row.get::<_, bool>(0),
        )
        .map_err(database_error)?;
    if !index_is_plain {
        return Ok(false);
    }
    let mut statement = connection
        .prepare(&format!("PRAGMA index_info('{index}')"))
        .map_err(database_error)?;
    let columns = statement
        .query_map([], |row| row.get::<_, String>(2))
        .map_err(database_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(database_error)?;
    Ok(columns == expected_columns)
}

fn schema_object_sql_matches(
    connection: &Connection,
    object_type: &str,
    name: &str,
    expected: &str,
) -> Result<bool, ScanError> {
    let actual = connection
        .query_row(
            "SELECT sql FROM sqlite_master WHERE type = ?1 AND name = ?2",
            params![object_type, name],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(database_error)?;
    Ok(
        actual
            .is_some_and(|actual| normalize_schema_sql(&actual) == normalize_schema_sql(expected)),
    )
}

fn normalize_schema_sql(value: &str) -> String {
    let mut canonical = String::with_capacity(value.len());
    let mut characters = value.chars().peekable();
    let mut pending_whitespace = false;
    while let Some(character) = characters.next() {
        if character.is_whitespace() {
            pending_whitespace = true;
            continue;
        }
        if pending_whitespace
            && canonical
                .chars()
                .next_back()
                .is_some_and(|previous| schema_tokens_require_separator(previous, character))
        {
            canonical.push(' ');
        }
        pending_whitespace = false;
        if matches!(character, '\'' | '"' | '`' | '[') {
            canonical.push(character);
            let terminator = if character == '[' { ']' } else { character };
            while let Some(quoted) = characters.next() {
                canonical.push(quoted);
                if quoted != terminator {
                    continue;
                }
                if characters.peek() == Some(&terminator) {
                    canonical.push(terminator);
                    characters.next();
                } else {
                    break;
                }
            }
        } else {
            canonical.extend(character.to_lowercase());
        }
    }
    canonical
}

fn schema_tokens_require_separator(previous: char, next: char) -> bool {
    !is_schema_separator(previous) && !is_schema_separator(next)
}

fn is_schema_separator(character: char) -> bool {
    matches!(
        character,
        '(' | ')'
            | ','
            | ';'
            | '.'
            | '='
            | '<'
            | '>'
            | '!'
            | '+'
            | '-'
            | '*'
            | '/'
            | '%'
            | '|'
            | '&'
            | '~'
    )
}

fn repair_prerelease_v24_source_range_id_triggers(
    connection: &mut Connection,
) -> Result<(), ScanError> {
    let mut current_count = 0;
    let mut legacy_count = 0;
    for ((name, current), (legacy_name, legacy)) in PERSISTENT_JOURNAL_CANONICAL_TRIGGER_DDL
        .iter()
        .zip(PERSISTENT_JOURNAL_LEGACY_V24_TRIGGER_DDL)
    {
        if name != legacy_name {
            return Err(unverifiable_persistent_journal_contract());
        }
        let actual = connection
            .query_row(
                "SELECT sql FROM sqlite_master WHERE type = 'trigger' AND name = ?1",
                [name],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(database_error)?;
        let Some(actual) = actual else {
            return Ok(());
        };
        current_count +=
            usize::from(normalize_schema_sql(&actual) == normalize_schema_sql(current));
        legacy_count += usize::from(normalize_schema_sql(&actual) == normalize_schema_sql(legacy));
    }
    if current_count == PERSISTENT_JOURNAL_CANONICAL_TRIGGER_DDL.len() {
        return Ok(());
    }
    if legacy_count != PERSISTENT_JOURNAL_LEGACY_V24_TRIGGER_DDL.len() {
        return Ok(());
    }
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(database_error)?;
    for (name, sql) in PERSISTENT_JOURNAL_CANONICAL_TRIGGER_DDL {
        transaction
            .execute_batch(&format!("DROP TRIGGER {name};"))
            .map_err(database_error)?;
        transaction.execute_batch(sql).map_err(database_error)?;
    }
    transaction.commit().map_err(database_error)
}

fn repair_prerelease_v26_recovery_window_schema(
    connection: &mut Connection,
) -> Result<(), ScanError> {
    let index_sql = connection
        .query_row(
            "SELECT sql FROM sqlite_master
             WHERE type = 'index' AND name = 'library_persistent_journal_baselines_root'",
            [],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(database_error)?;
    let trigger_sql = connection
        .query_row(
            "SELECT sql FROM sqlite_master
             WHERE type = 'trigger'
               AND name = 'library_persistent_journal_baseline_insert_guard'",
            [],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(database_error)?;
    let is_current = index_sql.as_deref().is_some_and(|sql| {
        normalize_schema_sql(sql)
            == normalize_schema_sql(PERSISTENT_JOURNAL_BASELINE_ROOT_INDEX_DDL)
    }) && trigger_sql.as_deref().is_some_and(|sql| {
        normalize_schema_sql(sql)
            == normalize_schema_sql(PERSISTENT_JOURNAL_BASELINE_INSERT_GUARD_DDL)
    });
    if is_current {
        return Ok(());
    }
    let is_legacy = index_sql.as_deref().is_some_and(|sql| {
        normalize_schema_sql(sql)
            == normalize_schema_sql(LEGACY_V26_PERSISTENT_JOURNAL_BASELINE_ROOT_INDEX_DDL)
    }) && trigger_sql.as_deref().is_some_and(|sql| {
        normalize_schema_sql(sql)
            == normalize_schema_sql(LEGACY_V26_PERSISTENT_JOURNAL_BASELINE_INSERT_GUARD_DDL)
    });
    if !is_legacy {
        return Ok(());
    }
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(database_error)?;
    transaction
        .execute_batch(
            "DROP INDEX library_persistent_journal_baselines_root;
             DROP TRIGGER library_persistent_journal_baseline_insert_guard;",
        )
        .map_err(database_error)?;
    transaction
        .execute_batch(PERSISTENT_JOURNAL_BASELINE_ROOT_INDEX_DDL)
        .map_err(database_error)?;
    transaction
        .execute_batch(PERSISTENT_JOURNAL_BASELINE_INSERT_GUARD_DDL)
        .map_err(database_error)?;
    transaction.commit().map_err(database_error)
}

fn cascade_foreign_key_matches(
    connection: &Connection,
    table: &str,
    from: &str,
    target_table: &str,
    target_column: &str,
) -> Result<bool, ScanError> {
    connection
        .query_row(
            &format!(
                "SELECT COUNT(*) = 1 AND EXISTS(
                   SELECT 1 FROM pragma_foreign_key_list('{table}')
                   WHERE \"table\" = ?1 AND \"from\" = ?2 AND \"to\" = ?3
                     AND on_update = 'NO ACTION' AND on_delete = 'CASCADE'
                     AND \"match\" = 'NONE'
                 ) FROM pragma_foreign_key_list('{table}')"
            ),
            params![target_table, from, target_column],
            |row| row.get::<_, bool>(0),
        )
        .map_err(database_error)
}

fn repair_prerelease_v19_derived_indexes(connection: &mut Connection) -> Result<(), ScanError> {
    let (
        has_table,
        has_marker,
        has_named_path_index,
        has_path_columns,
        has_handoff_table,
        has_handoff_columns,
        has_named_asset_index,
        has_named_preview_index,
        has_lineage_table,
        has_lineage_columns,
        has_named_lineage_index,
        has_obsolete_peer_index,
    ) = connection
        .query_row(
            "SELECT
               EXISTS(SELECT 1 FROM sqlite_master
                 WHERE type = 'table' AND name = 'library_change_catch_up_state'),
               EXISTS(SELECT 1 FROM pragma_table_info('library_change_queue_contract')
                 WHERE name = 'change_catch_up_complete'),
               EXISTS(SELECT 1 FROM sqlite_master
                 WHERE type = 'index' AND name = 'asset_locations_root_relative'),
                (SELECT COUNT(*) FROM pragma_table_info('asset_locations')
                  WHERE name IN ('root_id', 'relative_path', 'scan_id', 'location_id')) = 4,
                EXISTS(SELECT 1 FROM sqlite_master
                  WHERE type = 'table' AND name = 'library_change_catch_up_handoffs'),
                (SELECT COUNT(*) FROM pragma_table_info('library_change_catch_up_handoffs')
                  WHERE name IN (
                    'catch_up_source', 'catch_up_watermark', 'file_identity_scheme',
                    'file_identity_value', 'asset_id', 'source_location_id', 'root_id',
                    'absolute_path', 'relative_path', 'preview_path', 'file_size',
                    'created_unix_ms', 'modified_unix_ms', 'width', 'height',
                    'preview_status', 'preview_issue_code', 'preview_issue_message',
                    'metadata_engine_id', 'metadata_engine_version', 'capture_local_time',
                    'capture_offset_minutes', 'capture_time_source', 'capture_raw_value',
                    'updated_unix_ms'
                  )) = 25,
                EXISTS(SELECT 1 FROM sqlite_master
                  WHERE type = 'index'
                    AND name = 'library_change_catch_up_handoffs_asset'),
                EXISTS(SELECT 1 FROM sqlite_master
                  WHERE type = 'index'
                    AND name = 'library_change_catch_up_handoffs_preview'),
                EXISTS(SELECT 1 FROM sqlite_master
                  WHERE type = 'table'
                    AND name = 'library_change_queue_catch_up_lineage'),
                (SELECT COUNT(*) FROM pragma_table_info(
                  'library_change_queue_catch_up_lineage'
                ) WHERE name IN (
                  'change_id', 'catch_up_source', 'catch_up_watermark', 'enrolled_unix_ms'
                )) = 4,
                EXISTS(SELECT 1 FROM sqlite_master
                  WHERE type = 'index'
                    AND name = 'library_change_queue_catch_up_lineage_evidence'),
                EXISTS(SELECT 1 FROM sqlite_master
                  WHERE type = 'index'
                    AND name = 'library_change_queue_catch_up_peer')",
            [],
            |row| {
                Ok((
                    row.get::<_, bool>(0)?,
                    row.get::<_, bool>(1)?,
                    row.get::<_, bool>(2)?,
                    row.get::<_, bool>(3)?,
                    row.get::<_, bool>(4)?,
                    row.get::<_, bool>(5)?,
                    row.get::<_, bool>(6)?,
                    row.get::<_, bool>(7)?,
                    row.get::<_, bool>(8)?,
                    row.get::<_, bool>(9)?,
                    row.get::<_, bool>(10)?,
                    row.get::<_, bool>(11)?,
                ))
            },
        )
        .map_err(database_error)?;
    if !has_table || !has_marker || !has_path_columns {
        return Ok(());
    }
    let marker_complete = connection
        .query_row(
            "SELECT change_catch_up_complete = 1
             FROM library_change_queue_contract WHERE singleton = 1",
            [],
            |row| row.get::<_, bool>(0),
        )
        .optional()
        .map_err(database_error)?
        .unwrap_or(false);
    if !marker_complete {
        return Ok(());
    }
    if (has_handoff_table && !has_handoff_columns) || (has_lineage_table && !has_lineage_columns) {
        return Ok(());
    }
    let has_unseeded_lineage = if has_lineage_table {
        connection
            .query_row(
                "SELECT EXISTS(
                   SELECT 1 FROM library_change_queue AS changes
                   WHERE changes.catch_up_source IS NOT NULL
                     AND changes.catch_up_watermark IS NOT NULL
                     AND NOT EXISTS (
                       SELECT 1 FROM library_change_queue_catch_up_lineage AS lineage
                       WHERE lineage.change_id = changes.id
                         AND lineage.catch_up_source = changes.catch_up_source
                         AND lineage.catch_up_watermark = changes.catch_up_watermark
                     )
                 )",
                [],
                |row| row.get::<_, bool>(0),
            )
            .map_err(database_error)?
    } else {
        false
    };
    let has_handoff_rows = if has_handoff_table {
        connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM library_change_catch_up_handoffs)",
                [],
                |row| row.get::<_, bool>(0),
            )
            .map_err(database_error)?
    } else {
        false
    };
    if has_handoff_rows && (!has_lineage_table || has_unseeded_lineage) {
        return Err(ScanError::new(
            "catalog_change_catch_up_contract_unverifiable",
            "The catalog cannot prove the watermark lineage of its retained handoff evidence",
        ));
    }
    if has_named_path_index
        && has_handoff_table
        && has_named_asset_index
        && has_named_preview_index
        && has_lineage_table
        && has_named_lineage_index
        && !has_obsolete_peer_index
        && !has_unseeded_lineage
    {
        return Ok(());
    }

    let transaction = connection.transaction().map_err(database_error)?;
    if !has_named_path_index {
        transaction
            .execute(
                "CREATE INDEX asset_locations_root_relative
                 ON asset_locations(root_id, relative_path, scan_id, location_id)",
                [],
            )
            .map_err(database_error)?;
    }
    if !has_handoff_table {
        create_change_catch_up_handoff_contract(&transaction)?;
    } else if has_handoff_columns {
        if !has_named_asset_index {
            transaction
                .execute(
                    "CREATE INDEX library_change_catch_up_handoffs_asset
                     ON library_change_catch_up_handoffs(
                       asset_id, catch_up_source, catch_up_watermark
                     )",
                    [],
                )
                .map_err(database_error)?;
        }
        if !has_named_preview_index {
            transaction
                .execute(
                    "CREATE INDEX library_change_catch_up_handoffs_preview
                     ON library_change_catch_up_handoffs(
                       preview_path, preview_status, catch_up_source, catch_up_watermark
                     )",
                    [],
                )
                .map_err(database_error)?;
        }
    }
    if !has_lineage_table {
        create_change_catch_up_lineage_contract(&transaction)?;
    } else if has_lineage_columns && !has_named_lineage_index {
        transaction
            .execute(
                "CREATE INDEX library_change_queue_catch_up_lineage_evidence
                 ON library_change_queue_catch_up_lineage(
                   catch_up_source, catch_up_watermark, change_id
                 )",
                [],
            )
            .map_err(database_error)?;
    }
    if !has_lineage_table || has_lineage_columns {
        seed_change_catch_up_lineage(&transaction)?;
    }
    transaction
        .execute(
            "DROP INDEX IF EXISTS library_change_queue_catch_up_peer",
            [],
        )
        .map_err(database_error)?;
    transaction.commit().map_err(database_error)
}

fn repair_prerelease_v19_scan_lineage(connection: &mut Connection) -> Result<(), ScanError> {
    let (has_state_table, has_change_marker, has_scan_marker, has_scan_lineage_table) = connection
        .query_row(
            "SELECT
               EXISTS(SELECT 1 FROM sqlite_master
                 WHERE type = 'table' AND name = 'library_change_catch_up_state'),
               EXISTS(SELECT 1 FROM pragma_table_info('library_change_queue_contract')
                 WHERE name = 'change_catch_up_complete'),
               EXISTS(SELECT 1 FROM pragma_table_info('library_change_queue_contract')
                 WHERE name = 'scan_catch_up_lineage_complete'),
               EXISTS(SELECT 1 FROM sqlite_master
                 WHERE type = 'table' AND name = 'scan_run_catch_up_lineage')",
            [],
            |row| {
                Ok((
                    row.get::<_, bool>(0)?,
                    row.get::<_, bool>(1)?,
                    row.get::<_, bool>(2)?,
                    row.get::<_, bool>(3)?,
                ))
            },
        )
        .map_err(database_error)?;
    if !has_state_table || !has_change_marker || has_scan_marker {
        return Ok(());
    }
    let change_marker_complete = connection
        .query_row(
            "SELECT change_catch_up_complete = 1
             FROM library_change_queue_contract WHERE singleton = 1",
            [],
            |row| row.get::<_, bool>(0),
        )
        .optional()
        .map_err(database_error)?
        .unwrap_or(false);
    if !change_marker_complete {
        return Ok(());
    }
    if has_scan_lineage_table {
        return Ok(());
    }
    let has_unprovable_active_scan = connection
        .query_row(
            "SELECT EXISTS(
               SELECT 1 FROM scan_runs AS scans
               WHERE scans.status IN ('running', 'paused')
                 AND scans.change_queue_high_watermark IS NOT NULL
                 AND NOT EXISTS (
                   SELECT 1 FROM library_change_queue AS changes
                   WHERE changes.root_id = scans.root_id
                     AND changes.root_generation = scans.root_generation_at_start
                     AND changes.id <= scans.change_queue_high_watermark
                     AND changes.status IN ('pending', 'leased', 'retry_wait')
                 )
             )",
            [],
            |row| row.get::<_, bool>(0),
        )
        .map_err(database_error)?;
    if has_unprovable_active_scan {
        return Err(ScanError::new(
            "catalog_change_catch_up_contract_unverifiable",
            "The catalog cannot reconstruct the frozen lineage of an active scan",
        ));
    }
    let transaction = connection.transaction().map_err(database_error)?;
    create_scan_run_catch_up_lineage_contract(&transaction)?;
    transaction
        .execute(
            "INSERT INTO scan_run_catch_up_lineage(
               scan_id, catch_up_source, catch_up_watermark, enrolled_unix_ms
             )
             SELECT scans.id, lineage.catch_up_source, lineage.catch_up_watermark,
                    MAX(lineage.enrolled_unix_ms)
             FROM scan_runs AS scans
             JOIN library_change_queue AS changes
               ON changes.root_id = scans.root_id
              AND changes.root_generation = scans.root_generation_at_start
              AND changes.id <= scans.change_queue_high_watermark
             JOIN library_change_queue_catch_up_lineage AS lineage
               ON lineage.change_id = changes.id
             WHERE scans.status IN ('running', 'paused')
               AND changes.status IN ('pending', 'leased', 'retry_wait')
             GROUP BY scans.id, lineage.catch_up_source, lineage.catch_up_watermark",
            [],
        )
        .map_err(database_error)?;
    let oversized_scan = transaction
        .query_row(
            "SELECT EXISTS(
               SELECT 1 FROM scan_run_catch_up_lineage
               GROUP BY scan_id HAVING COUNT(*) > ?1
             )",
            [MAX_SCAN_CATCH_UP_LINEAGE],
            |row| row.get::<_, bool>(0),
        )
        .map_err(database_error)?;
    if oversized_scan {
        return Err(ScanError::new(
            "catalog_change_catch_up_contract_unverifiable",
            "The catalog cannot reconstruct a bounded scan lineage",
        ));
    }
    transaction
        .execute(
            "ALTER TABLE library_change_queue_contract
             ADD COLUMN scan_catch_up_lineage_complete INTEGER NOT NULL DEFAULT 1
             CHECK(scan_catch_up_lineage_complete = 1)",
            [],
        )
        .map_err(database_error)?;
    transaction.commit().map_err(database_error)
}

fn repair_prerelease_v19_scan_handoff_batches(
    connection: &mut Connection,
) -> Result<(), ScanError> {
    let (
        has_change_marker,
        has_scan_lineage_marker,
        has_scan_handoff_marker,
        has_batches,
        has_batch_lineage,
        has_batch_items,
    ) = connection
        .query_row(
            "SELECT
               EXISTS(SELECT 1 FROM pragma_table_info('library_change_queue_contract')
                 WHERE name = 'change_catch_up_complete'),
               EXISTS(SELECT 1 FROM pragma_table_info('library_change_queue_contract')
                 WHERE name = 'scan_catch_up_lineage_complete'),
               EXISTS(SELECT 1 FROM pragma_table_info('library_change_queue_contract')
                 WHERE name = 'scan_handoff_batch_complete'),
               EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table'
                 AND name = 'library_change_scan_handoff_batches'),
               EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table'
                 AND name = 'library_change_scan_handoff_lineage'),
               EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table'
                 AND name = 'library_change_scan_handoff_items')",
            [],
            |row| {
                Ok((
                    row.get::<_, bool>(0)?,
                    row.get::<_, bool>(1)?,
                    row.get::<_, bool>(2)?,
                    row.get::<_, bool>(3)?,
                    row.get::<_, bool>(4)?,
                    row.get::<_, bool>(5)?,
                ))
            },
        )
        .map_err(database_error)?;
    if !has_change_marker || !has_scan_lineage_marker || has_scan_handoff_marker {
        return Ok(());
    }
    if has_batches || has_batch_lineage || has_batch_items {
        return Ok(());
    }
    let prerequisite_complete = connection
        .query_row(
            "SELECT change_catch_up_complete = 1 AND scan_catch_up_lineage_complete = 1
             FROM library_change_queue_contract WHERE singleton = 1",
            [],
            |row| row.get::<_, bool>(0),
        )
        .optional()
        .map_err(database_error)?
        .unwrap_or(false);
    if !prerequisite_complete {
        return Ok(());
    }
    let authority_is_unprovable = connection
        .query_row(
            "SELECT EXISTS(
               SELECT 1 FROM library_change_catch_up_handoffs AS handoffs
               WHERE NOT EXISTS (
                 SELECT 1
                 FROM library_change_queue_catch_up_lineage AS lineage
                 JOIN library_change_queue AS changes ON changes.id = lineage.change_id
                 WHERE lineage.catch_up_source = handoffs.catch_up_source
                   AND lineage.catch_up_watermark = handoffs.catch_up_watermark
                   AND changes.status IN ('pending', 'leased', 'retry_wait')
               ) AND NOT EXISTS (
                 SELECT 1
                 FROM scan_run_catch_up_lineage AS lineage
                 JOIN scan_runs AS scans ON scans.id = lineage.scan_id
                 WHERE lineage.catch_up_source = handoffs.catch_up_source
                   AND lineage.catch_up_watermark = handoffs.catch_up_watermark
                   AND scans.status IN ('running', 'paused')
               )
             ) OR EXISTS(
               SELECT 1
               FROM scan_run_catch_up_lineage AS frozen
               JOIN scan_runs AS scans ON scans.id = frozen.scan_id
               WHERE scans.status IN ('running', 'paused')
                 AND NOT EXISTS (
                   SELECT 1
                   FROM library_change_queue AS changes
                   JOIN library_change_queue_catch_up_lineage AS lineage
                     ON lineage.change_id = changes.id
                   WHERE changes.root_id = scans.root_id
                     AND changes.root_generation = scans.root_generation_at_start
                     AND changes.id <= scans.change_queue_high_watermark
                     AND lineage.catch_up_source = frozen.catch_up_source
                     AND lineage.catch_up_watermark = frozen.catch_up_watermark
                 )
             )",
            [],
            |row| row.get::<_, bool>(0),
        )
        .map_err(database_error)?;
    if authority_is_unprovable {
        return Err(ScanError::new(
            "catalog_change_catch_up_contract_unverifiable",
            "The catalog cannot prove the ownership of its prerelease handoff evidence",
        ));
    }

    let transaction = connection.transaction().map_err(database_error)?;
    create_scan_handoff_batch_contract(&transaction)?;
    transaction
        .execute(
            "ALTER TABLE library_change_queue_contract
             ADD COLUMN scan_handoff_batch_complete INTEGER NOT NULL DEFAULT 1
             CHECK(scan_handoff_batch_complete = 1)",
            [],
        )
        .map_err(database_error)?;
    transaction.commit().map_err(database_error)
}

fn validate_authoritative_recovery_marker(connection: &Connection) -> Result<(), ScanError> {
    validate_authoritative_recovery_base_marker(connection)?;
    let has_scan_ownership_contract = connection
        .query_row(
            "SELECT EXISTS(
               SELECT 1 FROM pragma_table_info('library_change_queue_contract')
               WHERE name = 'scan_ownership_complete'
             )",
            [],
            |row| row.get::<_, bool>(0),
        )
        .map_err(database_error)?;
    if !has_scan_ownership_contract {
        return Err(unverifiable_authoritative_recovery_contract());
    }
    let ownership_complete = connection
        .query_row(
            "SELECT scan_ownership_complete = 1
             FROM library_change_queue_contract WHERE singleton = 1",
            [],
            |row| row.get::<_, bool>(0),
        )
        .optional()
        .map_err(database_error)?
        .unwrap_or(false);
    if !ownership_complete {
        return Err(unverifiable_authoritative_recovery_contract());
    }
    Ok(())
}

fn validate_authoritative_recovery_base_marker(connection: &Connection) -> Result<(), ScanError> {
    let has_recovery_contract = connection
        .query_row(
            "SELECT EXISTS(
               SELECT 1 FROM pragma_table_info('library_change_queue_contract')
               WHERE name = 'authoritative_recovery_complete'
             )",
            [],
            |row| row.get::<_, bool>(0),
        )
        .map_err(database_error)?;
    if !has_recovery_contract {
        return Err(unverifiable_authoritative_recovery_contract());
    }
    let recovery_complete = connection
        .query_row(
            "SELECT authoritative_recovery_complete = 1
             FROM library_change_queue_contract WHERE singleton = 1",
            [],
            |row| row.get::<_, bool>(0),
        )
        .optional()
        .map_err(database_error)?
        .unwrap_or(false);
    if !recovery_complete {
        return Err(unverifiable_authoritative_recovery_contract());
    }
    Ok(())
}

fn repair_prerelease_v18_scan_owner_index(connection: &mut Connection) -> Result<(), ScanError> {
    validate_change_queue_authority(connection)?;
    validate_authoritative_recovery_base_marker(connection)?;
    let (has_scan_runs, has_single_scan_owner_index, has_scan_owner, has_ownership_marker) =
        connection
            .query_row(
                "SELECT
               EXISTS(SELECT 1 FROM sqlite_master
                 WHERE type = 'table' AND name = 'scan_runs'),
               EXISTS(SELECT 1 FROM pragma_index_list('scan_runs')
                 WHERE name = 'scan_runs_one_active_root'
                   AND \"unique\" = 1 AND partial = 1),
               EXISTS(SELECT 1 FROM pragma_table_info('scan_runs')
                 WHERE name = 'scan_owner'),
               EXISTS(SELECT 1 FROM pragma_table_info('library_change_queue_contract')
                 WHERE name = 'scan_ownership_complete')",
                [],
                |row| {
                    Ok((
                        row.get::<_, bool>(0)?,
                        row.get::<_, bool>(1)?,
                        row.get::<_, bool>(2)?,
                        row.get::<_, bool>(3)?,
                    ))
                },
            )
            .map_err(database_error)?;
    if (!has_scan_runs || has_single_scan_owner_index && has_scan_owner) && has_ownership_marker {
        return Ok(());
    }
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(database_error)?;
    let (has_single_scan_owner_index, has_scan_owner, has_ownership_marker) = transaction
        .query_row(
            "SELECT
               EXISTS(SELECT 1 FROM pragma_index_list('scan_runs')
                 WHERE name = 'scan_runs_one_active_root'
                   AND \"unique\" = 1 AND partial = 1),
               EXISTS(SELECT 1 FROM pragma_table_info('scan_runs')
                 WHERE name = 'scan_owner'),
               EXISTS(SELECT 1 FROM pragma_table_info('library_change_queue_contract')
                 WHERE name = 'scan_ownership_complete')",
            [],
            |row| {
                Ok((
                    row.get::<_, bool>(0)?,
                    row.get::<_, bool>(1)?,
                    row.get::<_, bool>(2)?,
                ))
            },
        )
        .map_err(database_error)?;
    if has_single_scan_owner_index && has_scan_owner && has_ownership_marker {
        return transaction.commit().map_err(database_error);
    }
    let has_conflicting_scan = if has_scan_runs {
        transaction
            .query_row(
                "SELECT EXISTS(
                   SELECT 1 FROM scan_runs
                   WHERE status IN ('running', 'paused')
                   GROUP BY root_id HAVING COUNT(*) > 1
                 )",
                [],
                |row| row.get::<_, bool>(0),
            )
            .map_err(database_error)?
    } else {
        false
    };
    if has_conflicting_scan {
        return Err(unverifiable_authoritative_recovery_contract());
    }
    if has_scan_runs && !has_scan_owner {
        transaction
            .execute_batch(
                "ALTER TABLE scan_runs ADD COLUMN scan_owner TEXT NOT NULL DEFAULT 'foreground'
                   CHECK(scan_owner IN ('foreground', 'authoritative_recovery'));
                 UPDATE scan_runs SET scan_owner = 'authoritative_recovery'
                 WHERE id LIKE 'sync-recovery-%';",
            )
            .map_err(database_error)?;
    }
    if !has_ownership_marker {
        transaction
            .execute_batch(
                "ALTER TABLE library_change_queue_contract
                   ADD COLUMN scan_ownership_complete INTEGER NOT NULL DEFAULT 1
                   CHECK(scan_ownership_complete = 1);",
            )
            .map_err(database_error)?;
    }
    if has_scan_runs && !has_single_scan_owner_index {
        transaction
            .execute(
                "CREATE UNIQUE INDEX scan_runs_one_active_root
                 ON scan_runs(root_id) WHERE status IN ('running', 'paused')",
                [],
            )
            .map_err(database_error)?;
    }
    transaction.commit().map_err(database_error)
}

fn validate_change_queue_authority(connection: &Connection) -> Result<(), ScanError> {
    let has_contract = connection
        .query_row(
            "SELECT EXISTS(
               SELECT 1 FROM sqlite_master
               WHERE type = 'table' AND name = 'library_change_queue_contract'
             )",
            [],
            |row| row.get::<_, bool>(0),
        )
        .map_err(database_error)?;
    if !has_contract {
        return Err(unverifiable_change_queue_authority());
    }
    let authority_complete = connection
        .query_row(
            "SELECT root_authority_complete = 1
             FROM library_change_queue_contract WHERE singleton = 1",
            [],
            |row| row.get::<_, bool>(0),
        )
        .optional()
        .map_err(database_error)?
        .unwrap_or(false);
    if !authority_complete {
        return Err(unverifiable_change_queue_authority());
    }
    Ok(())
}

fn unverifiable_change_queue_authority() -> ScanError {
    ScanError::new(
        "catalog_change_queue_authority_unverifiable",
        "This prerelease schema 17 catalog cannot prove its highest root generations and must not be opened",
    )
}

fn unverifiable_authoritative_recovery_contract() -> ScanError {
    ScanError::new(
        "catalog_authoritative_recovery_contract_unverifiable",
        "This prerelease schema 18 catalog cannot prove its authoritative recovery contract",
    )
}

fn migrate_v1_to_v2(connection: &mut Connection) -> Result<(), ScanError> {
    let transaction = connection.transaction().map_err(database_error)?;
    transaction
        .execute_batch(
            "ALTER TABLE asset_locations RENAME TO asset_locations_v1;
             CREATE TABLE assets (
               id TEXT PRIMARY KEY,
               created_unix_ms INTEGER NOT NULL
             );
             INSERT INTO assets(id, created_unix_ms)
               SELECT 'legacy:' || locations.scan_id || ':' || locations.location_id,
                      scans.started_unix_ms
               FROM asset_locations_v1 AS locations
               JOIN scan_runs AS scans ON scans.id = locations.scan_id;
             CREATE TABLE asset_locations (
               scan_id TEXT NOT NULL,
               asset_id TEXT NOT NULL,
               location_id TEXT NOT NULL,
               root_id TEXT NOT NULL,
               absolute_path TEXT NOT NULL,
               relative_path TEXT NOT NULL,
               preview_path TEXT NOT NULL,
               file_size INTEGER NOT NULL,
               modified_unix_ms INTEGER NOT NULL,
               width INTEGER NOT NULL,
               height INTEGER NOT NULL,
               PRIMARY KEY(scan_id, location_id),
               FOREIGN KEY(scan_id) REFERENCES scan_runs(id),
               FOREIGN KEY(asset_id) REFERENCES assets(id),
               FOREIGN KEY(root_id) REFERENCES library_roots(id)
             );
             INSERT INTO asset_locations(
               scan_id, asset_id, location_id, root_id, absolute_path, relative_path,
               preview_path, file_size, modified_unix_ms, width, height
             )
               SELECT scan_id, 'legacy:' || scan_id || ':' || location_id,
                      location_id, root_id, absolute_path, relative_path,
                      preview_path, file_size, modified_unix_ms, width, height
               FROM asset_locations_v1;
             DROP TABLE asset_locations_v1;
             CREATE INDEX asset_locations_active_root
               ON asset_locations(root_id, scan_id, relative_path, location_id);
             UPDATE schema_info SET version = 2;",
        )
        .map_err(database_error)?;
    transaction.commit().map_err(database_error)
}

fn migrate_v2_to_v3(connection: &mut Connection) -> Result<(), ScanError> {
    let transaction = connection.transaction().map_err(database_error)?;
    transaction
        .execute_batch(
            "CREATE TABLE catalog_state (
               revision INTEGER NOT NULL CHECK(revision >= 0)
             );
             INSERT INTO catalog_state(revision)
               SELECT COUNT(*) FROM scan_runs WHERE status = 'completed';
             UPDATE schema_info SET version = 3;",
        )
        .map_err(database_error)?;
    transaction.commit().map_err(database_error)
}

fn migrate_v3_to_v4(connection: &mut Connection) -> Result<(), ScanError> {
    let transaction = connection.transaction().map_err(database_error)?;
    transaction
        .execute_batch(
            "ALTER TABLE scan_runs ADD COLUMN max_items INTEGER;
             ALTER TABLE scan_runs ADD COLUMN max_entries INTEGER;
             ALTER TABLE scan_runs ADD COLUMN preview_edge INTEGER NOT NULL DEFAULT 512;
             ALTER TABLE scan_runs ADD COLUMN last_visited_relative_path TEXT;
             ALTER TABLE scan_runs ADD COLUMN visited_entries INTEGER NOT NULL DEFAULT 0;
             ALTER TABLE scan_runs ADD COLUMN accepted_items INTEGER NOT NULL DEFAULT 0;
             UPDATE scan_runs
               SET status = 'interrupted_unrecoverable',
                   completed_unix_ms = COALESCE(completed_unix_ms, started_unix_ms)
               WHERE status = 'running';
             CREATE UNIQUE INDEX scan_issues_identity
               ON scan_issues(scan_id, IFNULL(path, ''), code, message);
             UPDATE schema_info SET version = 4;",
        )
        .map_err(database_error)?;
    transaction.commit().map_err(database_error)
}

fn migrate_v4_to_v5(connection: &mut Connection) -> Result<(), ScanError> {
    let transaction = connection.transaction().map_err(database_error)?;
    transaction
        .execute_batch(
            "ALTER TABLE scan_runs ADD COLUMN current_directory_relative_path TEXT;
             CREATE TABLE scan_directory_frontier (
               id INTEGER PRIMARY KEY AUTOINCREMENT,
               scan_id TEXT NOT NULL,
               relative_path TEXT NOT NULL,
               FOREIGN KEY(scan_id) REFERENCES scan_runs(id),
               UNIQUE(scan_id, relative_path)
             );
             CREATE INDEX scan_directory_frontier_order
               ON scan_directory_frontier(scan_id, id);
             UPDATE scan_runs
               SET status = 'interrupted_unrecoverable',
                   completed_unix_ms = COALESCE(completed_unix_ms, started_unix_ms)
               WHERE status IN ('running', 'paused');
             UPDATE schema_info SET version = 5;",
        )
        .map_err(database_error)?;
    transaction.commit().map_err(database_error)
}

fn migrate_v5_to_v6(connection: &mut Connection) -> Result<(), ScanError> {
    let transaction = connection.transaction().map_err(database_error)?;
    transaction
        .execute_batch(
            "ALTER TABLE scan_runs ADD COLUMN current_directory_enumerated INTEGER NOT NULL
               DEFAULT 0 CHECK(current_directory_enumerated IN (0, 1));
             CREATE TABLE scan_directory_entries (
               scan_id TEXT NOT NULL,
               directory_relative_path TEXT NOT NULL,
               relative_path TEXT NOT NULL,
               PRIMARY KEY(scan_id, directory_relative_path, relative_path),
               FOREIGN KEY(scan_id) REFERENCES scan_runs(id)
             );
             UPDATE scan_runs
               SET status = 'interrupted_unrecoverable',
                   completed_unix_ms = COALESCE(completed_unix_ms, started_unix_ms),
                   current_directory_relative_path = NULL,
                   current_directory_enumerated = 0,
                   last_visited_relative_path = NULL
               WHERE status IN ('running', 'paused');
             DELETE FROM scan_directory_frontier
               WHERE scan_id IN (
                 SELECT id FROM scan_runs WHERE status = 'interrupted_unrecoverable'
               );
             UPDATE schema_info SET version = 6;",
        )
        .map_err(database_error)?;
    transaction.commit().map_err(database_error)
}

fn migrate_v6_to_v7(connection: &mut Connection) -> Result<(), ScanError> {
    let transaction = connection.transaction().map_err(database_error)?;
    transaction
        .execute_batch(
            "ALTER TABLE asset_locations ADD COLUMN preview_status TEXT NOT NULL
               DEFAULT 'ready' CHECK(preview_status IN ('pending', 'ready', 'failed'));
             ALTER TABLE asset_locations ADD COLUMN preview_issue_code TEXT;
             ALTER TABLE asset_locations ADD COLUMN preview_issue_message TEXT;
             UPDATE schema_info SET version = 7;",
        )
        .map_err(database_error)?;
    transaction.commit().map_err(database_error)
}

fn migrate_v7_to_v8(connection: &mut Connection) -> Result<(), ScanError> {
    let transaction = connection.transaction().map_err(database_error)?;
    transaction
        .execute_batch(
            "ALTER TABLE asset_locations ADD COLUMN metadata_engine_id TEXT NOT NULL
               DEFAULT 'unknown';
             ALTER TABLE asset_locations ADD COLUMN metadata_engine_version TEXT NOT NULL
               DEFAULT '0';
             ALTER TABLE asset_locations ADD COLUMN capture_local_time TEXT;
             ALTER TABLE asset_locations ADD COLUMN capture_offset_minutes INTEGER;
             ALTER TABLE asset_locations ADD COLUMN capture_time_source TEXT
               CHECK(capture_time_source IS NULL OR capture_time_source IN (
                 'exif_original', 'exif_digitized', 'exif_datetime'
               ));
             ALTER TABLE asset_locations ADD COLUMN capture_raw_value TEXT;
             UPDATE schema_info SET version = 8;",
        )
        .map_err(database_error)?;
    transaction.commit().map_err(database_error)
}

fn migrate_v8_to_v9(connection: &mut Connection) -> Result<(), ScanError> {
    let transaction = connection.transaction().map_err(database_error)?;
    transaction
        .execute_batch(
            "ALTER TABLE asset_locations ADD COLUMN file_identity_scheme TEXT;
             ALTER TABLE asset_locations ADD COLUMN file_identity_value TEXT;
             CREATE INDEX asset_locations_file_identity
               ON asset_locations(scan_id, file_identity_scheme, file_identity_value);
             UPDATE schema_info SET version = 9;",
        )
        .map_err(database_error)?;
    transaction.commit().map_err(database_error)
}

fn migrate_v9_to_v10(connection: &mut Connection) -> Result<(), ScanError> {
    let transaction = connection.transaction().map_err(database_error)?;
    transaction
        .execute_batch(
            "CREATE INDEX asset_locations_active_file_identity
               ON asset_locations(file_identity_scheme, file_identity_value, scan_id, location_id);
             CREATE INDEX asset_locations_location_id
               ON asset_locations(location_id, scan_id);
             CREATE INDEX asset_locations_asset_id
               ON asset_locations(asset_id);
             UPDATE schema_info SET version = 10;",
        )
        .map_err(database_error)?;
    transaction.commit().map_err(database_error)
}

fn migrate_v10_to_v11(connection: &mut Connection) -> Result<(), ScanError> {
    let transaction = connection.transaction().map_err(database_error)?;
    transaction
        .execute_batch(
            "CREATE INDEX asset_locations_gallery_time
               ON asset_locations(
                 (capture_local_time IS NULL), IFNULL(capture_local_time, '') DESC,
                 modified_unix_ms DESC, root_id, location_id, scan_id
               );
             UPDATE schema_info SET version = 11;",
        )
        .map_err(database_error)?;
    transaction.commit().map_err(database_error)
}

fn migrate_v11_to_v12(connection: &mut Connection) -> Result<(), ScanError> {
    let transaction = connection.transaction().map_err(database_error)?;
    transaction
        .execute_batch(
            "ALTER TABLE asset_locations ADD COLUMN created_unix_ms INTEGER;
             ALTER TABLE asset_locations ADD COLUMN parent_relative_path TEXT NOT NULL DEFAULT '';
             ALTER TABLE asset_locations ADD COLUMN natural_name_key TEXT NOT NULL DEFAULT '';",
        )
        .map_err(database_error)?;
    let locations = {
        let mut statement = transaction
            .prepare("SELECT rowid, relative_path FROM asset_locations")
            .map_err(database_error)?;
        let rows = statement
            .query_map([], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(database_error)?;
        let mut locations = Vec::new();
        for row in rows {
            locations.push(row.map_err(database_error)?);
        }
        locations
    };
    for (row_id, relative_path) in locations {
        transaction
            .execute(
                "UPDATE asset_locations
                 SET parent_relative_path = ?1, natural_name_key = ?2
                 WHERE rowid = ?3",
                params![
                    parent_relative_path(&relative_path),
                    natural_name_key(&relative_path),
                    row_id,
                ],
            )
            .map_err(database_error)?;
    }
    transaction
        .execute_batch(
            "CREATE INDEX asset_locations_gallery_created
               ON asset_locations(
                 (created_unix_ms IS NULL), IFNULL(created_unix_ms, 0),
                 root_id, location_id, scan_id
               );
             CREATE INDEX asset_locations_gallery_modified
               ON asset_locations(modified_unix_ms, root_id, location_id, scan_id);
             CREATE INDEX asset_locations_gallery_name
               ON asset_locations(natural_name_key, root_id, location_id, scan_id);
             CREATE INDEX asset_locations_parent_folder
               ON asset_locations(root_id, parent_relative_path, scan_id, location_id);
             UPDATE schema_info SET version = 12;",
        )
        .map_err(database_error)?;
    transaction.commit().map_err(database_error)
}

fn migrate_v12_to_v13(connection: &mut Connection) -> Result<(), ScanError> {
    let transaction = connection.transaction().map_err(database_error)?;
    transaction
        .execute_batch(
            "ALTER TABLE asset_locations ADD COLUMN file_local_time TEXT;
             UPDATE asset_locations
               SET file_local_time = strftime(
                 '%Y-%m-%dT%H:%M:%f',
                 COALESCE(created_unix_ms, modified_unix_ms) / 1000.0,
                 'unixepoch', 'localtime'
               );
             DROP INDEX asset_locations_gallery_time;
             DROP INDEX asset_locations_gallery_created;
             CREATE INDEX asset_locations_gallery_time
               ON asset_locations(
                 (COALESCE(capture_local_time, file_local_time) IS NULL),
                 IFNULL(COALESCE(capture_local_time, file_local_time), '') DESC,
                 modified_unix_ms DESC, root_id, location_id, scan_id
               );
             CREATE INDEX asset_locations_gallery_created
               ON asset_locations(
                 (file_local_time IS NULL), IFNULL(file_local_time, ''),
                 modified_unix_ms, root_id, location_id, scan_id
               );
             UPDATE schema_info SET version = 13;",
        )
        .map_err(database_error)?;
    transaction.commit().map_err(database_error)
}

fn migrate_v13_to_v14(connection: &mut Connection) -> Result<(), ScanError> {
    let transaction = connection.transaction().map_err(database_error)?;
    transaction
        .execute_batch(
            "CREATE TABLE preview_artifacts (
               artifact_key TEXT PRIMARY KEY,
               location_id TEXT NOT NULL,
               source_file_size INTEGER NOT NULL CHECK(source_file_size >= 0),
               source_modified_unix_ms INTEGER NOT NULL,
               source_identity_scheme TEXT,
               source_identity_value TEXT,
               algorithm_id TEXT NOT NULL,
               algorithm_version INTEGER NOT NULL CHECK(algorithm_version >= 0),
               orientation_contract TEXT NOT NULL,
               size_bucket INTEGER NOT NULL CHECK(size_bucket > 0),
               encoded_width INTEGER NOT NULL CHECK(encoded_width > 0),
               encoded_height INTEGER NOT NULL CHECK(encoded_height > 0),
               artifact_path TEXT NOT NULL UNIQUE,
               byte_size INTEGER NOT NULL CHECK(byte_size >= 0),
               lifecycle_state TEXT NOT NULL
                 CHECK(lifecycle_state IN ('ready', 'stale', 'evictable')),
               created_unix_ms INTEGER NOT NULL,
               last_used_unix_ms INTEGER NOT NULL,
               CHECK(
                 (source_identity_scheme IS NULL AND source_identity_value IS NULL)
                 OR
                 (source_identity_scheme IS NOT NULL AND source_identity_value IS NOT NULL)
               )
             );
             CREATE INDEX preview_artifacts_location
               ON preview_artifacts(location_id, size_bucket, lifecycle_state);
             CREATE INDEX preview_artifacts_reclamation
               ON preview_artifacts(lifecycle_state, last_used_unix_ms, artifact_key);
             CREATE INDEX preview_artifacts_compatibility
               ON preview_artifacts(
                 location_id, source_file_size, source_modified_unix_ms,
                 algorithm_id, algorithm_version, orientation_contract, size_bucket
               );
             UPDATE schema_info SET version = 14;",
        )
        .map_err(database_error)?;
    transaction.commit().map_err(database_error)
}

fn migrate_v14_to_v15(connection: &mut Connection) -> Result<(), ScanError> {
    let transaction = connection.transaction().map_err(database_error)?;
    let has_preview_path: bool = transaction
        .query_row(
            "SELECT EXISTS(
               SELECT 1 FROM pragma_table_info('asset_locations')
               WHERE name = 'preview_path'
             )",
            [],
            |row| row.get(0),
        )
        .map_err(database_error)?;
    transaction
        .execute_batch(
            "ALTER TABLE preview_artifacts RENAME TO preview_artifacts_v14;
             DROP INDEX preview_artifacts_location;
             DROP INDEX preview_artifacts_reclamation;
             DROP INDEX preview_artifacts_compatibility;
             CREATE TABLE preview_artifacts (
               artifact_key TEXT PRIMARY KEY,
               source_file_size INTEGER NOT NULL CHECK(source_file_size >= 0),
               source_modified_unix_ms INTEGER NOT NULL,
               source_identity_scheme TEXT,
               source_identity_value TEXT,
               algorithm_id TEXT NOT NULL,
               algorithm_version INTEGER NOT NULL CHECK(algorithm_version >= 0),
               orientation_contract TEXT NOT NULL,
               size_bucket INTEGER NOT NULL CHECK(size_bucket > 0),
               encoded_width INTEGER NOT NULL CHECK(encoded_width > 0),
               encoded_height INTEGER NOT NULL CHECK(encoded_height > 0),
               artifact_path TEXT NOT NULL UNIQUE,
               byte_size INTEGER NOT NULL CHECK(byte_size >= 0),
               lifecycle_state TEXT NOT NULL
                 CHECK(lifecycle_state IN ('ready', 'stale', 'evictable')),
               created_unix_ms INTEGER NOT NULL,
               last_used_unix_ms INTEGER NOT NULL,
               CHECK(
                 (source_identity_scheme IS NULL AND source_identity_value IS NULL)
                 OR
                 (source_identity_scheme IS NOT NULL AND source_identity_value IS NOT NULL)
               )
             );
             CREATE TABLE preview_artifact_locations (
               artifact_key TEXT NOT NULL,
               location_id TEXT NOT NULL,
               PRIMARY KEY(artifact_key, location_id),
               FOREIGN KEY(artifact_key) REFERENCES preview_artifacts(artifact_key)
                 ON DELETE CASCADE
             );
             INSERT INTO preview_artifacts(
               artifact_key, source_file_size, source_modified_unix_ms,
               source_identity_scheme, source_identity_value, algorithm_id,
               algorithm_version, orientation_contract, size_bucket, encoded_width,
               encoded_height, artifact_path, byte_size, lifecycle_state,
               created_unix_ms, last_used_unix_ms
             )
             SELECT artifacts.artifact_key, artifacts.source_file_size,
                    artifacts.source_modified_unix_ms,
                    artifacts.source_identity_scheme, artifacts.source_identity_value,
                    artifacts.algorithm_id, artifacts.algorithm_version,
                    artifacts.orientation_contract, artifacts.size_bucket,
                    artifacts.encoded_width, artifacts.encoded_height,
                    artifacts.artifact_path, artifacts.byte_size,
                    artifacts.lifecycle_state, artifacts.created_unix_ms,
                    artifacts.last_used_unix_ms
             FROM preview_artifacts_v14 AS artifacts;
             DROP TABLE preview_artifacts_v14;
             CREATE INDEX preview_artifact_locations_location
               ON preview_artifact_locations(location_id, artifact_key);
             CREATE INDEX preview_artifacts_reclamation
               ON preview_artifacts(lifecycle_state, last_used_unix_ms, artifact_key);
             CREATE INDEX preview_artifacts_compatibility
               ON preview_artifacts(
                 source_file_size, source_modified_unix_ms,
                 algorithm_id, algorithm_version, orientation_contract, size_bucket
               );
             UPDATE schema_info SET version = 15;",
        )
        .map_err(database_error)?;
    if has_preview_path {
        transaction
            .execute(
                "INSERT INTO preview_artifact_locations(artifact_key, location_id)
                 SELECT DISTINCT artifacts.artifact_key, locations.location_id
                 FROM preview_artifacts AS artifacts
                 JOIN asset_locations AS locations
                   ON locations.preview_path = artifacts.artifact_path",
                [],
            )
            .map_err(database_error)?;
    }
    transaction.commit().map_err(database_error)
}

fn migrate_v15_to_v16(connection: &mut Connection) -> Result<(), ScanError> {
    let transaction = connection.transaction().map_err(database_error)?;
    let has_library_roots: bool = transaction
        .query_row(
            "SELECT EXISTS(
               SELECT 1 FROM sqlite_master
               WHERE type = 'table' AND name = 'library_roots'
             )",
            [],
            |row| row.get(0),
        )
        .map_err(database_error)?;
    let location_columns = [
        "scan_id",
        "location_id",
        "root_id",
        "preview_path",
        "preview_status",
    ];
    let can_reconcile_active_ownership = has_library_roots
        && location_columns
            .iter()
            .try_fold(true, |all_present, column| {
                transaction
                    .query_row(
                        "SELECT EXISTS(
                       SELECT 1 FROM pragma_table_info('asset_locations')
                       WHERE name = ?1
                     )",
                        [column],
                        |row| row.get::<_, bool>(0),
                    )
                    .map(|present| all_present && present)
                    .map_err(database_error)
            })?;
    if can_reconcile_active_ownership {
        transaction
            .execute(
                "DELETE FROM preview_artifact_locations
                 WHERE NOT EXISTS (
                   SELECT 1
                   FROM preview_artifacts AS artifacts
                   JOIN asset_locations AS locations
                     ON locations.location_id = preview_artifact_locations.location_id
                    AND locations.preview_path = artifacts.artifact_path
                    AND locations.preview_status = 'ready'
                   JOIN library_roots AS roots
                     ON roots.id = locations.root_id
                    AND roots.active_scan_id = locations.scan_id
                   WHERE artifacts.artifact_key = preview_artifact_locations.artifact_key
                 )",
                [],
            )
            .map_err(database_error)?;
    }
    transaction
        .execute_batch(
            "UPDATE preview_artifacts
             SET lifecycle_state = 'stale'
             WHERE lifecycle_state = 'ready'
               AND NOT EXISTS (
                 SELECT 1 FROM preview_artifact_locations AS owners
                 WHERE owners.artifact_key = preview_artifacts.artifact_key
               );
             UPDATE schema_info SET version = 16;",
        )
        .map_err(database_error)?;
    transaction.commit().map_err(database_error)
}

fn migrate_v16_to_v17(connection: &mut Connection) -> Result<(), ScanError> {
    let transaction = connection.transaction().map_err(database_error)?;
    create_library_change_queue_schema(&transaction)?;
    transaction
        .execute("UPDATE schema_info SET version = 17", [])
        .map_err(database_error)?;
    transaction.commit().map_err(database_error)
}

fn migrate_v17_to_v18(connection: &mut Connection) -> Result<(), ScanError> {
    let transaction = connection.transaction().map_err(database_error)?;
    let has_scan_runs = transaction
        .query_row(
            "SELECT EXISTS(
               SELECT 1 FROM sqlite_master
               WHERE type = 'table' AND name = 'scan_runs'
             )",
            [],
            |row| row.get::<_, bool>(0),
        )
        .map_err(database_error)?;
    transaction
        .execute(
            "ALTER TABLE library_change_queue
             ADD COLUMN authoritative_scan_id TEXT",
            [],
        )
        .map_err(database_error)?;
    transaction
        .execute(
            "ALTER TABLE library_change_root_state
             ADD COLUMN last_consistency_audit_unix_ms INTEGER",
            [],
        )
        .map_err(database_error)?;
    add_authoritative_recovery_contract_marker(&transaction)?;
    if has_scan_runs {
        transaction
            .execute_batch(
                "ALTER TABLE scan_runs ADD COLUMN root_generation_at_start INTEGER
                   CHECK(root_generation_at_start IS NULL OR root_generation_at_start > 0);
                 ALTER TABLE scan_runs ADD COLUMN change_queue_high_watermark INTEGER
                   CHECK(change_queue_high_watermark IS NULL OR change_queue_high_watermark > 0);
                 ALTER TABLE scan_runs ADD COLUMN requires_previous_snapshot INTEGER NOT NULL
                   DEFAULT 0 CHECK(requires_previous_snapshot IN (0, 1));
                 ALTER TABLE scan_runs ADD COLUMN scan_owner TEXT NOT NULL DEFAULT 'foreground'
                   CHECK(scan_owner IN ('foreground', 'authoritative_recovery'));
                 UPDATE scan_runs
                 SET status = 'interrupted_unrecoverable',
                     completed_unix_ms = COALESCE(completed_unix_ms, started_unix_ms),
                     current_directory_relative_path = NULL,
                     current_directory_enumerated = 0,
                     last_visited_relative_path = NULL
                 WHERE status IN ('running', 'paused');
                 CREATE UNIQUE INDEX scan_runs_one_active_root
                   ON scan_runs(root_id) WHERE status IN ('running', 'paused');",
            )
            .map_err(database_error)?;
    }
    normalize_relative_paths_for_continuous_synchronization(&transaction)?;
    transaction
        .execute("UPDATE schema_info SET version = 18", [])
        .map_err(database_error)?;
    transaction.commit().map_err(database_error)
}

fn migrate_v18_to_v19(connection: &mut Connection) -> Result<(), ScanError> {
    let transaction = connection.transaction().map_err(database_error)?;
    add_change_catch_up_contract(&transaction)?;
    add_preview_expectation_repair_marker(&transaction)?;
    transaction
        .execute("UPDATE schema_info SET version = 19", [])
        .map_err(database_error)?;
    transaction.commit().map_err(database_error)
}

fn migrate_v19_to_v20(connection: &mut Connection) -> Result<(), ScanError> {
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(database_error)?;
    migrate_v19_to_v20_transaction(&transaction)?;
    transaction.commit().map_err(database_error)
}

fn migrate_v19_to_v20_transaction(transaction: &Transaction<'_>) -> Result<(), ScanError> {
    rebuild_library_change_queue_for_metadata_inventory(transaction)?;
    create_metadata_inventory_contract(transaction)?;
    transaction
        .execute("UPDATE schema_info SET version = 20", [])
        .map_err(database_error)?;
    Ok(())
}

fn migrate_v20_to_v21(connection: &mut Connection) -> Result<(), ScanError> {
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(database_error)?;
    migrate_v20_to_v21_transaction(&transaction)?;
    transaction.commit().map_err(database_error)
}

fn migrate_v20_to_v21_transaction(transaction: &Transaction<'_>) -> Result<(), ScanError> {
    transaction
        .execute_batch(
            "CREATE TABLE library_terminal_media_evidence_contract (
               singleton INTEGER PRIMARY KEY CHECK(singleton = 1),
               complete INTEGER NOT NULL CHECK(complete = 1)
             );
             INSERT INTO library_terminal_media_evidence_contract(singleton, complete)
               VALUES (1, 1);
             CREATE TABLE library_terminal_media_evidence (
               root_id TEXT NOT NULL,
               relative_path TEXT NOT NULL,
               file_size INTEGER NOT NULL CHECK(file_size >= 0),
               modified_unix_ms INTEGER NOT NULL,
               file_identity_scheme TEXT,
               file_identity_value TEXT,
               inspection_engine_id TEXT NOT NULL,
               inspection_engine_version INTEGER NOT NULL
                 CHECK(inspection_engine_version > 0),
               issue_code TEXT NOT NULL,
               issue_message TEXT NOT NULL,
               updated_unix_ms INTEGER NOT NULL,
               CHECK(length(relative_path) > 0),
               CHECK(instr(relative_path, char(92)) = 0),
               CHECK(length(inspection_engine_id) BETWEEN 1 AND 128),
               CHECK(length(issue_code) BETWEEN 1 AND 128),
               CHECK(length(issue_message) BETWEEN 1 AND 4096),
               CHECK(
                 (file_identity_scheme IS NULL AND file_identity_value IS NULL)
                 OR
                 (file_identity_scheme IS NOT NULL AND file_identity_value IS NOT NULL)
               ),
               PRIMARY KEY(root_id, relative_path),
               FOREIGN KEY(root_id) REFERENCES library_roots(id) ON DELETE CASCADE
             );",
        )
        .map_err(database_error)?;
    transaction
        .execute("UPDATE schema_info SET version = 21", [])
        .map_err(database_error)?;
    Ok(())
}

fn migrate_v21_to_v22(connection: &mut Connection) -> Result<(), ScanError> {
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(database_error)?;
    migrate_v21_to_v22_transaction(&transaction)?;
    transaction.commit().map_err(database_error)
}

fn migrate_v21_to_v22_transaction(transaction: &Transaction<'_>) -> Result<(), ScanError> {
    transaction
        .execute_batch(
            "CREATE UNIQUE INDEX library_change_root_state_generation_identity
               ON library_change_root_state(root_id, generation);
             CREATE TABLE library_persistent_journal_contract (
               singleton INTEGER PRIMARY KEY CHECK(singleton = 1),
               contract_version INTEGER NOT NULL CHECK(contract_version = 1),
               complete INTEGER NOT NULL CHECK(complete = 1)
             );
             INSERT INTO library_persistent_journal_contract(
               singleton, contract_version, complete
             ) VALUES (1, 1, 1);
             CREATE TABLE library_persistent_journal_root_state (
               root_id TEXT NOT NULL,
               root_generation INTEGER NOT NULL CHECK(root_generation > 0),
               protocol_version INTEGER NOT NULL
                 CHECK(protocol_version BETWEEN 0 AND 65535),
               contract_version INTEGER NOT NULL CHECK(contract_version = 1),
               capability_state TEXT NOT NULL CHECK(capability_state IN (
                 'unknown', 'supported', 'live_only'
               )),
               continuity_state TEXT NOT NULL CHECK(continuity_state IN (
                 'baseline_required', 'catching_up', 'current', 'recovery_required',
                 'live_only', 'unavailable'
               )),
               last_failure_code TEXT,
               last_failure_message TEXT,
               updated_unix_ms INTEGER NOT NULL CHECK(updated_unix_ms >= 0),
               CHECK(
                 (last_failure_code IS NULL AND last_failure_message IS NULL)
                 OR
                 (last_failure_code IS NOT NULL AND last_failure_message IS NOT NULL)
               ),
               CHECK(
                 (capability_state = 'unknown' AND protocol_version = 0)
                 OR
                 (capability_state <> 'unknown' AND protocol_version > 0)
               ),
               CHECK(capability_state <> 'supported' OR last_failure_code IS NULL),
               CHECK(
                 continuity_state <> 'current'
                 OR (capability_state = 'supported' AND last_failure_code IS NULL)
               ),
               PRIMARY KEY(root_id, root_generation),
               FOREIGN KEY(root_id) REFERENCES library_roots(id) ON DELETE CASCADE
             );
             CREATE INDEX library_persistent_journal_root_state_continuity
               ON library_persistent_journal_root_state(
                 continuity_state, capability_state, updated_unix_ms, root_id
               );
             CREATE TABLE library_persistent_journal_checkpoints (
               root_id TEXT PRIMARY KEY,
               root_generation INTEGER NOT NULL CHECK(root_generation > 0),
               volume_guid TEXT NOT NULL CHECK(length(volume_guid) BETWEEN 1 AND 512),
               volume_serial TEXT NOT NULL CHECK(length(volume_serial) BETWEEN 1 AND 20),
               root_reference_version INTEGER NOT NULL
                 CHECK(root_reference_version IN (2, 3)),
               root_file_reference BLOB NOT NULL,
               journal_id TEXT NOT NULL CHECK(length(journal_id) BETWEEN 1 AND 20),
               next_unread_usn TEXT NOT NULL CHECK(length(next_unread_usn) BETWEEN 1 AND 19),
               captured_exclusive_end TEXT NOT NULL
                 CHECK(length(captured_exclusive_end) BETWEEN 1 AND 19),
               covered_catalog_revision INTEGER NOT NULL
                 CHECK(covered_catalog_revision >= 0),
               protocol_version INTEGER NOT NULL CHECK(protocol_version BETWEEN 1 AND 65535),
               contract_version INTEGER NOT NULL CHECK(contract_version = 1),
               continuity_state TEXT NOT NULL CHECK(continuity_state IN (
                 'catching_up', 'current', 'recovery_required', 'live_only', 'unavailable'
               )),
               last_failure_code TEXT,
               last_failure_message TEXT,
               updated_unix_ms INTEGER NOT NULL CHECK(updated_unix_ms >= 0),
               CHECK(
                 (root_reference_version = 2 AND length(root_file_reference) = 8)
                 OR
                 (root_reference_version = 3 AND length(root_file_reference) = 16)
               ),
               CHECK(journal_id <> '0' AND journal_id NOT GLOB '*[^0-9]*'),
               CHECK(next_unread_usn NOT GLOB '*[^0-9]*'),
               CHECK(captured_exclusive_end NOT GLOB '*[^0-9]*'),
               CHECK(CAST(next_unread_usn AS INTEGER) <= CAST(captured_exclusive_end AS INTEGER)),
               CHECK(
                 (last_failure_code IS NULL AND last_failure_message IS NULL)
                 OR
                 (last_failure_code IS NOT NULL AND last_failure_message IS NOT NULL)
               ),
               CHECK(
                 continuity_state <> 'current'
                 OR (
                   next_unread_usn = captured_exclusive_end
                   AND last_failure_code IS NULL
                 )
               ),
               FOREIGN KEY(root_id, root_generation)
                 REFERENCES library_persistent_journal_root_state(root_id, root_generation)
                 ON DELETE CASCADE
             );
             CREATE INDEX library_persistent_journal_checkpoints_volume
               ON library_persistent_journal_checkpoints(
                 volume_guid, journal_id, continuity_state, root_id
               );
             CREATE TABLE library_persistent_journal_source_ranges (
               id TEXT PRIMARY KEY CHECK(length(id) BETWEEN 1 AND 512),
               root_id TEXT NOT NULL,
               root_generation INTEGER NOT NULL CHECK(root_generation > 0),
               volume_guid TEXT NOT NULL CHECK(length(volume_guid) BETWEEN 1 AND 512),
               volume_serial TEXT NOT NULL CHECK(length(volume_serial) BETWEEN 1 AND 20),
               journal_id TEXT NOT NULL CHECK(length(journal_id) BETWEEN 1 AND 20),
               requested_start_usn TEXT NOT NULL
                 CHECK(length(requested_start_usn) BETWEEN 1 AND 19),
               requested_end_usn TEXT NOT NULL
                 CHECK(length(requested_end_usn) BETWEEN 1 AND 19),
               covered_until_usn TEXT NOT NULL
                 CHECK(length(covered_until_usn) BETWEEN 1 AND 19),
               is_complete INTEGER NOT NULL CHECK(is_complete IN (0, 1)),
               protocol_version INTEGER NOT NULL CHECK(protocol_version BETWEEN 1 AND 65535),
               contract_version INTEGER NOT NULL CHECK(contract_version = 1),
               status TEXT NOT NULL CHECK(status IN (
                 'enrolled', 'checkpointed', 'superseded'
               )),
               enrolled_unix_ms INTEGER NOT NULL CHECK(enrolled_unix_ms >= 0),
               checkpointed_unix_ms INTEGER CHECK(checkpointed_unix_ms >= 0),
               CHECK(journal_id <> '0' AND journal_id NOT GLOB '*[^0-9]*'),
               CHECK(requested_start_usn NOT GLOB '*[^0-9]*'),
               CHECK(requested_end_usn NOT GLOB '*[^0-9]*'),
               CHECK(covered_until_usn NOT GLOB '*[^0-9]*'),
               CHECK(
                 CAST(requested_start_usn AS INTEGER) < CAST(requested_end_usn AS INTEGER)
                 AND CAST(requested_start_usn AS INTEGER) < CAST(covered_until_usn AS INTEGER)
                 AND CAST(covered_until_usn AS INTEGER) <= CAST(requested_end_usn AS INTEGER)
               ),
               CHECK(
                 is_complete = (
                   CAST(covered_until_usn AS INTEGER) = CAST(requested_end_usn AS INTEGER)
                 )
               ),
               CHECK(
                 (status = 'checkpointed' AND checkpointed_unix_ms IS NOT NULL)
                 OR
                 (status <> 'checkpointed' AND checkpointed_unix_ms IS NULL)
               ),
               FOREIGN KEY(root_id, root_generation)
                 REFERENCES library_persistent_journal_root_state(root_id, root_generation)
                 ON DELETE CASCADE
             );
             CREATE INDEX library_persistent_journal_source_ranges_root
               ON library_persistent_journal_source_ranges(
                 root_id, root_generation, status, enrolled_unix_ms, id
               );
             CREATE INDEX library_persistent_journal_source_ranges_volume
               ON library_persistent_journal_source_ranges(
                 volume_guid, journal_id, requested_start_usn, id
               );
             CREATE TABLE library_persistent_journal_queue_lineage (
               source_range_id TEXT NOT NULL,
               change_id INTEGER NOT NULL,
               enrolled_unix_ms INTEGER NOT NULL CHECK(enrolled_unix_ms >= 0),
               PRIMARY KEY(source_range_id, change_id),
               FOREIGN KEY(source_range_id)
                 REFERENCES library_persistent_journal_source_ranges(id) ON DELETE CASCADE,
               FOREIGN KEY(change_id) REFERENCES library_change_queue(id) ON DELETE CASCADE
             );
             CREATE INDEX library_persistent_journal_queue_lineage_change
               ON library_persistent_journal_queue_lineage(change_id, source_range_id);
             CREATE TABLE library_persistent_journal_cross_root_lineage (
               id TEXT PRIMARY KEY CHECK(length(id) BETWEEN 1 AND 512),
               volume_guid TEXT NOT NULL CHECK(length(volume_guid) BETWEEN 1 AND 512),
               volume_serial TEXT NOT NULL CHECK(length(volume_serial) BETWEEN 1 AND 20),
               file_reference_version INTEGER NOT NULL
                 CHECK(file_reference_version IN (2, 3)),
               file_reference BLOB NOT NULL,
               previous_root_id TEXT NOT NULL,
               previous_root_generation INTEGER NOT NULL
                 CHECK(previous_root_generation > 0),
               previous_relative_path TEXT NOT NULL,
               current_root_id TEXT NOT NULL,
               current_root_generation INTEGER NOT NULL CHECK(current_root_generation > 0),
               current_relative_path TEXT NOT NULL,
               status TEXT NOT NULL CHECK(status IN ('pending', 'completed', 'superseded')),
               created_unix_ms INTEGER NOT NULL CHECK(created_unix_ms >= 0),
               updated_unix_ms INTEGER NOT NULL CHECK(updated_unix_ms >= 0),
               CHECK(previous_root_id <> current_root_id),
               CHECK(length(previous_relative_path) BETWEEN 1 AND 32767),
               CHECK(length(current_relative_path) BETWEEN 1 AND 32767),
               CHECK(instr(previous_relative_path, char(92)) = 0),
               CHECK(instr(current_relative_path, char(92)) = 0),
               CHECK(
                 (file_reference_version = 2 AND length(file_reference) = 8)
                 OR
                 (file_reference_version = 3 AND length(file_reference) = 16)
               ),
               FOREIGN KEY(previous_root_id, previous_root_generation)
                 REFERENCES library_persistent_journal_root_state(root_id, root_generation),
               FOREIGN KEY(current_root_id, current_root_generation)
                 REFERENCES library_persistent_journal_root_state(root_id, root_generation)
             );
             CREATE INDEX library_persistent_journal_cross_root_lineage_previous
               ON library_persistent_journal_cross_root_lineage(
                 previous_root_id, previous_root_generation, status, id
               );
             CREATE INDEX library_persistent_journal_cross_root_lineage_current
               ON library_persistent_journal_cross_root_lineage(
                 current_root_id, current_root_generation, status, id
               );
             CREATE TABLE library_persistent_journal_cross_root_ranges (
               lineage_id TEXT NOT NULL,
               source_range_id TEXT NOT NULL,
               participant_role TEXT NOT NULL CHECK(participant_role IN ('previous', 'current')),
               enrolled_unix_ms INTEGER NOT NULL CHECK(enrolled_unix_ms >= 0),
               PRIMARY KEY(lineage_id, source_range_id),
               FOREIGN KEY(lineage_id)
                 REFERENCES library_persistent_journal_cross_root_lineage(id) ON DELETE CASCADE,
               FOREIGN KEY(source_range_id)
                 REFERENCES library_persistent_journal_source_ranges(id) ON DELETE CASCADE
             );
             CREATE INDEX library_persistent_journal_cross_root_ranges_source
               ON library_persistent_journal_cross_root_ranges(source_range_id, lineage_id);",
        )
        .map_err(database_error)?;
    normalize_legacy_metadata_inventory_runs(transaction)?;
    transaction
        .execute(
            "INSERT INTO library_persistent_journal_root_state(
               root_id, root_generation, protocol_version, contract_version,
               capability_state, continuity_state, last_failure_code,
               last_failure_message, updated_unix_ms
             )
             SELECT roots.id, state.generation, 0, 1, 'unknown', 'baseline_required',
                    NULL, NULL, 0
             FROM library_roots AS roots
             JOIN library_change_root_state AS state ON state.root_id = roots.id
             WHERE state.is_active = 1",
            [],
        )
        .map_err(database_error)?;
    transaction
        .execute("UPDATE schema_info SET version = 22", [])
        .map_err(database_error)?;
    Ok(())
}

fn migrate_v22_to_v23(connection: &mut Connection) -> Result<(), ScanError> {
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(database_error)?;
    migrate_v22_to_v23_transaction(&transaction)?;
    transaction.commit().map_err(database_error)
}

fn migrate_v22_to_v23_transaction(transaction: &Transaction<'_>) -> Result<(), ScanError> {
    transaction
        .execute_batch(
            "CREATE TABLE library_persistent_journal_range_lifecycle (
               source_range_id TEXT PRIMARY KEY,
               lifecycle_state TEXT NOT NULL
                 CHECK(lifecycle_state IN ('pending', 'completed', 'superseded')),
               completed_unix_ms INTEGER CHECK(completed_unix_ms >= 0),
               updated_unix_ms INTEGER NOT NULL CHECK(updated_unix_ms >= 0),
               CHECK(
                 (lifecycle_state = 'completed' AND completed_unix_ms IS NOT NULL)
                 OR
                 (lifecycle_state <> 'completed' AND completed_unix_ms IS NULL)
               ),
               FOREIGN KEY(source_range_id)
                 REFERENCES library_persistent_journal_source_ranges(id) ON DELETE CASCADE
             );
             CREATE INDEX library_persistent_journal_range_lifecycle_state
               ON library_persistent_journal_range_lifecycle(
                 lifecycle_state, updated_unix_ms, source_range_id
               );
             INSERT INTO library_persistent_journal_range_lifecycle(
               source_range_id, lifecycle_state, completed_unix_ms, updated_unix_ms
             )
             SELECT id,
                    CASE status WHEN 'superseded' THEN 'superseded' ELSE 'pending' END,
                    NULL,
                    COALESCE(checkpointed_unix_ms, enrolled_unix_ms)
             FROM library_persistent_journal_source_ranges;
             CREATE TABLE library_persistent_journal_pending_renames (
               carry_id TEXT PRIMARY KEY CHECK(length(carry_id) BETWEEN 1 AND 512),
               source_range_id TEXT NOT NULL,
               volume_guid TEXT NOT NULL CHECK(length(volume_guid) BETWEEN 1 AND 512),
               volume_serial TEXT NOT NULL CHECK(length(volume_serial) BETWEEN 1 AND 20),
               journal_id TEXT NOT NULL CHECK(length(journal_id) BETWEEN 1 AND 20),
               file_reference_version INTEGER NOT NULL
                 CHECK(file_reference_version IN (2, 3)),
               file_reference BLOB NOT NULL,
               old_usn TEXT NOT NULL CHECK(length(old_usn) BETWEEN 1 AND 19),
               previous_root_id TEXT NOT NULL,
               previous_root_generation INTEGER NOT NULL CHECK(previous_root_generation > 0),
               previous_relative_path TEXT NOT NULL,
               is_directory INTEGER NOT NULL CHECK(is_directory IN (0, 1)),
               enrolled_unix_ms INTEGER NOT NULL CHECK(enrolled_unix_ms >= 0),
               CHECK(journal_id <> '0' AND journal_id NOT GLOB '*[^0-9]*'),
               CHECK(old_usn NOT GLOB '*[^0-9]*'),
               CHECK(length(previous_relative_path) BETWEEN 1 AND 32767),
               CHECK(instr(previous_relative_path, char(92)) = 0),
               CHECK(
                 (file_reference_version = 2 AND length(file_reference) = 8)
                 OR
                 (file_reference_version = 3 AND length(file_reference) = 16)
               ),
               FOREIGN KEY(source_range_id)
                 REFERENCES library_persistent_journal_source_ranges(id) ON DELETE CASCADE,
               FOREIGN KEY(previous_root_id, previous_root_generation)
                 REFERENCES library_persistent_journal_root_state(root_id, root_generation)
             );
             CREATE INDEX library_persistent_journal_pending_renames_volume
               ON library_persistent_journal_pending_renames(
                 volume_guid, journal_id, old_usn, carry_id
               );
             CREATE INDEX library_persistent_journal_pending_renames_source
               ON library_persistent_journal_pending_renames(source_range_id, carry_id);",
        )
        .map_err(database_error)?;
    transaction
        .execute("UPDATE schema_info SET version = 23", [])
        .map_err(database_error)?;
    Ok(())
}

fn migrate_v23_to_v24(connection: &mut Connection) -> Result<(), ScanError> {
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(database_error)?;
    migrate_v23_to_v24_transaction(&transaction)?;
    transaction.commit().map_err(database_error)
}

fn migrate_v23_to_v24_transaction(transaction: &Transaction<'_>) -> Result<(), ScanError> {
    let has_unprovable_lineage = transaction
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM library_persistent_journal_cross_root_lineage)",
            [],
            |row| row.get::<_, bool>(0),
        )
        .map_err(database_error)?;
    if has_unprovable_lineage {
        return Err(ScanError::new(
            "persistent_journal_v23_lineage_unverifiable",
            "The v23 catalog cannot prove the consumed-carry identity required by v24",
        ));
    }
    transaction
        .execute_batch(
            "ALTER TABLE library_persistent_journal_source_ranges
               ADD COLUMN canonical_payload BLOB NOT NULL DEFAULT X'';
             ALTER TABLE library_persistent_journal_cross_root_lineage
               ADD COLUMN journal_id TEXT;
             ALTER TABLE library_persistent_journal_cross_root_lineage
               ADD COLUMN old_usn TEXT;
             ALTER TABLE library_persistent_journal_cross_root_lineage
               ADD COLUMN new_usn TEXT;
             ALTER TABLE library_persistent_journal_cross_root_lineage
               ADD COLUMN previous_carry_id TEXT;",
        )
        .map_err(database_error)?;

    let range_ids = {
        let mut statement = transaction
            .prepare("SELECT id FROM library_persistent_journal_source_ranges ORDER BY id")
            .map_err(database_error)?;
        let rows = statement
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(database_error)?;
        let mut ids = Vec::new();
        for row in rows {
            ids.push(row.map_err(database_error)?);
        }
        ids
    };
    for old_id in range_ids {
        let payload = legacy_v23_range_payload(transaction, &old_id)?;
        let new_id = persistent_journal_batch_id_from_payload(&payload);
        let collision = transaction
            .query_row(
                "SELECT EXISTS(
                   SELECT 1 FROM library_persistent_journal_source_ranges
                   WHERE id = ?1 AND id <> ?2
                 )",
                params![new_id, old_id],
                |row| row.get::<_, bool>(0),
            )
            .map_err(database_error)?;
        if collision {
            return Err(unverifiable_persistent_journal_contract());
        }
        if new_id == old_id {
            transaction
                .execute(
                    "UPDATE library_persistent_journal_source_ranges
                     SET canonical_payload = ?1 WHERE id = ?2",
                    params![payload, old_id],
                )
                .map_err(database_error)?;
            continue;
        }
        transaction
            .execute(
                "INSERT INTO library_persistent_journal_source_ranges(
                   id, root_id, root_generation, volume_guid, volume_serial, journal_id,
                   requested_start_usn, requested_end_usn, covered_until_usn, is_complete,
                   protocol_version, contract_version, status, enrolled_unix_ms,
                   checkpointed_unix_ms, canonical_payload
                 )
                 SELECT ?1, root_id, root_generation, volume_guid, volume_serial, journal_id,
                        requested_start_usn, requested_end_usn, covered_until_usn, is_complete,
                        protocol_version, contract_version, status, enrolled_unix_ms,
                        checkpointed_unix_ms, ?2
                 FROM library_persistent_journal_source_ranges WHERE id = ?3",
                params![new_id, payload, old_id],
            )
            .map_err(database_error)?;
        for sql in [
            "UPDATE library_persistent_journal_queue_lineage SET source_range_id = ?1 WHERE source_range_id = ?2",
            "UPDATE library_persistent_journal_cross_root_ranges SET source_range_id = ?1 WHERE source_range_id = ?2",
            "UPDATE library_persistent_journal_range_lifecycle SET source_range_id = ?1 WHERE source_range_id = ?2",
            "UPDATE library_persistent_journal_pending_renames SET source_range_id = ?1 WHERE source_range_id = ?2",
        ] {
            transaction
                .execute(sql, params![new_id, old_id])
                .map_err(database_error)?;
        }
        transaction
            .execute(
                "UPDATE library_change_queue_catch_up_lineage
                 SET catch_up_watermark = ?1
                 WHERE catch_up_source = 'persistent_journal_v1'
                   AND catch_up_watermark = ?2",
                params![new_id, old_id],
            )
            .map_err(database_error)?;
        transaction
            .execute(
                "UPDATE library_change_queue
                 SET catch_up_watermark = ?1
                 WHERE catch_up_source = 'persistent_journal_v1'
                   AND catch_up_watermark = ?2",
                params![new_id, old_id],
            )
            .map_err(database_error)?;
        transaction
            .execute(
                "DELETE FROM library_persistent_journal_source_ranges WHERE id = ?1",
                [&old_id],
            )
            .map_err(database_error)?;
    }
    for (_, sql) in PERSISTENT_JOURNAL_CANONICAL_TRIGGER_DDL {
        transaction.execute_batch(sql).map_err(database_error)?;
    }
    backfill_persistent_journal_lifecycle(transaction)?;
    transaction
        .execute("UPDATE schema_info SET version = 24", [])
        .map_err(database_error)?;
    normalize_legacy_metadata_inventory_runs(transaction)?;
    validate_persistent_journal_contract(transaction)
}

fn migrate_v24_to_v25(connection: &mut Connection) -> Result<(), ScanError> {
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(database_error)?;
    migrate_v24_to_v25_transaction(&transaction)?;
    transaction.commit().map_err(database_error)
}

fn migrate_v24_to_v25_transaction(transaction: &Transaction<'_>) -> Result<(), ScanError> {
    normalize_legacy_metadata_inventory_runs(transaction)?;
    validate_persistent_journal_contract(transaction)?;
    transaction
        .execute_batch(
            "CREATE TABLE library_change_lane_contract (
               singleton INTEGER PRIMARY KEY CHECK(singleton = 1),
               complete INTEGER NOT NULL CHECK(complete = 1)
             );
             INSERT INTO library_change_lane_contract(singleton, complete) VALUES (1, 1);
             CREATE TABLE library_change_queue_lanes (
               change_id INTEGER PRIMARY KEY,
               lane TEXT NOT NULL CHECK(lane IN ('p0_live', 'p1_journal', 'p2_recovery')),
               FOREIGN KEY(change_id) REFERENCES library_change_queue(id) ON DELETE CASCADE
             );
             INSERT INTO library_change_queue_lanes(change_id, lane)
             SELECT id, CASE origin
               WHEN 'live_notification' THEN 'p0_live'
               WHEN 'startup_catch_up' THEN 'p1_journal'
               ELSE 'p2_recovery'
             END
             FROM library_change_queue;
             CREATE INDEX library_change_queue_lanes_eligible
               ON library_change_queue_lanes(lane, change_id);
             CREATE TRIGGER library_change_queue_lane_insert_guard
               BEFORE INSERT ON library_change_queue_lanes
               WHEN NEW.lane <> (
                 SELECT CASE origin
                   WHEN 'live_notification' THEN 'p0_live'
                   WHEN 'startup_catch_up' THEN 'p1_journal'
                   ELSE 'p2_recovery'
                 END
                 FROM library_change_queue WHERE id = NEW.change_id
               )
               BEGIN
                 SELECT RAISE(ABORT, 'change queue lane does not match origin');
               END;
             CREATE TRIGGER library_change_queue_lane_update_guard
               BEFORE UPDATE OF lane, change_id ON library_change_queue_lanes
               WHEN NEW.lane <> (
                 SELECT CASE origin
                   WHEN 'live_notification' THEN 'p0_live'
                   WHEN 'startup_catch_up' THEN 'p1_journal'
                   ELSE 'p2_recovery'
                 END
                 FROM library_change_queue WHERE id = NEW.change_id
               )
               BEGIN
                 SELECT RAISE(ABORT, 'change queue lane does not match origin');
               END;
             CREATE TRIGGER library_change_queue_lane_insert
               AFTER INSERT ON library_change_queue
               BEGIN
                 INSERT INTO library_change_queue_lanes(change_id, lane)
                 VALUES (
                   NEW.id,
                   CASE NEW.origin
                     WHEN 'live_notification' THEN 'p0_live'
                     WHEN 'startup_catch_up' THEN 'p1_journal'
                     ELSE 'p2_recovery'
                   END
                 );
               END;
             CREATE TRIGGER library_change_queue_lane_origin_update
               AFTER UPDATE OF origin ON library_change_queue
               BEGIN
                 UPDATE library_change_queue_lanes
                 SET lane = CASE NEW.origin
                   WHEN 'live_notification' THEN 'p0_live'
                   WHEN 'startup_catch_up' THEN 'p1_journal'
                   ELSE 'p2_recovery'
                 END
                 WHERE change_id = NEW.id;
               END;
             CREATE TABLE library_recovery_authority_contract (
               singleton INTEGER PRIMARY KEY CHECK(singleton = 1),
               complete INTEGER NOT NULL CHECK(complete = 1)
             );
             INSERT INTO library_recovery_authority_contract(singleton, complete)
             VALUES (1, 1);
             CREATE TABLE library_recovery_authorities (
               change_id INTEGER PRIMARY KEY,
               run_id TEXT NOT NULL UNIQUE CHECK(length(run_id) BETWEEN 1 AND 512),
               root_id TEXT NOT NULL CHECK(length(root_id) BETWEEN 1 AND 512),
               root_generation INTEGER NOT NULL CHECK(root_generation > 0),
               reason TEXT NOT NULL CHECK(reason IN (
                 'existing_root_baseline', 'first_import_boundary', 'journal_gap',
                 'journal_reset', 'journal_trim', 'journal_reconstruction_failure',
                 'containment_failure', 'broker_after_current_failure', 'watcher_uncovered_gap'
               )),
               opening_journal_id TEXT,
               opening_next_usn TEXT,
               authorized_unix_ms INTEGER NOT NULL,
               retired_unix_ms INTEGER,
               CHECK(
                 (reason IN ('existing_root_baseline', 'first_import_boundary')
                   AND opening_journal_id IS NOT NULL AND opening_next_usn IS NOT NULL)
                 OR
                 (reason NOT IN ('existing_root_baseline', 'first_import_boundary')
                   AND opening_journal_id IS NULL AND opening_next_usn IS NULL)
               ),
               CHECK(retired_unix_ms IS NULL OR retired_unix_ms >= authorized_unix_ms),
               FOREIGN KEY(change_id) REFERENCES library_change_queue(id) ON DELETE CASCADE
             );
             CREATE INDEX library_recovery_authorities_root
               ON library_recovery_authorities(
                 root_id, root_generation, retired_unix_ms, change_id
               );
             CREATE TRIGGER library_recovery_authority_insert_guard
               BEFORE INSERT ON library_recovery_authorities
               WHEN NOT EXISTS (
                 SELECT 1
                 FROM library_change_queue AS queue
                 JOIN library_change_queue_lanes AS lanes ON lanes.change_id = queue.id
                 WHERE queue.id = NEW.change_id
                   AND queue.root_id = NEW.root_id
                   AND queue.root_generation = NEW.root_generation
                   AND lanes.lane = 'p2_recovery'
               )
               BEGIN
                 SELECT RAISE(ABORT, 'recovery authority does not match P2 queue work');
               END;
             CREATE TRIGGER library_recovery_authority_update_guard
               BEFORE UPDATE OF change_id, root_id, root_generation, reason,
                                opening_journal_id, opening_next_usn, authorized_unix_ms
               ON library_recovery_authorities
               BEGIN
                 SELECT RAISE(ABORT, 'recovery authority identity is immutable');
               END;
             UPDATE schema_info SET version = 25;",
        )
        .map_err(database_error)?;
    for sql in [
        PERSISTENT_JOURNAL_BASELINE_TABLE_DDL,
        PERSISTENT_JOURNAL_BASELINE_ROOT_INDEX_DDL,
        PERSISTENT_JOURNAL_BASELINE_INSERT_GUARD_DDL,
        PERSISTENT_JOURNAL_BASELINE_UPDATE_GUARD_DDL,
    ] {
        transaction.execute_batch(sql).map_err(database_error)?;
    }
    validate_change_lane_contract(transaction)?;
    validate_recovery_authority_contract(transaction)?;
    validate_persistent_journal_baseline_contract(transaction)
}

fn normalize_legacy_metadata_inventory_runs(
    transaction: &Transaction<'_>,
) -> Result<(), ScanError> {
    transaction
        .execute(
            "UPDATE library_metadata_inventory_runs
             SET status = 'superseded', absence_authority = 0,
                 completed_unix_ms = NULL,
                 last_issue_code = COALESCE(
                   last_issue_code, 'persistent_journal_migration_baseline_required'
                 ),
                 last_issue_message = COALESCE(
                   last_issue_message,
                   'The partial inventory was retired without absence authority.'
                 )
             WHERE status IN ('running', 'comparing')",
            [],
        )
        .map_err(database_error)?;
    transaction
        .execute(
            "UPDATE library_metadata_inventory_runs
             SET absence_authority = 0
             WHERE status <> 'completed' AND absence_authority <> 0",
            [],
        )
        .map_err(database_error)?;
    Ok(())
}

fn migrate_v25_to_v26(connection: &mut Connection) -> Result<(), ScanError> {
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(database_error)?;
    migrate_v25_to_v26_transaction(&transaction)?;
    transaction.commit().map_err(database_error)
}

fn migrate_v25_to_v26_transaction(transaction: &Transaction<'_>) -> Result<(), ScanError> {
    repair_legacy_terminal_metadata_inventory_state_transaction(transaction, 25)?;
    validate_change_lane_contract(transaction)?;
    validate_recovery_authority_contract(transaction)?;
    validate_persistent_journal_baseline_contract(transaction)?;
    for sql in [
        RECOVERY_EXECUTION_CONTRACT_TABLE_DDL,
        METADATA_INVENTORY_CANDIDATE_OWNER_TABLE_DDL,
        METADATA_INVENTORY_CANDIDATE_CHANGE_INDEX_DDL,
        METADATA_INVENTORY_CANDIDATE_INSERT_GUARD_DDL,
        METADATA_INVENTORY_CANDIDATE_UPDATE_GUARD_DDL,
        METADATA_INVENTORY_FRONTIER_TABLE_DDL,
        METADATA_INVENTORY_FRONTIER_STATE_INDEX_DDL,
    ] {
        transaction.execute_batch(sql).map_err(database_error)?;
    }
    transaction
        .execute(
            "INSERT INTO library_recovery_execution_contract(
               singleton, contract_version, complete
             ) VALUES (1, 1, 1)",
            [],
        )
        .map_err(database_error)?;
    transaction
        .execute("UPDATE schema_info SET version = 26", [])
        .map_err(database_error)?;
    validate_recovery_execution_contract(transaction)
}

fn migrate_v26_to_v27(connection: &mut Connection) -> Result<(), ScanError> {
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(database_error)?;
    migrate_v26_to_v27_transaction(&transaction)?;
    transaction.commit().map_err(database_error)
}

fn migrate_v26_to_v27_transaction(transaction: &Transaction<'_>) -> Result<(), ScanError> {
    repair_legacy_terminal_metadata_inventory_state_transaction(transaction, 26)?;
    validate_recovery_execution_contract(transaction)?;
    for sql in [
        METADATA_INVENTORY_SPOOL_CONTRACT_TABLE_V27_DDL,
        METADATA_INVENTORY_SPOOL_TABLE_V27_DDL,
        METADATA_INVENTORY_SPOOL_DIRECTORY_TABLE_DDL,
        METADATA_INVENTORY_SPOOL_DIRECTORY_STATE_INDEX_DDL,
        METADATA_INVENTORY_SPOOL_ENTRY_TABLE_DDL,
        METADATA_INVENTORY_SPOOL_ENTRY_ORDER_INDEX_DDL,
        METADATA_INVENTORY_SPOOL_BINDING_UPDATE_GUARD_V27_DDL,
        METADATA_INVENTORY_SPOOL_DIRECTORY_COMPLETE_GUARD_DDL,
    ] {
        transaction.execute_batch(sql).map_err(database_error)?;
    }
    transaction
        .execute(
            "INSERT INTO library_metadata_inventory_spool_contract(
               singleton, contract_version, complete
             ) VALUES (1, 1, 1)",
            [],
        )
        .map_err(database_error)?;
    transaction
        .execute_batch(
            "PRAGMA application_id = 1095583025;
             PRAGMA user_version = 27;
             UPDATE schema_info SET version = 27;",
        )
        .map_err(database_error)?;
    debug_assert_eq!(SQLITE_APPLICATION_ID, 1_095_583_025);
    validate_metadata_inventory_spool_contract_version(transaction, 27, 1)
}

fn migrate_v27_to_v28(connection: &mut Connection) -> Result<(), ScanError> {
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(database_error)?;
    migrate_v27_to_v28_transaction(&transaction)?;
    transaction.commit().map_err(database_error)
}

fn migrate_v27_to_v28_transaction(transaction: &Transaction<'_>) -> Result<(), ScanError> {
    repair_legacy_terminal_metadata_inventory_state_transaction(transaction, 27)?;
    validate_metadata_inventory_spool_contract_version(transaction, 27, 1)?;
    transaction
        .execute_batch(
            "UPDATE library_change_queue
             SET status = 'pending', next_retry_unix_ms = NULL,
                 lease_expires_unix_ms = NULL, catalog_revision_at_success = NULL,
                 last_failure_code = 'metadata_inventory_v28_recapture_required',
                 last_failure_message =
                   'The v27 recovery must recapture its pinned-root identity proof'
             WHERE id IN (
               SELECT change_id FROM library_recovery_authorities
               WHERE retired_unix_ms IS NULL
             );
             UPDATE library_change_queue
             SET status = 'superseded', next_retry_unix_ms = NULL,
                 lease_expires_unix_ms = NULL, catalog_revision_at_success = NULL,
                 last_failure_code = 'metadata_inventory_v28_recapture_required',
                 last_failure_message =
                   'The v27 candidate lacked a durable pinned-root identity proof'
             WHERE id IN (
               SELECT owner.change_id
               FROM library_metadata_inventory_candidate_owners AS owner
               JOIN library_recovery_authorities AS authority
                 ON authority.run_id = owner.run_id
               WHERE authority.retired_unix_ms IS NULL
             ) AND status IN ('pending', 'leased', 'retry_wait');
             DELETE FROM library_metadata_inventory_candidate_owners
             WHERE run_id IN (
               SELECT run_id FROM library_recovery_authorities
               WHERE retired_unix_ms IS NULL
             );
             DELETE FROM library_metadata_inventory_entries
             WHERE run_id IN (
               SELECT run_id FROM library_recovery_authorities
               WHERE retired_unix_ms IS NULL
             );
             DELETE FROM library_metadata_inventory_frontier
             WHERE run_id IN (
               SELECT run_id FROM library_recovery_authorities
               WHERE retired_unix_ms IS NULL
             );
             UPDATE library_metadata_inventory_runs
             SET status = 'running', next_page_index = 1, enumeration_cursor = NULL,
                 comparison_cursor = NULL, absence_cursor = NULL, staged_entry_count = 0,
                 candidate_count = 0, enumeration_complete = 0, absence_authority = 0,
                 completed_unix_ms = NULL, last_issue_code = NULL, last_issue_message = NULL
             WHERE id IN (
               SELECT run_id FROM library_recovery_authorities
               WHERE retired_unix_ms IS NULL
             );
             UPDATE library_persistent_journal_baselines
             SET phase = 'inventory', closing_next_usn = NULL, completed_unix_ms = NULL
             WHERE change_id IN (
               SELECT change_id FROM library_recovery_authorities
               WHERE retired_unix_ms IS NULL
             ) AND phase <> 'inventory';
             DELETE FROM library_persistent_journal_checkpoints
             WHERE (root_id, root_generation) IN (
               SELECT baseline.root_id, baseline.root_generation
               FROM library_persistent_journal_baselines AS baseline
               JOIN library_recovery_authorities AS authority
                 ON authority.change_id = baseline.change_id
               WHERE authority.retired_unix_ms IS NULL
                 AND authority.reason IN ('existing_root_baseline', 'first_import_boundary')
             );
             UPDATE library_persistent_journal_checkpoints
             SET continuity_state = 'recovery_required',
                 last_failure_code = 'metadata_inventory_v28_recapture_required',
                 last_failure_message =
                   'The v27 recovery did not retain a pinned-root identity proof'
             WHERE root_id IN (
               SELECT baseline.root_id
               FROM library_persistent_journal_baselines AS baseline
               JOIN library_recovery_authorities AS authority
                 ON authority.change_id = baseline.change_id
               WHERE authority.retired_unix_ms IS NULL
                 AND authority.reason NOT IN (
                   'existing_root_baseline', 'first_import_boundary'
                 )
               UNION
               SELECT baseline.root_id
               FROM library_persistent_journal_baselines AS baseline
               JOIN library_recovery_authorities AS authority
                 ON authority.change_id = baseline.change_id
               WHERE baseline.phase = 'completed' OR authority.retired_unix_ms IS NOT NULL
             ) AND continuity_state IN ('current', 'catching_up', 'recovery_required');
             UPDATE library_persistent_journal_root_state
             SET continuity_state = 'baseline_required'
             WHERE root_id IN (
               SELECT baseline.root_id
               FROM library_persistent_journal_baselines AS baseline
               JOIN library_recovery_authorities AS authority
                 ON authority.change_id = baseline.change_id
               WHERE authority.retired_unix_ms IS NULL
                 AND authority.reason IN ('existing_root_baseline', 'first_import_boundary')
             ) AND capability_state = 'supported';
             UPDATE library_persistent_journal_root_state
             SET continuity_state = 'recovery_required'
             WHERE root_id IN (
               SELECT baseline.root_id
               FROM library_persistent_journal_baselines AS baseline
               JOIN library_recovery_authorities AS authority
                 ON authority.change_id = baseline.change_id
               WHERE authority.retired_unix_ms IS NULL
                 AND authority.reason NOT IN (
                   'existing_root_baseline', 'first_import_boundary'
                 )
               UNION
               SELECT baseline.root_id
               FROM library_persistent_journal_baselines AS baseline
               JOIN library_recovery_authorities AS authority
                 ON authority.change_id = baseline.change_id
               WHERE baseline.phase = 'completed' OR authority.retired_unix_ms IS NOT NULL
              ) AND capability_state = 'supported'
                AND continuity_state IN ('current', 'catching_up', 'recovery_required');
             UPDATE library_change_queue
             SET last_failure_code = 'metadata_inventory_v28_recapture_required',
                 last_failure_message =
                   'The completed v27 recovery lacked a durable pinned-root identity proof'
             WHERE id IN (
               SELECT baseline.change_id
               FROM library_persistent_journal_baselines AS baseline
               JOIN library_recovery_authorities AS authority
                 ON authority.change_id = baseline.change_id
               WHERE baseline.phase = 'completed' OR authority.retired_unix_ms IS NOT NULL
             );
             DELETE FROM library_metadata_inventory_spools;
             DROP TRIGGER library_metadata_inventory_spool_directory_complete_guard;
             DROP TRIGGER library_metadata_inventory_spool_binding_update_guard;
             DROP INDEX library_metadata_inventory_spool_entries_order;
             DROP INDEX library_metadata_inventory_spool_directories_state;
             DROP TABLE library_metadata_inventory_spool_entries;
             DROP TABLE library_metadata_inventory_spool_directories;
             DROP TABLE library_metadata_inventory_spools;
             DROP TABLE library_metadata_inventory_spool_contract;",
        )
        .map_err(database_error)?;
    for sql in [
        METADATA_INVENTORY_SPOOL_CONTRACT_TABLE_DDL,
        METADATA_INVENTORY_SPOOL_TABLE_DDL,
        METADATA_INVENTORY_SPOOL_DIRECTORY_TABLE_DDL,
        METADATA_INVENTORY_SPOOL_DIRECTORY_STATE_INDEX_DDL,
        METADATA_INVENTORY_SPOOL_ENTRY_TABLE_DDL,
        METADATA_INVENTORY_SPOOL_ENTRY_ORDER_INDEX_DDL,
        METADATA_INVENTORY_SPOOL_BINDING_UPDATE_GUARD_DDL,
        METADATA_INVENTORY_SPOOL_DIRECTORY_COMPLETE_GUARD_DDL,
    ] {
        transaction.execute_batch(sql).map_err(database_error)?;
    }
    transaction
        .execute(
            "INSERT INTO library_metadata_inventory_spool_contract(
               singleton, contract_version, complete
             ) VALUES (1, 2, 1)",
            [],
        )
        .map_err(database_error)?;
    transaction
        .execute_batch(
            "PRAGMA application_id = 1095583025;
             PRAGMA user_version = 28;
             UPDATE schema_info SET version = 28;",
        )
        .map_err(database_error)?;
    validate_metadata_inventory_spool_contract_version(transaction, 28, 2)
}

fn migrate_v28_to_v29(connection: &mut Connection) -> Result<(), ScanError> {
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(database_error)?;
    migrate_v28_to_v29_transaction(&transaction)?;
    transaction.commit().map_err(database_error)
}

fn migrate_v28_to_v29_transaction(transaction: &Transaction<'_>) -> Result<(), ScanError> {
    repair_legacy_terminal_metadata_inventory_state_transaction(transaction, 28)?;
    validate_metadata_inventory_spool_contract_version(transaction, 28, 2)?;
    for sql in [
        ROOT_PUBLICATION_NAMESPACE_CONTRACT_TABLE_DDL,
        ROOT_PUBLICATION_NAMESPACE_TABLE_DDL,
        SCAN_PUBLICATION_NAMESPACE_BINDING_TABLE_DDL,
        SCAN_PUBLICATION_NAMESPACE_ROOT_INDEX_DDL,
    ] {
        transaction.execute_batch(sql).map_err(database_error)?;
    }
    transaction
        .execute(
            "INSERT INTO library_root_publication_namespace_contract(
               singleton, contract_version, complete
             ) VALUES (1, 1, 1)",
            [],
        )
        .map_err(database_error)?;

    let mut proofs =
        std::collections::BTreeMap::<(String, i64), (String, String, String, i64)>::new();
    {
        let mut statement = transaction
            .prepare(
                "SELECT spool.root_id, spool.root_generation,
                        spool.root_identity_scheme, spool.root_identity_value,
                        run.updated_unix_ms
                 FROM library_metadata_inventory_spools AS spool
                 JOIN library_metadata_inventory_runs AS run ON run.id = spool.run_id
                 JOIN library_change_root_state AS active ON active.root_id = spool.root_id
                 WHERE active.is_active = 1
                   AND active.generation = spool.root_generation
                 ORDER BY spool.root_id, spool.root_generation, spool.run_id",
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
                ))
            })
            .map_err(database_error)?;
        for row in rows {
            let (root_id, generation, scheme, value, updated_unix_ms) =
                row.map_err(database_error)?;
            validate_windows_root_identity(&scheme, &value)?;
            insert_migrated_publication_proof(
                &mut proofs,
                root_id,
                generation,
                scheme,
                value,
                "metadata_inventory",
                updated_unix_ms,
            )?;
        }
    }
    {
        let mut statement = transaction
            .prepare(
                "SELECT checkpoint.root_id, checkpoint.root_generation,
                        checkpoint.volume_serial, checkpoint.root_file_reference,
                        checkpoint.updated_unix_ms
                 FROM library_persistent_journal_checkpoints AS checkpoint
                 JOIN library_change_root_state AS active ON active.root_id = checkpoint.root_id
                 WHERE active.is_active = 1
                   AND active.generation = checkpoint.root_generation
                   AND checkpoint.root_reference_version = 3
                 ORDER BY checkpoint.root_id",
            )
            .map_err(database_error)?;
        let rows = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Vec<u8>>(3)?,
                    row.get::<_, i64>(4)?,
                ))
            })
            .map_err(database_error)?;
        for row in rows {
            let (root_id, generation, volume_serial, reference, updated_unix_ms) =
                row.map_err(database_error)?;
            if reference.len() != 16 {
                return Err(unverifiable_root_publication_namespace_contract());
            }
            let serial = volume_serial
                .parse::<u64>()
                .map_err(|_| unverifiable_root_publication_namespace_contract())?;
            if serial.to_string() != volume_serial {
                return Err(unverifiable_root_publication_namespace_contract());
            }
            let mut identifier = [0_u8; 16];
            identifier.copy_from_slice(&reference);
            let value = format!("{serial:016x}:{:032x}", u128::from_le_bytes(identifier));
            insert_migrated_publication_proof(
                &mut proofs,
                root_id,
                generation,
                "windows-file-id-128-v1".to_owned(),
                value,
                "journal_v3_migration",
                updated_unix_ms,
            )?;
        }
    }
    let catalog_revision = transaction
        .query_row("SELECT revision FROM catalog_state", [], |row| {
            row.get::<_, i64>(0)
        })
        .map_err(database_error)?;
    for ((root_id, generation), (scheme, value, authority_kind, updated_unix_ms)) in proofs {
        transaction
            .execute(
                "INSERT INTO library_root_publication_namespaces(
                   root_id, root_generation, identity_scheme, identity_value,
                   authority_kind, established_catalog_revision,
                   established_unix_ms, updated_unix_ms
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
                params![
                    root_id,
                    generation,
                    scheme,
                    value,
                    authority_kind,
                    catalog_revision,
                    updated_unix_ms,
                ],
            )
            .map_err(database_error)?;
    }
    transaction
        .execute_batch(
            "UPDATE library_persistent_journal_checkpoints
             SET continuity_state = 'recovery_required',
                 last_failure_code = 'root_publication_namespace_unproven',
                 last_failure_message =
                   'The migrated root has no durable configured namespace identity'
             WHERE root_id NOT IN (
               SELECT root_id FROM library_root_publication_namespaces
             ) AND continuity_state = 'current';
             UPDATE library_persistent_journal_root_state
             SET capability_state = CASE
                   WHEN capability_state = 'supported' THEN 'live_only'
                   ELSE capability_state END,
                 continuity_state = 'recovery_required',
                 last_failure_code = 'root_publication_namespace_unproven',
                 last_failure_message =
                   'The migrated root has no durable configured namespace identity'
             WHERE root_id NOT IN (
               SELECT root_id FROM library_root_publication_namespaces
             ) AND continuity_state = 'current';
             PRAGMA application_id = 1095583025;
             PRAGMA user_version = 29;
             UPDATE schema_info SET version = 29;",
        )
        .map_err(database_error)?;
    validate_root_publication_namespace_contract(transaction)
}

fn migrate_v29_to_v30(connection: &mut Connection) -> Result<(), ScanError> {
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(database_error)?;
    migrate_v29_to_v30_transaction(&transaction)?;
    transaction.commit().map_err(database_error)
}

fn migrate_v29_to_v30_transaction(transaction: &Transaction<'_>) -> Result<(), ScanError> {
    repair_legacy_terminal_metadata_inventory_state_transaction(transaction, 29)?;
    validate_pre_live_gap_schema_contract(transaction, 29)?;
    for sql in [
        LIVE_GAP_RECOVERY_CONTRACT_TABLE_DDL,
        LIVE_GAP_RECOVERY_CLAIM_TABLE_DDL,
        LIVE_GAP_RECOVERY_ROOT_INDEX_DDL,
        LIVE_GAP_RECOVERY_INSERT_GUARD_DDL,
        LIVE_GAP_RECOVERY_IDENTITY_UPDATE_GUARD_DDL,
    ] {
        transaction.execute_batch(sql).map_err(database_error)?;
    }
    transaction
        .execute(
            "INSERT INTO library_live_gap_recovery_contract(
               singleton, contract_version, complete
             ) VALUES (1, 1, 1)",
            [],
        )
        .map_err(database_error)?;

    let orphaned_gap_rows = {
        let mut statement = transaction
            .prepare(
                "SELECT gap.id, gap.root_id, gap.root_generation, gap.updated_unix_ms
                 FROM library_change_queue AS gap
                 JOIN library_change_queue_lanes AS lane ON lane.change_id = gap.id
                 WHERE gap.origin = 'startup_catch_up'
                   AND lane.lane = 'p1_journal'
                   AND gap.intent_kind = 'freshness_unknown'
                   AND gap.scope = 'root' AND gap.relative_path = ''
                   AND gap.previous_relative_path IS NULL
                   AND gap.status IN ('pending', 'leased', 'retry_wait')
                   AND gap.authoritative_scan_id IS NULL
                   AND gap.catch_up_source IS NULL AND gap.catch_up_watermark IS NULL
                   AND gap.superseded_by_change_id IS NULL
                   AND NOT EXISTS(
                     SELECT 1 FROM library_change_queue_catch_up_lineage AS lineage
                     WHERE lineage.change_id = gap.id
                   )
                   AND NOT EXISTS(
                     SELECT 1 FROM library_persistent_journal_queue_lineage AS lineage
                     WHERE lineage.change_id = gap.id
                   )
                   AND NOT EXISTS(
                     SELECT 1 FROM library_recovery_authorities AS authority
                     WHERE authority.change_id = gap.id
                   )
                   AND NOT EXISTS(
                     SELECT 1 FROM library_persistent_journal_baselines AS baseline
                     WHERE baseline.change_id = gap.id
                   )
                   AND NOT EXISTS(
                     SELECT 1 FROM library_metadata_inventory_candidate_owners AS owner
                     WHERE owner.change_id = gap.id
                   )
                 ORDER BY gap.id",
            )
            .map_err(database_error)?;
        statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                ))
            })
            .map_err(database_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(database_error)?
    };
    for (gap_change_id, root_id, root_generation, updated_unix_ms) in orphaned_gap_rows {
        transaction
            .execute(
                "UPDATE library_change_queue
                 SET status = 'retry_wait', next_retry_unix_ms = NULL,
                     lease_expires_unix_ms = NULL, authoritative_scan_id = NULL,
                     last_failure_code =
                       'live_gap_v30_explicit_recovery_required',
                     last_failure_message =
                       'The v29 gap has no event-specific provenance and requires an explicit library update'
                 WHERE id = ?1",
                [gap_change_id],
            )
            .map_err(database_error)?;
        transaction
            .execute(
                "INSERT INTO library_live_gap_recovery_claims(
                   gap_change_id, root_id, root_generation, consumer_kind,
                   created_unix_ms
                 ) VALUES (?1, ?2, ?3, 'explicit_recovery_required', ?4)",
                params![gap_change_id, root_id, root_generation, updated_unix_ms],
            )
            .map_err(database_error)?;
    }

    transaction
        .execute_batch(
            "PRAGMA application_id = 1095583025;
             PRAGMA user_version = 30;
             UPDATE schema_info SET version = 30;",
        )
        .map_err(database_error)?;
    validate_live_gap_recovery_contract(transaction)
}

fn migrate_v30_to_v31(connection: &mut Connection) -> Result<(), ScanError> {
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(database_error)?;
    migrate_v30_to_v31_transaction(&transaction)?;
    transaction.commit().map_err(database_error)
}

fn migrate_v30_to_v31_transaction(transaction: &Transaction<'_>) -> Result<(), ScanError> {
    repair_legacy_terminal_metadata_inventory_state_transaction(transaction, 30)?;
    validate_pre_live_gap_schema_contract(transaction, 30)?;
    repair_retired_live_gap_claims_transaction(transaction, 30)?;
    validate_live_gap_recovery_contract(transaction)?;
    transaction
        .execute_batch(
            "ALTER TABLE catalog_state ADD COLUMN next_source_generation INTEGER NOT NULL DEFAULT 1
               CHECK(next_source_generation > 0);
             ALTER TABLE asset_locations ADD COLUMN source_revision_token TEXT;
             ALTER TABLE asset_locations ADD COLUMN source_generation INTEGER;
             UPDATE asset_locations AS location
             SET file_identity_scheme = NULL, file_identity_value = NULL
             WHERE location.file_identity_scheme IS NOT NULL
               AND EXISTS (
                 SELECT 1 FROM asset_locations AS peer
                 WHERE peer.file_identity_scheme = location.file_identity_scheme
                   AND peer.file_identity_value = location.file_identity_value
                 GROUP BY peer.file_identity_scheme, peer.file_identity_value
                 HAVING MIN(peer.file_size) <> MAX(peer.file_size)
                    OR MIN(peer.modified_unix_ms) <> MAX(peer.modified_unix_ms)
               );
             UPDATE asset_locations AS location
             SET source_generation = CASE
               WHEN file_identity_scheme IS NOT NULL THEN (
                 SELECT MIN(peer.rowid) FROM asset_locations AS peer
                 WHERE peer.file_identity_scheme = location.file_identity_scheme
                   AND peer.file_identity_value = location.file_identity_value
               )
               ELSE location.rowid
             END;
             ALTER TABLE preview_artifacts ADD COLUMN source_revision_token TEXT;
             ALTER TABLE preview_artifacts ADD COLUMN source_generation INTEGER;

             ALTER TABLE library_terminal_media_evidence ADD COLUMN source_revision_token TEXT;
             ALTER TABLE library_terminal_media_evidence ADD COLUMN source_generation INTEGER;
             ALTER TABLE library_change_catch_up_handoffs ADD COLUMN source_revision_token TEXT;
             ALTER TABLE library_change_catch_up_handoffs ADD COLUMN source_generation INTEGER;
             ALTER TABLE library_change_scan_handoff_items ADD COLUMN source_revision_token TEXT;
             ALTER TABLE library_change_scan_handoff_items ADD COLUMN source_generation INTEGER;
             ALTER TABLE library_metadata_inventory_entries ADD COLUMN source_revision_token TEXT;

             DELETE FROM library_terminal_media_evidence;
             DELETE FROM library_change_catch_up_handoffs;
             DELETE FROM library_change_scan_handoff_batches;

             UPDATE library_metadata_inventory_runs
             SET status = 'failed', absence_authority = 0,
                 completed_unix_ms = updated_unix_ms,
                 last_issue_code = 'source_revision_v31_recapture_required',
                 last_issue_message =
                   'The v30 metadata inventory lacked persistent source revision evidence'
             WHERE id IN (SELECT run_id FROM library_metadata_inventory_spools)
               AND status IN ('running', 'comparing', 'completed');
             DELETE FROM library_metadata_inventory_spools;
             DROP INDEX library_metadata_inventory_spool_entries_order;
             DROP TABLE library_metadata_inventory_spool_entries;
             DROP TABLE library_metadata_inventory_spool_contract;
             CREATE TABLE library_metadata_inventory_spool_contract (
               singleton INTEGER PRIMARY KEY CHECK(singleton = 1),
               contract_version INTEGER NOT NULL CHECK(contract_version = 3),
               complete INTEGER NOT NULL CHECK(complete = 1)
             );
             INSERT INTO library_metadata_inventory_spool_contract(
               singleton, contract_version, complete
             ) VALUES (1, 3, 1);
             CREATE TRIGGER asset_locations_source_generation_insert_guard
             BEFORE INSERT ON asset_locations
             WHEN NEW.source_generation IS NULL OR NEW.source_generation <= 0
             BEGIN
               SELECT RAISE(ABORT, 'asset location source generation is required');
             END;
             CREATE TRIGGER asset_locations_source_generation_update_guard
             BEFORE UPDATE OF source_generation ON asset_locations
             WHEN NEW.source_generation IS NULL OR NEW.source_generation <= 0
             BEGIN
               SELECT RAISE(ABORT, 'asset location source generation is required');
             END;

             PRAGMA application_id = 1095583025;
             PRAGMA user_version = 31;
             UPDATE schema_info SET version = 31;",
        )
        .map_err(database_error)?;
    invalidate_precontract_source_revision_metadata(transaction)?;
    create_source_revision_metadata_contract(transaction)?;
    transaction
        .execute_batch(METADATA_INVENTORY_SPOOL_ENTRY_TABLE_V31_DDL)
        .map_err(database_error)?;
    transaction
        .execute_batch(METADATA_INVENTORY_SPOOL_ENTRY_ORDER_INDEX_DDL)
        .map_err(database_error)?;
    let maximum_generation = transaction
        .query_row(
            "SELECT COALESCE(MAX(source_generation), 0) FROM asset_locations",
            [],
            |row| row.get::<_, i64>(0),
        )
        .map_err(database_error)?;
    let next_generation = maximum_generation.checked_add(1).ok_or_else(|| {
        ScanError::new(
            "catalog_source_generation_exhausted",
            "The migrated catalog cannot allocate another source generation",
        )
    })?;
    transaction
        .execute(
            "UPDATE catalog_state SET next_source_generation = ?1",
            [next_generation.max(1)],
        )
        .map_err(database_error)?;
    validate_current_schema_contract_with_source_revision_rows(transaction)
}

fn validate_source_revision_structure_contract(connection: &Connection) -> Result<(), ScanError> {
    let required_columns = [
        ("catalog_state", "next_source_generation"),
        ("asset_locations", "source_revision_token"),
        ("asset_locations", "source_generation"),
        ("preview_artifacts", "source_revision_token"),
        ("preview_artifacts", "source_generation"),
        ("library_terminal_media_evidence", "source_revision_token"),
        ("library_terminal_media_evidence", "source_generation"),
        ("library_change_catch_up_handoffs", "source_revision_token"),
        ("library_change_catch_up_handoffs", "source_generation"),
        ("library_change_scan_handoff_items", "source_revision_token"),
        ("library_change_scan_handoff_items", "source_generation"),
        (
            "library_metadata_inventory_entries",
            "source_revision_token",
        ),
        (
            "library_metadata_inventory_spool_entries",
            "source_revision_token",
        ),
    ];
    let mut structure_matches = true;
    for (table, column) in required_columns {
        structure_matches &= connection
            .query_row(
                &format!(
                    "SELECT EXISTS(SELECT 1 FROM pragma_table_info('{table}') WHERE name = ?1)"
                ),
                [column],
                |row| row.get::<_, bool>(0),
            )
            .map_err(database_error)?;
    }
    structure_matches &= schema_object_sql_matches(
        connection,
        "trigger",
        "asset_locations_source_generation_insert_guard",
        "CREATE TRIGGER asset_locations_source_generation_insert_guard
           BEFORE INSERT ON asset_locations
           WHEN NEW.source_generation IS NULL OR NEW.source_generation <= 0
           BEGIN
             SELECT RAISE(ABORT, 'asset location source generation is required');
           END",
    )?;
    structure_matches &= schema_object_sql_matches(
        connection,
        "trigger",
        "asset_locations_source_generation_update_guard",
        "CREATE TRIGGER asset_locations_source_generation_update_guard
           BEFORE UPDATE OF source_generation ON asset_locations
           WHEN NEW.source_generation IS NULL OR NEW.source_generation <= 0
           BEGIN
             SELECT RAISE(ABORT, 'asset location source generation is required');
           END",
    )?;
    if !structure_matches {
        return Err(ScanError::new(
            "catalog_source_revision_contract_unverifiable",
            "The catalog cannot prove its source revision and generation contract",
        ));
    }
    Ok(())
}

fn validate_source_revision_rows(connection: &Connection) -> Result<(), ScanError> {
    #[cfg(test)]
    SOURCE_REVISION_ROW_AUDIT_COUNT.with(|count| count.set(count.get() + 1));
    let invalid_state = connection
        .query_row(
            "SELECT EXISTS(
               SELECT 1 FROM asset_locations
               WHERE source_generation IS NULL OR source_generation <= 0
                  OR (source_revision_token IS NOT NULL AND (
                    length(source_revision_token) <> 50
                    OR source_revision_token NOT GLOB
                      'windows-file-change-time-100ns-v1:[0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f]'
                  ))
             ) OR EXISTS(
               SELECT 1 FROM preview_artifacts
               WHERE lifecycle_state = 'ready'
                 AND (source_generation IS NULL OR source_generation <= 0
                      OR source_revision_token IS NULL)
             ) OR EXISTS(
               SELECT 1
               FROM asset_locations AS locations
               JOIN library_roots AS roots
                 ON roots.id = locations.root_id
                AND roots.active_scan_id = locations.scan_id
               WHERE locations.file_identity_scheme IS NOT NULL
               GROUP BY locations.file_identity_scheme, locations.file_identity_value
               HAVING MIN(locations.source_generation) <> MAX(locations.source_generation)
                  OR COUNT(DISTINCT locations.source_revision_token) > 1
                  OR MIN(locations.file_size) <> MAX(locations.file_size)
                  OR MIN(locations.modified_unix_ms) <> MAX(locations.modified_unix_ms)
             ) OR EXISTS(
               SELECT 1 FROM asset_locations AS locations
               WHERE locations.file_identity_scheme IS NOT NULL
               GROUP BY locations.scan_id, locations.file_identity_scheme,
                        locations.file_identity_value
               HAVING MIN(locations.source_generation) <> MAX(locations.source_generation)
                  OR COUNT(DISTINCT locations.source_revision_token) > 1
                  OR MIN(locations.file_size) <> MAX(locations.file_size)
                  OR MIN(locations.modified_unix_ms) <> MAX(locations.modified_unix_ms)
             ) OR (SELECT next_source_generation FROM catalog_state) <= COALESCE(
               (SELECT MAX(source_generation) FROM asset_locations), 0
             )",
            [],
            |row| row.get::<_, bool>(0),
        )
        .map_err(database_error)?;
    if invalid_state {
        return Err(ScanError::new(
            "catalog_source_revision_contract_unverifiable",
            "The catalog cannot prove its source revision and generation contract",
        ));
    }
    Ok(())
}

#[cfg(test)]
pub(super) fn downgrade_source_revision_contract_to_v30_for_test(connection: &Connection) {
    let version = connection
        .query_row("SELECT version FROM schema_info LIMIT 1", [], |row| {
            row.get::<_, i64>(0)
        })
        .expect("source revision fixture schema version");
    if version != 31 {
        return;
    }

    connection
        .execute_batch(
            "DROP TRIGGER asset_locations_source_generation_update_guard;
             DROP TRIGGER asset_locations_source_generation_insert_guard;
             DROP TABLE library_source_revision_metadata_contract;
             DROP TABLE library_metadata_inventory_spool_contract;
             ALTER TABLE library_metadata_inventory_spool_entries
               DROP COLUMN source_revision_token;",
        )
        .expect("remove v31-only source revision schema");
    connection
        .execute_batch(METADATA_INVENTORY_SPOOL_CONTRACT_TABLE_DDL)
        .expect("restore v30 inventory spool marker shape");
    connection
        .execute_batch(
            "INSERT INTO library_metadata_inventory_spool_contract(
               singleton, contract_version, complete
             ) VALUES (1, 2, 1);",
        )
        .expect("restore v30 inventory spool marker");
    connection
        .execute_batch(
            "ALTER TABLE catalog_state DROP COLUMN next_source_generation;
             ALTER TABLE asset_locations DROP COLUMN source_revision_token;
             ALTER TABLE asset_locations DROP COLUMN source_generation;
             ALTER TABLE preview_artifacts DROP COLUMN source_revision_token;
             ALTER TABLE preview_artifacts DROP COLUMN source_generation;
             ALTER TABLE library_terminal_media_evidence DROP COLUMN source_revision_token;
             ALTER TABLE library_terminal_media_evidence DROP COLUMN source_generation;
             ALTER TABLE library_change_catch_up_handoffs DROP COLUMN source_revision_token;
             ALTER TABLE library_change_catch_up_handoffs DROP COLUMN source_generation;
             ALTER TABLE library_change_scan_handoff_items DROP COLUMN source_revision_token;
             ALTER TABLE library_change_scan_handoff_items DROP COLUMN source_generation;
             ALTER TABLE library_metadata_inventory_entries DROP COLUMN source_revision_token;
             PRAGMA user_version = 30;
             UPDATE schema_info SET version = 30;",
        )
        .expect("restore exact v30 source revision shape");
}

fn insert_migrated_publication_proof(
    proofs: &mut std::collections::BTreeMap<(String, i64), (String, String, String, i64)>,
    root_id: String,
    generation: i64,
    scheme: String,
    value: String,
    authority_kind: &str,
    updated_unix_ms: i64,
) -> Result<(), ScanError> {
    if generation <= 0 || updated_unix_ms < 0 {
        return Err(unverifiable_root_publication_namespace_contract());
    }
    validate_windows_root_identity(&scheme, &value)?;
    match proofs.entry((root_id, generation)) {
        std::collections::btree_map::Entry::Vacant(entry) => {
            entry.insert((scheme, value, authority_kind.to_owned(), updated_unix_ms));
        }
        std::collections::btree_map::Entry::Occupied(mut entry) => {
            if entry.get().0 != scheme || entry.get().1 != value {
                return Err(unverifiable_root_publication_namespace_contract());
            }
            let proof = entry.get_mut();
            if authority_kind == "metadata_inventory" {
                proof.2 = authority_kind.to_owned();
            }
            proof.3 = proof.3.max(updated_unix_ms);
        }
    }
    Ok(())
}

fn legacy_v23_range_payload(
    transaction: &Transaction<'_>,
    source_range_id: &str,
) -> Result<Vec<u8>, ScanError> {
    let range = transaction
        .query_row(
            "SELECT root_id, root_generation, volume_guid, volume_serial, journal_id,
                    requested_start_usn, requested_end_usn, covered_until_usn, is_complete,
                    protocol_version, contract_version, enrolled_unix_ms
             FROM library_persistent_journal_source_ranges WHERE id = ?1",
            [source_range_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, String>(7)?,
                    row.get::<_, i64>(8)?,
                    row.get::<_, i64>(9)?,
                    row.get::<_, i64>(10)?,
                    row.get::<_, i64>(11)?,
                ))
            },
        )
        .map_err(database_error)?;
    let (
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
        enrolled_unix_ms,
    ) = range;
    let source_range = PersistentJournalSourceRange {
        batch_id: "0".repeat(64),
        root_id,
        root_generation: parse_root_generation(root_generation)?,
        volume: PersistentJournalVolumeIdentity {
            volume_guid,
            volume_serial: parse_canonical_u64(&volume_serial)?,
        },
        journal_id: JournalIdentifier::parse_canonical(&journal_id)
            .map_err(|_| unverifiable_persistent_journal_contract())?,
        requested_start_usn: JournalUsn::parse_canonical(&requested_start_usn)
            .map_err(|_| unverifiable_persistent_journal_contract())?,
        requested_end_usn: JournalUsn::parse_canonical(&requested_end_usn)
            .map_err(|_| unverifiable_persistent_journal_contract())?,
        covered_until_usn: JournalUsn::parse_canonical(&covered_until_usn)
            .map_err(|_| unverifiable_persistent_journal_contract())?,
        is_complete: is_complete != 0,
        protocol_version: u16::try_from(protocol_version)
            .map_err(|_| unverifiable_persistent_journal_contract())?,
        contract_version: u16::try_from(contract_version)
            .map_err(|_| unverifiable_persistent_journal_contract())?,
        state: PersistentJournalRangeState::Enrolled,
        enrolled_unix_ms,
        checkpointed_unix_ms: None,
    };
    source_range
        .validate()
        .map_err(|_| unverifiable_persistent_journal_contract())?;
    let ambiguous_queue = transaction
        .query_row(
            "SELECT EXISTS(
               SELECT 1
               FROM library_persistent_journal_queue_lineage AS ownership
               JOIN library_change_queue AS changes ON changes.id = ownership.change_id
               WHERE ownership.source_range_id = ?1
                 AND (changes.coalesced_observation_count <> 1
                   OR changes.first_observed_unix_ms <> changes.most_recent_observed_unix_ms
                   OR changes.first_sequence <> changes.most_recent_sequence)
             )",
            [source_range_id],
            |row| row.get::<_, bool>(0),
        )
        .map_err(database_error)?;
    if ambiguous_queue {
        return Err(ScanError::new(
            "persistent_journal_v23_batch_unverifiable",
            "The v23 queue row no longer proves its original normalized batch content",
        ));
    }
    let mut statement = transaction
        .prepare(
            "SELECT changes.root_id, changes.root_generation, changes.intent_kind,
                    changes.scope, changes.relative_path, changes.previous_relative_path,
                    changes.origin, changes.first_observed_unix_ms,
                    changes.most_recent_observed_unix_ms, changes.first_sequence,
                    changes.most_recent_sequence, changes.coalesced_observation_count
             FROM library_persistent_journal_queue_lineage AS ownership
             JOIN library_change_queue AS changes ON changes.id = ownership.change_id
             WHERE ownership.source_range_id = ?1 ORDER BY changes.id",
        )
        .map_err(database_error)?;
    let rows = statement
        .query_map([source_range_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, Option<String>>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, i64>(7)?,
                row.get::<_, i64>(8)?,
                row.get::<_, String>(9)?,
                row.get::<_, String>(10)?,
                row.get::<_, i64>(11)?,
            ))
        })
        .map_err(database_error)?;
    let mut intents = Vec::new();
    for row in rows {
        let row = row.map_err(database_error)?;
        intents.push(LibraryChangeIntent {
            root_id: row.0,
            root_generation: parse_root_generation(row.1)?,
            kind: parse_legacy_intent_kind(&row.2)?,
            scope: parse_legacy_intent_scope(&row.3)?,
            relative_path: row.4,
            previous_relative_path: row.5,
            origin: parse_legacy_intent_origin(&row.6)?,
            first_observed_unix_ms: row.7,
            most_recent_observed_unix_ms: row.8,
            first_sequence: parse_canonical_u64(&row.9)?,
            most_recent_sequence: parse_canonical_u64(&row.10)?,
            coalesced_observation_count: u32::try_from(row.11)
                .map_err(|_| unverifiable_persistent_journal_contract())?,
        });
    }
    let mut statement = transaction
        .prepare(
            "SELECT carry_id, volume_guid, volume_serial, journal_id,
                    file_reference_version, file_reference, old_usn,
                    previous_root_id, previous_root_generation, previous_relative_path,
                    is_directory, enrolled_unix_ms
             FROM library_persistent_journal_pending_renames
             WHERE source_range_id = ?1 ORDER BY carry_id",
        )
        .map_err(database_error)?;
    let rows = statement
        .query_map([source_range_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, i64>(4)?,
                row.get::<_, Vec<u8>>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, String>(7)?,
                row.get::<_, i64>(8)?,
                row.get::<_, String>(9)?,
                row.get::<_, bool>(10)?,
                row.get::<_, i64>(11)?,
            ))
        })
        .map_err(database_error)?;
    let mut pending_renames = Vec::new();
    for row in rows {
        let row = row.map_err(database_error)?;
        let file_reference = JournalFileReference::from_bytes(&row.5)
            .map_err(|_| unverifiable_persistent_journal_contract())?;
        if i64::from(file_reference.record_version()) != row.4 {
            return Err(unverifiable_persistent_journal_contract());
        }
        pending_renames.push(PersistentJournalPendingRename {
            carry_id: row.0,
            source_range_id: "0".repeat(64),
            volume: PersistentJournalVolumeIdentity {
                volume_guid: row.1,
                volume_serial: parse_canonical_u64(&row.2)?,
            },
            journal_id: JournalIdentifier::parse_canonical(&row.3)
                .map_err(|_| unverifiable_persistent_journal_contract())?,
            file_reference,
            old_usn: JournalUsn::parse_canonical(&row.6)
                .map_err(|_| unverifiable_persistent_journal_contract())?,
            previous_root_id: row.7,
            previous_root_generation: parse_root_generation(row.8)?,
            previous_relative_path: row.9,
            is_directory: row.10,
            enrolled_unix_ms: row.11,
        });
    }
    let batch = PersistentJournalEnrollmentBatch {
        range: source_range,
        intents,
        cross_root_lineage: Vec::new(),
        carried_cross_root_lineage: Vec::new(),
        pending_renames,
        consumed_pending_rename_ids: Vec::new(),
    };
    batch
        .validate()
        .map_err(|_| unverifiable_persistent_journal_contract())?;
    Ok(persistent_journal_batch_payload(&batch))
}

fn parse_legacy_intent_kind(value: &str) -> Result<LibraryChangeIntentKind, ScanError> {
    match value {
        "reconcile" => Ok(LibraryChangeIntentKind::Reconcile),
        "rename_candidate" => Ok(LibraryChangeIntentKind::RenameCandidate),
        "freshness_unknown" => Ok(LibraryChangeIntentKind::FreshnessUnknown),
        _ => Err(unverifiable_persistent_journal_contract()),
    }
}

fn parse_legacy_intent_scope(value: &str) -> Result<LibraryChangeScope, ScanError> {
    match value {
        "path" => Ok(LibraryChangeScope::Path),
        "subtree" => Ok(LibraryChangeScope::Subtree),
        "root" => Ok(LibraryChangeScope::Root),
        _ => Err(unverifiable_persistent_journal_contract()),
    }
}

fn parse_legacy_intent_origin(value: &str) -> Result<LibraryChangeOrigin, ScanError> {
    match value {
        "live_notification" => Ok(LibraryChangeOrigin::LiveNotification),
        "metadata_inventory" => Ok(LibraryChangeOrigin::MetadataInventory),
        "startup_catch_up" => Ok(LibraryChangeOrigin::StartupCatchUp),
        "user_refresh" => Ok(LibraryChangeOrigin::UserRefresh),
        "consistency_audit" => Ok(LibraryChangeOrigin::ConsistencyAudit),
        _ => Err(unverifiable_persistent_journal_contract()),
    }
}

fn backfill_persistent_journal_lifecycle(transaction: &Transaction<'_>) -> Result<(), ScanError> {
    transaction
        .execute(
            "UPDATE library_persistent_journal_range_lifecycle AS lifecycle
             SET lifecycle_state = 'completed',
                 completed_unix_ms = COALESCE(
                   (SELECT ranges.checkpointed_unix_ms
                    FROM library_persistent_journal_source_ranges AS ranges
                    WHERE ranges.id = lifecycle.source_range_id),
                   updated_unix_ms
                 )
             WHERE lifecycle_state = 'pending'
               AND EXISTS(
                 SELECT 1 FROM library_persistent_journal_source_ranges AS ranges
                 WHERE ranges.id = lifecycle.source_range_id
                   AND ranges.status = 'checkpointed'
               )
               AND NOT EXISTS(
                 SELECT 1
                 FROM library_persistent_journal_queue_lineage AS ownership
                 JOIN library_change_queue AS changes ON changes.id = ownership.change_id
                 WHERE ownership.source_range_id = lifecycle.source_range_id
                   AND changes.status NOT IN ('completed', 'superseded')
               )
               AND NOT EXISTS(
                 SELECT 1 FROM library_persistent_journal_pending_renames AS pending
                 WHERE pending.source_range_id = lifecycle.source_range_id
               )
               AND NOT EXISTS(
                 SELECT 1
                 FROM library_persistent_journal_cross_root_ranges AS owners
                 JOIN library_persistent_journal_cross_root_lineage AS lineage
                   ON lineage.id = owners.lineage_id
                 WHERE owners.source_range_id = lifecycle.source_range_id
                   AND lineage.status <> 'completed'
               )",
            [],
        )
        .map_err(database_error)?;
    Ok(())
}

fn rebuild_library_change_queue_for_metadata_inventory(
    transaction: &Transaction<'_>,
) -> Result<(), ScanError> {
    transaction
        .execute_batch(
            "CREATE TABLE library_change_queue_v20 (
               id INTEGER PRIMARY KEY AUTOINCREMENT,
               root_id TEXT NOT NULL,
               root_generation INTEGER NOT NULL CHECK(root_generation > 0),
               intent_kind TEXT NOT NULL CHECK(intent_kind IN (
                 'reconcile', 'rename_candidate', 'freshness_unknown'
               )),
               scope TEXT NOT NULL CHECK(scope IN ('path', 'subtree', 'root')),
               relative_path TEXT NOT NULL,
               previous_relative_path TEXT,
               origin TEXT NOT NULL CHECK(origin IN (
                 'live_notification', 'metadata_inventory', 'startup_catch_up',
                 'user_refresh', 'consistency_audit'
               )),
               first_observed_unix_ms INTEGER NOT NULL,
               most_recent_observed_unix_ms INTEGER NOT NULL,
               first_sequence TEXT NOT NULL CHECK(length(first_sequence) > 0),
               most_recent_sequence TEXT NOT NULL CHECK(length(most_recent_sequence) > 0),
               coalesced_observation_count INTEGER NOT NULL
                 CHECK(coalesced_observation_count > 0),
               status TEXT NOT NULL CHECK(status IN (
                 'pending', 'leased', 'retry_wait', 'completed', 'superseded'
               )),
               ready_unix_ms INTEGER NOT NULL,
               attempt_count INTEGER NOT NULL DEFAULT 0 CHECK(attempt_count >= 0),
               next_retry_unix_ms INTEGER,
               lease_generation INTEGER NOT NULL DEFAULT 0 CHECK(lease_generation >= 0),
               lease_expires_unix_ms INTEGER,
               last_failure_code TEXT,
               last_failure_message TEXT,
               catalog_revision_at_enqueue INTEGER NOT NULL
                 CHECK(catalog_revision_at_enqueue >= 0),
               catalog_revision_at_success INTEGER
                 CHECK(catalog_revision_at_success IS NULL OR catalog_revision_at_success >= 0),
               catch_up_source TEXT,
               catch_up_watermark TEXT,
               authoritative_scan_id TEXT,
               superseded_by_change_id INTEGER,
               created_unix_ms INTEGER NOT NULL,
               updated_unix_ms INTEGER NOT NULL,
               CHECK(first_observed_unix_ms <= most_recent_observed_unix_ms),
               CHECK(
                 (last_failure_code IS NULL AND last_failure_message IS NULL)
                 OR
                 (last_failure_code IS NOT NULL AND last_failure_message IS NOT NULL)
               ),
               CHECK(
                 (status = 'leased' AND lease_expires_unix_ms IS NOT NULL)
                 OR
                 (status <> 'leased' AND lease_expires_unix_ms IS NULL)
               ),
               CHECK(status = 'retry_wait' OR next_retry_unix_ms IS NULL),
               CHECK(
                 (status = 'completed' AND catalog_revision_at_success IS NOT NULL)
                 OR
                 (status <> 'completed' AND catalog_revision_at_success IS NULL)
               ),
               FOREIGN KEY(superseded_by_change_id) REFERENCES library_change_queue_v20(id)
                 ON DELETE SET NULL
             );
             INSERT INTO library_change_queue_v20(
               id, root_id, root_generation, intent_kind, scope, relative_path,
               previous_relative_path, origin, first_observed_unix_ms,
               most_recent_observed_unix_ms, first_sequence, most_recent_sequence,
               coalesced_observation_count, status, ready_unix_ms, attempt_count,
               next_retry_unix_ms, lease_generation, lease_expires_unix_ms,
               last_failure_code, last_failure_message, catalog_revision_at_enqueue,
               catalog_revision_at_success, catch_up_source, catch_up_watermark,
               authoritative_scan_id, superseded_by_change_id, created_unix_ms, updated_unix_ms
             )
             SELECT
               id, root_id, root_generation, intent_kind, scope, relative_path,
               previous_relative_path, origin, first_observed_unix_ms,
               most_recent_observed_unix_ms, first_sequence, most_recent_sequence,
               coalesced_observation_count, status, ready_unix_ms, attempt_count,
               next_retry_unix_ms, lease_generation, lease_expires_unix_ms,
               last_failure_code, last_failure_message, catalog_revision_at_enqueue,
               catalog_revision_at_success, catch_up_source, catch_up_watermark,
               authoritative_scan_id, superseded_by_change_id, created_unix_ms, updated_unix_ms
             FROM library_change_queue;
             CREATE TABLE library_change_queue_catch_up_lineage_v20 (
               change_id INTEGER NOT NULL,
               catch_up_source TEXT NOT NULL CHECK(length(catch_up_source) BETWEEN 1 AND 128),
               catch_up_watermark TEXT NOT NULL
                 CHECK(length(catch_up_watermark) BETWEEN 1 AND 1024),
               enrolled_unix_ms INTEGER NOT NULL,
               PRIMARY KEY(change_id, catch_up_source, catch_up_watermark),
               FOREIGN KEY(change_id) REFERENCES library_change_queue_v20(id) ON DELETE CASCADE
             );
             INSERT INTO library_change_queue_catch_up_lineage_v20(
               change_id, catch_up_source, catch_up_watermark, enrolled_unix_ms
             )
             SELECT change_id, catch_up_source, catch_up_watermark, enrolled_unix_ms
             FROM library_change_queue_catch_up_lineage;
             DROP TABLE library_change_queue_catch_up_lineage;
             DROP TABLE library_change_queue;
             ALTER TABLE library_change_queue_v20 RENAME TO library_change_queue;
             ALTER TABLE library_change_queue_catch_up_lineage_v20
               RENAME TO library_change_queue_catch_up_lineage;
             CREATE INDEX library_change_queue_eligible
               ON library_change_queue(
                 root_id, root_generation, status, ready_unix_ms, next_retry_unix_ms, id
               );
             CREATE INDEX library_change_queue_lease_expiry
               ON library_change_queue(status, lease_expires_unix_ms, id);
             CREATE INDEX library_change_queue_active_path
               ON library_change_queue(
                 root_id, root_generation, status, relative_path, scope, id
               );
             CREATE INDEX library_change_queue_cleanup
               ON library_change_queue(status, updated_unix_ms, id);
             CREATE INDEX library_change_queue_catch_up_lineage_evidence
               ON library_change_queue_catch_up_lineage(
                 catch_up_source, catch_up_watermark, change_id
               );",
        )
        .map_err(database_error)
}

fn create_metadata_inventory_contract(transaction: &Transaction<'_>) -> Result<(), ScanError> {
    transaction
        .execute_batch(
            "CREATE TABLE library_metadata_inventory_contract (
               singleton INTEGER PRIMARY KEY CHECK(singleton = 1),
               complete INTEGER NOT NULL CHECK(complete = 1)
             );
             INSERT INTO library_metadata_inventory_contract(singleton, complete) VALUES (1, 1);
             CREATE TABLE library_metadata_inventory_runs (
               id TEXT NOT NULL PRIMARY KEY CHECK(length(id) BETWEEN 1 AND 256),
               root_id TEXT NOT NULL,
               root_generation INTEGER NOT NULL CHECK(root_generation > 0),
               epoch INTEGER NOT NULL CHECK(epoch > 0),
               scope_kind TEXT NOT NULL CHECK(scope_kind IN ('root', 'subtree')),
               scope_relative_path TEXT NOT NULL,
               status TEXT NOT NULL CHECK(status IN (
                 'running', 'comparing', 'completed', 'failed', 'cancelled', 'superseded'
               )),
               next_page_index INTEGER NOT NULL CHECK(next_page_index > 0),
               enumeration_cursor TEXT,
               comparison_cursor TEXT,
               absence_cursor TEXT,
               staged_entry_count INTEGER NOT NULL DEFAULT 0 CHECK(staged_entry_count >= 0),
               candidate_count INTEGER NOT NULL DEFAULT 0 CHECK(candidate_count >= 0),
               enumeration_complete INTEGER NOT NULL DEFAULT 0
                 CHECK(enumeration_complete IN (0, 1)),
               absence_authority INTEGER NOT NULL DEFAULT 0
                 CHECK(absence_authority IN (0, 1)),
               started_unix_ms INTEGER NOT NULL,
               updated_unix_ms INTEGER NOT NULL,
               completed_unix_ms INTEGER,
               last_issue_code TEXT,
               last_issue_message TEXT,
               CHECK(
                 (scope_kind = 'root' AND scope_relative_path = '')
                 OR
                 (scope_kind = 'subtree' AND length(scope_relative_path) > 0)
               ),
               CHECK(instr(scope_relative_path, char(92)) = 0),
               CHECK(
                 (last_issue_code IS NULL AND last_issue_message IS NULL)
                 OR
                 (last_issue_code IS NOT NULL AND last_issue_message IS NOT NULL)
               ),
               CHECK(enumeration_complete = 1 OR absence_authority = 0),
               CHECK(
                 (status = 'completed' AND completed_unix_ms IS NOT NULL
                   AND enumeration_complete = 1 AND absence_authority = 1)
                 OR
                 (status <> 'completed' AND completed_unix_ms IS NULL)
               ),
               UNIQUE(root_id, root_generation, epoch),
               FOREIGN KEY(root_id) REFERENCES library_roots(id) ON DELETE CASCADE
             );
             CREATE UNIQUE INDEX library_metadata_inventory_runs_one_active_root
               ON library_metadata_inventory_runs(root_id)
               WHERE status IN ('running', 'comparing');
             CREATE INDEX library_metadata_inventory_runs_cleanup
               ON library_metadata_inventory_runs(status, updated_unix_ms, id);
             CREATE TABLE library_metadata_inventory_entries (
               run_id TEXT NOT NULL,
               relative_path TEXT NOT NULL,
               entry_kind TEXT NOT NULL CHECK(entry_kind IN ('file', 'directory', 'other')),
               file_size INTEGER CHECK(file_size IS NULL OR file_size >= 0),
               modified_unix_ms INTEGER NOT NULL,
               file_identity_scheme TEXT,
               file_identity_value TEXT,
               placeholder_state TEXT NOT NULL CHECK(placeholder_state IN (
                 'available', 'offline', 'recall_on_open', 'recall_on_data_access'
               )),
               is_reparse_point INTEGER NOT NULL CHECK(is_reparse_point IN (0, 1)),
               staged_page_index INTEGER NOT NULL CHECK(staged_page_index > 0),
               comparison_status TEXT NOT NULL DEFAULT 'pending'
                 CHECK(comparison_status IN ('pending', 'unchanged', 'enqueued')),
               candidate_previous_relative_path TEXT,
               staged_unix_ms INTEGER NOT NULL,
               CHECK(length(relative_path) > 0),
               CHECK(instr(relative_path, char(92)) = 0),
               CHECK(
                 (entry_kind = 'file' AND file_size IS NOT NULL)
                 OR
                 (entry_kind <> 'file' AND file_size IS NULL)
               ),
               CHECK(
                 (file_identity_scheme IS NULL AND file_identity_value IS NULL)
                 OR
                 (file_identity_scheme IS NOT NULL AND file_identity_value IS NOT NULL)
               ),
               CHECK(
                 candidate_previous_relative_path IS NULL
                 OR comparison_status = 'enqueued'
               ),
               PRIMARY KEY(run_id, relative_path),
               FOREIGN KEY(run_id) REFERENCES library_metadata_inventory_runs(id)
                 ON DELETE CASCADE
             );
             CREATE INDEX library_metadata_inventory_entries_compare
               ON library_metadata_inventory_entries(run_id, comparison_status, relative_path);
             CREATE INDEX library_metadata_inventory_entries_identity
               ON library_metadata_inventory_entries(
                 run_id, file_identity_scheme, file_identity_value, relative_path
               );
             CREATE INDEX library_metadata_inventory_entries_previous
               ON library_metadata_inventory_entries(
                 run_id, candidate_previous_relative_path, relative_path
               );",
        )
        .map_err(database_error)
}

fn add_change_catch_up_contract(transaction: &Transaction<'_>) -> Result<(), ScanError> {
    transaction
        .execute_batch(
            "CREATE TABLE library_change_catch_up_state (
               volume_id TEXT PRIMARY KEY CHECK(length(volume_id) BETWEEN 1 AND 512),
               journal_id TEXT NOT NULL CHECK(length(journal_id) BETWEEN 1 AND 32),
               next_usn TEXT NOT NULL CHECK(length(next_usn) BETWEEN 1 AND 32),
               root_set_fingerprint TEXT NOT NULL
                 CHECK(length(root_set_fingerprint) = 64),
               catalog_revision INTEGER NOT NULL CHECK(catalog_revision >= 0),
               updated_unix_ms INTEGER NOT NULL CHECK(updated_unix_ms >= 0)
             );
             CREATE INDEX asset_locations_root_relative
               ON asset_locations(root_id, relative_path, scan_id, location_id);
             ALTER TABLE library_change_queue_contract
               ADD COLUMN change_catch_up_complete INTEGER NOT NULL DEFAULT 1
               CHECK(change_catch_up_complete = 1);
             ALTER TABLE library_change_queue_contract
               ADD COLUMN scan_catch_up_lineage_complete INTEGER NOT NULL DEFAULT 1
               CHECK(scan_catch_up_lineage_complete = 1);
             ALTER TABLE library_change_queue_contract
               ADD COLUMN scan_handoff_batch_complete INTEGER NOT NULL DEFAULT 1
               CHECK(scan_handoff_batch_complete = 1);",
        )
        .map_err(database_error)?;
    create_change_catch_up_handoff_contract(transaction)?;
    create_change_catch_up_lineage_contract(transaction)?;
    create_scan_run_catch_up_lineage_contract(transaction)?;
    create_scan_handoff_batch_contract(transaction)?;
    seed_change_catch_up_lineage(transaction)
}

fn create_change_catch_up_handoff_contract(transaction: &Transaction<'_>) -> Result<(), ScanError> {
    transaction
        .execute_batch(
            "CREATE TABLE library_change_catch_up_handoffs (
               catch_up_source TEXT NOT NULL CHECK(length(catch_up_source) BETWEEN 1 AND 128),
               catch_up_watermark TEXT NOT NULL
                 CHECK(length(catch_up_watermark) BETWEEN 1 AND 1024),
               file_identity_scheme TEXT NOT NULL,
               file_identity_value TEXT NOT NULL,
               asset_id TEXT NOT NULL,
               source_location_id TEXT NOT NULL,
               root_id TEXT NOT NULL,
               absolute_path TEXT NOT NULL,
               relative_path TEXT NOT NULL,
               preview_path TEXT NOT NULL,
               file_size INTEGER NOT NULL CHECK(file_size >= 0),
               created_unix_ms INTEGER,
               modified_unix_ms INTEGER NOT NULL,
               width INTEGER NOT NULL CHECK(width >= 0),
               height INTEGER NOT NULL CHECK(height >= 0),
               preview_status TEXT NOT NULL
                 CHECK(preview_status IN ('pending', 'ready', 'failed')),
               preview_issue_code TEXT,
               preview_issue_message TEXT,
               metadata_engine_id TEXT NOT NULL,
               metadata_engine_version TEXT NOT NULL,
               capture_local_time TEXT,
               capture_offset_minutes INTEGER,
               capture_time_source TEXT
                 CHECK(capture_time_source IS NULL OR capture_time_source IN (
                   'exif_original', 'exif_digitized', 'exif_datetime'
                 )),
               capture_raw_value TEXT,
               updated_unix_ms INTEGER NOT NULL CHECK(updated_unix_ms >= 0),
               CHECK(
                 (capture_local_time IS NULL AND capture_time_source IS NULL
                   AND capture_raw_value IS NULL)
                 OR
                 (capture_local_time IS NOT NULL AND capture_time_source IS NOT NULL
                   AND capture_raw_value IS NOT NULL)
               ),
               PRIMARY KEY(
                 catch_up_source, catch_up_watermark,
                 file_identity_scheme, file_identity_value
               )
             );
             CREATE INDEX library_change_catch_up_handoffs_asset
               ON library_change_catch_up_handoffs(
                 asset_id, catch_up_source, catch_up_watermark
               );
             CREATE INDEX library_change_catch_up_handoffs_preview
               ON library_change_catch_up_handoffs(
                 preview_path, preview_status, catch_up_source, catch_up_watermark
               );",
        )
        .map_err(database_error)
}

fn create_change_catch_up_lineage_contract(transaction: &Transaction<'_>) -> Result<(), ScanError> {
    transaction
        .execute_batch(
            "CREATE TABLE library_change_queue_catch_up_lineage (
               change_id INTEGER NOT NULL,
               catch_up_source TEXT NOT NULL CHECK(length(catch_up_source) BETWEEN 1 AND 128),
               catch_up_watermark TEXT NOT NULL
                 CHECK(length(catch_up_watermark) BETWEEN 1 AND 1024),
               enrolled_unix_ms INTEGER NOT NULL,
               PRIMARY KEY(change_id, catch_up_source, catch_up_watermark),
               FOREIGN KEY(change_id) REFERENCES library_change_queue(id) ON DELETE CASCADE
             );
             CREATE INDEX library_change_queue_catch_up_lineage_evidence
               ON library_change_queue_catch_up_lineage(
                 catch_up_source, catch_up_watermark, change_id
               );",
        )
        .map_err(database_error)
}

fn create_scan_run_catch_up_lineage_contract(
    transaction: &Transaction<'_>,
) -> Result<(), ScanError> {
    transaction
        .execute_batch(
            "CREATE TABLE scan_run_catch_up_lineage (
               scan_id TEXT NOT NULL,
               catch_up_source TEXT NOT NULL CHECK(length(catch_up_source) BETWEEN 1 AND 128),
               catch_up_watermark TEXT NOT NULL
                 CHECK(length(catch_up_watermark) BETWEEN 1 AND 1024),
               enrolled_unix_ms INTEGER NOT NULL,
               PRIMARY KEY(scan_id, catch_up_source, catch_up_watermark),
               FOREIGN KEY(scan_id) REFERENCES scan_runs(id) ON DELETE CASCADE
             );
             CREATE INDEX scan_run_catch_up_lineage_evidence
               ON scan_run_catch_up_lineage(
                 catch_up_source, catch_up_watermark, scan_id
               );",
        )
        .map_err(database_error)
}

fn create_scan_handoff_batch_contract(transaction: &Transaction<'_>) -> Result<(), ScanError> {
    transaction
        .execute_batch(
            "CREATE TABLE library_change_scan_handoff_batches (
               id TEXT NOT NULL PRIMARY KEY CHECK(length(id) BETWEEN 1 AND 1024),
               source_root_id TEXT NOT NULL CHECK(length(source_root_id) BETWEEN 1 AND 4096),
               updated_unix_ms INTEGER NOT NULL CHECK(updated_unix_ms >= 0)
             );
             CREATE TABLE library_change_scan_handoff_lineage (
               batch_id TEXT NOT NULL,
               catch_up_source TEXT NOT NULL CHECK(length(catch_up_source) BETWEEN 1 AND 128),
               catch_up_watermark TEXT NOT NULL
                 CHECK(length(catch_up_watermark) BETWEEN 1 AND 1024),
               enrolled_unix_ms INTEGER NOT NULL,
               PRIMARY KEY(batch_id, catch_up_source, catch_up_watermark),
               FOREIGN KEY(batch_id) REFERENCES library_change_scan_handoff_batches(id)
                 ON DELETE CASCADE
             );
             CREATE INDEX library_change_scan_handoff_lineage_evidence
               ON library_change_scan_handoff_lineage(
                 catch_up_source, catch_up_watermark, batch_id
               );
             CREATE TABLE library_change_scan_handoff_items (
               batch_id TEXT NOT NULL,
               file_identity_scheme TEXT NOT NULL,
               file_identity_value TEXT NOT NULL,
               asset_id TEXT NOT NULL,
               source_location_id TEXT NOT NULL,
               root_id TEXT NOT NULL,
               absolute_path TEXT NOT NULL,
               relative_path TEXT NOT NULL,
               preview_path TEXT NOT NULL,
               file_size INTEGER NOT NULL CHECK(file_size >= 0),
               created_unix_ms INTEGER,
               modified_unix_ms INTEGER NOT NULL,
               width INTEGER NOT NULL CHECK(width >= 0),
               height INTEGER NOT NULL CHECK(height >= 0),
               preview_status TEXT NOT NULL
                 CHECK(preview_status IN ('pending', 'ready', 'failed')),
               preview_issue_code TEXT,
               preview_issue_message TEXT,
               metadata_engine_id TEXT NOT NULL,
               metadata_engine_version TEXT NOT NULL,
               capture_local_time TEXT,
               capture_offset_minutes INTEGER,
               capture_time_source TEXT
                 CHECK(capture_time_source IS NULL OR capture_time_source IN (
                   'exif_original', 'exif_digitized', 'exif_datetime'
                 )),
               capture_raw_value TEXT,
               CHECK(
                 (capture_local_time IS NULL AND capture_time_source IS NULL
                   AND capture_raw_value IS NULL)
                 OR
                 (capture_local_time IS NOT NULL AND capture_time_source IS NOT NULL
                   AND capture_raw_value IS NOT NULL)
               ),
               PRIMARY KEY(batch_id, file_identity_scheme, file_identity_value),
               FOREIGN KEY(batch_id) REFERENCES library_change_scan_handoff_batches(id)
                 ON DELETE CASCADE
             );
             CREATE INDEX library_change_scan_handoff_items_identity
               ON library_change_scan_handoff_items(
                 file_identity_scheme, file_identity_value, batch_id
               );
             CREATE INDEX library_change_scan_handoff_items_asset
               ON library_change_scan_handoff_items(asset_id, batch_id);
             CREATE INDEX library_change_scan_handoff_items_preview
               ON library_change_scan_handoff_items(
                 preview_path, preview_status, batch_id
               );",
        )
        .map_err(database_error)
}

fn seed_change_catch_up_lineage(transaction: &Transaction<'_>) -> Result<(), ScanError> {
    transaction
        .execute(
            "INSERT OR IGNORE INTO library_change_queue_catch_up_lineage(
               change_id, catch_up_source, catch_up_watermark, enrolled_unix_ms
             )
             SELECT id, catch_up_source, catch_up_watermark, 0
             FROM library_change_queue
             WHERE catch_up_source IS NOT NULL AND catch_up_watermark IS NOT NULL",
            [],
        )
        .map_err(database_error)?;
    Ok(())
}

fn add_authoritative_recovery_contract_marker(
    transaction: &Transaction<'_>,
) -> Result<(), ScanError> {
    transaction
        .execute_batch(
            "ALTER TABLE library_change_queue_contract
               ADD COLUMN authoritative_recovery_complete INTEGER NOT NULL DEFAULT 1
               CHECK(authoritative_recovery_complete = 1);
             ALTER TABLE library_change_queue_contract
               ADD COLUMN scan_ownership_complete INTEGER NOT NULL DEFAULT 1
               CHECK(scan_ownership_complete = 1);",
        )
        .map_err(database_error)
}

fn normalize_relative_paths_for_continuous_synchronization(
    transaction: &Transaction<'_>,
) -> Result<(), ScanError> {
    for (table, column) in [
        ("asset_locations", "relative_path"),
        ("scan_directory_frontier", "relative_path"),
        ("scan_directory_entries", "relative_path"),
        ("scan_runs", "current_directory_relative_path"),
        ("scan_runs", "last_visited_relative_path"),
    ] {
        let exists = transaction
            .query_row(
                "SELECT EXISTS(
                   SELECT 1 FROM sqlite_master
                   WHERE type = 'table' AND name = ?1
                 ) AND EXISTS(
                   SELECT 1 FROM pragma_table_info(?1) WHERE name = ?2
                 )",
                [table, column],
                |row| row.get::<_, bool>(0),
            )
            .map_err(database_error)?;
        if exists {
            transaction
                .execute(
                    &format!(
                        "UPDATE {table} SET {column} = replace({column}, char(92), '/')
                         WHERE instr({column}, char(92)) > 0"
                    ),
                    [],
                )
                .map_err(database_error)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use rusqlite::Connection;
    use tempfile::{NamedTempFile, tempdir};

    use super::{
        PERSISTENT_JOURNAL_CANONICAL_TRIGGER_DDL, PERSISTENT_JOURNAL_LEGACY_V24_TRIGGER_DDL,
        SCHEMA_VERSION, create_metadata_inventory_contract, create_schema_v19, migrate_schema,
        migrate_v16_to_v17, migrate_v17_to_v18, migrate_v18_to_v19, normalize_schema_sql,
        preview_repair_marker_is_complete, repair_missing_v19_preview_expectation_marker,
        repair_prerelease_v18_scan_owner_index,
    };
    use crate::adapters::SqliteCatalog;
    use crate::adapters::sqlite_catalog::remove_persistent_journal_v22_contract_for_test;

    #[test]
    fn fresh_catalog_enables_incremental_auto_vacuum_before_schema_creation() {
        let catalog = NamedTempFile::new().expect("fresh catalog");
        let mut connection = Connection::open(catalog.path()).expect("open fresh catalog");

        migrate_schema(&mut connection).expect("create current schema");

        let mode: i64 = connection
            .query_row("PRAGMA auto_vacuum", [], |row| row.get(0))
            .expect("read auto-vacuum mode");
        assert_eq!(mode, 2);
    }

    #[test]
    fn production_catalog_open_enables_incremental_before_wal_initialization() {
        let directory = tempdir().expect("catalog directory");
        let path = directory.path().join("ame.sqlite3");

        drop(SqliteCatalog::open(path.clone()).expect("initialize production catalog"));

        let connection = Connection::open(path).expect("inspect production catalog");
        let mode: i64 = connection
            .query_row("PRAGMA auto_vacuum", [], |row| row.get(0))
            .expect("read production auto-vacuum mode");
        assert_eq!(mode, 2);
    }

    #[test]
    fn opening_a_current_legacy_none_catalog_does_not_vacuum_during_migration() {
        let directory = tempdir().expect("catalog directory");
        let path = directory.path().join("ame.sqlite3");
        let mut connection = Connection::open(&path).expect("open legacy catalog");
        migrate_schema(&mut connection).expect("create current schema");
        connection
            .execute_batch(
                "PRAGMA journal_mode = DELETE;
                 PRAGMA auto_vacuum = NONE;
                 VACUUM;
                 CREATE TABLE migration_reclamation_fixture(payload BLOB NOT NULL);
                 WITH RECURSIVE rows(value) AS (
                   SELECT 1 UNION ALL SELECT value + 1 FROM rows WHERE value < 512
                 )
                 INSERT INTO migration_reclamation_fixture(payload)
                 SELECT zeroblob(4096) FROM rows;
                 DELETE FROM migration_reclamation_fixture;",
            )
            .expect("seed legacy freelist");
        let before_freelist: i64 = connection
            .query_row("PRAGMA freelist_count", [], |row| row.get(0))
            .expect("legacy freelist before reopen migration");
        assert!(before_freelist > 0);
        drop(connection);

        drop(SqliteCatalog::open(path.clone()).expect("reopen current legacy catalog"));

        let connection = Connection::open(path).expect("inspect reopened legacy catalog");
        let mode: i64 = connection
            .query_row("PRAGMA auto_vacuum", [], |row| row.get(0))
            .expect("legacy auto-vacuum mode after migration");
        let after_freelist: i64 = connection
            .query_row("PRAGMA freelist_count", [], |row| row.get(0))
            .expect("legacy freelist after migration");
        assert_eq!(mode, 0);
        assert_eq!(after_freelist, before_freelist);
    }

    fn remove_root_publication_namespace_v29_contract_for_test(connection: &Connection) {
        super::downgrade_source_revision_contract_to_v30_for_test(connection);
        connection
            .execute_batch(
                "DROP TRIGGER IF EXISTS library_live_gap_recovery_claim_identity_update_guard;
                 DROP TRIGGER IF EXISTS library_live_gap_recovery_claim_insert_guard;
                 DROP INDEX IF EXISTS library_live_gap_recovery_claims_root;
                 DROP TABLE IF EXISTS library_live_gap_recovery_claims;
                 DROP TABLE IF EXISTS library_live_gap_recovery_contract;
                 DROP INDEX IF EXISTS library_scan_publication_namespace_root;
                 DROP TABLE IF EXISTS library_scan_publication_namespace_bindings;
                 DROP TABLE IF EXISTS library_root_publication_namespaces;
                 DROP TABLE IF EXISTS library_root_publication_namespace_contract;",
            )
            .expect("remove v29 configured-root publication fixture");
    }

    fn downgrade_current_catalog_to_v29(connection: &Connection) {
        super::downgrade_source_revision_contract_to_v30_for_test(connection);
        connection
            .execute_batch(
                "DROP TRIGGER IF EXISTS library_live_gap_recovery_claim_identity_update_guard;
                 DROP TRIGGER IF EXISTS library_live_gap_recovery_claim_insert_guard;
                 DROP INDEX IF EXISTS library_live_gap_recovery_claims_root;
                 DROP TABLE IF EXISTS library_live_gap_recovery_claims;
                 DROP TABLE IF EXISTS library_live_gap_recovery_contract;
                 PRAGMA user_version = 29;
                 UPDATE schema_info SET version = 29;",
            )
            .expect("remove v30 live-gap recovery fixture");
        super::validate_pre_live_gap_schema_contract(connection, 29)
            .expect("validate exact v29 fixture");
    }

    fn downgrade_current_catalog_to_v28(connection: &Connection) {
        remove_root_publication_namespace_v29_contract_for_test(connection);
        connection
            .execute_batch(
                "PRAGMA user_version = 28;
                 UPDATE schema_info SET version = 28;",
            )
            .expect("publish exact v28 fixture version");
        super::validate_metadata_inventory_spool_contract_version(connection, 28, 2)
            .expect("validate exact v28 fixture");
    }

    fn remove_change_lane_v25_contract_for_test(connection: &Connection) {
        remove_root_publication_namespace_v29_contract_for_test(connection);
        connection
            .execute_batch(
                "DROP TRIGGER IF EXISTS library_metadata_inventory_spool_directory_complete_guard;
                 DROP TRIGGER IF EXISTS library_metadata_inventory_spool_binding_update_guard;
                 DROP INDEX IF EXISTS library_metadata_inventory_spool_entries_order;
                 DROP TABLE IF EXISTS library_metadata_inventory_spool_entries;
                 DROP INDEX IF EXISTS library_metadata_inventory_spool_directories_state;
                 DROP TABLE IF EXISTS library_metadata_inventory_spool_directories;
                 DROP TABLE IF EXISTS library_metadata_inventory_spools;
                 DROP TABLE IF EXISTS library_metadata_inventory_spool_contract;
                 DROP TRIGGER IF EXISTS library_metadata_inventory_candidate_owner_update_guard;
                 DROP TRIGGER IF EXISTS library_metadata_inventory_candidate_owner_insert_guard;
                 DROP INDEX IF EXISTS library_metadata_inventory_candidate_owners_change;
                 DROP TABLE IF EXISTS library_metadata_inventory_candidate_owners;
                 DROP INDEX IF EXISTS library_metadata_inventory_frontier_state;
                 DROP TABLE IF EXISTS library_metadata_inventory_frontier;
                 DROP TABLE IF EXISTS library_recovery_execution_contract;
                 DROP TRIGGER IF EXISTS library_persistent_journal_baseline_update_guard;
                 DROP TRIGGER IF EXISTS library_persistent_journal_baseline_insert_guard;
                 DROP INDEX IF EXISTS library_persistent_journal_baselines_root;
                 DROP TABLE IF EXISTS library_persistent_journal_baselines;
                 DROP TRIGGER IF EXISTS library_recovery_authority_update_guard;
                 DROP TRIGGER IF EXISTS library_recovery_authority_insert_guard;
                 DROP INDEX IF EXISTS library_recovery_authorities_root;
                 DROP TABLE IF EXISTS library_recovery_authorities;
                 DROP TABLE IF EXISTS library_recovery_authority_contract;
                 DROP TRIGGER IF EXISTS library_change_queue_lane_origin_update;
                 DROP TRIGGER IF EXISTS library_change_queue_lane_insert;
                 DROP TRIGGER IF EXISTS library_change_queue_lane_update_guard;
                 DROP TRIGGER IF EXISTS library_change_queue_lane_insert_guard;
                 DROP INDEX IF EXISTS library_change_queue_lanes_eligible;
                 DROP TABLE IF EXISTS library_change_queue_lanes;
                 DROP TABLE IF EXISTS library_change_lane_contract;",
            )
            .expect("remove v25 change-lane fixture");
    }

    fn legacy_superseded_inventory_authority_catalog(starting_version: i64) -> NamedTempFile {
        let catalog = NamedTempFile::new().expect("legacy inventory catalog");
        let mut connection =
            Connection::open(catalog.path()).expect("open legacy inventory catalog");
        migrate_schema(&mut connection).expect("fresh current catalog");
        connection
            .execute_batch(
                "INSERT INTO library_roots(id, path, active_scan_id, created_unix_ms)
                   VALUES ('root-a', 'C:/source', 'published-scan', 1);
                 INSERT INTO scan_runs(
                   id, root_id, status, started_unix_ms, completed_unix_ms, preview_edge
                 ) VALUES ('published-scan', 'root-a', 'completed', 1, 2, 128);
                 INSERT INTO library_change_root_state(
                   root_id, generation, is_active, updated_unix_ms
                 ) VALUES ('root-a', 1, 1, 2);
                 INSERT INTO library_persistent_journal_root_state(
                   root_id, root_generation, protocol_version, contract_version,
                   capability_state, continuity_state, updated_unix_ms
                 ) VALUES (
                   'root-a', 1, 0, 1, 'unknown', 'baseline_required', 2
                 );
                 INSERT INTO library_metadata_inventory_runs(
                   id, root_id, root_generation, epoch, scope_kind, scope_relative_path,
                   status, next_page_index, staged_entry_count, enumeration_complete,
                   absence_authority, started_unix_ms, updated_unix_ms, completed_unix_ms
                 ) VALUES (
                   'completed-inventory', 'root-a', 1, 1, 'root', '', 'completed', 2,
                   0, 1, 1, 1, 2, 2
                 );
                 INSERT INTO library_metadata_inventory_runs(
                   id, root_id, root_generation, epoch, scope_kind, scope_relative_path,
                   status, next_page_index, staged_entry_count, enumeration_complete,
                   absence_authority, started_unix_ms, updated_unix_ms,
                   last_issue_code, last_issue_message
                 ) VALUES (
                   'superseded-inventory', 'root-a', 1, 2, 'root', '', 'superseded', 2,
                   1, 1, 1, 3, 4, 'metadata_inventory_newer_epoch',
                   'A newer metadata inventory superseded this run'
                 );
                 INSERT INTO library_metadata_inventory_entries(
                   run_id, relative_path, entry_kind, file_size, modified_unix_ms,
                   placeholder_state, is_reparse_point, staged_page_index, staged_unix_ms
                 ) VALUES (
                   'superseded-inventory', 'retained.jpg', 'file', 123, 3,
                   'available', 0, 1, 3
                 );",
            )
            .expect("legacy superseded inventory authority fixture");
        remove_change_lane_v25_contract_for_test(&connection);
        match starting_version {
            23 => connection
                .execute_batch(
                    "DROP TRIGGER library_persistent_journal_source_range_id_insert;
                     DROP TRIGGER library_persistent_journal_source_range_id_update;
                     ALTER TABLE library_persistent_journal_source_ranges
                       DROP COLUMN canonical_payload;
                     ALTER TABLE library_persistent_journal_cross_root_lineage
                       DROP COLUMN journal_id;
                     ALTER TABLE library_persistent_journal_cross_root_lineage
                       DROP COLUMN old_usn;
                     ALTER TABLE library_persistent_journal_cross_root_lineage
                       DROP COLUMN new_usn;
                     ALTER TABLE library_persistent_journal_cross_root_lineage
                       DROP COLUMN previous_carry_id;
                     UPDATE schema_info SET version = 23;",
                )
                .expect("exact v23 inventory authority fixture"),
            24 => {
                connection
                    .execute("UPDATE schema_info SET version = 24", [])
                    .expect("exact v24 inventory authority fixture");
            }
            _ => panic!("unsupported legacy inventory fixture version {starting_version}"),
        }
        drop(connection);
        catalog
    }

    fn legacy_unfinished_inventory_catalog(starting_version: i64) -> NamedTempFile {
        let catalog = legacy_superseded_inventory_authority_catalog(starting_version);
        let connection =
            Connection::open(catalog.path()).expect("open unfinished inventory catalog");
        connection
            .execute_batch(
                "INSERT INTO library_metadata_inventory_runs(
                   id, root_id, root_generation, epoch, scope_kind, scope_relative_path,
                   status, next_page_index, staged_entry_count, enumeration_complete,
                   absence_authority, started_unix_ms, updated_unix_ms,
                   last_issue_code, last_issue_message
                 ) VALUES (
                   'unfinished-inventory', 'root-a', 1, 3, 'root', '', 'comparing', 2,
                   1, 1, 0, 5, 6, 'legacy_inventory_interrupted',
                   'Legacy inventory was interrupted before migration'
                 );
                 INSERT INTO library_metadata_inventory_entries(
                   run_id, relative_path, entry_kind, file_size, modified_unix_ms,
                   placeholder_state, is_reparse_point, staged_page_index, staged_unix_ms
                 ) VALUES (
                   'unfinished-inventory', 'unfinished.jpg', 'file', 456, 5,
                   'available', 0, 1, 5
                 );",
            )
            .expect("legacy unfinished inventory fixture");
        drop(connection);
        catalog
    }

    fn current_terminal_inventory_authority_catalog() -> NamedTempFile {
        let catalog = NamedTempFile::new().expect("current terminal inventory catalog");
        let mut connection =
            Connection::open(catalog.path()).expect("open current terminal inventory catalog");
        migrate_schema(&mut connection).expect("fresh current catalog");
        connection
            .execute_batch(
                "INSERT INTO library_roots(id, path, active_scan_id, created_unix_ms)
                   VALUES ('root-a', 'C:/source', 'published-scan', 1);
                 INSERT INTO scan_runs(
                   id, root_id, status, started_unix_ms, completed_unix_ms, preview_edge
                 ) VALUES ('published-scan', 'root-a', 'completed', 1, 2, 128);
                 INSERT INTO library_change_root_state(
                   root_id, generation, is_active, updated_unix_ms
                 ) VALUES ('root-a', 1, 1, 2);
                 INSERT INTO library_persistent_journal_root_state(
                   root_id, root_generation, protocol_version, contract_version,
                   capability_state, continuity_state, updated_unix_ms
                 ) VALUES (
                   'root-a', 1, 0, 1, 'unknown', 'baseline_required', 2
                 );
                 INSERT INTO library_metadata_inventory_runs(
                   id, root_id, root_generation, epoch, scope_kind, scope_relative_path,
                   status, next_page_index, staged_entry_count, enumeration_complete,
                   absence_authority, started_unix_ms, updated_unix_ms, completed_unix_ms,
                   last_issue_code, last_issue_message
                 ) VALUES (
                   'completed-inventory', 'root-a', 1, 1, 'root', '', 'completed', 2,
                   1, 1, 1, 1, 2, 2, 'completed_inventory_note',
                   'Completed inventory evidence must remain intact'
                 );
                 INSERT INTO library_metadata_inventory_entries(
                   run_id, relative_path, entry_kind, file_size, modified_unix_ms,
                   placeholder_state, is_reparse_point, staged_page_index, staged_unix_ms
                 ) VALUES (
                   'completed-inventory', 'completed.jpg', 'file', 123, 1,
                   'available', 0, 1, 1
                 );
                 INSERT INTO library_metadata_inventory_runs(
                   id, root_id, root_generation, epoch, scope_kind, scope_relative_path,
                   status, next_page_index, staged_entry_count, enumeration_complete,
                   absence_authority, started_unix_ms, updated_unix_ms,
                   last_issue_code, last_issue_message
                 ) VALUES (
                   'terminal-inventory', 'root-a', 1, 2, 'root', '', 'superseded', 2,
                   1, 1, 1, 3, 4, 'metadata_inventory_newer_epoch',
                   'A newer metadata inventory superseded this run'
                 );
                 INSERT INTO library_metadata_inventory_entries(
                   run_id, relative_path, entry_kind, file_size, modified_unix_ms,
                   placeholder_state, is_reparse_point, staged_page_index, staged_unix_ms
                 ) VALUES (
                   'terminal-inventory', 'retained.jpg', 'file', 456, 3,
                   'available', 0, 1, 3
                 );
                 INSERT INTO library_metadata_inventory_runs(
                   id, root_id, root_generation, epoch, scope_kind, scope_relative_path,
                   status, next_page_index, staged_entry_count, enumeration_complete,
                   absence_authority, started_unix_ms, updated_unix_ms
                 ) VALUES (
                   'active-inventory', 'root-a', 1, 3, 'root', '', 'running', 1,
                   0, 0, 0, 5, 5
                 );
                 INSERT INTO library_change_queue(
                   id, root_id, root_generation, intent_kind, scope, relative_path,
                   origin, first_observed_unix_ms, most_recent_observed_unix_ms,
                   first_sequence, most_recent_sequence, coalesced_observation_count,
                   status, ready_unix_ms, catalog_revision_at_enqueue,
                   created_unix_ms, updated_unix_ms
                 ) VALUES (
                   301, 'root-a', 1, 'freshness_unknown', 'root', '',
                   'consistency_audit', 5, 5, '3', '3', 1, 'pending', 5, 0, 5, 5
                 );
                 INSERT INTO library_recovery_authorities(
                   change_id, run_id, root_id, root_generation, reason, authorized_unix_ms
                 ) VALUES (
                   301, 'active-inventory', 'root-a', 1, 'containment_failure', 5
                 );
                 INSERT INTO library_metadata_inventory_runs(
                   id, root_id, root_generation, epoch, scope_kind, scope_relative_path,
                   status, next_page_index, staged_entry_count, enumeration_complete,
                   absence_authority, started_unix_ms, updated_unix_ms,
                   last_issue_code, last_issue_message
                 ) VALUES (
                   'terminal-spool-inventory', 'root-a', 1, 4, 'root', '', 'failed', 2,
                   0, 1, 1, 6, 7, 'legacy_terminal_spool',
                   'Legacy terminal inventory retained a derived source spool'
                 );
                 INSERT INTO library_change_queue(
                   id, root_id, root_generation, intent_kind, scope, relative_path,
                   origin, first_observed_unix_ms, most_recent_observed_unix_ms,
                   first_sequence, most_recent_sequence, coalesced_observation_count,
                   status, ready_unix_ms, catalog_revision_at_enqueue,
                   created_unix_ms, updated_unix_ms
                 ) VALUES (
                   302, 'root-a', 1, 'freshness_unknown', 'root', '',
                   'consistency_audit', 6, 6, '4', '4', 1, 'pending', 6, 0, 6, 6
                 );
                 INSERT INTO library_recovery_authorities(
                   change_id, run_id, root_id, root_generation, reason, authorized_unix_ms
                 ) VALUES (
                   302, 'terminal-spool-inventory', 'root-a', 1, 'containment_failure', 6
                 );
                 INSERT INTO library_metadata_inventory_spools(
                   run_id, authority_change_id, root_id, root_generation,
                   root_identity_scheme, root_identity_value,
                   scope_kind, scope_relative_path, state,
                   created_unix_ms, updated_unix_ms
                 ) VALUES (
                   'terminal-spool-inventory', 302, 'root-a', 1,
                   'windows-file-id-128-v1',
                   '000000000000004d:ffffffffffffffffffffffffffffffff',
                   'root', '', 'enumerating', 6, 6
                 );
                 INSERT INTO library_metadata_inventory_spool_directories(
                   run_id, ordinal, relative_directory, state,
                   created_unix_ms, updated_unix_ms
                 ) VALUES (
                   'terminal-spool-inventory', 0, '', 'pending', 6, 6
                 );",
            )
            .expect("polluted current terminal inventory fixture");
        drop(connection);
        catalog
    }

    fn seed_current_explicit_live_gap_claim(
        connection: &Connection,
        root_id: &str,
        gap_change_id: i64,
    ) {
        connection
            .execute_batch("PRAGMA foreign_keys = ON")
            .expect("enable explicit live-gap foreign keys");
        let root_path = format!("C:/retired-live-gap/{root_id}");
        connection
            .execute(
                "INSERT INTO library_roots(id, path, created_unix_ms) VALUES (?1, ?2, 1)",
                rusqlite::params![root_id, root_path],
            )
            .expect("insert explicit live-gap root");
        connection
            .execute(
                "INSERT INTO library_change_root_state(
                   root_id, generation, is_active, updated_unix_ms
                 ) VALUES (?1, 1, 1, 1)",
                [root_id],
            )
            .expect("insert explicit live-gap generation");
        connection
            .execute(
                "INSERT INTO library_persistent_journal_root_state(
                   root_id, root_generation, protocol_version, contract_version,
                   capability_state, continuity_state, updated_unix_ms
                 ) VALUES (?1, 1, 0, 1, 'unknown', 'baseline_required', 1)",
                [root_id],
            )
            .expect("insert explicit live-gap journal root");
        connection
            .execute(
                "INSERT INTO library_change_queue(
                   id, root_id, root_generation, intent_kind, scope, relative_path,
                   origin, first_observed_unix_ms, most_recent_observed_unix_ms,
                   first_sequence, most_recent_sequence, coalesced_observation_count,
                   status, ready_unix_ms, attempt_count, next_retry_unix_ms,
                   last_failure_code, last_failure_message,
                   catalog_revision_at_enqueue, created_unix_ms, updated_unix_ms
                 ) VALUES (
                   ?1, ?2, 1, 'freshness_unknown', 'root', '', 'startup_catch_up',
                   2, 2, ?3, ?3, 1, 'retry_wait', 2, 0, NULL,
                   'live_gap_v30_explicit_recovery_required',
                   'The retained gap requires an explicit library update',
                   0, 2, 2
                 )",
                rusqlite::params![gap_change_id, root_id, gap_change_id.to_string()],
            )
            .expect("insert explicit live-gap row");
        connection
            .execute(
                "INSERT INTO library_live_gap_recovery_claims(
                   gap_change_id, root_id, root_generation, consumer_kind,
                   created_unix_ms
                 ) VALUES (?1, ?2, 1, 'explicit_recovery_required', 2)",
                rusqlite::params![gap_change_id, root_id],
            )
            .expect("insert explicit live-gap claim");
    }

    fn seed_current_pending_journal_live_gap_claim(
        connection: &Connection,
        root_id: &str,
        gap_change_id: i64,
    ) {
        connection
            .execute_batch("PRAGMA foreign_keys = ON")
            .expect("enable pending-journal foreign keys");
        let root_path = format!("C:/retired-live-gap/{root_id}");
        connection
            .execute(
                "INSERT INTO library_roots(id, path, created_unix_ms) VALUES (?1, ?2, 1)",
                rusqlite::params![root_id, root_path],
            )
            .expect("insert pending-journal root");
        connection
            .execute(
                "INSERT INTO library_change_root_state(
                   root_id, generation, is_active, updated_unix_ms
                 ) VALUES (?1, 1, 1, 1)",
                [root_id],
            )
            .expect("insert pending-journal generation");
        connection
            .execute(
                "INSERT INTO library_persistent_journal_root_state(
                   root_id, root_generation, protocol_version, contract_version,
                   capability_state, continuity_state, updated_unix_ms
                 ) VALUES (?1, 1, 5, 1, 'supported', 'current', 1)",
                [root_id],
            )
            .expect("insert pending-journal root authority");
        connection
            .execute(
                "INSERT INTO library_persistent_journal_checkpoints(
                   root_id, root_generation, volume_guid, volume_serial,
                   root_reference_version, root_file_reference, journal_id,
                   next_unread_usn, captured_exclusive_end, covered_catalog_revision,
                   protocol_version, contract_version, continuity_state, updated_unix_ms
                 ) VALUES (
                   ?1, 1, 'volume-guid', '77', 3, ?2, '44', '55', '55', 0,
                   5, 1, 'current', 1
                 )",
                rusqlite::params![root_id, vec![1_u8; 16]],
            )
            .expect("insert pending-journal checkpoint");
        connection
            .execute(
                "INSERT INTO library_change_queue(
                   id, root_id, root_generation, intent_kind, scope, relative_path,
                   origin, first_observed_unix_ms, most_recent_observed_unix_ms,
                   first_sequence, most_recent_sequence, coalesced_observation_count,
                   status, ready_unix_ms, attempt_count, next_retry_unix_ms,
                   last_failure_code, last_failure_message,
                   catalog_revision_at_enqueue, created_unix_ms, updated_unix_ms
                 ) VALUES (
                   ?1, ?2, 1, 'freshness_unknown', 'root', '', 'live_notification',
                   2, 2, ?3, ?3, 1, 'retry_wait', 2, 0, 3,
                   'live_gap_waiting_for_journal_range',
                   'The live gap is waiting for its durable journal range',
                   0, 2, 2
                 )",
                rusqlite::params![gap_change_id, root_id, gap_change_id.to_string()],
            )
            .expect("insert pending-journal gap");
        connection
            .execute(
                "INSERT INTO library_live_gap_recovery_claims(
                   gap_change_id, root_id, root_generation, consumer_kind,
                   opening_volume_guid, opening_volume_serial,
                   opening_root_reference_version, opening_root_file_reference,
                   opening_journal_id, opening_next_usn, protocol_version,
                   contract_version, created_unix_ms
                 ) VALUES (
                   ?1, ?2, 1, 'pending_journal', 'volume-guid', '77', 3, ?3,
                   '44', '55', 5, 1, 2
                 )",
                rusqlite::params![gap_change_id, root_id, vec![1_u8; 16]],
            )
            .expect("insert pending-journal claim");
    }

    fn retire_live_gap_root(connection: &Connection, root_id: &str, gap_change_id: i64) {
        connection
            .execute(
                "UPDATE library_change_root_state
                 SET is_active = 0, updated_unix_ms = 3 WHERE root_id = ?1",
                [root_id],
            )
            .expect("retire live-gap generation");
        connection
            .execute(
                "UPDATE library_change_queue
                 SET status = 'superseded', next_retry_unix_ms = NULL,
                     lease_expires_unix_ms = NULL, superseded_by_change_id = NULL,
                     updated_unix_ms = 3
                 WHERE id = ?1",
                [gap_change_id],
            )
            .expect("supersede retired live gap");
        connection
            .execute("DELETE FROM library_roots WHERE id = ?1", [root_id])
            .expect("remove retired live-gap root");
    }

    fn advance_live_gap_root_generation(
        connection: &Connection,
        root_id: &str,
        gap_change_id: i64,
    ) {
        connection
            .execute(
                "UPDATE library_change_queue
                 SET status = 'superseded', next_retry_unix_ms = NULL,
                     lease_expires_unix_ms = NULL, superseded_by_change_id = NULL,
                     updated_unix_ms = 3
                 WHERE id = ?1",
                [gap_change_id],
            )
            .expect("supersede previous-generation live gap");
        connection
            .execute(
                "UPDATE library_persistent_journal_root_state
                 SET capability_state = CASE
                       WHEN protocol_version = 0 THEN 'unknown' ELSE 'live_only' END,
                     continuity_state = 'unavailable',
                     last_failure_code = 'root_generation_retired',
                     last_failure_message = 'The root generation was retired',
                     updated_unix_ms = 3
                 WHERE root_id = ?1 AND root_generation = 1",
                [root_id],
            )
            .expect("retire previous journal generation");
        connection
            .execute(
                "UPDATE library_change_root_state
                 SET generation = 2, is_active = 1, updated_unix_ms = 3
                 WHERE root_id = ?1",
                [root_id],
            )
            .expect("advance live-gap root generation");
        connection
            .execute(
                "INSERT INTO library_persistent_journal_root_state(
                   root_id, root_generation, protocol_version, contract_version,
                   capability_state, continuity_state, updated_unix_ms
                 ) VALUES (?1, 2, 0, 1, 'unknown', 'baseline_required', 3)",
                [root_id],
            )
            .expect("insert current journal generation");
    }

    fn assert_retired_live_gap_claim_repaired(connection: &Connection, gap_change_id: i64) {
        let evidence = connection
            .query_row(
                "SELECT gap.status, gap.superseded_by_change_id,
                        gap.last_failure_code,
                        (SELECT COUNT(*) FROM library_live_gap_recovery_claims AS claim
                         WHERE claim.gap_change_id = gap.id)
                 FROM library_change_queue AS gap WHERE gap.id = ?1",
                [gap_change_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, Option<i64>>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, i64>(3)?,
                    ))
                },
            )
            .expect("retained terminal live-gap evidence");
        assert_eq!(evidence.0, "superseded");
        assert_eq!(evidence.1, None);
        assert!(matches!(
            evidence.2.as_str(),
            "live_gap_v30_explicit_recovery_required" | "live_gap_waiting_for_journal_range"
        ));
        assert_eq!(evidence.3, 0);
    }

    fn assert_superseded_inventory_authority_retired(connection: &Connection) {
        let retained: (i64, String, i64, Option<String>, i64, i64) = connection
            .query_row(
                "SELECT
                   (SELECT version FROM schema_info),
                   (SELECT status FROM library_metadata_inventory_runs
                    WHERE id = 'superseded-inventory'),
                   (SELECT absence_authority FROM library_metadata_inventory_runs
                    WHERE id = 'superseded-inventory'),
                   (SELECT last_issue_code FROM library_metadata_inventory_runs
                    WHERE id = 'superseded-inventory'),
                   (SELECT COUNT(*) FROM library_metadata_inventory_entries
                    WHERE run_id = 'superseded-inventory' AND relative_path = 'retained.jpg'),
                   (SELECT absence_authority FROM library_metadata_inventory_runs
                    WHERE id = 'completed-inventory')",
                [],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                    ))
                },
            )
            .expect("retained inventory authority evidence");
        assert_eq!(
            retained,
            (
                SCHEMA_VERSION,
                "superseded".to_owned(),
                0,
                Some("metadata_inventory_newer_epoch".to_owned()),
                1,
                1,
            )
        );
    }

    fn assert_unfinished_inventory_retired(connection: &Connection) {
        let retained: (
            i64,
            String,
            i64,
            Option<String>,
            Option<String>,
            i64,
            String,
            i64,
        ) = connection
            .query_row(
                "SELECT
                       (SELECT version FROM schema_info),
                       (SELECT status FROM library_metadata_inventory_runs
                        WHERE id = 'unfinished-inventory'),
                       (SELECT absence_authority FROM library_metadata_inventory_runs
                        WHERE id = 'unfinished-inventory'),
                       (SELECT last_issue_code FROM library_metadata_inventory_runs
                        WHERE id = 'unfinished-inventory'),
                       (SELECT last_issue_message FROM library_metadata_inventory_runs
                        WHERE id = 'unfinished-inventory'),
                       (SELECT COUNT(*) FROM library_metadata_inventory_entries
                        WHERE run_id = 'unfinished-inventory'
                          AND relative_path = 'unfinished.jpg'),
                       (SELECT status FROM library_metadata_inventory_runs
                        WHERE id = 'completed-inventory'),
                       (SELECT absence_authority FROM library_metadata_inventory_runs
                        WHERE id = 'completed-inventory')",
                [],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                        row.get(6)?,
                        row.get(7)?,
                    ))
                },
            )
            .expect("retained unfinished inventory evidence");
        assert_eq!(
            retained,
            (
                SCHEMA_VERSION,
                "superseded".to_owned(),
                0,
                Some("legacy_inventory_interrupted".to_owned()),
                Some("Legacy inventory was interrupted before migration".to_owned()),
                1,
                "completed".to_owned(),
                1,
            )
        );
    }

    fn assert_current_terminal_inventory_repaired(connection: &Connection) {
        let terminal: (String, i64, Option<String>, Option<String>, i64) = connection
            .query_row(
                "SELECT
                   (SELECT status FROM library_metadata_inventory_runs
                    WHERE id = 'terminal-inventory'),
                   (SELECT absence_authority FROM library_metadata_inventory_runs
                    WHERE id = 'terminal-inventory'),
                   (SELECT last_issue_code FROM library_metadata_inventory_runs
                    WHERE id = 'terminal-inventory'),
                   (SELECT last_issue_message FROM library_metadata_inventory_runs
                    WHERE id = 'terminal-inventory'),
                   (SELECT COUNT(*) FROM library_metadata_inventory_entries
                    WHERE run_id = 'terminal-inventory' AND relative_path = 'retained.jpg')",
                [],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                    ))
                },
            )
            .expect("retained terminal inventory evidence");
        assert_eq!(
            terminal,
            (
                "superseded".to_owned(),
                0,
                Some("metadata_inventory_newer_epoch".to_owned()),
                Some("A newer metadata inventory superseded this run".to_owned()),
                1,
            )
        );

        let completed: (String, i64, Option<String>, i64) = connection
            .query_row(
                "SELECT
                   (SELECT status FROM library_metadata_inventory_runs
                    WHERE id = 'completed-inventory'),
                   (SELECT absence_authority FROM library_metadata_inventory_runs
                    WHERE id = 'completed-inventory'),
                   (SELECT last_issue_code FROM library_metadata_inventory_runs
                    WHERE id = 'completed-inventory'),
                   (SELECT COUNT(*) FROM library_metadata_inventory_entries
                    WHERE run_id = 'completed-inventory' AND relative_path = 'completed.jpg')",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .expect("retained completed inventory evidence");
        assert_eq!(
            completed,
            (
                "completed".to_owned(),
                1,
                Some("completed_inventory_note".to_owned()),
                1,
            )
        );

        let active: (String, i64, i64) = connection
            .query_row(
                "SELECT
                   (SELECT status FROM library_metadata_inventory_runs
                    WHERE id = 'active-inventory'),
                   (SELECT absence_authority FROM library_metadata_inventory_runs
                    WHERE id = 'active-inventory'),
                   (SELECT COUNT(*) FROM library_recovery_authorities
                    WHERE run_id = 'active-inventory' AND retired_unix_ms IS NULL)",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("retained active inventory evidence");
        assert_eq!(active, ("running".to_owned(), 0, 1));
        let terminal_spool: (String, i64, Option<String>, i64, i64, i64) = connection
            .query_row(
                "SELECT
                   (SELECT status FROM library_metadata_inventory_runs
                    WHERE id = 'terminal-spool-inventory'),
                   (SELECT absence_authority FROM library_metadata_inventory_runs
                    WHERE id = 'terminal-spool-inventory'),
                   (SELECT last_issue_code FROM library_metadata_inventory_runs
                    WHERE id = 'terminal-spool-inventory'),
                   (SELECT COUNT(*) FROM library_metadata_inventory_spools
                    WHERE run_id = 'terminal-spool-inventory'),
                   (SELECT COUNT(*) FROM library_metadata_inventory_spool_directories
                    WHERE run_id = 'terminal-spool-inventory'),
                   (SELECT COUNT(*) FROM library_recovery_authorities
                    WHERE run_id = 'terminal-spool-inventory' AND retired_unix_ms IS NULL)",
                [],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                    ))
                },
            )
            .expect("retained terminal spool evidence");
        assert_eq!(
            terminal_spool,
            (
                "failed".to_owned(),
                1,
                Some("legacy_terminal_spool".to_owned()),
                0,
                0,
                1,
            )
        );
    }

    fn remove_metadata_inventory_spool_v27_contract_for_test(connection: &Connection) {
        remove_root_publication_namespace_v29_contract_for_test(connection);
        connection
            .execute_batch(
                "DROP TRIGGER IF EXISTS library_metadata_inventory_spool_directory_complete_guard;
                 DROP TRIGGER IF EXISTS library_metadata_inventory_spool_binding_update_guard;
                 DROP INDEX IF EXISTS library_metadata_inventory_spool_entries_order;
                 DROP TABLE IF EXISTS library_metadata_inventory_spool_entries;
                 DROP INDEX IF EXISTS library_metadata_inventory_spool_directories_state;
                 DROP TABLE IF EXISTS library_metadata_inventory_spool_directories;
                 DROP TABLE IF EXISTS library_metadata_inventory_spools;
                 DROP TABLE IF EXISTS library_metadata_inventory_spool_contract;",
            )
            .expect("remove v27 inventory spool fixture");
    }

    fn replace_current_spool_contract_with_v27_for_test(connection: &Connection) {
        remove_metadata_inventory_spool_v27_contract_for_test(connection);
        for sql in [
            super::METADATA_INVENTORY_SPOOL_CONTRACT_TABLE_V27_DDL,
            super::METADATA_INVENTORY_SPOOL_TABLE_V27_DDL,
            super::METADATA_INVENTORY_SPOOL_DIRECTORY_TABLE_DDL,
            super::METADATA_INVENTORY_SPOOL_DIRECTORY_STATE_INDEX_DDL,
            super::METADATA_INVENTORY_SPOOL_ENTRY_TABLE_DDL,
            super::METADATA_INVENTORY_SPOOL_ENTRY_ORDER_INDEX_DDL,
            super::METADATA_INVENTORY_SPOOL_BINDING_UPDATE_GUARD_V27_DDL,
            super::METADATA_INVENTORY_SPOOL_DIRECTORY_COMPLETE_GUARD_DDL,
        ] {
            connection
                .execute_batch(sql)
                .expect("create exact v27 spool shape");
        }
        connection
            .execute_batch(
                "INSERT INTO library_metadata_inventory_spool_contract(
                   singleton, contract_version, complete
                 ) VALUES (1, 1, 1);
                 PRAGMA user_version = 27;
                 UPDATE schema_info SET version = 27;",
            )
            .expect("publish v27 spool fixture");
        super::validate_metadata_inventory_spool_contract_version(connection, 27, 1)
            .expect("validate exact v27 spool fixture");
    }

    fn remove_recovery_execution_v26_contract_for_test(connection: &Connection) {
        connection
            .execute_batch(
                "DROP TRIGGER IF EXISTS library_metadata_inventory_candidate_owner_update_guard;
                 DROP TRIGGER IF EXISTS library_metadata_inventory_candidate_owner_insert_guard;
                 DROP INDEX IF EXISTS library_metadata_inventory_candidate_owners_change;
                 DROP TABLE IF EXISTS library_metadata_inventory_candidate_owners;
                 DROP INDEX IF EXISTS library_metadata_inventory_frontier_state;
                 DROP TABLE IF EXISTS library_metadata_inventory_frontier;
                 DROP TABLE IF EXISTS library_recovery_execution_contract;",
            )
            .expect("remove v26 recovery execution fixture");
    }

    fn fresh_v19_catalog() -> Connection {
        let mut connection = Connection::open_in_memory().expect("catalog");
        let transaction = connection.transaction().expect("v19 transaction");
        create_schema_v19(&transaction).expect("fresh v19 schema");
        transaction.commit().expect("commit fresh v19 schema");
        connection
    }

    fn fresh_v30_catalog() -> Connection {
        let mut connection = fresh_v19_catalog();
        super::migrate_v19_to_v20(&mut connection).expect("v20");
        super::migrate_v20_to_v21(&mut connection).expect("v21");
        super::migrate_v21_to_v22(&mut connection).expect("v22");
        super::migrate_v22_to_v23(&mut connection).expect("v23");
        super::migrate_v23_to_v24(&mut connection).expect("v24");
        super::migrate_v24_to_v25(&mut connection).expect("v25");
        super::migrate_v25_to_v26(&mut connection).expect("v26");
        super::migrate_v26_to_v27(&mut connection).expect("v27");
        super::migrate_v27_to_v28(&mut connection).expect("v28");
        super::migrate_v28_to_v29(&mut connection).expect("v29");
        super::migrate_v29_to_v30(&mut connection).expect("v30");
        connection
    }

    #[test]
    fn v30_to_v31_adds_source_revision_contract_without_inventing_revision_evidence() {
        let mut connection = fresh_v30_catalog();
        connection
            .execute_batch(
                "INSERT INTO library_roots(id, path, active_scan_id, created_unix_ms)
                   VALUES ('root', 'C:/source', 'scan', 1);
                 INSERT INTO scan_runs(
                   id, root_id, status, started_unix_ms, completed_unix_ms,
                   asset_count, preview_edge
                 ) VALUES ('scan', 'root', 'completed', 1, 2, 2, 256);
                 INSERT INTO assets(id, created_unix_ms) VALUES ('asset-a', 1), ('asset-b', 1);
                 INSERT INTO asset_locations(
                   scan_id, asset_id, location_id, root_id, absolute_path, relative_path,
                   preview_path, file_size, modified_unix_ms, width, height, preview_status,
                   metadata_engine_id, metadata_engine_version, capture_local_time,
                   capture_time_source, capture_raw_value,
                   file_identity_scheme, file_identity_value
                 ) VALUES
                   ('scan', 'asset-a', 'location-a', 'root', 'C:/source/a.png', 'a.png',
                    'C:/cache/a.jpg', 10, 20, 1, 1, 'ready',
                    'legacy-metadata', '7', '2020-01-02T03:04:05',
                    'exif_original', '2020:01:02 03:04:05',
                    'windows-file-id-128-v1', '0000000000000001:00000000000000000000000000000001'),
                   ('scan', 'asset-b', 'location-b', 'root', 'C:/source/b.png', 'b.png',
                    'C:/cache/b.jpg', 10, 20, 1, 1, 'ready',
                    'legacy-metadata', '7', '2020-01-02T03:04:05',
                    'exif_original', '2020:01:02 03:04:05',
                    'windows-file-id-128-v1', '0000000000000001:00000000000000000000000000000001');
                 INSERT INTO preview_artifacts(
                   artifact_key, source_file_size, source_modified_unix_ms,
                   algorithm_id, algorithm_version, orientation_contract, size_bucket,
                   encoded_width, encoded_height, artifact_path, byte_size, lifecycle_state,
                   created_unix_ms, last_used_unix_ms
                 ) VALUES ('artifact', 10, 20, 'preview', 1, 'orientation', 256,
                           1, 1, 'C:/cache/a.jpg', 10, 'ready', 1, 1);
                 INSERT INTO preview_artifact_locations(artifact_key, location_id)
                   VALUES ('artifact', 'location-a');",
            )
            .expect("v30 source fixture");

        super::reset_source_revision_row_audit_count();
        super::reset_current_schema_row_audit_count();
        super::migrate_v30_to_v31(&mut connection).expect("v31");
        assert_eq!(super::source_revision_row_audit_count(), 1);
        assert!(super::current_schema_row_audit_count() > 0);

        assert_eq!(
            super::schema_version(&connection).expect("schema version"),
            31
        );
        assert_eq!(
            connection
                .query_row("PRAGMA user_version", [], |row| row.get::<_, i64>(0))
                .expect("user version"),
            31
        );
        assert_eq!(
            connection
                .query_row(
                    "SELECT contract_version FROM library_metadata_inventory_spool_contract",
                    [],
                    |row| row.get::<_, i64>(0),
                )
                .expect("spool contract"),
            3
        );
        let migrated_locations = connection
            .query_row(
                "SELECT COUNT(*), COUNT(DISTINCT source_generation),
                        COUNT(source_revision_token),
                        SUM(preview_status = 'pending' AND preview_path = ''),
                        SUM(metadata_engine_id = 'ame-invalidated-media-metadata'
                            AND metadata_engine_version = '0'),
                        SUM(capture_local_time IS NULL
                            AND capture_time_source IS NULL
                            AND capture_raw_value IS NULL)
                 FROM asset_locations",
                [],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, i64>(2)?,
                        row.get::<_, i64>(3)?,
                        row.get::<_, i64>(4)?,
                        row.get::<_, i64>(5)?,
                    ))
                },
            )
            .expect("migrated locations");
        assert_eq!(migrated_locations, (2, 1, 0, 2, 2, 2));
        assert!(
            super::source_revision_metadata_contract_is_complete(&connection)
                .expect("source metadata contract")
        );
        assert_eq!(
            connection
                .query_row(
                    "SELECT lifecycle_state, source_revision_token, source_generation
                     FROM preview_artifacts WHERE artifact_key = 'artifact'",
                    [],
                    |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, Option<String>>(1)?,
                            row.get::<_, Option<i64>>(2)?,
                        ))
                    },
                )
                .expect("migrated artifact"),
            ("evictable".to_owned(), None, None)
        );
        assert_eq!(
            connection
                .query_row(
                    "SELECT COUNT(*) FROM preview_artifact_locations",
                    [],
                    |row| { row.get::<_, i64>(0) }
                )
                .expect("preview owners"),
            0
        );
        super::validate_current_schema_contract(&connection).expect("source revision contract");
    }

    #[test]
    fn v30_to_v31_quarantines_conflicting_legacy_identity_observations() {
        let mut connection = fresh_v30_catalog();
        connection
            .execute_batch(
                "INSERT INTO library_roots(id, path, active_scan_id, created_unix_ms)
                   VALUES ('root-a', 'C:/source-a', 'scan-a', 1),
                          ('root-b', 'C:/source-b', 'scan-b', 1);
                 INSERT INTO scan_runs(
                   id, root_id, status, started_unix_ms, completed_unix_ms,
                   asset_count, preview_edge
                 ) VALUES ('scan-a', 'root-a', 'completed', 1, 2, 5, 256),
                          ('scan-b', 'root-b', 'completed', 1, 2, 1, 256);
                 INSERT INTO assets(id, created_unix_ms) VALUES
                   ('asset-compatible-a', 1), ('asset-compatible-b', 1),
                   ('asset-scan-conflict-a', 1), ('asset-scan-conflict-b', 1),
                   ('asset-root-conflict-a', 1), ('asset-root-conflict-b', 1);
                 INSERT INTO asset_locations(
                   scan_id, asset_id, location_id, root_id, absolute_path, relative_path,
                   preview_path, file_size, modified_unix_ms, width, height, preview_status,
                   file_identity_scheme, file_identity_value
                 ) VALUES
                   ('scan-a', 'asset-compatible-a', 'compatible-a', 'root-a',
                    'C:/source-a/compatible-a.png', 'compatible-a.png', '',
                    10, 20, 1, 1, 'pending', 'windows-file-id-128-v1',
                    '0000000000000001:00000000000000000000000000000001'),
                   ('scan-a', 'asset-compatible-b', 'compatible-b', 'root-a',
                    'C:/source-a/compatible-b.png', 'compatible-b.png', '',
                    10, 20, 1, 1, 'pending', 'windows-file-id-128-v1',
                    '0000000000000001:00000000000000000000000000000001'),
                   ('scan-a', 'asset-scan-conflict-a', 'scan-conflict-a', 'root-a',
                    'C:/source-a/scan-conflict-a.png', 'scan-conflict-a.png', '',
                    11, 21, 1, 1, 'pending', 'windows-file-id-128-v1',
                    '0000000000000001:00000000000000000000000000000002'),
                   ('scan-a', 'asset-scan-conflict-b', 'scan-conflict-b', 'root-a',
                    'C:/source-a/scan-conflict-b.png', 'scan-conflict-b.png', '',
                    12, 22, 1, 1, 'pending', 'windows-file-id-128-v1',
                    '0000000000000001:00000000000000000000000000000002'),
                   ('scan-a', 'asset-root-conflict-a', 'root-conflict-a', 'root-a',
                    'C:/source-a/root-conflict.png', 'root-conflict.png', '',
                    13, 23, 1, 1, 'pending', 'windows-file-id-128-v1',
                    '0000000000000001:00000000000000000000000000000003'),
                   ('scan-b', 'asset-root-conflict-b', 'root-conflict-b', 'root-b',
                    'C:/source-b/root-conflict.png', 'root-conflict.png', '',
                    14, 24, 1, 1, 'pending', 'windows-file-id-128-v1',
                    '0000000000000001:00000000000000000000000000000003');",
            )
            .expect("v30 conflicting identity fixtures");

        super::migrate_v30_to_v31(&mut connection).expect("v31 conflict-safe migration");

        let compatible = connection
            .query_row(
                "SELECT COUNT(*), COUNT(DISTINCT source_generation)
                 FROM asset_locations
                 WHERE file_identity_value =
                   '0000000000000001:00000000000000000000000000000001'",
                [],
                |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
            )
            .expect("compatible identity group");
        let quarantined = connection
            .query_row(
                "SELECT COUNT(*), COUNT(DISTINCT source_generation)
                 FROM asset_locations
                 WHERE location_id IN (
                   'scan-conflict-a', 'scan-conflict-b',
                   'root-conflict-a', 'root-conflict-b'
                 ) AND file_identity_scheme IS NULL AND file_identity_value IS NULL",
                [],
                |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
            )
            .expect("quarantined identity groups");
        assert_eq!(compatible, (2, 1));
        assert_eq!(quarantined, (4, 4));
        super::validate_current_schema_contract(&connection)
            .expect("conflicting legacy identities no longer block startup");
    }

    fn recovery_lifecycle_catalog(is_completed: bool) -> NamedTempFile {
        let catalog = NamedTempFile::new().expect("temporary recovery lifecycle catalog");
        let mut connection = Connection::open(catalog.path()).expect("recovery lifecycle catalog");
        migrate_schema(&mut connection).expect("fresh current catalog");
        connection
            .execute_batch(
                "INSERT INTO library_roots(id, path, created_unix_ms)
                   VALUES ('lifecycle-root', 'C:/lifecycle-source', 1);
                 INSERT INTO library_change_root_state(
                   root_id, generation, is_active, updated_unix_ms
                 ) VALUES ('lifecycle-root', 1, 1, 1);
                 INSERT INTO library_persistent_journal_root_state(
                   root_id, root_generation, protocol_version, contract_version,
                   capability_state, continuity_state, updated_unix_ms
                 ) VALUES (
                   'lifecycle-root', 1, 5, 1, 'supported', 'baseline_required', 1
                 );
                 INSERT INTO library_change_queue(
                   id, root_id, root_generation, intent_kind, scope, relative_path,
                   origin, first_observed_unix_ms, most_recent_observed_unix_ms,
                   first_sequence, most_recent_sequence, coalesced_observation_count,
                   status, ready_unix_ms, catalog_revision_at_enqueue,
                   created_unix_ms, updated_unix_ms
                 ) VALUES (
                   901, 'lifecycle-root', 1, 'freshness_unknown', 'root', '',
                   'consistency_audit', 1, 1, '1', '1', 1,
                   'pending', 1, 0, 1, 1
                 );
                 INSERT INTO library_recovery_authorities(
                   change_id, run_id, root_id, root_generation, reason,
                   opening_journal_id, opening_next_usn, authorized_unix_ms
                 ) VALUES (
                   901, 'lifecycle-run', 'lifecycle-root', 1,
                   'existing_root_baseline', '44', '10', 1
                 );
                 INSERT INTO library_persistent_journal_baselines(
                   change_id, root_id, root_generation, volume_guid, volume_serial,
                   root_reference_version, root_file_reference, journal_id,
                   opening_next_usn, protocol_version, contract_version,
                   phase, authorized_unix_ms, updated_unix_ms
                 ) VALUES (
                   901, 'lifecycle-root', 1, 'lifecycle-volume', '77',
                   3, X'01010101010101010101010101010101', '44',
                   '10', 5, 1, 'inventory', 1, 1
                 );",
            )
            .expect("valid active recovery lifecycle");
        migrate_schema(&mut connection).expect("validate active recovery lifecycle");
        if is_completed {
            connection
                .execute_batch(
                    "INSERT INTO library_persistent_journal_checkpoints(
                       root_id, root_generation, volume_guid, volume_serial,
                       root_reference_version, root_file_reference, journal_id,
                       next_unread_usn, captured_exclusive_end, covered_catalog_revision,
                       protocol_version, contract_version, continuity_state,
                       updated_unix_ms
                     ) VALUES (
                       'lifecycle-root', 1, 'lifecycle-volume', '77',
                       3, X'01010101010101010101010101010101', '44',
                       '20', '20', 0, 5, 1, 'current', 3
                     );
                     UPDATE library_persistent_journal_root_state
                     SET continuity_state = 'current', updated_unix_ms = 3
                     WHERE root_id = 'lifecycle-root';
                     UPDATE library_persistent_journal_baselines
                     SET closing_next_usn = '20', phase = 'completed',
                         updated_unix_ms = 3, completed_unix_ms = 3
                     WHERE change_id = 901;
                     UPDATE library_change_queue
                     SET status = 'completed', catalog_revision_at_success = 0,
                         updated_unix_ms = 3
                     WHERE id = 901;
                     UPDATE library_recovery_authorities
                     SET retired_unix_ms = 3 WHERE change_id = 901;",
                )
                .expect("valid completed recovery lifecycle");
            migrate_schema(&mut connection).expect("validate completed recovery lifecycle");
        }
        drop(connection);
        catalog
    }

    fn v27_root_proof_phase_catalog(phase: &str) -> NamedTempFile {
        let catalog = recovery_lifecycle_catalog(phase == "completed");
        let connection = Connection::open(catalog.path()).expect("v27 root-proof phase catalog");
        connection
            .execute_batch("PRAGMA foreign_keys = ON")
            .expect("enable v27 fixture foreign keys");
        connection
            .execute_batch(
                "INSERT INTO scan_runs(
                   id, root_id, status, started_unix_ms, completed_unix_ms, preview_edge
                 ) VALUES ('lifecycle-scan', 'lifecycle-root', 'completed', 1, 1, 128);
                 UPDATE library_roots
                 SET active_scan_id = 'lifecycle-scan' WHERE id = 'lifecycle-root';",
            )
            .expect("last trustworthy catalog fixture");
        match phase {
            "opening" => {
                connection
                    .execute(
                        "DELETE FROM library_persistent_journal_baselines
                         WHERE change_id = 901",
                        [],
                    )
                    .expect("opening phase has not captured a baseline");
            }
            "inventory" => {
                connection
                    .execute_batch(
                        "INSERT INTO library_metadata_inventory_runs(
                           id, root_id, root_generation, epoch, scope_kind, scope_relative_path,
                           status, next_page_index, started_unix_ms, updated_unix_ms
                         ) VALUES (
                           'lifecycle-run', 'lifecycle-root', 1, 1, 'root', '',
                           'running', 1, 1, 1
                         );",
                    )
                    .expect("v27 inventory run");
            }
            "replay" | "absence" => {
                let (next_unread, absence_authority) = if phase == "absence" {
                    ("20", 1)
                } else {
                    ("15", 0)
                };
                connection
                    .execute(
                        "UPDATE library_persistent_journal_root_state
                         SET continuity_state = 'catching_up'
                         WHERE root_id = 'lifecycle-root'",
                        [],
                    )
                    .expect("v27 catch-up root state");
                connection
                    .execute(
                        "UPDATE library_persistent_journal_baselines
                         SET phase = ?1, closing_next_usn = '20', updated_unix_ms = 2
                         WHERE change_id = 901",
                        [phase],
                    )
                    .expect("v27 replay or absence baseline");
                connection
                    .execute(
                        "INSERT INTO library_persistent_journal_checkpoints(
                           root_id, root_generation, volume_guid, volume_serial,
                           root_reference_version, root_file_reference, journal_id,
                           next_unread_usn, captured_exclusive_end, covered_catalog_revision,
                           protocol_version, contract_version, continuity_state,
                           updated_unix_ms
                         ) VALUES (
                           'lifecycle-root', 1, 'lifecycle-volume', '77',
                           3, X'01010101010101010101010101010101', '44',
                           ?1, '20', 0, 5, 1, 'catching_up', 2
                         )",
                        [next_unread],
                    )
                    .expect("v27 catch-up checkpoint");
                connection
                    .execute(
                        "INSERT INTO library_metadata_inventory_runs(
                           id, root_id, root_generation, epoch, scope_kind, scope_relative_path,
                           status, next_page_index, comparison_cursor, staged_entry_count,
                           candidate_count, enumeration_complete, absence_authority,
                           started_unix_ms, updated_unix_ms
                         ) VALUES (
                           'lifecycle-run', 'lifecycle-root', 1, 1, 'root', '',
                           'comparing', 2, 'photo.jpg', 1, 1, 1, ?1, 1, 2
                         )",
                        [absence_authority],
                    )
                    .expect("v27 comparing run");
                connection
                    .execute_batch(
                        "INSERT INTO library_metadata_inventory_entries(
                           run_id, relative_path, entry_kind, file_size, modified_unix_ms,
                           placeholder_state, is_reparse_point, staged_page_index,
                           comparison_status, staged_unix_ms
                         ) VALUES (
                           'lifecycle-run', 'photo.jpg', 'file', 1, 1,
                           'available', 0, 1, 'enqueued', 1
                         );
                         INSERT INTO library_metadata_inventory_frontier(
                           run_id, ordinal, relative_directory, state,
                           directory_identity_scheme, directory_identity_value,
                           enumerated_entry_count, updated_unix_ms
                         ) VALUES (
                           'lifecycle-run', 0, '', 'completed',
                           'windows-file-id-128-v1', '77:root', 1, 2
                         );
                         INSERT INTO library_change_queue(
                           id, root_id, root_generation, intent_kind, scope, relative_path,
                           origin, first_observed_unix_ms, most_recent_observed_unix_ms,
                           first_sequence, most_recent_sequence, coalesced_observation_count,
                           status, ready_unix_ms, catalog_revision_at_enqueue,
                           created_unix_ms, updated_unix_ms
                         ) VALUES (
                           902, 'lifecycle-root', 1, 'reconcile', 'path', 'photo.jpg',
                           'consistency_audit', 2, 2, '2', '2', 1,
                           'pending', 2, 0, 2, 2
                         );
                         INSERT INTO library_metadata_inventory_candidate_owners(
                           run_id, candidate_key, change_id, candidate_role,
                           relative_path, owned_unix_ms
                         ) VALUES (
                           'lifecycle-run', 'photo.jpg', 902, 'present', 'photo.jpg', 2
                         );",
                    )
                    .expect("v27 derived candidate work");
            }
            "completed" => {}
            other => panic!("unsupported v27 phase fixture: {other}"),
        }
        replace_current_spool_contract_with_v27_for_test(&connection);
        if phase == "inventory" {
            connection
                .execute_batch(
                    "INSERT INTO library_metadata_inventory_spools(
                       run_id, authority_change_id, root_id, root_generation,
                       scope_kind, scope_relative_path, state,
                       created_unix_ms, updated_unix_ms
                     ) VALUES (
                       'lifecycle-run', 901, 'lifecycle-root', 1,
                       'root', '', 'enumerating', 1, 1
                     );
                     INSERT INTO library_metadata_inventory_spool_directories(
                       run_id, ordinal, relative_directory, state,
                       created_unix_ms, updated_unix_ms
                     ) VALUES ('lifecycle-run', 0, '', 'pending', 1, 1);",
                )
                .expect("v27 active source spool");
        }
        super::validate_recovery_authority_contract(&connection)
            .expect("valid v27 recovery authority");
        super::validate_persistent_journal_baseline_contract(&connection)
            .expect("valid v27 baseline phase");
        super::validate_recovery_execution_contract(&connection)
            .expect("valid v27 recovery execution");
        super::validate_metadata_inventory_spool_contract_version(&connection, 27, 1)
            .expect("valid v27 source spool");
        drop(connection);
        catalog
    }

    #[test]
    fn v28_to_v29_migrates_only_full_v3_root_identity_and_fails_v2_closed() {
        let mut connection = Connection::open_in_memory().expect("catalog");
        migrate_schema(&mut connection).expect("fresh current catalog");
        connection
            .execute_batch(
                "INSERT INTO library_roots(id, path, created_unix_ms) VALUES
                   ('root-v3', 'C:/v3', 1), ('root-v2', 'C:/v2', 1);
                 INSERT INTO library_change_root_state(root_id, generation, is_active, updated_unix_ms)
                   VALUES ('root-v3', 3, 1, 1), ('root-v2', 2, 1, 1);
                 INSERT INTO library_persistent_journal_root_state(
                   root_id, root_generation, protocol_version, contract_version,
                   capability_state, continuity_state, updated_unix_ms
                 ) VALUES
                   ('root-v3', 3, 5, 1, 'supported', 'current', 10),
                   ('root-v2', 2, 5, 1, 'supported', 'current', 10);
                 INSERT INTO library_persistent_journal_checkpoints(
                   root_id, root_generation, volume_guid, volume_serial,
                   root_reference_version, root_file_reference, journal_id,
                   next_unread_usn, captured_exclusive_end, covered_catalog_revision,
                   protocol_version, contract_version, continuity_state, updated_unix_ms
                 ) VALUES
                   ('root-v3', 3, 'volume-v3', '77', 3,
                    X'01010101010101010101010101010101', '10', '20', '20', 0, 5, 1,
                    'current', 10),
                   ('root-v2', 2, 'volume-v2', '88', 2,
                    X'0202020202020202', '11', '20', '20', 0, 5, 1,
                    'current', 10);",
            )
            .expect("v28 journal proof fixtures");
        downgrade_current_catalog_to_v28(&connection);

        super::migrate_v28_to_v29(&mut connection).expect("migrate v28 root proofs");

        let proofs = connection
            .prepare(
                "SELECT root_id, root_generation, identity_value, authority_kind
                 FROM library_root_publication_namespaces ORDER BY root_id",
            )
            .expect("proof query")
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            })
            .expect("proof rows")
            .collect::<rusqlite::Result<Vec<_>>>()
            .expect("proof evidence");
        assert_eq!(
            proofs,
            vec![(
                "root-v3".to_owned(),
                3,
                "000000000000004d:01010101010101010101010101010101".to_owned(),
                "journal_v3_migration".to_owned(),
            )]
        );
        let v2_state: (String, String, String) = connection
            .query_row(
                "SELECT checkpoint.continuity_state, root.continuity_state,
                        checkpoint.last_failure_code
                 FROM library_persistent_journal_checkpoints AS checkpoint
                 JOIN library_persistent_journal_root_state AS root
                   ON root.root_id = checkpoint.root_id
                  AND root.root_generation = checkpoint.root_generation
                 WHERE checkpoint.root_id = 'root-v2'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("v2 fail-closed projection");
        assert_eq!(
            v2_state,
            (
                "recovery_required".to_owned(),
                "recovery_required".to_owned(),
                "root_publication_namespace_unproven".to_owned(),
            )
        );
    }

    #[test]
    fn malformed_partial_v29_schema_rolls_back_without_advancing_v28() {
        let mut connection = Connection::open_in_memory().expect("catalog");
        migrate_schema(&mut connection).expect("fresh current catalog");
        downgrade_current_catalog_to_v28(&connection);
        connection
            .execute_batch(
                "CREATE TABLE library_root_publication_namespaces(root_id TEXT PRIMARY KEY);",
            )
            .expect("partial v29 object");

        super::migrate_v28_to_v29(&mut connection).expect_err("partial v29 must fail closed");

        let state: (i64, i64, i64, i64) = connection
            .query_row(
                "SELECT
                   (SELECT version FROM schema_info),
                   (SELECT user_version FROM pragma_user_version),
                   (SELECT COUNT(*) FROM sqlite_master
                    WHERE type = 'table' AND name = 'library_root_publication_namespace_contract'),
                   (SELECT COUNT(*) FROM pragma_table_info(
                     'library_root_publication_namespaces'))",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .expect("partial rollback evidence");
        assert_eq!(state, (28, 28, 0, 1));
    }

    #[test]
    fn v28_to_v29_rejects_conflicting_spool_and_v3_proofs_without_partial_publish() {
        let mut connection = Connection::open_in_memory().expect("catalog");
        migrate_schema(&mut connection).expect("fresh current catalog");
        connection
            .execute_batch(
                "INSERT INTO library_roots(id, path, created_unix_ms)
                   VALUES ('proof-conflict-root', 'C:/proof-conflict', 1);
                 INSERT INTO library_change_root_state(
                   root_id, generation, is_active, updated_unix_ms
                 ) VALUES ('proof-conflict-root', 1, 1, 1);
                 INSERT INTO library_persistent_journal_root_state(
                   root_id, root_generation, protocol_version, contract_version,
                   capability_state, continuity_state, updated_unix_ms
                 ) VALUES (
                   'proof-conflict-root', 1, 5, 1, 'supported', 'current', 10
                 );
                 INSERT INTO library_persistent_journal_checkpoints(
                   root_id, root_generation, volume_guid, volume_serial,
                   root_reference_version, root_file_reference, journal_id,
                   next_unread_usn, captured_exclusive_end, covered_catalog_revision,
                   protocol_version, contract_version, continuity_state, updated_unix_ms
                 ) VALUES (
                   'proof-conflict-root', 1, 'volume-conflict', '77', 3,
                   X'01010101010101010101010101010101', '44', '20', '20', 0,
                   5, 1, 'current', 10
                 );
                 INSERT INTO library_change_queue(
                   id, root_id, root_generation, intent_kind, scope, relative_path,
                   origin, first_observed_unix_ms, most_recent_observed_unix_ms,
                   first_sequence, most_recent_sequence, coalesced_observation_count,
                   status, ready_unix_ms, lease_expires_unix_ms,
                   catalog_revision_at_enqueue,
                   created_unix_ms, updated_unix_ms
                 ) VALUES (
                   991, 'proof-conflict-root', 1, 'freshness_unknown', 'root', '',
                   'consistency_audit', 1, 1, '1', '1', 1,
                   'leased', 1, 100, 0, 1, 1
                 );
                 INSERT INTO library_recovery_authorities(
                   change_id, run_id, root_id, root_generation, reason, authorized_unix_ms
                 ) VALUES (
                   991, 'proof-conflict-run', 'proof-conflict-root', 1,
                   'containment_failure', 1
                 );
                 INSERT INTO library_metadata_inventory_runs(
                   id, root_id, root_generation, epoch, scope_kind, scope_relative_path,
                   status, next_page_index, started_unix_ms, updated_unix_ms
                 ) VALUES (
                   'proof-conflict-run', 'proof-conflict-root', 1, 1, 'root', '',
                   'running', 1, 1, 11
                 );
                 INSERT INTO library_metadata_inventory_spools(
                   run_id, authority_change_id, root_id, root_generation,
                   root_identity_scheme, root_identity_value,
                   scope_kind, scope_relative_path, state,
                   created_unix_ms, updated_unix_ms
                 ) VALUES (
                   'proof-conflict-run', 991, 'proof-conflict-root', 1,
                   'windows-file-id-128-v1',
                   '000000000000004d:ffffffffffffffffffffffffffffffff',
                   'root', '', 'enumerating', 1, 11
                 );
                 INSERT INTO library_metadata_inventory_spool_directories(
                   run_id, ordinal, relative_directory, state,
                   created_unix_ms, updated_unix_ms
                 ) VALUES (
                   'proof-conflict-run', 0, '', 'pending', 1, 11
                 );",
            )
            .expect("conflicting v28 proof fixture");
        downgrade_current_catalog_to_v28(&connection);

        let error = super::migrate_v28_to_v29(&mut connection)
            .expect_err("conflicting durable identities must fail migration closed");
        assert_eq!(
            error.code,
            "catalog_root_publication_namespace_unverifiable"
        );
        let rollback: (i64, i64, i64, i64, i64) = connection
            .query_row(
                "SELECT
                   (SELECT version FROM schema_info),
                   (SELECT user_version FROM pragma_user_version),
                   (SELECT COUNT(*) FROM library_metadata_inventory_spools
                    WHERE run_id = 'proof-conflict-run'),
                   (SELECT COUNT(*) FROM library_persistent_journal_checkpoints
                    WHERE root_id = 'proof-conflict-root'),
                   (SELECT COUNT(*) FROM sqlite_master
                    WHERE type = 'table'
                      AND name = 'library_root_publication_namespace_contract')",
                [],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                    ))
                },
            )
            .expect("conflict rollback evidence");
        assert_eq!(rollback, (28, 28, 1, 1, 0));
    }

    #[test]
    fn v29_to_v30_keeps_historical_fallback_with_root_identity_explicitly_blocked() {
        let mut connection = Connection::open_in_memory().expect("catalog");
        migrate_schema(&mut connection).expect("fresh current catalog");
        connection
            .execute_batch(
                "INSERT INTO library_roots(id, path, created_unix_ms)
                   VALUES ('historical-fallback-root', 'C:/historical-fallback', 1);
                 INSERT INTO library_change_root_state(
                   root_id, generation, is_active, updated_unix_ms
                 ) VALUES ('historical-fallback-root', 7, 1, 1);
                 INSERT INTO library_persistent_journal_root_state(
                   root_id, root_generation, protocol_version, contract_version,
                   capability_state, continuity_state, updated_unix_ms
                 ) VALUES (
                   'historical-fallback-root', 7, 0, 1, 'unknown', 'baseline_required', 1
                 );
                 INSERT INTO library_root_publication_namespaces(
                   root_id, root_generation, identity_scheme, identity_value,
                   authority_kind, established_catalog_revision,
                   established_unix_ms, updated_unix_ms
                 ) VALUES (
                   'historical-fallback-root', 7, 'windows-file-id-128-v1',
                   '000000000000004d:01010101010101010101010101010101',
                   'foreground_scan', 0, 1, 1
                 );
                 INSERT INTO library_change_queue(
                   id, root_id, root_generation, intent_kind, scope, relative_path,
                   origin, first_observed_unix_ms, most_recent_observed_unix_ms,
                   first_sequence, most_recent_sequence, coalesced_observation_count,
                   status, ready_unix_ms, attempt_count,
                   catalog_revision_at_enqueue, created_unix_ms, updated_unix_ms
                 ) VALUES (
                   3001, 'historical-fallback-root', 7, 'freshness_unknown', 'root', '',
                   'startup_catch_up', 41, 41, '41', '41', 1,
                   'pending', 41, 0, 0, 41, 41
                 );",
            )
            .expect("historical pending v29 fallback fixture");
        downgrade_current_catalog_to_v29(&connection);

        super::migrate_v29_to_v30(&mut connection).expect("migrate ambiguous historical gap");
        type MigratedLiveGapEvidence = (
            i64,
            String,
            String,
            String,
            String,
            String,
            i64,
            Option<i64>,
            Option<i64>,
            String,
            i64,
            i64,
        );
        let migrated: MigratedLiveGapEvidence = connection
            .query_row(
                "SELECT
                   (SELECT version FROM schema_info),
                   gap.origin, lane.lane, gap.intent_kind, gap.scope, gap.status,
                   gap.attempt_count, gap.next_retry_unix_ms, gap.lease_expires_unix_ms,
                   gap.last_failure_code,
                   (SELECT COUNT(*) FROM library_live_gap_recovery_claims
                    WHERE gap_change_id = gap.id),
                   (SELECT COUNT(*) FROM library_root_publication_namespaces
                    WHERE root_id = gap.root_id AND root_generation = gap.root_generation)
                 FROM library_change_queue AS gap
                 JOIN library_change_queue_lanes AS lane ON lane.change_id = gap.id
                 WHERE gap.id = 3001",
                [],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                        row.get(6)?,
                        row.get(7)?,
                        row.get(8)?,
                        row.get(9)?,
                        row.get(10)?,
                        row.get(11)?,
                    ))
                },
            )
            .expect("migrated historical fallback evidence");
        assert_eq!(
            migrated,
            (
                30,
                "startup_catch_up".to_owned(),
                "p1_journal".to_owned(),
                "freshness_unknown".to_owned(),
                "root".to_owned(),
                "retry_wait".to_owned(),
                0,
                None,
                None,
                "live_gap_v30_explicit_recovery_required".to_owned(),
                1,
                1,
            )
        );
        let user_version = connection
            .query_row("SELECT user_version FROM pragma_user_version", [], |row| {
                row.get::<_, i64>(0)
            })
            .expect("migrated user version");
        assert_eq!(user_version, 30);

        migrate_schema(&mut connection).expect("idempotent current-schema reopen");
        let stable: (String, String, String, i64) = connection
            .query_row(
                "SELECT gap.origin, lane.lane, gap.last_failure_code,
                        (SELECT COUNT(*) FROM library_live_gap_recovery_contract)
                 FROM library_change_queue AS gap
                 JOIN library_change_queue_lanes AS lane ON lane.change_id = gap.id
                 WHERE gap.id = 3001",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .expect("stable live gap migration");
        assert_eq!(
            stable,
            (
                "startup_catch_up".to_owned(),
                "p1_journal".to_owned(),
                "live_gap_v30_explicit_recovery_required".to_owned(),
                1,
            )
        );
    }

    #[test]
    fn v29_to_v30_keeps_naked_gap_without_root_identity_explicitly_blocked() {
        let mut connection = Connection::open_in_memory().expect("catalog");
        migrate_schema(&mut connection).expect("fresh current catalog");
        connection
            .execute_batch(
                "INSERT INTO library_roots(id, path, created_unix_ms)
                   VALUES ('unproven-live-gap-root', 'C:/unproven-live-gap', 1);
                 INSERT INTO library_change_root_state(
                   root_id, generation, is_active, updated_unix_ms
                 ) VALUES ('unproven-live-gap-root', 3, 1, 1);
                 INSERT INTO library_persistent_journal_root_state(
                   root_id, root_generation, protocol_version, contract_version,
                   capability_state, continuity_state, updated_unix_ms
                 ) VALUES (
                   'unproven-live-gap-root', 3, 0, 1, 'unknown', 'baseline_required', 1
                 );
                 INSERT INTO library_change_queue(
                   id, root_id, root_generation, intent_kind, scope, relative_path,
                   origin, first_observed_unix_ms, most_recent_observed_unix_ms,
                   first_sequence, most_recent_sequence, coalesced_observation_count,
                   status, ready_unix_ms, catalog_revision_at_enqueue,
                   created_unix_ms, updated_unix_ms
                 ) VALUES (
                   3002, 'unproven-live-gap-root', 3, 'freshness_unknown', 'root', '',
                   'startup_catch_up', 51, 51, '51', '51', 1,
                   'pending', 51, 0, 51, 51
                 );",
            )
            .expect("unproven naked v29 gap fixture");
        downgrade_current_catalog_to_v29(&connection);

        super::migrate_v29_to_v30(&mut connection).expect("migrate unproven live gap");
        let migrated: (
            String,
            String,
            String,
            Option<i64>,
            String,
            String,
            String,
            i64,
            i64,
        ) = connection
            .query_row(
                "SELECT gap.origin, lane.lane, gap.status, gap.next_retry_unix_ms,
                        gap.last_failure_code, claim.consumer_kind, claim.root_id,
                        claim.root_generation,
                        (SELECT COUNT(*) FROM library_root_publication_namespaces
                         WHERE root_id = gap.root_id)
                 FROM library_change_queue AS gap
                 JOIN library_change_queue_lanes AS lane ON lane.change_id = gap.id
                 JOIN library_live_gap_recovery_claims AS claim
                   ON claim.gap_change_id = gap.id
                 WHERE gap.id = 3002",
                [],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                        row.get(6)?,
                        row.get(7)?,
                        row.get(8)?,
                    ))
                },
            )
            .expect("unproven fail-closed evidence");
        assert_eq!(
            migrated,
            (
                "startup_catch_up".to_owned(),
                "p1_journal".to_owned(),
                "retry_wait".to_owned(),
                None,
                "live_gap_v30_explicit_recovery_required".to_owned(),
                "explicit_recovery_required".to_owned(),
                "unproven-live-gap-root".to_owned(),
                3,
                0,
            )
        );
        migrate_schema(&mut connection).expect("validate fail-closed current catalog");
    }

    #[test]
    fn malformed_partial_v30_schema_rolls_back_without_advancing_v29() {
        let mut connection = Connection::open_in_memory().expect("catalog");
        migrate_schema(&mut connection).expect("fresh current catalog");
        downgrade_current_catalog_to_v29(&connection);
        connection
            .execute_batch(
                "CREATE TABLE library_live_gap_recovery_contract(
                   singleton INTEGER PRIMARY KEY
                 );",
            )
            .expect("partial v30 object");

        super::migrate_v29_to_v30(&mut connection).expect_err("partial v30 must fail closed");

        let rollback: (i64, i64, i64, i64) = connection
            .query_row(
                "SELECT
                   (SELECT version FROM schema_info),
                   (SELECT user_version FROM pragma_user_version),
                   (SELECT COUNT(*) FROM pragma_table_info(
                    'library_live_gap_recovery_contract')),
                   (SELECT COUNT(*) FROM sqlite_master
                    WHERE type = 'table' AND name = 'library_live_gap_recovery_claims')",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .expect("partial v30 rollback evidence");
        assert_eq!(rollback, (29, 29, 1, 0));
    }

    fn reopen_recovery_lifecycle_catalog(
        catalog: &NamedTempFile,
    ) -> Result<(), crate::domain::ScanError> {
        let mut connection = Connection::open(catalog.path()).expect("reopen lifecycle catalog");
        connection
            .execute_batch("PRAGMA foreign_keys = ON")
            .expect("enable lifecycle foreign keys");
        migrate_schema(&mut connection)
    }

    fn active_frontier_catalog() -> NamedTempFile {
        let catalog = recovery_lifecycle_catalog(false);
        let mut connection = Connection::open(catalog.path()).expect("active frontier catalog");
        connection
            .execute_batch(
                "PRAGMA foreign_keys = ON;
                 INSERT INTO scan_runs(
                   id, root_id, status, started_unix_ms, completed_unix_ms, preview_edge
                 ) VALUES ('lifecycle-scan', 'lifecycle-root', 'completed', 1, 1, 128);
                 UPDATE library_roots
                 SET active_scan_id = 'lifecycle-scan' WHERE id = 'lifecycle-root';
                 INSERT INTO library_metadata_inventory_runs(
                   id, root_id, root_generation, epoch, scope_kind, scope_relative_path,
                   status, next_page_index, enumeration_cursor, staged_entry_count,
                   started_unix_ms, updated_unix_ms
                 ) VALUES (
                   'lifecycle-run', 'lifecycle-root', 1, 1, 'root', '',
                   'running', 2, 'album', 1, 1, 2
                 );
                 INSERT INTO library_metadata_inventory_entries(
                   run_id, relative_path, entry_kind, file_size, modified_unix_ms,
                   placeholder_state, is_reparse_point, staged_page_index, staged_unix_ms
                 ) VALUES (
                   'lifecycle-run', 'album', 'directory', NULL, 1,
                   'available', 0, 1, 2
                 );
                 INSERT INTO library_metadata_inventory_frontier(
                   run_id, ordinal, relative_directory, state,
                   directory_identity_scheme, directory_identity_value,
                   resume_after_relative_path, enumerated_entry_count, updated_unix_ms
                 ) VALUES
                   ('lifecycle-run', 0, '', 'enumerating',
                    'windows-file-id-128-v1', 'volume:root', 'album', 1, 2),
                   ('lifecycle-run', 1, 'album', 'pending',
                    'windows-file-id-128-v1', 'volume:album', NULL, 0, 2);",
            )
            .expect("valid active frontier lifecycle");
        migrate_schema(&mut connection).expect("validate active frontier lifecycle");
        drop(connection);
        catalog
    }

    #[test]
    fn v20_migration_preserves_queue_lineage_and_admits_inventory_origin() {
        let mut connection = fresh_v19_catalog();
        connection
            .execute_batch(
                "INSERT INTO library_change_queue(
                   id, root_id, root_generation, intent_kind, scope, relative_path,
                   previous_relative_path, origin, first_observed_unix_ms,
                   most_recent_observed_unix_ms, first_sequence, most_recent_sequence,
                   coalesced_observation_count, status, ready_unix_ms,
                   catalog_revision_at_enqueue, catch_up_source, catch_up_watermark,
                   created_unix_ms, updated_unix_ms
                 ) VALUES (
                   41, 'root-a', 3, 'reconcile', 'path', 'photo.jpg', NULL,
                   'startup_catch_up', 10, 10, '1', '1', 1, 'pending', 10, 2,
                   'windows_usn_v1', 'volume|1|2', 10, 10
                 );
                 INSERT INTO library_change_queue_catch_up_lineage(
                   change_id, catch_up_source, catch_up_watermark, enrolled_unix_ms
                 ) VALUES (41, 'windows_usn_v1', 'volume|1|2', 10);",
            )
            .expect("v19 queue fixture");

        migrate_schema(&mut connection).expect("migrate v19 to current schema");
        let (version, origin, lineage_count): (i64, String, i64) = connection
            .query_row(
                "SELECT
                   (SELECT version FROM schema_info),
                   (SELECT origin FROM library_change_queue WHERE id = 41),
                   (SELECT COUNT(*) FROM library_change_queue_catch_up_lineage
                    WHERE change_id = 41)",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("migrated queue evidence");
        assert_eq!(version, SCHEMA_VERSION);
        assert_eq!(origin, "startup_catch_up");
        assert_eq!(lineage_count, 1);

        connection
            .execute(
                "INSERT INTO library_change_queue(
                   root_id, root_generation, intent_kind, scope, relative_path,
                   previous_relative_path, origin, first_observed_unix_ms,
                   most_recent_observed_unix_ms, first_sequence, most_recent_sequence,
                   coalesced_observation_count, status, ready_unix_ms,
                   catalog_revision_at_enqueue, created_unix_ms, updated_unix_ms
                 ) VALUES (
                   'root-a', 3, 'reconcile', 'path', 'new.jpg', NULL,
                   'metadata_inventory', 11, 11, '2', '2', 1, 'pending', 11, 2, 11, 11
                 )",
                [],
            )
            .expect("metadata inventory origin");
        let new_id: i64 = connection.last_insert_rowid();
        assert!(new_id > 41);
        migrate_schema(&mut connection).expect("validate current v20");
    }

    #[test]
    fn v22_migration_terminalizes_partial_inventory_and_seeds_baseline_authority() {
        let mut connection = Connection::open_in_memory().expect("catalog");
        migrate_schema(&mut connection).expect("fresh current catalog");
        connection
            .execute_batch(
                "INSERT INTO library_roots(id, path, active_scan_id, created_unix_ms)
                   VALUES ('root-a', 'C:/source', 'published-scan', 1);
                 INSERT INTO scan_runs(
                   id, root_id, status, started_unix_ms, completed_unix_ms, preview_edge
                 ) VALUES ('published-scan', 'root-a', 'completed', 1, 1, 128);
                 INSERT INTO library_change_root_state(
                   root_id, generation, is_active, updated_unix_ms
                 ) VALUES ('root-a', 7, 1, 1);
                 INSERT INTO library_metadata_inventory_runs(
                   id, root_id, root_generation, epoch, scope_kind, scope_relative_path,
                   status, next_page_index, enumeration_complete, absence_authority,
                   started_unix_ms, updated_unix_ms
                 ) VALUES (
                   'partial-inventory', 'root-a', 7, 1, 'root', '', 'comparing', 1,
                   1, 1, 1, 1
                 );
                 INSERT INTO library_change_queue(
                   id, root_id, root_generation, intent_kind, scope, relative_path,
                   origin, first_observed_unix_ms, most_recent_observed_unix_ms,
                   first_sequence, most_recent_sequence, coalesced_observation_count,
                   status, ready_unix_ms, catalog_revision_at_enqueue,
                   created_unix_ms, updated_unix_ms
                 ) VALUES (
                   71, 'root-a', 7, 'reconcile', 'path', 'retained.jpg',
                   'user_refresh', 1, 1, '1', '1', 1, 'pending', 1, 0, 1, 1
                 );",
            )
            .expect("v21 durable fixture");
        remove_persistent_journal_v22_contract_for_test(&connection);
        remove_change_lane_v25_contract_for_test(&connection);
        connection
            .execute("UPDATE schema_info SET version = 21", [])
            .expect("v21 fixture version");

        migrate_schema(&mut connection).expect("migrate v21 through v23");

        let migrated: (i64, String, i64, Option<i64>, String, String, i64, i64) = connection
            .query_row(
                "SELECT
                   (SELECT version FROM schema_info),
                   (SELECT status FROM library_metadata_inventory_runs
                    WHERE id = 'partial-inventory'),
                   (SELECT absence_authority FROM library_metadata_inventory_runs
                    WHERE id = 'partial-inventory'),
                   (SELECT completed_unix_ms FROM library_metadata_inventory_runs
                    WHERE id = 'partial-inventory'),
                   (SELECT capability_state FROM library_persistent_journal_root_state
                    WHERE root_id = 'root-a'),
                   (SELECT continuity_state FROM library_persistent_journal_root_state
                    WHERE root_id = 'root-a'),
                   (SELECT COUNT(*) FROM library_persistent_journal_checkpoints),
                   (SELECT COUNT(*) FROM library_change_queue WHERE id = 71)",
                [],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                        row.get(6)?,
                        row.get(7)?,
                    ))
                },
            )
            .expect("migrated persistent journal authority");
        assert_eq!(
            migrated,
            (
                SCHEMA_VERSION,
                "superseded".to_owned(),
                0,
                None,
                "unknown".to_owned(),
                "baseline_required".to_owned(),
                0,
                1,
            )
        );
        migrate_schema(&mut connection).expect("validate current schema");
    }

    #[test]
    fn malformed_partial_v22_rolls_back_without_retiring_inventory() {
        let mut connection = Connection::open_in_memory().expect("catalog");
        migrate_schema(&mut connection).expect("fresh current catalog");
        connection
            .execute_batch(
                "INSERT INTO library_roots(id, path, active_scan_id, created_unix_ms)
                   VALUES ('root-a', 'C:/source', 'published-scan', 1);
                 INSERT INTO scan_runs(
                   id, root_id, status, started_unix_ms, completed_unix_ms, preview_edge
                 ) VALUES ('published-scan', 'root-a', 'completed', 1, 1, 128);
                 INSERT INTO library_change_root_state(
                   root_id, generation, is_active, updated_unix_ms
                 ) VALUES ('root-a', 1, 1, 1);
                 INSERT INTO library_metadata_inventory_runs(
                   id, root_id, root_generation, epoch, scope_kind, scope_relative_path,
                   status, next_page_index, enumeration_complete, absence_authority,
                   started_unix_ms, updated_unix_ms
                 ) VALUES (
                   'partial-inventory', 'root-a', 1, 1, 'root', '', 'comparing', 1,
                   1, 1, 1, 1
                 );",
            )
            .expect("partial inventory fixture");
        remove_persistent_journal_v22_contract_for_test(&connection);
        remove_change_lane_v25_contract_for_test(&connection);
        connection
            .execute_batch(
                "UPDATE schema_info SET version = 21;
                 CREATE TABLE library_persistent_journal_contract (
                   singleton INTEGER PRIMARY KEY,
                   contract_version INTEGER NOT NULL,
                   complete INTEGER NOT NULL
                 );
                 INSERT INTO library_persistent_journal_contract(
                   singleton, contract_version, complete
                 ) VALUES (1, 1, 0);",
            )
            .expect("malformed partial v22 fixture");

        let error = migrate_schema(&mut connection).expect_err("partial v22 must fail closed");
        let retained: (i64, String, i64, bool) = connection
            .query_row(
                "SELECT
                   (SELECT version FROM schema_info),
                   (SELECT status FROM library_metadata_inventory_runs
                    WHERE id = 'partial-inventory'),
                   (SELECT absence_authority FROM library_metadata_inventory_runs
                    WHERE id = 'partial-inventory'),
                   EXISTS(
                     SELECT 1 FROM sqlite_master
                     WHERE type = 'index'
                       AND name = 'library_change_root_state_generation_identity'
                   )",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .expect("rolled back migration state");

        assert_eq!(error.code, "catalog_database_error");
        assert_eq!(retained, (21, "comparing".to_owned(), 1, false));
    }

    #[test]
    fn current_v22_malformed_source_range_shape_fails_closed() {
        let mut connection = Connection::open_in_memory().expect("catalog");
        migrate_schema(&mut connection).expect("fresh current catalog");
        connection
            .execute_batch(
                "DROP INDEX library_persistent_journal_source_ranges_volume;
                 CREATE INDEX library_persistent_journal_source_ranges_volume
                   ON library_persistent_journal_source_ranges(
                     volume_guid, requested_start_usn, id
                   );",
            )
            .expect("malformed v22 index fixture");

        let error = migrate_schema(&mut connection).expect_err("malformed v22 shape");

        assert_eq!(
            error.code,
            "catalog_persistent_journal_contract_unverifiable"
        );
    }

    #[test]
    fn current_v22_rejects_replaced_check_with_the_same_check_count() {
        let mut connection = Connection::open_in_memory().expect("catalog");
        migrate_schema(&mut connection).expect("fresh current catalog");
        let original = connection
            .query_row(
                "SELECT sql FROM sqlite_master
                 WHERE type = 'table' AND name = 'library_persistent_journal_root_state'",
                [],
                |row| row.get::<_, String>(0),
            )
            .expect("canonical root-state DDL");
        let modified = original.replacen("CHECK(updated_unix_ms >= 0)", "CHECK(1)", 1);
        assert_ne!(modified, original);
        assert_eq!(
            normalize_schema_sql(&modified).matches("check(").count(),
            normalize_schema_sql(&original).matches("check(").count()
        );
        connection
            .execute_batch("PRAGMA writable_schema = ON")
            .expect("enable controlled schema mutation");
        connection
            .execute(
                "UPDATE sqlite_master SET sql = ?1
                 WHERE type = 'table' AND name = 'library_persistent_journal_root_state'",
                [modified],
            )
            .expect("replace one CHECK");
        connection
            .execute_batch("PRAGMA writable_schema = OFF; PRAGMA schema_version = 999")
            .expect("publish controlled schema mutation");

        let error = migrate_schema(&mut connection).expect_err("weakened CHECK must fail closed");
        assert_eq!(
            error.code,
            "catalog_persistent_journal_contract_unverifiable"
        );
    }

    #[test]
    fn schema_sql_canonicalization_preserves_quoted_tokens_and_escapes() {
        let formatted = "CREATE  TABLE [Range Name] (kind TEXT CHECK(kind = 'a  b'),\n\
                         note TEXT CHECK(note = 'it''s'), value BLOB DEFAULT X'0A')";
        let compact = "create table [Range Name](kind text check(kind='a  b'),\
                       note text check(note='it''s'),value blob default x'0A')";
        assert_eq!(
            normalize_schema_sql(formatted),
            normalize_schema_sql(compact)
        );
        for changed in [
            compact.replace("[Range Name]", "[range name]"),
            compact.replace("'a  b'", "'a b'"),
            compact.replace("'it''s'", "'IT''S'"),
            compact.replace("x'0A'", "x'0a'"),
        ] {
            assert_ne!(
                normalize_schema_sql(compact),
                normalize_schema_sql(&changed)
            );
        }
        assert_ne!(
            normalize_schema_sql("CREATE TABLE `Range` (`Value` TEXT)"),
            normalize_schema_sql("CREATE TABLE `range` (`Value` TEXT)")
        );
        assert_ne!(
            normalize_schema_sql("CREATE TABLE \"Range\" (\"Value\" TEXT)"),
            normalize_schema_sql("CREATE TABLE \"range\" (\"Value\" TEXT)")
        );
    }

    #[test]
    fn current_v24_rejects_quoted_schema_literal_mutations() {
        let mutations = [
            (
                "trigger",
                "library_persistent_journal_source_range_id_insert",
                "'*[^0-9a-f]*'",
                "'*[^0-9A-F]*'",
            ),
            (
                "trigger",
                "library_persistent_journal_source_range_id_insert",
                "'text'",
                "'TEXT'",
            ),
            (
                "trigger",
                "library_persistent_journal_source_range_id_insert",
                "'invalid persistent journal source range id'",
                "'invalid  persistent journal source range id'",
            ),
            (
                "trigger",
                "library_persistent_journal_source_range_id_insert",
                "'invalid persistent journal source range id'",
                "'invalid persistent journal source range ''id'''",
            ),
            (
                "table",
                "library_persistent_journal_source_ranges",
                "'enrolled'",
                "'ENROLLED'",
            ),
        ];
        for (index, (object_type, name, original, replacement)) in mutations.into_iter().enumerate()
        {
            let mut connection = Connection::open_in_memory().expect("catalog");
            migrate_schema(&mut connection).expect("fresh current catalog");
            let sql = connection
                .query_row(
                    "SELECT sql FROM sqlite_master WHERE type = ?1 AND name = ?2",
                    [object_type, name],
                    |row| row.get::<_, String>(0),
                )
                .expect("canonical schema object");
            let modified = sql.replacen(original, replacement, 1);
            assert_ne!(modified, sql, "mutation {index} must change schema SQL");
            connection
                .execute_batch("PRAGMA writable_schema = ON")
                .expect("enable controlled schema mutation");
            connection
                .execute(
                    "UPDATE sqlite_master SET sql = ?1 WHERE type = ?2 AND name = ?3",
                    rusqlite::params![modified, object_type, name],
                )
                .expect("mutate quoted schema token");
            connection
                .execute_batch("PRAGMA writable_schema = OFF; PRAGMA schema_version = 1001")
                .expect("publish controlled schema mutation");

            let error = migrate_schema(&mut connection)
                .expect_err("quoted schema token mutation must fail closed");
            assert_eq!(
                error.code, "catalog_persistent_journal_contract_unverifiable",
                "unexpected mutation {index} error"
            );
        }
    }

    #[test]
    fn v23_migration_retires_superseded_inventory_absence_authority() {
        let catalog = legacy_superseded_inventory_authority_catalog(23);
        let mut connection = Connection::open(catalog.path()).expect("open v23 inventory catalog");

        migrate_schema(&mut connection).expect("migrate retained v23 inventory authority");
        assert_superseded_inventory_authority_retired(&connection);
        drop(connection);

        let mut reopened = Connection::open(catalog.path()).expect("reopen migrated v23 catalog");
        migrate_schema(&mut reopened).expect("validate reopened v23 migration");
        assert_superseded_inventory_authority_retired(&reopened);
    }

    #[test]
    fn v24_migration_retires_superseded_inventory_absence_authority_before_lane_validation() {
        let catalog = legacy_superseded_inventory_authority_catalog(24);
        let mut connection = Connection::open(catalog.path()).expect("open v24 inventory catalog");

        migrate_schema(&mut connection).expect("migrate retained v24 inventory authority");
        assert_superseded_inventory_authority_retired(&connection);
        drop(connection);

        let mut reopened = Connection::open(catalog.path()).expect("reopen migrated v24 catalog");
        migrate_schema(&mut reopened).expect("validate reopened v24 migration");
        assert_superseded_inventory_authority_retired(&reopened);
    }

    #[test]
    fn v23_migration_retires_unfinished_inventory_before_journal_validation() {
        let catalog = legacy_unfinished_inventory_catalog(23);
        let mut connection = Connection::open(catalog.path()).expect("open v23 inventory catalog");

        migrate_schema(&mut connection).expect("migrate unfinished v23 inventory");
        assert_unfinished_inventory_retired(&connection);
        assert_superseded_inventory_authority_retired(&connection);
        drop(connection);

        let mut reopened = Connection::open(catalog.path()).expect("reopen migrated v23 catalog");
        migrate_schema(&mut reopened).expect("validate reopened unfinished v23 migration");
        assert_unfinished_inventory_retired(&reopened);
        assert_superseded_inventory_authority_retired(&reopened);
    }

    #[test]
    fn v24_migration_retires_unfinished_inventory_before_lane_validation() {
        let catalog = legacy_unfinished_inventory_catalog(24);
        let mut connection = Connection::open(catalog.path()).expect("open v24 inventory catalog");

        migrate_schema(&mut connection).expect("migrate unfinished v24 inventory");
        assert_unfinished_inventory_retired(&connection);
        assert_superseded_inventory_authority_retired(&connection);
        drop(connection);

        let mut reopened = Connection::open(catalog.path()).expect("reopen migrated v24 catalog");
        migrate_schema(&mut reopened).expect("validate reopened unfinished v24 migration");
        assert_unfinished_inventory_retired(&reopened);
        assert_superseded_inventory_authority_retired(&reopened);
    }

    #[test]
    fn clean_current_schema_validates_once_with_full_authority_audit() {
        let mut connection = Connection::open_in_memory().expect("clean catalog");
        migrate_schema(&mut connection).expect("create current catalog");
        connection
            .execute_batch("PRAGMA query_only = ON")
            .expect("make clean current catalog read only");

        super::reset_current_schema_validation_count();
        super::reset_source_revision_row_audit_count();
        super::reset_current_schema_row_audit_count();
        migrate_schema(&mut connection).expect("validate clean current catalog");

        assert_eq!(super::current_schema_validation_count(), 1);
        assert_eq!(super::source_revision_row_audit_count(), 1);
        assert!(super::current_schema_row_audit_count() > 0);
    }

    #[test]
    fn current_v31_source_revision_structure_damage_still_fails_closed() {
        let mut connection = Connection::open_in_memory().expect("current catalog");
        migrate_schema(&mut connection).expect("create current catalog");
        connection
            .execute_batch("DROP TRIGGER asset_locations_source_generation_update_guard;")
            .expect("damage source revision structure");

        super::reset_source_revision_row_audit_count();
        let error = migrate_schema(&mut connection).expect_err("reject damaged current catalog");

        assert_eq!(error.code, "catalog_source_revision_contract_unverifiable");
        assert_eq!(super::source_revision_row_audit_count(), 0);
    }

    #[test]
    fn current_v31_change_catch_up_ddl_damage_still_fails_closed() {
        let mut connection = Connection::open_in_memory().expect("current catalog");
        migrate_schema(&mut connection).expect("create current catalog");
        connection
            .execute_batch("DROP INDEX scan_run_catch_up_lineage_evidence;")
            .expect("damage catch-up DDL");

        super::reset_current_schema_row_audit_count();
        let error = migrate_schema(&mut connection).expect_err("reject damaged current catalog");

        assert_eq!(error.code, "catalog_change_catch_up_contract_unverifiable");
        assert_eq!(super::current_schema_row_audit_count(), 0);
    }

    #[test]
    fn current_v31_source_revision_marker_damage_still_fails_closed() {
        let mut connection = Connection::open_in_memory().expect("current catalog");
        migrate_schema(&mut connection).expect("create current catalog");
        connection
            .execute_batch("DELETE FROM library_source_revision_metadata_contract;")
            .expect("damage source revision marker");

        super::reset_current_schema_row_audit_count();
        let error = migrate_schema(&mut connection).expect_err("reject damaged current catalog");

        assert_eq!(error.code, "catalog_source_revision_contract_unverifiable");
        assert_eq!(super::current_schema_row_audit_count(), 0);
    }

    #[test]
    fn current_schema_repairs_removed_root_explicit_claim_idempotently() {
        let catalog = NamedTempFile::new().expect("retired explicit catalog");
        let mut connection = Connection::open(catalog.path()).expect("open retired catalog");
        migrate_schema(&mut connection).expect("create current catalog");
        seed_current_explicit_live_gap_claim(&connection, "removed-explicit-root", 4_001);
        retire_live_gap_root(&connection, "removed-explicit-root", 4_001);
        drop(connection);

        drop(
            SqliteCatalog::open(catalog.path().to_path_buf())
                .expect("repair removed-root explicit claim"),
        );
        let evidence = Connection::open(catalog.path()).expect("inspect repaired explicit claim");
        assert_retired_live_gap_claim_repaired(&evidence, 4_001);
        let lifecycle: (i64, i64, i64) = evidence
            .query_row(
                "SELECT
                   (SELECT generation FROM library_change_root_state
                    WHERE root_id = 'removed-explicit-root'),
                   (SELECT is_active FROM library_change_root_state
                    WHERE root_id = 'removed-explicit-root'),
                   (SELECT COUNT(*) FROM library_roots
                    WHERE id = 'removed-explicit-root')",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("removed-root lifecycle evidence");
        assert_eq!(lifecycle, (1, 0, 0));
        drop(evidence);

        drop(
            SqliteCatalog::open(catalog.path().to_path_buf())
                .expect("idempotently reopen repaired catalog"),
        );
        let evidence = Connection::open(catalog.path()).expect("inspect idempotent repair");
        assert_retired_live_gap_claim_repaired(&evidence, 4_001);
    }

    #[test]
    fn current_schema_repairs_explicit_claim_from_an_older_generation() {
        let catalog = NamedTempFile::new().expect("advanced explicit catalog");
        let mut connection = Connection::open(catalog.path()).expect("open advanced catalog");
        migrate_schema(&mut connection).expect("create current catalog");
        seed_current_explicit_live_gap_claim(&connection, "advanced-explicit-root", 4_002);
        advance_live_gap_root_generation(&connection, "advanced-explicit-root", 4_002);
        drop(connection);

        drop(
            SqliteCatalog::open(catalog.path().to_path_buf())
                .expect("repair previous-generation claim"),
        );
        let evidence = Connection::open(catalog.path()).expect("inspect advanced repair");
        assert_retired_live_gap_claim_repaired(&evidence, 4_002);
        let lifecycle: (i64, i64, i64) = evidence
            .query_row(
                "SELECT
                   (SELECT generation FROM library_change_root_state
                    WHERE root_id = 'advanced-explicit-root'),
                   (SELECT is_active FROM library_change_root_state
                    WHERE root_id = 'advanced-explicit-root'),
                   (SELECT COUNT(*) FROM library_roots
                    WHERE id = 'advanced-explicit-root')",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("advanced-root lifecycle evidence");
        assert_eq!(lifecycle, (2, 1, 1));
    }

    #[test]
    fn current_schema_repairs_the_retained_two_removed_one_advanced_shape() {
        let catalog = NamedTempFile::new().expect("retained live-gap catalog");
        let mut connection = Connection::open(catalog.path()).expect("open retained catalog");
        migrate_schema(&mut connection).expect("create current catalog");
        for (root_id, gap_change_id) in
            [("removed-explicit-a", 4_011), ("removed-explicit-b", 4_012)]
        {
            seed_current_explicit_live_gap_claim(&connection, root_id, gap_change_id);
            retire_live_gap_root(&connection, root_id, gap_change_id);
        }
        seed_current_explicit_live_gap_claim(&connection, "advanced-explicit", 4_013);
        advance_live_gap_root_generation(&connection, "advanced-explicit", 4_013);
        drop(connection);

        drop(
            SqliteCatalog::open(catalog.path().to_path_buf())
                .expect("repair retained live-gap shape"),
        );
        let evidence = Connection::open(catalog.path()).expect("inspect retained live-gap repair");
        for gap_change_id in [4_011, 4_012, 4_013] {
            assert_retired_live_gap_claim_repaired(&evidence, gap_change_id);
        }
        let lifecycle: (i64, i64, i64, i64) = evidence
            .query_row(
                "SELECT
                   (SELECT COUNT(*) FROM library_change_root_state
                    WHERE is_active = 0 AND generation = 1
                      AND root_id IN ('removed-explicit-a', 'removed-explicit-b')),
                   (SELECT COUNT(*) FROM library_roots
                    WHERE id IN ('removed-explicit-a', 'removed-explicit-b')),
                   (SELECT generation FROM library_change_root_state
                    WHERE root_id = 'advanced-explicit'),
                   (SELECT COUNT(*) FROM library_live_gap_recovery_claims)",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .expect("retained lifecycle projection");
        assert_eq!(lifecycle, (2, 0, 2, 0));
    }

    #[test]
    fn current_schema_repairs_retired_pending_journal_claim() {
        let catalog = NamedTempFile::new().expect("retired pending-journal catalog");
        let mut connection = Connection::open(catalog.path()).expect("open pending catalog");
        migrate_schema(&mut connection).expect("create current catalog");
        seed_current_pending_journal_live_gap_claim(&connection, "removed-pending-root", 4_021);
        retire_live_gap_root(&connection, "removed-pending-root", 4_021);
        drop(connection);

        drop(
            SqliteCatalog::open(catalog.path().to_path_buf())
                .expect("repair retired pending-journal claim"),
        );
        let evidence = Connection::open(catalog.path()).expect("inspect pending-journal repair");
        assert_retired_live_gap_claim_repaired(&evidence, 4_021);
    }

    #[test]
    fn current_schema_repairs_inventory_and_live_gap_residue_atomically() {
        let catalog = current_terminal_inventory_authority_catalog();
        let connection = Connection::open(catalog.path()).expect("open mixed repair fixture");
        seed_current_explicit_live_gap_claim(&connection, "mixed-removed-root", 4_031);
        retire_live_gap_root(&connection, "mixed-removed-root", 4_031);
        drop(connection);

        drop(
            SqliteCatalog::open(catalog.path().to_path_buf()).expect("repair mixed terminal state"),
        );
        let evidence = Connection::open(catalog.path()).expect("inspect mixed repair evidence");
        assert_current_terminal_inventory_repaired(&evidence);
        assert_retired_live_gap_claim_repaired(&evidence, 4_031);
    }

    #[test]
    fn v30_to_v31_repairs_retired_live_gap_claim_before_validation() {
        let catalog = NamedTempFile::new().expect("v30 retired claim catalog");
        let mut connection = Connection::open(catalog.path()).expect("open v30 claim catalog");
        migrate_schema(&mut connection).expect("create current catalog");
        seed_current_explicit_live_gap_claim(&connection, "v30-removed-root", 4_041);
        retire_live_gap_root(&connection, "v30-removed-root", 4_041);
        super::downgrade_source_revision_contract_to_v30_for_test(&connection);
        drop(connection);

        drop(
            SqliteCatalog::open(catalog.path().to_path_buf()).expect("migrate repaired v30 claim"),
        );
        let evidence = Connection::open(catalog.path()).expect("inspect migrated claim repair");
        assert_eq!(
            evidence
                .query_row("SELECT version FROM schema_info", [], |row| row
                    .get::<_, i64>(0))
                .expect("migrated schema version"),
            SCHEMA_VERSION,
        );
        assert_retired_live_gap_claim_repaired(&evidence, 4_041);
    }

    #[test]
    fn current_schema_preserves_legal_explicit_and_pending_journal_claims() {
        let catalog = NamedTempFile::new().expect("legal live-gap catalog");
        let mut connection = Connection::open(catalog.path()).expect("open legal claim catalog");
        migrate_schema(&mut connection).expect("create current catalog");
        seed_current_explicit_live_gap_claim(&connection, "legal-explicit-root", 4_051);
        seed_current_pending_journal_live_gap_claim(&connection, "legal-pending-root", 4_052);
        drop(connection);

        drop(
            SqliteCatalog::open(catalog.path().to_path_buf())
                .expect("validate legal live-gap claims"),
        );
        let evidence = Connection::open(catalog.path()).expect("inspect legal live-gap claims");
        let retained: Vec<(String, String, String)> = evidence
            .prepare(
                "SELECT claim.root_id, claim.consumer_kind, gap.status
                 FROM library_live_gap_recovery_claims AS claim
                 JOIN library_change_queue AS gap ON gap.id = claim.gap_change_id
                 ORDER BY claim.root_id",
            )
            .expect("legal claim statement")
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
            .expect("legal claim rows")
            .collect::<Result<_, _>>()
            .expect("collect legal claims");
        assert_eq!(
            retained,
            vec![
                (
                    "legal-explicit-root".to_owned(),
                    "explicit_recovery_required".to_owned(),
                    "retry_wait".to_owned(),
                ),
                (
                    "legal-pending-root".to_owned(),
                    "pending_journal".to_owned(),
                    "retry_wait".to_owned(),
                ),
            ],
        );
    }

    #[test]
    fn current_schema_live_gap_repair_rolls_back_when_another_claim_is_invalid() {
        let catalog = NamedTempFile::new().expect("mixed valid-invalid claim catalog");
        let mut connection = Connection::open(catalog.path()).expect("open rollback catalog");
        migrate_schema(&mut connection).expect("create current catalog");
        seed_current_explicit_live_gap_claim(&connection, "repairable-removed-root", 4_061);
        retire_live_gap_root(&connection, "repairable-removed-root", 4_061);
        seed_current_explicit_live_gap_claim(&connection, "invalid-active-root", 4_062);
        connection
            .execute(
                "UPDATE library_change_queue SET status = 'superseded' WHERE id = 4062",
                [],
            )
            .expect("make active-generation claim invalid");
        drop(connection);

        let error = match SqliteCatalog::open(catalog.path().to_path_buf()) {
            Ok(_) => panic!("invalid peer claim must roll back safe repair"),
            Err(error) => error,
        };
        assert_eq!(
            error.code,
            "catalog_live_gap_recovery_contract_unverifiable"
        );
        let evidence =
            Connection::open(catalog.path()).expect("inspect rolled-back live-gap repair");
        let retained: (i64, i64) = evidence
            .query_row(
                "SELECT
                   (SELECT COUNT(*) FROM library_live_gap_recovery_claims
                    WHERE gap_change_id = 4061),
                   (SELECT COUNT(*) FROM library_live_gap_recovery_claims
                    WHERE gap_change_id = 4062)",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("rolled-back claim evidence");
        assert_eq!(retained, (1, 1));
    }

    #[test]
    fn current_schema_live_gap_repair_keeps_malformed_states_fail_closed() {
        for case in 0..6 {
            let catalog = NamedTempFile::new().expect("malformed live-gap catalog");
            let mut connection =
                Connection::open(catalog.path()).expect("open malformed live-gap catalog");
            migrate_schema(&mut connection).expect("create current catalog");
            let gap_change_id = 4_100 + case;
            let root_id = format!("malformed-live-gap-{case}");
            if case == 5 {
                seed_current_pending_journal_live_gap_claim(&connection, &root_id, gap_change_id);
            } else {
                seed_current_explicit_live_gap_claim(&connection, &root_id, gap_change_id);
            }
            match case {
                0 => {
                    connection
                        .execute(
                            "UPDATE library_change_queue
                             SET status = 'superseded' WHERE id = ?1",
                            [gap_change_id],
                        )
                        .expect("make active same-generation claim invalid");
                }
                1 => {
                    advance_live_gap_root_generation(&connection, &root_id, gap_change_id);
                    let successor_id = gap_change_id + 100;
                    connection
                        .execute(
                            "INSERT INTO library_change_queue(
                               id, root_id, root_generation, intent_kind, scope, relative_path,
                               origin, first_observed_unix_ms, most_recent_observed_unix_ms,
                               first_sequence, most_recent_sequence,
                               coalesced_observation_count, status, ready_unix_ms,
                               catalog_revision_at_enqueue, catalog_revision_at_success,
                               created_unix_ms, updated_unix_ms
                             ) VALUES (
                               ?1, ?2, 2, 'reconcile', 'path', 'successor.jpg',
                               'user_refresh', 4, 4, ?3, ?3, 1, 'completed', 4,
                               0, 0, 4, 4
                             )",
                            rusqlite::params![successor_id, root_id, successor_id.to_string()],
                        )
                        .expect("insert explicit successor evidence");
                    connection
                        .execute(
                            "UPDATE library_change_queue
                             SET superseded_by_change_id = ?1 WHERE id = ?2",
                            rusqlite::params![successor_id, gap_change_id],
                        )
                        .expect("attach forbidden explicit successor");
                }
                2 => {
                    retire_live_gap_root(&connection, &root_id, gap_change_id);
                    connection
                        .execute(
                            "UPDATE library_change_queue
                             SET last_failure_code = 'wrong_explicit_failure',
                                 last_failure_message = 'Wrong explicit failure'
                             WHERE id = ?1",
                            [gap_change_id],
                        )
                        .expect("corrupt explicit failure evidence");
                }
                3 => {
                    retire_live_gap_root(&connection, &root_id, gap_change_id);
                    connection
                        .execute("DELETE FROM library_live_gap_recovery_contract", [])
                        .expect("remove live-gap marker");
                }
                4 => {
                    retire_live_gap_root(&connection, &root_id, gap_change_id);
                    let original = connection
                        .query_row(
                            "SELECT sql FROM sqlite_master
                             WHERE type = 'table'
                               AND name = 'library_live_gap_recovery_claims'",
                            [],
                            |row| row.get::<_, String>(0),
                        )
                        .expect("canonical live-gap claim DDL");
                    let modified =
                        original.replacen("'foreground_scan'", "'foreground_scan_other'", 1);
                    assert_ne!(modified, original);
                    connection
                        .execute_batch("PRAGMA writable_schema = ON")
                        .expect("enable controlled live-gap DDL mutation");
                    connection
                        .execute(
                            "UPDATE sqlite_master SET sql = ?1
                             WHERE type = 'table'
                               AND name = 'library_live_gap_recovery_claims'",
                            [modified],
                        )
                        .expect("mutate live-gap claim DDL");
                    connection
                        .execute_batch("PRAGMA writable_schema = OFF; PRAGMA schema_version = 1410")
                        .expect("publish controlled live-gap DDL mutation");
                }
                5 => {
                    retire_live_gap_root(&connection, &root_id, gap_change_id);
                    connection
                        .execute(
                            "UPDATE library_change_queue
                             SET last_failure_code = 'wrong_pending_journal_failure',
                                 last_failure_message = 'Wrong pending-journal failure'
                             WHERE id = ?1",
                            [gap_change_id],
                        )
                        .expect("corrupt pending-journal failure evidence");
                }
                _ => unreachable!(),
            }
            drop(connection);

            let error = match SqliteCatalog::open(catalog.path().to_path_buf()) {
                Ok(_) => panic!("malformed live-gap case {case} must fail closed"),
                Err(error) => error,
            };
            assert_eq!(
                error.code, "catalog_live_gap_recovery_contract_unverifiable",
                "unexpected error for malformed live-gap case {case}",
            );
            let evidence =
                Connection::open(catalog.path()).expect("inspect malformed live-gap row");
            let retained = evidence
                .query_row(
                    "SELECT COUNT(*) FROM library_live_gap_recovery_claims
                     WHERE gap_change_id = ?1",
                    [gap_change_id],
                    |row| row.get::<_, i64>(0),
                )
                .expect("retained malformed claim");
            assert_eq!(retained, 1, "case {case} must remain unchanged");
        }
    }

    #[test]
    fn current_v31_repairs_precontract_metadata_for_every_hardlink_alias() {
        let mut connection = Connection::open_in_memory().expect("current catalog");
        migrate_schema(&mut connection).expect("create current catalog");
        connection
            .execute_batch(
                "UPDATE catalog_state SET next_source_generation = 2;
                 INSERT INTO library_roots(id, path, active_scan_id, created_unix_ms)
                   VALUES ('root-a', 'C:/source', 'published-scan', 1);
                 INSERT INTO scan_runs(
                   id, root_id, status, started_unix_ms, completed_unix_ms,
                   asset_count, preview_edge
                 ) VALUES ('published-scan', 'root-a', 'completed', 1, 2, 2, 256);
                 INSERT INTO library_change_root_state(
                   root_id, generation, is_active, updated_unix_ms
                 ) VALUES ('root-a', 1, 1, 2);
                 INSERT INTO library_persistent_journal_root_state(
                   root_id, root_generation, protocol_version, contract_version,
                   capability_state, continuity_state, updated_unix_ms
                 ) VALUES ('root-a', 1, 0, 1, 'unknown', 'baseline_required', 2);
                 INSERT INTO assets(id, created_unix_ms)
                   VALUES ('asset-a', 1), ('asset-b', 1);
                 INSERT INTO asset_locations(
                   scan_id, asset_id, location_id, root_id, absolute_path, relative_path,
                   preview_path, file_size, modified_unix_ms, width, height, preview_status,
                   metadata_engine_id, metadata_engine_version, capture_local_time,
                   capture_time_source, capture_raw_value,
                   file_identity_scheme, file_identity_value,
                   source_revision_token, source_generation
                 ) VALUES
                   ('published-scan', 'asset-a', 'location-a', 'root-a',
                    'C:/source/a.bmp', 'a.bmp', 'C:/cache/a.jpg', 1024, 55, 8, 6, 'ready',
                    'legacy-metadata', '7', '2020-01-02T03:04:05',
                    'exif_original', '2020:01:02 03:04:05',
                    'windows-file-id-128-v1',
                    '0000000000000001:00000000000000000000000000000001',
                    'windows-file-change-time-100ns-v1:0000000000000042', 1),
                   ('published-scan', 'asset-b', 'location-b', 'root-a',
                    'C:/source/b.bmp', 'b.bmp', 'C:/cache/b.jpg', 1024, 55, 8, 6, 'ready',
                    'legacy-metadata', '7', '2020-01-02T03:04:05',
                    'exif_original', '2020:01:02 03:04:05',
                    'windows-file-id-128-v1',
                    '0000000000000001:00000000000000000000000000000001',
                    'windows-file-change-time-100ns-v1:0000000000000042', 1);
                 DROP TABLE library_source_revision_metadata_contract;",
            )
            .expect("precontract v31 hardlink fixture");

        super::reset_source_revision_row_audit_count();
        super::reset_current_schema_row_audit_count();
        migrate_schema(&mut connection).expect("repair precontract v31 metadata");
        assert_eq!(
            super::source_revision_row_audit_count(),
            3,
            "initial validation plus the repair transaction's before-and-after proof",
        );
        assert!(super::current_schema_row_audit_count() > 0);

        let repaired = connection
            .query_row(
                "SELECT COUNT(*),
                        SUM(metadata_engine_id = 'ame-invalidated-media-metadata'
                            AND metadata_engine_version = '0'),
                        SUM(capture_local_time IS NULL
                            AND capture_time_source IS NULL
                            AND capture_raw_value IS NULL),
                        SUM(preview_status = 'pending' AND preview_path = ''),
                        COUNT(DISTINCT source_generation),
                        COUNT(DISTINCT source_revision_token)
                 FROM asset_locations",
                [],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, i64>(2)?,
                        row.get::<_, i64>(3)?,
                        row.get::<_, i64>(4)?,
                        row.get::<_, i64>(5)?,
                    ))
                },
            )
            .expect("repaired alias evidence");
        assert_eq!(repaired, (2, 2, 2, 2, 1, 1));
        assert!(
            super::source_revision_metadata_contract_is_complete(&connection)
                .expect("repaired source metadata contract")
        );

        connection
            .execute_batch("PRAGMA query_only = ON")
            .expect("make repaired catalog read only");
        migrate_schema(&mut connection).expect("idempotently validate repaired v31 metadata");
    }

    #[test]
    fn current_schema_repairs_orphaned_terminal_inventory_authority_before_validation() {
        let catalog = current_terminal_inventory_authority_catalog();
        let catalog_path = catalog.path().to_path_buf();

        let reopened = SqliteCatalog::open(catalog_path.clone())
            .expect("reopen current terminal inventory catalog");
        drop(reopened);
        let evidence = Connection::open(&catalog_path).expect("open repaired current evidence");
        assert_current_terminal_inventory_repaired(&evidence);
        drop(evidence);

        let reopened = SqliteCatalog::open(catalog_path.clone())
            .expect("idempotently reopen current terminal inventory catalog");
        drop(reopened);
        let evidence = Connection::open(catalog_path).expect("reopen repaired current evidence");
        assert_current_terminal_inventory_repaired(&evidence);
    }

    #[test]
    fn v30_to_v31_repairs_orphaned_terminal_inventory_authority_before_validation() {
        let catalog = current_terminal_inventory_authority_catalog();
        let catalog_path = catalog.path().to_path_buf();
        let connection = Connection::open(&catalog_path).expect("open v30 authority fixture");
        connection
            .execute(
                "DELETE FROM library_metadata_inventory_spools
                 WHERE run_id = 'terminal-spool-inventory'",
                [],
            )
            .expect("isolate terminal authority fixture");
        super::downgrade_source_revision_contract_to_v30_for_test(&connection);
        drop(connection);

        let reopened = SqliteCatalog::open(catalog_path.clone())
            .expect("migrate v30 terminal authority catalog");
        drop(reopened);
        let evidence = Connection::open(&catalog_path).expect("open migrated authority evidence");
        assert_current_terminal_inventory_repaired(&evidence);
        drop(evidence);

        let reopened = SqliteCatalog::open(catalog_path.clone())
            .expect("idempotently reopen migrated terminal authority catalog");
        drop(reopened);
        let evidence = Connection::open(catalog_path).expect("reopen migrated authority evidence");
        assert_current_terminal_inventory_repaired(&evidence);
    }

    #[test]
    fn v30_to_v31_retires_terminal_inventory_spool_before_validation() {
        let catalog = current_terminal_inventory_authority_catalog();
        let catalog_path = catalog.path().to_path_buf();
        let connection = Connection::open(&catalog_path).expect("open v30 spool fixture");
        connection
            .execute(
                "UPDATE library_metadata_inventory_runs
                 SET absence_authority = 0
                 WHERE id = 'terminal-inventory'",
                [],
            )
            .expect("isolate terminal spool fixture");
        super::downgrade_source_revision_contract_to_v30_for_test(&connection);
        drop(connection);

        let reopened =
            SqliteCatalog::open(catalog_path.clone()).expect("migrate v30 terminal spool catalog");
        drop(reopened);
        let evidence = Connection::open(&catalog_path).expect("open migrated spool evidence");
        assert_current_terminal_inventory_repaired(&evidence);
        drop(evidence);

        let reopened = SqliteCatalog::open(catalog_path.clone())
            .expect("idempotently reopen migrated terminal spool catalog");
        drop(reopened);
        let evidence = Connection::open(catalog_path).expect("reopen migrated spool evidence");
        assert_current_terminal_inventory_repaired(&evidence);
    }

    #[test]
    fn v30_to_v31_terminal_inventory_repair_rejects_malformed_ddl_without_writes() {
        let catalog = current_terminal_inventory_authority_catalog();
        let catalog_path = catalog.path().to_path_buf();
        let connection = Connection::open(&catalog_path).expect("open malformed v30 fixture");
        super::downgrade_source_revision_contract_to_v30_for_test(&connection);
        let original = connection
            .query_row(
                "SELECT sql FROM sqlite_master
                 WHERE type = 'table' AND name = 'library_metadata_inventory_runs'",
                [],
                |row| row.get::<_, String>(0),
            )
            .expect("canonical v30 inventory run DDL");
        let modified = original.replacen(
            "CHECK(enumeration_complete = 1 OR absence_authority = 0)",
            "CHECK(1)",
            1,
        );
        assert_ne!(modified, original);
        connection
            .execute_batch("PRAGMA writable_schema = ON")
            .expect("enable controlled v30 inventory DDL mutation");
        connection
            .execute(
                "UPDATE sqlite_master SET sql = ?1
                 WHERE type = 'table' AND name = 'library_metadata_inventory_runs'",
                [modified],
            )
            .expect("weaken v30 inventory run DDL");
        connection
            .execute_batch("PRAGMA writable_schema = OFF; PRAGMA schema_version = 1310")
            .expect("publish controlled v30 inventory DDL mutation");
        drop(connection);

        let error = match SqliteCatalog::open(catalog_path.clone()) {
            Ok(_) => panic!("malformed v30 inventory DDL must fail closed"),
            Err(error) => error,
        };
        assert_eq!(
            error.code,
            "catalog_persistent_journal_contract_unverifiable"
        );
        let evidence = Connection::open(catalog_path).expect("open rolled-back v30 evidence");
        let retained: (i64, i64, i64) = evidence
            .query_row(
                "SELECT
                   (SELECT version FROM schema_info),
                   (SELECT absence_authority FROM library_metadata_inventory_runs
                    WHERE id = 'terminal-inventory'),
                   (SELECT COUNT(*) FROM library_metadata_inventory_spools
                    WHERE run_id = 'terminal-spool-inventory')",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("load rolled-back v30 repair evidence");
        assert_eq!(retained, (30, 1, 1));
    }

    #[test]
    fn current_schema_repair_rolls_back_when_active_inventory_has_no_authority() {
        let catalog = current_terminal_inventory_authority_catalog();
        let catalog_path = catalog.path().to_path_buf();
        let connection = Connection::open(&catalog_path).expect("open invalid active fixture");
        connection
            .execute(
                "DELETE FROM library_recovery_authorities WHERE change_id = 301",
                [],
            )
            .expect("remove active inventory authority");
        drop(connection);

        let error = match SqliteCatalog::open(catalog_path.clone()) {
            Ok(_) => panic!("unowned active inventory must fail closed"),
            Err(error) => error,
        };
        assert_eq!(
            error.code,
            "catalog_persistent_journal_contract_unverifiable"
        );
        let evidence = Connection::open(catalog_path).expect("open rolled-back repair evidence");
        let retained: (i64, i64, i64) = evidence
            .query_row(
                "SELECT
                   (SELECT absence_authority FROM library_metadata_inventory_runs
                    WHERE id = 'terminal-inventory'),
                   (SELECT COUNT(*) FROM library_metadata_inventory_spools
                    WHERE run_id = 'terminal-spool-inventory'),
                   (SELECT COUNT(*) FROM library_recovery_authorities
                    WHERE run_id = 'active-inventory' AND retired_unix_ms IS NULL)",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("load rolled-back repair evidence");
        assert_eq!(retained, (1, 1, 0));
    }

    #[test]
    fn current_schema_repair_does_not_write_through_malformed_inventory_ddl() {
        let catalog = current_terminal_inventory_authority_catalog();
        let catalog_path = catalog.path().to_path_buf();
        let connection = Connection::open(&catalog_path).expect("open malformed inventory fixture");
        let original = connection
            .query_row(
                "SELECT sql FROM sqlite_master
                 WHERE type = 'table' AND name = 'library_metadata_inventory_runs'",
                [],
                |row| row.get::<_, String>(0),
            )
            .expect("canonical inventory run DDL");
        let modified = original.replacen(
            "CHECK(enumeration_complete = 1 OR absence_authority = 0)",
            "CHECK(1)",
            1,
        );
        assert_ne!(modified, original);
        connection
            .execute_batch("PRAGMA writable_schema = ON")
            .expect("enable controlled inventory DDL mutation");
        connection
            .execute(
                "UPDATE sqlite_master SET sql = ?1
                 WHERE type = 'table' AND name = 'library_metadata_inventory_runs'",
                [modified],
            )
            .expect("weaken inventory run DDL");
        connection
            .execute_batch("PRAGMA writable_schema = OFF; PRAGMA schema_version = 1300")
            .expect("publish controlled inventory DDL mutation");
        drop(connection);

        let error = match SqliteCatalog::open(catalog_path.clone()) {
            Ok(_) => panic!("malformed inventory DDL must not be normalized through"),
            Err(error) => error,
        };
        assert_eq!(
            error.code,
            "catalog_persistent_journal_contract_unverifiable"
        );
        let evidence = Connection::open(catalog_path).expect("open malformed repair evidence");
        let retained: (i64, i64) = evidence
            .query_row(
                "SELECT
                   (SELECT absence_authority FROM library_metadata_inventory_runs
                    WHERE id = 'terminal-inventory'),
                   (SELECT COUNT(*) FROM library_metadata_inventory_spools
                    WHERE run_id = 'terminal-spool-inventory')",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("load malformed repair evidence");
        assert_eq!(retained, (1, 1));
    }

    #[test]
    fn v27_terminal_spool_is_retired_before_spool_validation() {
        let catalog = v27_root_proof_phase_catalog("inventory");
        let mut connection = Connection::open(catalog.path()).expect("open v27 terminal spool");
        connection
            .execute(
                "UPDATE library_metadata_inventory_runs
                 SET status = 'superseded',
                     last_issue_code = 'metadata_inventory_newer_epoch',
                     last_issue_message = 'A newer metadata inventory superseded this run'
                 WHERE id = 'lifecycle-run'",
                [],
            )
            .expect("pollute v27 terminal spool");

        migrate_schema(&mut connection).expect("migrate v27 terminal spool catalog");
        let migrated: (i64, String, i64, i64) = connection
            .query_row(
                "SELECT
                   (SELECT version FROM schema_info),
                   (SELECT status FROM library_metadata_inventory_runs
                    WHERE id = 'lifecycle-run'),
                   (SELECT COUNT(*) FROM library_metadata_inventory_spools
                    WHERE run_id = 'lifecycle-run'),
                   (SELECT COUNT(*) FROM library_recovery_authorities
                    WHERE run_id = 'lifecycle-run' AND retired_unix_ms IS NULL)",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .expect("load migrated v27 terminal spool evidence");
        assert_eq!(migrated, (SCHEMA_VERSION, "running".to_owned(), 0, 1));
    }

    #[test]
    fn v24_forward_migration_and_pending_carry_ddl_are_exact() {
        let mut connection = Connection::open_in_memory().expect("catalog");
        migrate_schema(&mut connection).expect("fresh current catalog");
        remove_change_lane_v25_contract_for_test(&connection);
        connection
            .execute_batch(
                "DROP TRIGGER library_persistent_journal_source_range_id_insert;
                 DROP TRIGGER library_persistent_journal_source_range_id_update;
                 ALTER TABLE library_persistent_journal_source_ranges
                   DROP COLUMN canonical_payload;
                 ALTER TABLE library_persistent_journal_cross_root_lineage DROP COLUMN journal_id;
                 ALTER TABLE library_persistent_journal_cross_root_lineage DROP COLUMN old_usn;
                 ALTER TABLE library_persistent_journal_cross_root_lineage DROP COLUMN new_usn;
                 ALTER TABLE library_persistent_journal_cross_root_lineage
                   DROP COLUMN previous_carry_id;
                 UPDATE schema_info SET version = 23;",
            )
            .expect("v23 fixture");

        migrate_schema(&mut connection).expect("forward migrate v23 to v24");
        let migrated: (i64, i64, i64) = connection
            .query_row(
                "SELECT
                   (SELECT version FROM schema_info),
                   (SELECT COUNT(*) FROM pragma_table_info(
                     'library_persistent_journal_range_lifecycle'
                   )),
                   (SELECT COUNT(*) FROM pragma_table_info(
                     'library_persistent_journal_pending_renames'
                   ))",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("v23 schema shape");
        assert_eq!(migrated, (SCHEMA_VERSION, 4, 13));

        let original = connection
            .query_row(
                "SELECT sql FROM sqlite_master
                 WHERE type = 'table'
                   AND name = 'library_persistent_journal_pending_renames'",
                [],
                |row| row.get::<_, String>(0),
            )
            .expect("canonical pending-carry DDL");
        let modified = original.replacen("CHECK(is_directory IN (0, 1))", "CHECK(1)", 1);
        assert_ne!(modified, original);
        assert_eq!(
            normalize_schema_sql(&modified).matches("check(").count(),
            normalize_schema_sql(&original).matches("check(").count()
        );
        connection
            .execute_batch("PRAGMA writable_schema = ON")
            .expect("enable controlled schema mutation");
        connection
            .execute(
                "UPDATE sqlite_master SET sql = ?1
                 WHERE type = 'table'
                   AND name = 'library_persistent_journal_pending_renames'",
                [modified],
            )
            .expect("replace pending carry CHECK");
        connection
            .execute_batch("PRAGMA writable_schema = OFF; PRAGMA schema_version = 1000")
            .expect("publish controlled schema mutation");
        let error = migrate_schema(&mut connection).expect_err("weakened v24 CHECK must fail");
        assert_eq!(
            error.code,
            "catalog_persistent_journal_contract_unverifiable"
        );
    }

    #[test]
    fn v25_migration_classifies_existing_queue_rows_and_tracks_new_origins() {
        let mut connection = Connection::open_in_memory().expect("catalog");
        migrate_schema(&mut connection).expect("fresh current catalog");
        remove_change_lane_v25_contract_for_test(&connection);
        connection
            .execute_batch(
                "UPDATE schema_info SET version = 24;
                 INSERT INTO library_change_queue(
                   id, root_id, root_generation, intent_kind, scope, relative_path,
                   previous_relative_path, origin, first_observed_unix_ms,
                   most_recent_observed_unix_ms, first_sequence, most_recent_sequence,
                   coalesced_observation_count, status, ready_unix_ms,
                   catalog_revision_at_enqueue, created_unix_ms, updated_unix_ms
                 ) VALUES
                   (101, 'root-a', 1, 'reconcile', 'path', 'live.jpg', NULL,
                    'live_notification', 1, 1, '1', '1', 1, 'pending', 1, 0, 1, 1),
                   (102, 'root-a', 1, 'reconcile', 'path', 'journal.jpg', NULL,
                    'startup_catch_up', 1, 1, '2', '2', 1, 'pending', 1, 0, 1, 1),
                   (103, 'root-a', 1, 'reconcile', 'path', 'inventory.jpg', NULL,
                    'metadata_inventory', 1, 1, '3', '3', 1, 'pending', 1, 0, 1, 1),
                   (104, 'root-a', 1, 'reconcile', 'path', 'audit.jpg', NULL,
                    'consistency_audit', 1, 1, '4', '4', 1, 'pending', 1, 0, 1, 1),
                   (105, 'root-a', 1, 'reconcile', 'path', 'refresh.jpg', NULL,
                    'user_refresh', 1, 1, '5', '5', 1, 'pending', 1, 0, 1, 1);",
            )
            .expect("v24 queue fixture");

        migrate_schema(&mut connection).expect("migrate v24 to v25");
        let lanes = connection
            .prepare(
                "SELECT queue.origin, lanes.lane
                 FROM library_change_queue AS queue
                 JOIN library_change_queue_lanes AS lanes ON lanes.change_id = queue.id
                 WHERE queue.id BETWEEN 101 AND 105
                 ORDER BY queue.id",
            )
            .expect("lane query")
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .expect("lane rows")
            .collect::<rusqlite::Result<Vec<_>>>()
            .expect("lane evidence");
        assert_eq!(
            lanes,
            vec![
                ("live_notification".to_owned(), "p0_live".to_owned()),
                ("startup_catch_up".to_owned(), "p1_journal".to_owned()),
                ("metadata_inventory".to_owned(), "p2_recovery".to_owned()),
                ("consistency_audit".to_owned(), "p2_recovery".to_owned()),
                ("user_refresh".to_owned(), "p2_recovery".to_owned()),
            ]
        );
        let implicit_authority_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM library_recovery_authorities",
                [],
                |row| row.get(0),
            )
            .expect("no implicit recovery authority");
        assert_eq!(implicit_authority_count, 0);

        connection
            .execute(
                "UPDATE library_change_queue SET origin = 'live_notification' WHERE id = 102",
                [],
            )
            .expect("origin update");
        let updated_lane: String = connection
            .query_row(
                "SELECT lane FROM library_change_queue_lanes WHERE change_id = 102",
                [],
                |row| row.get(0),
            )
            .expect("updated lane");
        assert_eq!(updated_lane, "p0_live");

        let tampering = connection
            .execute(
                "UPDATE library_change_queue_lanes SET lane = 'p2_recovery' WHERE change_id = 102",
                [],
            )
            .expect_err("lane guard rejects origin mismatch");
        assert!(tampering.to_string().contains("lane does not match origin"));
        migrate_schema(&mut connection).expect("validate current v25");
    }

    #[test]
    fn v25_and_v26_zero_owner_p2_capacity_debt_migrates_exactly_and_reopens() {
        const LEGACY_UNOWNED_P2: i64 = 3_584;

        for starting_version in [25_i64, 26_i64] {
            let catalog_file = NamedTempFile::new().expect("catalog file");
            let catalog_path = catalog_file.path().to_path_buf();
            let mut connection = Connection::open(&catalog_path).expect("catalog");
            migrate_schema(&mut connection).expect("fresh current catalog");
            remove_metadata_inventory_spool_v27_contract_for_test(&connection);
            if starting_version == 25 {
                remove_recovery_execution_v26_contract_for_test(&connection);
            }
            connection
                .execute_batch(&format!(
                    "UPDATE schema_info SET version = {starting_version};
                     PRAGMA application_id = 0;
                     PRAGMA user_version = 0;"
                ))
                .expect("publish valid legacy schema version");
            let transaction = connection.transaction().expect("legacy debt transaction");
            {
                let mut insert = transaction
                    .prepare(
                        "INSERT INTO library_change_queue(
                           root_id, root_generation, intent_kind, scope, relative_path,
                           previous_relative_path, origin, first_observed_unix_ms,
                           most_recent_observed_unix_ms, first_sequence,
                           most_recent_sequence, coalesced_observation_count, status,
                           ready_unix_ms, catalog_revision_at_enqueue,
                           created_unix_ms, updated_unix_ms
                         ) VALUES (
                           'root-a', 1, 'reconcile', 'path', ?1, NULL,
                           'metadata_inventory', ?2, ?2, ?3, ?3, 1, 'pending', ?2, 0,
                           ?2, ?2
                         )",
                    )
                    .expect("legacy queue insert");
                for ordinal in 1..=LEGACY_UNOWNED_P2 {
                    insert
                        .execute(rusqlite::params![
                            format!("legacy-{ordinal:04}.jpg"),
                            1_000 + ordinal,
                            ordinal.to_string(),
                        ])
                        .expect("insert legacy P2 row");
                }
            }
            transaction.commit().expect("commit exact legacy debt");

            migrate_schema(&mut connection).expect("migrate legacy debt to current schema");
            let evidence: (i64, i64, i64, i64, i64) = connection
                .query_row(
                    "SELECT
                       (SELECT version FROM schema_info),
                       (SELECT COUNT(*) FROM library_change_queue AS queue
                        JOIN library_change_queue_lanes AS lane ON lane.change_id = queue.id
                        WHERE lane.lane = 'p2_recovery'
                          AND queue.status IN ('pending', 'leased', 'retry_wait')),
                       (SELECT COUNT(*) FROM library_metadata_inventory_candidate_owners),
                       (SELECT COUNT(*) FROM library_recovery_authorities),
                       (SELECT COUNT(*) FROM library_change_queue)",
                    [],
                    |row| {
                        Ok((
                            row.get(0)?,
                            row.get(1)?,
                            row.get(2)?,
                            row.get(3)?,
                            row.get(4)?,
                        ))
                    },
                )
                .expect("migrated debt evidence");
            assert_eq!(
                evidence,
                (SCHEMA_VERSION, LEGACY_UNOWNED_P2, 0, 0, LEGACY_UNOWNED_P2),
                "starting schema v{starting_version}"
            );
            migrate_schema(&mut connection).expect("idempotent current migration");
            drop(connection);

            let mut reopened = Connection::open(&catalog_path).expect("reopen migrated catalog");
            migrate_schema(&mut reopened).expect("validate reopened migrated catalog");
            let reopened_evidence: (i64, i64) = reopened
                .query_row(
                    "SELECT
                       (SELECT COUNT(*) FROM library_change_queue),
                       (SELECT COUNT(*) FROM library_metadata_inventory_candidate_owners)",
                    [],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .expect("reopened debt evidence");
            assert_eq!(reopened_evidence, (LEGACY_UNOWNED_P2, 0));
        }
    }

    #[test]
    fn current_v25_rejects_weakened_lane_constraint_and_missing_lane_rows() {
        let mut connection = Connection::open_in_memory().expect("catalog");
        migrate_schema(&mut connection).expect("fresh current catalog");
        let original = connection
            .query_row(
                "SELECT sql FROM sqlite_master
                 WHERE type = 'table' AND name = 'library_change_queue_lanes'",
                [],
                |row| row.get::<_, String>(0),
            )
            .expect("canonical lane DDL");
        let modified = original.replacen(
            "CHECK(lane IN ('p0_live', 'p1_journal', 'p2_recovery'))",
            "CHECK(1)",
            1,
        );
        assert_ne!(modified, original);
        connection
            .execute_batch("PRAGMA writable_schema = ON")
            .expect("enable controlled schema mutation");
        connection
            .execute(
                "UPDATE sqlite_master SET sql = ?1
                 WHERE type = 'table' AND name = 'library_change_queue_lanes'",
                [modified],
            )
            .expect("weaken lane CHECK");
        connection
            .execute_batch("PRAGMA writable_schema = OFF; PRAGMA schema_version = 1002")
            .expect("publish controlled schema mutation");

        let error = migrate_schema(&mut connection).expect_err("weakened v25 CHECK must fail");
        assert_eq!(error.code, "catalog_change_lane_contract_unverifiable");

        let mut connection = Connection::open_in_memory().expect("second catalog");
        migrate_schema(&mut connection).expect("fresh current catalog");
        connection
            .execute_batch(
                "INSERT INTO library_change_queue(
                   id, root_id, root_generation, intent_kind, scope, relative_path,
                   origin, first_observed_unix_ms, most_recent_observed_unix_ms,
                   first_sequence, most_recent_sequence, coalesced_observation_count,
                   status, ready_unix_ms, catalog_revision_at_enqueue,
                   created_unix_ms, updated_unix_ms
                 ) VALUES (
                   201, 'root-a', 1, 'reconcile', 'path', 'photo.jpg',
                   'live_notification', 1, 1, '1', '1', 1, 'pending', 1, 0, 1, 1
                 );
                 DELETE FROM library_change_queue_lanes WHERE change_id = 201;",
            )
            .expect("missing lane fixture");
        let error = migrate_schema(&mut connection).expect_err("missing lane row must fail");
        assert_eq!(error.code, "catalog_change_lane_contract_unverifiable");
    }

    #[test]
    fn current_v25_recovery_authority_is_allowlisted_and_bound_to_p2_work() {
        let mut connection = Connection::open_in_memory().expect("catalog");
        migrate_schema(&mut connection).expect("fresh current catalog");
        connection
            .execute_batch(
                "INSERT INTO library_change_queue(
                   id, root_id, root_generation, intent_kind, scope, relative_path,
                   origin, first_observed_unix_ms, most_recent_observed_unix_ms,
                   first_sequence, most_recent_sequence, coalesced_observation_count,
                   status, ready_unix_ms, catalog_revision_at_enqueue,
                   created_unix_ms, updated_unix_ms
                 ) VALUES
                   (301, 'root-a', 1, 'freshness_unknown', 'root', '',
                    'consistency_audit', 1, 1, '1', '1', 1, 'pending', 1, 0, 1, 1),
                   (302, 'root-a', 1, 'reconcile', 'path', 'live.jpg',
                    'live_notification', 1, 1, '2', '2', 1, 'pending', 1, 0, 1, 1);",
            )
            .expect("queue fixtures");

        let invalid_reason = connection
            .execute(
                "INSERT INTO library_recovery_authorities(
                   change_id, run_id, root_id, root_generation, reason, authorized_unix_ms
                 ) VALUES (301, 'run-invalid', 'root-a', 1, 'slow_queue', 1)",
                [],
            )
            .expect_err("non-allowlisted reason");
        assert!(
            invalid_reason
                .to_string()
                .contains("CHECK constraint failed")
        );
        let missing_boundary = connection
            .execute(
                "INSERT INTO library_recovery_authorities(
                   change_id, run_id, root_id, root_generation, reason, authorized_unix_ms
                 ) VALUES (301, 'run-baseline', 'root-a', 1,
                           'existing_root_baseline', 1)",
                [],
            )
            .expect_err("baseline requires opening boundary");
        assert!(
            missing_boundary
                .to_string()
                .contains("CHECK constraint failed")
        );
        let wrong_lane = connection
            .execute(
                "INSERT INTO library_recovery_authorities(
                   change_id, run_id, root_id, root_generation, reason, authorized_unix_ms
                 ) VALUES (302, 'run-live', 'root-a', 1, 'containment_failure', 1)",
                [],
            )
            .expect_err("P0 cannot own P2 authority");
        assert!(wrong_lane.to_string().contains("does not match P2"));
        connection
            .execute(
                "INSERT INTO library_recovery_authorities(
                   change_id, run_id, root_id, root_generation, reason, authorized_unix_ms
                 ) VALUES (301, 'run-valid', 'root-a', 1, 'containment_failure', 1)",
                [],
            )
            .expect("allowlisted P2 authority");
        let immutable = connection
            .execute(
                "UPDATE library_recovery_authorities
                 SET reason = 'journal_gap' WHERE change_id = 301",
                [],
            )
            .expect_err("authority identity is immutable");
        assert!(immutable.to_string().contains("identity is immutable"));
        migrate_schema(&mut connection).expect("validate recovery authority contract");
    }

    #[test]
    fn current_v25_rejects_weakened_recovery_authority_shape() {
        let mut connection = Connection::open_in_memory().expect("catalog");
        migrate_schema(&mut connection).expect("fresh current catalog");
        let original = connection
            .query_row(
                "SELECT sql FROM sqlite_master
                 WHERE type = 'table' AND name = 'library_recovery_authorities'",
                [],
                |row| row.get::<_, String>(0),
            )
            .expect("canonical recovery authority DDL");
        let modified = original.replacen(
            "CHECK(retired_unix_ms IS NULL OR retired_unix_ms >= authorized_unix_ms)",
            "CHECK(1)",
            1,
        );
        assert_ne!(modified, original);
        connection
            .execute_batch("PRAGMA writable_schema = ON")
            .expect("enable controlled schema mutation");
        connection
            .execute(
                "UPDATE sqlite_master SET sql = ?1
                 WHERE type = 'table' AND name = 'library_recovery_authorities'",
                [modified],
            )
            .expect("weaken recovery authority CHECK");
        connection
            .execute_batch("PRAGMA writable_schema = OFF; PRAGMA schema_version = 1003")
            .expect("publish controlled schema mutation");

        let error = migrate_schema(&mut connection).expect_err("weakened authority must fail");
        assert_eq!(
            error.code,
            "catalog_recovery_authority_contract_unverifiable"
        );
    }

    #[test]
    fn current_v25_rejects_weakened_baseline_lifecycle_shape() {
        let mut connection = Connection::open_in_memory().expect("catalog");
        migrate_schema(&mut connection).expect("fresh current catalog");
        let original = connection
            .query_row(
                "SELECT sql FROM sqlite_master
                 WHERE type = 'table'
                   AND name = 'library_persistent_journal_baselines'",
                [],
                |row| row.get::<_, String>(0),
            )
            .expect("canonical baseline DDL");
        let modified = original.replacen(
            "CHECK((phase = 'completed') = (completed_unix_ms IS NOT NULL))",
            "CHECK(1)",
            1,
        );
        assert_ne!(modified, original);
        connection
            .execute_batch("PRAGMA writable_schema = ON")
            .expect("enable controlled schema mutation");
        connection
            .execute(
                "UPDATE sqlite_master SET sql = ?1
                 WHERE type = 'table'
                   AND name = 'library_persistent_journal_baselines'",
                [modified],
            )
            .expect("weaken baseline lifecycle CHECK");
        connection
            .execute_batch("PRAGMA writable_schema = OFF; PRAGMA schema_version = 1004")
            .expect("publish controlled schema mutation");

        let error = migrate_schema(&mut connection).expect_err("weakened baseline must fail");
        assert_eq!(
            error.code,
            "catalog_persistent_journal_baseline_contract_unverifiable"
        );
    }

    #[test]
    fn v27_root_proof_migration_fails_closed_across_every_recovery_phase() {
        for phase in ["opening", "inventory", "replay", "absence", "completed"] {
            let catalog = v27_root_proof_phase_catalog(phase);
            let mut connection = Connection::open(catalog.path()).expect("open v27 phase catalog");
            connection
                .execute_batch("PRAGMA foreign_keys = ON")
                .expect("enable migrated fixture foreign keys");

            migrate_schema(&mut connection)
                .unwrap_or_else(|error| panic!("migrate exact v27 phase {phase}: {error:?}"));

            let schema: (i64, i64, i64, i64) = connection
                .query_row(
                    "SELECT
                       (SELECT version FROM schema_info),
                       (SELECT contract_version
                        FROM library_metadata_inventory_spool_contract WHERE singleton = 1),
                       (SELECT COUNT(*) FROM pragma_table_info(
                          'library_metadata_inventory_spools'
                        ) WHERE name IN ('root_identity_scheme', 'root_identity_value')),
                       (SELECT COUNT(*) FROM library_roots
                        WHERE id = 'lifecycle-root' AND active_scan_id = 'lifecycle-scan')",
                    [],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
                )
                .expect("current exact shape evidence");
            assert_eq!(schema, (SCHEMA_VERSION, 3, 2, 1), "phase {phase}");

            let projection: (String, i64, i64) = connection
                .query_row(
                    "SELECT
                       continuity_state,
                       (SELECT COUNT(*) FROM library_persistent_journal_root_state
                        WHERE root_id = 'lifecycle-root' AND continuity_state = 'current'),
                       (SELECT COUNT(*) FROM library_persistent_journal_checkpoints
                        WHERE root_id = 'lifecycle-root' AND continuity_state = 'current')
                     FROM library_persistent_journal_root_state
                     WHERE root_id = 'lifecycle-root'",
                    [],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                )
                .expect("v28 authority projection");
            assert_eq!(
                projection.1, 0,
                "phase {phase} must not retain root Current"
            );
            assert_eq!(
                projection.2, 0,
                "phase {phase} must not retain checkpoint Current"
            );

            let control: (String, Option<String>, bool) = connection
                .query_row(
                    "SELECT queue.status, queue.last_failure_code,
                            authority.retired_unix_ms IS NULL
                     FROM library_change_queue AS queue
                     JOIN library_recovery_authorities AS authority
                       ON authority.change_id = queue.id
                     WHERE queue.id = 901",
                    [],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                )
                .expect("v28 recovery control state");
            assert_eq!(
                control.1.as_deref(),
                Some("metadata_inventory_v28_recapture_required"),
                "phase {phase}"
            );

            match phase {
                "opening" => {
                    assert_eq!(projection.0, "baseline_required");
                    assert_eq!(control.0, "pending");
                    assert!(control.2);
                    let derived: (i64, i64) = connection
                        .query_row(
                            "SELECT
                               (SELECT COUNT(*) FROM library_persistent_journal_baselines
                                WHERE change_id = 901),
                               (SELECT COUNT(*) FROM library_metadata_inventory_runs
                                WHERE id = 'lifecycle-run')",
                            [],
                            |row| Ok((row.get(0)?, row.get(1)?)),
                        )
                        .expect("opening phase state");
                    assert_eq!(derived, (0, 0));
                }
                "inventory" | "replay" | "absence" => {
                    assert_eq!(projection.0, "baseline_required");
                    assert_eq!(control.0, "pending");
                    assert!(control.2);
                    let reset: (String, i64, i64, i64, i64, String, i64, i64, i64) = connection
                        .query_row(
                            "SELECT
                               run.status, run.next_page_index, run.staged_entry_count,
                               run.candidate_count, run.enumeration_complete, baseline.phase,
                               (SELECT COUNT(*) FROM library_metadata_inventory_entries
                                WHERE run_id = 'lifecycle-run'),
                               (SELECT COUNT(*) FROM library_metadata_inventory_frontier
                                WHERE run_id = 'lifecycle-run'),
                               (SELECT COUNT(*) FROM library_metadata_inventory_spools
                                WHERE run_id = 'lifecycle-run')
                             FROM library_metadata_inventory_runs AS run
                             JOIN library_persistent_journal_baselines AS baseline
                               ON baseline.change_id = 901
                             WHERE run.id = 'lifecycle-run'",
                            [],
                            |row| {
                                Ok((
                                    row.get(0)?,
                                    row.get(1)?,
                                    row.get(2)?,
                                    row.get(3)?,
                                    row.get(4)?,
                                    row.get(5)?,
                                    row.get(6)?,
                                    row.get(7)?,
                                    row.get(8)?,
                                ))
                            },
                        )
                        .expect("v28 active recovery reset");
                    assert_eq!(
                        reset,
                        (
                            "running".to_owned(),
                            1,
                            0,
                            0,
                            0,
                            "inventory".to_owned(),
                            0,
                            0,
                            0
                        ),
                        "phase {phase}"
                    );
                    if matches!(phase, "replay" | "absence") {
                        let candidate: (String, Option<String>, i64) = connection
                            .query_row(
                                "SELECT status, last_failure_code,
                                        (SELECT COUNT(*)
                                         FROM library_metadata_inventory_candidate_owners
                                         WHERE change_id = 902)
                                 FROM library_change_queue WHERE id = 902",
                                [],
                                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                            )
                            .expect("v27 derived candidate retirement");
                        assert_eq!(candidate.0, "superseded", "phase {phase}");
                        assert_eq!(
                            candidate.1.as_deref(),
                            Some("metadata_inventory_v28_recapture_required"),
                            "phase {phase}"
                        );
                        assert_eq!(candidate.2, 0, "phase {phase}");
                    }
                }
                "completed" => {
                    assert_eq!(projection.0, "recovery_required");
                    assert_eq!(control.0, "completed");
                    assert!(!control.2);
                    let terminal: (String, String, String, Option<String>) = connection
                        .query_row(
                            "SELECT baseline.phase, root.continuity_state,
                                    checkpoint.continuity_state,
                                    checkpoint.last_failure_code
                             FROM library_persistent_journal_baselines AS baseline
                             JOIN library_persistent_journal_root_state AS root
                               ON root.root_id = baseline.root_id
                              AND root.root_generation = baseline.root_generation
                             JOIN library_persistent_journal_checkpoints AS checkpoint
                               ON checkpoint.root_id = baseline.root_id
                              AND checkpoint.root_generation = baseline.root_generation
                             WHERE baseline.change_id = 901",
                            [],
                            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
                        )
                        .expect("v28 invalidated terminal history");
                    assert_eq!(terminal.0, "completed");
                    assert_eq!(terminal.1, "recovery_required");
                    assert_eq!(terminal.2, "recovery_required");
                    assert_eq!(
                        terminal.3.as_deref(),
                        Some("metadata_inventory_v28_recapture_required")
                    );
                }
                _ => unreachable!(),
            }

            migrate_schema(&mut connection).expect("v28 migration is idempotent");
            drop(connection);
            reopen_recovery_lifecycle_catalog(&catalog).expect("reopen migrated v28 phase");
        }
    }

    #[test]
    fn malformed_v27_spool_shape_rolls_back_root_proof_migration() {
        let catalog = v27_root_proof_phase_catalog("inventory");
        let mut connection = Connection::open(catalog.path()).expect("open malformed v27 catalog");
        connection
            .execute_batch(
                "PRAGMA foreign_keys = ON;
                 DROP INDEX library_metadata_inventory_spool_entries_order;",
            )
            .expect("corrupt v27 spool order index");

        let error = migrate_schema(&mut connection).expect_err("malformed v27 must fail closed");
        let retained: (i64, String, Option<String>, String, i64) = connection
            .query_row(
                "SELECT
                   (SELECT version FROM schema_info),
                   (SELECT status FROM library_change_queue WHERE id = 901),
                   (SELECT last_failure_code FROM library_change_queue WHERE id = 901),
                   (SELECT status FROM library_metadata_inventory_runs
                    WHERE id = 'lifecycle-run'),
                   (SELECT COUNT(*) FROM library_metadata_inventory_spools
                    WHERE run_id = 'lifecycle-run')",
                [],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                    ))
                },
            )
            .expect("rolled-back v27 state");

        assert_eq!(
            error.code,
            "catalog_metadata_inventory_spool_contract_unverifiable"
        );
        assert_eq!(
            retained,
            (27, "pending".to_owned(), None, "running".to_owned(), 1)
        );
    }

    #[test]
    fn current_v27_reopens_legal_active_and_completed_recovery_lifecycles() {
        let active = recovery_lifecycle_catalog(false);
        reopen_recovery_lifecycle_catalog(&active).expect("reopen legal active lifecycle");
        let completed = recovery_lifecycle_catalog(true);
        reopen_recovery_lifecycle_catalog(&completed).expect("reopen legal completed lifecycle");
    }

    #[test]
    fn current_v27_reopens_a_legal_restart_safe_inventory_frontier() {
        let catalog = active_frontier_catalog();

        reopen_recovery_lifecycle_catalog(&catalog).expect("reopen legal active frontier");
    }

    #[test]
    fn current_v27_rejects_an_active_run_that_lost_its_frontier_on_reopen() {
        let catalog = active_frontier_catalog();
        let connection = Connection::open(catalog.path()).expect("mutate active frontier");
        connection
            .execute("DELETE FROM library_metadata_inventory_frontier", [])
            .expect("remove durable frontier");
        drop(connection);

        let error = reopen_recovery_lifecycle_catalog(&catalog)
            .expect_err("advanced active run requires a frontier");
        assert_eq!(
            error.code,
            "catalog_recovery_execution_contract_unverifiable"
        );
    }

    #[test]
    fn current_v27_rejects_a_frontier_ordinal_gap_on_reopen() {
        let catalog = active_frontier_catalog();
        let connection = Connection::open(catalog.path()).expect("mutate frontier ordinal");
        connection
            .execute(
                "UPDATE library_metadata_inventory_frontier
                 SET ordinal = 3 WHERE relative_directory = 'album'",
                [],
            )
            .expect("create frontier ordinal gap");
        drop(connection);

        let error = reopen_recovery_lifecycle_catalog(&catalog)
            .expect_err("frontier ordinals must remain contiguous");
        assert_eq!(
            error.code,
            "catalog_recovery_execution_contract_unverifiable"
        );
    }

    #[test]
    fn current_v27_rejects_a_pending_frontier_without_its_staged_directory() {
        let catalog = active_frontier_catalog();
        let connection = Connection::open(catalog.path()).expect("mutate pending frontier");
        connection
            .execute_batch(
                "DELETE FROM library_metadata_inventory_entries
                 WHERE relative_path = 'album';
                 INSERT INTO library_metadata_inventory_entries(
                   run_id, relative_path, entry_kind, file_size, modified_unix_ms,
                   placeholder_state, is_reparse_point, staged_page_index, staged_unix_ms
                 ) VALUES (
                   'lifecycle-run', 'replacement.txt', 'file', 1, 1,
                   'available', 0, 1, 2
                 );",
            )
            .expect("remove pending directory evidence");
        drop(connection);

        let error = reopen_recovery_lifecycle_catalog(&catalog)
            .expect_err("pending frontier requires staged directory evidence");
        assert_eq!(
            error.code,
            "catalog_recovery_execution_contract_unverifiable"
        );
    }

    #[test]
    fn current_v27_rejects_a_running_frontier_marked_completed_on_reopen() {
        let catalog = active_frontier_catalog();
        let connection = Connection::open(catalog.path()).expect("mutate frontier state");
        connection
            .execute(
                "UPDATE library_metadata_inventory_frontier
                 SET state = 'completed', resume_after_relative_path = NULL,
                     enumerated_entry_count = 0",
                [],
            )
            .expect("prematurely complete frontier");
        drop(connection);

        let error = reopen_recovery_lifecycle_catalog(&catalog)
            .expect_err("running inventory cannot own completed frontier");
        assert_eq!(
            error.code,
            "catalog_recovery_execution_contract_unverifiable"
        );
    }

    #[test]
    fn current_v27_rejects_completed_baseline_with_active_authority_on_reopen() {
        let catalog = recovery_lifecycle_catalog(true);
        let connection = Connection::open(catalog.path()).expect("mutate completed lifecycle");
        connection
            .execute_batch(
                "UPDATE library_change_queue
                 SET status = 'pending', catalog_revision_at_success = NULL
                 WHERE id = 901;
                 UPDATE library_recovery_authorities
                 SET retired_unix_ms = NULL WHERE change_id = 901;",
            )
            .expect("reactivate completed authority");
        drop(connection);

        let error = reopen_recovery_lifecycle_catalog(&catalog)
            .expect_err("completed baseline cannot retain active authority");
        assert_eq!(
            error.code,
            "catalog_persistent_journal_baseline_contract_unverifiable"
        );
    }

    #[test]
    fn current_v27_rejects_completed_baseline_with_noncurrent_root_on_reopen() {
        let catalog = recovery_lifecycle_catalog(true);
        let connection = Connection::open(catalog.path()).expect("mutate completed lifecycle");
        connection
            .execute(
                "UPDATE library_persistent_journal_root_state
                 SET continuity_state = 'catching_up'
                 WHERE root_id = 'lifecycle-root'",
                [],
            )
            .expect("regress completed root continuity");
        drop(connection);

        let error = reopen_recovery_lifecycle_catalog(&catalog)
            .expect_err("completed baseline requires Current root");
        assert_eq!(
            error.code,
            "catalog_persistent_journal_baseline_contract_unverifiable"
        );
    }

    #[test]
    fn current_v27_rejects_completed_baseline_with_noncurrent_checkpoint_on_reopen() {
        let catalog = recovery_lifecycle_catalog(true);
        let connection = Connection::open(catalog.path()).expect("mutate completed lifecycle");
        connection
            .execute(
                "UPDATE library_persistent_journal_checkpoints
                 SET continuity_state = 'catching_up'
                 WHERE root_id = 'lifecycle-root'",
                [],
            )
            .expect("regress completed checkpoint continuity");
        drop(connection);

        let error = reopen_recovery_lifecycle_catalog(&catalog)
            .expect_err("completed baseline requires Current checkpoint");
        assert_eq!(
            error.code,
            "catalog_persistent_journal_baseline_contract_unverifiable"
        );
    }

    #[test]
    fn current_v27_rejects_active_baseline_with_retired_authority_on_reopen() {
        let catalog = recovery_lifecycle_catalog(false);
        let connection = Connection::open(catalog.path()).expect("mutate active lifecycle");
        connection
            .execute_batch(
                "UPDATE library_change_queue
                 SET status = 'completed', catalog_revision_at_success = 0
                 WHERE id = 901;
                 UPDATE library_recovery_authorities
                 SET retired_unix_ms = 2 WHERE change_id = 901;",
            )
            .expect("retire active authority");
        drop(connection);

        let error = reopen_recovery_lifecycle_catalog(&catalog)
            .expect_err("active baseline cannot lose its authority");
        assert_eq!(
            error.code,
            "catalog_persistent_journal_baseline_contract_unverifiable"
        );
    }

    #[test]
    fn current_schema_rejects_active_run_owned_by_a_different_baseline_authority() {
        let catalog = active_frontier_catalog();
        let connection = Connection::open(catalog.path()).expect("mutate active lifecycle");
        connection
            .execute_batch(
                "INSERT INTO library_change_queue(
                   id, root_id, root_generation, intent_kind, scope, relative_path,
                   origin, first_observed_unix_ms, most_recent_observed_unix_ms,
                   first_sequence, most_recent_sequence, coalesced_observation_count,
                   status, ready_unix_ms, catalog_revision_at_enqueue,
                   created_unix_ms, updated_unix_ms
                 ) VALUES (
                   902, 'lifecycle-root', 1, 'freshness_unknown', 'root', '',
                   'metadata_inventory', 2, 2, '2', '2', 1,
                   'pending', 2, 0, 2, 2
                 );
                 UPDATE library_recovery_authorities
                 SET run_id = 'baseline-waiting-run' WHERE change_id = 901;
                 INSERT INTO library_recovery_authorities(
                   change_id, run_id, root_id, root_generation, reason,
                   authorized_unix_ms
                 ) VALUES (
                   902, 'lifecycle-run', 'lifecycle-root', 1,
                   'watcher_uncovered_gap', 2
                 );",
            )
            .expect("insert mismatched active authority");
        drop(connection);

        let error = reopen_recovery_lifecycle_catalog(&catalog)
            .expect_err("active run must own the unfinished baseline authority");
        assert_eq!(
            error.code,
            "catalog_persistent_journal_baseline_contract_unverifiable"
        );
    }

    #[test]
    fn current_schema_rejects_terminal_run_still_referenced_by_unfinished_baseline() {
        let catalog = recovery_lifecycle_catalog(false);
        let connection = Connection::open(catalog.path()).expect("mutate active lifecycle");
        connection
            .execute_batch(
                "INSERT INTO library_metadata_inventory_runs(
                   id, root_id, root_generation, epoch, scope_kind, scope_relative_path,
                   status, next_page_index, started_unix_ms, updated_unix_ms
                 ) VALUES (
                   'lifecycle-run', 'lifecycle-root', 1, 1, 'root', '',
                   'failed', 1, 1, 2
                 );",
            )
            .expect("insert terminal run referenced by unfinished baseline");
        drop(connection);

        let error = reopen_recovery_lifecycle_catalog(&catalog)
            .expect_err("unfinished baseline cannot retain a terminal run owner");
        assert_eq!(
            error.code,
            "catalog_persistent_journal_baseline_contract_unverifiable"
        );
    }

    #[test]
    fn current_v27_rejects_active_baseline_with_current_projection_on_reopen() {
        let catalog = recovery_lifecycle_catalog(false);
        let connection = Connection::open(catalog.path()).expect("mutate active lifecycle");
        connection
            .execute_batch(
                "INSERT INTO library_persistent_journal_checkpoints(
                   root_id, root_generation, volume_guid, volume_serial,
                   root_reference_version, root_file_reference, journal_id,
                   next_unread_usn, captured_exclusive_end, covered_catalog_revision,
                   protocol_version, contract_version, continuity_state,
                   updated_unix_ms
                 ) VALUES (
                   'lifecycle-root', 1, 'lifecycle-volume', '77',
                   3, X'01010101010101010101010101010101', '44',
                   '10', '10', 0, 5, 1, 'current', 2
                 );
                 UPDATE library_persistent_journal_root_state
                 SET continuity_state = 'current', updated_unix_ms = 2
                 WHERE root_id = 'lifecycle-root';",
            )
            .expect("premature Current projection");
        drop(connection);

        let error = reopen_recovery_lifecycle_catalog(&catalog)
            .expect_err("active baseline cannot project Current");
        assert_eq!(
            error.code,
            "catalog_persistent_journal_baseline_contract_unverifiable"
        );
    }

    #[test]
    fn malformed_v24_rolls_back_before_creating_change_lane_authority() {
        let mut connection = Connection::open_in_memory().expect("catalog");
        migrate_schema(&mut connection).expect("fresh current catalog");
        remove_change_lane_v25_contract_for_test(&connection);
        connection
            .execute_batch(
                "UPDATE schema_info SET version = 24;
                 DROP INDEX library_persistent_journal_source_ranges_volume;",
            )
            .expect("malformed v24 fixture");

        let error = migrate_schema(&mut connection).expect_err("malformed v24 must fail closed");
        let retained: (i64, bool) = connection
            .query_row(
                "SELECT
                   (SELECT version FROM schema_info),
                   EXISTS(SELECT 1 FROM sqlite_master
                     WHERE type = 'table' AND name = 'library_change_queue_lanes')",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("rolled back v25 state");
        assert_eq!(
            error.code,
            "catalog_persistent_journal_contract_unverifiable"
        );
        assert_eq!(retained, (24, false));
    }

    #[test]
    fn prerelease_v24_id_triggers_upgrade_atomically_to_typed_guards() {
        let mut connection = Connection::open_in_memory().expect("catalog");
        migrate_schema(&mut connection).expect("fresh v24 catalog");
        for (name, sql) in PERSISTENT_JOURNAL_LEGACY_V24_TRIGGER_DDL {
            connection
                .execute_batch(&format!("DROP TRIGGER {name};"))
                .expect("drop typed trigger");
            connection
                .execute_batch(sql)
                .expect("install legacy trigger");
        }

        migrate_schema(&mut connection).expect("upgrade exact legacy v24 triggers");
        for (name, expected) in PERSISTENT_JOURNAL_CANONICAL_TRIGGER_DDL {
            let actual = connection
                .query_row(
                    "SELECT sql FROM sqlite_master WHERE type = 'trigger' AND name = ?1",
                    [name],
                    |row| row.get::<_, String>(0),
                )
                .expect("upgraded trigger DDL");
            assert_eq!(
                normalize_schema_sql(&actual),
                normalize_schema_sql(expected)
            );
        }
        for invalid in [
            rusqlite::types::Value::Null,
            rusqlite::types::Value::Integer(7),
        ] {
            let error = connection
                .execute(
                    "INSERT INTO library_persistent_journal_source_ranges(id) VALUES (?1)",
                    [invalid],
                )
                .expect_err("typed ID guard must run before row constraints");
            assert!(
                error
                    .to_string()
                    .contains("invalid persistent journal source range id")
            );
        }
    }

    #[test]
    fn current_v20_malformed_inventory_contract_fails_closed() {
        let mut connection = Connection::open_in_memory().expect("catalog");
        migrate_schema(&mut connection).expect("fresh v20 catalog");
        connection
            .execute_batch(
                "DROP INDEX library_metadata_inventory_entries_compare;
                 CREATE INDEX library_metadata_inventory_entries_compare
                   ON library_metadata_inventory_entries(run_id, relative_path);",
            )
            .expect("malformed inventory index fixture");

        let error = migrate_schema(&mut connection).expect_err("malformed inventory contract");

        assert_eq!(
            error.code,
            "catalog_metadata_inventory_contract_unverifiable"
        );
    }

    #[test]
    fn current_v21_incomplete_terminal_media_contract_fails_closed() {
        let mut connection = Connection::open_in_memory().expect("catalog");
        migrate_schema(&mut connection).expect("fresh v21 catalog");
        connection
            .execute_batch(
                "DROP TABLE library_terminal_media_evidence_contract;
                 CREATE TABLE library_terminal_media_evidence_contract (
                   singleton INTEGER PRIMARY KEY,
                   complete INTEGER NOT NULL
                 );
                 INSERT INTO library_terminal_media_evidence_contract(singleton, complete)
                   VALUES (1, 0);",
            )
            .expect("incomplete terminal evidence contract fixture");

        let error = migrate_schema(&mut connection).expect_err("incomplete evidence contract");

        assert_eq!(
            error.code,
            "catalog_terminal_media_evidence_contract_unverifiable"
        );
    }

    #[test]
    fn current_v20_malformed_active_inventory_predicate_fails_closed() {
        let mut connection = Connection::open_in_memory().expect("catalog");
        migrate_schema(&mut connection).expect("fresh v20 catalog");
        connection
            .execute_batch(
                "DROP INDEX library_metadata_inventory_runs_one_active_root;
                 CREATE UNIQUE INDEX library_metadata_inventory_runs_one_active_root
                   ON library_metadata_inventory_runs(root_id)
                   WHERE status = 'running';",
            )
            .expect("malformed active inventory index fixture");

        let error = migrate_schema(&mut connection).expect_err("malformed active inventory index");

        assert_eq!(
            error.code,
            "catalog_metadata_inventory_contract_unverifiable"
        );
    }

    #[test]
    fn current_v20_without_inventory_queue_origin_fails_closed() {
        let mut connection = fresh_v19_catalog();
        let transaction = connection.transaction().expect("v20 fixture transaction");
        create_metadata_inventory_contract(&transaction).expect("inventory contract fixture");
        transaction
            .execute("UPDATE schema_info SET version = 20", [])
            .expect("v20 fixture version");
        transaction.commit().expect("commit v20 fixture");

        let error = migrate_schema(&mut connection).expect_err("missing inventory queue origin");

        assert_eq!(
            error.code,
            "catalog_metadata_inventory_contract_unverifiable"
        );
    }

    #[test]
    fn current_v20_active_inventory_with_stale_generation_fails_closed() {
        let mut connection = Connection::open_in_memory().expect("catalog");
        migrate_schema(&mut connection).expect("fresh v20 catalog");
        connection
            .execute_batch(
                "INSERT INTO library_roots(id, path, active_scan_id, created_unix_ms)
                   VALUES ('root-a', 'C:/source', 'published-scan', 1);
                 INSERT INTO library_change_root_state(
                   root_id, generation, is_active, updated_unix_ms
                 ) VALUES ('root-a', 1, 1, 1);
                 INSERT INTO library_metadata_inventory_runs(
                   id, root_id, root_generation, epoch, scope_kind, scope_relative_path,
                   status, next_page_index, started_unix_ms, updated_unix_ms
                 ) VALUES (
                   'inventory-a', 'root-a', 2, 1, 'root', '', 'running', 1, 1, 1
                 );",
            )
            .expect("stale inventory generation fixture");

        let error = migrate_schema(&mut connection).expect_err("stale active inventory");

        assert_eq!(
            error.code,
            "catalog_metadata_inventory_contract_unverifiable"
        );
    }

    #[test]
    fn current_v20_active_inventory_with_staged_count_mismatch_fails_closed() {
        let mut connection = Connection::open_in_memory().expect("catalog");
        migrate_schema(&mut connection).expect("fresh v20 catalog");
        connection
            .execute_batch(
                "INSERT INTO library_roots(id, path, created_unix_ms)
                   VALUES ('root-a', 'C:/source', 1);
                 INSERT INTO scan_runs(
                   id, root_id, status, started_unix_ms, completed_unix_ms, preview_edge
                 ) VALUES ('published-scan', 'root-a', 'completed', 1, 1, 128);
                 UPDATE library_roots SET active_scan_id = 'published-scan' WHERE id = 'root-a';
                 INSERT INTO library_change_root_state(
                   root_id, generation, is_active, updated_unix_ms
                 ) VALUES ('root-a', 1, 1, 1);
                 INSERT INTO library_metadata_inventory_runs(
                   id, root_id, root_generation, epoch, scope_kind, scope_relative_path,
                   status, next_page_index, staged_entry_count, started_unix_ms, updated_unix_ms
                 ) VALUES (
                   'inventory-a', 'root-a', 1, 1, 'root', '', 'running', 2, 1, 1, 1
                 );",
            )
            .expect("active staged-count mismatch fixture");

        let error = migrate_schema(&mut connection).expect_err("active staged-count mismatch");

        assert_eq!(
            error.code,
            "catalog_metadata_inventory_contract_unverifiable"
        );
    }

    type CatchUpAuthorityMigrationState = (
        i64,
        bool,
        i64,
        i64,
        i64,
        i64,
        i64,
        i64,
        i64,
        bool,
        bool,
        bool,
        bool,
        bool,
    );

    #[test]
    fn prerelease_v19_without_catch_up_marker_fails_closed() {
        let mut connection = Connection::open_in_memory().expect("catalog");
        connection
            .execute_batch(
                "CREATE TABLE schema_info(version INTEGER NOT NULL);
                 INSERT INTO schema_info(version) VALUES (19);
                 CREATE TABLE library_change_queue_contract(
                   singleton INTEGER PRIMARY KEY CHECK(singleton = 1),
                   root_authority_complete INTEGER NOT NULL CHECK(root_authority_complete = 1),
                   authoritative_recovery_complete INTEGER NOT NULL
                     CHECK(authoritative_recovery_complete = 1),
                   scan_ownership_complete INTEGER NOT NULL
                     CHECK(scan_ownership_complete = 1)
                 );
                 INSERT INTO library_change_queue_contract VALUES (1, 1, 1, 1);",
            )
            .expect("prerelease v19 fixture without catch-up marker");

        let error = migrate_schema(&mut connection).expect_err("missing catch-up contract");

        assert_eq!(error.code, "catalog_change_catch_up_contract_unverifiable");
    }

    #[test]
    fn prerelease_v19_with_complete_marker_repairs_missing_path_index() {
        let mut connection = fresh_v19_catalog();
        connection
            .execute("DROP INDEX asset_locations_root_relative", [])
            .expect("restore prerelease v19 index state");

        migrate_schema(&mut connection).expect("repair derived lookup index");

        let index_columns = connection
            .prepare(
                "SELECT name FROM pragma_index_info('asset_locations_root_relative')
                 ORDER BY seqno",
            )
            .expect("index query")
            .query_map([], |row| row.get::<_, String>(0))
            .expect("index rows")
            .collect::<Result<Vec<_>, _>>()
            .expect("index columns");
        assert_eq!(
            index_columns,
            ["root_id", "relative_path", "scan_id", "location_id"]
        );
    }

    #[test]
    fn prerelease_v19_with_complete_marker_adds_durable_handoff_contract() {
        let mut connection = fresh_v19_catalog();
        connection
            .execute_batch(
                "INSERT INTO library_change_queue(
                   root_id, root_generation, intent_kind, scope, relative_path,
                   previous_relative_path, origin, first_observed_unix_ms,
                   most_recent_observed_unix_ms, first_sequence, most_recent_sequence,
                   coalesced_observation_count, status, ready_unix_ms,
                   catalog_revision_at_enqueue, catch_up_source, catch_up_watermark,
                   created_unix_ms, updated_unix_ms
                 ) VALUES (
                   'root-a', 1, 'reconcile', 'path', 'photo.jpg', NULL,
                   'startup_catch_up', 1, 1, '1', '1', 1, 'pending', 1, 0,
                   'windows_usn_v1', 'volume|12|40', 1, 1
                 );
                 DROP TABLE library_change_scan_handoff_items;
                 DROP TABLE library_change_scan_handoff_lineage;
                 DROP TABLE library_change_scan_handoff_batches;
                 DROP TABLE library_change_queue_catch_up_lineage;
                 DROP TABLE scan_run_catch_up_lineage;
                 DROP TABLE library_change_catch_up_handoffs;
                 ALTER TABLE library_change_queue_contract
                   DROP COLUMN scan_catch_up_lineage_complete;
                 ALTER TABLE library_change_queue_contract
                   DROP COLUMN scan_handoff_batch_complete;
                 CREATE INDEX library_change_queue_catch_up_peer
                   ON library_change_queue(
                     catch_up_source, catch_up_watermark, status, root_id, id
                   );",
            )
            .expect("restore prerelease v19 handoff state");

        migrate_schema(&mut connection).expect("add durable catch-up handoff contract");

        let index_columns = connection
            .prepare(
                "SELECT name FROM pragma_index_info('library_change_catch_up_handoffs_asset')
                 ORDER BY seqno",
            )
            .expect("handoff index query")
            .query_map([], |row| row.get::<_, String>(0))
            .expect("handoff index rows")
            .collect::<Result<Vec<_>, _>>()
            .expect("handoff index columns");
        assert_eq!(
            index_columns,
            ["asset_id", "catch_up_source", "catch_up_watermark"]
        );
        let seeded_lineage = connection
            .query_row(
                "SELECT COUNT(*) FROM library_change_queue_catch_up_lineage
                 WHERE catch_up_source = 'windows_usn_v1'
                   AND catch_up_watermark = 'volume|12|40'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .expect("seeded lineage count");
        assert_eq!(seeded_lineage, 1);
        let scan_lineage_contract = connection
            .query_row(
                "SELECT
                   (SELECT scan_catch_up_lineage_complete = 1
                           AND scan_handoff_batch_complete = 1
                    FROM library_change_queue_contract WHERE singleton = 1)
                   AND EXISTS(SELECT 1 FROM sqlite_master
                     WHERE type = 'table' AND name = 'scan_run_catch_up_lineage')
                   AND EXISTS(SELECT 1 FROM sqlite_master
                     WHERE type = 'index' AND name = 'scan_run_catch_up_lineage_evidence')",
                [],
                |row| row.get::<_, bool>(0),
            )
            .expect("scan lineage contract");
        assert!(scan_lineage_contract);
        let obsolete_index = connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master
                 WHERE type = 'index' AND name = 'library_change_queue_catch_up_peer')",
                [],
                |row| row.get::<_, bool>(0),
            )
            .expect("obsolete peer index query");
        assert!(!obsolete_index);
    }

    #[test]
    fn prerelease_v19_with_unprovable_handoff_lineage_fails_closed() {
        let mut connection = Connection::open_in_memory().expect("catalog");
        migrate_schema(&mut connection).expect("fresh v19 catalog");
        connection
            .execute_batch(
                "INSERT INTO library_change_catch_up_handoffs(
                   catch_up_source, catch_up_watermark,
                   file_identity_scheme, file_identity_value,
                   asset_id, source_location_id, root_id, absolute_path, relative_path,
                   preview_path, file_size, created_unix_ms, modified_unix_ms,
                   width, height, preview_status, preview_issue_code, preview_issue_message,
                   metadata_engine_id, metadata_engine_version, capture_local_time,
                   capture_offset_minutes, capture_time_source, capture_raw_value,
                   updated_unix_ms
                 ) VALUES (
                   'windows_usn_v1', 'watermark-1', 'windows-file-id-128-v1', 'volume:file',
                   'asset-a', 'location-a', 'root-a', 'C:/source/photo.jpg', 'photo.jpg',
                   '', 1, NULL, 1, 1, 1, 'pending', NULL, NULL,
                   'metadata', '1', NULL, NULL, NULL, NULL, 1
                 );
                 DROP TABLE library_change_scan_handoff_items;
                 DROP TABLE library_change_scan_handoff_lineage;
                 DROP TABLE library_change_scan_handoff_batches;
                 ALTER TABLE library_change_queue_contract
                   DROP COLUMN scan_handoff_batch_complete;",
            )
            .expect("unprovable prerelease lineage");

        let error = migrate_schema(&mut connection).expect_err("unprovable handoff lineage");

        assert_eq!(error.code, "catalog_change_catch_up_contract_unverifiable");
    }

    #[test]
    fn prerelease_v19_with_malformed_path_index_fails_closed() {
        let mut connection = Connection::open_in_memory().expect("catalog");
        migrate_schema(&mut connection).expect("fresh v19 catalog");
        connection
            .execute_batch(
                "DROP INDEX asset_locations_root_relative;
                 CREATE INDEX asset_locations_root_relative
                   ON asset_locations(root_id);",
            )
            .expect("malformed prerelease index");

        let error = migrate_schema(&mut connection).expect_err("malformed derived lookup index");

        assert_eq!(error.code, "catalog_change_catch_up_contract_unverifiable");
    }

    #[test]
    fn current_v19_lineage_with_wrong_foreign_key_target_fails_closed() {
        let mut connection = Connection::open_in_memory().expect("catalog");
        migrate_schema(&mut connection).expect("fresh v19 catalog");
        connection
            .execute_batch(
                "DROP TABLE library_change_queue_catch_up_lineage;
                 CREATE TABLE library_change_queue_catch_up_lineage (
                   change_id INTEGER NOT NULL,
                   catch_up_source TEXT NOT NULL,
                   catch_up_watermark TEXT NOT NULL,
                   enrolled_unix_ms INTEGER NOT NULL,
                   PRIMARY KEY(change_id, catch_up_source, catch_up_watermark),
                   FOREIGN KEY(change_id) REFERENCES library_change_queue(root_id)
                     ON DELETE CASCADE
                 );
                 CREATE INDEX library_change_queue_catch_up_lineage_evidence
                   ON library_change_queue_catch_up_lineage(
                     catch_up_source, catch_up_watermark, change_id
                   );",
            )
            .expect("wrong-target lineage fixture");

        let error = migrate_schema(&mut connection).expect_err("wrong-target lineage foreign key");

        assert_eq!(error.code, "catalog_change_catch_up_contract_unverifiable");
    }

    #[test]
    fn current_v19_lineage_with_extra_required_column_fails_closed() {
        let mut connection = Connection::open_in_memory().expect("catalog");
        migrate_schema(&mut connection).expect("fresh v19 catalog");
        connection
            .execute_batch(
                "DROP TABLE library_change_queue_catch_up_lineage;
                 CREATE TABLE library_change_queue_catch_up_lineage (
                   change_id INTEGER NOT NULL,
                   catch_up_source TEXT NOT NULL,
                   catch_up_watermark TEXT NOT NULL,
                   enrolled_unix_ms INTEGER NOT NULL,
                   unexpected_authority TEXT NOT NULL,
                   PRIMARY KEY(change_id, catch_up_source, catch_up_watermark),
                   FOREIGN KEY(change_id) REFERENCES library_change_queue(id) ON DELETE CASCADE
                 );
                 CREATE INDEX library_change_queue_catch_up_lineage_evidence
                   ON library_change_queue_catch_up_lineage(
                     catch_up_source, catch_up_watermark, change_id
                   );",
            )
            .expect("extra-column lineage fixture");

        let error = migrate_schema(&mut connection).expect_err("extra required lineage column");

        assert_eq!(error.code, "catalog_change_catch_up_contract_unverifiable");
    }

    #[test]
    fn current_v19_checkpoint_with_incomplete_shape_fails_closed() {
        let mut connection = Connection::open_in_memory().expect("catalog");
        migrate_schema(&mut connection).expect("fresh v19 catalog");
        connection
            .execute_batch(
                "DROP TABLE library_change_catch_up_state;
                 CREATE TABLE library_change_catch_up_state(
                   volume_id TEXT PRIMARY KEY
                 );",
            )
            .expect("incomplete checkpoint fixture");

        let error = migrate_schema(&mut connection).expect_err("incomplete checkpoint contract");

        assert_eq!(error.code, "catalog_change_catch_up_contract_unverifiable");
    }

    #[test]
    fn current_v19_legacy_handoff_with_extra_required_column_fails_closed() {
        let mut connection = Connection::open_in_memory().expect("catalog");
        migrate_schema(&mut connection).expect("fresh v19 catalog");
        connection
            .execute_batch(
                "ALTER TABLE library_change_catch_up_handoffs
                   ADD COLUMN unexpected_owner TEXT NOT NULL DEFAULT 'unknown';",
            )
            .expect("extra legacy handoff column fixture");

        let error = migrate_schema(&mut connection).expect_err("extra legacy handoff column");

        assert_eq!(error.code, "catalog_change_catch_up_contract_unverifiable");
    }

    #[test]
    fn current_v19_orphan_lineage_fails_closed() {
        let mut connection = Connection::open_in_memory().expect("catalog");
        migrate_schema(&mut connection).expect("fresh v19 catalog");
        connection
            .execute_batch(
                "PRAGMA foreign_keys = OFF;
                 INSERT INTO library_change_queue_catch_up_lineage(
                   change_id, catch_up_source, catch_up_watermark, enrolled_unix_ms
                 ) VALUES (999, 'windows_usn_v1', 'orphan-watermark', 1);
                 PRAGMA foreign_keys = ON;",
            )
            .expect("orphan lineage fixture");

        let error = migrate_schema(&mut connection).expect_err("orphan lineage");

        assert_eq!(error.code, "catalog_change_catch_up_contract_unverifiable");
    }

    #[test]
    fn current_v19_scan_lineage_with_wrong_foreign_key_target_fails_closed() {
        let mut connection = Connection::open_in_memory().expect("catalog");
        migrate_schema(&mut connection).expect("fresh v19 catalog");
        connection
            .execute_batch(
                "DROP TABLE scan_run_catch_up_lineage;
                 CREATE TABLE scan_run_catch_up_lineage (
                   scan_id TEXT NOT NULL,
                   catch_up_source TEXT NOT NULL,
                   catch_up_watermark TEXT NOT NULL,
                   enrolled_unix_ms INTEGER NOT NULL,
                   PRIMARY KEY(scan_id, catch_up_source, catch_up_watermark),
                   FOREIGN KEY(scan_id) REFERENCES scan_runs(root_id) ON DELETE CASCADE
                 );
                 CREATE INDEX scan_run_catch_up_lineage_evidence
                   ON scan_run_catch_up_lineage(
                     catch_up_source, catch_up_watermark, scan_id
                   );",
            )
            .expect("wrong-target scan lineage fixture");

        let error = migrate_schema(&mut connection).expect_err("wrong-target scan lineage key");

        assert_eq!(error.code, "catalog_change_catch_up_contract_unverifiable");
    }

    #[test]
    fn current_v19_orphan_scan_lineage_fails_closed() {
        let mut connection = Connection::open_in_memory().expect("catalog");
        migrate_schema(&mut connection).expect("fresh v19 catalog");
        connection
            .execute_batch(
                "PRAGMA foreign_keys = OFF;
                 INSERT INTO scan_run_catch_up_lineage(
                   scan_id, catch_up_source, catch_up_watermark, enrolled_unix_ms
                 ) VALUES ('missing-scan', 'windows_usn_v1', 'orphan-watermark', 1);
                 PRAGMA foreign_keys = ON;",
            )
            .expect("orphan scan lineage fixture");

        let error = migrate_schema(&mut connection).expect_err("orphan scan lineage");

        assert_eq!(error.code, "catalog_change_catch_up_contract_unverifiable");
    }

    #[test]
    fn current_v19_orphan_handoff_fails_closed() {
        let mut connection = Connection::open_in_memory().expect("catalog");
        migrate_schema(&mut connection).expect("fresh v19 catalog");
        connection
            .execute_batch(
                "INSERT INTO library_change_catch_up_handoffs(
                   catch_up_source, catch_up_watermark,
                   file_identity_scheme, file_identity_value,
                   asset_id, source_location_id, root_id, absolute_path, relative_path,
                   preview_path, file_size, created_unix_ms, modified_unix_ms,
                   width, height, preview_status, preview_issue_code, preview_issue_message,
                   metadata_engine_id, metadata_engine_version, capture_local_time,
                   capture_offset_minutes, capture_time_source, capture_raw_value,
                   updated_unix_ms
                 ) VALUES (
                   'windows_usn_v1', 'orphan-watermark', 'windows-file-id-128-v1', 'volume:file',
                   'asset-a', 'location-a', 'root-a', 'C:/source/photo.jpg', 'photo.jpg',
                   '', 1, NULL, 1, 1, 1, 'pending', NULL, NULL,
                   'metadata', '1', NULL, NULL, NULL, NULL, 1
                 );",
            )
            .expect("orphan handoff fixture");

        let error = migrate_schema(&mut connection).expect_err("orphan handoff");

        assert_eq!(error.code, "catalog_change_catch_up_contract_unverifiable");
    }

    #[test]
    fn current_v19_orphan_scan_handoff_batch_fails_closed() {
        let mut connection = Connection::open_in_memory().expect("catalog");
        migrate_schema(&mut connection).expect("fresh v19 catalog");
        connection
            .execute_batch(
                "INSERT INTO library_change_scan_handoff_batches(
                   id, source_root_id, updated_unix_ms
                 ) VALUES ('batch-a', 'root-a', 1);
                 INSERT INTO library_change_scan_handoff_lineage(
                   batch_id, catch_up_source, catch_up_watermark, enrolled_unix_ms
                 ) VALUES ('batch-a', 'windows_usn_v1', 'orphan-watermark', 1);
                 INSERT INTO library_change_scan_handoff_items(
                   batch_id, file_identity_scheme, file_identity_value,
                   asset_id, source_location_id, root_id, absolute_path, relative_path,
                   preview_path, file_size, created_unix_ms, modified_unix_ms,
                   width, height, preview_status, preview_issue_code, preview_issue_message,
                   metadata_engine_id, metadata_engine_version, capture_local_time,
                   capture_offset_minutes, capture_time_source, capture_raw_value
                 ) VALUES (
                   'batch-a', 'windows-file-id-128-v1', 'volume:file',
                   'asset-a', 'location-a', 'root-a', 'C:/source/photo.jpg', 'photo.jpg',
                   '', 1, NULL, 1, 1, 1, 'pending', NULL, NULL,
                   'metadata', '1', NULL, NULL, NULL, NULL
                 );",
            )
            .expect("orphan scan handoff fixture");

        let error = migrate_schema(&mut connection).expect_err("orphan scan handoff batch");

        assert_eq!(error.code, "catalog_change_catch_up_contract_unverifiable");
    }

    #[test]
    fn current_v19_scan_handoff_lineage_with_wrong_foreign_key_fails_closed() {
        let mut connection = Connection::open_in_memory().expect("catalog");
        migrate_schema(&mut connection).expect("fresh v19 catalog");
        connection
            .execute_batch(
                "DROP TABLE library_change_scan_handoff_lineage;
                 CREATE TABLE library_change_scan_handoff_lineage (
                   batch_id TEXT NOT NULL,
                   catch_up_source TEXT NOT NULL,
                   catch_up_watermark TEXT NOT NULL,
                   enrolled_unix_ms INTEGER NOT NULL,
                   PRIMARY KEY(batch_id, catch_up_source, catch_up_watermark),
                   FOREIGN KEY(batch_id)
                     REFERENCES library_change_scan_handoff_batches(source_root_id)
                     ON DELETE CASCADE
                 );
                 CREATE INDEX library_change_scan_handoff_lineage_evidence
                   ON library_change_scan_handoff_lineage(
                     catch_up_source, catch_up_watermark, batch_id
                   );",
            )
            .expect("wrong-target scan handoff lineage fixture");

        let error = migrate_schema(&mut connection).expect_err("wrong-target scan handoff key");

        assert_eq!(error.code, "catalog_change_catch_up_contract_unverifiable");
    }

    #[test]
    fn current_v19_unbacked_active_scan_lineage_fails_closed() {
        let mut connection = Connection::open_in_memory().expect("catalog");
        migrate_schema(&mut connection).expect("fresh v19 catalog");
        connection
            .execute_batch(
                "INSERT INTO library_roots(id, path, created_unix_ms)
                 VALUES ('root-a', 'C:/source', 1);
                 INSERT INTO scan_runs(
                   id, root_id, status, scan_owner, started_unix_ms, preview_edge,
                   root_generation_at_start, change_queue_high_watermark
                 ) VALUES (
                   'scan-a', 'root-a', 'running', 'authoritative_recovery', 1, 128, 1, 1
                 );
                 INSERT INTO scan_run_catch_up_lineage(
                   scan_id, catch_up_source, catch_up_watermark, enrolled_unix_ms
                 ) VALUES ('scan-a', 'windows_usn_v1', 'unbacked-watermark', 1);",
            )
            .expect("unbacked active scan lineage fixture");

        let error = migrate_schema(&mut connection).expect_err("unbacked active scan lineage");

        assert_eq!(error.code, "catalog_change_catch_up_contract_unverifiable");
    }

    #[test]
    fn current_v19_malformed_preview_repair_marker_fails_closed() {
        let mut connection = Connection::open_in_memory().expect("catalog");
        migrate_schema(&mut connection).expect("fresh v19 catalog");
        connection
            .execute_batch(
                "DROP TABLE library_change_preview_repair_contract;
                 CREATE TABLE library_change_preview_repair_contract(
                   singleton INTEGER PRIMARY KEY
                 );
                 INSERT INTO library_change_preview_repair_contract(singleton) VALUES (1);",
            )
            .expect("malformed preview repair marker fixture");

        let error = migrate_schema(&mut connection).expect_err("malformed preview repair marker");

        assert_eq!(error.code, "catalog_change_catch_up_contract_unverifiable");
    }

    #[test]
    fn concurrent_prerelease_v19_preview_repair_accepts_completed_marker() {
        let catalog = NamedTempFile::new().expect("temporary catalog");
        let mut first = Connection::open(catalog.path()).expect("first catalog connection");
        migrate_schema(&mut first).expect("fresh v19 catalog");
        first
            .execute_batch("DROP TABLE library_change_preview_repair_contract")
            .expect("restore prerelease preview repair marker");
        let mut second = Connection::open(catalog.path()).expect("second catalog connection");

        assert!(!preview_repair_marker_is_complete(&first).expect("first marker preflight"));
        assert!(!preview_repair_marker_is_complete(&second).expect("second marker preflight"));

        repair_missing_v19_preview_expectation_marker(&mut first)
            .expect("first connection preview repair");
        repair_missing_v19_preview_expectation_marker(&mut second)
            .expect("second connection accepts completed repair");

        assert!(preview_repair_marker_is_complete(&first).expect("first repaired marker"));
        assert!(preview_repair_marker_is_complete(&second).expect("second repaired marker"));
    }

    #[test]
    fn prerelease_v18_without_recovery_marker_fails_closed() {
        let mut connection = Connection::open_in_memory().expect("catalog");
        connection
            .execute_batch(
                "CREATE TABLE schema_info(version INTEGER NOT NULL);
                 INSERT INTO schema_info(version) VALUES (18);
                 CREATE TABLE library_change_queue_contract(
                   singleton INTEGER PRIMARY KEY CHECK(singleton = 1),
                   root_authority_complete INTEGER NOT NULL CHECK(root_authority_complete = 1)
                 );
                 INSERT INTO library_change_queue_contract VALUES (1, 1);",
            )
            .expect("prerelease v18 fixture without recovery marker");

        let error = migrate_schema(&mut connection).expect_err("missing recovery contract");

        assert_eq!(
            error.code,
            "catalog_authoritative_recovery_contract_unverifiable"
        );
    }

    #[test]
    fn prerelease_v18_repairs_unambiguous_scan_ownership_contract() {
        let mut connection = Connection::open_in_memory().expect("catalog");
        connection
            .execute_batch(
                "CREATE TABLE schema_info(version INTEGER NOT NULL);
                 INSERT INTO schema_info(version) VALUES (18);
                 CREATE TABLE library_change_queue_contract(
                   singleton INTEGER PRIMARY KEY CHECK(singleton = 1),
                   root_authority_complete INTEGER NOT NULL CHECK(root_authority_complete = 1),
                   authoritative_recovery_complete INTEGER NOT NULL
                     CHECK(authoritative_recovery_complete = 1)
                 );
                 INSERT INTO library_change_queue_contract VALUES (1, 1, 1);
                 CREATE TABLE library_roots(
                   id TEXT PRIMARY KEY,
                   active_scan_id TEXT
                 );
                 CREATE TABLE scan_runs(
                   id TEXT PRIMARY KEY,
                   root_id TEXT NOT NULL,
                   status TEXT NOT NULL,
                   root_generation_at_start INTEGER,
                   change_queue_high_watermark INTEGER
                 );
                  CREATE TABLE asset_locations(
                    root_id TEXT NOT NULL,
                    relative_path TEXT NOT NULL,
                    scan_id TEXT NOT NULL,
                    location_id TEXT NOT NULL,
                    preview_path TEXT NOT NULL DEFAULT '',
                    preview_status TEXT NOT NULL DEFAULT 'pending'
                  );
                  CREATE TABLE preview_artifacts(
                    artifact_path TEXT NOT NULL,
                    lifecycle_state TEXT NOT NULL
                  );
                  CREATE TABLE library_change_queue(
                    id INTEGER PRIMARY KEY,
                    root_id TEXT NOT NULL,
                    root_generation INTEGER NOT NULL DEFAULT 1,
                    status TEXT NOT NULL,
                    catch_up_source TEXT,
                    catch_up_watermark TEXT
                  );
                  INSERT INTO scan_runs(id, root_id, status)
                  VALUES ('foreground-a', 'root-a', 'running');
                 INSERT INTO scan_runs(id, root_id, status) VALUES (
                   'sync-recovery-1-2-3', 'root-b', 'running'
                 );",
            )
            .expect("repairable prerelease v18 fixture");

        repair_prerelease_v18_scan_owner_index(&mut connection)
            .expect("repair scan ownership index");
        let (has_index, ownership_marker, foreground_owner, recovery_owner): (
            bool,
            bool,
            String,
            String,
        ) = connection
            .query_row(
                "SELECT
                   EXISTS(SELECT 1 FROM pragma_index_list('scan_runs')
                     WHERE name = 'scan_runs_one_active_root'
                       AND \"unique\" = 1 AND partial = 1),
                   (SELECT scan_ownership_complete = 1
                    FROM library_change_queue_contract WHERE singleton = 1),
                   (SELECT scan_owner FROM scan_runs WHERE id = 'foreground-a'),
                   (SELECT scan_owner FROM scan_runs WHERE id = 'sync-recovery-1-2-3')",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .expect("scan index evidence");

        assert!(has_index);
        assert!(ownership_marker);
        assert_eq!(foreground_owner, "foreground");
        assert_eq!(recovery_owner, "authoritative_recovery");
    }

    #[test]
    fn prerelease_v18_with_existing_index_repairs_missing_scan_owner() {
        let mut connection = Connection::open_in_memory().expect("catalog");
        connection
            .execute_batch(
                "CREATE TABLE schema_info(version INTEGER NOT NULL);
                 INSERT INTO schema_info(version) VALUES (18);
                 CREATE TABLE library_change_queue_contract(
                   singleton INTEGER PRIMARY KEY CHECK(singleton = 1),
                   root_authority_complete INTEGER NOT NULL CHECK(root_authority_complete = 1),
                   authoritative_recovery_complete INTEGER NOT NULL
                     CHECK(authoritative_recovery_complete = 1)
                 );
                 INSERT INTO library_change_queue_contract VALUES (1, 1, 1);
                 CREATE TABLE library_roots(
                   id TEXT PRIMARY KEY,
                   active_scan_id TEXT
                 );
                 CREATE TABLE scan_runs(
                   id TEXT PRIMARY KEY,
                   root_id TEXT NOT NULL,
                   status TEXT NOT NULL,
                   root_generation_at_start INTEGER,
                   change_queue_high_watermark INTEGER
                 );
                  CREATE TABLE asset_locations(
                    root_id TEXT NOT NULL,
                    relative_path TEXT NOT NULL,
                    scan_id TEXT NOT NULL,
                    location_id TEXT NOT NULL,
                    preview_path TEXT NOT NULL DEFAULT '',
                    preview_status TEXT NOT NULL DEFAULT 'pending'
                  );
                  CREATE TABLE preview_artifacts(
                    artifact_path TEXT NOT NULL,
                    lifecycle_state TEXT NOT NULL
                  );
                  CREATE TABLE library_change_queue(
                    id INTEGER PRIMARY KEY,
                    root_id TEXT NOT NULL,
                    root_generation INTEGER NOT NULL DEFAULT 1,
                    status TEXT NOT NULL,
                    catch_up_source TEXT,
                    catch_up_watermark TEXT
                  );
                  CREATE UNIQUE INDEX scan_runs_one_active_root
                   ON scan_runs(root_id) WHERE status IN ('running', 'paused');
                 INSERT INTO scan_runs(id, root_id, status) VALUES (
                   'sync-recovery-7-8-9', 'root-a', 'running'
                 );",
            )
            .expect("indexed prerelease v18 fixture");

        repair_prerelease_v18_scan_owner_index(&mut connection).expect("repair missing scan owner");
        let (owner, ownership_marker): (String, bool) = connection
            .query_row(
                "SELECT
                   (SELECT scan_owner FROM scan_runs WHERE id = 'sync-recovery-7-8-9'),
                   (SELECT scan_ownership_complete = 1
                    FROM library_change_queue_contract WHERE singleton = 1)",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("repaired ownership evidence");

        assert_eq!(owner, "authoritative_recovery");
        assert!(ownership_marker);
    }

    #[test]
    fn prerelease_v18_with_overlapping_scans_fails_closed() {
        let mut connection = Connection::open_in_memory().expect("catalog");
        connection
            .execute_batch(
                "CREATE TABLE schema_info(version INTEGER NOT NULL);
                 INSERT INTO schema_info(version) VALUES (18);
                 CREATE TABLE library_change_queue_contract(
                   singleton INTEGER PRIMARY KEY CHECK(singleton = 1),
                   root_authority_complete INTEGER NOT NULL CHECK(root_authority_complete = 1),
                   authoritative_recovery_complete INTEGER NOT NULL
                     CHECK(authoritative_recovery_complete = 1)
                 );
                 INSERT INTO library_change_queue_contract VALUES (1, 1, 1);
                 CREATE TABLE scan_runs(
                   id TEXT PRIMARY KEY,
                   root_id TEXT NOT NULL,
                   status TEXT NOT NULL
                 );
                 INSERT INTO scan_runs VALUES ('scan-a', 'root-a', 'running');
                 INSERT INTO scan_runs VALUES ('scan-b', 'root-a', 'paused');",
            )
            .expect("conflicting prerelease v18 fixture");

        let error = migrate_schema(&mut connection).expect_err("ambiguous scan ownership");

        assert_eq!(
            error.code,
            "catalog_authoritative_recovery_contract_unverifiable"
        );
    }

    #[test]
    fn v18_migration_normalizes_existing_relative_paths_and_invalidates_old_running_scans() {
        let mut connection = Connection::open_in_memory().expect("catalog");
        connection
            .execute_batch(
                "CREATE TABLE schema_info(version INTEGER NOT NULL);
                 INSERT INTO schema_info(version) VALUES (16);
                 CREATE TABLE library_roots(id TEXT PRIMARY KEY);
                 INSERT INTO library_roots(id) VALUES ('root-a');
                 CREATE TABLE scan_runs(
                   id TEXT PRIMARY KEY,
                   root_id TEXT NOT NULL,
                   status TEXT NOT NULL,
                   started_unix_ms INTEGER NOT NULL,
                   completed_unix_ms INTEGER,
                   current_directory_relative_path TEXT,
                   current_directory_enumerated INTEGER NOT NULL,
                   last_visited_relative_path TEXT
                 );
                 INSERT INTO scan_runs VALUES (
                   'scan-a', 'root-a', 'running', 1, NULL, 'album\\nested', 1,
                   'album\\nested\\photo.png'
                 );
                 CREATE TABLE asset_locations(relative_path TEXT NOT NULL);
                 INSERT INTO asset_locations VALUES ('album\\nested\\photo.png');
                 CREATE TABLE scan_directory_frontier(relative_path TEXT NOT NULL);
                 INSERT INTO scan_directory_frontier VALUES ('album\\nested');
                 CREATE TABLE scan_directory_entries(relative_path TEXT NOT NULL);
                 INSERT INTO scan_directory_entries VALUES ('album\\nested\\photo.png');",
            )
            .expect("v16 fixture");
        migrate_v16_to_v17(&mut connection).expect("v17 migration");
        migrate_v17_to_v18(&mut connection).expect("v18 migration");

        let (version, scan_status, location_path, frontier_path, entry_path): (
            i64,
            String,
            String,
            String,
            String,
        ) = connection
            .query_row(
                "SELECT
                   (SELECT version FROM schema_info),
                   (SELECT status FROM scan_runs WHERE id = 'scan-a'),
                   (SELECT relative_path FROM asset_locations),
                   (SELECT relative_path FROM scan_directory_frontier),
                   (SELECT relative_path FROM scan_directory_entries)",
                [],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                    ))
                },
            )
            .expect("migrated state");

        assert_eq!(version, 18);
        assert_eq!(scan_status, "interrupted_unrecoverable");
        assert_eq!(location_path, "album/nested/photo.png");
        assert_eq!(frontier_path, "album/nested");
        assert_eq!(entry_path, "album/nested/photo.png");
    }

    #[test]
    fn v19_migration_adds_empty_catch_up_authority() {
        let mut connection = Connection::open_in_memory().expect("catalog");
        connection
            .execute_batch(
                "CREATE TABLE schema_info(version INTEGER NOT NULL);
                 INSERT INTO schema_info(version) VALUES (16);
                 CREATE TABLE asset_locations(
                   root_id TEXT NOT NULL,
                   relative_path TEXT NOT NULL,
                   scan_id TEXT NOT NULL,
                   location_id TEXT NOT NULL
                 );",
            )
            .expect("v16 fixture");
        migrate_v16_to_v17(&mut connection).expect("v17 migration");
        migrate_v17_to_v18(&mut connection).expect("v18 migration");
        migrate_v18_to_v19(&mut connection).expect("v19 migration");

        let (
            version,
            marker,
            checkpoint_count,
            handoff_count,
            lineage_count,
            scan_lineage_count,
            scan_handoff_batch_count,
            scan_handoff_lineage_count,
            scan_handoff_item_count,
            has_path_index,
            has_handoff_index,
            has_lineage_index,
            has_scan_lineage_index,
            has_scan_handoff_index,
        ): CatchUpAuthorityMigrationState = connection
            .query_row(
                "SELECT
                   (SELECT version FROM schema_info),
                   (SELECT change_catch_up_complete = 1
                           AND scan_catch_up_lineage_complete = 1
                           AND scan_handoff_batch_complete = 1
                    FROM library_change_queue_contract WHERE singleton = 1),
                   (SELECT COUNT(*) FROM library_change_catch_up_state),
                   (SELECT COUNT(*) FROM library_change_catch_up_handoffs),
                   (SELECT COUNT(*) FROM library_change_queue_catch_up_lineage),
                   (SELECT COUNT(*) FROM scan_run_catch_up_lineage),
                   (SELECT COUNT(*) FROM library_change_scan_handoff_batches),
                   (SELECT COUNT(*) FROM library_change_scan_handoff_lineage),
                   (SELECT COUNT(*) FROM library_change_scan_handoff_items),
                   EXISTS(SELECT 1 FROM sqlite_master
                     WHERE type = 'index' AND name = 'asset_locations_root_relative'),
                   EXISTS(SELECT 1 FROM sqlite_master
                     WHERE type = 'index'
                       AND name = 'library_change_catch_up_handoffs_asset'),
                   EXISTS(SELECT 1 FROM sqlite_master
                     WHERE type = 'index'
                       AND name = 'library_change_queue_catch_up_lineage_evidence'),
                   EXISTS(SELECT 1 FROM sqlite_master
                     WHERE type = 'index'
                       AND name = 'scan_run_catch_up_lineage_evidence'),
                   EXISTS(SELECT 1 FROM sqlite_master
                     WHERE type = 'index'
                       AND name = 'library_change_scan_handoff_items_identity')",
                [],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                        row.get(6)?,
                        row.get(7)?,
                        row.get(8)?,
                        row.get(9)?,
                        row.get(10)?,
                        row.get(11)?,
                        row.get(12)?,
                        row.get(13)?,
                    ))
                },
            )
            .expect("v19 evidence");

        assert_eq!(version, 19);
        assert!(marker);
        assert_eq!(checkpoint_count, 0);
        assert_eq!(handoff_count, 0);
        assert_eq!(lineage_count, 0);
        assert_eq!(scan_lineage_count, 0);
        assert_eq!(scan_handoff_batch_count, 0);
        assert_eq!(scan_handoff_lineage_count, 0);
        assert_eq!(scan_handoff_item_count, 0);
        assert!(has_path_index);
        assert!(has_handoff_index);
        assert!(has_lineage_index);
        assert!(has_scan_lineage_index);
        assert!(has_scan_handoff_index);
    }
}
