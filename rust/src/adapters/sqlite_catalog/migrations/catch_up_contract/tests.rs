use rusqlite::Connection;

use super::super::migrate_schema;

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
