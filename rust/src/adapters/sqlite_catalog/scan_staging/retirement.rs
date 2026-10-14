use rusqlite::{OptionalExtension, Transaction};

use crate::domain::ScanError;

use super::super::database_error;

const RETIREMENT_CACHE_KIB: i64 = 64 * 1024;
const FOREGROUND_RETIREMENT_BATCH: usize = 128;

pub(in super::super) enum StagingRetirement {
    CompleteScan,
    UnobservedResumeLocations,
    RetiredForegroundBatch,
}

pub(in super::super) fn retire_scan_staging(
    transaction: &Transaction<'_>,
    scan_id: &str,
) -> Result<(), ScanError> {
    let owner: String = transaction
        .query_row(
            "SELECT scan_owner FROM scan_runs WHERE id = ?1",
            [scan_id],
            |row| row.get(0),
        )
        .map_err(database_error)?;
    let scope = if owner == "foreground" {
        StagingRetirement::RetiredForegroundBatch
    } else {
        StagingRetirement::CompleteScan
    };
    discard_scan_staging(transaction, scan_id, scope).map(|_| ())
}

pub(in super::super) fn discard_next_retired_batch(
    transaction: &Transaction<'_>,
) -> Result<usize, ScanError> {
    let scan_id: Option<String> = transaction
        .query_row(
            "SELECT scan.id FROM scan_runs AS scan
         WHERE scan.scan_owner = 'foreground'
           AND scan.status IN ('cancelled', 'failed', 'stale', 'superseded')
           AND NOT EXISTS (SELECT 1 FROM library_roots WHERE active_scan_id = scan.id)
           AND EXISTS (SELECT 1 FROM asset_locations WHERE scan_id = scan.id)
         LIMIT 1",
            [],
            |row| row.get(0),
        )
        .optional()
        .map_err(database_error)?;
    let Some(scan_id) = scan_id else { return Ok(0) };
    discard_scan_staging(
        transaction,
        &scan_id,
        StagingRetirement::RetiredForegroundBatch,
    )
}

pub(in super::super) fn discard_scan_staging(
    transaction: &Transaction<'_>,
    scan_id: &str,
    scope: StagingRetirement,
) -> Result<usize, ScanError> {
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
    let retired = retired?;
    restored?;
    Ok(retired)
}

fn discard_rows(
    transaction: &Transaction<'_>,
    scan_id: &str,
    scope: StagingRetirement,
) -> Result<usize, ScanError> {
    transaction.execute_batch(
        "CREATE TEMP TABLE IF NOT EXISTS ame_scan_discard_assets(asset_id TEXT PRIMARY KEY) WITHOUT ROWID;
         DELETE FROM temp.ame_scan_discard_assets;",
    ).map_err(database_error)?;
    let predicate = match scope {
        StagingRetirement::CompleteScan => "scan_id = ?1".to_owned(),
        StagingRetirement::UnobservedResumeLocations => "scan_id = ?1 AND location_id IN (
                SELECT location_id FROM scan_resume_pending_locations WHERE scan_id = ?1
            )"
        .to_owned(),
        StagingRetirement::RetiredForegroundBatch => format!(
            "scan_id = ?1 AND rowid IN (
                SELECT rowid FROM asset_locations WHERE scan_id = ?1 ORDER BY location_id
                LIMIT {FOREGROUND_RETIREMENT_BATCH}
             ) AND EXISTS (
                SELECT 1 FROM scan_runs AS scan WHERE scan.id = ?1
                  AND scan.scan_owner = 'foreground'
                  AND scan.status IN ('cancelled', 'failed', 'stale', 'superseded')
                  AND NOT EXISTS (SELECT 1 FROM library_roots WHERE active_scan_id = scan.id)
             )"
        ),
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
    let discarded = transaction
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
    Ok(discarded)
}
