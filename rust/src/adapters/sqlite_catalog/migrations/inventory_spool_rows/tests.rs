use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use rusqlite::{Connection, params};

use super::super::migrate_schema;
use super::validate;

fn fixture(directory_count: u32, entries_per_directory: u32) -> Connection {
    let mut connection = Connection::open_in_memory().expect("isolated catalog");
    connection
        .execute_batch("PRAGMA foreign_keys = ON")
        .expect("foreign keys");
    migrate_schema(&mut connection).expect("production schema");
    let transaction = connection.transaction().expect("fixture transaction");
    for (run_id, authority_id) in [("run-a", 1), ("run-b", 2)] {
        transaction
            .execute(
                "INSERT INTO library_roots(id, path, created_unix_ms) VALUES (?1, ?2, 1)",
                params![run_id, format!("C:/generated-spool-contract/{run_id}")],
            )
            .expect("fixture root without filesystem access");
        transaction
            .execute(
                "INSERT INTO library_change_root_state(root_id, generation, is_active, updated_unix_ms)
                 VALUES (?1, 1, 1, 1)",
                [run_id],
            )
            .expect("independent active root");
        transaction
            .execute(
                "INSERT INTO library_metadata_inventory_runs(
                   id, root_id, root_generation, epoch, scope_kind, scope_relative_path,
                   status, next_page_index, staged_entry_count, enumeration_complete,
                   absence_authority, started_unix_ms, updated_unix_ms
                 ) VALUES (?1, ?1, 1, ?2, 'root', '', 'running', 1, 0, 0, 0, 1, 1)",
                params![run_id, authority_id],
            )
            .expect("fixture run");
        transaction
            .execute(
                "INSERT INTO library_change_queue(
                   id, root_id, root_generation, intent_kind, scope, relative_path,
                   origin, first_observed_unix_ms, most_recent_observed_unix_ms,
                   first_sequence, most_recent_sequence, coalesced_observation_count,
                   status, ready_unix_ms, catalog_revision_at_enqueue,
                   created_unix_ms, updated_unix_ms
                 ) VALUES (?1, ?2, 1, 'freshness_unknown', 'root', '',
                   'consistency_audit', 1, 1, '1', '1', 1, 'pending', 1, 0, 1, 1)",
                params![authority_id, run_id],
            )
            .expect("fixture queue parent");
        transaction
            .execute(
                "INSERT INTO library_recovery_authorities(
                   change_id, run_id, root_id, root_generation, reason, authorized_unix_ms
                 ) VALUES (?1, ?2, ?2, 1, 'containment_failure', 1)",
                params![authority_id, run_id],
            )
            .expect("fixture recovery parent");
        transaction
            .execute(
                "INSERT INTO library_metadata_inventory_spools(
                   run_id, authority_change_id, root_id, root_generation,
                   root_identity_scheme, root_identity_value,
                   scope_kind, scope_relative_path, state, created_unix_ms, updated_unix_ms
                 ) VALUES (?1, ?2, ?1, 1, 'windows-file-id-128-v1',
                   '000000000000004d:ffffffffffffffffffffffffffffffff',
                   'root', '', 'ready', 1, 1)",
                params![run_id, authority_id],
            )
            .expect("fixture spool");
        let mut directory = transaction
            .prepare(
                "INSERT INTO library_metadata_inventory_spool_directories(
                   run_id, ordinal, relative_directory, state, directory_identity_scheme,
                   directory_identity_value, source_entry_count, created_unix_ms, updated_unix_ms
                 ) VALUES (?1, ?2, ?3, 'completed', 'fixture', ?3, ?4, 1, 1)",
            )
            .expect("directory statement");
        let mut entry = transaction
            .prepare(
                "INSERT INTO library_metadata_inventory_spool_entries(
                   run_id, directory_relative_path, relative_path, entry_kind, file_size,
                   modified_unix_ms, placeholder_state, is_reparse_point, staged_unix_ms
                 ) VALUES (?1, ?2, ?3, 'file', 1, 1, 'available', 0, 1)",
            )
            .expect("entry statement");
        for ordinal in 0..directory_count {
            let path = format!("folder-{ordinal:05}");
            directory
                .execute(params![run_id, ordinal, path, entries_per_directory])
                .expect("fixture directory");
            for item in 0..entries_per_directory {
                entry
                    .execute(params![run_id, path, format!("{path}/file-{item:05}.png")])
                    .expect("fixture entry");
            }
        }
    }
    transaction.commit().expect("seed fixture");
    connection
}

#[test]
fn spool_rows_validation_work_does_not_multiply_directory_and_entry_counts() {
    let small = fixture(64, 4);
    let large = fixture(256, 4);
    let small_steps = validation_steps(&small);
    let large_steps = validation_steps(&large);
    eprintln!("spool row validation VM steps: small={small_steps}, large={large_steps}");
    assert!(small_steps > 0);
    assert!(
        large_steps <= small_steps * 6,
        "quadrupling fixed-shape input must not approach quadratic work: small={small_steps}, large={large_steps}"
    );
}

