use rusqlite::{Connection, OptionalExtension, Transaction, params};

use crate::domain::{
    LeasedLibraryChange, LibraryChangeLeaseIdentity, LibraryChangeLeaseUpdateOutcome,
    MetadataInventoryRun, MetadataInventoryRunRequest, MetadataInventoryRunStatus, ScanError,
};

use super::super::change_queue::classify_lease_update;
use super::{
    SqliteCatalog, database_error, load_metadata_inventory_authority_owner_for_run, require_run,
    scope_parts, sqlite_integer, validate_active_root,
};

#[cfg(test)]
mod test_support;

#[derive(Clone, Debug)]
pub(crate) struct MetadataInventorySpoolExecution {
    request: MetadataInventoryRunRequest,
    lease: LibraryChangeLeaseIdentity,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum SpoolState {
    Enumerating,
    Ready,
}

impl MetadataInventorySpoolExecution {
    pub(super) fn capture(run: &MetadataInventoryRun, authority: &LeasedLibraryChange) -> Self {
        Self {
            request: run.request.clone(),
            lease: LibraryChangeLeaseIdentity {
                change_id: authority.change.id,
                lease_generation: authority.lease_generation,
            },
        }
    }

    pub(crate) fn run_id(&self) -> &str {
        &self.request.run_id
    }

    pub(super) fn require_current_run(
        &self,
        transaction: &Transaction<'_>,
    ) -> Result<MetadataInventoryRun, ScanError> {
        let current = require_run(transaction, self.run_id())?;
        if current.status != MetadataInventoryRunStatus::Running || current.request != self.request
        {
            return Err(stale_execution());
        }
        validate_active_root(transaction, &current.request)?;
        if classify_lease_update(
            transaction,
            self.lease.change_id,
            self.lease.lease_generation,
            None,
        )? != LibraryChangeLeaseUpdateOutcome::Applied
        {
            return Err(stale_execution());
        }
        let owner = load_metadata_inventory_authority_owner_for_run(
            transaction,
            self.run_id(),
            &current.request.root_id,
            current.request.root_generation,
        )?;
        if owner
            != Some(sqlite_integer(
                self.lease.change_id.value(),
                "inventory source change id",
            )?)
        {
            return Err(stale_execution());
        }
        Ok(current)
    }

    pub(super) fn require_spool(
        &self,
        transaction: &Transaction<'_>,
    ) -> Result<(MetadataInventoryRun, SpoolState), ScanError> {
        let run = self.require_current_run(transaction)?;
        let (scope_kind, scope_path) = scope_parts(&self.request.scope);
        let state: Option<String> = transaction
            .query_row(
                "SELECT state FROM library_metadata_inventory_spools
             WHERE run_id = ?1 AND authority_change_id = ?2
               AND root_id = ?3 AND root_generation = ?4
               AND scope_kind = ?5 AND scope_relative_path = ?6",
                params![
                    self.run_id(),
                    sqlite_integer(self.lease.change_id.value(), "inventory source change id")?,
                    self.request.root_id,
                    sqlite_integer(
                        self.request.root_generation.value(),
                        "inventory source root generation"
                    )?,
                    scope_kind,
                    scope_path
                ],
                |row| row.get(0),
            )
            .optional()
            .map_err(database_error)?;
        let state = match state.as_deref() {
            Some("enumerating") => SpoolState::Enumerating,
            Some("ready") => SpoolState::Ready,
            _ => return Err(stale_execution()),
        };
        Ok((run, state))
    }
}

impl SqliteCatalog {
    pub(crate) fn rebind_metadata_inventory_spool_execution(
        &self,
        previous: &MetadataInventorySpoolExecution,
        authority: &LeasedLibraryChange,
    ) -> Result<MetadataInventorySpoolExecution, ScanError> {
        if previous.lease.change_id != authority.change.id {
            return Err(stale_execution());
        }
        let mut current = previous.clone();
        current.lease.lease_generation = authority.lease_generation;
        with_read_spool(self, &current, |_, _, _| Ok(()))?;
        Ok(current)
    }

    pub(crate) fn validate_metadata_inventory_spool_execution(
        &self,
        execution: &MetadataInventorySpoolExecution,
    ) -> Result<(), ScanError> {
        with_read_spool(self, execution, |_, _, _| Ok(()))
    }
}

pub(super) fn with_read_spool<T>(
    catalog: &SqliteCatalog,
    execution: &MetadataInventorySpoolExecution,
    read: impl FnOnce(&Connection, MetadataInventoryRun, SpoolState) -> Result<T, ScanError>,
) -> Result<T, ScanError> {
    if !catalog.connection.is_autocommit() {
        return Err(ScanError::new(
            "metadata_inventory_source_transaction_active",
            "Source reads require their own consistent catalog snapshot",
        ));
    }
    let transaction = catalog
        .connection
        .unchecked_transaction()
        .map_err(database_error)?;
    let (run, state) = execution.require_spool(&transaction)?;
    #[cfg(test)]
    test_support::after_authority_check();
    let value = read(&transaction, run, state)?;
    transaction.commit().map_err(database_error)?;
    Ok(value)
}

fn stale_execution() -> ScanError {
    ScanError::new(
        "metadata_inventory_spool_authority_mismatch",
        "The source spool execution no longer owns the current recovery run and lease",
    )
}
