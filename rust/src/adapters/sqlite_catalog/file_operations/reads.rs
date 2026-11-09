use rusqlite::{Connection, Row, params};

use crate::domain::ScanError;
use crate::domain::file_operation_journal::{FileOperationRecord, invalid};

use super::super::{database_error, sqlite_integer, sqlite_unsigned};
use super::codec;

const CURRENT: &str =
    "SELECT i.id, i.authorization, e.snapshot, e.recorded_unix_ms, i.version, i.phase
    FROM file_operation_items AS i
    JOIN file_operation_events AS e ON e.item_id = i.id AND e.version = i.version";

fn decode(row: &Row<'_>) -> rusqlite::Result<(i64, String, String, i64, i64, String)> {
    Ok((
        row.get(0)?,
        row.get(1)?,
        row.get(2)?,
        row.get(3)?,
        row.get(4)?,
        row.get(5)?,
    ))
}

fn record(
    value: (i64, String, String, i64, i64, String),
) -> Result<FileOperationRecord, ScanError> {
    let (id, authorization, snapshot, updated_unix_ms, version, phase) = value;
    let checkpoint = codec::decode_checkpoint(&snapshot)?;
    if checkpoint.version != sqlite_unsigned(version, "operation version")?
        || codec::phase_name(checkpoint.phase) != phase
    {
        return Err(invalid(
            "The operation checkpoint does not match its journal head",
        ));
    }
    Ok(FileOperationRecord {
        id: sqlite_unsigned(id, "operation ID")?,
        authorization: codec::decode_authorization(&authorization)?,
        checkpoint,
        updated_unix_ms,
    })
}

pub(super) fn batch_count(connection: &Connection, batch_id: &str) -> Result<u64, ScanError> {
    let count = connection
        .query_row(
            "SELECT COUNT(*) FROM file_operation_items WHERE batch_id = ?1",
            [batch_id],
            |row| row.get::<_, i64>(0),
        )
        .map_err(database_error)?;
    sqlite_unsigned(count, "batch item count")
}

pub(super) fn batch_page(
    connection: &Connection,
    batch_id: &str,
    after_ordinal: Option<u64>,
    limit: usize,
) -> Result<Vec<FileOperationRecord>, ScanError> {
    let after = after_ordinal
        .map(|value| sqlite_integer(value, "batch cursor"))
        .transpose()?
        .unwrap_or(-1);
    let mut statement = connection
        .prepare(&format!(
            "{CURRENT} WHERE i.batch_id = ?1 AND i.ordinal > ?2 ORDER BY i.ordinal LIMIT ?3"
        ))
        .map_err(database_error)?;
    let rows = statement
        .query_map(params![batch_id, after, limit as i64], decode)
        .map_err(database_error)?;
    rows.map(|row| record(row.map_err(database_error)?))
        .collect()
}

pub(super) fn load(connection: &Connection, id: u64) -> Result<FileOperationRecord, ScanError> {
    let value = connection
        .query_row(
            &format!("{CURRENT} WHERE i.id = ?1"),
            [sqlite_integer(id, "operation ID")?],
            decode,
        )
        .map_err(database_error)?;
    record(value)
}

pub(super) fn history(
    connection: &Connection,
    before_id: Option<u64>,
    limit: usize,
) -> Result<Vec<FileOperationRecord>, ScanError> {
    let before = before_id
        .map(|value| sqlite_integer(value, "history cursor"))
        .transpose()?
        .unwrap_or(i64::MAX);
    let mut statement = connection
        .prepare(&format!(
            "{CURRENT} WHERE i.id < ?1 ORDER BY i.id DESC LIMIT ?2"
        ))
        .map_err(database_error)?;
    let rows = statement
        .query_map(params![before, limit as i64], decode)
        .map_err(database_error)?;
    rows.map(|row| record(row.map_err(database_error)?))
        .collect()
}

pub(super) fn unfinished(
    connection: &Connection,
    after_id: u64,
    limit: usize,
) -> Result<Vec<FileOperationRecord>, ScanError> {
    let mut statement = connection.prepare(&format!("{CURRENT}
        WHERE i.id > ?1 AND i.phase IN ('prepared', 'executing', 'name_occupied', 'copied', 'removing_source', 'native_completed', 'uncertain')
        ORDER BY i.id LIMIT ?2")).map_err(database_error)?;
    let rows = statement
        .query_map(
            params![sqlite_integer(after_id, "recovery cursor")?, limit as i64],
            decode,
        )
        .map_err(database_error)?;
    rows.map(|row| record(row.map_err(database_error)?))
        .collect()
}
