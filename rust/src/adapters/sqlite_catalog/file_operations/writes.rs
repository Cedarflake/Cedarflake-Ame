use std::collections::HashSet;

use rusqlite::{Connection, OptionalExtension, params};

use crate::domain::ScanError;
use crate::domain::file_operation_journal::{
    AuthorizedFileOperation, FileOperationCheckpoint, FileOperationEvent, FileOperationRecord,
    invalid,
};

use super::super::{database_error, source_revision_token, sqlite_integer, sqlite_unsigned};
use super::{codec, reads};

pub(super) fn authorize(
    connection: &Connection,
    items: &[AuthorizedFileOperation],
) -> Result<Vec<FileOperationRecord>, ScanError> {
    let mut keys = HashSet::new();
    let mut records = Vec::with_capacity(items.len());
    for item in items {
        if !keys.insert((&item.batch_id, item.ordinal)) {
            return Err(invalid("The authorization batch repeats an item ordinal"));
        }
        let authorization = codec::encode_authorization(item)?;
        let existing: Option<(i64, String)> = connection.query_row(
            "SELECT id, authorization FROM file_operation_items WHERE batch_id = ?1 AND ordinal = ?2",
            params![item.batch_id, sqlite_integer(item.ordinal, "operation ordinal")?],
            |row| Ok((row.get(0)?, row.get(1)?)),
        ).optional().map_err(database_error)?;
        if let Some((id, previous)) = existing {
            if previous != authorization {
                return Err(invalid(
                    "An existing operation cannot accept different authorization",
                ));
            }
            records.push(reads::load(
                connection,
                sqlite_unsigned(id, "operation ID")?,
            )?);
            continue;
        }
        revalidate_catalog(connection, item)?;
        connection
            .execute(
                "INSERT INTO file_operation_items(batch_id, ordinal, authorization, version, phase)
             VALUES (?1, ?2, ?3, 0, 'prepared')",
                params![
                    item.batch_id,
                    sqlite_integer(item.ordinal, "operation ordinal")?,
                    authorization
                ],
            )
            .map_err(database_error)?;
        let id = sqlite_unsigned(connection.last_insert_rowid(), "operation ID")?;
        let checkpoint = FileOperationCheckpoint::default();
        append(connection, id, &checkpoint, item.authorized_unix_ms)?;
        records.push(FileOperationRecord {
            id,
            authorization: item.clone(),
            checkpoint,
            updated_unix_ms: item.authorized_unix_ms,
        });
    }
    Ok(records)
}

pub(super) fn advance(
    connection: &Connection,
    id: u64,
    expected_version: u64,
    event: FileOperationEvent,
    now_unix_ms: i64,
) -> Result<FileOperationRecord, ScanError> {
    let mut record = reads::load(connection, id)?;
    if record.checkpoint.version != expected_version {
        return Err(invalid("A newer operation result already owns this item"));
    }
    if matches!(
        event,
        FileOperationEvent::Started(_) | FileOperationEvent::RemovingSource
    ) {
        revalidate_catalog(connection, &record.authorization)?;
    }
    let checkpoint = record.checkpoint.advance(&record.authorization, event)?;
    append(connection, id, &checkpoint, now_unix_ms)?;
    let updated = connection.execute(
        "UPDATE file_operation_items SET version = ?2, phase = ?3 WHERE id = ?1 AND version = ?4",
        params![sqlite_integer(id, "operation ID")?, sqlite_integer(checkpoint.version, "operation version")?, codec::phase_name(checkpoint.phase), sqlite_integer(expected_version, "expected operation version")?],
    ).map_err(database_error)?;
    if updated != 1 {
        return Err(invalid(
            "The operation journal head changed before publication",
        ));
    }
    record.checkpoint = checkpoint;
    record.updated_unix_ms = now_unix_ms;
    Ok(record)
}

fn append(
    connection: &Connection,
    id: u64,
    checkpoint: &FileOperationCheckpoint,
    now_unix_ms: i64,
) -> Result<(), ScanError> {
    connection.execute(
        "INSERT INTO file_operation_events(item_id, version, snapshot, recorded_unix_ms) VALUES (?1, ?2, ?3, ?4)",
        params![sqlite_integer(id, "operation ID")?, sqlite_integer(checkpoint.version, "operation version")?, codec::encode_checkpoint(checkpoint)?, now_unix_ms],
    ).map_err(database_error)?;
    Ok(())
}

fn revalidate_catalog(
    connection: &Connection,
    item: &AuthorizedFileOperation,
) -> Result<(), ScanError> {
    if !catalog_matches(connection, item)? {
        return Err(ScanError::new(
            "file_operation_selection_changed",
            "The selected physical file or library root changed",
        ));
    }
    Ok(())
}

pub(super) fn catalog_matches(
    connection: &Connection,
    item: &AuthorizedFileOperation,
) -> Result<bool, ScanError> {
    let binding = &item.catalog;
    let source = &item.request.source.state;
    let identity = source
        .file_identity
        .as_ref()
        .ok_or_else(|| invalid("Missing source identity"))?;
    let revision = source
        .source_revision
        .as_ref()
        .ok_or_else(|| invalid("Missing source revision"))?;
    connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM asset_locations AS l
         JOIN library_roots AS root ON root.id = l.root_id AND root.active_scan_id = l.scan_id
         JOIN library_change_root_state AS state ON state.root_id = l.root_id
         WHERE l.location_id = ?1 AND l.asset_id = ?2 AND l.source_generation = ?3
           AND l.root_id = ?4 AND state.generation = ?5 AND state.is_active = 1
           AND l.absolute_path = ?6 AND l.file_size = ?7 AND l.modified_unix_ms = ?8
           AND l.file_identity_scheme = ?9 AND l.file_identity_value = ?10 AND l.source_revision_token = ?11)",
        params![binding.location_id, binding.asset_id, sqlite_integer(binding.source_generation, "source generation")?, binding.root_id, sqlite_integer(binding.root_generation, "root generation")?, source.absolute_path, sqlite_integer(source.file_size, "source size")?, source.modified_unix_ms, identity.scheme, identity.value, source_revision_token(revision)?],
        |row| row.get(0),
    ).map_err(database_error)
}
