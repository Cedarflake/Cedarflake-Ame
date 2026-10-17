use rusqlite::Connection;

use super::super::migrate_schema;

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
