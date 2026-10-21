use rusqlite::params;

use crate::domain::{
    LeasedLibraryChange, LibraryChangeIntent, LibraryChangeLane, LibraryChangeQueuePolicy,
    MetadataInventoryRunStatus, ScanError,
};

use super::{
    SqliteCatalog, database_error, load_recovery_authority, metadata_inventory_candidate_key,
    require_run, sqlite_integer, upsert_metadata_inventory_candidate_owner, validate_active_root,
    validate_metadata_inventory_candidate_intent,
};

impl SqliteCatalog {
    pub(crate) fn publish_indexed_recovery_candidates(
        &mut self,
        lease: &LeasedLibraryChange,
        run_id: &str,
        intents: &[LibraryChangeIntent],
        now_unix_ms: i64,
        policy: LibraryChangeQueuePolicy,
    ) -> Result<u32, ScanError> {
        if intents.is_empty() {
            return Ok(0);
        }
        if intents.len() > 128 {
            return Err(invalid_authority());
        }
        for intent in intents {
            validate_metadata_inventory_candidate_intent(intent, &intent.relative_path, None)?;
        }
        let transaction = self.begin_write_in_lane(LibraryChangeLane::Recovery)?;
        let run = require_run(&transaction, run_id)?;
        let authority = load_recovery_authority(&transaction, lease.change.id)?
            .ok_or_else(invalid_authority)?;
        if !matches!(
            run.status,
            MetadataInventoryRunStatus::Running | MetadataInventoryRunStatus::Comparing
        ) || authority.retired_unix_ms.is_some()
            || authority.run_id != run_id
            || authority.root_id != run.request.root_id
            || authority.root_generation != run.request.root_generation
        {
            return Err(invalid_authority());
        }
        validate_active_root(&transaction, &run.request)?;
        let Some((_, ids)) =
            super::super::change_queue::enqueue_metadata_inventory_candidates_in_transaction(
                &transaction,
                lease,
                intents,
                now_unix_ms,
                policy,
            )?
        else {
            return Ok(0);
        };
        let mut new_owners = 0;
        for (intent, id) in intents.iter().zip(ids) {
            // Separate from inventory comparison receipts; hints never advance its cursors.
            let key = metadata_inventory_candidate_key("search-index", &intent.relative_path, None);
            let exists: bool = transaction
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM library_metadata_inventory_candidate_owners
                 WHERE run_id = ?1 AND candidate_key = ?2)",
                    params![run_id, key],
                    |row| row.get(0),
                )
                .map_err(database_error)?;
            upsert_metadata_inventory_candidate_owner(
                &transaction,
                run_id,
                &key,
                id,
                "present",
                &intent.relative_path,
                None,
                now_unix_ms,
            )?;
            new_owners += u32::from(!exists);
        }
        transaction.execute(
            "UPDATE library_metadata_inventory_runs SET candidate_count = candidate_count + ?2 WHERE id = ?1",
            params![run_id, sqlite_integer(u64::from(new_owners), "index candidate count")?],
        ).map_err(database_error)?;
        transaction.commit().map_err(database_error)?;
        Ok(intents.len() as u32)
    }
}

fn invalid_authority() -> ScanError {
    ScanError::new(
        "search_index_recovery_authority_stale",
        "Index hints require the current root recovery authority",
    )
}
