use std::sync::atomic::AtomicBool;

use rusqlite::{Transaction, params};

use crate::domain::{IncrementalCatalogRoot, ScanError};

use super::super::{database_error, sqlite_integer};
use super::write_attempt::LocationWriteAttempt;

mod journal_claims;
mod payload_cleanup;

pub(super) fn retire_inventory(
    transaction: &Transaction<'_>,
    root: &IncrementalCatalogRoot,
    now: i64,
    cancelled: &AtomicBool,
    attempt: &LocationWriteAttempt,
) -> Result<(), ScanError> {
    let generation = sqlite_integer(root.root_generation.value(), "root generation")?;
    if !is_bounded_retirement(transaction, &root.root_id, generation)? {
        return Err(ScanError::new(
            "root_location_inventory_cleanup_required",
            "Directory recovery exceeds the bounded inventory control-row limit",
        ));
    }
    journal_claims::retire(transaction, &root.root_id, generation)?;
    payload_cleanup::remove_pages(transaction, &root.root_id, generation, cancelled, attempt)?;
    transaction
        .execute(
            "UPDATE library_metadata_inventory_spools SET state = 'retired'
         WHERE root_id = ?1 AND root_generation = ?2 AND state <> 'retired'",
            params![root.root_id, generation],
        )
        .map_err(database_error)?;
    transaction
        .execute(
            "DELETE FROM library_persistent_journal_baselines
         WHERE root_id = ?1 AND root_generation = ?2",
            params![root.root_id, generation],
        )
        .map_err(database_error)?;
    transaction
        .execute(
            "DELETE FROM library_metadata_inventory_runs
         WHERE root_id = ?1 AND root_generation = ?2 AND status <> 'completed'",
            params![root.root_id, generation],
        )
        .map_err(database_error)?;
    transaction
        .execute(
            "UPDATE library_recovery_authorities
         SET retired_unix_ms = MAX(authorized_unix_ms, ?3)
         WHERE root_id = ?1 AND root_generation = ?2 AND retired_unix_ms IS NULL",
            params![root.root_id, generation, now],
        )
        .map_err(database_error)?;
    Ok(())
}

fn is_bounded_retirement(
    transaction: &Transaction<'_>,
    root_id: &str,
    generation: i64,
) -> Result<bool, ScanError> {
    for selection in [
        "SELECT 1 FROM library_metadata_inventory_runs
         WHERE root_id = ?1 AND root_generation = ?2 AND status <> 'completed'",
        "SELECT 1 FROM library_persistent_journal_baselines
         WHERE root_id = ?1 AND root_generation = ?2",
        "SELECT 1 FROM library_metadata_inventory_spools
         WHERE root_id = ?1 AND root_generation = ?2 AND state <> 'retired'",
        "SELECT 1 FROM library_recovery_authorities
         WHERE root_id = ?1 AND root_generation = ?2 AND retired_unix_ms IS NULL",
    ] {
        let mut statement = transaction
            .prepare(&format!("{selection} LIMIT 257"))
            .map_err(database_error)?;
        let mut rows = statement
            .query(params![root_id, generation])
            .map_err(database_error)?;
        let mut count = 0;
        while rows.next().map_err(database_error)?.is_some() {
            count += 1;
        }
        if count > 256 {
            return Ok(false);
        }
    }
    Ok(true)
}
