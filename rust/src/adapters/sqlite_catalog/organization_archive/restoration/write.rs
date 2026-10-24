use std::path::Path;

use crate::domain::organization_archive::{
    OrganizationArchiveProgress, OrganizationArchiveRestoreReceipt,
    OrganizationArchiveRestoreRequest,
};
use crate::domain::{LibraryChangeLane, ScanError};

use super::super::super::{SqliteCatalog, unix_time_ms};
use super::super::{database_error, error, staging};
use super::{admission, apply_decisions};

pub(in super::super) fn write(
    catalog: &SqliteCatalog,
    archive: &Path,
    request: &OrganizationArchiveRestoreRequest,
    progress: &mut dyn FnMut(OrganizationArchiveProgress) -> bool,
) -> Result<OrganizationArchiveRestoreReceipt, ScanError> {
    request.validate()?;
    let digest = admission::digest(request);
    if let Some(receipt) = admission::replay(&catalog.connection, request, &digest)? {
        return Ok(receipt);
    }
    let parent = catalog
        .path
        .parent()
        .ok_or_else(|| error("staging", "Missing catalog directory"))?;
    let stage = staging::read(archive, parent, progress)?;
    let verified = stage.verified();
    if verified.header.archive_id != request.archive_id || verified.checksum != request.checksum {
        return Err(error(
            "input_changed",
            "The backup changed after inspection",
        ));
    }
    // This admitted connection owns the attachment and all temporary state. It closes before
    // the staging file on both success and rollback, including Windows file-handle retirement.
    let mut target = catalog
        .session
        .open_in_lane_with_read_stage(LibraryChangeLane::Live, |_| Ok(()))?;
    let stage_path = stage
        .path()
        .to_str()
        .ok_or_else(|| error("staging", "The staging path is not valid UTF-8"))?;
    target
        .connection
        .execute("ATTACH DATABASE ?1 AS recovery", [stage_path])
        .map_err(database_error)?;
    target.connection.execute_batch(
        "PRAGMA recovery.cache_size = -2048; PRAGMA temp.cache_size = -2048; PRAGMA temp_store = FILE;",
    ).map_err(database_error)?;
    let transaction = target.begin_user_interactive_write()?;
    if let Some(receipt) = admission::replay(&transaction, request, &digest)? {
        return Ok(receipt);
    }
    let now = unix_time_ms();
    let revisions = admission::admit(
        &transaction,
        request,
        &digest,
        verified.header.organization_revision,
        now,
    )?;
    apply_decisions(
        &transaction,
        &request.archive_id,
        revisions.base,
        now,
        progress,
    )?;
    transaction
        .execute(
            "UPDATE organization_state SET revision = ?1",
            [revisions.completed],
        )
        .map_err(database_error)?;
    transaction
        .execute(
            "UPDATE catalog_state SET revision = ?1",
            [revisions.catalog],
        )
        .map_err(database_error)?;
    transaction.commit().map_err(database_error)?;
    Ok(OrganizationArchiveRestoreReceipt {
        operation_id: request.operation_id.clone(),
        archive_id: request.archive_id.clone(),
        organization_revision: revisions.completed as u64,
    })
}

#[cfg(test)]
mod tests;
