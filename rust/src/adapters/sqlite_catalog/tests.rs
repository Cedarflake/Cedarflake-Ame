use std::collections::HashSet;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use tempfile::tempdir;

use crate::domain::GallerySortDirection;

use super::*;

const TEST_QUERY_ID: &str = "test-default-query";
type GalleryQueryFixture<'a> = (&'a str, &'a str, Option<&'a str>, Option<i64>, i64);

fn load_default_snapshot(
    catalog: &mut SqliteCatalog,
    max_items: u32,
    after: Option<&CatalogCursor>,
) -> Result<CatalogSnapshot, ScanError> {
    catalog.load_snapshot(
        max_items,
        &GalleryQuery::default(),
        TEST_QUERY_ID,
        after,
        None,
        None,
    )
}

fn load_default_timeline(catalog: &mut SqliteCatalog) -> Result<GalleryTimeline, ScanError> {
    catalog.load_gallery_timeline(&GalleryQuery::default(), TEST_QUERY_ID)
}

fn load_default_layout_manifest_chunk(
    catalog: &mut SqliteCatalog,
    max_items: u32,
    after: Option<&GalleryLayoutManifestCursor>,
) -> Result<GalleryLayoutManifestChunk, ScanError> {
    catalog.load_gallery_layout_manifest_chunk(
        max_items,
        &GalleryQuery::default(),
        TEST_QUERY_ID,
        after,
    )
}

#[test]
fn catalog_reads_open_without_waiting_for_an_active_writer() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let holder = SqliteCatalog::open(path.clone()).expect("lock holder catalog");

    holder
        .connection
        .execute_batch("BEGIN IMMEDIATE")
        .expect("hold catalog writer lock");
    let reader = thread::spawn(move || {
        let started = Instant::now();
        let mut catalog = SqliteCatalog::open(path)?;
        let snapshot = load_default_snapshot(&mut catalog, 1, None)?;
        Ok::<_, ScanError>((started.elapsed(), snapshot))
    });
    let (elapsed, snapshot) = reader
        .join()
        .expect("catalog reader thread")
        .expect("open and read catalog while writer is active");
    holder
        .connection
        .execute_batch("ROLLBACK")
        .expect("release catalog writer lock");
    assert!(snapshot.roots.is_empty());
    assert!(elapsed < Duration::from_secs(1));
}

#[test]
fn preview_artifact_index_rolls_back_when_active_location_is_stale() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let mut catalog = SqliteCatalog::open(path).expect("catalog");
    publish_fixture(
        &mut catalog,
        "preview-index-scan",
        "preview-index-root",
        "C:\\PreviewIndexSource",
        "preview-index-location",
    );
    let mut location = catalog
        .load_active_location("preview-index-location")
        .expect("active location query")
        .expect("active location");
    location.modified_unix_ms += 1;
    location.preview_path = "C:\\AmeCache\\preview.jpg".to_owned();
    location.preview_status = PreviewStatus::Ready;
    let artifact = PreviewArtifact {
        artifact_key: "preview-artifact-key".to_owned(),
        algorithm_id: "ame-jpeg-thumbnail".to_owned(),
        algorithm_version: 2,
        orientation_contract: "exif-display-v1".to_owned(),
        size_bucket: 256,
        path: location.preview_path.clone(),
        byte_size: 1_024,
        encoded_width: 256,
        encoded_height: 192,
        width: location.width,
        height: location.height,
    };

    let error = catalog
        .update_active_preview(&location, Some(&artifact))
        .expect_err("stale preview publication");
    let artifact_count: i64 = catalog
        .connection
        .query_row("SELECT COUNT(*) FROM preview_artifacts", [], |row| {
            row.get(0)
        })
        .expect("artifact count");

    assert_eq!(error.code, "active_preview_location_stale");
    assert_eq!(artifact_count, 0);
}

#[test]
fn preview_publication_rejects_same_timestamp_file_identity_replacement() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let mut catalog = SqliteCatalog::open(path).expect("catalog");
    publish_fixture(
        &mut catalog,
        "preview-original-scan",
        "preview-identity-root",
        "C:\\PreviewIdentitySource",
        "preview-identity-location",
    );
    let mut original = catalog
        .load_active_location("preview-identity-location")
        .expect("original location query")
        .expect("original location");
    let replacement_scan = fixture_request("preview-replacement-scan", "C:\\PreviewIdentitySource");
    let mut replacement = original.clone();
    replacement.absolute_path = "C:\\PreviewIdentitySource\\replacement.png".to_owned();
    replacement.display_path = replacement.absolute_path.clone();
    replacement.relative_path = "replacement.png".to_owned();
    replacement.file_identity = Some(FileIdentityEvidence {
        scheme: "windows-file-id-v1".to_owned(),
        value: "volume:replacement".to_owned(),
    });
    replacement.preview_path.clear();
    replacement.preview_status = PreviewStatus::Pending;
    catalog
        .begin_scan(
            &replacement_scan,
            "preview-identity-root",
            &replacement_scan.root_path,
        )
        .expect("begin replacement scan");
    catalog
        .stage_location(
            &replacement_scan.scan_id,
            "preview-identity-root",
            &replacement,
        )
        .expect("stage replacement");
    catalog
        .publish_scan(&replacement_scan.scan_id, "preview-identity-root", 1, 0)
        .expect("publish replacement scan");
    original.preview_path = "C:\\AmeCache\\stale-preview.jpg".to_owned();
    original.preview_status = PreviewStatus::Ready;
    let artifact = PreviewArtifact {
        artifact_key: "stale-preview-artifact".to_owned(),
        algorithm_id: "ame-jpeg-thumbnail".to_owned(),
        algorithm_version: 2,
        orientation_contract: "exif-display-v1".to_owned(),
        size_bucket: 256,
        path: original.preview_path.clone(),
        byte_size: 1_024,
        encoded_width: 256,
        encoded_height: 192,
        width: original.width,
        height: original.height,
    };

    let error = catalog
        .update_active_preview(&original, Some(&artifact))
        .expect_err("stale identity publication");
    let artifact_count: i64 = catalog
        .connection
        .query_row("SELECT COUNT(*) FROM preview_artifacts", [], |row| {
            row.get(0)
        })
        .expect("artifact count");

    assert_eq!(error.code, "active_preview_location_stale");
    assert_eq!(artifact_count, 0);
    let active = catalog
        .load_active_location("preview-identity-location")
        .expect("active replacement query")
        .expect("active replacement");
    assert_eq!(active.absolute_path, replacement.absolute_path);
    assert!(matches!(active.preview_status, PreviewStatus::Pending));
}

#[test]
fn preview_usage_touches_are_coarsened_to_page_publication_intervals() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let mut catalog = SqliteCatalog::open(path).expect("catalog");
    publish_fixture(
        &mut catalog,
        "preview-touch-scan",
        "preview-touch-root",
        "C:\\PreviewTouchSource",
        "preview-touch-location",
    );
    let mut location = catalog
        .load_active_location("preview-touch-location")
        .expect("active location query")
        .expect("active location");
    location.preview_path = "C:\\AmeCache\\preview-touch.jpg".to_owned();
    let artifact = PreviewArtifact {
        artifact_key: "preview-touch-artifact".to_owned(),
        algorithm_id: "ame-jpeg-thumbnail".to_owned(),
        algorithm_version: 2,
        orientation_contract: "exif-display-v1".to_owned(),
        size_bucket: 256,
        path: location.preview_path.clone(),
        byte_size: 1_024,
        encoded_width: 256,
        encoded_height: 192,
        width: location.width,
        height: location.height,
    };
    catalog
        .update_active_preview(&location, Some(&artifact))
        .expect("publish preview artifact");
    catalog
        .connection
        .execute(
            "UPDATE preview_artifacts SET last_used_unix_ms = 0 WHERE artifact_key = ?1",
            [&artifact.artifact_key],
        )
        .expect("age preview artifact");
    let visible = vec![(location.location_id, location.preview_path)];

    assert_eq!(
        catalog
            .touch_preview_artifacts(&visible)
            .expect("first usage touch"),
        1
    );
    assert_eq!(
        catalog
            .touch_preview_artifacts(&visible)
            .expect("coarsened usage touch"),
        0
    );
}

