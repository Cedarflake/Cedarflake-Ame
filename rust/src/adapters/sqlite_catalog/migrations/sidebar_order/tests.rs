use crate::adapters::sqlite_catalog::SqliteCatalog;

use super::*;

fn prior() -> (tempfile::TempDir, SqliteCatalog) {
    let directory = tempfile::tempdir().unwrap();
    let catalog = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
    downgrade_to_v57_for_test(&catalog.connection);
    catalog.connection.execute_batch(
        "INSERT INTO organization_albums VALUES ('c', 'Third', 1, 1, 1), ('a', 'First', 1, 2, 2);
         INSERT INTO library_roots VALUES
           ('c', 'C:\\Third', NULL, 2), ('d', 'C:\\Second', NULL, 1), ('a', 'C:\\First', NULL, 1);",
    ).unwrap();
    (directory, catalog)
}

#[test]
fn upgrade_preserves_previous_positions_and_appends_new_items() {
    let (directory, mut catalog) = prior();
    migrate(&mut catalog.connection).unwrap();
    current_schema::validate_schema_version(&catalog.connection, 58).unwrap();
    for (table, key, expected_order) in [
        ("library_sidebar_root_order", "root_id", "a,d,c"),
        ("organization_sidebar_album_order", "album_id", "a,c"),
    ] {
        let order: String = catalog
            .connection
            .query_row(
                &format!(
                    "SELECT GROUP_CONCAT({key}) FROM (SELECT {key} FROM {table} ORDER BY position)"
                ),
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(order, expected_order);
    }
    catalog
        .connection
        .execute_batch(
            "INSERT INTO organization_albums VALUES ('b', 'Added', 1, 0, 0);
         INSERT INTO library_roots VALUES ('b', 'C:\\Added', NULL, 0);",
        )
        .unwrap();
    let positions: (i64, i64) = catalog
        .connection
        .query_row(
            "SELECT (SELECT position FROM organization_sidebar_album_order WHERE album_id = 'b'),
                (SELECT position FROM library_sidebar_root_order WHERE root_id = 'b')",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(positions, (2, 3));
    drop(catalog);
    SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
}

#[test]
fn failed_upgrade_rolls_back_before_retry() {
    let (_directory, mut catalog) = prior();
    catalog
        .connection
        .execute_batch(
            "CREATE TRIGGER reject_upgrade BEFORE UPDATE ON schema_info WHEN NEW.version = 58
         BEGIN SELECT RAISE(ABORT, 'injected failure'); END;",
        )
        .unwrap();
    assert!(migrate(&mut catalog.connection).is_err());
    assert_eq!(
        schema_shape::schema_version(&catalog.connection).unwrap(),
        57
    );
    let tables: i64 = catalog
        .connection
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE name = 'organization_sidebar_album_order'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(tables, 0);
    catalog
        .connection
        .execute_batch("DROP TRIGGER reject_upgrade")
        .unwrap();
    migrate(&mut catalog.connection).unwrap();
}

#[test]
fn admission_rejects_missing_positions_duplicate_positions_and_changed_triggers() {
    let (_directory, mut catalog) = prior();
    migrate(&mut catalog.connection).unwrap();
    catalog
        .connection
        .execute_batch("DELETE FROM library_sidebar_root_order WHERE root_id = 'a'")
        .unwrap();
    assert_eq!(
        validate_rows(&catalog.connection).unwrap_err().code,
        "catalog_sidebar_order_invalid"
    );
    catalog
        .connection
        .execute_batch(
            "INSERT INTO library_sidebar_root_order VALUES ('a', 0);
         UPDATE organization_sidebar_album_order SET position = 0;",
        )
        .unwrap();
    assert_eq!(
        validate_rows(&catalog.connection).unwrap_err().code,
        "catalog_sidebar_order_invalid"
    );
    catalog
        .connection
        .execute_batch("DROP TRIGGER organization_sidebar_album_append")
        .unwrap();
    assert_eq!(
        validate(&catalog.connection).unwrap_err().code,
        "catalog_sidebar_order_invalid"
    );
}
