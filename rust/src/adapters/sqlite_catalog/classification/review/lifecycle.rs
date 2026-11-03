use rusqlite::{Connection, OptionalExtension, params};

use crate::domain::ScanError;

use super::super::super::{
    database_error,
    gallery::review::{ReviewRead, candidate},
};
use super::{position, scope, stale};

pub(in crate::adapters::sqlite_catalog) fn forget_source(
    connection: &Connection,
    root: &str,
) -> Result<(), ScanError> {
    for table in [
        "classification_review_session",
        "classification_review_bookmark",
    ] {
        connection
            .execute(
                &format!("DELETE FROM {table} WHERE json_extract(scope, '$.root_id') = ?1"),
                [root],
            )
            .map_err(database_error)?;
    }
    let Some(bookmark) = position::load(connection)? else {
        return Ok(());
    };
    if bookmark.position.root_id != root {
        return Ok(());
    }
    if let Some(current) = candidate(
        connection,
        &bookmark.scope.gallery,
        ReviewRead::Current(&bookmark.record_id),
    )? {
        let boundary = current.boundary;
        connection.execute(
            "UPDATE classification_review_bookmark SET admission_id = ?1, root_id = ?2, location_id = ?3,
             primary_missing = ?4, primary_text = ?5, primary_number = ?6 WHERE singleton = 1",
            params![boundary.admission_id, boundary.root_id, boundary.location_id, boundary.primary_missing, boundary.primary_text, boundary.primary_number],
        ).map_err(database_error)?;
    } else {
        connection
            .execute("DELETE FROM classification_review_bookmark", [])
            .map_err(database_error)?;
    }
    Ok(())
}

pub(in crate::adapters::sqlite_catalog) fn associate_roots(
    connection: &Connection,
    current: &str,
) -> Result<(), ScanError> {
    for table in [
        "classification_review_session",
        "classification_review_bookmark",
    ] {
        connection.execute(&format!(
            "UPDATE {table} SET scope = json_set(scope, '$.root_id', ?1)
             WHERE json_extract(scope, '$.root_id') IN (SELECT root_id FROM temp.organization_recovery_root_matches)"
        ), [current]).map_err(database_error)?;
    }
    connection.execute(
        "UPDATE classification_review_bookmark SET root_id = ?1 WHERE root_id IN (SELECT root_id FROM temp.organization_recovery_root_matches)",
        [current],
    ).map_err(database_error)?;
    Ok(())
}

pub(in crate::adapters::sqlite_catalog) fn transfer_source(
    connection: &Connection,
    previous: &str,
    destination: &str,
) -> Result<(), ScanError> {
    connection
        .execute(
            "UPDATE classification_review_bookmark SET root_id = ?2 WHERE root_id = ?1",
            params![previous, destination],
        )
        .map_err(database_error)?;
    let has_scope: bool = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM classification_review_session WHERE json_extract(scope, '$.root_id') = ?1)
         OR EXISTS(SELECT 1 FROM classification_review_bookmark WHERE json_extract(scope, '$.root_id') = ?1)",
        [previous], |row| row.get(0),
    ).map_err(database_error)?;
    if !has_scope {
        return Ok(());
    }
    let paths: (String, String) = connection.query_row(
        "SELECT old.path, new.path FROM library_roots AS old, library_roots AS new WHERE old.id = ?1 AND new.id = ?2",
        params![previous, destination], |row| Ok((row.get(0)?, row.get(1)?)),
    ).map_err(database_error)?;
    let prefix = std::path::Path::new(&paths.0)
        .strip_prefix(&paths.1)
        .map_err(|_| stale())?
        .to_string_lossy()
        .replace('\\', "/");
    for table in [
        "classification_review_session",
        "classification_review_bookmark",
    ] {
        let payload: Option<String> = connection
            .query_row(
                &format!("SELECT scope FROM {table} WHERE json_extract(scope, '$.root_id') = ?1"),
                [previous],
                |row| row.get(0),
            )
            .optional()
            .map_err(database_error)?;
        let Some(payload) = payload else {
            continue;
        };
        let mut saved = scope::decode(&payload)?;
        saved.gallery.root_id = Some(destination.into());
        let folder = saved.gallery.folder_relative_path.as_deref().unwrap_or("");
        let joined = [prefix.as_str(), folder]
            .into_iter()
            .filter(|value| !value.is_empty())
            .collect::<Vec<_>>()
            .join("/");
        saved.gallery.folder_relative_path = if joined.is_empty() {
            None
        } else {
            Some(joined)
        };
        connection
            .execute(
                &format!("UPDATE {table} SET scope = ?1 WHERE singleton = 1"),
                [scope::encode(&saved)?],
            )
            .map_err(database_error)?;
    }
    Ok(())
}
