use rusqlite::Connection;

use super::super::{self as migrations, initialize_schema};

#[test]
fn current_source_revision_structure_damage_still_fails_closed() {
    let mut connection = Connection::open_in_memory().expect("current catalog");
    initialize_schema(&mut connection).expect("create current catalog");
    connection
        .execute_batch("DROP TRIGGER asset_locations_source_generation_update_guard;")
        .expect("damage source revision structure");

    super::reset_row_audit_count();
    let error = initialize_schema(&mut connection).expect_err("reject damaged current catalog");

    assert_eq!(error.code, "catalog_source_revision_contract_unverifiable");
    assert_eq!(super::row_audit_count(), 0);
}

#[test]
fn current_source_revision_marker_damage_still_fails_closed() {
    let mut connection = Connection::open_in_memory().expect("current catalog");
    initialize_schema(&mut connection).expect("create current catalog");
    connection
        .execute_batch("DELETE FROM library_source_revision_metadata_contract;")
        .expect("damage source revision marker");

    migrations::current_schema::reset_current_schema_row_audit_count();
    let error = initialize_schema(&mut connection).expect_err("reject damaged current catalog");

    assert_eq!(error.code, "catalog_source_revision_contract_unverifiable");
    assert_eq!(
        migrations::current_schema::current_schema_row_audit_count(),
        0
    );
}
