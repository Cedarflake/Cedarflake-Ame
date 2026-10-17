use rusqlite::Connection;

use super::super::migrate_schema;

#[test]
fn current_v25_rejects_weakened_lane_constraint_and_missing_lane_rows() {
    let mut connection = Connection::open_in_memory().expect("catalog");
    migrate_schema(&mut connection).expect("fresh current catalog");
    let original = connection
        .query_row(
            "SELECT sql FROM sqlite_master
             WHERE type = 'table' AND name = 'library_change_queue_lanes'",
            [],
            |row| row.get::<_, String>(0),
        )
        .expect("canonical lane DDL");
    let modified = original.replacen(
        "CHECK(lane IN ('p0_live', 'p1_journal', 'p2_recovery'))",
        "CHECK(1)",
        1,
    );
    assert_ne!(modified, original);
    connection
        .execute_batch("PRAGMA writable_schema = ON")
        .expect("enable controlled schema mutation");
    connection
        .execute(
            "UPDATE sqlite_master SET sql = ?1
             WHERE type = 'table' AND name = 'library_change_queue_lanes'",
            [modified],
        )
        .expect("weaken lane CHECK");
    connection
        .execute_batch("PRAGMA writable_schema = OFF; PRAGMA schema_version = 1002")
        .expect("publish controlled schema mutation");

    let error = migrate_schema(&mut connection).expect_err("weakened v25 CHECK must fail");
    assert_eq!(error.code, "catalog_change_lane_contract_unverifiable");

    let mut connection = Connection::open_in_memory().expect("second catalog");
    migrate_schema(&mut connection).expect("fresh current catalog");
    connection
        .execute_batch(
            "INSERT INTO library_change_queue(
               id, root_id, root_generation, intent_kind, scope, relative_path,
               origin, first_observed_unix_ms, most_recent_observed_unix_ms,
               first_sequence, most_recent_sequence, coalesced_observation_count,
               status, ready_unix_ms, catalog_revision_at_enqueue,
               created_unix_ms, updated_unix_ms
             ) VALUES (
               201, 'root-a', 1, 'reconcile', 'path', 'photo.jpg',
               'live_notification', 1, 1, '1', '1', 1, 'pending', 1, 0, 1, 1
             );
             DELETE FROM library_change_queue_lanes WHERE change_id = 201;",
        )
        .expect("missing lane fixture");
    let error = migrate_schema(&mut connection).expect_err("missing lane row must fail");
    assert_eq!(error.code, "catalog_change_lane_contract_unverifiable");
}
