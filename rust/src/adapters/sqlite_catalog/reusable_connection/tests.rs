use rusqlite::Connection;
use tempfile::{TempDir, tempdir};

use super::super::{
    AssetLocationView, LibraryChangeLane, PendingLocation, PreviewStatus, SqliteCatalogReadStage,
    full_schema_validation_count, reset_full_schema_validation_count,
};
use super::{SqliteCatalog, SqliteCatalogSession};

mod maintenance;

fn fixture() -> (TempDir, SqliteCatalogSession, SqliteCatalog) {
    let directory = tempdir().expect("catalog directory");
    let path = directory.path().join("catalog.sqlite3");
    reset_full_schema_validation_count(&path);
    let session = SqliteCatalogSession::validate(path).expect("validate catalog");
    let catalog = session
        .open_in_lane(LibraryChangeLane::Live)
        .expect("open retained connection");
    (directory, session, catalog)
}

fn assert_rejected(session: &SqliteCatalogSession, catalog: &SqliteCatalog, code: &str) {
    assert_eq!(
        session
            .revalidate_connection(catalog)
            .expect_err("connection must not be reused")
            .code,
        code,
    );
}

#[test]
fn reusable_connection_observes_committed_data_without_full_revalidation() {
    let (_directory, session, catalog) = fixture();
    for expected_revision in 1..=3_i64 {
        let writer = Connection::open(session.path()).expect("independent writer");
        writer
            .execute("UPDATE catalog_state SET revision = revision + 1", [])
            .expect("commit ordinary data");
        drop(writer);

        session
            .revalidate_connection(&catalog)
            .expect("ordinary data preserves session proof");
        let revision: i64 = catalog
            .connection
            .query_row("SELECT revision FROM catalog_state", [], |row| row.get(0))
            .expect("retained connection sees current committed data");
        assert_eq!(revision, expected_revision);
    }
    assert_eq!(full_schema_validation_count(session.path()), 1);
}

#[test]
fn reusable_connection_rejects_schema_and_header_drift() {
    for sql in [
        "CREATE TABLE unexpected_runtime_table(value INTEGER)",
        "PRAGMA schema_version = 4242",
        "PRAGMA application_id = 0",
        "PRAGMA user_version = 0",
        "UPDATE schema_info SET version = 0",
    ] {
        let (_directory, session, catalog) = fixture();
        Connection::open(session.path())
            .expect("independent structural writer")
            .execute_batch(sql)
            .expect("change persisted proof");
        assert_rejected(&session, &catalog, "catalog_validated_session_stale");
        assert_eq!(full_schema_validation_count(session.path()), 1);
    }
}

#[test]
fn reusable_connection_rejects_non_wal_mode() {
    let (_directory, session, catalog) = fixture();
    catalog
        .connection
        .execute_batch("PRAGMA journal_mode = DELETE;")
        .expect("change mode while the only connection is idle");
    assert_rejected(&session, &catalog, "catalog_validated_session_stale");
}

#[test]
fn reusable_connection_rejects_another_catalog_or_original_session_proof() {
    let (_directory, session, mut catalog) = fixture();
    let (_other_directory, other_session, _other_catalog) = fixture();
    assert_rejected(&other_session, &catalog, "catalog_validated_session_stale");

    let original = catalog.session.clone();
    for changed in ["path", "identity", "application", "version", "cookie"] {
        catalog.session = original.clone();
        match changed {
            "path" => catalog.session.path = other_session.path.clone(),
            "identity" => {
                catalog.session.database_identity = other_session.database_identity.clone();
            }
            "application" => catalog.session.application_id = 0,
            "version" => catalog.session.user_version = 0,
            "cookie" => catalog.session.schema_cookie += 1,
            _ => unreachable!(),
        }
        assert_rejected(&session, &catalog, "catalog_validated_session_stale");
    }
    catalog.session = original;
    catalog.path = other_session.path.clone();
    assert_rejected(&session, &catalog, "catalog_validated_session_stale");
    catalog.path = session.path.clone();
    session
        .revalidate_connection(&catalog)
        .expect("restored original proof is reusable");
}

