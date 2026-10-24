use rusqlite::{Connection, params};

use crate::domain::ScanError;

use super::super::super::database_error;
use super::super::undo::unavailable;

pub(crate) fn require_current_fields(
    connection: &Connection,
    original_id: &str,
) -> Result<(), ScanError> {
    let query = if super::available(connection)? {
        "SELECT EXISTS(SELECT 1 FROM organization_operation_changes AS change
         JOIN organization_records AS record ON record.record_id = change.record_id
         LEFT JOIN organization_field_authority AS applied ON applied.operation_id = change.operation_id
           AND applied.record_id = change.record_id AND applied.field = change.field
         LEFT JOIN organization_field_authority AS current ON current.record_id = change.record_id
           AND current.field = change.field AND current.applied_revision =
             CASE change.field WHEN 'favorite' THEN record.favorite_revision ELSE record.organization_revision END
         WHERE change.operation_id = ?1
           AND COALESCE(current.applied_authority,
             CASE change.field WHEN 'favorite' THEN record.favorite_revision ELSE record.organization_revision END)
             <> COALESCE(applied.applied_authority, change.applied_revision))"
    } else {
        "SELECT EXISTS(SELECT 1 FROM organization_operation_changes AS change
         JOIN organization_records AS record ON record.record_id = change.record_id
         WHERE change.operation_id = ?1 AND change.applied_revision <>
           CASE change.field WHEN 'favorite' THEN record.favorite_revision ELSE record.organization_revision END)"
    };
    let invalid: bool = connection
        .query_row(query, [original_id], |row| row.get(0))
        .map_err(database_error)?;
    if invalid {
        return Err(unavailable());
    }
    Ok(())
}

pub(crate) fn require_current_album(
    connection: &Connection,
    album_id: &str,
    current: i64,
    applied: i64,
) -> Result<(), ScanError> {
    let matches = if super::available(connection)? {
        connection
            .query_row(
                "SELECT COALESCE((SELECT applied_authority FROM organization_album_authority
               WHERE album_id = ?1 AND applied_revision = ?2), ?2)
             = COALESCE((SELECT applied_authority FROM organization_album_authority
               WHERE album_id = ?1 AND applied_revision = ?3), ?3)",
                params![album_id, current, applied],
                |row| row.get(0),
            )
            .map_err(database_error)?
    } else {
        current == applied
    };
    if !matches {
        return Err(unavailable());
    }
    Ok(())
}

pub(crate) fn restore_fields(
    connection: &Connection,
    operation_id: &str,
    original_id: &str,
) -> Result<(), ScanError> {
    if !super::available(connection)? {
        return Ok(());
    }
    connection.execute(
        "UPDATE organization_field_authority AS applied SET applied_authority = (
           SELECT previous_authority FROM organization_field_authority AS original
           WHERE original.operation_id = ?2 AND original.record_id = applied.record_id AND original.field = applied.field)
         WHERE applied.operation_id = ?1 AND EXISTS(
           SELECT 1 FROM organization_field_authority AS original
           WHERE original.operation_id = ?2 AND original.record_id = applied.record_id AND original.field = applied.field)",
        params![operation_id, original_id],
    ).map_err(database_error)?;
    Ok(())
}

pub(crate) fn restore_album(
    connection: &Connection,
    operation_id: &str,
    original_id: &str,
) -> Result<(), ScanError> {
    if !super::available(connection)? {
        return Ok(());
    }
    connection
        .execute(
            "UPDATE organization_album_authority SET applied_authority = (
           SELECT previous_authority FROM organization_album_authority WHERE operation_id = ?2)
         WHERE operation_id = ?1 AND EXISTS(
           SELECT 1 FROM organization_album_authority WHERE operation_id = ?2)",
            params![operation_id, original_id],
        )
        .map_err(database_error)?;
    Ok(())
}

pub(crate) fn restores_album_creation(
    connection: &Connection,
    original_id: &str,
) -> Result<bool, ScanError> {
    if !super::available(connection)? {
        return Ok(false);
    }
    connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM organization_album_authority
         WHERE operation_id = ?1 AND previous_authority = 0)",
            [original_id],
            |row| row.get(0),
        )
        .map_err(database_error)
}
