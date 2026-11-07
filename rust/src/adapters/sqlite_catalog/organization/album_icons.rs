use rusqlite::{Connection, Row, params};

use crate::domain::ScanError;
use crate::domain::organization::{OrganizationAlbum, OrganizationAlbumIcon};

use super::super::database_error;

pub(super) fn read(row: &Row<'_>, index: usize) -> rusqlite::Result<OrganizationAlbumIcon> {
    OrganizationAlbumIcon::from_stored(&row.get::<_, String>(index)?)
        .ok_or(rusqlite::Error::InvalidQuery)
}

pub(super) fn set(
    connection: &Connection,
    album_id: &str,
    icon: OrganizationAlbumIcon,
) -> Result<(), ScanError> {
    connection
        .execute(
            "INSERT INTO organization_album_icons VALUES (?1, ?2)
             ON CONFLICT(album_id) DO UPDATE SET icon = excluded.icon",
            params![album_id, icon.stored()],
        )
        .map_err(database_error)?;
    Ok(())
}

pub(super) fn change(
    connection: &Connection,
    album_id: &str,
    icon: OrganizationAlbumIcon,
) -> Result<OrganizationAlbum, ScanError> {
    let mut album = super::albums::find(connection, album_id)?.ok_or_else(|| {
        ScanError::new(
            "organization_album_unavailable",
            "The selected album is no longer available",
        )
    })?;
    if album.icon != icon {
        set(connection, album_id, icon)?;
        connection
            .execute("UPDATE catalog_state SET revision = revision + 1", [])
            .map_err(database_error)?;
        album.icon = icon;
    }
    Ok(album)
}

#[cfg(test)]
mod tests;