#[test]
fn preview_root_activation_resets_only_artifacts_outside_the_new_root() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let mut catalog = SqliteCatalog::open(path).expect("catalog");
    for (suffix, preview_root) in [
        ("old", "C:\\OldPreviewRoot"),
        ("current", "D:\\CurrentPreviewRoot"),
    ] {
        let scan_id = format!("root-switch-{suffix}-scan");
        let root_id = format!("root-switch-{suffix}-root");
        let location_id = format!("root-switch-{suffix}-location");
        publish_fixture(
            &mut catalog,
            &scan_id,
            &root_id,
            &format!("C:\\RootSwitchSource\\{suffix}"),
            &location_id,
        );
        let mut location = catalog
            .load_active_location(&location_id)
            .expect("active location query")
            .expect("active location");
        location.preview_path = format!("{preview_root}\\{suffix}.jpg");
        location.preview_status = PreviewStatus::Ready;
        let artifact = PreviewArtifact {
            artifact_key: format!("root-switch-{suffix}"),
            algorithm_id: "ame-jpeg-thumbnail".to_owned(),
            algorithm_version: 2,
            orientation_contract: "exif-display-v1".to_owned(),
            size_bucket: 256,
            path: location.preview_path.clone(),
            byte_size: 1_024,
            encoded_width: 256,
            encoded_height: 192,
            width: location.width,
            height: location.height,
        };
        catalog
            .update_active_preview(&location, Some(&artifact))
            .expect("publish preview artifact");
    }

    let reset = catalog
        .reset_previews_outside_root("D:\\CurrentPreviewRoot\\")
        .expect("reset old preview root");

    assert_eq!(reset, 1);
    let old = catalog
        .load_active_location("root-switch-old-location")
        .expect("old location query")
        .expect("old location");
    let current = catalog
        .load_active_location("root-switch-current-location")
        .expect("current location query")
        .expect("current location");
    assert!(matches!(old.preview_status, PreviewStatus::Pending));
    assert!(old.preview_path.is_empty());
    assert_eq!((old.width, old.height), (40, 50));
    assert!(matches!(current.preview_status, PreviewStatus::Ready));
    assert_eq!(current.preview_path, "D:\\CurrentPreviewRoot\\current.jpg");
    let artifact_count: i64 = catalog
        .connection
        .query_row("SELECT COUNT(*) FROM preview_artifacts", [], |row| {
            row.get(0)
        })
        .expect("artifact count");
    assert_eq!(artifact_count, 1);
    assert!(
        catalog
            .is_preview_artifact_path_indexed(
                "C:\\EquivalentPreviewAlias\\current.jpg",
                Some("root-switch-current"),
            )
            .expect("artifact-key ownership lookup")
    );
}

#[test]
fn preview_reclamation_orders_stale_before_lru_and_preserves_protected_locations() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let mut catalog = SqliteCatalog::open(path).expect("catalog");
    for (suffix, last_used) in [("stale", 30_i64), ("lru", 10_i64), ("protected", 1_i64)] {
        let scan_id = format!("reclaim-{suffix}-scan");
        let root_id = format!("reclaim-{suffix}-root");
        let location_id = format!("reclaim-{suffix}-location");
        publish_fixture(
            &mut catalog,
            &scan_id,
            &root_id,
            &format!("C:\\ReclaimSource\\{suffix}"),
            &location_id,
        );
        let mut location = catalog
            .load_active_location(&location_id)
            .expect("active location query")
            .expect("active location");
        location.preview_path = format!("C:\\AmeCache\\{suffix}.jpg");
        let artifact = PreviewArtifact {
            artifact_key: format!("artifact-{suffix}"),
            algorithm_id: "ame-jpeg-thumbnail".to_owned(),
            algorithm_version: 2,
            orientation_contract: "exif-display-v1".to_owned(),
            size_bucket: 256,
            path: location.preview_path.clone(),
            byte_size: 1_024,
            encoded_width: 40,
            encoded_height: 50,
            width: location.width,
            height: location.height,
        };
        catalog
            .update_active_preview(&location, Some(&artifact))
            .expect("publish preview artifact");
        catalog
            .connection
            .execute(
                "UPDATE preview_artifacts
                 SET lifecycle_state = ?2, last_used_unix_ms = ?3
                 WHERE artifact_key = ?1",
                params![
                    artifact.artifact_key,
                    if suffix == "stale" { "stale" } else { "ready" },
                    last_used,
                ],
            )
            .expect("set reclamation evidence");
    }

    let protected = vec!["reclaim-protected-location".to_owned()];
    let candidates = catalog
        .load_preview_reclamation_candidates(
            &protected,
            "ame-jpeg-thumbnail",
            2,
            "exif-display-v1",
            "c:\\amecache\\",
            8,
        )
        .expect("reclamation candidates");

    assert_eq!(
        candidates
            .iter()
            .map(|candidate| candidate.location_id.as_str())
            .collect::<Vec<_>>(),
        ["reclaim-stale-location", "reclaim-lru-location"],
    );
    assert!(
        catalog
            .remove_reclaimed_preview(&candidates[0])
            .expect("remove reclaimed preview")
    );
    let reclaimed = catalog
        .load_active_location("reclaim-stale-location")
        .expect("reclaimed location query")
        .expect("reclaimed location");
    assert!(matches!(reclaimed.preview_status, PreviewStatus::Pending));
    assert!(reclaimed.preview_path.is_empty());
    assert_eq!((reclaimed.width, reclaimed.height), (40, 50));
}

#[test]
fn catalog_writers_wait_for_short_writer_contention() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let holder = SqliteCatalog::open(path.clone()).expect("lock holder catalog");
    let contender = SqliteCatalog::open(path).expect("contending catalog");
    let configured_timeout: i64 = contender
        .connection
        .query_row("PRAGMA busy_timeout", [], |row| row.get(0))
        .expect("configured busy timeout");
    assert_eq!(configured_timeout, 5_000);
    holder
        .connection
        .execute_batch("BEGIN IMMEDIATE")
        .expect("hold catalog writer lock");
    let attempt_started = Instant::now();
    let (ready_sender, ready_receiver) = mpsc::sync_channel(0);
    let writer = thread::spawn(move || {
        ready_sender.send(()).expect("announce writer attempt");
        contender
            .connection
            .execute("UPDATE catalog_state SET revision = revision", [])
    });
    ready_receiver.recv().expect("writer attempt ready");
    thread::sleep(Duration::from_millis(100));
    assert!(
        !writer.is_finished(),
        "contending writer must wait for the lock"
    );
    holder
        .connection
        .execute_batch("COMMIT")
        .expect("release catalog writer lock");
    assert_eq!(writer.join().expect("contending writer thread"), Ok(1));
    assert!(attempt_started.elapsed() >= Duration::from_millis(100));
}

