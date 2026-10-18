use crate::adapters::sqlite_catalog::tests::support::{
    downgrade_live_gap_contract_to_v29, fixture_request, publication_identity,
};
use std::time::{Duration, Instant};

use tempfile::tempdir;

use crate::domain::LibraryChangeQueueHealth;
use crate::ports::LibraryChangeQueue;

use crate::adapters::sqlite_catalog::migrations::downgrade_source_revision_contract_to_v30_for_test;
use crate::adapters::sqlite_catalog::*;

fn remove_v27_spool_contract_for_test(connection: &Connection) {
    downgrade_source_revision_contract_to_v30_for_test(connection);
    connection
        .execute_batch(
            "DROP TRIGGER IF EXISTS library_live_gap_recovery_claim_identity_update_guard;
             DROP TRIGGER IF EXISTS library_live_gap_recovery_claim_insert_guard;
             DROP INDEX IF EXISTS library_live_gap_recovery_claims_root;
             DROP TABLE IF EXISTS library_live_gap_recovery_claims;
             DROP TABLE IF EXISTS library_live_gap_recovery_contract;
             DROP INDEX IF EXISTS library_scan_publication_namespace_root;
             DROP TABLE IF EXISTS library_scan_publication_namespace_bindings;
             DROP TABLE IF EXISTS library_root_publication_namespaces;
             DROP TABLE IF EXISTS library_root_publication_namespace_contract;
             DROP TRIGGER library_metadata_inventory_spool_directory_complete_guard;
             DROP TRIGGER library_metadata_inventory_spool_binding_update_guard;
             DROP INDEX library_metadata_inventory_spool_entries_order;
             DROP TABLE library_metadata_inventory_spool_entries;
             DROP INDEX library_metadata_inventory_spool_directories_state;
             DROP TABLE library_metadata_inventory_spool_directories;
             DROP TABLE library_metadata_inventory_spools;
             DROP TABLE library_metadata_inventory_spool_contract;
             PRAGMA user_version = 26;
             UPDATE schema_info SET version = 26;",
        )
        .expect("restore v26 spool-free schema");
}

