use crate::adapters::sqlite_catalog::tests::support::{
    ensure_legacy_asset_location_contract, ensure_legacy_scan_runs_contract, load_default_snapshot,
};
use std::collections::HashSet;

use tempfile::tempdir;

use crate::adapters::sqlite_catalog::*;

fn add_catalog_state_to_legacy_fixture(connection: &Connection) {
    ensure_legacy_asset_location_contract(connection);
    connection
        .execute_batch(
            "CREATE TABLE catalog_state (
               revision INTEGER NOT NULL CHECK(revision >= 0)
             );
             INSERT INTO catalog_state(revision)
               SELECT COUNT(*) FROM scan_runs WHERE status = 'completed';",
        )
        .expect("add canonical catalog state to legacy fixture");
}

#[test]
fn migrates_v1_catalog_without_losing_the_active_location() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let connection = Connection::open(&path).expect("v1 catalog");
    connection
        .execute_batch(
            "PRAGMA foreign_keys = ON;
                 CREATE TABLE schema_info (version INTEGER NOT NULL);
                 INSERT INTO schema_info(version) VALUES (1);
                 CREATE TABLE library_roots (
                   id TEXT PRIMARY KEY, path TEXT NOT NULL UNIQUE,
                   active_scan_id TEXT, created_unix_ms INTEGER NOT NULL
                 );
                 CREATE TABLE scan_runs (
                   id TEXT PRIMARY KEY, root_id TEXT NOT NULL, status TEXT NOT NULL,
                   started_unix_ms INTEGER NOT NULL, completed_unix_ms INTEGER,
                   asset_count INTEGER NOT NULL DEFAULT 0,
                   issue_count INTEGER NOT NULL DEFAULT 0
                 );
                 CREATE TABLE asset_locations (
                   scan_id TEXT NOT NULL, location_id TEXT NOT NULL, root_id TEXT NOT NULL,
                   absolute_path TEXT NOT NULL, relative_path TEXT NOT NULL,
                   preview_path TEXT NOT NULL, file_size INTEGER NOT NULL,
                   modified_unix_ms INTEGER NOT NULL, width INTEGER NOT NULL,
                   height INTEGER NOT NULL, PRIMARY KEY(scan_id, location_id)
                 );
                 CREATE TABLE scan_issues (
                   id INTEGER PRIMARY KEY AUTOINCREMENT, scan_id TEXT NOT NULL,
                   path TEXT, code TEXT NOT NULL, message TEXT NOT NULL
                 );
                 INSERT INTO library_roots VALUES ('root-1', 'C:\\Pictures', 'scan-1', 10);
                 INSERT INTO scan_runs VALUES ('scan-1', 'root-1', 'completed', 11, 12, 1, 0);
                 INSERT INTO asset_locations VALUES (
                   'scan-1', 'location-1', 'root-1', 'C:\\Pictures\\one.png',
                   'one.png', 'C:\\Cache\\one.jpg', 20, 30, 40, 50
                 );",
        )
        .expect("v1 schema");
    drop(connection);

    let mut catalog = SqliteCatalog::open(path).expect("migrated catalog");
    let version: i64 = catalog
        .connection
        .query_row("SELECT version FROM schema_info", [], |row| row.get(0))
        .expect("schema version");
    let snapshot = load_default_snapshot(&mut catalog, 10, None).expect("snapshot");

    assert_eq!(version, SCHEMA_VERSION);
    assert_eq!(snapshot.revision, 1);
    assert_eq!(snapshot.roots.len(), 1);
    assert_eq!(snapshot.assets.len(), 1);
    assert_eq!(snapshot.assets[0].asset_id, "legacy:scan-1:location-1");
}

