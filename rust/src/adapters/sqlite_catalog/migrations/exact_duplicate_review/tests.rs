use super::*;

#[test]
fn migration_preserves_catalog_and_choices_when_reentered() {
    let mut connection = Connection::open_in_memory().unwrap();
    super::super::migrate_schema(&mut connection).unwrap();
    super::super::exact_candidate_index_retirement::downgrade_to_v42_for_test(&connection);
    connection
        .execute_batch(
            "INSERT INTO assets VALUES ('retained', 123);
         INSERT INTO exact_duplicate_decisions VALUES ('scheme',
           'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',
           20, 'location', 'retained', 1, 123);
         UPDATE schema_info SET version = 41; PRAGMA user_version = 41;",
        )
        .unwrap();
    migrate(&mut connection).unwrap();
    super::super::migrate_schema(&mut connection).unwrap();
    let actual: (i64, String, bool) = connection
        .query_row(
            "SELECT assets.created_unix_ms, decisions.preferred_location_id, decisions.ignored
         FROM assets, exact_duplicate_decisions AS decisions",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap();
    assert_eq!(actual, (123, "location".to_owned(), true));
}

#[test]
fn version_41_without_choices_upgrades_without_advancing_gallery_revision() {
    let mut connection = Connection::open_in_memory().unwrap();
    super::super::migrate_schema(&mut connection).unwrap();
    super::super::exact_candidate_index_retirement::downgrade_to_v42_for_test(&connection);
    let before: i64 = connection
        .query_row("SELECT revision FROM catalog_state", [], |row| row.get(0))
        .unwrap();
    connection
        .execute_batch(
            "DROP TABLE exact_duplicate_decisions;
        UPDATE schema_info SET version = 41; PRAGMA user_version = 41;",
        )
        .unwrap();
    migrate(&mut connection).unwrap();
    let actual: (i64, i64) = connection
        .query_row(
            "SELECT version, revision FROM schema_info, catalog_state",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(actual, (42, before));
}

#[test]
fn malformed_choice_table_cannot_replace_user_data_or_advance_schema() {
    let mut connection = Connection::open_in_memory().unwrap();
    super::super::migrate_schema(&mut connection).unwrap();
    super::super::exact_candidate_index_retirement::downgrade_to_v42_for_test(&connection);
    connection
        .execute_batch(
            "DROP TABLE exact_duplicate_decisions;
        CREATE TABLE exact_duplicate_decisions(value TEXT);
        INSERT INTO exact_duplicate_decisions VALUES ('preserve me');
        UPDATE schema_info SET version = 41; PRAGMA user_version = 41;",
        )
        .unwrap();
    assert_eq!(
        migrate(&mut connection).unwrap_err().code,
        "catalog_duplicate_decisions_contract_invalid"
    );
    let actual: (i64, String) = connection
        .query_row(
            "SELECT version, value FROM schema_info, exact_duplicate_decisions",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(actual, (41, "preserve me".to_owned()));
}
