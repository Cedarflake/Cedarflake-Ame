use crate::adapters::sqlite_catalog::tests::support::{
    ensure_legacy_scan_runs_contract, preview_lifecycle_state, preview_reference_count,
    publish_fixture, publish_preview_artifact,
};

use tempfile::tempdir;

use crate::adapters::sqlite_catalog::migrations::downgrade_source_revision_contract_to_v30_for_test;
use crate::adapters::sqlite_catalog::*;

#[test]
fn migrates_v20_terminal_media_evidence_without_rebuilding_the_catalog() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let mut catalog = SqliteCatalog::open(path.clone()).expect("catalog");
    publish_fixture(
        &mut catalog,
        "v20-evidence-scan",
        "v20-evidence-root",
        "C:\\V20EvidenceSource",
        "v20-evidence-location",
    );
    downgrade_source_revision_contract_to_v30_for_test(&catalog.connection);
    remove_persistent_journal_v22_contract_for_test(&catalog.connection);
    catalog
        .connection
        .execute_batch(
            "DROP TABLE library_terminal_media_evidence;
             DROP TABLE library_terminal_media_evidence_contract;
             UPDATE schema_info SET version = 20;",
        )
        .expect("restore v20 schema");
    drop(catalog);

    let reopened = SqliteCatalog::open(path).expect("migrate v20 catalog");
    let version: i64 = reopened
        .connection
        .query_row("SELECT version FROM schema_info", [], |row| row.get(0))
        .expect("schema version");
    let marker: i64 = reopened
        .connection
        .query_row(
            "SELECT complete FROM library_terminal_media_evidence_contract
             WHERE singleton = 1",
            [],
            |row| row.get(0),
        )
        .expect("terminal evidence contract marker");

    assert_eq!(version, SCHEMA_VERSION);
    assert_eq!(marker, 1);
    assert!(
        reopened
            .load_active_location("v20-evidence-location")
            .expect("preserved active location")
            .is_some()
    );
}

#[test]
fn prerelease_missing_active_preview_is_downgraded_on_reopen() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let mut catalog = SqliteCatalog::open(path.clone()).expect("catalog");
    publish_fixture(
        &mut catalog,
        "prerelease-missing-preview-scan",
        "prerelease-missing-preview-root",
        "C:\\PrereleaseMissingPreview",
        "prerelease-missing-preview-location",
    );
    let handoff_count: i64 = catalog
        .connection
        .query_row(
            "SELECT
               (SELECT COUNT(*) FROM library_change_catch_up_handoffs)
               + (SELECT COUNT(*) FROM library_change_scan_handoff_items)",
            [],
            |row| row.get(0),
        )
        .expect("terminal handoff count");
    assert_eq!(handoff_count, 0);
    downgrade_source_revision_contract_to_v30_for_test(&catalog.connection);
    remove_persistent_journal_v22_contract_for_test(&catalog.connection);
    catalog
        .connection
        .execute_batch(
            "DROP TABLE library_change_preview_repair_contract;
             DROP TABLE library_metadata_inventory_entries;
             DROP TABLE library_metadata_inventory_runs;
             DROP TABLE library_metadata_inventory_contract;
             DROP TABLE library_terminal_media_evidence;
             DROP TABLE library_terminal_media_evidence_contract;
             UPDATE schema_info SET version = 19;",
        )
        .expect("restore prerelease preview repair marker");
    drop(catalog);

    let reopened = SqliteCatalog::open(path).expect("repair missing active preview");
    let repaired = reopened
        .load_active_location("prerelease-missing-preview-location")
        .expect("repaired active location query")
        .expect("repaired active location");

    assert!(repaired.preview_path.is_empty());
    assert!(matches!(repaired.preview_status, PreviewStatus::Pending));
    assert!(repaired.preview_issue_code.is_none());
    assert!(repaired.preview_issue_message.is_none());
}

