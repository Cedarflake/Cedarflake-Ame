use rusqlite::{Connection, params};

use crate::domain::ScanError;

use super::super::exact_duplicates::read_unsigned;
use super::super::{database_error, unix_time_ms};

pub(super) fn retire(
    connection: &Connection,
    operation_id: &str,
    album_id: &str,
    revision: i64,
) -> Result<u64, ScanError> {
    let changed_groups = connection
        .query_row(
            "SELECT COUNT(*) FROM organization_records AS record
         JOIN organization_record_groups AS member ON member.record_id = record.record_id
         WHERE member.group_id = member.record_id AND record.album_id = ?1",
            [album_id],
            |row| read_unsigned(row, 0),
        )
        .map_err(database_error)?;
    // Alias fields retain lineage history, while only the canonical record owns membership.
    // Retiring an old album reference must not turn an unresolved group conflict into ignored.
    connection.execute(
        "UPDATE organization_records AS record SET (album_id, disposition, organization_revision) = (
           SELECT canonical.album_id, canonical.disposition, canonical.organization_revision
           FROM organization_record_groups AS member
           JOIN organization_records AS canonical ON canonical.record_id = member.group_id
           WHERE member.record_id = record.record_id)
         WHERE record.album_id = ?1 AND EXISTS(
           SELECT 1 FROM organization_record_groups AS member
           JOIN organization_records AS canonical ON canonical.record_id = member.group_id
           WHERE member.record_id = record.record_id AND canonical.album_id IS NOT ?1)",
        [album_id],
    ).map_err(database_error)?;
    connection.execute(
        "INSERT INTO organization_operation_changes(operation_id, record_id, field, previous_album_id, previous_disposition, applied_revision)
         SELECT ?1, member.record_id, 'organization', canonical.album_id, canonical.disposition, ?2
         FROM organization_record_groups AS member
         JOIN organization_records AS canonical ON canonical.record_id = member.group_id
         WHERE canonical.album_id = ?3", params![operation_id, revision, album_id],
    ).map_err(database_error)?;
    connection.execute(
        "UPDATE organization_records SET album_id = NULL, disposition = 'ignored', organization_revision = ?1, updated_unix_ms = ?2
         WHERE record_id IN (SELECT record_id FROM organization_operation_changes WHERE operation_id = ?3 AND field = 'organization')",
        params![revision, unix_time_ms(), operation_id],
    ).map_err(database_error)?;
    Ok(changed_groups)
}