#[test]
fn migrates_v2_catalog_revision_from_completed_scans() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let connection = Connection::open(&path).expect("v2 catalog");
    connection
        .execute_batch(
            "CREATE TABLE schema_info (version INTEGER NOT NULL);
                 INSERT INTO schema_info(version) VALUES (2);
                 CREATE TABLE scan_runs (
                   id TEXT PRIMARY KEY, root_id TEXT NOT NULL, status TEXT NOT NULL,
                   started_unix_ms INTEGER NOT NULL, completed_unix_ms INTEGER,
                   asset_count INTEGER NOT NULL DEFAULT 0,
                   issue_count INTEGER NOT NULL DEFAULT 0
                 );
                 CREATE TABLE library_roots (
                   id TEXT PRIMARY KEY, path TEXT NOT NULL UNIQUE,
                   active_scan_id TEXT, created_unix_ms INTEGER NOT NULL
                 );
                 CREATE TABLE scan_issues (
                   id INTEGER PRIMARY KEY AUTOINCREMENT, scan_id TEXT NOT NULL,
                   path TEXT, code TEXT NOT NULL, message TEXT NOT NULL
                 );
                 CREATE TABLE asset_locations (
                   scan_id TEXT NOT NULL, asset_id TEXT NOT NULL,
                   location_id TEXT NOT NULL, root_id TEXT NOT NULL,
                   absolute_path TEXT NOT NULL, relative_path TEXT NOT NULL,
                   preview_path TEXT NOT NULL, file_size INTEGER NOT NULL,
                   modified_unix_ms INTEGER NOT NULL, width INTEGER NOT NULL,
                   height INTEGER NOT NULL, PRIMARY KEY(scan_id, location_id)
                 );
                 INSERT INTO scan_runs VALUES
                   ('scan-1', 'root-1', 'completed', 1, 2, 1, 0),
                   ('scan-2', 'root-2', 'cancelled', 3, 4, 0, 0);",
        )
        .expect("v2 schema");
    ensure_legacy_asset_location_contract(&connection);
    drop(connection);

    let catalog = SqliteCatalog::open(path).expect("migrated catalog");
    let (version, revision): (i64, i64) = catalog
        .connection
        .query_row(
            "SELECT schema_info.version, catalog_state.revision
                 FROM schema_info CROSS JOIN catalog_state",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("schema state");

    assert_eq!(version, SCHEMA_VERSION);
    assert_eq!(revision, 1);
}

#[test]
fn migrates_v3_without_treating_an_uncheckpointed_scan_as_recoverable() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let connection = Connection::open(&path).expect("v3 catalog");
    connection
        .execute_batch(
            "CREATE TABLE schema_info (version INTEGER NOT NULL);
                 INSERT INTO schema_info(version) VALUES (3);
                 CREATE TABLE scan_runs (
                   id TEXT PRIMARY KEY, root_id TEXT NOT NULL, status TEXT NOT NULL,
                   started_unix_ms INTEGER NOT NULL, completed_unix_ms INTEGER,
                   asset_count INTEGER NOT NULL DEFAULT 0,
                   issue_count INTEGER NOT NULL DEFAULT 0
                 );
                 CREATE TABLE library_roots (
                   id TEXT PRIMARY KEY, path TEXT NOT NULL UNIQUE,
                   active_scan_id TEXT, created_unix_ms INTEGER NOT NULL
                 );
                 CREATE TABLE scan_issues (
                   id INTEGER PRIMARY KEY AUTOINCREMENT, scan_id TEXT NOT NULL,
                   path TEXT, code TEXT NOT NULL, message TEXT NOT NULL
                 );
                 CREATE TABLE asset_locations (
                   scan_id TEXT NOT NULL, asset_id TEXT NOT NULL,
                   location_id TEXT NOT NULL, root_id TEXT NOT NULL,
                   absolute_path TEXT NOT NULL, relative_path TEXT NOT NULL,
                   preview_path TEXT NOT NULL, file_size INTEGER NOT NULL,
                   modified_unix_ms INTEGER NOT NULL, width INTEGER NOT NULL,
                   height INTEGER NOT NULL, PRIMARY KEY(scan_id, location_id)
                 );
                 INSERT INTO scan_runs VALUES
                   ('scan-running', 'root-1', 'running', 11, NULL, 0, 0),
                   ('scan-complete', 'root-2', 'completed', 12, 13, 1, 0);
                 INSERT INTO library_roots VALUES
                   ('root-1', 'C:\\Pictures', NULL, 1),
                   ('root-2', 'C:\\Photos', 'scan-complete', 2);",
        )
        .expect("v3 schema");
    add_catalog_state_to_legacy_fixture(&connection);
    drop(connection);

    let catalog = SqliteCatalog::open(path).expect("migrated catalog");
    let (version, running_status): (i64, String) = catalog
        .connection
        .query_row(
            "SELECT schema_info.version, scan_runs.status
                 FROM schema_info JOIN scan_runs ON scan_runs.id = 'scan-running'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("migrated state");

    assert_eq!(version, SCHEMA_VERSION);
    assert_eq!(running_status, "interrupted_unrecoverable");
    assert!(
        catalog
            .load_recoverable_scan()
            .expect("recoverable scan query")
            .is_none()
    );
}

