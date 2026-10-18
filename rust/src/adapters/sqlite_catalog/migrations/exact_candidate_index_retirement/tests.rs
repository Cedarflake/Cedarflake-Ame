use super::*;

fn legacy_catalog() -> Connection {
    let mut connection = Connection::open_in_memory().unwrap();
    super::super::migrate_schema(&mut connection).unwrap();
    downgrade_to_v42_for_test(&connection);
    connection
        .execute_batch(
            "INSERT INTO assets VALUES ('retained-asset', 123);
             INSERT INTO exact_analysis_runs VALUES
               ('retained-run', 'blake3-256-full-v1', 'blake3', '1', 'full-file;output=256', 1);
             INSERT INTO content_fingerprints VALUES
               (7, 'blake3-256-full-v1', printf('%064d', 1), 20, NULL, NULL, NULL, 'retained-run');
             INSERT INTO gallery_content_fingerprints VALUES (7, 'blake3-256-full-v1');
             INSERT INTO exact_duplicate_decisions VALUES
               ('blake3-256-full-v1', printf('%064d', 1), 20, 'retained-location',
                'retained-asset', 1, 456);
             UPDATE catalog_state SET revision = 9;",
        )
        .unwrap();
    connection
}

#[test]
fn upgrade_retires_only_redundant_index_and_preserves_catalog_evidence_and_choices() {
    let mut connection = legacy_catalog();
    migrate(&mut connection).unwrap();
    super::super::migrate_schema(&mut connection).unwrap();
    let state: (i64, i64, i64, i64, i64, i64) = connection
        .query_row(
            "SELECT version, (SELECT user_version FROM pragma_user_version),
                    (SELECT revision FROM catalog_state),
                    (SELECT COUNT(*) FROM content_fingerprints),
                    (SELECT COUNT(*) FROM gallery_content_fingerprints),
                    (SELECT COUNT(*) FROM sqlite_master WHERE name = 'asset_locations_exact_cursor')
             FROM schema_info",
            [],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                ))
            },
        )
        .unwrap();
    assert_eq!(state, (44, 44, 9, 1, 1, 0));
    let decision: (String, String, i64, i64) = connection
        .query_row(
            "SELECT preferred_location_id, preferred_asset_id, ignored, updated_unix_ms
             FROM exact_duplicate_decisions",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .unwrap();
    assert_eq!(
        decision,
        ("retained-location".into(), "retained-asset".into(), 1, 456)
    );
    assert_eq!(
        connection
            .query_row("SELECT created_unix_ms FROM assets", [], |row| row
                .get::<_, i64>(0))
            .unwrap(),
        123
    );
}

#[test]
fn invalid_legacy_index_rolls_back_without_advancing_schema_or_erasing_decisions() {
    let mut connection = legacy_catalog();
    connection
        .execute_batch(
            "DROP INDEX asset_locations_exact_cursor;
             CREATE INDEX asset_locations_exact_cursor ON asset_locations(file_size);",
        )
        .unwrap();
    assert_eq!(
        migrate(&mut connection).unwrap_err().code,
        "catalog_exact_analysis_contract_invalid"
    );
    let state: (i64, i64, i64) = connection
        .query_row(
            "SELECT version, (SELECT user_version FROM pragma_user_version),
                    (SELECT COUNT(*) FROM exact_duplicate_decisions) FROM schema_info",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap();
    assert_eq!(state, (42, 42, 1));
}

#[test]
fn current_schema_rejects_reintroduced_redundant_index() {
    let mut connection = legacy_catalog();
    migrate(&mut connection).unwrap();
    connection
        .execute_batch(
            "CREATE INDEX asset_locations_exact_cursor
               ON asset_locations(location_id, root_id, scan_id, file_size, source_generation);",
        )
        .unwrap();
    assert_eq!(
        current_schema::validate_schema_version(&connection, 43)
            .unwrap_err()
            .code,
        "catalog_exact_candidate_index_contract_invalid"
    );
}
