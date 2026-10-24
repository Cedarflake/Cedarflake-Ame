use rusqlite::{Connection, OptionalExtension, params};

use crate::domain::ScanError;
use crate::domain::exact_duplicates::EXACT_FINGERPRINT_SCHEME;
use crate::domain::organization::OrganizationWriteReceipt;

use super::super::exact_duplicates::queries::{COMPATIBLE_FINGERPRINT, CURRENT_LOCATIONS};
use super::super::exact_duplicates::read_unsigned;
use super::super::{database_error, unix_time_ms};
use super::operations::{self, Admission};

mod album;

pub(super) fn apply(
    connection: &Connection,
    operation_id: &str,
    original_id: &str,
    expected_revision: u64,
) -> Result<OrganizationWriteReceipt, ScanError> {
    let digest = operations::request_digest("undo", expected_revision, &[original_id]);
    let revision =
        match operations::admit(connection, operation_id, &digest, "undo", expected_revision)? {
            Admission::Replayed(receipt) => return Ok(receipt),
            Admission::New { revision } => revision,
        };
    let original: Option<(String, u64)> = connection.query_row(
        "SELECT operation_kind, changed_groups FROM organization_operations WHERE operation_id = ?1 AND operation_id <> ?2",
        params![original_id, operation_id], |row| Ok((row.get(0)?, read_unsigned(row, 1)?))
    ).optional().map_err(database_error)?;
    let Some((kind, original_groups)) = original else {
        return Err(unavailable());
    };
    if kind == "undo" {
        return Err(unavailable());
    }
    validate_fields(connection, original_id)?;
    let changed_groups = connection.query_row(
        "SELECT COUNT(DISTINCT member.group_id) FROM organization_operation_changes AS change
         JOIN organization_record_groups AS member ON member.record_id = change.record_id WHERE change.operation_id = ?1",
        [original_id], |row| read_unsigned(row, 0)
    ).map_err(database_error)?;
    if original_groups > 0 && changed_groups == 0 {
        return Err(unavailable());
    }
    let metadata_changed = album::restore(connection, operation_id, original_id, revision)?;
    validate_album_targets(connection, original_id)?;
    restore_fields(connection, operation_id, original_id, revision)?;
    operations::finish(
        connection,
        operation_id,
        revision,
        changed_groups,
        metadata_changed,
    )
}

fn validate_album_targets(connection: &Connection, original_id: &str) -> Result<(), ScanError> {
    let retired: bool = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM organization_operation_changes AS change
         JOIN organization_album_retirements AS retirement ON retirement.album_id = change.previous_album_id
         WHERE change.operation_id = ?1 AND change.field = 'organization')",
        [original_id], |row| row.get(0)
    ).map_err(database_error)?;
    if retired {
        return Err(unavailable());
    }
    Ok(())
}

fn validate_fields(connection: &Connection, original_id: &str) -> Result<(), ScanError> {
    let invalid: bool = connection.query_row(&format!(
        "SELECT EXISTS(SELECT 1 FROM organization_operation_targets AS target
         LEFT JOIN organization_operation_changes AS change ON change.operation_id = target.operation_id
           AND change.record_id = target.record_id AND change.field = target.field
         LEFT JOIN organization_records AS record ON record.record_id = target.record_id
         WHERE target.operation_id = ?2 AND (record.record_id IS NULL OR change.record_id IS NULL
           OR (change.field = 'favorite' AND record.favorite_revision <> change.applied_revision)
           OR (change.field = 'organization' AND record.organization_revision <> change.applied_revision)
           OR NOT EXISTS(SELECT 1 FROM {CURRENT_LOCATIONS}
             JOIN organization_bindings AS binding ON binding.asset_id = l.asset_id AND binding.source_generation = l.source_generation
             JOIN organization_record_groups AS evidence ON evidence.record_id = binding.record_id
             JOIN organization_record_groups AS affected ON affected.group_id = evidence.group_id AND affected.record_id = target.record_id
             LEFT JOIN content_fingerprints AS f ON {COMPATIBLE_FINGERPRINT}
             WHERE binding.byte_length = l.file_size
               AND binding.file_identity_scheme IS l.file_identity_scheme AND binding.file_identity_value IS l.file_identity_value
               AND binding.source_revision_token IS l.source_revision_token
               AND (binding.digest IS NULL OR (binding.fingerprint_scheme = f.scheme AND binding.digest = f.digest)))))"
    ), params![EXACT_FINGERPRINT_SCHEME, original_id], |row| row.get(0)).map_err(database_error)?;
    if invalid {
        return Err(unavailable());
    }
    Ok(())
}

fn restore_fields(
    connection: &Connection,
    operation_id: &str,
    original_id: &str,
    revision: i64,
) -> Result<(), ScanError> {
    connection.execute(
        "INSERT INTO organization_operation_changes(operation_id, record_id, field, previous_album_id, previous_disposition, previous_favorite, applied_revision)
         SELECT ?1, record.record_id, change.field,
           CASE WHEN change.field = 'organization' THEN record.album_id END,
           CASE WHEN change.field = 'organization' THEN record.disposition END,
           CASE WHEN change.field = 'favorite' THEN record.favorite END, ?3
         FROM organization_operation_changes AS change JOIN organization_records AS record ON record.record_id = change.record_id
         WHERE change.operation_id = ?2", params![operation_id, original_id, revision]
    ).map_err(database_error)?;
    connection.execute(
        "INSERT INTO organization_operation_conflicts
         SELECT ?1, conflict.record_id, conflict.album_id, conflict.decision_revision FROM organization_conflicts AS conflict
         JOIN organization_operation_changes AS change ON change.record_id = conflict.record_id
         WHERE change.operation_id = ?2 AND change.field = 'organization'", params![operation_id, original_id]
    ).map_err(database_error)?;
    connection.execute(
        "DELETE FROM organization_conflicts WHERE record_id IN (SELECT record_id FROM organization_operation_changes WHERE operation_id = ?1 AND field = 'organization')", [original_id]
    ).map_err(database_error)?;
    connection.execute("INSERT INTO organization_conflicts SELECT record_id, album_id, decision_revision FROM organization_operation_conflicts WHERE operation_id = ?1", [original_id]).map_err(database_error)?;
    connection.execute(
        "UPDATE organization_records AS record SET
           album_id = (SELECT previous_album_id FROM organization_operation_changes WHERE operation_id = ?1 AND record_id = record.record_id AND field = 'organization'),
           disposition = (SELECT previous_disposition FROM organization_operation_changes WHERE operation_id = ?1 AND record_id = record.record_id AND field = 'organization'),
           organization_revision = ?2, updated_unix_ms = ?3
         WHERE record_id IN (SELECT record_id FROM organization_operation_changes WHERE operation_id = ?1 AND field = 'organization')",
        params![original_id, revision, unix_time_ms()]
    ).map_err(database_error)?;
    connection.execute(
        "UPDATE organization_records AS record SET
           favorite = (SELECT previous_favorite FROM organization_operation_changes WHERE operation_id = ?1 AND record_id = record.record_id AND field = 'favorite'),
           favorite_revision = ?2, updated_unix_ms = ?3
         WHERE record_id IN (SELECT record_id FROM organization_operation_changes WHERE operation_id = ?1 AND field = 'favorite')",
        params![original_id, revision, unix_time_ms()]
    ).map_err(database_error)?;
    Ok(())
}

pub(super) fn unavailable() -> ScanError {
    ScanError::new(
        "organization_undo_unavailable",
        "The affected decision or content changed; refresh before choosing another action",
    )
}