#[test]
fn migrates_v4_tasks_without_inventing_a_missing_directory_frontier() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let connection = Connection::open(&path).expect("v4 catalog");
    connection
        .execute_batch(
            "CREATE TABLE schema_info (version INTEGER NOT NULL);
                 INSERT INTO schema_info(version) VALUES (4);
                 CREATE TABLE library_roots (
                   id TEXT PRIMARY KEY, active_scan_id TEXT
                 );
                 CREATE TABLE scan_runs (
                   id TEXT PRIMARY KEY, root_id TEXT NOT NULL, status TEXT NOT NULL,
                   started_unix_ms INTEGER NOT NULL, completed_unix_ms INTEGER,
                   asset_count INTEGER NOT NULL DEFAULT 0,
                   issue_count INTEGER NOT NULL DEFAULT 0,
                   max_items INTEGER, max_entries INTEGER,
                   preview_edge INTEGER NOT NULL,
                   last_visited_relative_path TEXT,
                   visited_entries INTEGER NOT NULL DEFAULT 0,
                   accepted_items INTEGER NOT NULL DEFAULT 0
                 );
                 CREATE TABLE asset_locations (
                   scan_id TEXT NOT NULL, asset_id TEXT NOT NULL,
                   location_id TEXT NOT NULL, root_id TEXT NOT NULL,
                   absolute_path TEXT NOT NULL, relative_path TEXT NOT NULL,
                   preview_path TEXT NOT NULL, file_size INTEGER NOT NULL,
                   modified_unix_ms INTEGER NOT NULL, width INTEGER NOT NULL,
                   height INTEGER NOT NULL, PRIMARY KEY(scan_id, location_id)
                 );
                 INSERT INTO scan_runs VALUES
                   ('running', 'root-1', 'running', 1, NULL, 0, 0, 500, 2000, 512,
                    'old.png', 10, 2),
                   ('paused', 'root-2', 'paused', 2, NULL, 0, 0, 500, 2000, 512,
                    'old.png', 20, 3),
                   ('complete', 'root-3', 'completed', 3, 4, 1, 0, 500, 2000, 512,
                    NULL, 30, 4);",
        )
        .expect("v4 schema");
    add_catalog_state_to_legacy_fixture(&connection);
    drop(connection);

    let catalog = SqliteCatalog::open(path).expect("migrated catalog");
    let version: i64 = catalog
        .connection
        .query_row("SELECT version FROM schema_info", [], |row| row.get(0))
        .expect("schema version");
    let statuses = catalog
        .connection
        .prepare("SELECT id, status FROM scan_runs ORDER BY id")
        .expect("status statement")
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .expect("status rows")
        .collect::<rusqlite::Result<Vec<_>>>()
        .expect("stored statuses");

    assert_eq!(version, SCHEMA_VERSION);
    assert_eq!(
        statuses,
        vec![
            ("complete".to_owned(), "completed".to_owned()),
            ("paused".to_owned(), "interrupted_unrecoverable".to_owned(),),
            ("running".to_owned(), "interrupted_unrecoverable".to_owned(),),
        ]
    );
}