#[test]
fn catalog_writer_timeout_has_a_specific_error_code() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let holder = SqliteCatalog::open(path.clone()).expect("lock holder catalog");
    let contender = SqliteCatalog::open(path).expect("contending catalog");
    contender
        .connection
        .busy_timeout(Duration::from_millis(20))
        .expect("short test timeout");
    holder
        .connection
        .execute_batch("BEGIN IMMEDIATE")
        .expect("hold catalog writer lock");

    let error = contender
        .connection
        .execute("UPDATE catalog_state SET revision = revision", [])
        .expect_err("writer must time out while the lock is held");
    let error = database_error(error);

    holder
        .connection
        .execute_batch("ROLLBACK")
        .expect("release catalog writer lock");
    assert_eq!(error.code, "catalog_database_busy");
    assert!(
        error
            .message
            .starts_with("The catalog database remained busy after waiting")
    );
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
fn migrates_v6_previews_as_ready_without_losing_the_artifact_path() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let connection = Connection::open(&path).expect("v6 catalog");
    connection
        .execute_batch(
            "CREATE TABLE schema_info (version INTEGER NOT NULL);
                 INSERT INTO schema_info(version) VALUES (6);
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
    assert_eq!(preview_path, "C:\\Cache\\one.jpg");
    assert_eq!(preview_status, "ready");
    assert_eq!(engine_id, "unknown");
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
    assert_eq!(engine_id, "unknown");
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
fn migrates_v9_with_bounded_reconciliation_indexes() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let connection = Connection::open(&path).expect("v9 catalog");
    connection
        .execute_batch(
            "CREATE TABLE schema_info (version INTEGER NOT NULL);
                 INSERT INTO schema_info(version) VALUES (9);
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
                   PRIMARY KEY(scan_id, location_id)
                 );
                 INSERT INTO asset_locations VALUES (
                   'scan-1', 'asset-1', 'location-1', 'root-1',
                   'archive/one.png', 1, NULL,
                   'windows-file-id-128-v1',
                   '0000000000000001:00000000000000000000000000000002'
                 );",
        )
        .expect("v9 schema");
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
                 CREATE TABLE asset_locations (
                   scan_id TEXT NOT NULL,
                   location_id TEXT NOT NULL,
                   root_id TEXT NOT NULL,
                   relative_path TEXT NOT NULL,
                   modified_unix_ms INTEGER NOT NULL,
                   capture_local_time TEXT,
                   PRIMARY KEY(scan_id, location_id)
                 );
                 INSERT INTO asset_locations VALUES (
                   'scan-1', 'location-1', 'root-1', 'Album/img10.png', 123,
                   '2025-08-07T10:20:30.000000000'
                 );",
        )
        .expect("v10 schema");
    drop(connection);

    let catalog = SqliteCatalog::open(path).expect("migrated catalog");
    let (version, capture_time, parent_path, name_key): (i64, String, String, String) = catalog
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
    assert_eq!(capture_time, "2025-08-07T10:20:30.000000000");
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
             CREATE TABLE asset_locations (
               scan_id TEXT NOT NULL,
               location_id TEXT NOT NULL,
               root_id TEXT NOT NULL,
               created_unix_ms INTEGER,
               modified_unix_ms INTEGER NOT NULL,
               capture_local_time TEXT,
               PRIMARY KEY(scan_id, location_id)
             );
             INSERT INTO asset_locations VALUES (
               'scan-1', 'location-1', 'root-1',
               1749988800000, 1784116800000, NULL
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

#[test]
fn migrates_v13_with_an_empty_preview_artifact_index() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let connection = Connection::open(&path).expect("v13 catalog");
    connection
        .execute_batch(
            "CREATE TABLE schema_info (version INTEGER NOT NULL);
             INSERT INTO schema_info(version) VALUES (13);",
        )
        .expect("v13 schema");
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
    assert_eq!(index_count, 3);
}

#[test]
fn gallery_time_query_uses_its_ordering_index() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let catalog = SqliteCatalog::open(path).expect("catalog");
    let details = catalog
        .connection
        .prepare(
            "EXPLAIN QUERY PLAN
                 SELECT location_id FROM asset_locations
                 ORDER BY (COALESCE(capture_local_time, file_local_time) IS NULL),
                          IFNULL(COALESCE(capture_local_time, file_local_time), '') DESC,
                          modified_unix_ms DESC, root_id, location_id
                 LIMIT 100",
        )
        .expect("query plan")
        .query_map([], |row| row.get::<_, String>(3))
        .expect("query-plan rows")
        .collect::<Result<Vec<_>, _>>()
        .expect("query-plan details");

    assert!(
        details
            .iter()
            .any(|detail| detail.contains("asset_locations_gallery_time")),
        "unexpected gallery-time query plan: {details:?}"
    );
}

#[test]
fn orphan_cleanup_plan_uses_the_asset_identity_index() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let catalog = SqliteCatalog::open(path).expect("catalog");
    let details = catalog
        .connection
        .prepare(
            "EXPLAIN QUERY PLAN
                 SELECT 1 FROM assets
                 WHERE NOT EXISTS (
                   SELECT 1 FROM asset_locations
                   WHERE asset_locations.asset_id = assets.id
                 )",
        )
        .expect("query plan")
        .query_map([], |row| row.get::<_, String>(3))
        .expect("query-plan rows")
        .collect::<Result<Vec<_>, _>>()
        .expect("query-plan details");

    assert!(
        details
            .iter()
            .any(|detail| detail.contains("asset_locations_asset_id")),
        "unexpected orphan-cleanup query plan: {details:?}"
    );
}

#[test]
fn capture_time_evidence_round_trips_with_engine_identity() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let mut catalog = SqliteCatalog::open(path).expect("catalog");
    let request = fixture_request("scan-capture", "C:\\Pictures");
    catalog
        .begin_scan(&request, "root-capture", "C:\\Pictures")
        .expect("begin scan");
    catalog
        .stage_location(
            "scan-capture",
            "root-capture",
            &AssetLocationView {
                asset_id: "asset-capture".to_owned(),
                location_id: "location-capture".to_owned(),
                root_id: "root-capture".to_owned(),
                absolute_path: "C:\\Pictures\\capture.jpg".to_owned(),
                display_path: "C:\\Pictures\\capture.jpg".to_owned(),
                relative_path: "capture.jpg".to_owned(),
                preview_path: String::new(),
                file_size: 20,
                created_unix_ms: Some(25),
                modified_unix_ms: 30,
                file_identity: Some(FileIdentityEvidence {
                    scheme: "windows-file-id-128-v1".to_owned(),
                    value: "0000000000000001:00000000000000000000000000000002".to_owned(),
                }),
                width: 40,
                height: 50,
                preview_status: PreviewStatus::Pending,
                preview_issue_code: None,
                preview_issue_message: None,
                metadata_engine_id: "kamadak-exif".to_owned(),
                metadata_engine_version: "0.6.1".to_owned(),
                capture_time: Some(CaptureTimeEvidence {
                    local_time: "2025-07-08T09:10:11.123000000".to_owned(),
                    offset_minutes: Some(480),
                    source: CaptureTimeSource::Original,
                    raw_value: "2025:07:08 09:10:11|123|+08:00".to_owned(),
                }),
            },
        )
        .expect("stage location");
    catalog
        .publish_scan("scan-capture", "root-capture", 1, 0)
        .expect("publish scan");

    let snapshot = load_default_snapshot(&mut catalog, 10, None).expect("snapshot");
    let asset = snapshot.assets.first().expect("stored asset");
    let capture = asset.capture_time.as_ref().expect("capture evidence");

    assert_eq!(asset.metadata_engine_id, "kamadak-exif");
    assert_eq!(asset.metadata_engine_version, "0.6.1");
    assert_eq!(
        asset
            .file_identity
            .as_ref()
            .map(|identity| identity.scheme.as_str()),
        Some("windows-file-id-128-v1")
    );
    assert_eq!(capture.local_time, "2025-07-08T09:10:11.123000000");
    assert_eq!(capture.offset_minutes, Some(480));
    assert!(matches!(capture.source, CaptureTimeSource::Original));
}

