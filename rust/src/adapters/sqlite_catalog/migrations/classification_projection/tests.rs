use crate::adapters::sqlite_catalog::organization::tests::fixture;

use super::*;

#[test]
fn projection_upgrade_preserves_rows_and_failure_rolls_back_both_views() {
    let (_directory, mut catalog) = fixture();
    downgrade_to_v69_for_test(&catalog.connection);
    catalog
        .connection
        .execute_batch(
            "CREATE TRIGGER reject_projection_upgrade BEFORE UPDATE OF version ON schema_info
         WHEN NEW.version = 70 BEGIN SELECT RAISE(ABORT, 'injected projection failure'); END;",
        )
        .unwrap();
    assert!(
        migrate(&mut catalog.connection)
            .unwrap_err()
            .message
            .contains("injected projection failure")
    );
    assert_eq!(
        schema_shape::schema_version(&catalog.connection).unwrap(),
        69
    );
    for (_, name, _) in OBJECTS {
        let present: bool = catalog
            .connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name = ?1)",
                [name],
                |row| row.get(0),
            )
            .unwrap();
        assert!(!present);
    }
    catalog
        .connection
        .execute_batch("DROP TRIGGER reject_projection_upgrade")
        .unwrap();
    migrate(&mut catalog.connection).unwrap();
    validate(&catalog.connection).unwrap();
    let counts: (i64, i64) = catalog
        .connection
        .query_row(
            "SELECT (SELECT count(*) FROM asset_locations), (SELECT count(*) FROM library_roots)",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(counts, (3, 2));
}

#[test]
fn altered_projection_is_rejected_by_schema_validation() {
    let (_directory, catalog) = fixture();
    catalog
        .connection
        .execute_batch(
            "DROP VIEW classification_human_projection;
         CREATE VIEW classification_human_projection AS SELECT NULL AS category_id;",
        )
        .unwrap();
    assert_eq!(
        validate(&catalog.connection).unwrap_err().code,
        "classification_projection_invalid"
    );
}
