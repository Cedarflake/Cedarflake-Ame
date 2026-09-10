use rusqlite::{ErrorCode, types::Value};

use crate::adapters::SqliteCatalog;

use super::*;

fn insert_retired_header(connection: &Connection, run_id: &Value) -> rusqlite::Result<usize> {
    connection.execute(
        "INSERT INTO library_metadata_inventory_spools(
           run_id, authority_change_id, root_id, root_generation,
           root_identity_scheme, root_identity_value, scope_kind, scope_relative_path,
           state, created_unix_ms, updated_unix_ms
         ) VALUES (?1, 1, 'removed-root', 1, 'fixture', 'retained-identity',
                   'root', '', 'retired', 1, 1)",
        [run_id],
    )
}

fn set_header_schema(connection: &Connection, ddl: &str, schema_version: i64) {
    connection
        .execute_batch("PRAGMA writable_schema = ON")
        .expect("enable controlled fixture corruption");
    assert_eq!(
        connection
            .execute(
                "UPDATE sqlite_master SET sql = ?1
                 WHERE type = 'table' AND name = 'library_metadata_inventory_spools'",
                [ddl],
            )
            .expect("replace only the fixture header definition"),
        1,
    );
    connection
        .execute_batch(&format!(
            "PRAGMA writable_schema = OFF; PRAGMA schema_version = {schema_version}"
        ))
        .expect("reload fixture schema");
}

#[test]
fn v32_header_identity_rejects_null_empty_and_oversized_inserts() {
    let mut connection = Connection::open_in_memory().expect("isolated catalog");
    migrate_schema(&mut connection).expect("production schema");
    for identity in [
        Value::Null,
        Value::Text(String::new()),
        Value::Text("x".repeat(257)),
    ] {
        let error = insert_retired_header(&connection, &identity)
            .expect_err("retired history still requires a bounded non-null identity");
        assert_eq!(
            error.sqlite_error_code(),
            Some(ErrorCode::ConstraintViolation)
        );
    }
    for identity in ["x".to_owned(), "x".repeat(256)] {
        insert_retired_header(&connection, &Value::Text(identity))
            .expect("valid historical identity boundary");
    }
    current_schema::validate_schema_version(&connection, 32)
        .expect("valid historical headers need no surviving execution parents");
}

#[test]
fn v32_header_identity_full_reopen_rejects_corrupt_rows_without_mutation() {
    for identity in [
        Value::Null,
        Value::Text(String::new()),
        Value::Text("x".repeat(257)),
        Value::Blob(vec![1]),
    ] {
        let storage = tempfile::tempdir().expect("isolated catalog storage");
        let path = storage.path().join("catalog.sqlite3");
        drop(SqliteCatalog::open(path.clone()).expect("initialize production catalog"));
        let connection = Connection::open(&path).expect("owned corruption fixture");
        let header = super::super::HEADER;
        let relaxed = header.replace(
            "run_id TEXT NOT NULL PRIMARY KEY CHECK(length(run_id) BETWEEN 1 AND 256)",
            "run_id TEXT PRIMARY KEY",
        );
        assert_ne!(relaxed, header);
        set_header_schema(&connection, &relaxed, 10_001);
        insert_retired_header(&connection, &identity).expect("seed malformed historical identity");
        set_header_schema(&connection, header, 10_002);
        assert!(super::super::schema_matches(&connection).expect("restore exact v32 DDL"));
        let error = super::super::super::inventory_spool_rows::validate(&connection, 4)
            .expect_err("row proof rejects the malformed identity under exact DDL");
        assert_eq!(
            error.code,
            "catalog_metadata_inventory_spool_contract_unverifiable",
        );
        let before = raw_evidence(&connection);
        drop(connection);

        let error = match SqliteCatalog::open(path.clone()) {
            Ok(_) => panic!("corrupt retired identity must fail full catalog reopen: {identity:?}"),
            Err(error) => error,
        };
        assert_eq!(
            error.code, "catalog_metadata_inventory_spool_contract_unverifiable",
            "{identity:?}: {error:?}",
        );
        let connection = Connection::open(&path).expect("read rejected fixture evidence");
        assert_eq!(raw_evidence(&connection), before);
        let versions: (i64, i64) = connection
            .query_row(
                "SELECT (SELECT version FROM schema_info),
                        (SELECT user_version FROM pragma_user_version)",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("rejected reopen preserves schema markers");
        assert_eq!(versions, (32, 32));
    }
}

#[test]
fn v32_header_identity_migration_rejects_null_legacy_header_atomically() {
    let mut connection = v31_catalog();
    connection
        .execute(
            "DELETE FROM library_metadata_inventory_spools WHERE run_id = 'root-1'",
            [],
        )
        .expect("release only the first fixture header's unique authority binding");
    current_schema::validate_schema_version(&connection, 31)
        .expect("the legacy graph remains valid before identity corruption");
    connection
        .execute_batch(
            "INSERT INTO library_metadata_inventory_spools(
               run_id, authority_change_id, root_id, root_generation,
               root_identity_scheme, root_identity_value, scope_kind, scope_relative_path,
               state, created_unix_ms, updated_unix_ms
             ) VALUES (NULL, 1, 'root-1', 1, 'fixture', 'retained-identity',
                       'subtree', 'album', 'ready', 1, 1)",
        )
        .expect("legacy nullable primary key accepts malformed identity");
    let before = raw_evidence(&connection);
    let error = migrate_schema(&mut connection).expect_err("reject unprovable historical header");
    assert_eq!(
        error.code,
        "catalog_metadata_inventory_spool_contract_unverifiable",
    );
    assert_eq!(raw_evidence(&connection), before);
    let version: i64 = connection
        .query_row("SELECT version FROM schema_info", [], |row| row.get(0))
        .expect("failed migration preserves the legacy version");
    assert_eq!(version, 31);
}