#[test]
fn directory_entry_frontier_is_idempotent_sorted_and_windowed() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let request = fixture_request("wide-scan", "C:\\Pictures");
    let mut catalog = SqliteCatalog::open(path).expect("catalog");
    catalog
        .begin_scan(&request, "wide-root", "C:\\Pictures")
        .expect("begin scan");
    assert_eq!(
        catalog
            .claim_next_directory("wide-scan")
            .expect("claim root"),
        Some(String::new())
    );
    assert!(
        !catalog
            .is_current_directory_enumerated("wide-scan", "")
            .expect("enumeration state")
    );

    let mut entries = (0..1025)
        .rev()
        .map(|index| format!("image-{index:04}.png"))
        .collect::<Vec<_>>();
    catalog
        .stage_directory_entries("wide-scan", "", &entries[..17])
        .expect("partial enumeration");
    entries.push("image-0000.png".to_owned());
    for batch in entries.chunks(113) {
        catalog
            .stage_directory_entries("wide-scan", "", batch)
            .expect("entry batch");
    }
    catalog
        .complete_directory_enumeration("wide-scan", "")
        .expect("complete enumeration");

    let mut loaded = Vec::new();
    loop {
        let window = catalog
            .load_directory_entry_window("wide-scan", "", loaded.last().map(String::as_str), 128)
            .expect("entry window");
        assert!(window.len() <= 128);
        if window.is_empty() {
            break;
        }
        loaded.extend(window);
    }

    assert_eq!(loaded.len(), 1025);
    assert!(loaded.windows(2).all(|pair| pair[0] < pair[1]));
    assert!(
        catalog
            .has_directory_entry("wide-scan", "", "image-0512.png")
            .expect("entry identity")
    );
    catalog
        .complete_directory("wide-scan", &ScanCheckpoint::default())
        .expect("complete directory");
    let queued: i64 = catalog
        .connection
        .query_row("SELECT COUNT(*) FROM scan_directory_entries", [], |row| {
            row.get(0)
        })
        .expect("entry count");
    assert_eq!(queued, 0);
}

#[test]
fn persists_and_validates_a_recoverable_scan_checkpoint() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let raw_root_path = r"\\?\C:\Pictures";
    let request = fixture_request("scan-resume", raw_root_path);
    let mut catalog = SqliteCatalog::open(path.clone()).expect("catalog");
    let initial = catalog
        .begin_scan(&request, "root-resume", raw_root_path)
        .expect("begin scan");
    assert_eq!(initial.visited_entries, 0);
    let checkpoint = ScanCheckpoint {
        last_visited_relative_path: Some("nested\\last.png".to_owned()),
        visited_entries: 128,
        accepted_items: 40,
        issue_count: 3,
    };
    catalog
        .checkpoint_scan("scan-resume", &checkpoint)
        .expect("persist checkpoint");
    drop(catalog);

    let mut restored = SqliteCatalog::open(path).expect("restored catalog");
    let recoverable = restored
        .load_recoverable_scan()
        .expect("recoverable scan")
        .expect("stored running scan");
    assert_eq!(recoverable.scan_id, "scan-resume");
    assert_eq!(recoverable.root_path, raw_root_path);
    assert_eq!(recoverable.display_root_path, "C:\\Pictures");
    assert_eq!(recoverable.visited_entries, 128);
    assert_eq!(recoverable.accepted_items, 40);
    assert_eq!(recoverable.issue_count, 3);
    let resumed = restored
        .begin_scan(&request, "root-resume", raw_root_path)
        .expect("resume scan");
    assert_eq!(
        resumed.last_visited_relative_path,
        checkpoint.last_visited_relative_path
    );

    let mut changed_request = request.clone();
    changed_request.preview_edge = 256;
    let error = restored
        .begin_scan(&changed_request, "root-resume", "C:\\Pictures")
        .expect_err("changed parameters cannot reuse a checkpoint");
    assert_eq!(error.code, "catalog_scan_resume_mismatch");
}

#[test]
fn keyset_pages_retain_multiple_roots_and_reject_stale_cursors() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let mut catalog = SqliteCatalog::open(path).expect("catalog");

    publish_fixture(&mut catalog, "scan-1", "root-1", "C:\\One", "location-1");
    publish_fixture(&mut catalog, "scan-2", "root-2", "C:\\Two", "location-2");

    let first_asset_id = catalog
        .load_active_location("location-1")
        .expect("existing location")
        .expect("active location")
        .asset_id;
    let first_page = load_default_snapshot(&mut catalog, 1, None).expect("first page");
    let cursor = first_page.next_cursor.clone().expect("next cursor");
    let second_page = load_default_snapshot(&mut catalog, 1, Some(&cursor)).expect("second page");

    assert_eq!(first_asset_id, "asset-scan-1");
    assert_eq!(first_page.revision, 2);
    assert_eq!(first_page.roots.len(), 2);
    assert_eq!(first_page.assets.len(), 1);
    assert_eq!(second_page.assets.len(), 1);
    assert!(second_page.next_cursor.is_none());
    assert_ne!(
        first_page.assets[0].location_id,
        second_page.assets[0].location_id,
    );

    publish_fixture(&mut catalog, "scan-3", "root-3", "C:\\Three", "location-3");
    let error = load_default_snapshot(&mut catalog, 1, Some(&cursor))
        .expect_err("published changes invalidate old cursors");
    assert_eq!(error.code, "catalog_cursor_stale");
}

#[test]
fn keyset_walk_returns_each_location_once_across_many_pages() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let mut catalog = SqliteCatalog::open(path).expect("catalog");
    let request = fixture_request("scan-many", "C:\\Many");
    catalog
        .begin_scan(&request, "root-many", "C:\\Many")
        .expect("begin scan");
    let transaction = catalog
        .connection
        .transaction()
        .expect("fixture transaction");
    for index in 0..1_025 {
        let asset_id = format!("asset-{index:04}");
        let location_id = format!("location-{index:04}");
        let relative_path = format!("{index:04}.png");
        transaction
            .execute(
                "INSERT INTO assets(id, created_unix_ms) VALUES (?1, 1)",
                [&asset_id],
            )
            .expect("fixture asset");
        transaction
            .execute(
                "INSERT INTO asset_locations(
                       scan_id, asset_id, location_id, root_id, absolute_path,
                       relative_path, preview_path, file_size, modified_unix_ms,
                       width, height
                     ) VALUES (
                       'scan-many', ?1, ?2, 'root-many', ?3, ?4, ?5, 20, 30, 40, 50
                     )",
                params![
                    asset_id,
                    location_id,
                    format!("C:\\Many\\{relative_path}"),
                    relative_path,
                    format!("C:\\Cache\\{index:04}.jpg"),
                ],
            )
            .expect("fixture location");
    }
    transaction.commit().expect("fixture commit");
    catalog
        .publish_scan("scan-many", "root-many", 1_025, 0)
        .expect("publish fixture");

    let mut cursor = None;
    let mut location_ids = Vec::new();
    loop {
        let page = load_default_snapshot(&mut catalog, 128, cursor.as_ref()).expect("keyset page");
        location_ids.extend(page.assets.iter().map(|asset| asset.location_id.clone()));
        cursor = page.next_cursor;
        if cursor.is_none() {
            break;
        }
    }

    let unique_ids = location_ids.iter().collect::<HashSet<_>>();
    assert_eq!(location_ids.len(), 1_025);
    assert_eq!(unique_ids.len(), 1_025);
    assert_eq!(
        location_ids.first().map(String::as_str),
        Some("location-0000")
    );
    assert_eq!(
        location_ids.last().map(String::as_str),
        Some("location-1024")
    );
}