#[test]
fn prerelease_stale_active_preview_and_owner_are_downgraded_on_reopen() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let mut catalog = SqliteCatalog::open(path.clone()).expect("catalog");
    publish_fixture(
        &mut catalog,
        "prerelease-stale-preview-scan",
        "prerelease-stale-preview-root",
        "C:\\PrereleaseStalePreview",
        "prerelease-stale-preview-location",
    );
    let artifact = publish_preview_artifact(
        &mut catalog,
        "prerelease-stale-preview-location",
        "prerelease-stale-preview-artifact",
        "C:\\AmeCache\\prerelease-stale-preview.jpg",
    );
    catalog
        .connection
        .execute(
            "UPDATE preview_artifacts SET lifecycle_state = 'stale'
             WHERE artifact_key = ?1",
            [&artifact.artifact_key],
        )
        .expect("restore prerelease stale artifact");
    downgrade_source_revision_contract_to_v30_for_test(&catalog.connection);
    remove_persistent_journal_v22_contract_for_test(&catalog.connection);
    catalog
        .connection
        .execute_batch(
            "DROP TABLE library_change_preview_repair_contract;
             DROP TABLE library_metadata_inventory_entries;
             DROP TABLE library_metadata_inventory_runs;
             DROP TABLE library_metadata_inventory_contract;
             DROP TABLE library_terminal_media_evidence;
             DROP TABLE library_terminal_media_evidence_contract;
             UPDATE schema_info SET version = 19;",
        )
        .expect("restore prerelease preview repair marker");
    assert_eq!(preview_reference_count(&catalog, &artifact.artifact_key), 1);
    drop(catalog);

    let reopened = SqliteCatalog::open(path).expect("repair stale active preview");
    let repaired = reopened
        .load_active_location("prerelease-stale-preview-location")
        .expect("repaired active location query")
        .expect("repaired active location");

    assert!(repaired.preview_path.is_empty());
    assert!(matches!(repaired.preview_status, PreviewStatus::Pending));
    assert_eq!(
        preview_reference_count(&reopened, &artifact.artifact_key),
        0
    );
    assert_eq!(
        preview_lifecycle_state(&reopened, &artifact.artifact_key),
        "evictable"
    );
}

#[test]
fn migrates_v6_previews_as_pending_when_source_revision_is_unknown() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let connection = Connection::open(&path).expect("v6 catalog");
    connection
        .execute_batch(
            "CREATE TABLE schema_info (version INTEGER NOT NULL);
                 INSERT INTO schema_info(version) VALUES (6);
                 CREATE TABLE library_roots (
                   id TEXT PRIMARY KEY, active_scan_id TEXT
                 );
                 CREATE TABLE asset_locations (
                   scan_id TEXT NOT NULL, asset_id TEXT NOT NULL,
                   location_id TEXT NOT NULL, root_id TEXT NOT NULL,
                   absolute_path TEXT NOT NULL, relative_path TEXT NOT NULL,
                   preview_path TEXT NOT NULL, file_size INTEGER NOT NULL,
                   modified_unix_ms INTEGER NOT NULL, width INTEGER NOT NULL,
                   height INTEGER NOT NULL, PRIMARY KEY(scan_id, location_id)
                 );
                 INSERT INTO asset_locations VALUES (
                   'scan-1', 'asset-1', 'location-1', 'root-1',
                   'C:\\Pictures\\one.png', 'one.png', 'C:\\Cache\\one.jpg',
                   20, 30, 40, 50
                 );",
        )
        .expect("v6 schema");
    ensure_legacy_scan_runs_contract(&connection);
    drop(connection);

    let catalog = SqliteCatalog::open(path).expect("migrated catalog");
    let (version, preview_path, preview_status, engine_id): (i64, String, String, String) = catalog
        .connection
        .query_row(
            "SELECT schema_info.version, asset_locations.preview_path,
                        asset_locations.preview_status, asset_locations.metadata_engine_id
                 FROM schema_info CROSS JOIN asset_locations",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .expect("migrated preview");

    assert_eq!(version, SCHEMA_VERSION);
    assert!(preview_path.is_empty());
    assert_eq!(preview_status, "pending");
    assert_eq!(engine_id, "ame-invalidated-media-metadata");
}

