use super::*;
use crate::adapters::sqlite_catalog::SqliteCatalog;

#[test]
fn unsupported_versions_leave_existing_rows_and_files_untouched() {
    for version in [0, SCHEMA_VERSION - 1, SCHEMA_VERSION + 1] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("catalog.sqlite3");
        let catalog = SqliteCatalog::open(path.clone()).unwrap();
        catalog
            .connection
            .execute("UPDATE schema_info SET version = ?1", [version])
            .unwrap();
        catalog
            .connection
            .pragma_update(None, "user_version", version)
            .unwrap();
        catalog
            .connection
            .execute("UPDATE catalog_state SET revision = 37", [])
            .unwrap();
        drop(catalog);
        let connection = Connection::open(&path).unwrap();
        connection
            .execute_batch("PRAGMA journal_mode = DELETE;")
            .unwrap();
        drop(connection);
        let original = std::fs::read(&path).unwrap();
        let error = SqliteCatalog::open(path.clone()).err().unwrap();
        assert_eq!(error.code, "catalog_schema_unsupported");
        assert_eq!(std::fs::read(&path).unwrap(), original);
        let connection = Connection::open(path).unwrap();
        assert_eq!(
            connection
                .query_row("SELECT version FROM schema_info", [], |row| row
                    .get::<_, i64>(0))
                .unwrap(),
            version
        );
        assert_eq!(
            connection
                .query_row("SELECT revision FROM catalog_state", [], |row| row
                    .get::<_, i64>(0))
                .unwrap(),
            37
        );
        assert!(std::fs::read_dir(directory.path()).unwrap().all(|entry| {
            !entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .contains(".before-v")
        }));
    }
}

#[test]
fn nonempty_unidentified_database_is_not_initialized() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("unidentified.sqlite3");
    let connection = Connection::open(&path).unwrap();
    connection
        .execute_batch(
            "CREATE TABLE sqliteX_fixture(value TEXT);
             INSERT INTO sqliteX_fixture VALUES ('keep');",
        )
        .unwrap();
    drop(connection);
    let original = std::fs::read(&path).unwrap();
    assert_eq!(
        SqliteCatalog::open(path.clone()).err().unwrap().code,
        "catalog_schema_unsupported"
    );
    assert_eq!(std::fs::read(&path).unwrap(), original);
    let connection = Connection::open(&path).unwrap();
    assert_eq!(user_schema_objects(&connection).unwrap(), 1);
    assert_eq!(
        connection
            .query_row("SELECT value FROM sqliteX_fixture", [], |row| row
                .get::<_, String>(0))
            .unwrap(),
        "keep"
    );
}

#[test]
fn exhausted_storage_rolls_back_partial_initialization_and_can_retry() {
    let mut connection = Connection::open_in_memory().unwrap();
    prepare_fresh_catalog_auto_vacuum(&connection).unwrap();
    connection
        .execute_batch("PRAGMA max_page_count = 8;")
        .unwrap();
    assert!(initialize(&mut connection).is_err());
    assert_eq!(user_schema_objects(&connection).unwrap(), 0);
    connection
        .execute_batch("PRAGMA max_page_count = 10000;")
        .unwrap();
    initialize(&mut connection).unwrap();
    assert_eq!(
        connection
            .query_row("SELECT version FROM schema_info", [], |row| row
                .get::<_, i64>(0))
            .unwrap(),
        SCHEMA_VERSION
    );
}

#[test]
fn missing_current_metadata_marker_is_rejected_without_repairing_rows() {
    let mut connection = Connection::open_in_memory().unwrap();
    super::super::initialize_schema(&mut connection).unwrap();
    connection
        .execute_batch(
            "DROP TABLE library_source_revision_metadata_contract;
             UPDATE catalog_state SET revision = 37;",
        )
        .unwrap();
    assert_eq!(
        super::super::initialize_schema(&mut connection)
            .unwrap_err()
            .code,
        "catalog_source_revision_contract_unverifiable"
    );
    assert_eq!(
        connection
            .query_row("SELECT revision FROM catalog_state", [], |row| row
                .get::<_, i64>(0))
            .unwrap(),
        37
    );
    let marker_exists = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_schema
             WHERE name = 'library_source_revision_metadata_contract')",
            [],
            |row| row.get::<_, bool>(0),
        )
        .unwrap();
    assert!(!marker_exists);
}
