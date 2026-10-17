use rusqlite::Connection;

use super::super::migrate_schema;

#[test]
fn current_v25_recovery_authority_is_allowlisted_and_bound_to_p2_work() {
    let mut connection = Connection::open_in_memory().expect("catalog");
    migrate_schema(&mut connection).expect("fresh current catalog");
    connection
        .execute_batch(
            "INSERT INTO library_change_queue(
               id, root_id, root_generation, intent_kind, scope, relative_path,
               origin, first_observed_unix_ms, most_recent_observed_unix_ms,
               first_sequence, most_recent_sequence, coalesced_observation_count,
               status, ready_unix_ms, catalog_revision_at_enqueue,
               created_unix_ms, updated_unix_ms
             ) VALUES
               (301, 'root-a', 1, 'freshness_unknown', 'root', '',
                'consistency_audit', 1, 1, '1', '1', 1, 'pending', 1, 0, 1, 1),
               (302, 'root-a', 1, 'reconcile', 'path', 'live.jpg',
                'live_notification', 1, 1, '2', '2', 1, 'pending', 1, 0, 1, 1);",
        )
        .expect("queue fixtures");

    let invalid_reason = connection
        .execute(
            "INSERT INTO library_recovery_authorities(
               change_id, run_id, root_id, root_generation, reason, authorized_unix_ms
             ) VALUES (301, 'run-invalid', 'root-a', 1, 'slow_queue', 1)",
            [],
        )
        .expect_err("non-allowlisted reason");
    assert!(
        invalid_reason
            .to_string()
            .contains("CHECK constraint failed")
    );
    let missing_boundary = connection
        .execute(
            "INSERT INTO library_recovery_authorities(
               change_id, run_id, root_id, root_generation, reason, authorized_unix_ms
             ) VALUES (301, 'run-baseline', 'root-a', 1,
                       'existing_root_baseline', 1)",
            [],
        )
        .expect_err("baseline requires opening boundary");
    assert!(
        missing_boundary
            .to_string()
            .contains("CHECK constraint failed")
    );
    let wrong_lane = connection
        .execute(
            "INSERT INTO library_recovery_authorities(
               change_id, run_id, root_id, root_generation, reason, authorized_unix_ms
             ) VALUES (302, 'run-live', 'root-a', 1, 'containment_failure', 1)",
            [],
        )
        .expect_err("P0 cannot own P2 authority");
    assert!(wrong_lane.to_string().contains("does not match P2"));
    connection
        .execute(
            "INSERT INTO library_recovery_authorities(
               change_id, run_id, root_id, root_generation, reason, authorized_unix_ms
             ) VALUES (301, 'run-valid', 'root-a', 1, 'containment_failure', 1)",
            [],
        )
        .expect("allowlisted P2 authority");
    let immutable = connection
        .execute(
            "UPDATE library_recovery_authorities
             SET reason = 'journal_gap' WHERE change_id = 301",
            [],
        )
        .expect_err("authority identity is immutable");
    assert!(immutable.to_string().contains("identity is immutable"));
    migrate_schema(&mut connection).expect("validate recovery authority contract");
}

#[test]
fn current_v25_rejects_weakened_recovery_authority_shape() {
    let mut connection = Connection::open_in_memory().expect("catalog");
    migrate_schema(&mut connection).expect("fresh current catalog");
    let original = connection
        .query_row(
            "SELECT sql FROM sqlite_master
             WHERE type = 'table' AND name = 'library_recovery_authorities'",
            [],
            |row| row.get::<_, String>(0),
        )
        .expect("canonical recovery authority DDL");
    let modified = original.replacen(
        "CHECK(retired_unix_ms IS NULL OR retired_unix_ms >= authorized_unix_ms)",
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
             WHERE type = 'table' AND name = 'library_recovery_authorities'",
            [modified],
        )
        .expect("weaken recovery authority CHECK");
    connection
        .execute_batch("PRAGMA writable_schema = OFF; PRAGMA schema_version = 1003")
        .expect("publish controlled schema mutation");

    let error = migrate_schema(&mut connection).expect_err("weakened authority must fail");
    assert_eq!(
        error.code,
        "catalog_recovery_authority_contract_unverifiable"
    );
}