#[test]
fn large_v26_catalog_validates_once_then_reopens_in_constant_time() {
    const ROW_COUNT: i64 = 1_024;

    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let catalog = SqliteCatalog::open(path.clone()).expect("current catalog");
    catalog
        .connection
        .execute_batch(
            "INSERT INTO library_roots(id, path, created_unix_ms)
               VALUES ('runtime-load-root', 'C:/runtime-load-root', 1);
             INSERT INTO library_change_root_state(
               root_id, generation, is_active, updated_unix_ms
             ) VALUES ('runtime-load-root', 1, 1, 1);
             INSERT INTO library_persistent_journal_root_state(
               root_id, root_generation, protocol_version, contract_version,
               capability_state, continuity_state, updated_unix_ms
             ) VALUES (
               'runtime-load-root', 1, 0, 1, 'unknown', 'baseline_required', 1
             );
             INSERT INTO library_metadata_inventory_runs(
               id, root_id, root_generation, epoch, scope_kind, scope_relative_path,
               status, next_page_index, staged_entry_count, started_unix_ms, updated_unix_ms,
               last_issue_code, last_issue_message
             ) VALUES (
               'runtime-load-run', 'runtime-load-root', 1, 1, 'root', '',
               'failed', 1025, 1024, 1, 1, 'fixture_terminal', 'fixture terminal run'
             );
             WITH RECURSIVE sequence(value) AS (
               VALUES(1) UNION ALL SELECT value + 1 FROM sequence WHERE value < 1024
             )
             INSERT INTO library_change_queue(
               id, root_id, root_generation, intent_kind, scope, relative_path,
               origin, first_observed_unix_ms, most_recent_observed_unix_ms,
               first_sequence, most_recent_sequence, coalesced_observation_count,
               status, ready_unix_ms, catalog_revision_at_enqueue,
               catalog_revision_at_success, catch_up_source, catch_up_watermark,
               created_unix_ms, updated_unix_ms
             )
             SELECT
               10000 + value, 'runtime-load-root', 1, 'reconcile', 'path',
               printf('entry-%04d.jpg', value), 'startup_catch_up', 1, 1,
               printf('%d', value), printf('%d', value), 1,
               'completed', 1, 0, 0, 'runtime_load_v1',
               printf('watermark-%04d', value), 1, 1
             FROM sequence;
             INSERT INTO library_change_queue_catch_up_lineage(
               change_id, catch_up_source, catch_up_watermark, enrolled_unix_ms
             )
             SELECT id, catch_up_source, catch_up_watermark, 1
             FROM library_change_queue WHERE root_id = 'runtime-load-root';
             INSERT INTO library_metadata_inventory_candidate_owners(
               run_id, candidate_key, change_id, candidate_role,
               relative_path, owned_unix_ms
             )
             SELECT 'runtime-load-run', printf('candidate-%04d', id - 10000), id,
                    'present', relative_path, 1
             FROM library_change_queue WHERE root_id = 'runtime-load-root';
             WITH RECURSIVE sequence(value) AS (
               VALUES(0) UNION ALL SELECT value + 1 FROM sequence WHERE value < 1023
             )
             INSERT INTO library_metadata_inventory_frontier(
               run_id, ordinal, relative_directory, state,
               enumerated_entry_count, updated_unix_ms
             )
             SELECT 'runtime-load-run', value, printf('directory-%04d', value),
                    'completed', 1, 1
             FROM sequence;",
        )
        .expect("seed large current catalog");
    let counts: (i64, i64, i64, i64) = catalog
        .connection
        .query_row(
            "SELECT
               (SELECT COUNT(*) FROM library_change_queue
                WHERE root_id = 'runtime-load-root'),
               (SELECT COUNT(*) FROM library_change_queue_catch_up_lineage),
               (SELECT COUNT(*) FROM library_metadata_inventory_candidate_owners),
               (SELECT COUNT(*) FROM library_metadata_inventory_frontier)",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .expect("large fixture counts");
    assert_eq!(counts, (ROW_COUNT, ROW_COUNT, ROW_COUNT, ROW_COUNT));
    remove_v27_spool_contract_for_test(&catalog.connection);
    drop(catalog);

    reset_full_schema_validation_count(&path);
    let validation_started = Instant::now();
    let session = SqliteCatalogSession::validate(path.clone()).expect("migrate and validate v26");
    let validation_elapsed = validation_started.elapsed();
    assert_eq!(full_schema_validation_count(&path), 1);

    let reopen_started = Instant::now();
    let mut maximum_reopen_elapsed = Duration::ZERO;
    for _ in 0..100 {
        let single_reopen_started = Instant::now();
        session
            .open_in_lane(LibraryChangeLane::Live)
            .expect("constant-time validated reopen");
        maximum_reopen_elapsed = maximum_reopen_elapsed.max(single_reopen_started.elapsed());
    }
    let reopen_elapsed = reopen_started.elapsed();
    assert_eq!(full_schema_validation_count(&path), 1);
    assert!(
        validation_elapsed < Duration::from_secs(10),
        "large v26 migration and validation took {validation_elapsed:?}"
    );
    assert!(
        maximum_reopen_elapsed < Duration::from_secs(1),
        "one validated reopen took {maximum_reopen_elapsed:?}"
    );
    eprintln!(
        "large-v26-validation rows={ROW_COUNT} validation={validation_elapsed:?} reopens100={reopen_elapsed:?} max_reopen={maximum_reopen_elapsed:?} full_validations={}",
        full_schema_validation_count(&path)
    );
}

#[test]
fn migrates_v25_through_v27_recovery_execution_and_spool_contracts() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let catalog = SqliteCatalog::open(path.clone()).expect("current catalog");
    remove_v27_spool_contract_for_test(&catalog.connection);
    catalog
        .connection
        .execute_batch(
            "DROP TRIGGER library_metadata_inventory_candidate_owner_update_guard;
             DROP TRIGGER library_metadata_inventory_candidate_owner_insert_guard;
             DROP INDEX library_metadata_inventory_candidate_owners_change;
             DROP INDEX library_metadata_inventory_frontier_state;
             DROP TABLE library_metadata_inventory_candidate_owners;
             DROP TABLE library_metadata_inventory_frontier;
             DROP TABLE library_recovery_execution_contract;
             UPDATE schema_info SET version = 25;",
        )
        .expect("restore v25 recovery execution shape");
    drop(catalog);

    let migrated = SqliteCatalog::open(path.clone()).expect("migrate v25 catalog");
    let evidence: (i64, i64, i64, i64) = migrated
        .connection
        .query_row(
            "SELECT
               (SELECT version FROM schema_info),
               (SELECT complete FROM library_recovery_execution_contract),
               (SELECT COUNT(*) FROM library_metadata_inventory_candidate_owners),
               (SELECT COUNT(*) FROM library_metadata_inventory_frontier)",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .expect("current recovery execution evidence");
    assert_eq!(evidence, (SCHEMA_VERSION, 1, 0, 0));
    drop(migrated);
    SqliteCatalog::open(path).expect("reopen migrated current catalog");
}

#[test]
fn migrates_v26_through_the_current_spool_contract_and_reopens() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let catalog = SqliteCatalog::open(path.clone()).expect("current catalog");
    remove_v27_spool_contract_for_test(&catalog.connection);
    drop(catalog);

    let migrated = SqliteCatalog::open(path.clone()).expect("migrate v26 catalog");
    let evidence: (i64, i64, i64, i64, i64) = migrated
        .connection
        .query_row(
            "SELECT
               (SELECT version FROM schema_info),
               (SELECT user_version FROM pragma_user_version),
               (SELECT contract_version
                FROM library_metadata_inventory_spool_contract WHERE singleton = 1),
               (SELECT complete
                FROM library_metadata_inventory_spool_contract WHERE singleton = 1),
               (SELECT COUNT(*) FROM library_metadata_inventory_spools)",
            [],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            },
        )
        .expect("current spool contract evidence");
    assert_eq!(evidence, (SCHEMA_VERSION, SCHEMA_VERSION, 4, 1, 0));
    drop(migrated);
    SqliteCatalog::open(path).expect("reopen migrated current catalog");
}

