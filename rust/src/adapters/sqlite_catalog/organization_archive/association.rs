use std::path::Path;

use rusqlite::{Connection, OpenFlags};

use crate::domain::ScanError;
use crate::domain::organization_archive::{
    OrganizationArchiveAssociations, OrganizationArchivePhase, OrganizationArchiveProgress,
};

use super::super::exact_duplicates::read_unsigned;
use super::{database_error, error, report, staging::StagedArchive};

mod current;
mod matching;

pub(super) fn inspect(
    catalog: &Path,
    stage: &StagedArchive,
    progress: &mut dyn FnMut(OrganizationArchiveProgress) -> bool,
) -> Result<OrganizationArchiveAssociations, ScanError> {
    let connection = Connection::open_with_flags(catalog, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(database_error)?;
    connection
        .execute_batch("PRAGMA temp_store = FILE; PRAGMA cache_size = -2048;")
        .map_err(database_error)?;
    let stage_path = stage
        .path()
        .to_str()
        .ok_or_else(|| error("staging", "The staging path is not valid UTF-8"))?;
    connection
        .execute("ATTACH DATABASE ?1 AS recovery", [stage_path])
        .map_err(database_error)?;
    connection
        .execute_batch("PRAGMA recovery.cache_size = -2048; PRAGMA temp.cache_size = -2048;")
        .map_err(database_error)?;
    let transaction = connection.unchecked_transaction().map_err(database_error)?;
    let mut result = transaction
        .query_row(
            "SELECT catalog_state.revision, organization_state.revision
             FROM catalog_state CROSS JOIN organization_state",
            [],
            |row| {
                Ok(OrganizationArchiveAssociations {
                    catalog_revision: read_unsigned(row, 0)?,
                    organization_revision: read_unsigned(row, 1)?,
                    ..Default::default()
                })
            },
        )
        .map_err(database_error)?;
    let mut completed = 0;
    let mut checkpoint = |count| {
        completed += count;
        report(progress, OrganizationArchivePhase::Matching, completed)
    };
    checkpoint(0)?;
    current::capture(&transaction, &mut checkpoint)?;
    matching::groups(&transaction, &mut checkpoint)?;
    (result.matched_groups, result.unmatched_groups) = transaction
        .query_row(
            "SELECT
               (SELECT COUNT(*) FROM temp.organization_restore_matched_groups),
               (SELECT COUNT(*) FROM recovery.archive_group WHERE record_id = group_id)
                 - (SELECT COUNT(*) FROM temp.organization_restore_matched_groups)",
            [],
            |row| Ok((read_unsigned(row, 0)?, read_unsigned(row, 1)?)),
        )
        .map_err(database_error)?;
    (
        result.resolved_preferred_copies,
        result.unresolved_preferred_copies,
    ) = matching::preferred_copies(&transaction, &mut checkpoint)?;
    checkpoint(0)?;
    // The read snapshot and its temporary matches expire together. These counts never authorize
    // a later write; application must match again under its own transaction and checksum guard.
    Ok(result)
}

#[cfg(test)]
mod tests;
