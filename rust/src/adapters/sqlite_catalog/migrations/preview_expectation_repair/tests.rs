use super::super::initialize_schema;
use super::*;

#[test]
fn current_malformed_preview_repair_marker_fails_closed() {
    let mut connection = Connection::open_in_memory().expect("catalog");
    initialize_schema(&mut connection).expect("fresh current catalog");
    connection
        .execute_batch(
            "DROP TABLE library_change_preview_repair_contract;
             CREATE TABLE library_change_preview_repair_contract(
               singleton INTEGER PRIMARY KEY
             );
             INSERT INTO library_change_preview_repair_contract(singleton) VALUES (1);",
        )
        .expect("malformed preview repair marker fixture");

    let error = initialize_schema(&mut connection).expect_err("malformed preview repair marker");

    assert_eq!(error.code, "catalog_change_catch_up_contract_unverifiable");
}
