use tempfile::tempdir;

use crate::adapters::SqliteCatalog;
use crate::domain::{ScanCheckpoint, ScanRequest};
use crate::ports::CatalogRepository;

use super::*;

fn downgrade(connection: &Connection) {
    connection
        .execute_batch(
            "DROP TABLE scan_directory_identities;
         UPDATE schema_info SET version = 34; PRAGMA user_version = 34;",
        )
        .expect("v34 fixture");
}

fn version(connection: &Connection) -> i64 {
    connection
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .expect("schema version")
}

#[test]
fn v34_catalog_migrates_without_inventing_legacy_directory_coverage() {
    let storage = tempdir().expect("derived storage");
    let path = storage.path().join("catalog.sqlite3");
    let mut catalog = SqliteCatalog::open(path.clone()).expect("catalog");
    let request = ScanRequest {
        scan_id: "retained".to_owned(),
        root_path: storage.path().join("source").to_string_lossy().into_owned(),
        max_items: None,
        max_entries: None,
        preview_edge: 128,
    };
    catalog
        .begin_scan(&request, "root", &request.root_path)
        .expect("retained scan");
    catalog
        .pause_scan(
            &request.scan_id,
            &ScanCheckpoint {
                visited_entries: 17,
                ..ScanCheckpoint::default()
            },
        )
        .expect("retained progress");
    downgrade(&catalog.connection);
    drop(catalog);
    for _ in 0..2 {
        let reopened = SqliteCatalog::open(path.clone()).expect("migrate and reopen");
        assert_eq!(version(&reopened.connection), 35);
        let retained: (String, i64) = reopened
            .connection
            .query_row(
                "SELECT status, visited_entries FROM scan_runs WHERE id = 'retained'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("preserved progress");
        assert_eq!(retained, ("paused".to_owned(), 17));
        let count: i64 = reopened
            .connection
            .query_row(
                "SELECT COUNT(*) FROM scan_directory_identities",
                [],
                |row| row.get(0),
            )
            .expect("no inferred directory identities");
        assert_eq!(count, 0);
    }
}

#[test]
fn conflicting_schema_rolls_back_version_and_preserves_the_existing_table() {
    let storage = tempdir().expect("derived storage");
    let path = storage.path().join("catalog.sqlite3");
    let catalog = SqliteCatalog::open(path.clone()).expect("catalog");
    downgrade(&catalog.connection);
    catalog
        .connection
        .execute_batch(
            "CREATE TABLE scan_directory_identities(retained TEXT);
         INSERT INTO scan_directory_identities VALUES ('keep');",
        )
        .expect("conflicting table");
    drop(catalog);
    let mut connection = Connection::open(path).expect("fixture connection");
    let error = migrate(&mut connection).expect_err("conflict must abort migration");
    assert_eq!(error.code, "catalog_directory_identity_contract_invalid");
    assert_eq!(version(&connection), 34);
    let retained: String = connection
        .query_row(
            "SELECT retained FROM scan_directory_identities",
            [],
            |row| row.get(0),
        )
        .expect("preserved conflicting data");
    assert_eq!(retained, "keep");
}

#[test]
fn missing_directory_identity_contract_is_rejected_on_reopen() {
    let storage = tempdir().expect("derived storage");
    let path = storage.path().join("catalog.sqlite3");
    let catalog = SqliteCatalog::open(path.clone()).expect("catalog");
    catalog
        .connection
        .execute_batch("DROP INDEX scan_directory_identities_by_identity")
        .expect("damaged required lookup");
    drop(catalog);
    let error = match SqliteCatalog::open(path) {
        Ok(_) => panic!("missing directory contract must be rejected"),
        Err(error) => error,
    };
    assert_eq!(error.code, "catalog_directory_identity_contract_invalid");
}
