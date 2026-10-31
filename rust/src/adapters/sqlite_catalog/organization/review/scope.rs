use rusqlite::{Connection, OptionalExtension, params};

use crate::domain::organization_queue::OrganizationQueue;
use crate::domain::{
    GalleryCollection, GalleryDuplicateMode, GalleryQuery, GallerySortDirection, GallerySortKey,
    ScanError,
};

use super::super::super::{database_error, gallery};

pub(super) fn validate(connection: &Connection, query: &GalleryQuery) -> Result<(), ScanError> {
    gallery::validate_gallery_query(query)?;
    gallery::organization::validate_collection(connection, query)?;
    if query.organization_queue.is_none()
        || query.root_id.is_none() && query.folder_relative_path.is_some()
    {
        return Err(invalid());
    }
    if let Some(root_id) = &query.root_id {
        let exists: bool = connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM library_roots WHERE id = ?1)",
                [root_id],
                |row| row.get(0),
            )
            .map_err(database_error)?;
        if !exists {
            return Err(invalid());
        }
    }
    Ok(())
}

pub(super) fn save_active(connection: &Connection, query: &GalleryQuery) -> Result<(), ScanError> {
    let (collection, album) = match &query.collection {
        GalleryCollection::Library => ("library", None),
        GalleryCollection::Favorites => ("favorites", None),
        GalleryCollection::Album { album_id } => ("album", Some(album_id.as_str())),
        GalleryCollection::SmartAlbum { .. } => return Err(invalid()),
    };
    let queue = query
        .organization_queue
        .ok_or_else(invalid)?
        .disposition()
        .stored();
    connection
        .execute(
            "INSERT INTO organization_review_scopes VALUES ('active', ?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(slot) DO UPDATE SET
               collection = excluded.collection, album_id = excluded.album_id,
               root_id = excluded.root_id, folder_relative_path = excluded.folder_relative_path,
               include_descendants = excluded.include_descendants,
               search_text = excluded.search_text, queue = excluded.queue",
            params![
                collection,
                album,
                query.root_id,
                query.folder_relative_path,
                query.include_descendants,
                query.search_text,
                queue
            ],
        )
        .map_err(database_error)?;
    super::ordering::save_scope(connection, query)?;
    super::filters::save_scope(connection, &query.metadata_filter)
}

pub(super) fn copy_to_bookmark(connection: &Connection) -> Result<(), ScanError> {
    connection
        .execute_batch(
            "INSERT INTO organization_review_scopes
             SELECT 'bookmark', collection, album_id, root_id, folder_relative_path,
               include_descendants, search_text, queue
             FROM organization_review_scopes WHERE slot = 'active'
             ON CONFLICT(slot) DO UPDATE SET
               collection = excluded.collection, album_id = excluded.album_id,
               root_id = excluded.root_id, folder_relative_path = excluded.folder_relative_path,
               include_descendants = excluded.include_descendants,
               search_text = excluded.search_text, queue = excluded.queue;",
        )
        .map_err(database_error)?;
    super::ordering::copy_to_bookmark(connection)?;
    super::filters::copy_to_bookmark(connection)
}

pub(in super::super::super) fn load(
    connection: &Connection,
    slot: &str,
) -> Result<Option<GalleryQuery>, ScanError> {
    let mut query = connection
        .query_row(
            "SELECT collection, album_id, root_id, folder_relative_path,
               include_descendants, search_text, queue
             FROM organization_review_scopes WHERE slot = ?1",
            [slot],
            |row| {
                let collection = match row.get::<_, String>(0)?.as_str() {
                    "library" => GalleryCollection::Library,
                    "favorites" => GalleryCollection::Favorites,
                    "album" => GalleryCollection::Album {
                        album_id: row.get(1)?,
                    },
                    _ => return Err(rusqlite::Error::InvalidQuery),
                };
                let queue = match row.get::<_, String>(6)?.as_str() {
                    "pending" => OrganizationQueue::Pending,
                    "deferred" => OrganizationQueue::Deferred,
                    "ignored" => OrganizationQueue::Ignored,
                    _ => return Err(rusqlite::Error::InvalidQuery),
                };
                Ok(GalleryQuery {
                    collection,
                    organization_queue: Some(queue),
                    new_since_admission: None,
                    root_id: row.get(2)?,
                    folder_relative_path: row.get(3)?,
                    include_descendants: row.get(4)?,
                    search_text: row.get(5)?,
                    metadata_filter: Default::default(),
                    sort_key: GallerySortKey::ModifiedTime,
                    sort_direction: GallerySortDirection::Descending,
                    duplicate_mode: GalleryDuplicateMode::Merged,
                })
            },
        )
        .optional()
        .map_err(database_error)?;
    if let Some(query) = &mut query {
        super::ordering::load_scope(connection, slot, query)?;
        query.metadata_filter = super::filters::load_scope(connection, slot)?;
    }
    Ok(query)
}

fn invalid() -> ScanError {
    ScanError::new(
        "organization_review_scope_unavailable",
        "The selected organization scope is no longer available",
    )
}

pub(in super::super) fn forget_source(
    connection: &Connection,
    root_id: &str,
) -> Result<(), ScanError> {
    connection
        .execute(
            "DELETE FROM organization_review_scopes WHERE root_id = ?1",
            [root_id],
        )
        .map_err(database_error)?;
    Ok(())
}

pub(in super::super) fn transfer_source(
    connection: &Connection,
    previous: &str,
    destination: &str,
) -> Result<(), ScanError> {
    let has_scope: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM organization_review_scopes WHERE root_id = ?1)",
            [previous],
            |row| row.get(0),
        )
        .map_err(database_error)?;
    if !has_scope {
        return Ok(());
    }
    let paths: (String, String) = connection
        .query_row(
            "SELECT old.path, new.path FROM library_roots AS old, library_roots AS new
             WHERE old.id = ?1 AND new.id = ?2",
            params![previous, destination],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(database_error)?;
    let relative = std::path::Path::new(&paths.0)
        .strip_prefix(&paths.1)
        .map_err(|_| invalid())?
        .to_string_lossy()
        .replace('\\', "/");
    connection
        .execute(
            "UPDATE organization_review_scopes SET root_id = ?2,
               folder_relative_path = CASE
                 WHEN folder_relative_path IS NULL OR folder_relative_path = '' THEN ?3
                 WHEN ?3 = '' THEN folder_relative_path
                 ELSE ?3 || '/' || folder_relative_path
               END
             WHERE root_id = ?1",
            params![previous, destination, relative],
        )
        .map_err(database_error)?;
    Ok(())
}
