use rusqlite::{Connection, OptionalExtension, Transaction, params};

use super::super::{database_error, sqlite_integer, sqlite_unsigned};
use crate::domain::{LibraryChangeId, ScanError};

pub(super) fn allows_absence(
    connection: &Connection,
    change_id: LibraryChangeId,
) -> Result<bool, ScanError> {
    if super::super::persistent_journal::metadata_fallback::owns(connection, change_id)? {
        return Ok(true);
    }
    let (has_window, needs_opening) = connection.query_row(
        "SELECT EXISTS (
           SELECT 1 FROM library_persistent_journal_baselines
           WHERE change_id = ?1 AND phase <> 'completed'
         ), EXISTS (
           SELECT 1 FROM library_recovery_authorities AS authority
           JOIN library_persistent_journal_root_state AS root
             ON root.root_id = authority.root_id AND root.root_generation = authority.root_generation
           WHERE authority.change_id = ?1 AND authority.retired_unix_ms IS NULL
             AND authority.reason = 'broker_after_current_failure'
             AND root.capability_state = 'supported'
         )",
        [sqlite_integer(change_id.value(), "recovery change ID")?],
        |row| Ok((row.get::<_, bool>(0)?, row.get::<_, bool>(1)?)),
    ).map_err(database_error)?;
    if has_window {
        return super::super::persistent_journal::baseline_closing_is_covered(
            connection, change_id,
        );
    }
    Ok(!needs_opening)
}

pub(super) fn authorize_absence(
    transaction: &Transaction<'_>,
    run_id: &str,
    updated_unix_ms: i64,
) -> Result<(), ScanError> {
    let change_id = transaction
        .query_row(
            "SELECT change_id FROM library_recovery_authorities
         WHERE run_id = ?1 AND retired_unix_ms IS NULL",
            [run_id],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .map_err(database_error)?;
    let Some(change_id) = change_id else {
        return Ok(());
    };
    let change_id = LibraryChangeId::new(sqlite_unsigned(change_id, "recovery change ID")?)
        .ok_or_else(|| {
            ScanError::new(
                "persistent_journal_baseline_corrupt",
                "The recovery change ID is invalid",
            )
        })?;
    if !allows_absence(transaction, change_id)? {
        return Err(ScanError::new(
            "persistent_journal_baseline_closing_incomplete",
            "Absence cannot become authoritative before closing replay completes",
        ));
    }
    if super::super::persistent_journal::metadata_fallback::owns(transaction, change_id)? {
        return Ok(());
    }
    transaction
        .execute(
            "UPDATE library_persistent_journal_baselines
         SET phase = 'absence', updated_unix_ms = ?2
         WHERE change_id = ?1 AND phase = 'replay'",
            params![
                sqlite_integer(change_id.value(), "recovery change ID")?,
                updated_unix_ms
            ],
        )
        .map_err(database_error)?;
    Ok(())
}

pub(super) fn unfinished_baseline_change_id(
    connection: &Connection,
    root_id: &str,
    generation: crate::domain::LibraryRootGeneration,
) -> Result<Option<i64>, ScanError> {
    connection
        .query_row(
            "SELECT baseline.change_id
         FROM library_persistent_journal_baselines AS baseline
         WHERE baseline.root_id = ?1 AND baseline.root_generation = ?2
           AND baseline.phase <> 'completed'
           AND NOT EXISTS (SELECT 1 FROM library_metadata_fallbacks AS fallback
             WHERE fallback.change_id = baseline.change_id)",
            params![
                root_id,
                sqlite_integer(generation.value(), "root generation")?
            ],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .map_err(database_error)?
        .map(|change_id| {
            sqlite_unsigned(change_id, "metadata inventory baseline change ID").map(|_| change_id)
        })
        .transpose()
}