#[test]
fn migrates_v5_tasks_without_inventing_a_missing_entry_snapshot() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let connection = Connection::open(&path).expect("v5 catalog");
    connection
        .execute_batch(
            "CREATE TABLE schema_info (version INTEGER NOT NULL);
                 INSERT INTO schema_info(version) VALUES (5);
                 CREATE TABLE library_roots (
                   id TEXT PRIMARY KEY, active_scan_id TEXT
                 );
                 CREATE TABLE scan_runs (
                   id TEXT PRIMARY KEY, root_id TEXT NOT NULL, status TEXT NOT NULL,
                   started_unix_ms INTEGER NOT NULL, completed_unix_ms INTEGER,
                   asset_count INTEGER NOT NULL DEFAULT 0,
                   issue_count INTEGER NOT NULL DEFAULT 0,
                   max_items INTEGER, max_entries INTEGER,
                   preview_edge INTEGER NOT NULL,
                   current_directory_relative_path TEXT,
                   last_visited_relative_path TEXT,
                   visited_entries INTEGER NOT NULL DEFAULT 0,
                   accepted_items INTEGER NOT NULL DEFAULT 0
                 );
                 CREATE TABLE scan_directory_frontier (
                   id INTEGER PRIMARY KEY AUTOINCREMENT,
                   scan_id TEXT NOT NULL,
                   relative_path TEXT NOT NULL,
                   UNIQUE(scan_id, relative_path)
                 );
                 CREATE TABLE asset_locations (
                   scan_id TEXT NOT NULL, asset_id TEXT NOT NULL,
                   location_id TEXT NOT NULL, root_id TEXT NOT NULL,
                   absolute_path TEXT NOT NULL, relative_path TEXT NOT NULL,
                   preview_path TEXT NOT NULL, file_size INTEGER NOT NULL,
                   modified_unix_ms INTEGER NOT NULL, width INTEGER NOT NULL,
                   height INTEGER NOT NULL, PRIMARY KEY(scan_id, location_id)
                 );
                 INSERT INTO scan_runs VALUES
                   ('running', 'root-1', 'running', 1, NULL, 0, 0, NULL, NULL,
                    512, 'wide', 'wide\\255.png', 256, 10),
                   ('paused', 'root-2', 'paused', 2, NULL, 0, 0, NULL, NULL,
                    512, 'other', NULL, 0, 0),
                   ('complete', 'root-3', 'completed', 3, 4, 1, 0, NULL, NULL,
                    512, NULL, NULL, 1, 1);
                 INSERT INTO scan_directory_frontier(scan_id, relative_path) VALUES
                   ('running', 'pending'), ('paused', 'pending');",
        )
        .expect("v5 schema");
    add_catalog_state_to_legacy_fixture(&connection);
    drop(connection);

    let catalog = SqliteCatalog::open(path).expect("migrated catalog");
    let version: i64 = catalog
        .connection
        .query_row("SELECT version FROM schema_info", [], |row| row.get(0))
        .expect("schema version");
    let statuses = catalog
        .connection
        .prepare("SELECT id, status FROM scan_runs ORDER BY id")
        .expect("status statement")
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .expect("status rows")
        .collect::<rusqlite::Result<Vec<_>>>()
        .expect("stored statuses");
    let frontier_count: i64 = catalog
        .connection
        .query_row("SELECT COUNT(*) FROM scan_directory_frontier", [], |row| {
            row.get(0)
        })
        .expect("frontier count");

    assert_eq!(version, SCHEMA_VERSION);
    assert_eq!(
        statuses,
        vec![
            ("complete".to_owned(), "completed".to_owned()),
            ("paused".to_owned(), "interrupted_unrecoverable".to_owned()),
            ("running".to_owned(), "interrupted_unrecoverable".to_owned()),
        ]
    );
    assert_eq!(frontier_count, 0);
}

