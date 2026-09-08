use rusqlite::{Connection, Transaction, params};

use crate::domain::ScanError;

use super::database_error;

#[cfg(test)]
mod tests;

pub(super) fn has_candidates(connection: &Connection) -> Result<bool, ScanError> {
    connection.query_row(
        "SELECT CASE WHEN EXISTS(SELECT 1 FROM library_metadata_inventory_spools WHERE state = 'retired')
         THEN 1 ELSE EXISTS(SELECT 1 FROM library_metadata_inventory_spool_entries AS entry
           INDEXED BY library_metadata_inventory_spool_entries_initial
           WHERE directory_relative_path IS NULL AND NOT EXISTS(
             SELECT 1 FROM library_metadata_inventory_spools AS spool WHERE spool.run_id = entry.run_id
           )) END", [], |row| row.get(0),
    ).map_err(database_error)
}

pub(super) fn cleanup_batch(
    transaction: &Transaction<'_>,
    entry_limit: u32,
    metadata_limit: u32,
) -> Result<u32, ScanError> {
    let removed = transaction
        .execute(
            "DELETE FROM library_metadata_inventory_spool_entries WHERE rowid IN (
           SELECT entry.rowid FROM library_metadata_inventory_spool_entries AS entry
           WHERE entry.run_id IN (
             SELECT run_id FROM library_metadata_inventory_spools
             WHERE state = 'retired' ORDER BY run_id LIMIT ?2
           ) ORDER BY entry.run_id, entry.relative_path LIMIT ?1
         )",
            params![entry_limit, metadata_limit],
        )
        .map_err(database_error)?;
    let remaining = usize::try_from(entry_limit)
        .map_err(|_| count_error())?
        .checked_sub(removed)
        .ok_or_else(count_error)?;
    let orphans = transaction.execute(
        "DELETE FROM library_metadata_inventory_spool_entries WHERE rowid IN (
           SELECT entry.rowid FROM library_metadata_inventory_spool_entries AS entry
           WHERE directory_relative_path IS NULL AND NOT EXISTS(
             SELECT 1 FROM library_metadata_inventory_spools AS spool WHERE spool.run_id = entry.run_id
           ) ORDER BY run_id, relative_path LIMIT ?1
         )", [i64::try_from(remaining).map_err(|_| count_error())?],
    ).map_err(database_error)?;
    transaction.execute(
        "DELETE FROM library_metadata_inventory_spool_directories WHERE rowid IN (
           SELECT directory.rowid FROM (
             SELECT rowid, run_id, relative_directory FROM library_metadata_inventory_spool_directories
             WHERE run_id IN (
               SELECT run_id FROM library_metadata_inventory_spools
               WHERE state = 'retired' ORDER BY run_id LIMIT ?1
             ) ORDER BY run_id, ordinal LIMIT ?1
           ) AS directory WHERE NOT EXISTS(
             SELECT 1 FROM library_metadata_inventory_spool_entries AS entry
             WHERE entry.run_id = directory.run_id AND entry.directory_relative_path = directory.relative_directory
           )
         )", [metadata_limit],
    ).map_err(database_error)?;
    transaction.execute(
        "DELETE FROM library_metadata_inventory_spools WHERE run_id IN (
           SELECT spool.run_id FROM (
             SELECT run_id FROM library_metadata_inventory_spools
             WHERE state = 'retired' ORDER BY run_id LIMIT ?1
           ) AS spool
           WHERE NOT EXISTS(SELECT 1 FROM library_metadata_inventory_spool_directories AS directory WHERE directory.run_id = spool.run_id)
             AND NOT EXISTS(SELECT 1 FROM library_metadata_inventory_spool_entries AS entry WHERE entry.run_id = spool.run_id)
         )", [metadata_limit],
    ).map_err(database_error)?;
    u32::try_from(removed + orphans).map_err(|_| count_error())
}

fn count_error() -> ScanError {
    ScanError::new(
        "metadata_inventory_cleanup_count_overflow",
        "Raw inventory cleanup exceeded its entry budget",
    )
}
