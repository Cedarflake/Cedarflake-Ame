use std::collections::HashMap;

use rusqlite::Transaction;

use crate::adapters::revalidate_file_state;
use crate::domain::{AssetLocationView, ExpectedFileState, ScanError};

use super::{
    IdentityGenerationExpectation, LOCATION_STAGE_BATCH, PendingLocation, SqliteCatalog,
    database_error, load_staging_identity_group_state, persist_location, revisions_conflict,
    scan_resume_inventory, source_revision_token, sqlite_integer,
};

mod retirement;

pub(super) use retirement::{
    StagingRetirement, discard_next_retired_batch, discard_scan_staging, retire_scan_staging,
};

pub(super) fn stage_location(
    catalog: &mut SqliteCatalog,
    scan_id: &str,
    root_id: &str,
    location: &AssetLocationView,
) -> Result<(), ScanError> {
    if location.root_id != root_id {
        return Err(ScanError::new(
            "catalog_root_mismatch",
            "The staged location does not belong to the scan root",
        ));
    }
    let location_file_size = sqlite_integer(location.file_size, "file size")?;
    let location_revision_token = location
        .source_revision
        .as_ref()
        .map(source_revision_token)
        .transpose()?;
    // A full batch can share the next progress commit. Flush only before admitting another item.
    if catalog.pending_locations.len() >= LOCATION_STAGE_BATCH {
        flush_pending_locations(catalog)?;
    }
    let identity_group_baseline = location
        .file_identity
        .as_ref()
        .map(|identity| load_staging_identity_group_state(&catalog.connection, identity, scan_id))
        .transpose()?
        .flatten();
    if identity_group_baseline.as_ref().is_some_and(|state| {
        state.file_size != location_file_size
            || state.modified_unix_ms != location.modified_unix_ms
            || revisions_conflict(
                state.source_revision_token.as_deref(),
                location_revision_token.as_deref(),
            )
    }) {
        revalidate_file_state(&ExpectedFileState {
            absolute_path: location.absolute_path.clone(),
            file_size: location.file_size,
            modified_unix_ms: location.modified_unix_ms,
            file_identity: location.file_identity.clone(),
            source_revision: location.source_revision.clone(),
        })
        .map_err(|issue| ScanError::new(issue.code, issue.message))?;
    }
    catalog.pending_locations.push(PendingLocation {
        scan_id: scan_id.to_owned(),
        root_id: root_id.to_owned(),
        location: location.clone(),
        identity_group_baseline,
    });
    Ok(())
}

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
