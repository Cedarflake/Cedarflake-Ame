use super::*;
use crate::adapters::sqlite_catalog::migrations::current_schema;

#[test]
fn current_schema_rejects_reintroduced_redundant_index() {
    let connection = current_catalog();

    connection
        .execute_batch(
            "CREATE INDEX asset_locations_exact_cursor
               ON asset_locations(location_id, root_id, scan_id, file_size, source_generation);",
        )
        .unwrap();
    assert_eq!(
        current_schema::validate_current_schema_contract(&connection)
            .unwrap_err()
            .code,
        "catalog_exact_candidate_index_contract_invalid"
    );
}

fn current_catalog() -> Connection {
    let mut connection = Connection::open_in_memory().unwrap();
    super::super::initialize_schema(&mut connection).unwrap();
    connection
}