#[test]
fn migrates_v9_with_bounded_reconciliation_indexes() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let connection = Connection::open(&path).expect("v9 catalog");
    connection
        .execute_batch(
            "CREATE TABLE schema_info (version INTEGER NOT NULL);
                 INSERT INTO schema_info(version) VALUES (9);
                 CREATE TABLE library_roots (
                   id TEXT PRIMARY KEY, active_scan_id TEXT
                 );
                 CREATE TABLE asset_locations (
                   scan_id TEXT NOT NULL,
                   asset_id TEXT NOT NULL,
                   location_id TEXT NOT NULL,
                   root_id TEXT NOT NULL,
                   relative_path TEXT NOT NULL,
                   modified_unix_ms INTEGER NOT NULL,
                   capture_local_time TEXT,
                   file_identity_scheme TEXT,
                   file_identity_value TEXT,
                   preview_path TEXT NOT NULL DEFAULT '',
                   preview_status TEXT NOT NULL DEFAULT 'pending',
                   PRIMARY KEY(scan_id, location_id)
                 );
                 INSERT INTO asset_locations VALUES (
                   'scan-1', 'asset-1', 'location-1', 'root-1',
                   'archive/one.png', 1, NULL,
                   'windows-file-id-128-v1',
                   '0000000000000001:00000000000000000000000000000002',
                   '', 'pending'
                 );",
        )
        .expect("v9 schema");
    ensure_legacy_scan_runs_contract(&connection);
    drop(connection);

    let catalog = SqliteCatalog::open(path).expect("migrated catalog");
    let (version, asset_id, identity_value): (i64, String, String) = catalog
        .connection
        .query_row(
            "SELECT schema_info.version, asset_locations.asset_id,
                        asset_locations.file_identity_value
                 FROM schema_info CROSS JOIN asset_locations",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("migrated row");
    let index_names = catalog
        .connection
        .prepare(
            "SELECT name FROM sqlite_master
                 WHERE type = 'index' AND tbl_name = 'asset_locations'",
        )
        .expect("index query")
        .query_map([], |row| row.get::<_, String>(0))
        .expect("index rows")
        .collect::<Result<HashSet<_>, _>>()
        .expect("index names");

    assert_eq!(version, SCHEMA_VERSION);
    assert_eq!(asset_id, "asset-1");
    assert_eq!(
        identity_value,
        "0000000000000001:00000000000000000000000000000002"
    );
    assert!(index_names.contains("asset_locations_active_file_identity"));
    assert!(index_names.contains("asset_locations_location_id"));
    assert!(index_names.contains("asset_locations_asset_id"));
    assert!(index_names.contains("asset_locations_gallery_time"));
    assert!(index_names.contains("asset_locations_gallery_created"));
    assert!(index_names.contains("asset_locations_gallery_modified"));
    assert!(index_names.contains("asset_locations_gallery_name"));
    assert!(index_names.contains("asset_locations_parent_folder"));
}

