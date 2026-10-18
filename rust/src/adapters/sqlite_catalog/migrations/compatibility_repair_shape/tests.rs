use super::*;

fn current_catalog() -> Connection {
    let mut connection = Connection::open_in_memory().unwrap();
    super::super::migrate_schema(&mut connection).unwrap();
    connection
}

#[test]
fn current_schema_admits_existing_terminal_repair_contracts_without_accepting_unknown_versions() {
    let connection = current_catalog();
    for version in 37..=44 {
        assert!(inventory_shape_matches(&connection, version).unwrap());
        assert!(live_gap_shape_matches(&connection, version).unwrap());
    }
    assert!(!inventory_shape_matches(&connection, 45).unwrap());
    assert!(!live_gap_shape_matches(&connection, 45).unwrap());
    assert!(!inventory_shape_matches(&connection, 24).unwrap());
    assert!(!live_gap_shape_matches(&connection, 29).unwrap());
}

#[test]
fn damaged_claim_guard_or_spool_marker_cannot_acquire_repair_authority() {
    let connection = current_catalog();
    connection
        .execute_batch("DROP TRIGGER library_live_gap_recovery_claim_insert_guard")
        .unwrap();
    for version in 37..=44 {
        assert!(!live_gap_shape_matches(&connection, version).unwrap());
    }
    let connection = current_catalog();
    connection
        .execute_batch("DELETE FROM library_metadata_inventory_spool_contract")
        .unwrap();
    for version in 37..=44 {
        assert!(!inventory_shape_matches(&connection, version).unwrap());
    }
}

#[test]
fn mismatched_or_incomplete_spool_markers_cannot_acquire_repair_authority() {
    for modification in [
        "UPDATE library_metadata_inventory_spool_contract SET contract_version = 3",
        "UPDATE library_metadata_inventory_spool_contract SET complete = 0",
    ] {
        let connection = current_catalog();
        connection
            .execute_batch("PRAGMA ignore_check_constraints = ON")
            .unwrap();
        connection.execute_batch(modification).unwrap();
        for version in 37..=44 {
            assert!(!inventory_shape_matches(&connection, version).unwrap());
        }
    }
}
