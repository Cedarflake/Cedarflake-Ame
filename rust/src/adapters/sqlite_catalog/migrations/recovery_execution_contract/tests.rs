use rusqlite::Connection;

use super::super::recovery_test_catalog::{
    active_frontier_catalog, reopen_recovery_lifecycle_catalog,
};

#[test]
fn current_v27_reopens_a_legal_restart_safe_inventory_frontier() {
    let catalog = active_frontier_catalog();

    reopen_recovery_lifecycle_catalog(&catalog).expect("reopen legal active frontier");
}

#[test]
fn current_v27_rejects_an_active_run_that_lost_its_frontier_on_reopen() {
    let catalog = active_frontier_catalog();
    let connection = Connection::open(catalog.path()).expect("mutate active frontier");
    connection
        .execute("DELETE FROM library_metadata_inventory_frontier", [])
        .expect("remove durable frontier");
    drop(connection);

    let error = reopen_recovery_lifecycle_catalog(&catalog)
        .expect_err("advanced active run requires a frontier");
    assert_eq!(
        error.code,
        "catalog_recovery_execution_contract_unverifiable"
    );
}

#[test]
fn current_v27_rejects_a_frontier_ordinal_gap_on_reopen() {
    let catalog = active_frontier_catalog();
    let connection = Connection::open(catalog.path()).expect("mutate frontier ordinal");
    connection
        .execute(
            "UPDATE library_metadata_inventory_frontier
             SET ordinal = 3 WHERE relative_directory = 'album'",
            [],
        )
        .expect("create frontier ordinal gap");
    drop(connection);

    let error = reopen_recovery_lifecycle_catalog(&catalog)
        .expect_err("frontier ordinals must remain contiguous");
    assert_eq!(
        error.code,
        "catalog_recovery_execution_contract_unverifiable"
    );
}

#[test]
fn current_v27_rejects_a_pending_frontier_without_its_staged_directory() {
    let catalog = active_frontier_catalog();
    let connection = Connection::open(catalog.path()).expect("mutate pending frontier");
    connection
        .execute_batch(
            "DELETE FROM library_metadata_inventory_entries
             WHERE relative_path = 'album';
             INSERT INTO library_metadata_inventory_entries(
               run_id, relative_path, entry_kind, file_size, modified_unix_ms,
               placeholder_state, is_reparse_point, staged_page_index, staged_unix_ms
             ) VALUES (
               'lifecycle-run', 'replacement.txt', 'file', 1, 1,
               'available', 0, 1, 2
             );",
        )
        .expect("remove pending directory evidence");
    drop(connection);

    let error = reopen_recovery_lifecycle_catalog(&catalog)
        .expect_err("pending frontier requires staged directory evidence");
    assert_eq!(
        error.code,
        "catalog_recovery_execution_contract_unverifiable"
    );
}

#[test]
fn current_v27_rejects_a_running_frontier_marked_completed_on_reopen() {
    let catalog = active_frontier_catalog();
    let connection = Connection::open(catalog.path()).expect("mutate frontier state");
    connection
        .execute(
            "UPDATE library_metadata_inventory_frontier
             SET state = 'completed', resume_after_relative_path = NULL,
                 enumerated_entry_count = 0",
            [],
        )
        .expect("prematurely complete frontier");
    drop(connection);

    let error = reopen_recovery_lifecycle_catalog(&catalog)
        .expect_err("running inventory cannot own completed frontier");
    assert_eq!(
        error.code,
        "catalog_recovery_execution_contract_unverifiable"
    );
}