#[test]
fn migrates_v7_locations_as_unanalyzed_metadata() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let connection = Connection::open(&path).expect("v7 catalog");
    connection
        .execute_batch(
            "CREATE TABLE schema_info (version INTEGER NOT NULL);
                 INSERT INTO schema_info(version) VALUES (7);
                 CREATE TABLE library_roots (
                   id TEXT PRIMARY KEY, active_scan_id TEXT
                 );
                 CREATE TABLE asset_locations (
                   scan_id TEXT NOT NULL, asset_id TEXT NOT NULL,
                   location_id TEXT NOT NULL, root_id TEXT NOT NULL,
                   absolute_path TEXT NOT NULL, relative_path TEXT NOT NULL,
                   preview_path TEXT NOT NULL, file_size INTEGER NOT NULL,
                   modified_unix_ms INTEGER NOT NULL, width INTEGER NOT NULL,
                   height INTEGER NOT NULL,
                   preview_status TEXT NOT NULL,
                   preview_issue_code TEXT,
                   preview_issue_message TEXT,
                   PRIMARY KEY(scan_id, location_id)
                 );
                 INSERT INTO asset_locations VALUES (
                   'scan-1', 'asset-1', 'location-1', 'root-1',
                   'C:\\Pictures\\one.png', 'one.png', 'C:\\Cache\\one.jpg',
                   20, 30, 40, 50, 'ready', NULL, NULL
                 );",
        )
        .expect("v7 schema");
    ensure_legacy_scan_runs_contract(&connection);
    drop(connection);

    let catalog = SqliteCatalog::open(path).expect("migrated catalog");
    let (version, engine_id, engine_version, capture_time): (i64, String, String, Option<String>) =
        catalog
            .connection
            .query_row(
                "SELECT schema_info.version, asset_locations.metadata_engine_id,
                        asset_locations.metadata_engine_version,
                        asset_locations.capture_local_time
                 FROM schema_info CROSS JOIN asset_locations",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .expect("migrated metadata state");

    assert_eq!(version, SCHEMA_VERSION);
    assert_eq!(engine_id, "ame-invalidated-media-metadata");
    assert_eq!(engine_version, "0");
    assert!(capture_time.is_none());
}

#[test]
fn migrates_v8_locations_with_unknown_file_identity() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let connection = Connection::open(&path).expect("v8 catalog");
    connection
        .execute_batch(
            "CREATE TABLE schema_info (version INTEGER NOT NULL);
                 INSERT INTO schema_info(version) VALUES (8);
                 CREATE TABLE library_roots (
                   id TEXT PRIMARY KEY, active_scan_id TEXT
                 );
                 CREATE TABLE asset_locations (
                   scan_id TEXT NOT NULL, asset_id TEXT NOT NULL,
                   location_id TEXT NOT NULL, root_id TEXT NOT NULL,
                   absolute_path TEXT NOT NULL, relative_path TEXT NOT NULL,
                   preview_path TEXT NOT NULL, file_size INTEGER NOT NULL,
                   modified_unix_ms INTEGER NOT NULL, width INTEGER NOT NULL,
                   height INTEGER NOT NULL, preview_status TEXT NOT NULL,
                   preview_issue_code TEXT, preview_issue_message TEXT,
                   metadata_engine_id TEXT NOT NULL,
                   metadata_engine_version TEXT NOT NULL,
                   capture_local_time TEXT, capture_offset_minutes INTEGER,
                   capture_time_source TEXT, capture_raw_value TEXT,
                   PRIMARY KEY(scan_id, location_id)
                 );
                 INSERT INTO asset_locations VALUES (
                   'scan-1', 'asset-1', 'location-1', 'root-1',
                   'C:\\Pictures\\one.png', 'one.png', 'C:\\Cache\\one.jpg',
                   20, 30, 40, 50, 'ready', NULL, NULL,
                   'kamadak-exif', '0.6.1', NULL, NULL, NULL, NULL
                 );",
        )
        .expect("v8 schema");
    ensure_legacy_scan_runs_contract(&connection);
    drop(connection);

    let catalog = SqliteCatalog::open(path).expect("migrated catalog");
    let (version, scheme, value): (i64, Option<String>, Option<String>) = catalog
        .connection
        .query_row(
            "SELECT schema_info.version, asset_locations.file_identity_scheme,
                        asset_locations.file_identity_value
                 FROM schema_info CROSS JOIN asset_locations",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("migrated file identity");

    assert_eq!(version, SCHEMA_VERSION);
    assert!(scheme.is_none());
    assert!(value.is_none());
}

