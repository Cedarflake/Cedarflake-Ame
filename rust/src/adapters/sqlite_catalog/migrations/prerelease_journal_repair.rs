use rusqlite::{Connection, OptionalExtension, TransactionBehavior};

use crate::domain::ScanError;

use super::journal_contract::schema::{
    PERSISTENT_JOURNAL_CANONICAL_TRIGGER_DDL, PERSISTENT_JOURNAL_LEGACY_V24_TRIGGER_DDL,
};
use super::schema_shape::normalize_schema_sql;
use super::{
    LEGACY_V26_PERSISTENT_JOURNAL_BASELINE_INSERT_GUARD_DDL,
    LEGACY_V26_PERSISTENT_JOURNAL_BASELINE_ROOT_INDEX_DDL,
    PERSISTENT_JOURNAL_BASELINE_INSERT_GUARD_DDL, PERSISTENT_JOURNAL_BASELINE_ROOT_INDEX_DDL,
    database_error, journal_contract,
};

pub(super) fn repair_range_id_triggers(connection: &mut Connection) -> Result<(), ScanError> {
    let mut current_count = 0;
    let mut legacy_count = 0;
    for ((name, current), (legacy_name, legacy)) in PERSISTENT_JOURNAL_CANONICAL_TRIGGER_DDL
        .iter()
        .zip(PERSISTENT_JOURNAL_LEGACY_V24_TRIGGER_DDL)
    {
        if name != legacy_name {
            return Err(journal_contract::values::unverifiable_contract());
        }
        let actual = connection
            .query_row(
                "SELECT sql FROM sqlite_master WHERE type = 'trigger' AND name = ?1",
                [name],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(database_error)?;
        let Some(actual) = actual else {
            return Ok(());
        };
        current_count +=
            usize::from(normalize_schema_sql(&actual) == normalize_schema_sql(current));
        legacy_count += usize::from(normalize_schema_sql(&actual) == normalize_schema_sql(legacy));
    }
    if current_count == PERSISTENT_JOURNAL_CANONICAL_TRIGGER_DDL.len() {
        return Ok(());
    }
    if legacy_count != PERSISTENT_JOURNAL_LEGACY_V24_TRIGGER_DDL.len() {
        return Ok(());
    }
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(database_error)?;
    for (name, sql) in PERSISTENT_JOURNAL_CANONICAL_TRIGGER_DDL {
        transaction
            .execute_batch(&format!("DROP TRIGGER {name};"))
            .map_err(database_error)?;
        transaction.execute_batch(sql).map_err(database_error)?;
    }
    transaction.commit().map_err(database_error)
}

pub(super) fn repair_recovery_window(connection: &mut Connection) -> Result<(), ScanError> {
    let index_sql = connection
        .query_row(
            "SELECT sql FROM sqlite_master
             WHERE type = 'index' AND name = 'library_persistent_journal_baselines_root'",
            [],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(database_error)?;
    let trigger_sql = connection
        .query_row(
            "SELECT sql FROM sqlite_master
             WHERE type = 'trigger'
               AND name = 'library_persistent_journal_baseline_insert_guard'",
            [],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(database_error)?;
    let is_current = index_sql.as_deref().is_some_and(|sql| {
        normalize_schema_sql(sql)
            == normalize_schema_sql(PERSISTENT_JOURNAL_BASELINE_ROOT_INDEX_DDL)
    }) && trigger_sql.as_deref().is_some_and(|sql| {
        normalize_schema_sql(sql)
            == normalize_schema_sql(PERSISTENT_JOURNAL_BASELINE_INSERT_GUARD_DDL)
    });
    if is_current {
        return Ok(());
    }
    let is_legacy = index_sql.as_deref().is_some_and(|sql| {
        normalize_schema_sql(sql)
            == normalize_schema_sql(LEGACY_V26_PERSISTENT_JOURNAL_BASELINE_ROOT_INDEX_DDL)
    }) && trigger_sql.as_deref().is_some_and(|sql| {
        normalize_schema_sql(sql)
            == normalize_schema_sql(LEGACY_V26_PERSISTENT_JOURNAL_BASELINE_INSERT_GUARD_DDL)
    });
    if !is_legacy {
        return Ok(());
    }
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(database_error)?;
    transaction
        .execute_batch(
            "DROP INDEX library_persistent_journal_baselines_root;
             DROP TRIGGER library_persistent_journal_baseline_insert_guard;",
        )
        .map_err(database_error)?;
    transaction
        .execute_batch(PERSISTENT_JOURNAL_BASELINE_ROOT_INDEX_DDL)
        .map_err(database_error)?;
    transaction
        .execute_batch(PERSISTENT_JOURNAL_BASELINE_INSERT_GUARD_DDL)
        .map_err(database_error)?;
    transaction.commit().map_err(database_error)
}

#[cfg(test)]
mod tests;
