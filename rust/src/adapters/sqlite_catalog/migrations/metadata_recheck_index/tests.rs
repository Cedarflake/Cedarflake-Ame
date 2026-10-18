use super::*;

fn retained_catalog() -> Connection {
    let mut connection = Connection::open_in_memory().unwrap();
    super::super::migrate_schema(&mut connection).unwrap();
    downgrade_to_v43_for_test(&connection);
    connection
        .execute_batch(
            "INSERT INTO assets VALUES ('retained-asset', 123);
         UPDATE catalog_state SET revision = 9;",
        )
        .unwrap();
    connection
}

#[test]
fn pending_metadata_index_upgrade_preserves_catalog_without_a_rescan() {
    let mut connection = retained_catalog();
    migrate(&mut connection).unwrap();
    let state: (i64, i64, i64, i64) = connection
        .query_row(
            "SELECT version, (SELECT user_version FROM pragma_user_version),
                (SELECT revision FROM catalog_state), (SELECT created_unix_ms FROM assets)
         FROM schema_info",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .unwrap();
    assert_eq!(state, (44, 44, 9, 123));
    validate(&connection).unwrap();
}

#[test]
fn conflicting_metadata_index_rolls_back_before_schema_advances() {
    let mut connection = retained_catalog();
    connection
        .execute_batch(
            "CREATE INDEX asset_locations_pending_metadata ON asset_locations(file_size);",
        )
        .unwrap();
    assert_eq!(
        migrate(&mut connection).unwrap_err().code,
        "catalog_metadata_recheck_index_invalid"
    );
    assert_eq!(
        super::super::schema_shape::schema_version(&connection).unwrap(),
        43
    );
    assert_eq!(
        connection
            .query_row("SELECT revision FROM catalog_state", [], |row| row
                .get::<_, i64>(0))
            .unwrap(),
        9
    );
}

#[test]
fn missing_metadata_index_is_rejected_by_current_schema_validation() {
    let mut connection = retained_catalog();
    migrate(&mut connection).unwrap();
    super::super::migrate_schema(&mut connection).unwrap();
    connection
        .execute_batch("DROP INDEX asset_locations_pending_metadata")
        .unwrap();
    assert_eq!(
        current_schema::validate_current_schema_structure(&connection)
            .unwrap_err()
            .code,
        "catalog_metadata_recheck_index_invalid"
    );
}