#[test]
fn gallery_keyset_orders_capture_and_fallback_times_across_roots() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let mut catalog = SqliteCatalog::open(path).expect("catalog");
    publish_gallery_fixture(
        &mut catalog,
        "gallery-scan-1",
        "gallery-root-1",
        "C:\\One",
        &[
            ("known-old", Some("2024-01-02T03:04:05.000000000"), 100),
            ("known-tie-low", Some("2025-01-02T03:04:05.000000000"), 100),
            ("unknown-old", None, 100),
        ],
    );
    publish_gallery_fixture(
        &mut catalog,
        "gallery-scan-2",
        "gallery-root-2",
        "C:\\Two",
        &[
            ("known-new", Some("2026-01-02T03:04:05.000000000"), 50),
            ("known-tie-high", Some("2025-01-02T03:04:05.000000000"), 200),
            ("unknown-new", None, 200),
        ],
    );

    let mut cursor = None;
    let mut locations = Vec::new();
    loop {
        let page = load_default_snapshot(&mut catalog, 2, cursor.as_ref()).expect("gallery page");
        locations.extend(page.assets.iter().map(|asset| asset.location_id.clone()));
        cursor = page.next_cursor;
        if cursor.is_none() {
            break;
        }
    }

    assert_eq!(
        locations,
        [
            "known-new",
            "known-tie-high",
            "known-tie-low",
            "known-old",
            "unknown-new",
            "unknown-old",
        ]
    );
}

#[test]
fn gallery_time_fallback_prefers_capture_then_creation_then_modification() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let mut catalog = SqliteCatalog::open(path).expect("catalog");
    publish_gallery_query_fixture(
        &mut catalog,
        "fallback-scan",
        "fallback-root",
        "C:\\Pictures",
        &[
            (
                "created-fallback",
                "created.png",
                None,
                Some(1_749_988_800_000),
                1_784_116_800_000,
            ),
            (
                "modified-fallback",
                "modified.png",
                None,
                None,
                1_715_774_400_000,
            ),
            (
                "capture",
                "capture.png",
                Some("2023-04-15T12:00:00.000000000"),
                Some(1_784_116_800_000),
                1_784_116_800_000,
            ),
        ],
    );

    let snapshot = load_default_snapshot(&mut catalog, 10, None).expect("fallback snapshot");
    assert_eq!(
        snapshot
            .assets
            .iter()
            .map(|asset| asset.location_id.as_str())
            .collect::<Vec<_>>(),
        ["created-fallback", "modified-fallback", "capture"]
    );
    assert!(snapshot.assets[0].capture_time.is_none());
    assert!(snapshot.assets[1].capture_time.is_none());

    let timeline = load_default_timeline(&mut catalog).expect("fallback timeline");
    assert_eq!(
        timeline
            .buckets
            .iter()
            .map(|bucket| bucket.month_key.as_deref())
            .collect::<Vec<_>>(),
        [Some("2025-06"), Some("2024-05"), Some("2023-04")]
    );
}

#[test]
fn gallery_query_combines_source_folder_search_and_all_sort_modes() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let mut catalog = SqliteCatalog::open(path).expect("catalog");
    publish_gallery_query_fixture(
        &mut catalog,
        "query-scan-1",
        "query-root-1",
        "C:\\One",
        &[
            (
                "img10",
                "Album/img10.png",
                Some("2026-01-02T03:04:05.000000000"),
                Some(30),
                40,
            ),
            (
                "img2",
                "Album/Sub/img2.png",
                Some("2025-01-02T03:04:05.000000000"),
                Some(10),
                20,
            ),
            ("cat", "Other/Cat.png", None, None, 60),
        ],
    );
    publish_gallery_query_fixture(
        &mut catalog,
        "query-scan-2",
        "query-root-2",
        "C:\\Two",
        &[(
            "img1-other",
            "Album/img1.png",
            Some("2024-01-02T03:04:05.000000000"),
            Some(5),
            5,
        )],
    );

    let source_query = GalleryQuery {
        root_id: Some("query-root-1".to_owned()),
        ..GalleryQuery::default()
    };
    let source_page = catalog
        .load_snapshot(10, &source_query, "source-query", None, None, None)
        .expect("source query");
    assert_eq!(
        source_page
            .assets
            .iter()
            .map(|asset| asset.location_id.as_str())
            .collect::<Vec<_>>(),
        ["img10", "img2", "cat"]
    );

    let direct_folder_query = GalleryQuery {
        folder_relative_path: Some("Album".to_owned()),
        include_descendants: false,
        ..source_query.clone()
    };
    let direct_folder_page = catalog
        .load_snapshot(
            10,
            &direct_folder_query,
            "direct-folder-query",
            None,
            None,
            None,
        )
        .expect("direct folder query");
    assert_eq!(direct_folder_page.assets.len(), 1);
    assert_eq!(direct_folder_page.assets[0].location_id, "img10");

    let descendant_search_query = GalleryQuery {
        folder_relative_path: Some("Album".to_owned()),
        search_text: "IMG".to_owned(),
        sort_key: GallerySortKey::FileName,
        sort_direction: GallerySortDirection::Ascending,
        ..source_query.clone()
    };
    let descendant_search_page = catalog
        .load_snapshot(
            10,
            &descendant_search_query,
            "descendant-search-query",
            None,
            None,
            None,
        )
        .expect("descendant search query");
    assert_eq!(
        descendant_search_page
            .assets
            .iter()
            .map(|asset| asset.location_id.as_str())
            .collect::<Vec<_>>(),
        ["img2", "img10"]
    );

    let all_name_query = GalleryQuery {
        search_text: "img".to_owned(),
        sort_key: GallerySortKey::FileName,
        sort_direction: GallerySortDirection::Ascending,
        ..GalleryQuery::default()
    };
    let first_name_page = catalog
        .load_snapshot(2, &all_name_query, "name-query", None, None, None)
        .expect("first name page");
    let second_name_page = catalog
        .load_snapshot(
            2,
            &all_name_query,
            "name-query",
            first_name_page.next_cursor.as_ref(),
            None,
            None,
        )
        .expect("second name page");
    let name_locations = first_name_page
        .assets
        .iter()
        .chain(&second_name_page.assets)
        .map(|asset| asset.location_id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(name_locations, ["img1-other", "img2", "img10"]);
    let previous_name_page = catalog
        .load_snapshot(
            2,
            &all_name_query,
            "name-query",
            None,
            second_name_page.previous_cursor.as_ref(),
            None,
        )
        .expect("previous name page");
    assert_eq!(
        previous_name_page
            .assets
            .iter()
            .map(|asset| asset.location_id.as_str())
            .collect::<Vec<_>>(),
        ["img1-other", "img2"]
    );
    assert!(previous_name_page.previous_cursor.is_none());

    let created_query = GalleryQuery {
        sort_key: GallerySortKey::CreatedTime,
        sort_direction: GallerySortDirection::Ascending,
        ..source_query.clone()
    };
    let created_page = catalog
        .load_snapshot(10, &created_query, "created-query", None, None, None)
        .expect("created query");
    assert_eq!(
        created_page
            .assets
            .iter()
            .map(|asset| asset.location_id.as_str())
            .collect::<Vec<_>>(),
        ["img2", "img10", "cat"]
    );

    let modified_query = GalleryQuery {
        sort_key: GallerySortKey::ModifiedTime,
        sort_direction: GallerySortDirection::Descending,
        ..source_query
    };
    let modified_page = catalog
        .load_snapshot(10, &modified_query, "modified-query", None, None, None)
        .expect("modified query");
    assert_eq!(
        modified_page
            .assets
            .iter()
            .map(|asset| asset.location_id.as_str())
            .collect::<Vec<_>>(),
        ["cat", "img10", "img2"]
    );
}