#[test]
fn spool_rows_validation_preserves_empty_and_independent_run_counts() {
    for count in [0, 1, 3] {
        let connection = fixture(count, 0);
        validate(&connection, 3).expect("complete directories may be empty");
    }
    let connection = fixture(3, 2);
    validate(&connection, 1).expect("legacy running owners");
    connection
        .execute_batch("UPDATE library_metadata_inventory_runs SET status = 'comparing'")
        .expect("comparison state");
    assert!(validate(&connection, 1).is_err());
    validate(&connection, 2).expect("current comparison owners");
    validate(&connection, 3).expect("source-revision comparison owners");
}

#[test]
fn spool_rows_validation_rejects_directory_and_lifecycle_corruption() {
    for mutation in [
        "UPDATE library_metadata_inventory_spool_directories SET ordinal = 9 WHERE run_id = 'run-a' AND ordinal = 2",
        "UPDATE library_metadata_inventory_spool_directories SET ordinal = 0.5 WHERE run_id = 'run-a' AND ordinal = 1",
        "UPDATE library_metadata_inventory_spool_directories SET source_entry_count = CASE ordinal WHEN 0 THEN 1 WHEN 1 THEN 3 ELSE 2 END WHERE run_id = 'run-a'",
        "UPDATE library_metadata_inventory_spool_directories SET source_entry_count = 1 WHERE run_id = 'run-a' AND ordinal = 0",
        "UPDATE library_metadata_inventory_spool_directories SET source_entry_count = 3 WHERE run_id = 'run-a' AND ordinal = 0",
        "UPDATE library_metadata_inventory_spool_directories SET directory_identity_scheme = NULL, directory_identity_value = NULL WHERE run_id = 'run-a' AND ordinal = 0",
        "UPDATE library_metadata_inventory_spool_directories SET state = 'pending' WHERE run_id = 'run-a' AND ordinal = 0",
        "UPDATE library_metadata_inventory_spools SET state = 'enumerating' WHERE run_id = 'run-a'",
        "UPDATE library_metadata_inventory_runs SET status = 'cancelled' WHERE id = 'run-a'",
        "UPDATE library_recovery_authorities SET retired_unix_ms = 2 WHERE change_id = 1",
    ] {
        let connection = fixture(3, 2);
        validate(&connection, 3).expect("healthy baseline");
        connection
            .execute_batch("DROP TRIGGER library_metadata_inventory_spool_directory_complete_guard")
            .expect("allow deliberate stored-row corruption");
        connection.execute_batch(mutation).expect("corrupt fixture");
        let error = validate(&connection, 3).expect_err(mutation);
        assert_eq!(
            error.code,
            "catalog_metadata_inventory_spool_contract_unverifiable"
        );
    }
}

#[test]
fn spool_rows_validation_keeps_incomplete_and_nullable_parent_semantics() {
    let connection = fixture(3, 2);
    connection
        .execute_batch(
            "DROP TRIGGER library_metadata_inventory_spool_directory_complete_guard;
             UPDATE library_metadata_inventory_spool_directories
             SET state = 'enumerating', source_entry_count = 0,
                 directory_identity_scheme = NULL, directory_identity_value = NULL
             WHERE run_id = 'run-a' AND ordinal = 2;
             UPDATE library_metadata_inventory_spools SET state = 'enumerating' WHERE run_id = 'run-a';
             INSERT INTO library_metadata_inventory_spool_entries(
               run_id, directory_relative_path, relative_path, entry_kind, file_size,
               modified_unix_ms, placeholder_state, is_reparse_point, staged_unix_ms
             ) VALUES ('run-a', NULL, 'initial-observation', 'directory', NULL, 1, 'available', 0, 1);",
        )
        .expect("provisional directory and initial subtree observation");
    validate(&connection, 3).expect("incomplete counts are not completion authority");
    connection
        .execute_batch(
            "PRAGMA foreign_keys = OFF;
             UPDATE library_metadata_inventory_spool_entries
             SET directory_relative_path = 'missing-parent' WHERE run_id = 'run-a' AND relative_path = 'initial-observation';
             PRAGMA foreign_keys = ON;",
        )
        .expect("orphan non-null parent");
    assert!(validate(&connection, 3).is_err());
}

fn validation_steps(connection: &Connection) -> usize {
    let count = Arc::new(AtomicUsize::new(0));
    let observed = Arc::clone(&count);
    connection
        .progress_handler(
            1,
            Some(move || {
                observed.fetch_add(1, Ordering::Relaxed);
                false
            }),
        )
        .expect("count bundled SQLite VM instructions");
    let result = validate(connection, 3);
    connection
        .progress_handler(0, None::<fn() -> bool>)
        .expect("retire observer");
    result.expect("valid spool relationships");
    count.load(Ordering::Relaxed)
}
