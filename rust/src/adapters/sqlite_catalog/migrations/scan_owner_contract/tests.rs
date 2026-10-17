use rusqlite::Connection;

use super::super::migrate_schema;

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

    super::repair(&mut connection).expect("repair scan ownership index");
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

    super::repair(&mut connection).expect("repair missing scan owner");
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
