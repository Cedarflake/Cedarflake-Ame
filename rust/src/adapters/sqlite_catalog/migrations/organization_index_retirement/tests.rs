use super::*;

fn prior_catalog() -> Connection {
    let mut connection = Connection::open_in_memory().unwrap();
    connection
        .execute_batch("PRAGMA foreign_keys = ON")
        .unwrap();
    super::super::migrate_schema(&mut connection).unwrap();
    downgrade_to_v49_for_test(&connection);
    connection
        .execute_batch(
            "INSERT INTO assets VALUES ('asset', 123);
             INSERT INTO organization_albums VALUES ('album', 'Album', 1, 1, 1);
             INSERT INTO organization_records VALUES
               ('record', 'album', 'organized', 1, 1, 2, 1, 2, NULL);
             INSERT INTO organization_record_groups VALUES ('record', 'record');
             INSERT INTO organization_bindings VALUES
               ('record', 'asset', 7, 20, NULL, NULL, NULL,
                'blake3-256-full-v1', printf('%064d', 1));
             INSERT INTO organization_operations VALUES
               ('favorite', printf('%064d', 1), 'favorite', 2, 1, 2);
             INSERT INTO organization_operation_changes VALUES
               ('favorite', 'record', 'favorite', NULL, NULL, 0, 2);
             INSERT INTO organization_operation_targets VALUES ('favorite', 'record', 'favorite');
             INSERT INTO exact_duplicate_decisions VALUES
               ('blake3-256-full-v1', printf('%064d', 1), 20, 'location', 'asset', 1, 456);
             UPDATE catalog_state SET revision = 9;
             UPDATE organization_state SET revision = 2;",
        )
        .unwrap();
    connection
}

fn retained_values(connection: &Connection) -> Vec<Vec<rusqlite::types::Value>> {
    let mut rows = Vec::new();
    for table in [
        "assets",
        "organization_albums",
        "organization_records",
        "organization_record_groups",
        "organization_bindings",
        "organization_operations",
        "organization_operation_changes",
        "organization_operation_targets",
        "exact_duplicate_decisions",
        "catalog_state",
        "organization_state",
    ] {
        let mut statement = connection
            .prepare(&format!("SELECT * FROM {table}"))
            .unwrap();
        let width = statement.column_count();
        rows.extend(
            statement
                .query_map([], |row| (0..width).map(|index| row.get(index)).collect())
                .unwrap()
                .collect::<rusqlite::Result<Vec<_>>>()
                .unwrap(),
        );
    }
    rows
}

#[test]
fn upgrade_preserves_catalog_decisions_bindings_and_revisions() {
    let mut connection = prior_catalog();
    let before = retained_values(&connection);
    migrate(&mut connection).unwrap();
    current_schema::validate_schema_version(&connection, 50).unwrap();
    assert_eq!(retained_values(&connection), before);
    super::super::migrate_schema(&mut connection).unwrap();
    assert_eq!(retained_values(&connection), before);
}

#[test]
fn interrupted_upgrade_restores_index_and_both_version_markers() {
    let mut connection = prior_catalog();
    let before = retained_values(&connection);
    connection
        .execute_batch(
            "CREATE TRIGGER reject_version BEFORE UPDATE ON schema_info
             BEGIN SELECT RAISE(ABORT, 'injected migration failure'); END;",
        )
        .unwrap();
    assert!(migrate(&mut connection).is_err());
    current_schema::validate_schema_version(&connection, 49).unwrap();
    let user_version: i64 = connection
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .unwrap();
    assert_eq!(user_version, 49);
    assert_eq!(retained_values(&connection), before);
    connection
        .execute_batch("DROP TRIGGER reject_version")
        .unwrap();
    migrate(&mut connection).unwrap();
}

#[test]
fn damaged_legacy_index_cannot_authorize_upgrade() {
    let mut connection = prior_catalog();
    let before = retained_values(&connection);
    connection
        .execute_batch(
            "DROP INDEX asset_locations_organization_generation;
             CREATE INDEX asset_locations_organization_generation ON asset_locations(file_size);",
        )
        .unwrap();
    assert_eq!(
        migrate(&mut connection).unwrap_err().code,
        "catalog_organization_reconciliation_invalid"
    );
    assert_eq!(schema_shape::schema_version(&connection).unwrap(), 49);
    assert_eq!(retained_values(&connection), before);
}

#[test]
fn current_contract_requires_retained_indexes_and_rejects_reintroduced_index() {
    for (modification, error) in [
        (
            GENERATION_INDEX.1,
            "catalog_organization_index_contract_invalid",
        ),
        (
            "DROP INDEX asset_locations_organization_root_generation",
            "catalog_organization_index_contract_invalid",
        ),
        (
            "DROP INDEX asset_locations_exact_source",
            "catalog_exact_analysis_contract_invalid",
        ),
    ] {
        let mut connection = prior_catalog();
        migrate(&mut connection).unwrap();
        connection.execute_batch(modification).unwrap();
        assert_eq!(
            current_schema::validate_schema_version(&connection, 50)
                .unwrap_err()
                .code,
            error,
            "{modification}"
        );
    }
}

#[test]
fn publication_retains_indexed_generation_and_root_keyset_reads() {
    let mut connection = prior_catalog();
    migrate(&mut connection).unwrap();
    for (query, expected) in [
        (
            "SELECT asset_id FROM asset_locations WHERE source_generation = 7",
            "asset_locations_exact_source",
        ),
        (
            "SELECT source_generation FROM asset_locations
             WHERE root_id = 'root' AND scan_id = 'scan' AND source_generation > 7
             ORDER BY source_generation LIMIT 256",
            ROOT_GENERATION_INDEX.0,
        ),
    ] {
        let plan = connection
            .prepare(&format!("EXPLAIN QUERY PLAN {query}"))
            .unwrap()
            .query_map([], |row| row.get::<_, String>(3))
            .unwrap()
            .collect::<rusqlite::Result<Vec<_>>>()
            .unwrap()
            .join("\n");
        assert!(plan.contains(expected), "{plan}");
        assert!(!plan.contains("SCAN asset_locations"), "{plan}");
        assert!(!plan.contains("USE TEMP B-TREE"), "{plan}");
    }
}
