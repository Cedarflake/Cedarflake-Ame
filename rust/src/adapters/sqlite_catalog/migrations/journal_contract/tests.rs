use rusqlite::Connection;

use super::super::migrate_schema;
use super::super::schema_shape::normalize_schema_sql;

#[test]
fn current_v22_malformed_source_range_shape_fails_closed() {
    let mut connection = Connection::open_in_memory().expect("catalog");
    migrate_schema(&mut connection).expect("fresh current catalog");
    connection
        .execute_batch(
            "DROP INDEX library_persistent_journal_source_ranges_volume;
             CREATE INDEX library_persistent_journal_source_ranges_volume
               ON library_persistent_journal_source_ranges(
                 volume_guid, requested_start_usn, id
               );",
        )
        .expect("malformed v22 index fixture");

    let error = migrate_schema(&mut connection).expect_err("malformed v22 shape");

    assert_eq!(
        error.code,
        "catalog_persistent_journal_contract_unverifiable"
    );
}

#[test]
fn current_v22_rejects_replaced_check_with_the_same_check_count() {
    let mut connection = Connection::open_in_memory().expect("catalog");
    migrate_schema(&mut connection).expect("fresh current catalog");
    let original = connection
        .query_row(
            "SELECT sql FROM sqlite_master
             WHERE type = 'table' AND name = 'library_persistent_journal_root_state'",
            [],
            |row| row.get::<_, String>(0),
        )
        .expect("canonical root-state DDL");
    let modified = original.replacen("CHECK(updated_unix_ms >= 0)", "CHECK(1)", 1);
    assert_ne!(modified, original);
    assert_eq!(
        normalize_schema_sql(&modified).matches("check(").count(),
        normalize_schema_sql(&original).matches("check(").count()
    );
    connection
        .execute_batch("PRAGMA writable_schema = ON")
        .expect("enable controlled schema mutation");
    connection
        .execute(
            "UPDATE sqlite_master SET sql = ?1
             WHERE type = 'table' AND name = 'library_persistent_journal_root_state'",
            [modified],
        )
        .expect("replace one CHECK");
    connection
        .execute_batch("PRAGMA writable_schema = OFF; PRAGMA schema_version = 999")
        .expect("publish controlled schema mutation");

    let error = migrate_schema(&mut connection).expect_err("weakened CHECK must fail closed");
    assert_eq!(
        error.code,
        "catalog_persistent_journal_contract_unverifiable"
    );
}

#[test]
fn current_v24_rejects_quoted_schema_literal_mutations() {
    let mutations = [
        (
            "trigger",
            "library_persistent_journal_source_range_id_insert",
            "'*[^0-9a-f]*'",
            "'*[^0-9A-F]*'",
        ),
        (
            "trigger",
            "library_persistent_journal_source_range_id_insert",
            "'text'",
            "'TEXT'",
        ),
        (
            "trigger",
            "library_persistent_journal_source_range_id_insert",
            "'invalid persistent journal source range id'",
            "'invalid  persistent journal source range id'",
        ),
        (
            "trigger",
            "library_persistent_journal_source_range_id_insert",
            "'invalid persistent journal source range id'",
            "'invalid persistent journal source range ''id'''",
        ),
        (
            "table",
            "library_persistent_journal_source_ranges",
            "'enrolled'",
            "'ENROLLED'",
        ),
    ];
    for (index, (object_type, name, original, replacement)) in mutations.into_iter().enumerate() {
        let mut connection = Connection::open_in_memory().expect("catalog");
        migrate_schema(&mut connection).expect("fresh current catalog");
        let sql = connection
            .query_row(
                "SELECT sql FROM sqlite_master WHERE type = ?1 AND name = ?2",
                [object_type, name],
                |row| row.get::<_, String>(0),
            )
            .expect("canonical schema object");
        let modified = sql.replacen(original, replacement, 1);
        assert_ne!(modified, sql, "mutation {index} must change schema SQL");
        connection
            .execute_batch("PRAGMA writable_schema = ON")
            .expect("enable controlled schema mutation");
        connection
            .execute(
                "UPDATE sqlite_master SET sql = ?1 WHERE type = ?2 AND name = ?3",
                rusqlite::params![modified, object_type, name],
            )
            .expect("mutate quoted schema token");
        connection
            .execute_batch("PRAGMA writable_schema = OFF; PRAGMA schema_version = 1001")
            .expect("publish controlled schema mutation");

        let error = migrate_schema(&mut connection)
            .expect_err("quoted schema token mutation must fail closed");
        assert_eq!(
            error.code, "catalog_persistent_journal_contract_unverifiable",
            "unexpected mutation {index} error"
        );
    }
}
