use rusqlite::Connection;
use tempfile::{NamedTempFile, tempdir};

use super::initialize_schema;
use super::recovery_test_catalog::{
    active_frontier_catalog, recovery_lifecycle_catalog, reopen_recovery_lifecycle_catalog,
};
use crate::adapters::SqliteCatalog;

#[test]
fn fresh_catalog_enables_incremental_auto_vacuum_before_schema_creation() {
    let catalog = NamedTempFile::new().expect("fresh catalog");
    let mut connection = Connection::open(catalog.path()).expect("open fresh catalog");

    initialize_schema(&mut connection).expect("create current schema");

    let mode: i64 = connection
        .query_row("PRAGMA auto_vacuum", [], |row| row.get(0))
        .expect("read auto-vacuum mode");
    assert_eq!(mode, 2);
}

#[test]
fn production_catalog_open_enables_incremental_before_wal_initialization() {
    let directory = tempdir().expect("catalog directory");
    let path = directory.path().join("ame.sqlite3");

    drop(SqliteCatalog::open(path.clone()).expect("initialize production catalog"));

    let connection = Connection::open(path).expect("inspect production catalog");
    let mode: i64 = connection
        .query_row("PRAGMA auto_vacuum", [], |row| row.get(0))
        .expect("read production auto-vacuum mode");
    assert_eq!(mode, 2);
}

#[test]
fn opening_current_none_auto_vacuum_catalog_does_not_vacuum() {
    let directory = tempdir().expect("catalog directory");
    let path = directory.path().join("ame.sqlite3");
    let mut connection = Connection::open(&path).expect("open legacy catalog");
    initialize_schema(&mut connection).expect("create current schema");
    connection
        .execute_batch(
            "PRAGMA journal_mode = DELETE;
                 PRAGMA auto_vacuum = NONE;
                 VACUUM;
                 CREATE TABLE migration_reclamation_fixture(payload BLOB NOT NULL);
                 WITH RECURSIVE rows(value) AS (
                   SELECT 1 UNION ALL SELECT value + 1 FROM rows WHERE value < 512
                 )
                 INSERT INTO migration_reclamation_fixture(payload)
                 SELECT zeroblob(4096) FROM rows;
                 DELETE FROM migration_reclamation_fixture;",
        )
        .expect("seed legacy freelist");
    let before_freelist: i64 = connection
        .query_row("PRAGMA freelist_count", [], |row| row.get(0))
        .expect("legacy freelist before reopen migration");
    assert!(before_freelist > 0);
    drop(connection);

    drop(SqliteCatalog::open(path.clone()).expect("reopen current legacy catalog"));

    let connection = Connection::open(path).expect("inspect reopened legacy catalog");
    let mode: i64 = connection
        .query_row("PRAGMA auto_vacuum", [], |row| row.get(0))
        .expect("legacy auto-vacuum mode after migration");
    let after_freelist: i64 = connection
        .query_row("PRAGMA freelist_count", [], |row| row.get(0))
        .expect("legacy freelist after migration");
    assert_eq!(mode, 0);
    assert_eq!(after_freelist, before_freelist);
}

#[test]
fn current_reopens_legal_active_and_completed_recovery_lifecycles() {
    let active = recovery_lifecycle_catalog(false);
    reopen_recovery_lifecycle_catalog(&active).expect("reopen legal active lifecycle");
    let completed = recovery_lifecycle_catalog(true);
    reopen_recovery_lifecycle_catalog(&completed).expect("reopen legal completed lifecycle");
}

#[test]
fn current_rejects_completed_baseline_with_active_authority_on_reopen() {
    let catalog = recovery_lifecycle_catalog(true);
    let connection = Connection::open(catalog.path()).expect("mutate completed lifecycle");
    connection
        .execute_batch(
            "UPDATE library_change_queue
                 SET status = 'pending', catalog_revision_at_success = NULL
                 WHERE id = 901;
                 UPDATE library_recovery_authorities
                 SET retired_unix_ms = NULL WHERE change_id = 901;",
        )
        .expect("reactivate completed authority");
    drop(connection);

    let error = reopen_recovery_lifecycle_catalog(&catalog)
        .expect_err("completed baseline cannot retain active authority");
    assert_eq!(
        error.code,
        "catalog_persistent_journal_baseline_contract_unverifiable"
    );
}

#[test]
fn current_rejects_completed_baseline_with_noncurrent_root_on_reopen() {
    let catalog = recovery_lifecycle_catalog(true);
    let connection = Connection::open(catalog.path()).expect("mutate completed lifecycle");
    connection
        .execute(
            "UPDATE library_persistent_journal_root_state
                 SET continuity_state = 'catching_up'
                 WHERE root_id = 'lifecycle-root'",
            [],
        )
        .expect("regress completed root continuity");
    drop(connection);

    let error = reopen_recovery_lifecycle_catalog(&catalog)
        .expect_err("completed baseline requires Current root");
    assert_eq!(
        error.code,
        "catalog_persistent_journal_baseline_contract_unverifiable"
    );
}

