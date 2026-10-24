use rusqlite::{Connection, params};

use crate::domain::ScanError;

use super::super::database_error;
use super::restoration_schema::RestorationSchema;

mod duplicates;
mod queue;
mod sources;

pub(super) use sources::{forget_source, transfer_source};

pub(in crate::adapters::sqlite_catalog) fn root(
    connection: &Connection,
    root_id: &str,
) -> Result<(), ScanError> {
    if !has_history(connection)? {
        return Ok(());
    }
    sources::associate(connection, root_id)?;
    let mut after = 0;
    loop {
        let generations = connection.prepare(
            "SELECT DISTINCT location.source_generation FROM asset_locations AS location
             JOIN library_roots AS root ON root.id = location.root_id AND root.active_scan_id = location.scan_id
             WHERE location.root_id = ?1 AND location.source_generation > ?2
             ORDER BY location.source_generation LIMIT 256",
        ).map_err(database_error)?.query_map(params![root_id, after], |row| row.get::<_, i64>(0))
            .map_err(database_error)?.collect::<rusqlite::Result<Vec<_>>>().map_err(database_error)?;
        let Some(last) = generations.last() else {
            break;
        };
        after = *last;
        queue::reconcile(connection, &generations)?;
    }
    Ok(())
}

pub(super) fn batch(connection: &Connection, generations: &[i64]) -> Result<(), ScanError> {
    if has_history(connection)? {
        queue::reconcile(connection, generations)?;
    }
    Ok(())
}

fn has_history(connection: &Connection) -> Result<bool, ScanError> {
    if matches!(
        RestorationSchema::read(connection)?,
        RestorationSchema::LocalDecisionsOnly
    ) {
        return Ok(false);
    }
    connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM organization_recovery_admissions)
         OR EXISTS(SELECT 1 FROM organization_recovery_duplicate_decisions)",
            [],
            |row| row.get(0),
        )
        .map_err(database_error)
}
