use rusqlite::{Connection, OptionalExtension, params};

use crate::domain::ScanError;

use super::super::super::{database_error, unix_time_ms};
use super::super::albums;

struct AlbumChange {
    album_id: String,
    previous_name: Option<String>,
    previous_retired: bool,
    applied_revision: i64,
}

pub(super) fn restore(
    connection: &Connection,
    operation_id: &str,
    original_id: &str,
    revision: i64,
) -> Result<bool, ScanError> {
    let change = connection.query_row(
        "SELECT album_id, previous_name, previous_retired, applied_revision FROM organization_album_operation_changes WHERE operation_id = ?1",
        [original_id], |row| Ok(AlbumChange { album_id: row.get(0)?, previous_name: row.get(1)?, previous_retired: row.get(2)?, applied_revision: row.get(3)? })
    ).optional().map_err(database_error)?;
    let Some(change) = change else {
        return Ok(false);
    };
    let (name, current_revision, retired): (String, i64, bool) = connection.query_row(
        "SELECT name, revision, EXISTS(SELECT 1 FROM organization_album_retirements WHERE album_id = album.album_id)
         FROM organization_albums AS album WHERE album_id = ?1", [&change.album_id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?))
    ).map_err(database_error)?;
    if current_revision != change.applied_revision {
        return Err(super::unavailable());
    }
    let target_retired = change.previous_name.is_none() || change.previous_retired;
    if change.previous_name.is_none() {
        let used: bool = connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM organization_records WHERE album_id = ?1)
             OR EXISTS(SELECT 1 FROM organization_conflicts WHERE album_id = ?1)",
                [&change.album_id],
                |row| row.get(0),
            )
            .map_err(database_error)?;
        if used {
            return Err(super::unavailable());
        }
    }
    let target_name = change.previous_name.as_deref().unwrap_or(&name);
    if !target_retired {
        albums::require_available_name(connection, target_name, &change.album_id)?;
    }
    connection
        .execute(
            "INSERT INTO organization_album_operation_changes VALUES (?1, ?2, ?3, ?4, ?5)",
            params![operation_id, change.album_id, name, retired, revision],
        )
        .map_err(database_error)?;
    if target_retired {
        connection.execute("INSERT INTO organization_album_retirements VALUES (?1, ?2, ?3) ON CONFLICT(album_id) DO UPDATE SET revision = excluded.revision, retired_unix_ms = excluded.retired_unix_ms", params![change.album_id, revision, unix_time_ms()]).map_err(database_error)?;
    } else {
        connection
            .execute(
                "DELETE FROM organization_album_retirements WHERE album_id = ?1",
                [&change.album_id],
            )
            .map_err(database_error)?;
    }
    connection.execute("UPDATE organization_albums SET name = ?1, revision = ?2, updated_unix_ms = ?3 WHERE album_id = ?4", params![target_name, revision, unix_time_ms(), change.album_id]).map_err(database_error)?;
    Ok(true)
}
