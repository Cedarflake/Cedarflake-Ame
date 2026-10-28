use rusqlite::{Connection, OptionalExtension, params};

use crate::domain::ScanError;
use crate::domain::organization::{
    OrganizationAlbum, OrganizationAlbumPage, OrganizationAlbumReceipt, OrganizationWriteReceipt,
};

use super::super::exact_duplicates::read_unsigned;
use super::super::{database_error, unix_time_ms};
use super::operations::{self, Admission};

const PAGE_SIZE: usize = 64;

pub(super) fn find(
    connection: &Connection,
    album_id: &str,
) -> Result<Option<OrganizationAlbum>, ScanError> {
    connection
        .query_row(
            "SELECT album.album_id, album.name, album.revision
         FROM organization_albums AS album
         WHERE album.album_id = ?1 AND NOT EXISTS (
             SELECT 1 FROM organization_album_retirements WHERE album_id = album.album_id
         )",
            [album_id],
            |row| {
                Ok(OrganizationAlbum {
                    album_id: row.get(0)?,
                    name: row.get(1)?,
                    revision: read_unsigned(row, 2)?,
                })
            },
        )
        .optional()
        .map_err(database_error)
}

pub(super) fn load(
    connection: &Connection,
    after: Option<&str>,
) -> Result<OrganizationAlbumPage, ScanError> {
    let organization_revision = connection
        .query_row(
            "SELECT revision FROM organization_state WHERE singleton = 1",
            [],
            |row| read_unsigned(row, 0),
        )
        .map_err(database_error)?;
    let total_albums = connection
        .query_row(
            "SELECT COUNT(*) FROM organization_albums AS album WHERE NOT EXISTS(
           SELECT 1 FROM organization_album_retirements WHERE album_id = album.album_id)",
            [],
            |row| read_unsigned(row, 0),
        )
        .map_err(database_error)?;
    let mut statement = connection.prepare(
        "SELECT album.album_id, album.name, album.revision
         FROM organization_sidebar_album_order AS ordering
         JOIN organization_albums AS album USING(album_id)
         WHERE ordering.position > COALESCE(
           (SELECT position FROM organization_sidebar_album_order WHERE album_id = ?1), -1)
         AND NOT EXISTS(SELECT 1 FROM organization_album_retirements WHERE album_id = album.album_id)
         ORDER BY ordering.position, album.album_id LIMIT ?2"
    ).map_err(database_error)?;
    let mut albums = statement
        .query_map(
            params![after.unwrap_or(""), (PAGE_SIZE + 1) as i64],
            |row| {
                Ok(OrganizationAlbum {
                    album_id: row.get(0)?,
                    name: row.get(1)?,
                    revision: read_unsigned(row, 2)?,
                })
            },
        )
        .map_err(database_error)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(database_error)?;
    let next_album_id = if albums.len() > PAGE_SIZE {
        albums.truncate(PAGE_SIZE);
        albums.last().map(|album| album.album_id.clone())
    } else {
        None
    };
    Ok(OrganizationAlbumPage {
        albums,
        total_albums,
        organization_revision,
        next_album_id,
    })
}

pub(super) fn require_active(connection: &Connection, album_id: &str) -> Result<String, ScanError> {
    connection.query_row(
        "SELECT name FROM organization_albums AS album WHERE album_id = ?1
         AND NOT EXISTS(SELECT 1 FROM organization_album_retirements WHERE album_id = album.album_id)",
        [album_id], |row| row.get(0)
    ).optional().map_err(database_error)?.ok_or_else(|| ScanError::new("organization_album_unavailable", "The selected album is no longer available"))
}

pub(super) fn require_available_name(
    connection: &Connection,
    name: &str,
    except: &str,
) -> Result<(), ScanError> {
    let exists: bool = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM organization_albums AS album WHERE name = ?1 COLLATE NOCASE
           AND album_id <> ?2 AND NOT EXISTS(SELECT 1 FROM organization_album_retirements WHERE album_id = album.album_id))",
        params![name, except], |row| row.get(0)
    ).map_err(database_error)?;
    if exists {
        return Err(ScanError::new(
            "organization_album_name_exists",
            "An album already uses this name",
        ));
    }
    Ok(())
}