#[test]
fn migrates_v13_with_an_empty_preview_artifact_index() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let connection = Connection::open(&path).expect("v13 catalog");
    connection
        .execute_batch(
            "CREATE TABLE schema_info (version INTEGER NOT NULL);
             INSERT INTO schema_info(version) VALUES (13);
             CREATE TABLE library_roots (
               id TEXT PRIMARY KEY, active_scan_id TEXT
             );
             CREATE TABLE asset_locations (
               scan_id TEXT NOT NULL, asset_id TEXT NOT NULL,
               location_id TEXT NOT NULL, root_id TEXT NOT NULL,
               relative_path TEXT NOT NULL,
               preview_path TEXT NOT NULL,
               preview_status TEXT NOT NULL DEFAULT 'pending',
               PRIMARY KEY(scan_id, location_id)
             );",
        )
        .expect("v13 schema");
    ensure_legacy_scan_runs_contract(&connection);
    drop(connection);

    let catalog = SqliteCatalog::open(path).expect("migrated catalog");
    let version: i64 = catalog
        .connection
        .query_row("SELECT version FROM schema_info", [], |row| row.get(0))
        .expect("schema version");
    let artifact_count: i64 = catalog
        .connection
        .query_row("SELECT COUNT(*) FROM preview_artifacts", [], |row| {
            row.get(0)
        })
        .expect("preview artifact count");
    let index_count: i64 = catalog
        .connection
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master
             WHERE type = 'index' AND name LIKE 'preview_artifacts_%'",
            [],
            |row| row.get(0),
        )
        .expect("preview artifact indexes");

    assert_eq!(version, SCHEMA_VERSION);
    assert_eq!(artifact_count, 0);
    let ownership_index_count: i64 = catalog
        .connection
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master
             WHERE type = 'index' AND name = 'preview_artifact_locations_location'",
            [],
            |row| row.get(0),
        )
        .expect("preview ownership index");

    assert_eq!(index_count, 2);
    assert_eq!(ownership_index_count, 1);
}

