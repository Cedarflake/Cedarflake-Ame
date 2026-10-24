use rusqlite::{Connection, Transaction, TransactionBehavior};

use crate::domain::ScanError;

use super::{current_schema, database_error, schema_shape::schema_object_sql_matches};

const TABLE: &str = "CREATE TABLE library_metadata_fallbacks (
    change_id INTEGER PRIMARY KEY,
    admitted_unix_ms INTEGER NOT NULL CHECK(admitted_unix_ms >= 0),
    FOREIGN KEY(change_id) REFERENCES library_recovery_authorities(change_id) ON DELETE CASCADE
)";
const GUARD: &str = "CREATE TRIGGER library_metadata_fallback_insert_guard
    BEFORE INSERT ON library_metadata_fallbacks
    WHEN NOT EXISTS (
      SELECT 1 FROM library_recovery_authorities AS authority
      JOIN library_change_queue AS control ON control.id = authority.change_id
      JOIN library_persistent_journal_checkpoints AS checkpoint
        ON checkpoint.root_id = authority.root_id
       AND checkpoint.root_generation = authority.root_generation
      WHERE authority.change_id = NEW.change_id AND authority.retired_unix_ms IS NULL
        AND control.scope = 'root' AND control.origin = 'metadata_inventory'
        AND control.status IN ('pending', 'leased', 'retry_wait')
        AND authority.reason IN (
          'journal_gap', 'journal_reset', 'journal_trim', 'journal_reconstruction_failure',
          'containment_failure', 'broker_after_current_failure', 'existing_root_baseline', 'watcher_uncovered_gap'
        )
        AND checkpoint.continuity_state = 'recovery_required'
        AND checkpoint.last_failure_code IS NOT NULL
        AND NEW.admitted_unix_ms >= authority.authorized_unix_ms
    )
    BEGIN SELECT RAISE(ABORT, 'metadata fallback requires a failed journal and root recovery'); END";

const UPDATE_GUARD: &str = "CREATE TRIGGER library_metadata_fallback_update_guard
    BEFORE UPDATE ON library_metadata_fallbacks
    BEGIN SELECT RAISE(ABORT, 'metadata fallback admission is immutable'); END";

pub(super) fn migrate(connection: &mut Connection) -> Result<(), ScanError> {
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(database_error)?;
    migrate_transaction(&transaction)?;
    transaction.commit().map_err(database_error)
}

pub(super) fn migrate_transaction(transaction: &Transaction<'_>) -> Result<(), ScanError> {
    current_schema::validate_schema_version(transaction, 45)?;
    transaction.execute_batch(TABLE).map_err(database_error)?;
    transaction.execute_batch(GUARD).map_err(database_error)?;
    transaction
        .execute_batch(UPDATE_GUARD)
        .map_err(database_error)?;
    transaction
        .execute_batch("UPDATE schema_info SET version = 46; PRAGMA user_version = 46;")
        .map_err(database_error)?;
    current_schema::validate_schema_version(transaction, 46)
}

pub(super) fn validate(connection: &Connection) -> Result<(), ScanError> {
    for (kind, name, ddl) in [
        ("table", "library_metadata_fallbacks", TABLE),
        (
            "trigger",
            "library_metadata_fallback_update_guard",
            UPDATE_GUARD,
        ),
        ("trigger", "library_metadata_fallback_insert_guard", GUARD),
    ] {
        if !schema_object_sql_matches(connection, kind, name, ddl)? {
            return Err(invalid());
        }
    }
    Ok(())
}

pub(super) fn validate_rows(connection: &Connection) -> Result<(), ScanError> {
    let invalid_rows: bool = connection.query_row(
        "SELECT EXISTS (
           SELECT 1 FROM library_metadata_fallbacks AS fallback
           LEFT JOIN library_recovery_authorities AS authority ON authority.change_id = fallback.change_id
           LEFT JOIN library_change_queue AS control ON control.id = fallback.change_id
           LEFT JOIN library_metadata_inventory_runs AS inventory ON inventory.id = authority.run_id
           WHERE authority.change_id IS NULL OR control.id IS NULL
             OR control.scope <> 'root' OR control.origin <> 'metadata_inventory'
             OR control.root_id <> authority.root_id
             OR control.root_generation <> authority.root_generation
             OR fallback.admitted_unix_ms < authority.authorized_unix_ms
             OR (control.status = 'completed' AND (
               authority.retired_unix_ms IS NULL OR inventory.status IS NOT 'completed'
               OR inventory.enumeration_complete IS NOT 1 OR inventory.absence_authority IS NOT 1
             ))
         ) OR EXISTS (SELECT 1 FROM pragma_foreign_key_check('library_metadata_fallbacks'))",
        [], |row| row.get(0),
    ).map_err(database_error)?;
    if invalid_rows {
        return Err(invalid());
    }
    Ok(())
}

fn invalid() -> ScanError {
    ScanError::new(
        "catalog_metadata_fallback_invalid",
        "Independent recovery lacks its durable admission or complete inventory",
    )
}

#[cfg(test)]
pub(super) fn downgrade_to_v45_for_test(connection: &Connection) {
    super::organization::downgrade_to_v46_for_test(connection);
    if super::schema_shape::schema_version(connection).unwrap() == 46 {
        connection
            .execute_batch(
                "DROP TRIGGER library_metadata_fallback_insert_guard;
            DROP TRIGGER library_metadata_fallback_update_guard;
            DROP TABLE library_metadata_fallbacks;
            UPDATE schema_info SET version = 45; PRAGMA user_version = 45;",
            )
            .unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn upgrade_preserves_existing_catalog_and_rejects_unowned_fallback() {
        let mut connection = Connection::open_in_memory().unwrap();
        super::super::migrate_schema(&mut connection).unwrap();
        downgrade_to_v45_for_test(&connection);
        let revision: i64 = connection
            .query_row("SELECT revision FROM catalog_state", [], |row| row.get(0))
            .unwrap();
        super::super::migrate_schema(&mut connection).unwrap();
        assert_eq!(
            super::super::schema_shape::schema_version(&connection).unwrap(),
            super::super::SCHEMA_VERSION
        );
        assert_eq!(
            connection
                .query_row("SELECT revision FROM catalog_state", [], |row| row
                    .get::<_, i64>(0))
                .unwrap(),
            revision
        );
        assert!(
            connection
                .execute("INSERT INTO library_metadata_fallbacks VALUES(999, 1)", [])
                .is_err()
        );
        validate_rows(&connection).unwrap();
        connection
            .execute_batch("DROP TRIGGER library_metadata_fallback_insert_guard")
            .unwrap();
        assert!(validate(&connection).is_err());
    }
}
