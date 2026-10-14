use rusqlite::Transaction;

use crate::domain::ScanError;

use super::super::database_error;

const RETIREMENT_CACHE_KIB: i64 = 64 * 1024;

pub(in super::super) enum StagingRetirement {
    CompleteScan,
    UnobservedResumeLocations,
}

pub(in super::super) fn discard_scan_staging(
    transaction: &Transaction<'_>,
    scan_id: &str,
    scope: StagingRetirement,
) -> Result<(), ScanError> {
    let previous_cache_size: i64 = transaction
        .pragma_query_value(None, "cache_size", |row| row.get(0))
        .map_err(database_error)?;
    // Retiring a large scan touches its gallery indexes in the same atomic write. The default
    // pager budget repeatedly spills those pages; only this admitted writer borrows the larger
    // budget, and both success and SQL failure restore the connection's previous setting.
    transaction
        .pragma_update(None, "cache_size", -RETIREMENT_CACHE_KIB)
        .map_err(database_error)?;
    let retired = discard_rows(transaction, scan_id, scope);
    let restored = transaction
        .pragma_update(None, "cache_size", previous_cache_size)
        .map_err(database_error);
    retired.and(restored)
}

fn discard_rows(
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
