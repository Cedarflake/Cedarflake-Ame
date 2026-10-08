use tempfile::tempdir;

use crate::adapters::SqliteCatalog;
use crate::domain::{AssetLocationView, PreviewStatus, ScanCheckpoint, ScanRequest};
use crate::ports::CatalogRepository;

use super::*;

#[test]
fn v32_migration_preserves_unpublished_checkpoint_and_retained_assets() {
    let directory = tempdir().expect("derived fixture");
    let source = tempdir().expect("source fixture");
    let path = directory.path().join("catalog.sqlite3");
    let mut catalog = SqliteCatalog::open(path.clone()).expect("fresh catalog");
    let root = source.path().to_string_lossy().into_owned();
    catalog
        .begin_scan(
            &ScanRequest {
                scan_id: "retained".to_owned(),
                root_path: root.clone(),
                max_items: None,
                max_entries: None,
                preview_edge: 128,
            },
            "root",
            &root,
        )
        .expect("retained scan");
    let source_path = source
        .path()
        .join("retained.png")
        .to_string_lossy()
        .into_owned();
    catalog
        .stage_location(
            "retained",
            "root",
            &AssetLocationView {
                asset_id: "retained-asset".to_owned(),
                location_id: "retained-location".to_owned(),
                root_id: "root".to_owned(),
                scan_id: "retained".to_owned(),
                absolute_path: source_path.clone(),
                display_path: source_path,
                relative_path: "retained.png".to_owned(),
                preview_path: String::new(),
                file_size: 64,
                created_unix_ms: Some(7),
                modified_unix_ms: 8,
                file_identity: None,
                source_revision: None,
                source_generation: 0,
                width: 37,
                height: 19,
                preview_status: PreviewStatus::Pending,
                preview_issue_code: None,
                preview_issue_message: None,
                metadata_engine_id: "fixture".to_owned(),
                metadata_engine_version: "1".to_owned(),
                capture_time: None,
            },
        )
        .expect("retain the unpublished inspection");
    catalog
        .checkpoint_scan(
            "retained",
            &ScanCheckpoint {
                visited_entries: 12,
                accepted_items: 1,
                ..ScanCheckpoint::default()
            },
        )
        .expect("checkpoint");
    drop(catalog);
    let connection = Connection::open(&path).expect("fixture connection");
    let retained_location = location_evidence(&connection);
    downgrade_to_v32_for_test(&connection);
    drop(connection);
    let catalog = SqliteCatalog::open(path.clone()).expect("migrate v32");
    drop(catalog);
    let connection = Connection::open(path).expect("migration evidence");
    let result: (i64, i64, i64) = connection
        .query_row(
            "SELECT (SELECT version FROM schema_info),
                (SELECT visited_entries FROM scan_runs WHERE id = 'retained'),
                (SELECT accepted_items FROM scan_runs WHERE id = 'retained')",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("preserved values");
    assert_eq!(result, (33, 12, 1));
    assert_eq!(location_evidence(&connection), retained_location);
    validate_structure(&connection).expect("current resume schema");
}

fn location_evidence(connection: &Connection) -> Vec<rusqlite::types::Value> {
    let mut statement = connection
        .prepare("SELECT * FROM asset_locations WHERE location_id = 'retained-location'")
        .expect("retained location evidence");
    let columns = statement.column_count();
    statement
        .query_row([], |row| {
            (0..columns).map(|column| row.get(column)).collect()
        })
        .expect("unchanged persisted inspection")
}

#[test]
fn conflicting_resume_schema_rolls_back_migration() {
    let directory = tempdir().expect("derived fixture");
    let path = directory.path().join("catalog.sqlite3");
    drop(SqliteCatalog::open(path.clone()).expect("fresh catalog"));
    let mut connection = Connection::open(path).expect("fixture connection");
    downgrade_to_v32_for_test(&connection);
    connection
        .execute_batch(
            "CREATE TABLE scan_resume_pending_locations(unexpected INTEGER);
        INSERT INTO scan_resume_pending_locations VALUES (23);",
        )
        .expect("conflicting retained input");
    assert!(migrate(&mut connection).is_err());
    let result: (i64, i64, i64) = connection
        .query_row(
            "SELECT (SELECT version FROM schema_info),
                (SELECT user_version FROM pragma_user_version),
                (SELECT unexpected FROM scan_resume_pending_locations)",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("rollback evidence");
    assert_eq!(result, (32, 32, 23));
}

#[test]
fn missing_publication_guard_is_not_repaired_as_a_valid_current_catalog() {
    let directory = tempdir().expect("derived fixture");
    let path = directory.path().join("catalog.sqlite3");
    drop(SqliteCatalog::open(path.clone()).expect("fresh catalog"));
    let connection = Connection::open(&path).expect("fixture connection");
    connection
        .execute_batch("DROP TRIGGER scan_resume_publication_guard")
        .expect("damaged fixture");
    drop(connection);
    let error = match SqliteCatalog::open(path) {
        Ok(_) => panic!("missing publication guard was admitted"),
        Err(error) => error,
    };
    assert_eq!(error.code, "catalog_scan_resume_inventory_unverifiable");
}
