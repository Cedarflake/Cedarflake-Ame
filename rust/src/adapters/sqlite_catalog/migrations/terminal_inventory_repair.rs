use rusqlite::{Connection, Transaction};

use crate::domain::ScanError;

use super::super::spool_retirement;
use super::compatibility_repair_shape::inventory_shape_matches;
use super::database_error;

pub(super) fn needed(connection: &Connection, schema_version: i64) -> Result<bool, ScanError> {
    if !inventory_shape_matches(connection, schema_version)? {
        return Ok(false);
    }
    let terminal_authority = connection
        .query_row(
            "SELECT EXISTS(
               SELECT 1 FROM library_metadata_inventory_runs AS run
               WHERE run.status IN ('failed', 'cancelled', 'superseded')
                 AND run.absence_authority <> 0
                 AND NOT EXISTS(
                   SELECT 1 FROM library_recovery_authorities AS authority
                   WHERE authority.run_id = run.id
                     AND authority.root_id = run.root_id
                     AND authority.root_generation = run.root_generation
                     AND authority.retired_unix_ms IS NULL
                 )
             )",
            [],
            |row| row.get::<_, bool>(0),
        )
        .map_err(database_error)?;
    let terminal_spool = if schema_version >= 27 {
        connection
            .query_row(
                "SELECT EXISTS(
                   SELECT 1 FROM library_metadata_inventory_spools AS spool
                   JOIN library_metadata_inventory_runs AS run ON run.id = spool.run_id
                   WHERE run.status IN ('failed', 'cancelled', 'superseded')
                     AND spool.state <> 'retired'
                 )",
                [],
                |row| row.get::<_, bool>(0),
            )
            .map_err(database_error)?
    } else {
        false
    };
    Ok(terminal_authority || terminal_spool)
}

pub(super) fn apply(transaction: &Transaction<'_>, schema_version: i64) -> Result<(), ScanError> {
    if !inventory_shape_matches(transaction, schema_version)? {
        return Ok(());
    }
    if schema_version >= 32 {
        spool_retirement::retire_owned_spools(
            transaction,
            spool_retirement::SpoolOwner::LegacyTerminalRuns,
        )?;
    } else if schema_version >= 27 {
        spool_retirement::delete_owned_spools(
            transaction,
            spool_retirement::SpoolOwner::LegacyTerminalRuns,
        )?;
    }
    transaction
        .execute(
            "UPDATE library_metadata_inventory_runs AS run
             SET absence_authority = 0
             WHERE run.status IN ('failed', 'cancelled', 'superseded')
               AND run.absence_authority <> 0
               AND NOT EXISTS(
                 SELECT 1 FROM library_recovery_authorities AS authority
                 WHERE authority.run_id = run.id
                   AND authority.root_id = run.root_id
                   AND authority.root_generation = run.root_generation
                   AND authority.retired_unix_ms IS NULL
               )",
            [],
        )
        .map_err(database_error)?;
    Ok(())
}

#[cfg(test)]
mod tests;

#[cfg(test)]
pub(super) mod test_catalog;