#[test]
fn folder_pages_are_bounded_scoped_and_revision_safe() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let mut catalog = SqliteCatalog::open(path).expect("catalog");
    publish_gallery_query_fixture(
        &mut catalog,
        "folder-scan",
        "folder-root",
        "C:\\Pictures",
        &[
            ("album", "Album/one.png", None, Some(1), 1),
            ("nested", "Album/Sub/two.png", None, Some(2), 2),
            ("other", "Other/three.png", None, Some(3), 3),
        ],
    );

    let first = catalog
        .load_folder_page("folder-root", "", 1, None)
        .expect("first root folder page");
    assert_eq!(first.folders.len(), 1);
    assert_eq!(first.folders[0].relative_path, "Album");
    assert_eq!(first.folders[0].direct_asset_count, 1);
    assert_eq!(first.folders[0].descendant_asset_count, 2);
    let cursor = first.next_cursor.expect("folder cursor");

    let second = catalog
        .load_folder_page("folder-root", "", 1, Some(&cursor))
        .expect("second root folder page");
    assert_eq!(second.folders.len(), 1);
    assert_eq!(second.folders[0].relative_path, "Other");
    assert!(second.next_cursor.is_none());

    let nested = catalog
        .load_folder_page("folder-root", "Album", 10, None)
        .expect("nested folder page");
    assert_eq!(nested.folders.len(), 1);
    assert_eq!(nested.folders[0].relative_path, "Album/Sub");

    let traversal = catalog
        .load_folder_page("folder-root", "../Outside", 10, None)
        .expect_err("folder traversal must be rejected");
    assert_eq!(traversal.code, "catalog_source_scope_invalid");

    publish_fixture(
        &mut catalog,
        "other-scan",
        "other-root",
        "C:\\Other",
        "other-location",
    );
    let stale = catalog
        .load_folder_page("folder-root", "", 1, Some(&cursor))
        .expect_err("published changes invalidate folder cursors");
    assert_eq!(stale.code, "catalog_folder_cursor_stale");
}

#[test]
fn gallery_timeline_uses_file_time_when_capture_time_is_missing() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let mut catalog = SqliteCatalog::open(path).expect("catalog");
    publish_gallery_fixture(
        &mut catalog,
        "timeline-scan-1",
        "timeline-root-1",
        "C:\\One",
        &[
            ("one-2025-a", Some("2025-08-07T03:04:05.000000000"), 10),
            ("one-2025-b", Some("2025-08-01T03:04:05.000000000"), 20),
            ("one-unknown", None, 30),
        ],
    );
    publish_gallery_fixture(
        &mut catalog,
        "timeline-scan-2",
        "timeline-root-2",
        "C:\\Two",
        &[
            ("two-2026", Some("2026-01-02T03:04:05.000000000"), 40),
            ("two-2024", Some("2024-12-31T03:04:05.000000000"), 50),
            ("two-unknown", None, 60),
        ],
    );

    let timeline = load_default_timeline(&mut catalog).expect("timeline");
    assert_eq!(timeline.revision, 2);
    assert_eq!(timeline.total_items, 6);
    assert_eq!(
        timeline.buckets,
        [
            GalleryTimeBucket {
                month_key: Some("2026-01".to_owned()),
                item_count: 1,
                aspect_ratio_milli_sum: 800,
            },
            GalleryTimeBucket {
                month_key: Some("2025-08".to_owned()),
                item_count: 2,
                aspect_ratio_milli_sum: 1_600,
            },
            GalleryTimeBucket {
                month_key: Some("2024-12".to_owned()),
                item_count: 1,
                aspect_ratio_milli_sum: 800,
            },
            GalleryTimeBucket {
                month_key: Some("1970-01".to_owned()),
                item_count: 2,
                aspect_ratio_milli_sum: 1_600,
            },
        ]
    );

    publish_gallery_fixture(
        &mut catalog,
        "timeline-scan-3",
        "timeline-root-1",
        "C:\\One",
        &[("one-replacement", Some("2023-07-01T03:04:05.000000000"), 70)],
    );
    let rescanned = load_default_timeline(&mut catalog).expect("timeline after rescan");
    assert_eq!(rescanned.revision, 3);
    assert_eq!(rescanned.total_items, 4);
    assert!(
        rescanned
            .buckets
            .iter()
            .all(|bucket| { bucket.month_key.as_deref() != Some("2025-08") })
    );
}

#[test]
fn gallery_timeline_bounds_aspect_ratio_weight_and_falls_back_for_missing_dimensions() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let mut catalog = SqliteCatalog::open(path).expect("catalog");
    publish_gallery_dimension_fixture(
        &mut catalog,
        "timeline-dimensions",
        "timeline-dimensions-root",
        "C:\\Dimensions",
        &[
            (
                "panorama",
                Some("2026-01-02T03:04:05.000000000"),
                10,
                1000,
                100,
            ),
            (
                "portrait",
                Some("2026-01-02T03:04:05.000000000"),
                20,
                100,
                1000,
            ),
            (
                "square",
                Some("2026-01-02T03:04:05.000000000"),
                30,
                500,
                500,
            ),
            ("missing", Some("2026-01-02T03:04:05.000000000"), 40, 0, 0),
        ],
    );

    let timeline = load_default_timeline(&mut catalog).expect("timeline");
    assert_eq!(timeline.buckets.len(), 1);
    assert_eq!(timeline.buckets[0].item_count, 4);
    assert_eq!(timeline.buckets[0].aspect_ratio_milli_sum, 7_200);
}

