use super::*;
use crate::adapters::sqlite_catalog::migrations::current_schema;

#[test]
fn missing_metadata_index_is_rejected_by_current_schema_validation() {
    let mut connection = current_catalog();

    super::super::initialize_schema(&mut connection).unwrap();
    connection
        .execute_batch("DROP INDEX asset_locations_pending_metadata")
        .unwrap();
    assert_eq!(
        current_schema::validate_current_schema_structure(&connection)
            .unwrap_err()
            .code,
        "catalog_metadata_recheck_index_invalid"
    );
}

fn current_catalog() -> Connection {
    let mut connection = Connection::open_in_memory().unwrap();
    super::super::initialize_schema(&mut connection).unwrap();
    connection
}
