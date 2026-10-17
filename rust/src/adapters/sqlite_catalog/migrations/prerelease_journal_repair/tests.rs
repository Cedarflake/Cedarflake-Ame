use rusqlite::Connection;

use super::super::journal_contract::schema::{
    PERSISTENT_JOURNAL_CANONICAL_TRIGGER_DDL, PERSISTENT_JOURNAL_LEGACY_V24_TRIGGER_DDL,
};
use super::super::migrate_schema;
use super::super::schema_shape::normalize_schema_sql;

#[test]
fn prerelease_v24_id_triggers_upgrade_atomically_to_typed_guards() {
    let mut connection = Connection::open_in_memory().expect("catalog");
    migrate_schema(&mut connection).expect("fresh v24 catalog");
    for (name, sql) in PERSISTENT_JOURNAL_LEGACY_V24_TRIGGER_DDL {
        connection
            .execute_batch(&format!("DROP TRIGGER {name};"))
            .expect("drop typed trigger");
        connection
            .execute_batch(sql)
            .expect("install legacy trigger");
    }

    migrate_schema(&mut connection).expect("upgrade exact legacy v24 triggers");
    for (name, expected) in PERSISTENT_JOURNAL_CANONICAL_TRIGGER_DDL {
        let actual = connection
            .query_row(
                "SELECT sql FROM sqlite_master WHERE type = 'trigger' AND name = ?1",
                [name],
                |row| row.get::<_, String>(0),
            )
            .expect("upgraded trigger DDL");
        assert_eq!(
            normalize_schema_sql(&actual),
            normalize_schema_sql(expected)
        );
    }
    for invalid in [
        rusqlite::types::Value::Null,
        rusqlite::types::Value::Integer(7),
    ] {
        let error = connection
            .execute(
                "INSERT INTO library_persistent_journal_source_ranges(id) VALUES (?1)",
                [invalid],
            )
            .expect_err("typed ID guard must run before row constraints");
        assert!(
            error
                .to_string()
                .contains("invalid persistent journal source range id")
        );
    }
}
