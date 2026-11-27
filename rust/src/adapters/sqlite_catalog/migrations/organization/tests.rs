use super::*;
use crate::adapters::sqlite_catalog::migrations::current_schema;

#[test]
fn altered_organization_schema_is_rejected_before_current_queries() {
    let mut connection = current_catalog();
    super::super::initialize_schema(&mut connection).unwrap();
    connection
        .execute_batch("DROP INDEX organization_bindings_content")
        .unwrap();
    assert_eq!(
        current_schema::validate_current_schema_structure(&connection)
            .unwrap_err()
            .code,
        "catalog_organization_contract_invalid"
    );
}

fn current_catalog() -> Connection {
    let mut connection = Connection::open_in_memory().unwrap();
    super::super::initialize_schema(&mut connection).unwrap();
    connection
}
