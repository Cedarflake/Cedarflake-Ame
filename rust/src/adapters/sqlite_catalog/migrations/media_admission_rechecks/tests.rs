use super::*;
use crate::adapters::SqliteCatalog;
use tempfile::tempdir;

#[test]
fn upgrade_is_catalog_only_and_does_not_seed_fresh_databases() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("catalog.sqlite3");
    drop(SqliteCatalog::open(path.clone()).unwrap());
    let connection = Connection::open(&path).unwrap();
    assert_eq!(
        connection
            .query_row(
                "SELECT count(*) FROM library_media_admission_rechecks",
                [],
                |row| row.get::<_, i64>(0)
            )
            .unwrap(),
        0
    );
    connection
        .execute_batch(
            "DROP TABLE library_media_admission_rechecks;
        UPDATE schema_info SET version = 37; PRAGMA user_version = 37;",
        )
        .unwrap();
    drop(connection);
    drop(SqliteCatalog::open(path.clone()).expect("upgrade schema 37"));
    let connection = Connection::open(&path).unwrap();
    validate(&connection).unwrap();
    current_schema::validate_schema_version(&connection, super::super::SCHEMA_VERSION).unwrap();
    assert_eq!(
        connection
            .query_row("SELECT count(*) FROM scan_runs", [], |row| row
                .get::<_, i64>(0))
            .unwrap(),
        0
    );
}

#[test]
fn conflicting_recheck_schema_rolls_back_without_promoting_the_catalog() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("catalog.sqlite3");
    drop(SqliteCatalog::open(path.clone()).unwrap());
    let connection = Connection::open(&path).unwrap();
    connection
        .execute_batch(
            "DROP TABLE library_media_admission_rechecks;
        CREATE TABLE library_media_admission_rechecks (root_id TEXT);
        UPDATE schema_info SET version = 37; PRAGMA user_version = 37;",
        )
        .unwrap();
    drop(connection);
    assert!(SqliteCatalog::open(path.clone()).is_err());
    let connection = Connection::open(path).unwrap();
    assert_eq!(
        connection
            .query_row("SELECT version FROM schema_info", [], |row| row
                .get::<_, i64>(0))
            .unwrap(),
        37
    );
}
