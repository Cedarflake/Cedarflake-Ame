use rusqlite::{Connection, Transaction, TransactionBehavior, params};

use crate::domain::{
    JournalFileReference, JournalIdentifier, JournalUsn, LibraryChangeIntent,
    PersistentJournalEnrollmentBatch, PersistentJournalPendingRename, PersistentJournalRangeState,
    PersistentJournalSourceRange, PersistentJournalVolumeIdentity, ScanError,
    persistent_journal_batch_id_from_payload, persistent_journal_batch_payload,
};

use super::{
    SCHEMA_VERSION, SQLITE_APPLICATION_ID, database_error, natural_name_key, parent_relative_path,
    unix_time_ms,
};

mod catch_up_contract;
mod catch_up_repair;
mod change_lane_contract;
mod compatibility_repair_shape;
mod current_recovery_state;
mod current_schema;
mod directory_identities;
mod exact_candidate_index_retirement;
mod exact_duplicate_review;
mod exact_duplicates;
mod interrupted_foreground_repair;
mod inventory_spool_contract;
mod live_gap_contract;
mod prerelease_journal_repair;
mod recovery_authority_contract;
mod recovery_execution_contract;
#[cfg(test)]
mod recovery_test_catalog;
mod retired_live_gap_repair;
mod root_namespace_contract;
mod scan_handoff_contract;
mod scan_owner_contract;
mod terminal_inventory_repair;
#[cfg(test)]
pub(crate) use exact_duplicates::downgrade_to_v39_for_test;
mod foreground_queue_index;
mod foreground_scan_capture;
mod gallery_duplicates;
mod inventory_spool_retirement;
pub(super) mod inventory_spool_rows;
pub(super) mod journal_contract;
mod journal_directory_namespace;
mod journal_scope_recovery;
#[cfg(test)]
mod legacy_test_catalog;
mod media_admission_rechecks;
mod metadata_fallback;
mod metadata_inventory_contract;
mod metadata_recheck_index;
mod organization;
mod organization_admission;
mod organization_association_index;
mod organization_index_retirement;
mod organization_native_recovery;
mod organization_reconciliation;
mod organization_restoration;
mod organization_review;
mod organization_transitions;
#[cfg(test)]
pub(super) use organization::downgrade_to_v46_for_test;
mod persistent_journal_baseline;
mod persistent_journal_range_history;
mod preview_expectation_repair;
mod queue_retirement_indexes;
mod retained_gallery_membership;
mod retained_source_state;
mod retired_root_authority;
mod scan_resume_inventory;
mod schema_shape;
mod source_revision_contract;
mod terminal_media_contract;

use persistent_journal_baseline::validate_persistent_journal_baseline_contract;

