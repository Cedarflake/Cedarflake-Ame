use crate::adapters::sqlite_catalog::SqliteCatalog;

use super::*;

fn prior() -> (tempfile::TempDir, SqliteCatalog) {
    let directory = tempfile::tempdir().unwrap();
    let catalog = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
    downgrade_to_v56_for_test(&catalog.connection);
    catalog.connection.execute_batch(
        "INSERT INTO organization_albums VALUES ('album', 'Retained', 1, 1, 1);
         INSERT INTO organization_records VALUES ('record', 'album', 'organized', 1, 1, 1, 1, 1, NULL);
         INSERT INTO organization_record_groups VALUES ('record', 'record');
         UPDATE organization_state SET revision = 1;
         INSERT INTO organization_review_scopes VALUES ('bookmark', 'library', NULL, NULL, NULL, 1, '', 'pending');
         INSERT INTO organization_review_bookmark VALUES (1, 'bookmark', 'record', 42, 'root', 'location', 1, 1);",
    ).unwrap();
    (directory, catalog)
}

#[test]
fn upgrade_preserves_decisions_and_converts_the_existing_admission_cursor() {
    let (directory, mut catalog) = prior();
    migrate(&mut catalog.connection).unwrap();
    current_schema::validate_schema_version(&catalog.connection, 57).unwrap();
    let retained: bool = catalog.connection.query_row(
        "SELECT album_id = 'album' AND disposition = 'organized' AND favorite = 1 FROM organization_records",
        [], |row| row.get(0),
    ).unwrap();
    assert!(retained);
    let order: (String, String, i64, String, i64) = catalog
        .connection
        .query_row(
            "SELECT sort_key, sort_direction, primary_missing, primary_text, primary_number
         FROM organization_review_sort, organization_review_cursor",
            [],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            },
        )
        .unwrap();
    assert_eq!(
        order,
        (
            "library_admission".into(),
            "descending".into(),
            0,
            String::new(),
            42
        )
    );
    drop(catalog);
    SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
}

#[test]
fn failed_upgrade_rolls_back_and_can_be_retried() {
    let (_directory, mut catalog) = prior();
    catalog
        .connection
        .execute_batch(
            "CREATE TRIGGER reject_upgrade BEFORE UPDATE ON schema_info
         WHEN NEW.version = 57 BEGIN SELECT RAISE(ABORT, 'injected failure'); END;",
        )
        .unwrap();
    assert!(migrate(&mut catalog.connection).is_err());
    assert_eq!(
        schema_shape::schema_version(&catalog.connection).unwrap(),
        56
    );
    catalog
        .connection
        .execute_batch("DROP TRIGGER reject_upgrade")
        .unwrap();
    migrate(&mut catalog.connection).unwrap();
    catalog
        .connection
        .execute_batch("DELETE FROM organization_review_cursor")
        .unwrap();
    assert_eq!(
        validate_rows(&catalog.connection).unwrap_err().code,
        "catalog_organization_review_order_invalid"
    );
}
