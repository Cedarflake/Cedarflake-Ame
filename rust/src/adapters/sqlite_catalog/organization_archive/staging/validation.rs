use rusqlite::Connection;

use crate::domain::ScanError;

use super::super::{database_error, error};

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
