use rusqlite::{Connection, OptionalExtension, params};

use crate::domain::ScanError;
use crate::domain::organization::{OrganizationWriteReceipt, stale_selection};

use super::super::exact_duplicates::read_unsigned;
use super::super::{database_error, sqlite_integer, unix_time_ms};
use super::authority::field;

pub(super) enum Admission {
    Replayed(OrganizationWriteReceipt),
    New { revision: i64 },
}

pub(super) struct GroupWrite {
    pub changed: bool,
    pub reconciled: bool,
}

pub(super) fn request_digest(kind: &str, revision: u64, values: &[&str]) -> String {
    let mut hasher = blake3::Hasher::new();
    field(&mut hasher, b"ame-organization-operation-v1");
    field(&mut hasher, kind.as_bytes());
    field(&mut hasher, &revision.to_le_bytes());
    for value in values {
        field(&mut hasher, value.as_bytes());
    }
    hasher.finalize().to_hex().to_string()
}

pub(super) fn admit(
    connection: &Connection,
    operation_id: &str,
    request_digest: &str,
    kind: &str,
    expected_revision: u64,
) -> Result<Admission, ScanError> {
    let previous = connection.query_row(
        "SELECT request_digest, revision, changed_groups FROM organization_operations WHERE operation_id = ?1",
        [operation_id], |row| Ok((row.get::<_, String>(0)?, read_unsigned(row, 1)?, read_unsigned(row, 2)?))
    ).optional().map_err(database_error)?;
    if let Some((stored_digest, organization_revision, changed_groups)) = previous {
        if stored_digest != request_digest {
            return Err(ScanError::new(
                "organization_operation_conflict",
                "This operation identifier already belongs to a different request",
            ));
        }
        return Ok(Admission::Replayed(OrganizationWriteReceipt {
            operation_id: operation_id.to_owned(),
            organization_revision,
            changed_groups,
        }));
    }
    let current: i64 = connection
        .query_row(
            "SELECT revision FROM organization_state WHERE singleton = 1",
            [],
            |row| row.get(0),
        )
        .map_err(database_error)?;
    if current != sqlite_integer(expected_revision, "organization revision")? {
        return Err(stale_selection());
    }
    let revision = current.checked_add(1).ok_or_else(stale_selection)?;
    connection.execute(
        "INSERT INTO organization_operations(operation_id, request_digest, operation_kind, revision, changed_groups, created_unix_ms)
         VALUES (?1, ?2, ?3, ?4, 0, ?5)", params![operation_id, request_digest, kind, current, unix_time_ms()]
    ).map_err(database_error)?;
    Ok(Admission::New { revision })
}

pub(super) fn finish(
    connection: &Connection,
    operation_id: &str,
    revision: i64,
    changed_groups: u64,
    metadata_changed: bool,
) -> Result<OrganizationWriteReceipt, ScanError> {
    connection.execute(
        "INSERT OR IGNORE INTO organization_operation_targets SELECT operation_id, record_id, field
         FROM organization_operation_changes WHERE operation_id = ?1", [operation_id]
    ).map_err(database_error)?;
    let committed_revision = if changed_groups > 0 || metadata_changed {
        connection
            .execute(
                "UPDATE organization_state SET revision = ?1 WHERE singleton = 1",
                [revision],
            )
            .map_err(database_error)?;
        connection
            .execute("UPDATE catalog_state SET revision = revision + 1", [])
            .map_err(database_error)?;
        revision
    } else {
        revision - 1
    };
    connection.execute("UPDATE organization_operations SET revision = ?1, changed_groups = ?2 WHERE operation_id = ?3",
        params![committed_revision, sqlite_integer(changed_groups, "changed groups")?, operation_id]).map_err(database_error)?;
    Ok(OrganizationWriteReceipt {
        operation_id: operation_id.to_owned(),
        organization_revision: committed_revision as u64,
        changed_groups,
    })
}