#[test]
fn migrates_v14_preview_ownership_to_every_active_location() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let connection = Connection::open(&path).expect("v14 catalog");
    connection
        .execute_batch(
            "CREATE TABLE schema_info (version INTEGER NOT NULL);
             INSERT INTO schema_info(version) VALUES (14);
             CREATE TABLE library_roots (
               id TEXT PRIMARY KEY, active_scan_id TEXT
             );
             INSERT INTO library_roots VALUES
               ('root-1', 'scan-1'), ('root-2', 'scan-2');
             CREATE TABLE asset_locations (
               scan_id TEXT NOT NULL, asset_id TEXT NOT NULL,
               location_id TEXT NOT NULL, root_id TEXT NOT NULL,
               relative_path TEXT NOT NULL,
               preview_path TEXT NOT NULL,
               preview_status TEXT NOT NULL,
               PRIMARY KEY(scan_id, location_id)
             );
             INSERT INTO asset_locations VALUES
               ('scan-1', 'asset-1', 'location-1', 'root-1', 'shared.jpg',
                'C:\\Cache\\shared.jpg', 'ready'),
               ('scan-2', 'asset-2', 'location-2', 'root-2', 'shared.jpg',
                'C:\\Cache\\shared.jpg', 'ready');
             CREATE TABLE preview_artifacts (
               artifact_key TEXT PRIMARY KEY,
               location_id TEXT NOT NULL,
               source_file_size INTEGER NOT NULL,
               source_modified_unix_ms INTEGER NOT NULL,
               source_identity_scheme TEXT,
               source_identity_value TEXT,
               algorithm_id TEXT NOT NULL,
               algorithm_version INTEGER NOT NULL,
               orientation_contract TEXT NOT NULL,
               size_bucket INTEGER NOT NULL,
               encoded_width INTEGER NOT NULL,
               encoded_height INTEGER NOT NULL,
               artifact_path TEXT NOT NULL UNIQUE,
               byte_size INTEGER NOT NULL,
               lifecycle_state TEXT NOT NULL,
               created_unix_ms INTEGER NOT NULL,
               last_used_unix_ms INTEGER NOT NULL
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
             INSERT INTO preview_artifacts VALUES (
               'shared-artifact', 'location-2', 20, 30, NULL, NULL,
               'ame-jpeg-thumbnail', 2, 'exif-display-v1', 256, 40, 50,
               'C:\\Cache\\shared.jpg', 1024, 'ready', 40, 50
             );",
        )
        .expect("v14 schema");
    ensure_legacy_scan_runs_contract(&connection);
    drop(connection);

    let catalog = SqliteCatalog::open(path).expect("migrated catalog");
    let version: i64 = catalog
        .connection
        .query_row("SELECT version FROM schema_info", [], |row| row.get(0))
        .expect("schema version");
    let locations = catalog
        .connection
        .prepare(
            "SELECT location_id FROM preview_artifact_locations
             WHERE artifact_key = 'shared-artifact' ORDER BY location_id",
        )
        .expect("ownership query")
        .query_map([], |row| row.get::<_, String>(0))
        .expect("ownership rows")
        .collect::<Result<Vec<_>, _>>()
        .expect("ownership collection");

    assert_eq!(version, SCHEMA_VERSION);
    assert!(locations.is_empty());
    assert_eq!(
        preview_lifecycle_state(&catalog, "shared-artifact"),
        "evictable"
    );
}

