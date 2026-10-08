use std::collections::HashMap;

use rusqlite::Transaction;

use crate::domain::ScanError;

use super::{
    IdentityGenerationExpectation, SqliteCatalog, database_error, persist_location,
    scan_resume_inventory,
};

pub(super) fn flush_pending_locations(catalog: &mut SqliteCatalog) -> Result<(), ScanError> {
    if catalog.pending_locations.is_empty() {
        return Ok(());
    }
    let pending = catalog.pending_locations.clone();
    let transaction = catalog.begin_write()?;
    let mut active_projection_changed = false;
    let mut identity_generation_batch = HashMap::new();
    for item in &pending {
        active_projection_changed |= persist_location(
            &transaction,
            &item.scan_id,
            &item.root_id,
            &item.location,
            IdentityGenerationExpectation::Captured(item.identity_group_baseline.clone()),
            &mut identity_generation_batch,
        )?
        .active_projection_changed;
        scan_resume_inventory::accept_location(
            &transaction,
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
    transaction.commit().map_err(database_error)?;
    catalog.pending_locations.clear();
    Ok(())
}

pub(super) enum StagingRetirement {
    CompleteScan,
    UnobservedResumeLocations,
}

pub(super) fn discard_scan_staging(
    transaction: &Transaction<'_>,
    scan_id: &str,
    scope: StagingRetirement,
) -> Result<(), ScanError> {
    transaction.execute_batch(
        "CREATE TEMP TABLE IF NOT EXISTS ame_scan_discard_assets(asset_id TEXT PRIMARY KEY) WITHOUT ROWID;
         DELETE FROM temp.ame_scan_discard_assets;",
    ).map_err(database_error)?;
    let predicate = match scope {
        StagingRetirement::CompleteScan => "scan_id = ?1",
        StagingRetirement::UnobservedResumeLocations => {
            "scan_id = ?1 AND location_id IN (
                SELECT location_id FROM scan_resume_pending_locations WHERE scan_id = ?1
            )"
        }
    };
    transaction
        .execute(
            &format!(
                "INSERT OR IGNORE INTO temp.ame_scan_discard_assets(asset_id)
                SELECT asset_id FROM asset_locations WHERE {predicate}"
            ),
            [scan_id],
        )
        .map_err(database_error)?;
    transaction
        .execute(
            &format!("DELETE FROM asset_locations WHERE {predicate}"),
            [scan_id],
        )
        .map_err(database_error)?;
    transaction.execute(
        "DELETE FROM assets WHERE id IN (SELECT asset_id FROM temp.ame_scan_discard_assets)
           AND NOT EXISTS(SELECT 1 FROM asset_locations WHERE asset_id = assets.id)
           AND NOT EXISTS(SELECT 1 FROM library_change_catch_up_handoffs WHERE asset_id = assets.id)
           AND NOT EXISTS(SELECT 1 FROM library_change_scan_handoff_items WHERE asset_id = assets.id)", [],
    ).map_err(database_error)?;
    transaction
        .execute("DELETE FROM temp.ame_scan_discard_assets", [])
        .map_err(database_error)?;
    Ok(())
}
