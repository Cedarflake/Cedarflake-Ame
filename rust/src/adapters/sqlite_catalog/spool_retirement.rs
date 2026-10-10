use rusqlite::{Transaction, params};

use crate::domain::ScanError;

use super::database_error;

pub(super) enum SpoolOwner<'a> {
    Run(&'a str),
    Root(&'a str),
    Change(i64),
    LegacyTerminalRuns,
}

pub(super) fn delete_owned_spools(
    transaction: &Transaction<'_>,
    owner: SpoolOwner<'_>,
) -> Result<(), ScanError> {
    let (predicate, identity) = owner_selection(owner);
    // Legacy migrations still own the pre-v32 cascade graph.
    transaction
        .execute(
            &format!(
                "DELETE FROM library_metadata_inventory_spool_entries
                 WHERE directory_relative_path IS NULL AND run_id IN (
                   SELECT run_id FROM library_metadata_inventory_spools WHERE {predicate}
                 )"
            ),
            params![identity],
        )
        .map_err(database_error)?;
    transaction
        .execute(
            &format!("DELETE FROM library_metadata_inventory_spools WHERE {predicate}"),
            params![identity],
        )
        .map_err(database_error)?;
    Ok(())
}

pub(super) fn retire_owned_spools(
    transaction: &Transaction<'_>,
    owner: SpoolOwner<'_>,
) -> Result<(), ScanError> {
    let (predicate, identity) = owner_selection(owner);
    transaction
        .execute(
            &format!(
                "UPDATE library_metadata_inventory_spools SET state = 'retired'
                  WHERE state <> 'retired' AND {predicate}"
            ),
            params![identity],
        )
        .map_err(database_error)?;
    Ok(())
}

fn owner_selection(owner: SpoolOwner<'_>) -> (&'static str, rusqlite::types::Value) {
    match owner {
        SpoolOwner::Run(run_id) => (
            "run_id = ?1",
            rusqlite::types::Value::from(run_id.to_owned()),
        ),
        SpoolOwner::Root(root_id) => ("root_id = ?1", root_id.to_owned().into()),
        SpoolOwner::Change(change_id) => ("authority_change_id = ?1", change_id.into()),
        SpoolOwner::LegacyTerminalRuns => (
            "run_id IN (
               SELECT id FROM library_metadata_inventory_runs
               WHERE status IN ('failed', 'cancelled', 'superseded')
             ) AND ?1 IS NULL",
            rusqlite::types::Value::Null,
        ),
    }
}