#[test]
fn current_v27_rejects_missing_spool_order_index_on_reopen() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let catalog = SqliteCatalog::open(path.clone()).expect("current catalog");
    catalog
        .connection
        .execute_batch("DROP INDEX library_metadata_inventory_spool_entries_order")
        .expect("corrupt spool order contract");
    drop(catalog);

    let error = match SqliteCatalog::open(path) {
        Ok(_) => panic!("corrupt v27 spool contract must fail closed"),
        Err(error) => error,
    };
    assert_eq!(
        error.code,
        "catalog_metadata_inventory_spool_contract_unverifiable"
    );
}

#[test]
fn current_v28_rejects_orphaned_spool_binding_on_reopen() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let catalog = SqliteCatalog::open(path.clone()).expect("current catalog");
    catalog
        .connection
        .execute_batch(
            "INSERT INTO library_roots(id, path, created_unix_ms)
               VALUES ('bound-root', 'C:/bound-root', 1);
             INSERT INTO scan_runs(
               id, root_id, status, started_unix_ms, completed_unix_ms, preview_edge
             ) VALUES ('bound-scan', 'bound-root', 'completed', 1, 1, 128);
             UPDATE library_roots SET active_scan_id = 'bound-scan' WHERE id = 'bound-root';
             INSERT INTO library_change_root_state(
               root_id, generation, is_active, updated_unix_ms
             ) VALUES ('bound-root', 1, 1, 1);
             INSERT INTO library_persistent_journal_root_state(
               root_id, root_generation, protocol_version, contract_version,
               capability_state, continuity_state, updated_unix_ms
             ) VALUES (
               'bound-root', 1, 5, 1, 'supported', 'recovery_required', 1
             );
             INSERT INTO library_persistent_journal_checkpoints(
               root_id, root_generation, volume_guid, volume_serial,
               root_reference_version, root_file_reference, journal_id,
               next_unread_usn, captured_exclusive_end, covered_catalog_revision,
               protocol_version, contract_version, continuity_state, updated_unix_ms
             ) VALUES (
               'bound-root', 1, 'bound-volume', '1',
               3, X'01010101010101010101010101010101', '44',
               '20', '20', 0, 5, 1, 'recovery_required', 1
             );
             INSERT INTO library_change_queue(
               id, root_id, root_generation, intent_kind, scope, relative_path,
               origin, first_observed_unix_ms, most_recent_observed_unix_ms,
               first_sequence, most_recent_sequence, coalesced_observation_count,
               status, ready_unix_ms, catalog_revision_at_enqueue,
               created_unix_ms, updated_unix_ms
             ) VALUES (
               999, 'bound-root', 1, 'freshness_unknown', 'root', '',
               'consistency_audit', 1, 1, '1', '1', 1,
               'pending', 1, 0, 1, 1
             );
             INSERT INTO library_recovery_authorities(
               change_id, run_id, root_id, root_generation, reason, authorized_unix_ms
             ) VALUES (999, 'bound-run', 'bound-root', 1, 'containment_failure', 1);
             INSERT INTO library_persistent_journal_baselines(
               change_id, root_id, root_generation, volume_guid, volume_serial,
               root_reference_version, root_file_reference, journal_id,
               opening_next_usn, protocol_version, contract_version,
               phase, authorized_unix_ms, updated_unix_ms
             ) VALUES (
               999, 'bound-root', 1, 'bound-volume', '1',
               3, X'01010101010101010101010101010101', '44',
               '20', 5, 1, 'inventory', 1, 1
             );
             INSERT INTO library_metadata_inventory_runs(
               id, root_id, root_generation, epoch, scope_kind, scope_relative_path,
               status, next_page_index, started_unix_ms, updated_unix_ms
             ) VALUES (
               'bound-run', 'bound-root', 1, 1, 'root', '',
               'running', 1, 1, 1
             );",
        )
        .expect("insert valid recovery binding fixture");
    drop(catalog);
    let catalog = SqliteCatalog::open(path.clone()).expect("reopen valid recovery binding fixture");
    catalog
        .connection
        .execute_batch(
            "INSERT INTO library_metadata_inventory_spools(
               run_id, authority_change_id, root_id, root_generation,
               root_identity_scheme, root_identity_value,
               scope_kind, scope_relative_path, state,
               created_unix_ms, updated_unix_ms
             ) VALUES (
               'bound-run', 999, 'mismatched-root', 1,
               'windows-file-id-128-v1', 'test-volume:test-root',
               'root', '', 'enumerating', 1, 1
             );",
        )
        .expect("insert mismatched spool binding with valid foreign keys");
    drop(catalog);

    let error = match SqliteCatalog::open(path) {
        Ok(_) => panic!("orphaned v28 spool binding must fail closed"),
        Err(error) => error,
    };
    assert_eq!(
        error.code,
        "catalog_metadata_inventory_spool_contract_unverifiable"
    );
}

