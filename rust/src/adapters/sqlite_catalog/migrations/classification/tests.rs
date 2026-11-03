use crate::adapters::sqlite_catalog::SqliteCatalog;
use crate::ports::organization::OrganizationRepository;

use super::*;

#[test]
fn upgrade_preserves_r4_decisions_and_has_no_initial_model() {
    let directory = tempfile::tempdir().unwrap();
    let mut catalog = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
    catalog
        .create_organization_album("retained-album", 0, "Retained")
        .unwrap();
    downgrade_to_v64_for_test(&catalog.connection);
    migrate(&mut catalog.connection).unwrap();
    let (albums, revision, active): (i64, i64, Option<String>) = catalog.connection.query_row(
        "SELECT (SELECT count(*) FROM organization_albums), revision, active_run_id FROM classification_state",
        [], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    ).unwrap();
    assert_eq!((albums, revision, active), (1, 0, None));
    drop(catalog);
    SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
}

#[test]
fn failed_upgrade_rolls_back_every_classification_table() {
    let directory = tempfile::tempdir().unwrap();
    let mut catalog = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
    downgrade_to_v64_for_test(&catalog.connection);
    catalog.connection.execute_batch("CREATE TRIGGER reject_classification_version BEFORE UPDATE OF version ON schema_info WHEN NEW.version = 65 BEGIN SELECT RAISE(ABORT, 'injected'); END;").unwrap();
    assert!(migrate(&mut catalog.connection).is_err());
    assert_eq!(
        schema_shape::schema_version(&catalog.connection).unwrap(),
        64
    );
    let tables: i64 = catalog.connection.query_row(
        "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name LIKE 'classification_%'", [], |row| row.get(0),
    ).unwrap();
    assert_eq!(tables, 0);
    catalog
        .connection
        .execute_batch("DROP TRIGGER reject_classification_version")
        .unwrap();
    migrate(&mut catalog.connection).unwrap();
    validate_rows(&catalog.connection).unwrap();
}
