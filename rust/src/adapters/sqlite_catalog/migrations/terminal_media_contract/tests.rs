use rusqlite::Connection;

use super::super::initialize_schema;

#[test]
fn current_incomplete_terminal_media_contract_fails_closed() {
    let mut connection = Connection::open_in_memory().expect("catalog");
    initialize_schema(&mut connection).expect("fresh current catalog");
    connection
        .execute_batch(
            "DROP TABLE library_terminal_media_evidence_contract;
             CREATE TABLE library_terminal_media_evidence_contract (
               singleton INTEGER PRIMARY KEY,
               complete INTEGER NOT NULL
             );
             INSERT INTO library_terminal_media_evidence_contract(singleton, complete)
               VALUES (1, 0);",
        )
        .expect("incomplete terminal evidence contract fixture");

    let error = initialize_schema(&mut connection).expect_err("incomplete evidence contract");

    assert_eq!(
        error.code,
        "catalog_terminal_media_evidence_contract_unverifiable"
    );
}
