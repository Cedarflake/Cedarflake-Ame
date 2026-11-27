use crate::adapters::sqlite_catalog::SqliteCatalog;

use super::*;

#[test]
fn current_contract_rejects_missing_root_index_and_unowned_admissions() {
    let (_directory, catalog) = prior_catalog();

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

    (directory, catalog)
}
