use rusqlite::{Connection, OptionalExtension, Transaction, TransactionBehavior, params};

use crate::domain::ScanError;

use super::{SCHEMA_VERSION, database_error, natural_name_key, parent_relative_path};

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
                repair_prerelease_v19_path_index(connection)?;
                return validate_current_schema_contract(connection);
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

fn validate_current_schema_contract(connection: &Connection) -> Result<(), ScanError> {
    validate_change_queue_authority(connection)?;
    validate_authoritative_recovery_marker(connection)?;
    validate_change_catch_up_contract(connection)?;
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
    Ok(())
}

fn validate_change_catch_up_contract(connection: &Connection) -> Result<(), ScanError> {
    let (has_table, has_marker, has_path_index) = connection
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
               )) = 'root_id,relative_path,scan_id,location_id'",
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
    if !has_table || !has_marker || !has_path_index {
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

fn repair_prerelease_v19_path_index(connection: &mut Connection) -> Result<(), ScanError> {
    let (has_table, has_marker, has_named_index, has_columns) = connection
        .query_row(
            "SELECT
               EXISTS(SELECT 1 FROM sqlite_master
                 WHERE type = 'table' AND name = 'library_change_catch_up_state'),
               EXISTS(SELECT 1 FROM pragma_table_info('library_change_queue_contract')
                 WHERE name = 'change_catch_up_complete'),
               EXISTS(SELECT 1 FROM sqlite_master
                 WHERE type = 'index' AND name = 'asset_locations_root_relative'),
               (SELECT COUNT(*) FROM pragma_table_info('asset_locations')
                 WHERE name IN ('root_id', 'relative_path', 'scan_id', 'location_id')) = 4",
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
    if has_named_index || !has_table || !has_marker || !has_columns {
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

    let transaction = connection.transaction().map_err(database_error)?;
    transaction
        .execute(
            "CREATE INDEX asset_locations_root_relative
             ON asset_locations(root_id, relative_path, scan_id, location_id)",
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
    transaction
        .execute("UPDATE schema_info SET version = 19", [])
        .map_err(database_error)?;
    transaction.commit().map_err(database_error)
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
               CHECK(change_catch_up_complete = 1);",
        )
        .map_err(database_error)
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

    use super::{migrate_schema, migrate_v16_to_v17, migrate_v17_to_v18, migrate_v18_to_v19};

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
        let mut connection = Connection::open_in_memory().expect("catalog");
        migrate_schema(&mut connection).expect("fresh v19 catalog");
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
                 CREATE TABLE scan_runs(
                   id TEXT PRIMARY KEY,
                   root_id TEXT NOT NULL,
                   status TEXT NOT NULL
                 );
                 CREATE TABLE asset_locations(
                   root_id TEXT NOT NULL,
                   relative_path TEXT NOT NULL,
                   scan_id TEXT NOT NULL,
                   location_id TEXT NOT NULL
                 );
                 INSERT INTO scan_runs VALUES ('foreground-a', 'root-a', 'running');
                 INSERT INTO scan_runs VALUES (
                   'sync-recovery-1-2-3', 'root-b', 'running'
                 );",
            )
            .expect("repairable prerelease v18 fixture");

        migrate_schema(&mut connection).expect("repair scan ownership index");
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
                 CREATE TABLE scan_runs(
                   id TEXT PRIMARY KEY,
                   root_id TEXT NOT NULL,
                   status TEXT NOT NULL
                 );
                 CREATE TABLE asset_locations(
                   root_id TEXT NOT NULL,
                   relative_path TEXT NOT NULL,
                   scan_id TEXT NOT NULL,
                   location_id TEXT NOT NULL
                 );
                 CREATE UNIQUE INDEX scan_runs_one_active_root
                   ON scan_runs(root_id) WHERE status IN ('running', 'paused');
                 INSERT INTO scan_runs VALUES (
                   'sync-recovery-7-8-9', 'root-a', 'running'
                 );",
            )
            .expect("indexed prerelease v18 fixture");

        migrate_schema(&mut connection).expect("repair missing scan owner");
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

        let (version, marker, checkpoint_count, has_path_index): (i64, bool, i64, bool) =
            connection
                .query_row(
                    "SELECT
                   (SELECT version FROM schema_info),
                   (SELECT change_catch_up_complete = 1
                    FROM library_change_queue_contract WHERE singleton = 1),
                   (SELECT COUNT(*) FROM library_change_catch_up_state),
                   EXISTS(SELECT 1 FROM sqlite_master
                     WHERE type = 'index' AND name = 'asset_locations_root_relative')",
                    [],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
                )
                .expect("v19 evidence");

        assert_eq!(version, 19);
        assert!(marker);
        assert_eq!(checkpoint_count, 0);
        assert!(has_path_index);
    }
}
