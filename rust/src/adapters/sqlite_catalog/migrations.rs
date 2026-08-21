use rusqlite::{Connection, OptionalExtension, Transaction, TransactionBehavior, params};

use crate::domain::ScanError;

use super::{
    MAX_SCAN_CATCH_UP_LINEAGE, SCHEMA_VERSION, database_error, natural_name_key,
    parent_relative_path,
};

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
        let transaction = connection.transaction().map_err(database_error)?;
        create_schema_v19(&transaction)?;
        migrate_v19_to_v20_transaction(&transaction)?;
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
                validate_current_schema_contract(connection)?;
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
    validate_change_queue_authority(connection)?;
    validate_authoritative_recovery_marker(connection)?;
    validate_change_catch_up_contract(connection)?;
    validate_scan_handoff_batch_contract(connection)?;
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

fn validate_current_schema_contract(connection: &Connection) -> Result<(), ScanError> {
    validate_v19_schema_contract(connection)?;
    validate_metadata_inventory_contract(connection)
}

fn validate_metadata_inventory_contract(connection: &Connection) -> Result<(), ScanError> {
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
        &[
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
        ],
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
                 WHERE runs.staged_entry_count <> (
                   SELECT COUNT(*) FROM library_metadata_inventory_entries AS entries
                   WHERE entries.run_id = runs.id
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

fn validate_change_catch_up_contract(connection: &Connection) -> Result<(), ScanError> {
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
    let handoff_columns_match = table_columns_match(
        connection,
        "library_change_catch_up_handoffs",
        &[
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
        ],
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
                   'updated_unix_ms'
                 )) = 25
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

fn validate_scan_handoff_batch_contract(connection: &Connection) -> Result<(), ScanError> {
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
    Ok(actual.len() == expected.len()
        && actual.iter().zip(expected).all(|(actual, expected)| {
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
    value
        .chars()
        .filter(|character| !character.is_whitespace())
        .flat_map(char::to_lowercase)
        .collect()
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
    use tempfile::NamedTempFile;

    use super::{
        create_metadata_inventory_contract, create_schema_v19, migrate_schema, migrate_v16_to_v17,
        migrate_v17_to_v18, migrate_v18_to_v19, preview_repair_marker_is_complete,
        repair_missing_v19_preview_expectation_marker, repair_prerelease_v18_scan_owner_index,
    };

    fn fresh_v19_catalog() -> Connection {
        let mut connection = Connection::open_in_memory().expect("catalog");
        let transaction = connection.transaction().expect("v19 transaction");
        create_schema_v19(&transaction).expect("fresh v19 schema");
        transaction.commit().expect("commit fresh v19 schema");
        connection
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

        migrate_schema(&mut connection).expect("migrate v19 to v20");
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
        assert_eq!(version, 20);
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
