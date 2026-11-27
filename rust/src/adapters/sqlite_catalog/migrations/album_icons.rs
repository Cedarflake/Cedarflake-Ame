use rusqlite::Connection;

use crate::domain::ScanError;
use crate::domain::organization::OrganizationAlbumIcon;

use super::{database_error, schema_shape};

const ICONS: &str = "CREATE TABLE organization_album_icons (
    album_id TEXT PRIMARY KEY NOT NULL REFERENCES organization_albums(album_id) ON DELETE CASCADE,
    icon TEXT NOT NULL CHECK(icon IN ('album', 'travel', 'nature', 'pets', 'people', 'food',
        'art', 'work', 'study', 'games', 'music', 'memories'))
)";

pub(super) fn validate(connection: &Connection) -> Result<(), ScanError> {
    if !schema_shape::schema_object_sql_matches(
        connection,
        "table",
        "organization_album_icons",
        ICONS,
    )? {
        return Err(invalid());
    }
    Ok(())
}

pub(super) fn validate_rows(connection: &Connection) -> Result<(), ScanError> {
    let orphaned: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM pragma_foreign_key_check('organization_album_icons'))",
            [],
            |row| row.get(0),
        )
        .map_err(database_error)?;
    if orphaned {
        return Err(invalid());
    }
    let mut statement = connection
        .prepare("SELECT DISTINCT icon FROM organization_album_icons")
        .map_err(database_error)?;
    let rows = statement
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(database_error)?;
    for row in rows {
        if OrganizationAlbumIcon::from_stored(&row.map_err(database_error)?).is_none() {
            return Err(invalid());
        }
    }
    Ok(())
}

fn invalid() -> ScanError {
    ScanError::new(
        "catalog_album_icon_invalid",
        "Album icons require valid album ownership",
    )
}