pub(super) fn create(
    connection: &Connection,
    operation_id: &str,
    expected_revision: u64,
    name: &str,
) -> Result<OrganizationAlbumReceipt, ScanError> {
    let digest = operations::request_digest("create-album", expected_revision, &[name]);
    let revision = match operations::admit(
        connection,
        operation_id,
        &digest,
        "album",
        expected_revision,
    )? {
        Admission::Replayed(operation) => {
            let album_id = connection.query_row("SELECT album_id FROM organization_album_operation_changes WHERE operation_id = ?1", [operation_id], |row| row.get(0)).map_err(database_error)?;
            return Ok(OrganizationAlbumReceipt {
                album_id,
                operation,
            });
        }
        Admission::New { revision } => revision,
    };
    require_available_name(connection, name, "")?;
    let album_id: String = connection
        .query_row("SELECT lower(hex(randomblob(16)))", [], |row| row.get(0))
        .map_err(database_error)?;
    connection
        .execute(
            "INSERT INTO organization_albums VALUES (?1, ?2, ?3, ?4, ?4)",
            params![album_id, name, revision, unix_time_ms()],
        )
        .map_err(database_error)?;
    connection
        .execute(
            "INSERT INTO organization_album_operation_changes VALUES (?1, ?2, NULL, 0, ?3)",
            params![operation_id, album_id, revision],
        )
        .map_err(database_error)?;
    let operation = operations::finish(connection, operation_id, revision, 0, true)?;
    Ok(OrganizationAlbumReceipt {
        album_id,
        operation,
    })
}

pub(super) fn rename(
    connection: &Connection,
    operation_id: &str,
    expected_revision: u64,
    album_id: &str,
    name: &str,
) -> Result<OrganizationAlbumReceipt, ScanError> {
    let digest = operations::request_digest("rename-album", expected_revision, &[album_id, name]);
    let revision = match operations::admit(
        connection,
        operation_id,
        &digest,
        "album",
        expected_revision,
    )? {
        Admission::Replayed(operation) => {
            return Ok(OrganizationAlbumReceipt {
                album_id: album_id.to_owned(),
                operation,
            });
        }
        Admission::New { revision } => revision,
    };
    let previous = require_active(connection, album_id)?;
    let changed = previous != name;
    if changed {
        require_available_name(connection, name, album_id)?;
        connection
            .execute(
                "INSERT INTO organization_album_operation_changes VALUES (?1, ?2, ?3, 0, ?4)",
                params![operation_id, album_id, previous, revision],
            )
            .map_err(database_error)?;
        connection.execute("UPDATE organization_albums SET name = ?1, revision = ?2, updated_unix_ms = ?3 WHERE album_id = ?4", params![name, revision, unix_time_ms(), album_id]).map_err(database_error)?;
    }
    let operation = operations::finish(connection, operation_id, revision, 0, changed)?;
    Ok(OrganizationAlbumReceipt {
        album_id: album_id.to_owned(),
        operation,
    })
}

pub(super) fn retire(
    connection: &Connection,
    operation_id: &str,
    expected_revision: u64,
    album_id: &str,
) -> Result<OrganizationWriteReceipt, ScanError> {
    let digest = operations::request_digest("delete-album", expected_revision, &[album_id]);
    let revision = match operations::admit(
        connection,
        operation_id,
        &digest,
        "album",
        expected_revision,
    )? {
        Admission::Replayed(receipt) => return Ok(receipt),
        Admission::New { revision } => revision,
    };
    let previous = require_active(connection, album_id)?;
    let changed_groups =
        super::album_membership::retire(connection, operation_id, album_id, revision)?;
    connection
        .execute(
            "INSERT INTO organization_album_operation_changes VALUES (?1, ?2, ?3, 0, ?4)",
            params![operation_id, album_id, previous, revision],
        )
        .map_err(database_error)?;
    connection
        .execute(
            "INSERT INTO organization_album_retirements VALUES (?1, ?2, ?3)",
            params![album_id, revision, unix_time_ms()],
        )
        .map_err(database_error)?;
    connection.execute("UPDATE organization_albums SET revision = ?1, updated_unix_ms = ?2 WHERE album_id = ?3", params![revision, unix_time_ms(), album_id]).map_err(database_error)?;
    operations::finish(connection, operation_id, revision, changed_groups, true)
}
