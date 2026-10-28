use crate::adapters::sqlite_catalog::SqliteCatalog;

use super::*;

fn prior() -> (tempfile::TempDir, SqliteCatalog) {
    let directory = tempfile::tempdir().unwrap();
    let catalog = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
    downgrade_to_v58_for_test(&catalog.connection);
    catalog
        .connection
        .execute_batch(
            "INSERT INTO exact_duplicate_decisions VALUES
           ('blake3-256-full-v1', printf('%064x', 1), 20, 'copy', 'asset', 0, 1),
           ('blake3-256-full-v1', printf('%064x', 2), 30, NULL, NULL, 1, 2);",
        )
        .unwrap();
    (directory, catalog)
}

#[test]
fn upgrade_indexes_preferences_without_changing_choices() {
    let (directory, mut catalog) = prior();
    migrate(&mut catalog.connection).unwrap();
    current_schema::validate_schema_version(&catalog.connection, 59).unwrap();
    let choices: (i64, i64, String) = catalog.connection.query_row(
        "SELECT COUNT(*), SUM(ignored), MAX(preferred_location_id) FROM exact_duplicate_decisions",
        [], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    ).unwrap();
    assert_eq!(choices, (2, 1, "copy".into()));
    drop(catalog);
    let reopened = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
    validate(&reopened.connection).unwrap();
}

#[test]
fn failed_upgrade_keeps_prior_version_and_allows_retry() {
    let (_directory, mut catalog) = prior();
    catalog
        .connection
        .execute_batch(
            "CREATE TRIGGER reject_preferred_index BEFORE UPDATE ON schema_info
         WHEN NEW.version = 59 BEGIN SELECT RAISE(ABORT, 'injected failure'); END;",
        )
        .unwrap();
    assert!(migrate(&mut catalog.connection).is_err());
    current_schema::validate_schema_version(&catalog.connection, 58).unwrap();
    let indexes: i64 = catalog
        .connection
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE name = ?1",
            [NAME],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(indexes, 0);
    catalog
        .connection
        .execute_batch("DROP TRIGGER reject_preferred_index")
        .unwrap();
    migrate(&mut catalog.connection).unwrap();
    validate(&catalog.connection).unwrap();
}

#[test]
fn admission_rejects_missing_or_different_preferred_index() {
    for replacement in [
        "",
        "CREATE INDEX exact_duplicate_preferred_location ON exact_duplicate_decisions(digest)",
        "CREATE INDEX exact_duplicate_preferred_location
           ON exact_duplicate_decisions(preferred_location_id, preferred_asset_id)",
    ] {
        let (directory, mut catalog) = prior();
        migrate(&mut catalog.connection).unwrap();
        catalog
            .connection
            .execute_batch("DROP INDEX exact_duplicate_preferred_location")
            .unwrap();
        catalog.connection.execute_batch(replacement).unwrap();
        drop(catalog);
        let error = SqliteCatalog::open(directory.path().join("catalog.sqlite3"))
            .err()
            .expect("damaged index must fail admission");
        assert_eq!(error.code, "catalog_preferred_copy_index_invalid");
    }
}
