use crate::adapters::sqlite_catalog::SqliteCatalog;

use super::*;

#[test]
fn organization_restoration_missing_current_history_is_not_legacy_state() {
    let (_directory, catalog) = prior();

    catalog
        .connection
        .execute_batch("DROP TABLE organization_recovery_admissions;")
        .unwrap();
    assert!(
        crate::adapters::sqlite_catalog::organization::publication::root(
            &catalog.connection,
            "unavailable-root"
        )
        .is_err()
    );
    assert_eq!(
        validate(&catalog.connection).unwrap_err().code,
        "catalog_organization_restoration_invalid"
    );
}

fn prior() -> (tempfile::TempDir, SqliteCatalog) {
    let directory = tempfile::tempdir().unwrap();
    let catalog = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
    catalog.connection.execute_batch(
        "INSERT INTO organization_records VALUES ('retained', NULL, 'ignored', 1, 1, 1, 1, 1, 10);
         INSERT INTO organization_record_groups VALUES ('retained', 'retained');
         UPDATE organization_state SET revision = 1;",
    ).unwrap();

    (directory, catalog)
}