#[test]
fn migrates_v10_by_adding_the_gallery_time_index_without_rewriting_rows() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let connection = Connection::open(&path).expect("v10 catalog");
    connection
        .execute_batch(
            "CREATE TABLE schema_info (version INTEGER NOT NULL);
                 INSERT INTO schema_info(version) VALUES (10);
                 CREATE TABLE library_roots (
                   id TEXT PRIMARY KEY, active_scan_id TEXT
                 );
                 CREATE TABLE asset_locations (
                   scan_id TEXT NOT NULL,
                   location_id TEXT NOT NULL,
                   root_id TEXT NOT NULL,
                   relative_path TEXT NOT NULL,
                   modified_unix_ms INTEGER NOT NULL,
                   capture_local_time TEXT,
                   preview_path TEXT NOT NULL DEFAULT '',
                   preview_status TEXT NOT NULL DEFAULT 'pending',
                   PRIMARY KEY(scan_id, location_id)
                 );
                 INSERT INTO asset_locations VALUES (
                   'scan-1', 'location-1', 'root-1', 'Album/img10.png', 123,
                   '2025-08-07T10:20:30.000000000', '', 'pending'
                 );",
        )
        .expect("v10 schema");
    ensure_legacy_scan_runs_contract(&connection);
    drop(connection);

    let catalog = SqliteCatalog::open(path).expect("migrated catalog");
    let (version, capture_time, parent_path, name_key): (i64, Option<String>, String, String) =
        catalog
            .connection
            .query_row(
                "SELECT schema_info.version, asset_locations.capture_local_time,
                        asset_locations.parent_relative_path,
                        asset_locations.natural_name_key
                 FROM schema_info CROSS JOIN asset_locations",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .expect("migrated gallery row");
    let gallery_index: i64 = catalog
        .connection
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master
                 WHERE type = 'index' AND name = 'asset_locations_gallery_time'",
            [],
            |row| row.get(0),
        )
        .expect("gallery index");

    assert_eq!(version, SCHEMA_VERSION);
    assert!(capture_time.is_none());
    assert_eq!(parent_path, "Album");
    assert_eq!(name_key, natural_name_key("Album/img10.png"));
    assert_eq!(gallery_index, 1);
}

#[test]
fn migrates_v12_by_materializing_the_file_time_fallback_key() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let connection = Connection::open(&path).expect("v12 catalog");
    connection
        .execute_batch(
            "CREATE TABLE schema_info (version INTEGER NOT NULL);
             INSERT INTO schema_info(version) VALUES (12);
             CREATE TABLE library_roots (
               id TEXT PRIMARY KEY, active_scan_id TEXT
             );
             CREATE TABLE asset_locations (
               scan_id TEXT NOT NULL,
               location_id TEXT NOT NULL,
               root_id TEXT NOT NULL,
               relative_path TEXT NOT NULL,
               created_unix_ms INTEGER,
               modified_unix_ms INTEGER NOT NULL,
               capture_local_time TEXT,
               preview_path TEXT NOT NULL DEFAULT '',
               preview_status TEXT NOT NULL DEFAULT 'pending',
               PRIMARY KEY(scan_id, location_id)
             );
             INSERT INTO asset_locations VALUES (
               'scan-1', 'location-1', 'root-1', 'Album/photo.png',
               1749988800000, 1784116800000, NULL, '', 'pending'
             );
             CREATE INDEX asset_locations_gallery_time
               ON asset_locations(
                 (capture_local_time IS NULL), IFNULL(capture_local_time, '') DESC,
                 modified_unix_ms DESC, root_id, location_id, scan_id
               );
             CREATE INDEX asset_locations_gallery_created
               ON asset_locations(
                 (created_unix_ms IS NULL), IFNULL(created_unix_ms, 0),
                 root_id, location_id, scan_id
               );",
        )
        .expect("v12 schema");
    ensure_legacy_scan_runs_contract(&connection);
    drop(connection);

    let catalog = SqliteCatalog::open(path).expect("migrated catalog");
    let (version, file_local_time, capture_local_time): (i64, String, Option<String>) = catalog
        .connection
        .query_row(
            "SELECT schema_info.version, asset_locations.file_local_time,
                    asset_locations.capture_local_time
             FROM schema_info CROSS JOIN asset_locations",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("migrated fallback row");

    assert_eq!(version, SCHEMA_VERSION);
    assert!(file_local_time.starts_with("2025-06-"));
    assert!(capture_local_time.is_none());
}
