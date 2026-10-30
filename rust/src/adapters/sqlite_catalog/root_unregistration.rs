use rusqlite::Transaction;

use crate::domain::ScanError;

use super::{
    change_queue, database_error, detach_preview_references_for_root_locations, persistent_journal,
    retire_root_change_queue, spool_retirement, unix_time_ms,
};

pub(super) fn unregister_root(
    transaction: &Transaction<'_>,
    root_id: &str,
) -> Result<bool, ScanError> {
    let root_exists = transaction
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM library_roots WHERE id = ?1)",
            [root_id],
            |row| row.get::<_, bool>(0),
        )
        .map_err(database_error)?;
    if !root_exists {
        return Ok(false);
    }
    super::organization::lifecycle::forget_source(transaction, root_id)?;
    retire_root_change_queue(transaction, root_id, unix_time_ms())?;
    spool_retirement::retire_owned_spools(
        transaction,
        spool_retirement::SpoolOwner::Root(root_id),
    )?;
    persistent_journal::remove_root_persistent_journal_state(transaction, root_id)?;
    for table in [
        "scan_directory_frontier",
        "scan_directory_entries",
        "scan_issues",
    ] {
        transaction
            .execute(
                &format!(
                    "DELETE FROM {table}
                     WHERE scan_id IN (
                       SELECT id FROM scan_runs WHERE root_id = ?1
                     )"
                ),
                [root_id],
            )
            .map_err(database_error)?;
    }
    prepare_root_unregister_scope(transaction, root_id)?;
    detach_preview_references_for_root_locations(transaction, root_id, None)?;
    mark_root_unregister_preview_artifacts_stale(transaction)?;
    transaction
        .execute("DELETE FROM asset_locations WHERE root_id = ?1", [root_id])
        .map_err(database_error)?;
    transaction
        .execute("DELETE FROM scan_runs WHERE root_id = ?1", [root_id])
        .map_err(database_error)?;
    let removed = transaction
        .execute("DELETE FROM library_roots WHERE id = ?1", [root_id])
        .map_err(database_error)?;
    if removed != 1 {
        return Err(ScanError::new(
            "catalog_root_unregister_failed",
            "The registered library root could not be removed",
        ));
    }
    change_queue::root_retirement::retire_removed_root_recovery_authorities(transaction, root_id)?;
    delete_root_unregister_orphan_assets(transaction)?;
    super::classification::review::lifecycle::forget_source(transaction, root_id)?;
    clear_root_unregister_scope(transaction)?;
    Ok(true)
}

fn prepare_root_unregister_scope(
    transaction: &Transaction<'_>,
    root_id: &str,
) -> Result<(), ScanError> {
    transaction
        .execute_batch(
            "CREATE TEMP TABLE IF NOT EXISTS ame_root_unregister_assets (
               asset_id TEXT PRIMARY KEY
             ) WITHOUT ROWID;
             CREATE TEMP TABLE IF NOT EXISTS ame_root_unregister_preview_artifacts (
               artifact_key TEXT PRIMARY KEY
             ) WITHOUT ROWID;
             DELETE FROM temp.ame_root_unregister_assets;
             DELETE FROM temp.ame_root_unregister_preview_artifacts;",
        )
        .map_err(database_error)?;
    transaction
        .execute(
            "INSERT OR IGNORE INTO temp.ame_root_unregister_assets(asset_id)
             SELECT asset_id
             FROM asset_locations
             WHERE root_id = ?1",
            [root_id],
        )
        .map_err(database_error)?;
    transaction
        .execute(
            "INSERT OR IGNORE INTO temp.ame_root_unregister_preview_artifacts(artifact_key)
             SELECT owners.artifact_key
             FROM preview_artifact_locations AS owners
             JOIN asset_locations AS locations
               ON locations.location_id = owners.location_id
             WHERE locations.root_id = ?1",
            [root_id],
        )
        .map_err(database_error)?;
    Ok(())
}

fn mark_root_unregister_preview_artifacts_stale(
    transaction: &Transaction<'_>,
) -> Result<(), ScanError> {
    transaction
        .execute(
            "UPDATE preview_artifacts
             SET lifecycle_state = 'stale'
             WHERE lifecycle_state = 'ready'
               AND artifact_key IN (
                 SELECT artifact_key
                 FROM temp.ame_root_unregister_preview_artifacts
               )
               AND NOT EXISTS (
                 SELECT 1 FROM preview_artifact_locations AS owners
                 WHERE owners.artifact_key = preview_artifacts.artifact_key
               )
               AND NOT EXISTS (
                 SELECT 1 FROM library_change_catch_up_handoffs AS handoffs
                 WHERE handoffs.preview_status = 'ready'
                   AND handoffs.preview_path = preview_artifacts.artifact_path
               )
               AND NOT EXISTS (
                 SELECT 1 FROM library_change_scan_handoff_items AS handoffs
                 WHERE handoffs.preview_status = 'ready'
                   AND handoffs.preview_path = preview_artifacts.artifact_path
               )",
            [],
        )
        .map_err(database_error)?;
    Ok(())
}

fn delete_root_unregister_orphan_assets(transaction: &Transaction<'_>) -> Result<(), ScanError> {
    transaction
        .execute(
            "DELETE FROM assets
             WHERE id IN (
               SELECT asset_id FROM temp.ame_root_unregister_assets
             )
               AND NOT EXISTS (
                 SELECT 1 FROM asset_locations WHERE asset_locations.asset_id = assets.id
               )
               AND NOT EXISTS (
                 SELECT 1 FROM library_change_catch_up_handoffs AS handoffs
                 WHERE handoffs.asset_id = assets.id
               )
               AND NOT EXISTS (
                 SELECT 1 FROM library_change_scan_handoff_items AS handoffs
                 WHERE handoffs.asset_id = assets.id
               )",
            [],
        )
        .map_err(database_error)?;
    Ok(())
}

fn clear_root_unregister_scope(transaction: &Transaction<'_>) -> Result<(), ScanError> {
    transaction
        .execute_batch(
            "DELETE FROM temp.ame_root_unregister_assets;
             DELETE FROM temp.ame_root_unregister_preview_artifacts;",
        )
        .map_err(database_error)
}
