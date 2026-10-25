use std::time::Duration;

use rusqlite::{Connection, TransactionBehavior, params};

use super::{
    LibraryChangeLane, SQLITE_BUSY_TIMEOUT, ScanError, SqliteCatalog, database_error, unix_time_ms,
};

pub(super) fn touch(
    catalog: &mut SqliteCatalog,
    artifacts: &[(String, String)],
) -> Result<u64, ScanError> {
    if artifacts.len() > 4_096 {
        return Err(ScanError::new(
            "preview_touch_batch_too_large",
            "Preview usage updates are limited to 4096 artifacts per batch",
        ));
    }
    if artifacts.is_empty() {
        return Ok(0);
    }
    let Some(_permit) = catalog
        .write_admission
        .try_acquire(LibraryChangeLane::Recovery)
    else {
        return Ok(0);
    };
    catalog
        .write_admission
        .checkpoint_policy
        .apply(&catalog.connection)?;
    // Approximate cache recency must not delay publishing an already loaded page.
    catalog
        .connection
        .busy_timeout(Duration::ZERO)
        .map_err(database_error)?;
    let result = record_usage(&mut catalog.connection, artifacts);
    catalog
        .connection
        .busy_timeout(SQLITE_BUSY_TIMEOUT)
        .map_err(database_error)?;
    match result {
        Err(error)
            if matches!(
                error.code.as_str(),
                "catalog_database_busy" | "catalog_database_locked"
            ) =>
        {
            Ok(0)
        }
        result => result,
    }
}

fn record_usage(
    connection: &mut Connection,
    artifacts: &[(String, String)],
) -> Result<u64, ScanError> {
    let now = unix_time_ms();
    let oldest_retained = now.saturating_sub(60_000);
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(database_error)?;
    let mut statement = transaction
        .prepare_cached(
            "UPDATE preview_artifacts
             SET last_used_unix_ms = ?3
             WHERE artifact_key IN (
                 SELECT artifact_key FROM preview_artifact_locations
                 WHERE location_id = ?1
             )
               AND artifact_path = ?2 AND last_used_unix_ms < ?4",
        )
        .map_err(database_error)?;
    let mut updated = 0_usize;
    for (location_id, artifact_path) in artifacts {
        updated = updated.saturating_add(
            statement
                .execute(params![location_id, artifact_path, now, oldest_retained])
                .map_err(database_error)?,
        );
    }
    drop(statement);
    transaction.commit().map_err(database_error)?;
    u64::try_from(updated).map_err(|_| {
        ScanError::new(
            "preview_touch_count_invalid",
            "The preview usage update count exceeds the supported range",
        )
    })
}
