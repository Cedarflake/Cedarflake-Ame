use tempfile::tempdir;

use crate::adapters::sqlite_catalog::*;

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
