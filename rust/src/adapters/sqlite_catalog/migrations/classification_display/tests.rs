use super::*;

fn previous_catalog() -> Connection {
    let connection = Connection::open_in_memory().unwrap();
    connection
        .execute_batch("PRAGMA foreign_keys = ON;")
        .unwrap();
    connection
        .execute_batch(include_str!("../initialization.sql"))
        .unwrap();
    connection
        .pragma_update(None, "application_id", SQLITE_APPLICATION_ID)
        .unwrap();
    connection.pragma_update(None, "user_version", 83).unwrap();
    connection
}

fn version(connection: &Connection) -> i64 {
    connection
        .query_row("SELECT version FROM schema_info", [], |row| row.get(0))
        .unwrap()
}

fn display_objects(connection: &Connection) -> i64 {
    connection
        .query_row(
            "SELECT COUNT(*) FROM sqlite_schema WHERE name LIKE 'classification_display_%'
              OR name IN ('classification_current_display_projection', 'classification_retained_display_projection')",
            [],
            |row| row.get(0),
        )
        .unwrap()
}

#[test]
fn display_upgrade_rolls_back_ddl_and_version_on_failure_and_retries_atomically() {
    let mut connection = previous_catalog();
    connection
        .execute_batch(
            "UPDATE catalog_state SET revision = 37;
         CREATE TEMP TRIGGER reject_upgrade BEFORE UPDATE ON schema_info
         BEGIN SELECT RAISE(ABORT, 'injected upgrade failure'); END;",
        )
        .unwrap();
    assert!(upgrade(&mut connection).is_err());
    assert_eq!(version(&connection), 83);
    assert_eq!(display_objects(&connection), 0);
    connection
        .execute_batch("DROP TRIGGER reject_upgrade;")
        .unwrap();
    upgrade(&mut connection).unwrap();
    upgrade(&mut connection).unwrap();
    assert_eq!(version(&connection), 84);
    assert_eq!(display_objects(&connection), 6);
    assert_eq!(
        connection
            .query_row("SELECT revision FROM catalog_state", [], |row| row
                .get::<_, i64>(0))
            .unwrap(),
        37
    );
    validate(&connection).unwrap();
    validate_rows(&connection).unwrap();
}

#[test]
fn display_upgrade_rejects_malformed_previous_schema_without_partial_repair() {
    let mut connection = previous_catalog();
    connection
        .execute_batch("DROP INDEX classification_predictions_content;")
        .unwrap();
    assert!(upgrade(&mut connection).is_err());
    assert_eq!(version(&connection), 83);
    assert_eq!(display_objects(&connection), 0);
    assert_eq!(
        connection
            .query_row("PRAGMA user_version", [], |row| row.get::<_, i64>(0))
            .unwrap(),
        83
    );
}
