use crate::adapters::sqlite_catalog::{SqliteCatalog, tests::publish_gallery_fixture};
use crate::ports::organization::OrganizationRepository;
use crate::ports::organization_review::OrganizationReviewRepository;

use super::*;

fn prior() -> (tempfile::TempDir, SqliteCatalog) {
    let directory = tempfile::tempdir().unwrap();
    let mut catalog = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
    publish_gallery_fixture(
        &mut catalog,
        "scan",
        "root",
        "C:\\Fixture",
        &[("a", None, 1)],
    );
    catalog.connection.execute_batch(
        "INSERT INTO organization_records VALUES ('retained', NULL, 'ignored', 1, 1, 1, 1, 1, NULL);
         INSERT INTO organization_record_groups VALUES ('retained', 'retained');
         UPDATE organization_state SET revision = 1;",
    ).unwrap();
    catalog
        .create_organization_album("create", 1, "Retained")
        .unwrap();
    downgrade_to_v59_for_test(&catalog.connection);
    (directory, catalog)
}

#[test]
fn upgrade_preserves_admissions_decisions_and_history_and_reopens() {
    let (directory, mut catalog) = prior();
    let history = catalog.load_organization_history().unwrap();
    migrate(&mut catalog.connection).unwrap();
    current_schema::validate_schema_version(&catalog.connection, 60).unwrap();
    assert_eq!(catalog.load_organization_admission_boundary().unwrap(), 1);
    assert_eq!(catalog.load_organization_history().unwrap(), history);
    let decision: (String, bool) = catalog
        .connection
        .query_row(
            "SELECT disposition, favorite FROM organization_records WHERE record_id = 'retained'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(decision, ("ignored".into(), true));
    assert_eq!(
        catalog.load_organization_albums(None).unwrap().albums[0].name,
        "Retained"
    );
    drop(catalog);
    let reopened = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
    assert_eq!(reopened.load_organization_admission_boundary().unwrap(), 1);
}

#[test]
fn failed_upgrade_rolls_back_clock_and_version_before_retry() {
    let (_directory, mut catalog) = prior();
    catalog
        .connection
        .execute_batch(
            "CREATE TRIGGER reject_clock BEFORE UPDATE ON schema_info
         WHEN NEW.version = 60 BEGIN SELECT RAISE(ABORT, 'injected failure'); END;",
        )
        .unwrap();
    assert!(migrate(&mut catalog.connection).is_err());
    current_schema::validate_schema_version(&catalog.connection, 59).unwrap();
    let tables: i64 = catalog
        .connection
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE name = ?1",
            [NAME],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(tables, 0);
    catalog
        .connection
        .execute_batch("DROP TRIGGER reject_clock")
        .unwrap();
    migrate(&mut catalog.connection).unwrap();
    validate_rows(&catalog.connection).unwrap();
}

#[test]
fn admission_rejects_missing_altered_or_out_of_range_clocks() {
    for damage in [
        "DROP TABLE organization_admission_clock;",
        "DELETE FROM organization_admission_clock;",
        "UPDATE organization_admission_clock SET restored_prefix = 2;",
        "DROP TABLE organization_admission_clock; CREATE TABLE organization_admission_clock(singleton INTEGER, restored_prefix INTEGER); INSERT INTO organization_admission_clock VALUES (1, 0);",
    ] {
        let (directory, mut catalog) = prior();
        migrate(&mut catalog.connection).unwrap();
        catalog.connection.execute_batch(damage).unwrap();
        drop(catalog);
        let error = SqliteCatalog::open(directory.path().join("catalog.sqlite3"))
            .err()
            .unwrap();
        assert_eq!(error.code, "catalog_admission_clock_invalid");
    }
}