use journal_contract::schema::PERSISTENT_JOURNAL_CANONICAL_TRIGGER_DDL;

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
        inventory_spool_retirement::migrate_transaction(&transaction)?;
        scan_resume_inventory::migrate_transaction(&transaction)?;
        foreground_queue_index::migrate_transaction(&transaction)?;
        directory_identities::migrate_transaction(&transaction)?;
        journal_directory_namespace::migrate_transaction(&transaction)?;
        journal_scope_recovery::migrate_transaction(&transaction)?;
        media_admission_rechecks::migrate_transaction(&transaction)?;
        queue_retirement_indexes::migrate_transaction(&transaction)?;
        exact_duplicates::migrate_transaction(&transaction)?;
        gallery_duplicates::migrate_transaction(&transaction)?;
        exact_duplicate_review::migrate_transaction(&transaction)?;
        exact_candidate_index_retirement::migrate_transaction(&transaction)?;
        metadata_recheck_index::migrate_transaction(&transaction)?;
        retained_gallery_membership::migrate_transaction(&transaction)?;
        metadata_fallback::migrate_transaction(&transaction)?;
        organization::migrate_transaction(&transaction)?;
        organization_transitions::migrate_transaction(&transaction)?;
        organization_reconciliation::migrate_transaction(&transaction)?;
        organization_index_retirement::migrate_transaction(&transaction)?;
        organization_admission::migrate_transaction(&transaction)?;
        organization_review::migrate_transaction(&transaction)?;
        organization_native_recovery::migrate_transaction(&transaction)?;
        organization_restoration::migrate_transaction(&transaction)?;
        organization_association_index::migrate_transaction(&transaction)?;
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
                prerelease_journal_repair::repair_range_id_triggers(connection)?;
                prerelease_journal_repair::repair_recovery_window(connection)?;
                current_recovery_state::repair_and_validate(connection)?;
                source_revision_contract::repair_precontract_metadata(connection)?;
                interrupted_foreground_repair::recover(connection)?;
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
                scan_owner_contract::validate_queue_authority(connection)?;
                migrate_v17_to_v18(connection)?;
            }
            18 => {
                scan_owner_contract::repair(connection)?;
                migrate_v18_to_v19(connection)?;
            }
            19 => {
                catch_up_contract::validate_prerelease_authority(connection)?;
                catch_up_repair::repair_derived_indexes(connection)?;
                catch_up_repair::repair_scan_lineage(connection)?;
                catch_up_repair::repair_scan_handoff_batches(connection)?;
                preview_expectation_repair::repair(connection)?;
                current_schema::validate_v19_schema_contract(connection)?;
                migrate_v19_to_v20(connection)?;
            }
            20 => {
                metadata_inventory_contract::validate(connection)?;
                migrate_v20_to_v21(connection)?;
            }
            21 => {
                current_schema::validate_v19_schema_contract(connection)?;
                metadata_inventory_contract::validate(connection)?;
                terminal_media_contract::validate(connection)?;
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
            31 => inventory_spool_retirement::migrate(connection)?,
            32 => scan_resume_inventory::migrate(connection)?,
            33 => foreground_queue_index::migrate(connection)?,
            34 => directory_identities::migrate(connection)?,
            35 => journal_directory_namespace::migrate(connection)?,
            36 => journal_scope_recovery::migrate(connection)?,
            37 => media_admission_rechecks::migrate(connection)?,
            38 => queue_retirement_indexes::migrate(connection)?,
            39 => exact_duplicates::migrate(connection)?,
            40 => gallery_duplicates::migrate(connection)?,
            41 => exact_duplicate_review::migrate(connection)?,
            42 => exact_candidate_index_retirement::migrate(connection)?,
            43 => metadata_recheck_index::migrate(connection)?,
            44 => retained_gallery_membership::migrate(connection)?,
            45 => metadata_fallback::migrate(connection)?,
            46 => organization::migrate(connection)?,
            47 => organization_transitions::migrate(connection)?,
            48 => organization_reconciliation::migrate(connection)?,
            49 => organization_index_retirement::migrate(connection)?,
            50 => organization_admission::migrate(connection)?,
            51 => organization_review::migrate(connection)?,
            52 => organization_native_recovery::migrate(connection)?,
            53 => organization_restoration::migrate(connection)?,
            54 => organization_association_index::migrate(connection)?,
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
    preview_expectation_repair::add_marker(transaction)?;
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
    preview_expectation_repair::add_marker(&transaction)?;
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
            return Err(journal_contract::values::unverifiable_contract());
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
    journal_contract::validate(transaction)
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
    journal_contract::validate(transaction)?;
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
    change_lane_contract::validate(transaction)?;
    recovery_authority_contract::validate(transaction)?;
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
    terminal_inventory_repair::apply(transaction, 25)?;
    change_lane_contract::validate(transaction)?;
    recovery_authority_contract::validate(transaction)?;
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
    recovery_execution_contract::validate(transaction)
}

fn migrate_v26_to_v27(connection: &mut Connection) -> Result<(), ScanError> {
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(database_error)?;
    migrate_v26_to_v27_transaction(&transaction)?;
    transaction.commit().map_err(database_error)
}

fn migrate_v26_to_v27_transaction(transaction: &Transaction<'_>) -> Result<(), ScanError> {
    terminal_inventory_repair::apply(transaction, 26)?;
    recovery_execution_contract::validate(transaction)?;
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
    inventory_spool_contract::validate_version(transaction, 27, 1)
}

fn migrate_v27_to_v28(connection: &mut Connection) -> Result<(), ScanError> {
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(database_error)?;
    migrate_v27_to_v28_transaction(&transaction)?;
    transaction.commit().map_err(database_error)
}

