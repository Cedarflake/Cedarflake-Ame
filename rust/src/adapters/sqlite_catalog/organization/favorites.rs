use rusqlite::{Connection, OptionalExtension, params};

use crate::domain::ScanError;
use crate::domain::organization::{
    OrganizationPreparedSelection, OrganizationWriteReceipt, stale_selection,
};

use super::super::exact_duplicates::read_unsigned;
use super::super::{database_error, sqlite_integer, unix_time_ms};
use super::{authority, records, selection};

pub(super) fn save(
    connection: &Connection,
    operation_id: &str,
    prepared: &OrganizationPreparedSelection,
    favorite: bool,
) -> Result<OrganizationWriteReceipt, ScanError> {
    let digest = authority::favorite_request(prepared, favorite);
    let previous: Option<(String, u64, u64)> = connection.query_row(
        "SELECT request_digest, revision, changed_groups FROM organization_operations WHERE operation_id = ?1",
        [operation_id], |row| Ok((row.get(0)?, read_unsigned(row, 1)?, read_unsigned(row, 2)?))
    ).optional().map_err(database_error)?;
    if let Some((request_digest, organization_revision, changed_groups)) = previous {
        if request_digest != digest {
            return Err(ScanError::new(
                "organization_operation_conflict",
                "This operation identifier already belongs to a different request",
            ));
        }
        return Ok(OrganizationWriteReceipt {
            operation_id: operation_id.to_owned(),
            organization_revision,
            changed_groups,
        });
    }
    if &selection::prepare(connection, &prepared.selections)? != prepared {
        return Err(stale_selection());
    }
    let next_revision = sqlite_integer(prepared.organization_revision, "organization revision")?
        .checked_add(1)
        .ok_or_else(stale_selection)?;
    connection.execute(
        "INSERT INTO organization_operations(operation_id, request_digest, operation_kind, revision, changed_groups, created_unix_ms)
         VALUES (?1, ?2, 'favorite', ?3, 0, ?4)",
        params![operation_id, digest, sqlite_integer(prepared.organization_revision, "organization revision")?, unix_time_ms()]
    ).map_err(database_error)?;
    let mut changed_groups = 0_u64;
    for group in &prepared.groups {
        records::materialize(connection, &group.key)?;
        let changed = connection.execute(&format!(
            "INSERT INTO organization_operation_changes(operation_id, record_id, field, previous_favorite, applied_revision)
             SELECT ?1, record_id, 'favorite', favorite, ?2 FROM organization_records
             WHERE record_id IN ({}) AND favorite <> ?3", records::CURRENT_RECORDS
        ), params![operation_id, next_revision, favorite]).map_err(database_error)?;
        if changed == 0 {
            continue;
        }
        connection.execute(&format!(
            "UPDATE organization_records SET favorite = ?1, favorite_revision = ?2, updated_unix_ms = ?3
             WHERE record_id IN ({}) AND favorite <> ?1", records::CURRENT_RECORDS
        ), params![favorite, next_revision, unix_time_ms()]).map_err(database_error)?;
        changed_groups += 1;
    }
    let organization_revision = if changed_groups > 0 {
        connection
            .execute(
                "UPDATE organization_state SET revision = ?1 WHERE singleton = 1",
                [next_revision],
            )
            .map_err(database_error)?;
        connection
            .execute("UPDATE catalog_state SET revision = revision + 1", [])
            .map_err(database_error)?;
        next_revision as u64
    } else {
        prepared.organization_revision
    };
    connection.execute(
        "UPDATE organization_operations SET revision = ?1, changed_groups = ?2 WHERE operation_id = ?3",
        params![sqlite_integer(organization_revision, "organization revision")?, sqlite_integer(changed_groups, "changed groups")?, operation_id]
    ).map_err(database_error)?;
    Ok(OrganizationWriteReceipt {
        operation_id: operation_id.to_owned(),
        organization_revision,
        changed_groups,
    })
}
