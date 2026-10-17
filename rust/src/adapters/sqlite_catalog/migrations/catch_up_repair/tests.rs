use rusqlite::Connection;

use super::super::legacy_test_catalog::fresh_v19_catalog;
use super::super::migrate_schema;

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
