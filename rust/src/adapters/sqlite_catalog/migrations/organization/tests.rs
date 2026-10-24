use super::*;

fn prior_catalog() -> Connection {
    let mut connection = Connection::open_in_memory().unwrap();
    connection
        .execute_batch("PRAGMA foreign_keys = ON")
        .unwrap();
    super::super::migrate_schema(&mut connection).unwrap();
    downgrade_to_v46_for_test(&connection);
    connection.execute_batch(
        "INSERT INTO assets VALUES ('retained-asset', 123);
         UPDATE catalog_state SET revision = 9;
         INSERT INTO exact_duplicate_decisions VALUES ('blake3-256-full-v1',
           'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa', 20, 'copy', 'retained-asset', 1, 123);"
    ).unwrap();
    connection
}

#[test]
fn upgrade_preserves_catalog_and_duplicate_choices_without_inventing_organization() {
    let mut connection = prior_catalog();
    migrate(&mut connection).unwrap();
    let actual: (i64, i64, i64, i64, i64, i64) = connection.query_row(
        "SELECT version, (SELECT user_version FROM pragma_user_version),
           (SELECT revision FROM catalog_state), (SELECT created_unix_ms FROM assets),
           (SELECT ignored FROM exact_duplicate_decisions), (SELECT COUNT(*) FROM organization_records)
         FROM schema_info", [], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?))
    ).unwrap();
    assert_eq!(actual, (47, 47, 9, 123, 1, 0));
    validate(&connection).unwrap();
}

#[test]
fn conflicting_schema_rolls_back_the_entire_migration() {
    let mut connection = prior_catalog();
    connection
        .execute_batch("CREATE TABLE organization_records(unrelated TEXT)")
        .unwrap();
    assert_eq!(
        migrate(&mut connection).unwrap_err().code,
        "catalog_organization_contract_invalid"
    );
    assert_eq!(
        super::super::schema_shape::schema_version(&connection).unwrap(),
        46
    );
    assert!(
        !connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name = 'organization_albums')",
                [],
                |row| row.get::<_, bool>(0)
            )
            .unwrap()
    );
    assert_eq!(
        connection
            .query_row("SELECT ignored FROM exact_duplicate_decisions", [], |row| {
                row.get::<_, i64>(0)
            })
            .unwrap(),
        1
    );
}

#[test]
fn derived_asset_removal_cannot_cascade_to_durable_decisions() {
    let mut connection = prior_catalog();
    migrate(&mut connection).unwrap();
    connection.execute_batch(
        "INSERT INTO organization_records VALUES ('record', NULL, 'deferred', 1, 1, 1, 1, 1, NULL);
         INSERT INTO organization_bindings VALUES ('record', 'retained-asset', 1, 20, NULL, NULL, NULL, NULL, NULL);
         DELETE FROM assets WHERE id = 'retained-asset';"
    ).unwrap();
    assert!(
        connection
            .query_row("SELECT favorite FROM organization_records", [], |row| row
                .get::<_, bool>(
                0
            ))
            .unwrap()
    );
    assert_eq!(
        connection
            .query_row("SELECT COUNT(*) FROM organization_bindings", [], |row| row
                .get::<_, i64>(
                0
            ))
            .unwrap(),
        1
    );
    assert!(
        connection
            .execute(
                "UPDATE organization_records SET disposition = 'organized'",
                []
            )
            .is_err()
    );
    assert!(
        connection
            .execute("UPDATE organization_records SET album_id = 'missing'", [])
            .is_err()
    );
}

#[test]
fn altered_organization_schema_is_rejected_before_current_queries() {
    let mut connection = prior_catalog();
    super::super::migrate_schema(&mut connection).unwrap();
    connection
        .execute_batch("DROP INDEX organization_bindings_content")
        .unwrap();
    assert_eq!(
        current_schema::validate_current_schema_structure(&connection)
            .unwrap_err()
            .code,
        "catalog_organization_contract_invalid"
    );
}

#[test]
fn orphaned_bindings_and_unpublished_revisions_fail_full_validation() {
    let mut connection = prior_catalog();
    migrate(&mut connection).unwrap();
    connection.execute_batch(
        "PRAGMA foreign_keys = OFF;
         INSERT INTO organization_bindings VALUES ('missing', 'asset', 1, 20, NULL, NULL, NULL, NULL, NULL);"
    ).unwrap();
    assert_eq!(
        validate_rows(&connection).unwrap_err().code,
        "catalog_organization_contract_invalid"
    );
    connection
        .execute_batch(
            "DELETE FROM organization_bindings;
         INSERT INTO organization_records VALUES ('record', NULL, 'pending', 1, 0, 1, 1, 1, NULL);",
        )
        .unwrap();
    assert_eq!(
        validate_rows(&connection).unwrap_err().code,
        "catalog_organization_contract_invalid"
    );
    connection
        .execute("UPDATE organization_state SET revision = 1", [])
        .unwrap();
    validate_rows(&connection).unwrap();
}
