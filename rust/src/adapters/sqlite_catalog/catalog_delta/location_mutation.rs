use std::collections::{HashMap, HashSet};

use rusqlite::{Transaction, params};

use crate::domain::{AssetLocationView, CatalogDeltaMutation, PreviewStatus, ScanError};

use super::super::gallery::retained_membership::capture_unchanged_display;
use super::super::source_reconciliation::{
    IdentityGenerationBatchState, IdentityGenerationExpectation,
};
use super::super::{database_error, persist_location, sqlite_unsigned};

pub(in super::super) fn apply(
    transaction: &Transaction<'_>,
    active_scan_id: &str,
    running_scan_id: Option<&str>,
    root_id: &str,
    mutation: &CatalogDeltaMutation,
    identity_generations: &mut HashMap<(String, String), IdentityGenerationBatchState>,
    staging_generations: &mut HashMap<(String, String), IdentityGenerationBatchState>,
) -> Result<(), ScanError> {
    let retained = mutation
        .upsert_location
        .as_ref()
        .map(|location| capture_unchanged_display(transaction, location))
        .transpose()?
        .flatten();
    let mut removals = mutation
        .remove_location_ids
        .iter()
        .map(String::as_str)
        .collect::<HashSet<_>>();
    if let Some(location) = &mutation.upsert_location {
        removals.insert(&location.location_id);
    }
    for location_id in removals {
        super::location_removal::remove(
            transaction,
            active_scan_id,
            running_scan_id,
            root_id,
            location_id,
        )?;
    }
    let Some(location) = &mutation.upsert_location else {
        return Ok(());
    };
    let persisted = persist_location(
        transaction,
        active_scan_id,
        root_id,
        location,
        IdentityGenerationExpectation::ResolveCurrent,
        identity_generations,
    )?;
    if let Some(retained) = retained {
        retained.bind(transaction, persisted.source_generation)?;
    }
    super::super::organization::admission::publish_generation(
        transaction,
        persisted.source_generation,
    )?;
    super::super::organization::records::publish_generation(
        transaction,
        persisted.source_generation,
    )?;
    if let Some(running_scan_id) = running_scan_id {
        let mut mirrored = location.clone();
        mirrored.source_generation =
            sqlite_unsigned(persisted.source_generation, "mirrored source generation")?;
        persist_location(
            transaction,
            running_scan_id,
            root_id,
            &mirrored,
            IdentityGenerationExpectation::MirrorCurrentActive,
            staging_generations,
        )?;
    }
    retain_ready_preview(transaction, location)
}

fn retain_ready_preview(
    transaction: &Transaction<'_>,
    location: &AssetLocationView,
) -> Result<(), ScanError> {
    if !matches!(location.preview_status, PreviewStatus::Ready) || location.preview_path.is_empty()
    {
        return Ok(());
    }
    transaction
        .execute(
            "INSERT OR IGNORE INTO preview_artifact_locations(artifact_key, location_id)
         SELECT artifact_key, ?1 FROM preview_artifacts
         WHERE artifact_path = ?2 AND lifecycle_state = 'ready'",
            params![location.location_id, location.preview_path],
        )
        .map_err(database_error)?;
    let owner = transaction
        .query_row(
            "SELECT EXISTS(
           SELECT 1 FROM preview_artifact_locations AS owners
           JOIN preview_artifacts AS artifacts ON artifacts.artifact_key = owners.artifact_key
           WHERE owners.location_id = ?1 AND artifacts.artifact_path = ?2
             AND artifacts.lifecycle_state = 'ready')",
            params![location.location_id, location.preview_path],
            |row| row.get::<_, bool>(0),
        )
        .map_err(database_error)?;
    if !owner {
        return Err(ScanError::new(
            "catalog_delta_preview_owner_missing",
            "A retained ready preview no longer has a compatible artifact owner",
        ));
    }
    Ok(())
}
