use std::time::Duration;

use rusqlite::{Connection, TransactionBehavior, params};

use crate::domain::{
    LibraryChangeEnqueueReport, LibraryChangeIntent, LibraryChangeIntentKind, LibraryChangeLane,
    LibraryChangeOrigin, LibraryChangeQueuePolicy, LibraryChangeScope, LibraryRootGeneration,
    ScanError,
};

use super::super::{
    SQLITE_BUSY_TIMEOUT, SqliteCatalog, database_error, load_catalog_revision, sqlite_unsigned,
};
use super::{EnqueueContext, enqueue_one, validate_policy};

const CANDIDATES: &str = "WITH candidates AS MATERIALIZED (
    SELECT location.root_id, root.generation, location.scan_id, location.relative_path
    FROM asset_locations AS location INDEXED BY asset_locations_pending_metadata
    JOIN library_roots AS library
      ON library.id = location.root_id AND library.active_scan_id = location.scan_id
    JOIN library_change_root_state AS root ON root.root_id = library.id AND root.is_active = 1
    WHERE location.metadata_engine_id = 'ame-invalidated-media-metadata'
      AND (location.root_id, location.scan_id, location.relative_path) > (?1, ?2, ?3)
    ORDER BY location.root_id, location.scan_id, location.relative_path LIMIT 32)
    SELECT root_id, generation, scan_id, relative_path,
      NOT EXISTS (SELECT 1 FROM scan_runs AS scan WHERE scan.root_id = candidates.root_id
        AND scan.status IN ('running', 'paused'))
      AND NOT EXISTS (SELECT 1 FROM library_change_queue AS work
        WHERE work.root_id = candidates.root_id AND work.root_generation = candidates.generation
          AND work.status IN ('pending', 'leased', 'retry_wait')
          AND (work.scope <> 'path' OR work.relative_path = candidates.relative_path))
    FROM candidates";

#[derive(Default)]
pub(in crate::adapters::sqlite_catalog) struct MetadataRecheckAdmission {
    cursor: (String, String, String),
}

struct Candidate {
    root_id: String,
    generation: i64,
    scan_id: String,
    relative_path: String,
    admissible: bool,
}

impl MetadataRecheckAdmission {
    fn advance(&mut self, candidates: &[Candidate]) {
        self.cursor = candidates.last().map_or_else(Default::default, |last| {
            (
                last.root_id.clone(),
                last.scan_id.clone(),
                last.relative_path.clone(),
            )
        });
    }
}

fn load_page(
    connection: &Connection,
    cursor: &(String, String, String),
) -> Result<Vec<Candidate>, ScanError> {
    let mut statement = connection
        .prepare_cached(CANDIDATES)
        .map_err(database_error)?;
    let rows = statement
        .query_map(params![cursor.0, cursor.1, cursor.2], |row| {
            Ok(Candidate {
                root_id: row.get(0)?,
                generation: row.get(1)?,
                scan_id: row.get(2)?,
                relative_path: row.get(3)?,
                admissible: row.get(4)?,
            })
        })
        .map_err(database_error)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(database_error)
}

struct RestoreBusyTimeout<'a>(&'a mut Connection);

impl Drop for RestoreBusyTimeout<'_> {
    fn drop(&mut self) {
        let _ = self.0.busy_timeout(SQLITE_BUSY_TIMEOUT);
    }
}

impl SqliteCatalog {
    pub(crate) fn enqueue_metadata_rechecks(
        &mut self,
        now_unix_ms: i64,
        policy: LibraryChangeQueuePolicy,
    ) -> Result<u32, ScanError> {
        validate_policy(policy)?;
        let cursor = self.metadata_recheck_admission.cursor.clone();
        let candidates = load_page(&self.connection, &cursor)?;
        self.metadata_recheck_admission.advance(&candidates);
        if !candidates.iter().any(|candidate| candidate.admissible) {
            return Ok(0);
        }
        let Some(_permit) = self
            .write_admission
            .try_acquire(LibraryChangeLane::Recovery)
        else {
            return Ok(0);
        };
        // Polling admits bounded work; source inspection belongs to the existing P2 worker.
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
        let candidates = load_page(&transaction, &cursor)?;
        self.metadata_recheck_admission.advance(&candidates);
        let revision = load_catalog_revision(&transaction)?;
        let mut admitted = 0;
        for candidate in candidates
            .into_iter()
            .filter(|candidate| candidate.admissible)
        {
            let root_generation = LibraryRootGeneration::new(sqlite_unsigned(
                candidate.generation,
                "root generation",
            )?)
            .ok_or_else(|| {
                ScanError::new(
                    "metadata_recheck_generation_invalid",
                    "Metadata recovery requires an active root generation",
                )
            })?;
            let intent = LibraryChangeIntent {
                root_id: candidate.root_id,
                root_generation,
                kind: LibraryChangeIntentKind::Reconcile,
                scope: LibraryChangeScope::Path,
                relative_path: candidate.relative_path,
                previous_relative_path: None,
                origin: LibraryChangeOrigin::ConsistencyAudit,
                first_observed_unix_ms: now_unix_ms,
                most_recent_observed_unix_ms: now_unix_ms,
                first_sequence: 1,
                most_recent_sequence: 1,
                coalesced_observation_count: 1,
            };
            match enqueue_one(
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
            ) {
                Ok(_) => admitted += 1,
                Err(error)
                    if matches!(
                        error.code.as_str(),
                        "change_queue_backpressure" | "metadata_inventory_backpressure"
                    ) =>
                {
                    break;
                }
                Err(error) => return Err(error),
            }
        }
        transaction.commit().map_err(database_error)?;
        Ok(admitted)
    }

    pub(crate) fn roots_with_pending_metadata(&self) -> Result<Vec<String>, ScanError> {
        let mut statement = self.connection.prepare_cached(
            "SELECT library.id FROM library_roots AS library
             JOIN library_change_root_state AS root ON root.root_id = library.id AND root.is_active = 1
             WHERE EXISTS (SELECT 1 FROM asset_locations INDEXED BY asset_locations_pending_metadata
                WHERE root_id = library.id AND scan_id = library.active_scan_id
                  AND metadata_engine_id = 'ame-invalidated-media-metadata')",
        ).map_err(database_error)?;
        let rows = statement
            .query_map([], |row| row.get(0))
            .map_err(database_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(database_error)
    }
}
