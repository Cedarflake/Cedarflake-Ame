use rusqlite::Connection;

use super::super::create_metadata_inventory_contract;
use super::super::legacy_test_catalog::fresh_v19_catalog;
use super::super::migrate_schema;

#[test]
fn current_v20_malformed_inventory_contract_fails_closed() {
    let mut connection = Connection::open_in_memory().expect("catalog");
    migrate_schema(&mut connection).expect("fresh v20 catalog");
    connection
        .execute_batch(
            "DROP INDEX library_metadata_inventory_entries_compare;
             CREATE INDEX library_metadata_inventory_entries_compare
               ON library_metadata_inventory_entries(run_id, relative_path);",
        )
        .expect("malformed inventory index fixture");

    let error = migrate_schema(&mut connection).expect_err("malformed inventory contract");

    assert_eq!(
        error.code,
        "catalog_metadata_inventory_contract_unverifiable"
    );
}

#[test]
fn current_v20_malformed_active_inventory_predicate_fails_closed() {
    let mut connection = Connection::open_in_memory().expect("catalog");
    migrate_schema(&mut connection).expect("fresh v20 catalog");
    connection
        .execute_batch(
            "DROP INDEX library_metadata_inventory_runs_one_active_root;
             CREATE UNIQUE INDEX library_metadata_inventory_runs_one_active_root
               ON library_metadata_inventory_runs(root_id)
               WHERE status = 'running';",
        )
        .expect("malformed active inventory index fixture");

    let error = migrate_schema(&mut connection).expect_err("malformed active inventory index");

    assert_eq!(
        error.code,
        "catalog_metadata_inventory_contract_unverifiable"
    );
}

#[test]
fn current_v20_without_inventory_queue_origin_fails_closed() {
    let mut connection = fresh_v19_catalog();
    let transaction = connection.transaction().expect("v20 fixture transaction");
    create_metadata_inventory_contract(&transaction).expect("inventory contract fixture");
    transaction
        .execute("UPDATE schema_info SET version = 20", [])
        .expect("v20 fixture version");
    transaction.commit().expect("commit v20 fixture");

    let error = migrate_schema(&mut connection).expect_err("missing inventory queue origin");

    assert_eq!(
        error.code,
        "catalog_metadata_inventory_contract_unverifiable"
    );
}

#[test]
fn current_v20_active_inventory_with_stale_generation_fails_closed() {
    let mut connection = Connection::open_in_memory().expect("catalog");
    migrate_schema(&mut connection).expect("fresh v20 catalog");
    connection
        .execute_batch(
            "INSERT INTO library_roots(id, path, active_scan_id, created_unix_ms)
               VALUES ('root-a', 'C:/source', 'published-scan', 1);
             INSERT INTO library_change_root_state(
               root_id, generation, is_active, updated_unix_ms
             ) VALUES ('root-a', 1, 1, 1);
             INSERT INTO library_metadata_inventory_runs(
               id, root_id, root_generation, epoch, scope_kind, scope_relative_path,
               status, next_page_index, started_unix_ms, updated_unix_ms
             ) VALUES (
               'inventory-a', 'root-a', 2, 1, 'root', '', 'running', 1, 1, 1
             );",
        )
        .expect("stale inventory generation fixture");

    let error = migrate_schema(&mut connection).expect_err("stale active inventory");

    assert_eq!(
        error.code,
        "catalog_metadata_inventory_contract_unverifiable"
    );
}

#[test]
fn current_v20_active_inventory_with_staged_count_mismatch_fails_closed() {
    let mut connection = Connection::open_in_memory().expect("catalog");
    migrate_schema(&mut connection).expect("fresh v20 catalog");
    connection
        .execute_batch(
            "INSERT INTO library_roots(id, path, created_unix_ms)
               VALUES ('root-a', 'C:/source', 1);
             INSERT INTO scan_runs(
               id, root_id, status, started_unix_ms, completed_unix_ms, preview_edge
             ) VALUES ('published-scan', 'root-a', 'completed', 1, 1, 128);
             UPDATE library_roots SET active_scan_id = 'published-scan' WHERE id = 'root-a';
             INSERT INTO library_change_root_state(
               root_id, generation, is_active, updated_unix_ms
             ) VALUES ('root-a', 1, 1, 1);
             INSERT INTO library_metadata_inventory_runs(
               id, root_id, root_generation, epoch, scope_kind, scope_relative_path,
               status, next_page_index, staged_entry_count, started_unix_ms, updated_unix_ms
             ) VALUES (
               'inventory-a', 'root-a', 1, 1, 'root', '', 'running', 2, 1, 1, 1
             );",
        )
        .expect("active staged-count mismatch fixture");

    let error = migrate_schema(&mut connection).expect_err("active staged-count mismatch");

    assert_eq!(
        error.code,
        "catalog_metadata_inventory_contract_unverifiable"
    );
}
