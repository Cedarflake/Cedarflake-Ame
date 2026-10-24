use rusqlite::{Connection, OptionalExtension, params};

use crate::domain::ScanError;
use crate::domain::organization::{
    OrganizationHistory, OrganizationHistoryDirection, OrganizationHistoryReceipt,
};

use super::super::database_error;
use super::super::exact_duplicates::read_unsigned;
use super::{operations, undo};

pub(super) mod authority;

pub(super) fn available(connection: &Connection) -> Result<bool, ScanError> {
    connection
        .query_row("SELECT version >= 56 FROM schema_info", [], |row| {
            row.get(0)
        })
        .map_err(database_error)
}

pub(super) fn load(connection: &Connection) -> Result<OrganizationHistory, ScanError> {
    connection.query_row(
        "SELECT state.revision,
           (SELECT operation_id FROM organization_history_entries WHERE position = history.cursor),
           (SELECT operation_id FROM organization_history_entries WHERE position = history.cursor + 1)
         FROM organization_history_state AS history, organization_state AS state",
        [], |row| Ok(OrganizationHistory {
            organization_revision: read_unsigned(row, 0)?,
            undo_operation_id: row.get(1)?,
            redo_operation_id: row.get(2)?,
        }),
    ).map_err(database_error)
}

pub(super) fn step(
    connection: &Connection,
    operation_id: &str,
    direction: OrganizationHistoryDirection,
    original_id: &str,
    expected_revision: u64,
) -> Result<OrganizationHistoryReceipt, ScanError> {
    let kind = match direction {
        OrganizationHistoryDirection::Undo => "history-undo",
        OrganizationHistoryDirection::Redo => "history-redo",
    };
    let digest = operations::request_digest(kind, expected_revision, &[original_id]);
    let operation =
        match operations::admit(connection, operation_id, &digest, "undo", expected_revision)? {
            operations::Admission::Replayed(receipt) => receipt,
            operations::Admission::New { revision } => {
                let history = load(connection)?;
                let expected = match direction {
                    OrganizationHistoryDirection::Undo => history.undo_operation_id,
                    OrganizationHistoryDirection::Redo => history.redo_operation_id,
                };
                if expected.as_deref() != Some(original_id) {
                    return Err(ScanError::new(
                        "organization_history_changed",
                        "The next organization history operation has changed",
                    ));
                }
                undo::restore(connection, operation_id, original_id, revision)?
            }
        };
    let retired_album_id = connection
        .query_row(
            "SELECT change.album_id FROM organization_album_operation_changes AS change
         JOIN organization_album_retirements AS retirement ON retirement.album_id = change.album_id
         WHERE change.operation_id = ?1",
            [operation_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(database_error)?;
    Ok(OrganizationHistoryReceipt {
        operation,
        retired_album_id,
    })
}

pub(super) fn record(connection: &Connection, operation_id: &str) -> Result<(), ScanError> {
    if !available(connection)? {
        return Ok(());
    }
    let kind: String = connection
        .query_row(
            "SELECT operation_kind FROM organization_operations WHERE operation_id = ?1",
            [operation_id],
            |row| row.get(0),
        )
        .map_err(database_error)?;
    match kind.as_str() {
        "favorite" | "organization" | "album" => append(connection, operation_id),
        "restore" | "cleanup" => reset(connection),
        _ => Ok(()),
    }
}

pub(in crate::adapters::sqlite_catalog) fn reset(connection: &Connection) -> Result<(), ScanError> {
    if !available(connection)? {
        return Ok(());
    }
    connection
        .execute_batch(
            "DELETE FROM organization_history_entries;
         UPDATE organization_history_state SET cursor = 0,
           started_revision = (SELECT revision FROM organization_state);",
        )
        .map_err(database_error)
}

pub(super) fn inverted(
    connection: &Connection,
    operation_id: &str,
    original_id: &str,
) -> Result<(), ScanError> {
    if !available(connection)? {
        return Ok(());
    }
    let cursor: i64 = connection
        .query_row("SELECT cursor FROM organization_history_state", [], |row| {
            row.get(0)
        })
        .map_err(database_error)?;
    let position: Option<i64> = connection
        .query_row(
            "SELECT position FROM organization_history_entries WHERE operation_id = ?1
           AND (position = ?2 OR position = ?2 + 1)",
            params![original_id, cursor],
            |row| row.get(0),
        )
        .optional()
        .map_err(database_error)?;
    let Some(position) = position else {
        return append(connection, operation_id);
    };
    let next_cursor = if position == cursor {
        cursor - 1
    } else {
        position
    };
    connection
        .execute(
            "UPDATE organization_history_entries SET operation_id = ?1 WHERE position = ?2",
            params![operation_id, position],
        )
        .map_err(database_error)?;
    connection
        .execute(
            "UPDATE organization_history_state SET cursor = ?1",
            [next_cursor],
        )
        .map_err(database_error)?;
    Ok(())
}

fn append(connection: &Connection, operation_id: &str) -> Result<(), ScanError> {
    let cursor: i64 = connection
        .query_row("SELECT cursor FROM organization_history_state", [], |row| {
            row.get(0)
        })
        .map_err(database_error)?;
    let position = cursor.checked_add(1).ok_or_else(|| {
        ScanError::new(
            "organization_history_limit",
            "The organization history position has reached its limit",
        )
    })?;
    connection
        .execute(
            "DELETE FROM organization_history_entries WHERE position > ?1",
            [cursor],
        )
        .map_err(database_error)?;
    connection
        .execute(
            "INSERT INTO organization_history_entries(position, operation_id) VALUES (?1, ?2)",
            params![position, operation_id],
        )
        .map_err(database_error)?;
    connection
        .execute(
            "UPDATE organization_history_state SET cursor = ?1",
            [position],
        )
        .map_err(database_error)?;
    Ok(())
}
