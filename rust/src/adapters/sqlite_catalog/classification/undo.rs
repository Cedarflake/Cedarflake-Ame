use rusqlite::{Connection, OptionalExtension, params};

use crate::domain::ScanError;
use crate::domain::classification::ClassificationRecordId;
use crate::domain::classification_catalog::ClassificationWriteReceipt;
use crate::domain::exact_duplicates::EXACT_FINGERPRINT_SCHEME;

use super::super::exact_duplicates::queries::{COMPATIBLE_FINGERPRINT, CURRENT_LOCATIONS};
use super::super::{database_error, exact_duplicates::read_unsigned};
use super::{codec, operations};

pub(super) fn apply(
    connection: &Connection,
    id: &ClassificationRecordId,
    original: &ClassificationRecordId,
) -> Result<ClassificationWriteReceipt, ScanError> {
    let mut hash = blake3::Hasher::new_derive_key("ame.classification.undo.v1");
    operations::field(&mut hash, original.as_str().as_bytes());
    let digest = hash.finalize().to_hex().to_string();
    if let Some(receipt) = operations::replay(connection, id, &digest)? {
        return Ok(receipt);
    }
    let saved: Option<(String, u64)> = connection.query_row(
        "SELECT operation_kind, changed_groups FROM classification_operations WHERE operation_id = ?1 AND operation_id <> ?2 AND restored = 0",
        params![original.as_str(), id.as_str()], |row| Ok((row.get(0)?, read_unsigned(row, 1)?)),
    ).optional().map_err(database_error)?;
    let (kind, original_groups) = saved.ok_or_else(unavailable)?;
    if !matches!(kind.as_str(), "category" | "defer" | "undo") {
        return Err(unavailable());
    }
    validate_targets(connection, original)?;
    let changed_groups = connection
        .query_row(
            "SELECT COUNT(DISTINCT member.group_id) FROM classification_operation_changes AS change
         JOIN organization_record_groups AS member USING(record_id) WHERE change.operation_id = ?1",
            [original.as_str()],
            |row| read_unsigned(row, 0),
        )
        .map_err(database_error)?;
    if original_groups > 0 && changed_groups == 0 {
        return Err(unavailable());
    }
    let revision = operations::begin(connection, id, &digest, "undo")?;
    restore(connection, id, original, revision)?;
    operations::finish(connection, id, revision, changed_groups, false)
}

fn validate_targets(
    connection: &Connection,
    original: &ClassificationRecordId,
) -> Result<(), ScanError> {
    let invalid: bool = connection.query_row(&format!(
        "SELECT EXISTS(SELECT 1 FROM classification_operation_targets AS target
         LEFT JOIN classification_operation_changes AS change ON change.operation_id = target.operation_id
           AND change.record_id = target.record_id AND change.field = target.field
         LEFT JOIN classification_decisions AS decision ON decision.record_id = target.record_id
         WHERE target.operation_id = ?2 AND (decision.record_id IS NULL OR change.record_id IS NULL
           OR change.applied_revision <> CASE WHEN target.field = 'category' THEN decision.choice_revision ELSE decision.review_revision END
           OR EXISTS(SELECT 1 FROM organization_record_groups AS affected
             JOIN organization_record_groups AS member ON member.group_id = affected.group_id
             JOIN classification_decisions AS added ON added.record_id = member.record_id
             WHERE affected.record_id = target.record_id
               AND CASE WHEN target.field = 'category' THEN added.choice_revision ELSE added.review_revision END > 0
               AND NOT EXISTS(SELECT 1 FROM classification_operation_targets AS original
                 WHERE original.operation_id = target.operation_id AND original.field = target.field
                   AND original.record_id = member.record_id))
           OR NOT EXISTS(SELECT 1 FROM {CURRENT_LOCATIONS}
             JOIN organization_bindings AS binding ON binding.asset_id = l.asset_id AND binding.source_generation = l.source_generation
             JOIN organization_record_groups AS evidence ON evidence.record_id = binding.record_id
             JOIN organization_record_groups AS affected ON affected.group_id = evidence.group_id AND affected.record_id = target.record_id
             LEFT JOIN content_fingerprints AS f ON {COMPATIBLE_FINGERPRINT}
             WHERE binding.byte_length = l.file_size
               AND binding.file_identity_scheme IS l.file_identity_scheme AND binding.file_identity_value IS l.file_identity_value
               AND binding.source_revision_token IS l.source_revision_token
               AND (binding.digest IS NULL OR (binding.fingerprint_scheme = f.scheme AND binding.digest = f.digest)))))"
    ), params![EXACT_FINGERPRINT_SCHEME, original.as_str()], |row| row.get(0)).map_err(database_error)?;
    if invalid {
        return Err(unavailable());
    }
    Ok(())
}

fn restore(
    connection: &Connection,
    id: &ClassificationRecordId,
    original: &ClassificationRecordId,
    revision: i64,
) -> Result<(), ScanError> {
    let mut statement = connection
        .prepare_cached(
            "SELECT previous_payload FROM classification_operation_changes
         WHERE operation_id = ?1 AND field = 'category' AND previous_payload IS NOT NULL",
        )
        .map_err(database_error)?;
    let mut rows = statement
        .query([original.as_str()])
        .map_err(database_error)?;
    while let Some(row) = rows.next().map_err(database_error)? {
        codec::decode_choice(&row.get::<_, String>(0).map_err(database_error)?)?;
    }
    connection.execute(
        "INSERT INTO classification_operation_changes(operation_id, record_id, field, previous_payload,
         previous_revision, previous_restored, applied_revision)
         SELECT ?1, decision.record_id, change.field,
           CASE WHEN change.field = 'category' THEN decision.choice WHEN decision.deferred THEN 'true' ELSE 'false' END,
           CASE WHEN change.field = 'category' THEN decision.choice_revision ELSE decision.review_revision END,
           CASE WHEN change.field = 'category' THEN decision.choice_restored ELSE decision.review_restored END, ?3
         FROM classification_operation_changes AS change JOIN classification_decisions AS decision USING(record_id)
         WHERE change.operation_id = ?2", params![id.as_str(), original.as_str(), revision],
    ).map_err(database_error)?;
    connection.execute(
        "UPDATE classification_decisions AS decision SET
           choice = change.previous_payload, category_id = json_extract(change.previous_payload, '$.choice.category'),
           choice_restored = change.previous_restored, choice_revision = ?2
         FROM classification_operation_changes AS change
         WHERE change.operation_id = ?1 AND change.field = 'category' AND change.record_id = decision.record_id",
        params![original.as_str(), revision],
    ).map_err(database_error)?;
    connection.execute(
        "UPDATE classification_decisions AS decision SET
           deferred = change.previous_payload = 'true', review_restored = change.previous_restored, review_revision = ?2
         FROM classification_operation_changes AS change
         WHERE change.operation_id = ?1 AND change.field = 'review' AND change.record_id = decision.record_id",
        params![original.as_str(), revision],
    ).map_err(database_error)?;
    Ok(())
}

fn unavailable() -> ScanError {
    ScanError::new(
        "classification_undo_unavailable",
        "The affected classification or content changed; refresh before choosing another action",
    )
}