fn migrate_v27_to_v28_transaction(transaction: &Transaction<'_>) -> Result<(), ScanError> {
    terminal_inventory_repair::apply(transaction, 27)?;
    inventory_spool_contract::validate_version(transaction, 27, 1)?;
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
    inventory_spool_contract::validate_version(transaction, 28, 2)
}

fn migrate_v28_to_v29(connection: &mut Connection) -> Result<(), ScanError> {
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(database_error)?;
    migrate_v28_to_v29_transaction(&transaction)?;
    transaction.commit().map_err(database_error)
}

fn migrate_v28_to_v29_transaction(transaction: &Transaction<'_>) -> Result<(), ScanError> {
    terminal_inventory_repair::apply(transaction, 28)?;
    inventory_spool_contract::validate_version(transaction, 28, 2)?;
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
            root_namespace_contract::validate_windows_root_identity(&scheme, &value)?;
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
                return Err(root_namespace_contract::unverifiable_contract());
            }
            let serial = volume_serial
                .parse::<u64>()
                .map_err(|_| root_namespace_contract::unverifiable_contract())?;
            if serial.to_string() != volume_serial {
                return Err(root_namespace_contract::unverifiable_contract());
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
    root_namespace_contract::validate(transaction)
}

fn migrate_v29_to_v30(connection: &mut Connection) -> Result<(), ScanError> {
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(database_error)?;
    migrate_v29_to_v30_transaction(&transaction)?;
    transaction.commit().map_err(database_error)
}

fn migrate_v29_to_v30_transaction(transaction: &Transaction<'_>) -> Result<(), ScanError> {
    terminal_inventory_repair::apply(transaction, 29)?;
    current_schema::validate_pre_live_gap_schema_contract(transaction, 29)?;
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
    live_gap_contract::validate(transaction)
}

fn migrate_v30_to_v31(connection: &mut Connection) -> Result<(), ScanError> {
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(database_error)?;
    migrate_v30_to_v31_transaction(&transaction)?;
    transaction.commit().map_err(database_error)
}

fn migrate_v30_to_v31_transaction(transaction: &Transaction<'_>) -> Result<(), ScanError> {
    terminal_inventory_repair::apply(transaction, 30)?;
    current_schema::validate_pre_live_gap_schema_contract(transaction, 30)?;
    retired_live_gap_repair::apply(transaction, 30)?;
    live_gap_contract::validate(transaction)?;
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
    source_revision_contract::invalidate_precontract_metadata(transaction)?;
    source_revision_contract::create_metadata_contract(transaction)?;
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
    current_schema::validate_schema_version(transaction, 31)
}

