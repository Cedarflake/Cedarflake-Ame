use rusqlite::{Connection, OptionalExtension, params};

use crate::domain::ScanError;
use crate::domain::classification::ClassificationRecordId;
use crate::domain::classification_catalog::ClassificationWriteReceipt;

use super::super::{database_error, exact_duplicates::read_unsigned, unix_time_ms};
use super::{changed, invalid_record};

pub(super) fn replay(
    connection: &Connection,
    id: &ClassificationRecordId,
    digest: &str,
) -> Result<Option<ClassificationWriteReceipt>, ScanError> {
    let saved = connection.query_row(
        "SELECT request_digest, revision, changed_groups FROM classification_operations WHERE operation_id = ?1",
        [id.as_str()], |row| Ok((row.get::<_, String>(0)?, read_unsigned(row, 1)?, read_unsigned(row, 2)?)),
    ).optional().map_err(database_error)?;
    let Some((previous_digest, revision, changed_groups)) = saved else {
        return Ok(None);
    };
    if previous_digest != digest {
        return Err(ScanError::new(
            "classification_operation_conflict",
            "The operation identifier was already used for a different request",
        ));
    }
    Ok(Some(ClassificationWriteReceipt {
        operation_id: id.as_str().into(),
        revision,
        changed_groups,
    }))
}

pub(super) fn next_revision(connection: &Connection) -> Result<i64, ScanError> {
    let revision: i64 = connection
        .query_row(
            "SELECT revision FROM classification_state WHERE singleton = 1",
            [],
            |row| row.get(0),
        )
        .map_err(database_error)?;
    revision.checked_add(1).ok_or_else(invalid_record)
}

pub(super) fn begin(
    connection: &Connection,
    id: &ClassificationRecordId,
    digest: &str,
    kind: &str,
) -> Result<i64, ScanError> {
    let revision = next_revision(connection)?;
    connection.execute(
        "INSERT INTO classification_operations(operation_id, request_digest, operation_kind, revision, changed_groups, created_unix_ms)
         VALUES (?1, ?2, ?3, ?4, 0, ?5)", params![id.as_str(), digest, kind, revision - 1, unix_time_ms()],
    ).map_err(database_error)?;
    Ok(revision)
}

pub(super) fn finish(
    connection: &Connection,
    id: &ClassificationRecordId,
    revision: i64,
    changed_groups: u64,
    metadata_changed: bool,
) -> Result<ClassificationWriteReceipt, ScanError> {
    let published_revision = if changed_groups > 0 || metadata_changed {
        connection
            .execute(
                "UPDATE classification_state SET revision = ?1 WHERE singleton = 1",
                [revision],
            )
            .map_err(database_error)?;
        publish_catalog(connection)?;
        revision
    } else {
        revision - 1
    };
    connection.execute(
        "INSERT INTO classification_operation_targets
         SELECT operation_id, record_id, field FROM classification_operation_changes WHERE operation_id = ?1", [id.as_str()],
    ).map_err(database_error)?;
    connection.execute(
        "UPDATE classification_operations SET revision = ?2, changed_groups = ?3 WHERE operation_id = ?1",
        params![id.as_str(), published_revision, super::super::sqlite_integer(changed_groups, "changed classification groups")?],
    ).map_err(database_error)?;
    Ok(ClassificationWriteReceipt {
        operation_id: id.as_str().into(),
        revision: published_revision as u64,
        changed_groups,
    })
}

pub(super) fn publish_catalog(connection: &Connection) -> Result<(), ScanError> {
    let revision: i64 = connection
        .query_row("SELECT revision FROM catalog_state", [], |row| row.get(0))
        .map_err(database_error)?;
    let next = revision.checked_add(1).ok_or_else(invalid_record)?;
    connection
        .execute("UPDATE catalog_state SET revision = ?1", [next])
        .map_err(database_error)?;
    Ok(())
}

pub(super) fn require_revision(connection: &Connection, expected: u64) -> Result<(), ScanError> {
    let current = connection
        .query_row(
            "SELECT revision FROM classification_state WHERE singleton = 1",
            [],
            |row| read_unsigned(row, 0),
        )
        .map_err(database_error)?;
    if current != expected {
        return Err(changed());
    }
    Ok(())
}

pub(super) fn field(hasher: &mut blake3::Hasher, value: &[u8]) {
    hasher.update(&(value.len() as u64).to_le_bytes());
    hasher.update(value);
}
