use std::time::Duration;
#[cfg(debug_assertions)]
use std::{io::Write, time::Instant};

use rusqlite::{Connection, TransactionBehavior, params};

use crate::domain::{
    LibraryChangeEnqueueReport, LibraryChangeIntent, LibraryChangeIntentKind, LibraryChangeLane,
    LibraryChangeOrigin, LibraryChangeQueuePolicy, LibraryChangeScope, LibraryRootGeneration,
    ScanError,
};

use super::super::{
    SQLITE_BUSY_TIMEOUT, SqliteCatalog, database_error, load_catalog_revision, sqlite_integer,
    sqlite_unsigned,
};
use super::{EnqueueContext, enqueue_one, validate_policy};

const PAGE_SIZE: usize = 32;

struct RestoreBusyTimeout<'a>(&'a mut Connection);

impl Drop for RestoreBusyTimeout<'_> {
    fn drop(&mut self) {
        let _ = self.0.busy_timeout(SQLITE_BUSY_TIMEOUT);
    }
}

const CANDIDATES: &str = "SELECT recheck.root_id, recheck.root_generation,
    recheck.relative_path, recheck.asset_id,
    EXISTS (SELECT 1 FROM asset_locations AS location
        WHERE location.scan_id = library.active_scan_id AND location.root_id = library.id
          AND location.location_id = recheck.location_id AND location.asset_id = recheck.asset_id
          AND location.width = 0 AND location.height = 0 AND location.preview_status = 'failed'
          AND location.preview_issue_code IN ('image_format_unsupported', 'media_type_unsupported'))
    FROM library_media_admission_rechecks AS recheck
    JOIN library_roots AS library ON library.id = recheck.root_id
    JOIN library_change_root_state AS root ON root.root_id = recheck.root_id
      AND root.is_active = 1 AND root.generation = recheck.root_generation
    LEFT JOIN library_change_queue AS work ON work.id = recheck.change_id
    WHERE recheck.state <> 'rejected'
      AND (work.id IS NULL OR work.status IN ('completed', 'superseded'))
      AND library.active_scan_id IS NOT NULL
      AND NOT EXISTS (SELECT 1 FROM scan_runs AS scan WHERE scan.root_id = library.id
        AND scan.status IN ('running', 'paused'))
    ORDER BY recheck.root_id, recheck.relative_path LIMIT 32";

impl SqliteCatalog {
    pub(crate) fn enqueue_media_admission_rechecks(
        &mut self,
        now_unix_ms: i64,
        policy: LibraryChangeQueuePolicy,
    ) -> Result<u32, ScanError> {
        validate_policy(policy)?;
        #[cfg(debug_assertions)]
        let started = Instant::now();
        if !self
            .connection
            .prepare_cached(CANDIDATES)
            .map_err(database_error)?
            .exists([])
            .map_err(database_error)?
        {
            return Ok(0);
        }
        // This admission runs on the polling thread. Yield to active writers so P2 repair
        // cannot hold up live observation, gallery refresh or completed-worker delivery.
        let Some(_permit) = self
            .write_admission
            .try_acquire(LibraryChangeLane::Recovery)
        else {
            return Ok(0);
        };
        let restore = RestoreBusyTimeout(&mut self.connection);
        restore
            .0
            .busy_timeout(Duration::ZERO)
            .map_err(database_error)?;
        let transaction = match restore
            .0
            .transaction_with_behavior(TransactionBehavior::Immediate)
        {
            Ok(transaction) => transaction,
            Err(error) if error.sqlite_error_code() == Some(rusqlite::ErrorCode::DatabaseBusy) => {
                return Ok(0);
            }
            Err(error) => return Err(database_error(error)),
        };
        let candidates = {
            let mut statement = transaction
                .prepare_cached(CANDIDATES)
                .map_err(database_error)?;
            let rows = statement
                .query_map([], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, bool>(4)?,
                    ))
                })
                .map_err(database_error)?;
            rows.collect::<Result<Vec<_>, _>>()
                .map_err(database_error)?
        };
        #[cfg(debug_assertions)]
        let prepare_ms = started.elapsed().as_millis();
        let revision = load_catalog_revision(&transaction)?;
        let mut admitted = 0;
        for (root_id, generation, relative_path, asset_id, still_failed) in
            candidates.into_iter().take(PAGE_SIZE)
        {
            if !still_failed {
                transaction
                    .execute(
                        "DELETE FROM library_media_admission_rechecks
                     WHERE root_id = ?1 AND relative_path = ?2 AND asset_id = ?3",
                        params![root_id, relative_path, asset_id],
                    )
                    .map_err(database_error)?;
                continue;
            }
            let root_generation =
                LibraryRootGeneration::new(sqlite_unsigned(generation, "root generation")?)
                    .ok_or_else(|| {
                        ScanError::new(
                            "media_recheck_root_generation_invalid",
                            "A recheck requires a current root generation",
                        )
                    })?;
            let intent = LibraryChangeIntent {
                root_id: root_id.clone(),
                root_generation,
                kind: LibraryChangeIntentKind::Reconcile,
                scope: LibraryChangeScope::Path,
                relative_path: relative_path.clone(),
                previous_relative_path: None,
                origin: LibraryChangeOrigin::ConsistencyAudit,
                first_observed_unix_ms: now_unix_ms,
                most_recent_observed_unix_ms: now_unix_ms,
                first_sequence: 1,
                most_recent_sequence: 1,
                coalesced_observation_count: 1,
            };
            let enqueued = enqueue_one(
                &transaction,
                &intent,
                EnqueueContext {
                    enqueued_unix_ms: now_unix_ms,
                    catalog_revision: revision,
                    policy,
                    evidence: None,
                    protected_change_ids: &[],
                    allow_scope_degradation: false,
                },
                &mut LibraryChangeEnqueueReport::default(),
            );
            let change_id = match enqueued {
                Ok(change_id) => change_id,
                Err(error)
                    if matches!(
                        error.code.as_str(),
                        "change_queue_backpressure" | "metadata_inventory_backpressure"
                    ) =>
                {
                    break;
                }
                Err(error) => return Err(error),
            };
            transaction
                .execute(
                    "UPDATE library_media_admission_rechecks SET state = 'queued', change_id = ?3
                 WHERE root_id = ?1 AND relative_path = ?2",
                    params![
                        root_id,
                        relative_path,
                        sqlite_integer(change_id.value(), "media recheck change")?
                    ],
                )
                .map_err(database_error)?;
            admitted += 1;
        }
        transaction.commit().map_err(database_error)?;
        #[cfg(debug_assertions)]
        if started.elapsed() >= Duration::from_millis(100) {
            let _ = writeln!(
                std::io::stderr().lock(),
                "[Ame media recheck] admitted={admitted} prepare_ms={prepare_ms} total_ms={}",
                started.elapsed().as_millis()
            );
        }
        Ok(admitted)
    }
}