#[test]
fn migrates_v15_by_reconciling_preview_ownership_with_active_locations() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let connection = Connection::open(&path).expect("v15 catalog");
    connection
        .execute_batch(
            "CREATE TABLE schema_info (version INTEGER NOT NULL);
             INSERT INTO schema_info(version) VALUES (15);
             CREATE TABLE library_roots (
               id TEXT PRIMARY KEY, active_scan_id TEXT
             );
             INSERT INTO library_roots VALUES
               ('root-1', 'scan-1'),
               ('root-2', 'scan-2'),
               ('root-retired', 'scan-current');
             CREATE TABLE asset_locations (
               scan_id TEXT NOT NULL,
               location_id TEXT NOT NULL,
               root_id TEXT NOT NULL,
               relative_path TEXT NOT NULL,
               preview_path TEXT NOT NULL,
               preview_status TEXT NOT NULL,
               PRIMARY KEY(scan_id, location_id)
             );
             INSERT INTO asset_locations VALUES
               ('scan-1', 'location-1', 'root-1', 'shared.jpg',
                'C:\\Cache\\shared.jpg', 'ready'),
               ('scan-2', 'location-2', 'root-2', 'shared.jpg',
                'C:\\Cache\\shared.jpg', 'ready'),
               ('scan-retired', 'location-retired', 'root-retired', 'retired.jpg',
                'C:\\Cache\\retired.jpg', 'ready');
             CREATE TABLE preview_artifacts (
               artifact_key TEXT PRIMARY KEY,
               source_file_size INTEGER NOT NULL,
               source_modified_unix_ms INTEGER NOT NULL,
               source_identity_scheme TEXT,
               source_identity_value TEXT,
               algorithm_id TEXT NOT NULL,
               algorithm_version INTEGER NOT NULL,
               orientation_contract TEXT NOT NULL,
               size_bucket INTEGER NOT NULL,
               encoded_width INTEGER NOT NULL,
               encoded_height INTEGER NOT NULL,
               artifact_path TEXT NOT NULL UNIQUE,
               byte_size INTEGER NOT NULL,
               lifecycle_state TEXT NOT NULL,
               created_unix_ms INTEGER NOT NULL,
               last_used_unix_ms INTEGER NOT NULL
             );
             INSERT INTO preview_artifacts VALUES
               ('shared-artifact', 20, 30, NULL, NULL,
                'ame-jpeg-thumbnail', 2, 'exif-display-v1', 256, 40, 50,
                'C:\\Cache\\shared.jpg', 1024, 'ready', 40, 50),
               ('retired-artifact', 20, 30, NULL, NULL,
                'ame-jpeg-thumbnail', 2, 'exif-display-v1', 256, 40, 50,
                'C:\\Cache\\retired.jpg', 1024, 'ready', 40, 50),
               ('wrong-path-artifact', 20, 30, NULL, NULL,
                'ame-jpeg-thumbnail', 2, 'exif-display-v1', 256, 40, 50,
                'C:\\Cache\\wrong.jpg', 1024, 'ready', 40, 50);
             CREATE TABLE preview_artifact_locations (
               artifact_key TEXT NOT NULL,
               location_id TEXT NOT NULL,
               PRIMARY KEY(artifact_key, location_id),
               FOREIGN KEY(artifact_key) REFERENCES preview_artifacts(artifact_key)
                 ON DELETE CASCADE
             );
             CREATE INDEX preview_artifact_locations_location
               ON preview_artifact_locations(location_id, artifact_key);
             INSERT INTO preview_artifact_locations VALUES
               ('shared-artifact', 'location-1'),
               ('shared-artifact', 'location-2'),
               ('retired-artifact', 'location-retired'),
               ('wrong-path-artifact', 'location-1');",
        )
        .expect("v15 schema");
    ensure_legacy_scan_runs_contract(&connection);
    drop(connection);

    let catalog = SqliteCatalog::open(path).expect("migrated catalog");
    let version: i64 = catalog
        .connection
        .query_row("SELECT version FROM schema_info", [], |row| row.get(0))
        .expect("schema version");
    let shared_locations = catalog
        .connection
        .prepare(
            "SELECT location_id FROM preview_artifact_locations
             WHERE artifact_key = 'shared-artifact' ORDER BY location_id",
        )
        .expect("shared ownership query")
        .query_map([], |row| row.get::<_, String>(0))
        .expect("shared ownership rows")
        .collect::<Result<Vec<_>, _>>()
        .expect("shared ownership collection");

    assert_eq!(version, SCHEMA_VERSION);
    assert!(shared_locations.is_empty());
    assert_eq!(preview_reference_count(&catalog, "retired-artifact"), 0);
    assert_eq!(
        preview_lifecycle_state(&catalog, "retired-artifact"),
        "evictable"
    );
    assert_eq!(preview_reference_count(&catalog, "wrong-path-artifact"), 0);
    assert_eq!(
        preview_lifecycle_state(&catalog, "wrong-path-artifact"),
        "evictable"
    );
    assert_eq!(
        preview_lifecycle_state(&catalog, "shared-artifact"),
        "evictable"
    );
}