#[test]
fn current_v27_rejects_missing_candidate_ownership_index_on_reopen() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let catalog = SqliteCatalog::open(path.clone()).expect("current catalog");
    catalog
        .connection
        .execute_batch("DROP INDEX library_metadata_inventory_candidate_owners_change")
        .expect("corrupt candidate ownership contract");
    drop(catalog);

    let error = match SqliteCatalog::open(path) {
        Ok(_) => panic!("corrupt v27 catalog must fail closed"),
        Err(error) => error,
    };
    assert_eq!(
        error.code,
        "catalog_recovery_execution_contract_unverifiable"
    );
}

fn migrate_v29_foreground_owned_gap_fixture(
    catalog_path: &Path,
    root_id: &str,
    root_path: &str,
    scan_id: &str,
    is_paused: bool,
) -> ScanRequest {
    let mut catalog = SqliteCatalog::open(catalog_path.to_path_buf()).expect("catalog");
    let identity = publication_identity('f');
    let initial = fixture_request(&format!("{scan_id}-initial"), root_path);
    catalog
        .begin_scan_with_publication_namespace(&initial, root_id, root_path, &identity)
        .expect("begin initial publication");
    catalog
        .prove_live_only_first_import_handoff_for_test(&initial.scan_id)
        .expect("prove v29 fixture first-import handoff");
    catalog
        .publish_scan(&initial.scan_id, root_id, 0, 0)
        .expect("publish initial catalog");
    catalog
        .connection
        .execute(
            "INSERT INTO library_change_queue(
               root_id, root_generation, intent_kind, scope, relative_path,
               origin, first_observed_unix_ms, most_recent_observed_unix_ms,
               first_sequence, most_recent_sequence, coalesced_observation_count,
               status, ready_unix_ms, catalog_revision_at_enqueue,
               created_unix_ms, updated_unix_ms
             ) VALUES (
               ?1, 1, 'freshness_unknown', 'root', '', 'startup_catch_up',
               41, 41, '41', '41', 1, 'pending', 41, 1, 41, 41
             )",
            [root_id],
        )
        .expect("insert v29 foreground-owned gap");
    let request = fixture_request(scan_id, root_path);
    catalog
        .begin_scan_with_publication_namespace(&request, root_id, root_path, &identity)
        .expect("begin foreground owner");
    if is_paused {
        catalog
            .pause_scan(&request.scan_id, &ScanCheckpoint::default())
            .expect("pause foreground owner");
    }
    let owned: (String, String, String, i64) = catalog
        .connection
        .query_row(
            "SELECT scans.status, scans.scan_owner, gap.authoritative_scan_id,
                    (SELECT COUNT(*) FROM library_live_gap_recovery_claims
                     WHERE gap_change_id = gap.id)
             FROM scan_runs AS scans
             JOIN library_change_queue AS gap
               ON gap.authoritative_scan_id = scans.id
             WHERE scans.id = ?1",
            [&request.scan_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .expect("v29 foreground ownership evidence");
    assert_eq!(owned.0, if is_paused { "paused" } else { "running" });
    assert_eq!(owned.1, "foreground");
    assert_eq!(owned.2, request.scan_id);
    assert_eq!(owned.3, 0);
    downgrade_live_gap_contract_to_v29(&catalog.connection);
    drop(catalog);
    request
}

fn assert_v29_foreground_owned_gap_resumes_after_migration(is_paused: bool) {
    let directory = tempdir().expect("catalog directory");
    let catalog_path = directory.path().join("catalog.sqlite3");
    let root_id = if is_paused {
        "v29-paused-foreground-root"
    } else {
        "v29-running-foreground-root"
    };
    let root_path = if is_paused {
        "C:\\V29PausedForeground"
    } else {
        "C:\\V29RunningForeground"
    };
    let scan_id = if is_paused {
        "v29-paused-foreground-scan"
    } else {
        "v29-running-foreground-scan"
    };
    let request = migrate_v29_foreground_owned_gap_fixture(
        &catalog_path,
        root_id,
        root_path,
        scan_id,
        is_paused,
    );

    let mut catalog = SqliteCatalog::open(catalog_path.clone()).expect("migrate owned v29 gap");
    let migrated: (String, String, String, i64, Option<String>) = catalog
        .connection
        .query_row(
            "SELECT scans.status, scans.scan_owner, gap.status,
                    (SELECT COUNT(*) FROM library_live_gap_recovery_claims
                     WHERE gap_change_id = gap.id),
                    gap.authoritative_scan_id
             FROM scan_runs AS scans
             JOIN library_change_queue AS gap
               ON gap.authoritative_scan_id = scans.id
             WHERE scans.id = ?1",
            [&request.scan_id],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            },
        )
        .expect("migrated foreground ownership evidence");
    assert_eq!(
        migrated,
        (
            if is_paused { "paused" } else { "running" }.to_owned(),
            "foreground".to_owned(),
            "leased".to_owned(),
            0,
            Some(request.scan_id.clone()),
        )
    );
    catalog
        .resume_scan(&request, root_id, root_path)
        .expect("resume migrated foreground scan");
    catalog
        .publish_scan(&request.scan_id, root_id, 0, 0)
        .expect("publish migrated foreground scan");
    drop(catalog);

    let reopened = SqliteCatalog::open(catalog_path).expect("reopen published v30 catalog");
    let published: (String, String, String, i64, String) = reopened
        .connection
        .query_row(
            "SELECT scans.status, scans.scan_owner, gap.status,
                    (SELECT COUNT(*) FROM library_live_gap_recovery_claims
                     WHERE gap_change_id = gap.id),
                    roots.active_scan_id
             FROM scan_runs AS scans
             JOIN library_change_queue AS gap
               ON gap.root_id = scans.root_id
              AND gap.root_generation = scans.root_generation_at_start
              AND gap.id <= scans.change_queue_high_watermark
             JOIN library_roots AS roots ON roots.id = scans.root_id
             WHERE scans.id = ?1",
            [&request.scan_id],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            },
        )
        .expect("published migrated foreground evidence");
    assert_eq!(
        published,
        (
            "completed".to_owned(),
            "foreground".to_owned(),
            "completed".to_owned(),
            0,
            request.scan_id,
        )
    );
    let metrics = reopened
        .load_library_change_root_queue_metrics(
            root_id,
            LibraryRootGeneration::initial(),
            unix_time_ms(),
            LibraryChangeQueuePolicy::default(),
        )
        .expect("load synchronized queue metrics");
    assert_eq!(metrics.pending_count, 0);
    assert_eq!(metrics.leased_count, 0);
    assert_eq!(metrics.retry_wait_count, 0);
    assert_eq!(metrics.explicit_recovery_required_count, 0);
    assert_eq!(metrics.health, LibraryChangeQueueHealth::Idle);
}

#[test]
fn v29_running_foreground_scan_retains_owned_gap_through_resume_and_publication() {
    assert_v29_foreground_owned_gap_resumes_after_migration(false);
}

#[test]
fn v29_paused_foreground_scan_retains_owned_gap_through_resume_and_publication() {
    assert_v29_foreground_owned_gap_resumes_after_migration(true);
}
