use rusqlite::Connection;

use crate::domain::ScanError;
use crate::domain::organization_archive::{OrganizationArchivePhase, OrganizationArchiveProgress};

mod admission;
mod duplicates;
mod history;
mod identifiers;
mod identities;
mod queue;
mod records;
mod write;

pub(super) use write::write;

pub(super) fn apply_decisions(
    connection: &Connection,
    archive_id: &str,
    revision_base: i64,
    now: i64,
    progress: &mut dyn FnMut(OrganizationArchiveProgress) -> bool,
) -> Result<(), ScanError> {
    super::report(progress, OrganizationArchivePhase::Restoring, 0)?;
    identities::generations(connection, archive_id)?;
    identities::roots(connection, archive_id)?;
    identifiers::prepare(connection, archive_id)?;
    super::report(progress, OrganizationArchivePhase::Restoring, 1)?;
    records::prepare(connection)?;
    records::decisions(connection, revision_base, now)?;
    records::bindings(connection, archive_id)?;
    super::report(progress, OrganizationArchivePhase::Restoring, 2)?;
    history::apply(connection, revision_base)?;
    duplicates::retain(connection, now)?;
    super::report(progress, OrganizationArchivePhase::Restoring, 3)?;
    queue::apply(connection, archive_id)?;
    super::report(progress, OrganizationArchivePhase::Restoring, 4)
}

#[cfg(test)]
mod tests;
