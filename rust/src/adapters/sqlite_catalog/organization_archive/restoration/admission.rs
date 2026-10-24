use rusqlite::{Connection, OptionalExtension, params};

use crate::domain::ScanError;
use crate::domain::organization_archive::{
    OrganizationArchiveRestoreReceipt, OrganizationArchiveRestoreRequest,
};

use super::super::{database_error, error};

pub(super) fn digest(request: &OrganizationArchiveRestoreRequest) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"ame-organization-archive-restore-v1\0");
    for value in [&request.archive_id, &request.checksum] {
        hasher.update(&(value.len() as u64).to_le_bytes());
        hasher.update(value.as_bytes());
    }
    hasher.update(&request.expected_catalog_revision.to_le_bytes());
    hasher.update(&request.expected_organization_revision.to_le_bytes());
    hasher.finalize().to_hex().to_string()
}

pub(super) fn replay(
    connection: &Connection,
    request: &OrganizationArchiveRestoreRequest,
    digest: &str,
) -> Result<Option<OrganizationArchiveRestoreReceipt>, ScanError> {
    let previous: Option<(String, String, i64)> = connection.query_row(
        "SELECT request_digest, operation_kind, revision FROM organization_operations WHERE operation_id = ?1",
        [&request.operation_id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    ).optional().map_err(database_error)?;
    let Some((stored, kind, revision)) = previous else {
        return Ok(None);
    };
    if stored != digest || kind != "restore" {
        return Err(error(
            "operation_conflict",
            "This operation identifier belongs to another request",
        ));
    }
    Ok(Some(OrganizationArchiveRestoreReceipt {
        operation_id: request.operation_id.clone(),
        archive_id: request.archive_id.clone(),
        organization_revision: revision as u64,
    }))
}

pub(super) struct Revisions {
    pub base: i64,
    pub completed: i64,
    pub catalog: i64,
}

pub(super) fn admit(
    connection: &Connection,
    request: &OrganizationArchiveRestoreRequest,
    digest: &str,
    source_revision: i64,
    now: i64,
) -> Result<Revisions, ScanError> {
    let (catalog, organization): (i64, i64) = connection.query_row(
        "SELECT catalog_state.revision, organization_state.revision FROM catalog_state CROSS JOIN organization_state",
        [], |row| Ok((row.get(0)?, row.get(1)?)),
    ).map_err(database_error)?;
    if catalog as u64 != request.expected_catalog_revision
        || organization as u64 != request.expected_organization_revision
    {
        return Err(error(
            "inspection_changed",
            "The library changed; inspect the backup again before restoring",
        ));
    }
    let capacity = || {
        error(
            "revision_capacity",
            "The catalog cannot allocate more decision revisions",
        )
    };
    let base = organization.checked_add(1).ok_or_else(capacity)?;
    let completed = base.checked_add(source_revision).ok_or_else(capacity)?;
    let catalog = catalog.checked_add(1).ok_or_else(capacity)?;
    let retained: Option<String> = connection
        .query_row(
            "SELECT checksum FROM organization_archive_restores WHERE archive_id = ?1",
            [&request.archive_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(database_error)?;
    if retained
        .as_ref()
        .is_some_and(|value| value != &request.checksum)
    {
        return Err(error(
            "identity_conflict",
            "The backup identity already belongs to different verified contents",
        ));
    }
    let operation_collision: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM recovery.archive_operation WHERE operation_id = ?1)",
            [&request.operation_id],
            |row| row.get(0),
        )
        .map_err(database_error)?;
    if operation_collision {
        return Err(error(
            "operation_conflict",
            "The restore identifier is already present in the backup history",
        ));
    }
    connection
        .execute(
            "INSERT INTO organization_operations VALUES (?1, ?2, 'restore', ?3, 0, ?4)",
            params![request.operation_id, digest, completed, now],
        )
        .map_err(database_error)?;
    connection
        .execute(
            "INSERT INTO organization_archive_restores VALUES (?1, ?2, ?3, ?4, ?5, ?6)
         ON CONFLICT(archive_id) DO NOTHING",
            params![
                request.archive_id,
                request.checksum,
                request.operation_id,
                base,
                source_revision,
                now
            ],
        )
        .map_err(database_error)?;
    Ok(Revisions {
        base,
        completed,
        catalog,
    })
}
