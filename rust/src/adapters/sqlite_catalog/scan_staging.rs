use std::collections::HashMap;

use rusqlite::Transaction;

use crate::domain::ScanError;

use super::{
    IdentityGenerationExpectation, PendingLocation, SqliteCatalog, database_error,
    persist_location, scan_resume_inventory,
};

mod retirement;

pub(super) use retirement::{StagingRetirement, discard_scan_staging};

pub(super) fn flush_pending_locations(catalog: &mut SqliteCatalog) -> Result<(), ScanError> {
    if catalog.pending_locations.is_empty() {
        return Ok(());
    }
    let pending = catalog.pending_locations.clone();
    let transaction = catalog.begin_write()?;
    persist_pending_locations(&transaction, &pending)?;
    transaction.commit().map_err(database_error)?;
    catalog.pending_locations.clear();
    Ok(())
}

pub(super) fn persist_pending_locations(
    transaction: &Transaction<'_>,
    pending: &[PendingLocation],
) -> Result<(), ScanError> {
    let mut active_projection_changed = false;
    let mut identity_generation_batch = HashMap::new();
    for item in pending {
        active_projection_changed |= persist_location(
            transaction,
            &item.scan_id,
            &item.root_id,
            &item.location,
            IdentityGenerationExpectation::Captured(item.identity_group_baseline.clone()),
            &mut identity_generation_batch,
        )?
        .active_projection_changed;
        scan_resume_inventory::accept_location(
            transaction,
            &item.scan_id,
            &item.location.location_id,
        )?;
    }
    if active_projection_changed {
        let updated = transaction
            .execute("UPDATE catalog_state SET revision = revision + 1", [])
            .map_err(database_error)?;
        if updated != 1 {
            return Err(ScanError::new(
                "catalog_revision_unavailable",
                "The catalog revision state is missing or invalid",
            ));
        }
    }
    Ok(())
}