#[test]
fn current_rejects_completed_baseline_with_noncurrent_checkpoint_on_reopen() {
    let catalog = recovery_lifecycle_catalog(true);
    let connection = Connection::open(catalog.path()).expect("mutate completed lifecycle");
    connection
        .execute(
            "UPDATE library_persistent_journal_checkpoints
                 SET continuity_state = 'catching_up'
                 WHERE root_id = 'lifecycle-root'",
            [],
        )
        .expect("regress completed checkpoint continuity");
    drop(connection);

    let error = reopen_recovery_lifecycle_catalog(&catalog)
        .expect_err("completed baseline requires Current checkpoint");
    assert_eq!(
        error.code,
        "catalog_persistent_journal_baseline_contract_unverifiable"
    );
}

#[test]
fn current_rejects_active_baseline_with_retired_authority_on_reopen() {
    let catalog = recovery_lifecycle_catalog(false);
    let connection = Connection::open(catalog.path()).expect("mutate active lifecycle");
    connection
        .execute_batch(
            "UPDATE library_change_queue
                 SET status = 'completed', catalog_revision_at_success = 0
                 WHERE id = 901;
                 UPDATE library_recovery_authorities
                 SET retired_unix_ms = 2 WHERE change_id = 901;",
        )
        .expect("retire active authority");
    drop(connection);

    let error = reopen_recovery_lifecycle_catalog(&catalog)
        .expect_err("active baseline cannot lose its authority");
    assert_eq!(
        error.code,
        "catalog_persistent_journal_baseline_contract_unverifiable"
    );
}

#[test]
fn current_schema_rejects_active_run_owned_by_a_different_baseline_authority() {
    let catalog = active_frontier_catalog();
    let connection = Connection::open(catalog.path()).expect("mutate active lifecycle");
    connection
        .execute_batch(
            "INSERT INTO library_change_queue(
                   id, root_id, root_generation, intent_kind, scope, relative_path,
                   origin, first_observed_unix_ms, most_recent_observed_unix_ms,
                   first_sequence, most_recent_sequence, coalesced_observation_count,
                   status, ready_unix_ms, catalog_revision_at_enqueue,
                   created_unix_ms, updated_unix_ms
                 ) VALUES (
                   902, 'lifecycle-root', 1, 'freshness_unknown', 'root', '',
                   'metadata_inventory', 2, 2, '2', '2', 1,
                   'pending', 2, 0, 2, 2
                 );
                 UPDATE library_recovery_authorities
                 SET run_id = 'baseline-waiting-run' WHERE change_id = 901;
                 INSERT INTO library_recovery_authorities(
                   change_id, run_id, root_id, root_generation, reason,
                   authorized_unix_ms
                 ) VALUES (
                   902, 'lifecycle-run', 'lifecycle-root', 1,
                   'watcher_uncovered_gap', 2
                 );",
        )
        .expect("insert mismatched active authority");
    drop(connection);

    let error = reopen_recovery_lifecycle_catalog(&catalog)
        .expect_err("active run must own the unfinished baseline authority");
    assert_eq!(
        error.code,
        "catalog_persistent_journal_baseline_contract_unverifiable"
    );
}

#[test]
fn current_schema_rejects_terminal_run_still_referenced_by_unfinished_baseline() {
    let catalog = recovery_lifecycle_catalog(false);
    let connection = Connection::open(catalog.path()).expect("mutate active lifecycle");
    connection
        .execute_batch(
            "INSERT INTO library_metadata_inventory_runs(
                   id, root_id, root_generation, epoch, scope_kind, scope_relative_path,
                   status, next_page_index, started_unix_ms, updated_unix_ms
                 ) VALUES (
                   'lifecycle-run', 'lifecycle-root', 1, 1, 'root', '',
                   'failed', 1, 1, 2
                 );",
        )
        .expect("insert terminal run referenced by unfinished baseline");
    drop(connection);

    let error = reopen_recovery_lifecycle_catalog(&catalog)
        .expect_err("unfinished baseline cannot retain a terminal run owner");
    assert_eq!(
        error.code,
        "catalog_persistent_journal_baseline_contract_unverifiable"
    );
}

#[test]
fn current_rejects_active_baseline_with_current_projection_on_reopen() {
    let catalog = recovery_lifecycle_catalog(false);
    let connection = Connection::open(catalog.path()).expect("mutate active lifecycle");
    connection
        .execute_batch(
            "INSERT INTO library_persistent_journal_checkpoints(
                   root_id, root_generation, volume_guid, volume_serial,
                   root_reference_version, root_file_reference, journal_id,
                   next_unread_usn, captured_exclusive_end, covered_catalog_revision,
                   protocol_version, contract_version, continuity_state,
                   updated_unix_ms
                 ) VALUES (
                   'lifecycle-root', 1, 'lifecycle-volume', '77',
                   3, X'01010101010101010101010101010101', '44',
                   '10', '10', 0, 5, 1, 'current', 2
                 );
                 UPDATE library_persistent_journal_root_state
                 SET continuity_state = 'current', updated_unix_ms = 2
                 WHERE root_id = 'lifecycle-root';",
        )
        .expect("premature Current projection");
    drop(connection);

    let error = reopen_recovery_lifecycle_catalog(&catalog)
        .expect_err("active baseline cannot project Current");
    assert_eq!(
        error.code,
        "catalog_persistent_journal_baseline_contract_unverifiable"
    );
}
