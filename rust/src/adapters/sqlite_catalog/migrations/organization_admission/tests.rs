use crate::adapters::sqlite_catalog::SqliteCatalog;

use super::*;

fn prior_catalog() -> (tempfile::TempDir, SqliteCatalog) {
    let directory = tempfile::tempdir().unwrap();
    let mut catalog = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
    crate::adapters::sqlite_catalog::tests::publish_gallery_fixture(
        &mut catalog,
        "scan",
        "root",
        "C:\\Fixture",
        &[("first", None, 100), ("second", None, 1)],
    );
    catalog
        .connection
        .execute_batch(
            "INSERT INTO organization_records
             VALUES ('retained', NULL, 'ignored', 1, 1, 1, 1, 1, NULL);
             INSERT INTO organization_record_groups VALUES ('retained', 'retained');
             UPDATE organization_state SET revision = 1;",
        )
        .unwrap();
    downgrade_to_v50_for_test(&catalog.connection);
    (directory, catalog)
}

#[test]
fn upgrade_seeds_visible_versions_without_changing_catalog_or_decisions() {
    let (_directory, mut catalog) = prior_catalog();
    let before: i64 = catalog
        .connection
        .query_row("SELECT revision FROM catalog_state", [], |row| row.get(0))
        .unwrap();
    migrate(&mut catalog.connection).unwrap();
    current_schema::validate_schema_version(&catalog.connection, 51).unwrap();
    let state: (i64, i64, bool) = catalog
        .connection
        .query_row(
            "SELECT (SELECT COUNT(*) FROM organization_admissions), revision,
               (SELECT disposition = 'ignored' AND favorite = 1
                FROM organization_records WHERE record_id = 'retained')
             FROM catalog_state",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap();
    assert_eq!(state, (2, before, true));
    let sources: i64 = catalog
        .connection
        .query_row(
            "SELECT COUNT(*) FROM organization_admission_sources",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(sources, 2);
    super::super::migrate_schema(&mut catalog.connection).unwrap();
    assert_eq!(
        catalog
            .connection
            .query_row(
                "SELECT MAX(admission_id) FROM organization_admissions",
                [],
                |row| row.get::<_, i64>(0)
            )
            .unwrap(),
        2
    );
}

#[test]
fn failed_upgrade_rolls_back_admission_schema_and_all_seeded_rows() {
    let (_directory, mut catalog) = prior_catalog();
    catalog
        .connection
        .execute_batch(
            "CREATE TRIGGER reject_version BEFORE UPDATE ON schema_info
         BEGIN SELECT RAISE(ABORT, 'injected migration failure'); END;",
        )
        .unwrap();
    assert!(migrate(&mut catalog.connection).is_err());
    current_schema::validate_schema_version(&catalog.connection, 50).unwrap();
    let tables: i64 = catalog
        .connection
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE name IN
             ('organization_admissions', 'organization_admission_roots',
              'organization_admission_sources')",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(tables, 0);
    catalog
        .connection
        .execute_batch("DROP TRIGGER reject_version")
        .unwrap();
    migrate(&mut catalog.connection).unwrap();
}

#[test]
fn current_contract_rejects_missing_root_index_and_unowned_admissions() {
    let (_directory, mut catalog) = prior_catalog();
    migrate(&mut catalog.connection).unwrap();
    catalog
        .connection
        .execute_batch("DELETE FROM organization_admission_sources")
        .unwrap();
    assert_eq!(
        validate_rows(&catalog.connection).unwrap_err().code,
        "catalog_organization_admission_invalid"
    );
    catalog
        .connection
        .execute_batch("DROP INDEX organization_admission_sources_root")
        .unwrap();
    assert_eq!(
        validate(&catalog.connection).unwrap_err().code,
        "catalog_organization_admission_invalid"
    );
}