#[cfg(test)]
pub(super) fn downgrade_source_revision_contract_to_v30_for_test(connection: &Connection) {
    inventory_spool_retirement::downgrade_to_v31_for_test(connection);
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
        return Err(root_namespace_contract::unverifiable_contract());
    }
    root_namespace_contract::validate_windows_root_identity(&scheme, &value)?;
    match proofs.entry((root_id, generation)) {
        std::collections::btree_map::Entry::Vacant(entry) => {
            entry.insert((scheme, value, authority_kind.to_owned(), updated_unix_ms));
        }
        std::collections::btree_map::Entry::Occupied(mut entry) => {
            if entry.get().0 != scheme || entry.get().1 != value {
                return Err(root_namespace_contract::unverifiable_contract());
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
        root_generation: journal_contract::values::parse_root_generation(root_generation)?,
        volume: PersistentJournalVolumeIdentity {
            volume_guid,
            volume_serial: journal_contract::values::parse_canonical_u64(&volume_serial)?,
        },
        journal_id: JournalIdentifier::parse_canonical(&journal_id)
            .map_err(|_| journal_contract::values::unverifiable_contract())?,
        requested_start_usn: JournalUsn::parse_canonical(&requested_start_usn)
            .map_err(|_| journal_contract::values::unverifiable_contract())?,
        requested_end_usn: JournalUsn::parse_canonical(&requested_end_usn)
            .map_err(|_| journal_contract::values::unverifiable_contract())?,
        covered_until_usn: JournalUsn::parse_canonical(&covered_until_usn)
            .map_err(|_| journal_contract::values::unverifiable_contract())?,
        is_complete: is_complete != 0,
        protocol_version: u16::try_from(protocol_version)
            .map_err(|_| journal_contract::values::unverifiable_contract())?,
        contract_version: u16::try_from(contract_version)
            .map_err(|_| journal_contract::values::unverifiable_contract())?,
        state: PersistentJournalRangeState::Enrolled,
        enrolled_unix_ms,
        checkpointed_unix_ms: None,
    };
    source_range
        .validate()
        .map_err(|_| journal_contract::values::unverifiable_contract())?;
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
            root_generation: journal_contract::values::parse_root_generation(row.1)?,
            kind: journal_contract::values::parse_intent_kind(&row.2)?,
            scope: journal_contract::values::parse_intent_scope(&row.3)?,
            relative_path: row.4,
            previous_relative_path: row.5,
            origin: journal_contract::values::parse_intent_origin(&row.6)?,
            first_observed_unix_ms: row.7,
            most_recent_observed_unix_ms: row.8,
            first_sequence: journal_contract::values::parse_canonical_u64(&row.9)?,
            most_recent_sequence: journal_contract::values::parse_canonical_u64(&row.10)?,
            coalesced_observation_count: u32::try_from(row.11)
                .map_err(|_| journal_contract::values::unverifiable_contract())?,
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
            .map_err(|_| journal_contract::values::unverifiable_contract())?;
        if i64::from(file_reference.record_version()) != row.4 {
            return Err(journal_contract::values::unverifiable_contract());
        }
        pending_renames.push(PersistentJournalPendingRename {
            carry_id: row.0,
            source_range_id: "0".repeat(64),
            volume: PersistentJournalVolumeIdentity {
                volume_guid: row.1,
                volume_serial: journal_contract::values::parse_canonical_u64(&row.2)?,
            },
            journal_id: JournalIdentifier::parse_canonical(&row.3)
                .map_err(|_| journal_contract::values::unverifiable_contract())?,
            file_reference,
            old_usn: JournalUsn::parse_canonical(&row.6)
                .map_err(|_| journal_contract::values::unverifiable_contract())?,
            previous_root_id: row.7,
            previous_root_generation: journal_contract::values::parse_root_generation(row.8)?,
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
        directory_changes: Vec::new(),
    };
    batch
        .validate()
        .map_err(|_| journal_contract::values::unverifiable_contract())?;
    Ok(persistent_journal_batch_payload(&batch))
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

    use super::legacy_test_catalog::fresh_v19_catalog;
    use super::recovery_test_catalog::{
        active_frontier_catalog, recovery_lifecycle_catalog,
        remove_metadata_inventory_spool_v27_contract_for_test,
        remove_root_publication_namespace_v29_contract_for_test, reopen_recovery_lifecycle_catalog,
        v27_root_proof_phase_catalog,
    };
    use super::schema_shape::normalize_schema_sql;
    use super::{
        SCHEMA_VERSION, migrate_schema, migrate_v16_to_v17, migrate_v17_to_v18, migrate_v18_to_v19,
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
        super::current_schema::validate_pre_live_gap_schema_contract(connection, 29)
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
        super::inventory_spool_contract::validate_version(connection, 28, 2)
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

        super::source_revision_contract::reset_row_audit_count();
        super::current_schema::reset_current_schema_row_audit_count();
        super::migrate_v30_to_v31(&mut connection).expect("v31");
        assert_eq!(super::source_revision_contract::row_audit_count(), 1);
        assert!(super::current_schema::current_schema_row_audit_count() > 0);

        assert_eq!(
            super::schema_shape::schema_version(&connection).expect("schema version"),
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
            super::source_revision_contract::metadata_contract_is_complete(&connection)
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
        super::current_schema::validate_schema_version(&connection, 31)
            .expect("source revision contract");
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
        super::current_schema::validate_schema_version(&connection, 31)
            .expect("conflicting legacy identities no longer block startup");
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
            assert_eq!(schema, (SCHEMA_VERSION, 4, 2, 1), "phase {phase}");

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