#[test]
fn reusable_connection_rejects_an_open_transaction_without_rolling_it_back() {
    let (_directory, session, catalog) = fixture();
    catalog
        .connection
        .execute_batch("BEGIN IMMEDIATE; UPDATE catalog_state SET revision = revision + 1;")
        .expect("leave an owned transaction open");

    assert_rejected(&session, &catalog, "catalog_connection_not_reusable");
    assert!(!catalog.connection.is_autocommit());
    let revision: i64 = catalog
        .connection
        .query_row("SELECT revision FROM catalog_state", [], |row| row.get(0))
        .expect("uncommitted data remains owned");
    assert_eq!(revision, 1);
    catalog
        .connection
        .execute_batch("ROLLBACK;")
        .expect("explicit owner rollback");
    session
        .revalidate_connection(&catalog)
        .expect("owner released the transaction");
    let revision: i64 = catalog
        .connection
        .query_row("SELECT revision FROM catalog_state", [], |row| row.get(0))
        .expect("rollback preserved committed data");
    assert_eq!(revision, 0);
}

#[test]
fn reusable_connection_rejects_a_busy_statement_in_autocommit_mode() {
    let (_directory, session, catalog) = fixture();
    let mut statement = catalog
        .connection
        .prepare("SELECT 1 UNION ALL SELECT 2")
        .expect("prepare retained cursor");
    let mut rows = statement.query([]).expect("start cursor");
    assert!(rows.next().expect("first row").is_some());
    assert!(catalog.connection.is_autocommit());
    assert!(catalog.connection.is_busy());
    assert_rejected(&session, &catalog, "catalog_connection_not_reusable");
    assert!(catalog.connection.is_busy());
    let second: i64 = rows
        .next()
        .expect("cursor remains owned")
        .expect("second row")
        .get(0)
        .expect("decode second row");
    assert_eq!(second, 2);
    drop(rows);
    drop(statement);
    session
        .revalidate_connection(&catalog)
        .expect("reset statements release the connection");
}

#[test]
fn reusable_connection_rejects_each_pending_publication_buffer_without_clearing_it() {
    let (directory, session, mut catalog) = fixture();
    let path = directory
        .path()
        .join("unread.jpg")
        .to_string_lossy()
        .into_owned();
    catalog.pending_locations.push(PendingLocation {
        scan_id: "pending-scan".to_owned(),
        root_id: "pending-root".to_owned(),
        location: AssetLocationView {
            asset_id: "pending-asset".to_owned(),
            location_id: "pending-location".to_owned(),
            root_id: "pending-root".to_owned(),
            scan_id: "pending-scan".to_owned(),
            absolute_path: path.clone(),
            display_path: path,
            relative_path: "unread.jpg".to_owned(),
            preview_path: String::new(),
            file_size: 0,
            created_unix_ms: None,
            modified_unix_ms: 0,
            file_identity: None,
            source_revision: None,
            source_generation: 0,
            width: 0,
            height: 0,
            preview_status: PreviewStatus::Pending,
            preview_issue_code: None,
            preview_issue_message: None,
            metadata_engine_id: String::new(),
            metadata_engine_version: String::new(),
            capture_time: None,
        },
        identity_group_baseline: None,
    });
    assert_rejected(&session, &catalog, "catalog_connection_not_reusable");
    assert_eq!(catalog.pending_locations.len(), 1);
    catalog.pending_locations.clear();
    session
        .revalidate_connection(&catalog)
        .expect("location staging released by its owner");

    catalog
        .pending_authoritative_retry_paths
        .push("unread.jpg".to_owned());
    assert_rejected(&session, &catalog, "catalog_connection_not_reusable");
    assert_eq!(catalog.pending_authoritative_retry_paths, ["unread.jpg"]);
    catalog.pending_authoritative_retry_paths.clear();
    session
        .revalidate_connection(&catalog)
        .expect("retry staging released by its owner");
}

#[test]
fn reusable_connection_keeps_existing_open_diagnostic_stage_order() {
    let directory = tempdir().expect("catalog directory");
    let session = SqliteCatalogSession::validate(directory.path().join("catalog.sqlite3"))
        .expect("validate catalog");
    let mut stages = Vec::new();
    let catalog = session
        .open_in_lane_with_read_stage(LibraryChangeLane::Live, |stage| {
            stages.push(stage);
            Ok(())
        })
        .expect("open through original diagnostic seam");
    assert!(matches!(
        stages.as_slice(),
        [
            SqliteCatalogReadStage::Open,
            SqliteCatalogReadStage::Validation
        ]
    ));
    session
        .revalidate_connection(&catalog)
        .expect("common proof accepts the newly opened connection");
}
