use super::*;

fn restore_v39(connection: &Connection) {
    downgrade_to_v39_for_test(connection);
}

#[test]
fn upgrade_preserves_existing_catalog_and_reopen_reuses_the_schema() {
    let mut connection = Connection::open_in_memory().unwrap();
    super::super::migrate_schema(&mut connection).unwrap();
    restore_v39(&connection);
    connection
        .execute("INSERT INTO assets VALUES ('retained', 123)", [])
        .unwrap();
    migrate(&mut connection).unwrap();
    validate(&connection).unwrap();
    super::super::migrate_schema(&mut connection).unwrap();
    let row = connection
        .query_row(
            "SELECT version, created_unix_ms FROM schema_info, assets",
            [],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
        )
        .unwrap();
    assert_eq!(row, (super::super::super::SCHEMA_VERSION, 123));
}

#[test]
fn conflicting_schema_rolls_back_without_advancing_the_version() {
    let mut connection = Connection::open_in_memory().unwrap();
    super::super::migrate_schema(&mut connection).unwrap();
    restore_v39(&connection);
    connection
        .execute_batch("CREATE TABLE content_fingerprints(unrelated TEXT)")
        .unwrap();
    assert_eq!(
        migrate(&mut connection).unwrap_err().code,
        "catalog_exact_analysis_contract_invalid"
    );
    let row = connection
        .query_row(
            "SELECT version,
                    (SELECT COUNT(*) FROM sqlite_master WHERE name = 'exact_analysis_runs')
             FROM schema_info",
            [],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
        )
        .unwrap();
    assert_eq!(row, (39, 0));
}
