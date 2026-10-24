use crate::adapters::sqlite_catalog::SqliteCatalog;

use super::*;

fn prior() -> (tempfile::TempDir, SqliteCatalog) {
    let directory = tempfile::tempdir().unwrap();
    let catalog = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
    catalog.connection.execute_batch(
        "INSERT INTO organization_records VALUES ('retained', NULL, 'ignored', 1, 1, 1, 1, 1, 10);
         INSERT INTO organization_record_groups VALUES ('retained', 'retained');
         UPDATE organization_state SET revision = 1;",
    ).unwrap();
    downgrade_to_v53_for_test(&catalog.connection);
    (directory, catalog)
}

#[test]
fn organization_restoration_upgrade_preserves_existing_decisions_without_catalog_work() {
    let (directory, mut catalog) = prior();
    let before: i64 = catalog
        .connection
        .query_row("SELECT revision FROM catalog_state", [], |row| row.get(0))
        .unwrap();
    migrate(&mut catalog.connection).unwrap();
    current_schema::validate_schema_version(&catalog.connection, 54).unwrap();
    assert_eq!(
        catalog
            .connection
            .query_row("SELECT revision FROM catalog_state", [], |row| row
                .get::<_, i64>(0))
            .unwrap(),
        before
    );
    assert_eq!(catalog.connection.query_row(
        "SELECT COUNT(*) FROM organization_records WHERE favorite = 1 AND disposition = 'ignored' AND unassociated_since_unix_ms = 10", [], |row| row.get::<_, i64>(0),
    ).unwrap(), 1);
    drop(catalog);
    SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
}

#[test]
fn organization_restoration_upgrade_failure_retires_every_new_object_and_allows_retry() {
    let (_directory, mut catalog) = prior();
    catalog.connection.execute_batch("CREATE TRIGGER fail_version BEFORE UPDATE ON schema_info BEGIN SELECT RAISE(ABORT, 'injected migration failure'); END;").unwrap();
    assert!(migrate(&mut catalog.connection).is_err());
    current_schema::validate_schema_version(&catalog.connection, 53).unwrap();
    for (_, name, _) in schema::OBJECTS {
        assert!(
            !catalog
                .connection
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name = ?1)",
                    [name],
                    |row| row.get::<_, bool>(0)
                )
                .unwrap(),
            "{name}"
        );
    }
    catalog
        .connection
        .execute_batch("DROP TRIGGER fail_version")
        .unwrap();
    migrate(&mut catalog.connection).unwrap();
}

#[test]
fn organization_restoration_schema_rejects_changed_queue_projection() {
    let (_directory, mut catalog) = prior();
    migrate(&mut catalog.connection).unwrap();
    catalog.connection.execute_batch("DROP VIEW organization_queue_versions; CREATE VIEW organization_queue_versions AS SELECT admission_id, source_generation FROM organization_admissions;").unwrap();
    assert_eq!(
        validate(&catalog.connection).unwrap_err().code,
        "catalog_organization_restoration_invalid"
    );
}

#[test]
fn organization_restoration_missing_current_history_is_not_legacy_state() {
    let (_directory, mut catalog) = prior();
    migrate(&mut catalog.connection).unwrap();
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

#[test]
fn organization_restoration_rows_reject_unowned_queue_history() {
    let (_directory, mut catalog) = prior();
    migrate(&mut catalog.connection).unwrap();
    catalog
        .connection
        .execute_batch("INSERT INTO organization_recovery_admissions VALUES (100, 1);")
        .unwrap();
    assert_eq!(
        validate_rows(&catalog.connection).unwrap_err().code,
        "catalog_organization_restoration_invalid"
    );
    catalog
        .connection
        .execute_batch(
            "INSERT INTO organization_recovery_admission_sources VALUES (100, 'unknown-source');",
        )
        .unwrap();
    assert_eq!(
        validate_rows(&catalog.connection).unwrap_err().code,
        "catalog_organization_restoration_invalid"
    );
}
