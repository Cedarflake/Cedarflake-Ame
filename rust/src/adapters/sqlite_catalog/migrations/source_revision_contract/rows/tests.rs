use super::*;

fn fixture() -> Connection {
    let connection = Connection::open_in_memory().unwrap();
    connection.execute_batch(
        "CREATE TABLE asset_locations(scan_id TEXT, root_id TEXT, file_identity_scheme TEXT,
           file_identity_value TEXT, source_generation INTEGER, source_revision_token TEXT,
           file_size INTEGER, modified_unix_ms INTEGER);
         CREATE INDEX asset_locations_active_file_identity ON asset_locations(file_identity_scheme, file_identity_value, scan_id);
         CREATE INDEX asset_locations_file_identity ON asset_locations(scan_id, file_identity_scheme, file_identity_value);
         CREATE TABLE library_roots(id TEXT, active_scan_id TEXT);
         CREATE TABLE preview_artifacts(lifecycle_state TEXT, source_generation INTEGER, source_revision_token TEXT);
         CREATE TABLE catalog_state(next_source_generation INTEGER);
         INSERT INTO catalog_state VALUES(100);
         INSERT INTO library_roots VALUES('root-a','scan-a'),('root-b','scan-b');
         INSERT INTO asset_locations VALUES
           ('scan-a','root-a','windows','same-file',1,'windows-file-change-time-100ns-v1:0000000000000001',10,20),
           ('scan-b','root-b','windows','same-file',1,'windows-file-change-time-100ns-v1:0000000000000001',10,20);"
    ).unwrap();
    connection
}

#[test]
fn rejects_disagreement_in_active_cross_root_and_staged_same_scan_aliases() {
    for staged in [false, true] {
        for change in [
            "source_generation = 2",
            "file_size = 11",
            "modified_unix_ms = 21",
            "source_revision_token = 'windows-file-change-time-100ns-v1:0000000000000002'",
        ] {
            let connection = fixture();
            if staged {
                connection
                    .execute("UPDATE asset_locations SET scan_id = 'staged'", [])
                    .unwrap();
            }
            validate(&connection).expect("consistent aliases");
            connection
                .execute(
                    &format!("UPDATE asset_locations SET {change} WHERE root_id = 'root-b'"),
                    [],
                )
                .unwrap();
            assert_eq!(
                validate(&connection).unwrap_err().code,
                "catalog_source_revision_contract_unverifiable",
                "staged={staged}, {change}"
            );
        }
    }
}

#[test]
fn retains_unknown_revision_and_independent_scan_semantics() {
    let connection = fixture();
    connection
        .execute(
            "UPDATE asset_locations SET source_revision_token = NULL WHERE root_id = 'root-b'",
            [],
        )
        .unwrap();
    validate(&connection).expect("one known revision may coexist with an unknown alias revision");
    connection.execute("UPDATE asset_locations SET scan_id = 'old', source_generation = 2, file_size = 11 WHERE root_id = 'root-b'", []).unwrap();
    validate(&connection)
        .expect("different unpublished scans have independent source observations");
    connection
        .execute(
            "UPDATE asset_locations SET source_generation = 0 WHERE scan_id = 'old'",
            [],
        )
        .unwrap();
    assert_eq!(
        validate(&connection).unwrap_err().code,
        "catalog_source_revision_contract_unverifiable"
    );
}
