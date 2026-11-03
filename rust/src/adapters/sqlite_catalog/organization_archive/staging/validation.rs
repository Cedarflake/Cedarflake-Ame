use rusqlite::Connection;

use crate::domain::ScanError;
use crate::domain::organization::{normalize_album_name, validate_identifier};

use super::super::{codec::Cell, database_error, error};

pub(super) fn record(kind: &str, values: &[Cell]) -> Result<(), ScanError> {
    if kind.starts_with("classification_") {
        return super::classification::record(kind, values);
    }
    let identifiers: &[usize] = match kind {
        "album" | "retired_album" | "operation" | "root_evidence" => &[0],
        "record" | "binding" | "source_owner" | "group" | "conflict" | "album_change"
        | "operation_target" | "review_operation" => &[0, 1],
        "operation_change" => &[0, 1, 3],
        "operation_conflict" | "version_evidence" => &[0, 1, 2],
        "review_scope" => &[1, 2],
        "bookmark" => &[0, 2, 3],
        "admission_source" => &[1],
        "duplicate_decision" => &[3, 4],
        "admission" => &[],
        _ => return Err(invalid()),
    };
    for index in identifiers {
        if let Some(Cell::Text(value)) = values.get(*index) {
            validate_identifier(value).map_err(|_| invalid())?;
        }
    }
    let name = match kind {
        "album" => values.get(1),
        "album_change" => values.get(2),
        _ => None,
    };
    if let Some(Cell::Text(name)) = name {
        normalize_album_name(name).map_err(|_| invalid())?;
    }
    if kind == "review_scope" {
        super::metadata_filter::validate(values)?;
    }
    Ok(())
}

const RELATIONSHIPS: &[&str] = &[
    "SELECT EXISTS(SELECT 1 FROM pragma_foreign_key_check)",
    "SELECT EXISTS(SELECT 1 FROM archive_record AS record
       LEFT JOIN archive_group AS member ON member.record_id = record.record_id
       LEFT JOIN archive_group AS leader ON leader.record_id = member.group_id
       WHERE member.record_id IS NULL OR leader.group_id IS NOT member.group_id)",
    "SELECT EXISTS(SELECT 1 FROM archive_operation_change AS change
       LEFT JOIN archive_operation_target AS target ON target.operation_id = change.operation_id
         AND target.record_id = change.record_id AND target.field = change.field
       WHERE target.operation_id IS NULL)
     OR EXISTS(SELECT 1 FROM archive_operation_target AS target
       JOIN archive_record AS record ON record.record_id = target.record_id
       LEFT JOIN archive_operation_change AS change ON change.operation_id = target.operation_id
         AND change.record_id = target.record_id AND change.field = target.field
       WHERE change.operation_id IS NULL)",
    "SELECT EXISTS(SELECT 1 FROM archive_record AS record
       JOIN archive_retired_album AS retired ON retired.album_id = record.album_id)",
    "SELECT EXISTS(SELECT 1 FROM archive_conflict AS conflict
       JOIN archive_record AS record ON record.record_id = conflict.record_id
       JOIN archive_group AS member ON member.record_id = record.record_id
       WHERE record.disposition <> 'deferred' OR record.album_id IS NOT NULL
         OR member.record_id <> member.group_id)",
    "SELECT EXISTS(SELECT 1 FROM archive_bookmark)
       AND NOT EXISTS(SELECT 1 FROM archive_review_scope)",
    "SELECT EXISTS(SELECT 1 FROM archive_bookmark AS bookmark
       WHERE NOT EXISTS(SELECT 1 FROM archive_admission WHERE admission_id = bookmark.admission_id)
         OR NOT EXISTS(SELECT 1 FROM archive_root_evidence WHERE root_id = bookmark.root_id))",
    "SELECT EXISTS(SELECT 1 FROM archive_review_scope AS scope WHERE scope.root_id IS NOT NULL
       AND NOT EXISTS(SELECT 1 FROM archive_root_evidence WHERE root_id = scope.root_id))",
    "SELECT EXISTS(SELECT 1 FROM archive_admission AS admission
       WHERE NOT EXISTS(SELECT 1 FROM archive_admission_source AS source WHERE source.admission_id = admission.admission_id))",
    "SELECT EXISTS(SELECT 1 FROM archive_operation_change AS change
       JOIN archive_operation AS operation USING(operation_id) WHERE change.applied_revision <> operation.revision)
     OR EXISTS(SELECT 1 FROM archive_album_change AS change
       JOIN archive_operation AS operation USING(operation_id) WHERE change.applied_revision <> operation.revision)
     OR EXISTS(SELECT 1 FROM archive_operation_conflict AS conflict
       JOIN archive_operation AS operation USING(operation_id) WHERE conflict.decision_revision > operation.revision)",
];

const REVISION_CHECKS: &[&str] = &[
    "SELECT EXISTS(SELECT 1 FROM archive_album WHERE revision > ?1)",
    "SELECT EXISTS(SELECT 1 FROM archive_record WHERE organization_revision > ?1 OR favorite_revision > ?1)",
    "SELECT EXISTS(SELECT 1 FROM archive_conflict WHERE decision_revision > ?1)",
    "SELECT EXISTS(SELECT 1 FROM archive_operation WHERE revision > ?1)",
    "SELECT EXISTS(SELECT 1 FROM archive_operation_change WHERE applied_revision > ?1)",
    "SELECT EXISTS(SELECT 1 FROM archive_retired_album WHERE revision > ?1)",
    "SELECT EXISTS(SELECT 1 FROM archive_album_change WHERE applied_revision > ?1)",
    "SELECT EXISTS(SELECT 1 FROM archive_operation_conflict WHERE decision_revision > ?1)",
];

pub(super) fn validate(
    connection: &Connection,
    revision: i64,
    checkpoint: &mut dyn FnMut() -> Result<(), ScanError>,
) -> Result<(), ScanError> {
    for query in RELATIONSHIPS {
        checkpoint()?;
        if connection
            .query_row(query, [], |row| row.get::<_, bool>(0))
            .map_err(database_error)?
        {
            return Err(invalid());
        }
    }
    for query in REVISION_CHECKS {
        checkpoint()?;
        if connection
            .query_row(query, [revision], |row| row.get::<_, bool>(0))
            .map_err(database_error)?
        {
            return Err(invalid());
        }
    }
    Ok(())
}

fn invalid() -> ScanError {
    error(
        "records_invalid",
        "The backup contains inconsistent organization records",
    )
}