#[test]
fn layout_manifest_chunks_preserve_order_ordinals_and_final_geometry_evidence() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let mut catalog = SqliteCatalog::open(path).expect("catalog");
    publish_gallery_dimension_fixture(
        &mut catalog,
        "layout-manifest-scan",
        "layout-manifest-root",
        "C:\\Layout",
        &[
            ("wide", Some("2026-08-09T12:00:00"), 4_000, 200, 100),
            (
                "unknown-dimensions",
                Some("2026-08-09T11:00:00"),
                3_000,
                0,
                0,
            ),
            (
                "extreme-wide",
                Some("2026-08-08T12:00:00"),
                2_000,
                1_000,
                100,
            ),
            ("extreme-tall", Some("2026-08-07T12:00:00"), 1_000, 10, 100),
        ],
    );

    let first = load_default_layout_manifest_chunk(&mut catalog, 2, None)
        .expect("first layout manifest chunk");
    assert_eq!(first.start_ordinal, 0);
    assert_eq!(first.total_items, 4);
    assert_eq!(first.location_ids, ["wide", "unknown-dimensions"]);
    assert_eq!(first.aspect_ratio_milli, [2_000, 1_000]);
    assert_eq!(first.flags, [LAYOUT_FLAG_DIMENSIONS_KNOWN, 0]);
    assert_eq!(first.date_group_indices, [0, 0]);
    assert_eq!(
        first.date_groups,
        [GalleryLayoutDateGroup {
            date_key: Some("2026-08-09".to_owned()),
        }]
    );

    let cursor = first.next_cursor.expect("next layout cursor");
    assert_eq!(cursor.next_ordinal, 2);
    assert_eq!(cursor.total_items, 4);
    let second = load_default_layout_manifest_chunk(&mut catalog, 2, Some(&cursor))
        .expect("second layout manifest chunk");
    assert_eq!(second.start_ordinal, 2);
    assert_eq!(second.total_items, 4);
    assert_eq!(second.location_ids, ["extreme-wide", "extreme-tall"]);
    assert_eq!(second.aspect_ratio_milli, [5_000, 200]);
    assert_eq!(second.flags, [1, 1]);
    assert_eq!(second.date_group_indices, [0, 1]);
    assert_eq!(
        second.date_groups,
        [
            GalleryLayoutDateGroup {
                date_key: Some("2026-08-08".to_owned()),
            },
            GalleryLayoutDateGroup {
                date_key: Some("2026-08-07".to_owned()),
            },
        ]
    );
    assert!(second.next_cursor.is_none());
}

#[test]
fn layout_manifest_rejects_invalid_limits_and_stale_cursors() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let mut catalog = SqliteCatalog::open(path).expect("catalog");
    publish_gallery_fixture(
        &mut catalog,
        "layout-stale-scan-1",
        "layout-stale-root-1",
        "C:\\One",
        &[("first", Some("2026-08-09T12:00:00"), 1_000)],
    );

    let zero_limit = load_default_layout_manifest_chunk(&mut catalog, 0, None)
        .expect_err("zero layout chunk limit must fail");
    assert_eq!(zero_limit.code, "catalog_layout_chunk_limit_invalid");
    let excessive_limit =
        load_default_layout_manifest_chunk(&mut catalog, MAX_LAYOUT_MANIFEST_CHUNK_ITEMS + 1, None)
            .expect_err("excessive layout chunk limit must fail");
    assert_eq!(excessive_limit.code, "catalog_layout_chunk_limit_invalid");

    let first = load_default_layout_manifest_chunk(&mut catalog, 1, None)
        .expect("first layout manifest chunk");
    let cursor = first.next_cursor;
    assert!(cursor.is_none(), "one item does not produce a next cursor");

    publish_gallery_fixture(
        &mut catalog,
        "layout-stale-scan-2",
        "layout-stale-root-2",
        "C:\\Two",
        &[("second", Some("2026-08-08T12:00:00"), 900)],
    );
    let page = load_default_layout_manifest_chunk(&mut catalog, 1, None)
        .expect("layout page with a cursor");
    let cursor = page.next_cursor.expect("next layout cursor");
    publish_gallery_fixture(
        &mut catalog,
        "layout-stale-scan-3",
        "layout-stale-root-3",
        "C:\\Three",
        &[("third", Some("2026-08-07T12:00:00"), 800)],
    );

    let stale = load_default_layout_manifest_chunk(&mut catalog, 1, Some(&cursor))
        .expect_err("stale layout cursor must fail");
    assert_eq!(stale.code, "catalog_layout_cursor_stale");
}

#[test]
fn unregistering_a_root_removes_only_catalog_state_and_preserves_source_bytes() {
    let directory = tempdir().expect("temporary directory");
    let source_directory = directory.path().join("source");
    std::fs::create_dir(&source_directory).expect("source directory");
    let source_file = source_directory.join("one.png");
    let source_bytes = b"irreplaceable source bytes";
    std::fs::write(&source_file, source_bytes).expect("source fixture");
    let path = directory.path().join("catalog.sqlite3");
    let mut catalog = SqliteCatalog::open(path).expect("catalog");
    let root_path = source_directory.to_string_lossy();
    publish_gallery_fixture(
        &mut catalog,
        "remove-scan",
        "remove-root",
        &root_path,
        &[("one", Some("2026-01-02T03:04:05.000000000"), 10)],
    );
    let revision_before = load_default_snapshot(&mut catalog, 10, None)
        .expect("snapshot before removal")
        .revision;

    assert!(
        catalog
            .unregister_root("remove-root")
            .expect("unregister root")
    );

    let snapshot = load_default_snapshot(&mut catalog, 10, None).expect("snapshot after removal");
    assert!(snapshot.roots.is_empty());
    assert!(snapshot.assets.is_empty());
    assert_eq!(snapshot.revision, revision_before + 1);
    assert_eq!(
        load_default_timeline(&mut catalog)
            .expect("timeline after removal")
            .total_items,
        0
    );
    assert_eq!(
        std::fs::read(&source_file).expect("source remains readable"),
        source_bytes
    );
    assert!(
        !catalog
            .unregister_root("remove-root")
            .expect("second unregister is idempotent")
    );
}

#[test]
fn gallery_anchor_cursors_begin_at_capture_or_fallback_month_without_gaps() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let mut catalog = SqliteCatalog::open(path).expect("catalog");
    publish_gallery_fixture(
        &mut catalog,
        "anchor-scan",
        "anchor-root",
        "C:\\Pictures",
        &[
            ("newer", Some("2026-02-01T00:00:00.000000000"), 50),
            ("selected-new", Some("2025-08-20T00:00:00.000000000"), 40),
            ("selected-old", Some("2025-08-01T00:00:00.000000000"), 30),
            ("older", Some("2024-01-01T00:00:00.000000000"), 20),
            ("unknown", None, 10),
        ],
    );
    let timeline = load_default_timeline(&mut catalog).expect("timeline");
    let month_anchor = GalleryTimeAnchor {
        revision: timeline.revision,
        query_id: TEST_QUERY_ID.to_owned(),
        month_key: Some("2025-08".to_owned()),
        item_offset: 0,
    };
    let month_page = catalog
        .load_snapshot(
            3,
            &GalleryQuery::default(),
            TEST_QUERY_ID,
            None,
            None,
            Some(&month_anchor),
        )
        .expect("month page");
    assert_eq!(
        month_page
            .assets
            .iter()
            .map(|asset| asset.location_id.as_str())
            .collect::<Vec<_>>(),
        ["selected-new", "selected-old", "older"]
    );
    assert!(month_page.next_cursor.is_some());
    assert!(month_page.previous_cursor.is_some());

    let month_middle = GalleryTimeAnchor {
        item_offset: 1,
        ..month_anchor.clone()
    };
    let middle_page = catalog
        .load_snapshot(
            3,
            &GalleryQuery::default(),
            TEST_QUERY_ID,
            None,
            None,
            Some(&month_middle),
        )
        .expect("month middle page");
    assert_eq!(
        middle_page
            .assets
            .iter()
            .map(|asset| asset.location_id.as_str())
            .collect::<Vec<_>>(),
        ["selected-old", "older", "unknown"]
    );
    let first_previous_page = catalog
        .load_snapshot(
            1,
            &GalleryQuery::default(),
            TEST_QUERY_ID,
            None,
            middle_page.previous_cursor.as_ref(),
            None,
        )
        .expect("first previous page");
    assert_eq!(first_previous_page.assets[0].location_id, "selected-new");
    assert!(first_previous_page.previous_cursor.is_some());
    assert!(first_previous_page.next_cursor.is_some());

    let oldest_previous_page = catalog
        .load_snapshot(
            1,
            &GalleryQuery::default(),
            TEST_QUERY_ID,
            None,
            first_previous_page.previous_cursor.as_ref(),
            None,
        )
        .expect("oldest previous page");
    assert_eq!(oldest_previous_page.assets[0].location_id, "newer");
    assert!(oldest_previous_page.previous_cursor.is_none());
    assert!(oldest_previous_page.next_cursor.is_some());

    let fallback_anchor = GalleryTimeAnchor {
        revision: timeline.revision,
        query_id: TEST_QUERY_ID.to_owned(),
        month_key: Some("1970-01".to_owned()),
        item_offset: 0,
    };
    let fallback_page = catalog
        .load_snapshot(
            3,
            &GalleryQuery::default(),
            TEST_QUERY_ID,
            None,
            None,
            Some(&fallback_anchor),
        )
        .expect("fallback page");
    assert_eq!(fallback_page.assets.len(), 1);
    assert_eq!(fallback_page.assets[0].location_id, "unknown");
    assert!(fallback_page.next_cursor.is_none());
}

