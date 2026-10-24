use crate::adapters::sqlite_catalog::SqliteCatalog;

use super::*;

fn prior_catalog() -> (tempfile::TempDir, SqliteCatalog) {
    let directory = tempfile::tempdir().unwrap();
    let catalog = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
    catalog
        .connection
        .execute_batch(
            "INSERT INTO organization_records VALUES ('retained', NULL, 'ignored', 1, 1, 1, 1, 1, NULL);
             INSERT INTO organization_record_groups VALUES ('retained', 'retained');
             UPDATE organization_state SET revision = 1;",
        )
        .unwrap();
    downgrade_to_v51_for_test(&catalog.connection);
    (directory, catalog)
}

#[test]
fn upgrade_preserves_decisions_and_revisions_without_manufacturing_review_activity() {
    let (directory, mut catalog) = prior_catalog();
    let before: i64 = catalog
        .connection
        .query_row("SELECT revision FROM catalog_state", [], |row| row.get(0))
        .unwrap();
    migrate(&mut catalog.connection).unwrap();
    current_schema::validate_schema_version(&catalog.connection, 52).unwrap();
    let result: (i64, i64, bool) = catalog
        .connection
        .query_row(
            "SELECT (SELECT COUNT(*) FROM organization_review_bookmark), revision,
               (SELECT favorite = 1 AND disposition = 'ignored'
                FROM organization_records WHERE record_id = 'retained')
             FROM catalog_state",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap();
    assert_eq!(result, (0, before, true));
    drop(catalog);
    SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
}

#[test]
fn failed_upgrade_rolls_back_every_review_table() {
    let (_directory, mut catalog) = prior_catalog();
    catalog
        .connection
        .execute_batch(
            "CREATE TRIGGER reject_version BEFORE UPDATE ON schema_info
             BEGIN SELECT RAISE(ABORT, 'injected migration failure'); END;",
        )
        .unwrap();
    assert!(migrate(&mut catalog.connection).is_err());
    current_schema::validate_schema_version(&catalog.connection, 51).unwrap();
    let count: i64 = catalog
        .connection
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE name LIKE 'organization_review_%'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 0);
    catalog
        .connection
        .execute_batch("DROP TRIGGER reject_version")
        .unwrap();
    migrate(&mut catalog.connection).unwrap();
}

#[test]
fn missing_schema_and_orphaned_bookmarks_are_rejected() {
    let (_directory, mut catalog) = prior_catalog();
    migrate(&mut catalog.connection).unwrap();
    catalog
        .connection
        .execute_batch(
            "PRAGMA foreign_keys = OFF;
             INSERT INTO organization_review_scopes
               VALUES ('bookmark', 'library', NULL, NULL, NULL, 1, '', 'pending');
             INSERT INTO organization_review_bookmark
               VALUES (1, 'bookmark', 'missing', 1, 'root', 'location', 0, 0);
             PRAGMA foreign_keys = ON;",
        )
        .unwrap();
    assert_eq!(
        validate_rows(&catalog.connection).unwrap_err().code,
        "catalog_organization_review_invalid"
    );
    catalog
        .connection
        .execute_batch(
            "DELETE FROM organization_review_bookmark; DROP TABLE organization_review_session;",
        )
        .unwrap();
    assert_eq!(
        validate(&catalog.connection).unwrap_err().code,
        "catalog_organization_review_invalid"
    );
}
