use super::*;

fn prior_catalog() -> Connection {
    let mut connection = Connection::open_in_memory().unwrap();
    connection
        .execute_batch("PRAGMA foreign_keys = ON")
        .unwrap();
    super::super::migrate_schema(&mut connection).unwrap();
    downgrade_to_v48_for_test(&connection);
    connection.execute_batch(
        "UPDATE organization_state SET revision = 3;
         INSERT INTO organization_albums VALUES ('first', 'First', 1, 1, 1), ('second', 'Second', 1, 1, 1);
         INSERT INTO organization_records VALUES
           ('a', 'first', 'organized', 0, 1, 0, 1, 1, NULL),
           ('b', 'second', 'organized', 1, 2, 3, 2, 3, NULL);
         INSERT INTO organization_record_groups VALUES ('a', 'a'), ('b', 'b');
         INSERT INTO organization_bindings VALUES
           ('a', 'asset-a', 1, 20, NULL, NULL, NULL, 'blake3-256-full-v1', printf('%064d', 1)),
           ('b', 'asset-b', 2, 20, NULL, NULL, NULL, 'blake3-256-full-v1', printf('%064d', 1));
         INSERT INTO organization_operations VALUES ('favorite', printf('%064d', 1), 'favorite', 3, 1, 3);
         INSERT INTO organization_operation_changes VALUES ('favorite', 'b', 'favorite', NULL, NULL, 0, 3);
         INSERT INTO organization_operation_targets VALUES ('favorite', 'b', 'favorite');"
    ).unwrap();
    connection
}

#[test]
fn upgrade_normalizes_known_content_and_preserves_conflict_provenance_favorites_and_history() {
    let mut connection = prior_catalog();
    migrate(&mut connection).unwrap();
    current_schema::validate_schema_version(&connection, 49).unwrap();
    let canonical: (Option<String>, String, bool, i64) = connection.query_row(
        "SELECT album_id, disposition, favorite, favorite_revision FROM organization_records WHERE record_id = 'a'", [],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
    ).unwrap();
    assert_eq!(canonical, (None, "deferred".into(), true, 3));
    assert_eq!(
        connection
            .query_row(
                "SELECT COUNT(DISTINCT group_id) FROM organization_record_groups",
                [],
                |row| row.get::<_, i64>(0)
            )
            .unwrap(),
        1
    );
    assert_eq!(
        connection
            .query_row(
                "SELECT COUNT(*) FROM organization_conflicts WHERE record_id = 'a'",
                [],
                |row| row.get::<_, i64>(0)
            )
            .unwrap(),
        2
    );
    assert_eq!(connection.query_row("SELECT COUNT(*) FROM organization_operation_changes WHERE operation_id = 'favorite' AND record_id = 'b'", [], |row| row.get::<_, i64>(0)).unwrap(), 1);
    let revision: i64 = connection
        .query_row("SELECT revision FROM catalog_state", [], |row| row.get(0))
        .unwrap();
    super::super::migrate_schema(&mut connection).unwrap();
    assert_eq!(
        connection
            .query_row("SELECT revision FROM catalog_state", [], |row| row
                .get::<_, i64>(0))
            .unwrap(),
        revision
    );
}

#[test]
fn failed_upgrade_preserves_previous_version_and_separate_decisions() {
    let mut connection = prior_catalog();
    connection.execute_batch("CREATE TRIGGER reject_conflict BEFORE INSERT ON organization_conflicts BEGIN SELECT RAISE(ABORT, 'injected migration failure'); END;").unwrap();
    assert!(migrate(&mut connection).is_err());
    assert_eq!(
        super::super::schema_shape::schema_version(&connection).unwrap(),
        48
    );
    assert_eq!(
        connection
            .query_row(
                "SELECT COUNT(DISTINCT group_id) FROM organization_record_groups",
                [],
                |row| row.get::<_, i64>(0)
            )
            .unwrap(),
        2
    );
    assert_eq!(
        connection
            .query_row(
                "SELECT album_id FROM organization_records WHERE record_id = 'a'",
                [],
                |row| row.get::<_, String>(0)
            )
            .unwrap(),
        "first"
    );
    connection
        .execute_batch("DROP TRIGGER reject_conflict")
        .unwrap();
    migrate(&mut connection).unwrap();
}

#[test]
fn full_validation_rejects_split_authority_for_the_same_content() {
    let connection = prior_catalog();
    assert_eq!(
        validate_rows(&connection).unwrap_err().code,
        "catalog_organization_reconciliation_invalid"
    );
}

#[test]
fn generation_indexes_are_required_and_supply_point_and_root_range_reads() {
    let mut connection = prior_catalog();
    migrate(&mut connection).unwrap();
    for (query, expected) in [
        (
            "SELECT asset_id FROM asset_locations WHERE source_generation = 1",
            "asset_locations_organization_generation",
        ),
        (
            "SELECT source_generation FROM asset_locations WHERE root_id = 'root' AND scan_id = 'scan' AND source_generation > 1 ORDER BY source_generation LIMIT 256",
            "asset_locations_organization_root_generation",
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
        assert!(!plan.contains("USE TEMP B-TREE"), "{plan}");
    }
    connection
        .execute_batch("DROP INDEX asset_locations_organization_generation")
        .unwrap();
    assert_eq!(
        validate(&connection).unwrap_err().code,
        "catalog_organization_reconciliation_invalid"
    );
}