fn publish_fixture(
    catalog: &mut SqliteCatalog,
    scan_id: &str,
    root_id: &str,
    root_path: &str,
    location_id: &str,
) {
    let request = fixture_request(scan_id, root_path);
    catalog
        .begin_scan(&request, root_id, root_path)
        .expect("begin scan");
    catalog
        .stage_location(
            scan_id,
            root_id,
            &AssetLocationView {
                asset_id: format!("asset-{scan_id}"),
                location_id: location_id.to_owned(),
                root_id: root_id.to_owned(),
                absolute_path: format!("{root_path}\\one.png"),
                display_path: format!("{root_path}\\one.png"),
                relative_path: "one.png".to_owned(),
                preview_path: format!("C:\\Cache\\{scan_id}.jpg"),
                file_size: 20,
                created_unix_ms: Some(25),
                modified_unix_ms: 30,
                file_identity: None,
                width: 40,
                height: 50,
                preview_status: PreviewStatus::Ready,
                preview_issue_code: None,
                preview_issue_message: None,
                metadata_engine_id: "fixture-metadata".to_owned(),
                metadata_engine_version: "1".to_owned(),
                capture_time: None,
            },
        )
        .expect("stage location");
    catalog
        .publish_scan(scan_id, root_id, 1, 0)
        .expect("publish scan");
}

fn publish_gallery_fixture(
    catalog: &mut SqliteCatalog,
    scan_id: &str,
    root_id: &str,
    root_path: &str,
    locations: &[(&str, Option<&str>, i64)],
) {
    let dimensioned_locations = locations
        .iter()
        .map(|(location_id, capture_local_time, modified_unix_ms)| {
            (*location_id, *capture_local_time, *modified_unix_ms, 40, 50)
        })
        .collect::<Vec<_>>();
    publish_gallery_dimension_fixture(catalog, scan_id, root_id, root_path, &dimensioned_locations);
}

fn publish_gallery_query_fixture(
    catalog: &mut SqliteCatalog,
    scan_id: &str,
    root_id: &str,
    root_path: &str,
    locations: &[GalleryQueryFixture<'_>],
) {
    let request = fixture_request(scan_id, root_path);
    catalog
        .begin_scan(&request, root_id, root_path)
        .expect("begin query fixture scan");
    for (location_id, relative_path, capture_local_time, created_unix_ms, modified_unix_ms) in
        locations
    {
        catalog
            .stage_location(
                scan_id,
                root_id,
                &AssetLocationView {
                    asset_id: format!("asset-{scan_id}-{location_id}"),
                    location_id: (*location_id).to_owned(),
                    root_id: root_id.to_owned(),
                    absolute_path: format!("{root_path}\\{}", relative_path.replace('/', "\\")),
                    display_path: format!("{root_path}\\{}", relative_path.replace('/', "\\")),
                    relative_path: (*relative_path).to_owned(),
                    preview_path: String::new(),
                    file_size: 20,
                    created_unix_ms: *created_unix_ms,
                    modified_unix_ms: *modified_unix_ms,
                    file_identity: None,
                    width: 40,
                    height: 50,
                    preview_status: PreviewStatus::Pending,
                    preview_issue_code: None,
                    preview_issue_message: None,
                    metadata_engine_id: "fixture-metadata".to_owned(),
                    metadata_engine_version: "1".to_owned(),
                    capture_time: capture_local_time.map(|local_time| CaptureTimeEvidence {
                        local_time: local_time.to_owned(),
                        offset_minutes: None,
                        source: CaptureTimeSource::Original,
                        raw_value: local_time.to_owned(),
                    }),
                },
            )
            .expect("stage query fixture location");
    }
    catalog
        .publish_scan(
            scan_id,
            root_id,
            u64::try_from(locations.len()).expect("query fixture location count"),
            0,
        )
        .expect("publish query fixture scan");
}

fn publish_gallery_dimension_fixture(
    catalog: &mut SqliteCatalog,
    scan_id: &str,
    root_id: &str,
    root_path: &str,
    locations: &[(&str, Option<&str>, i64, u32, u32)],
) {
    let request = fixture_request(scan_id, root_path);
    catalog
        .begin_scan(&request, root_id, root_path)
        .expect("begin gallery scan");
    for (location_id, capture_local_time, modified_unix_ms, width, height) in locations {
        catalog
            .stage_location(
                scan_id,
                root_id,
                &AssetLocationView {
                    asset_id: format!("asset-{scan_id}-{location_id}"),
                    location_id: (*location_id).to_owned(),
                    root_id: root_id.to_owned(),
                    absolute_path: format!("{root_path}\\{location_id}.png"),
                    display_path: format!("{root_path}\\{location_id}.png"),
                    relative_path: format!("{location_id}.png"),
                    preview_path: String::new(),
                    file_size: 20,
                    created_unix_ms: Some(*modified_unix_ms - 1),
                    modified_unix_ms: *modified_unix_ms,
                    file_identity: None,
                    width: *width,
                    height: *height,
                    preview_status: PreviewStatus::Pending,
                    preview_issue_code: None,
                    preview_issue_message: None,
                    metadata_engine_id: "fixture-metadata".to_owned(),
                    metadata_engine_version: "1".to_owned(),
                    capture_time: capture_local_time.map(|local_time| CaptureTimeEvidence {
                        local_time: local_time.to_owned(),
                        offset_minutes: None,
                        source: CaptureTimeSource::Original,
                        raw_value: local_time.to_owned(),
                    }),
                },
            )
            .expect("stage gallery location");
    }
    catalog
        .publish_scan(
            scan_id,
            root_id,
            u64::try_from(locations.len()).expect("gallery location count"),
            0,
        )
        .expect("publish gallery scan");
}

fn fixture_request(scan_id: &str, root_path: &str) -> ScanRequest {
    ScanRequest {
        scan_id: scan_id.to_owned(),
        root_path: root_path.to_owned(),
        max_items: Some(500),
        max_entries: Some(2_000),
        preview_edge: 512,
    }
}
