use std::cell::Cell;
use std::time::{Duration, Instant};

use crate::domain::{
    AssetLocationView, CatalogCursor, CatalogReadRetryCause, CatalogReadRetryDetails,
    CatalogReadRetryOperation, CatalogSnapshot, GalleryLayoutManifestChunk,
    GalleryLayoutManifestCursor, GalleryQuery, GalleryTimeAnchor, GalleryTimeline,
    IncrementalCatalogRoot, LibraryFolderCursor, LibraryFolderPage, ScanError,
};
use crate::ports::{CatalogRepository, GalleryQueryRepository, IncrementalCatalogRepository};

#[cfg(test)]
use super::WatcherRecoveryObservation;
use super::{SqliteCatalog, SqliteCatalogReadStage, SqliteCatalogSession};

const MAX_PROTOCOL_READ_ATTEMPTS: usize = 5;
pub(super) const PROTOCOL_READ_DEADLINE: Duration = Duration::from_millis(100);
const PROTOCOL_READ_BACKOFF: Duration = Duration::from_millis(1);
const MAX_PROTOCOL_READ_BACKOFF: Duration = Duration::from_millis(8);

pub(crate) struct SqliteCatalogReadExecutor {
    session: SqliteCatalogSession,
    policy: SqliteCatalogReadRetryPolicy,
    #[cfg(test)]
    attempt_metrics: std::sync::Arc<SqliteCatalogReadAttemptMetrics>,
    #[cfg(test)]
    fault_injection: Option<SqliteCatalogReadFaultInjection>,
    #[cfg(test)]
    clock: Option<std::sync::Arc<dyn ProtocolReadClock>>,
    #[cfg(test)]
    stage_observer: Option<Box<dyn Fn(SqliteCatalogReadStage)>>,
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct SqliteCatalogReadAttemptStats {
    pub(crate) operations: usize,
    pub(crate) attempts: usize,
    pub(crate) protocol_retries: usize,
    pub(crate) maximum_attempts: usize,
}

#[cfg(test)]
#[derive(Default)]
struct SqliteCatalogReadAttemptMetrics {
    operations: std::sync::atomic::AtomicUsize,
    attempts: std::sync::atomic::AtomicUsize,
    maximum_attempts: std::sync::atomic::AtomicUsize,
}

struct ProtocolReadOutcome<T> {
    value: T,
    #[cfg(test)]
    attempts: usize,
}

enum CatalogReadAttemptError {
    Protocol(CatalogReadRetryCause),
    Terminal(ScanError),
}

#[derive(Clone, Copy)]
struct SqliteCatalogReadRetryPolicy {
    max_attempts: usize,
    deadline: Duration,
    initial_backoff: Duration,
}

impl Default for SqliteCatalogReadRetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: MAX_PROTOCOL_READ_ATTEMPTS,
            deadline: PROTOCOL_READ_DEADLINE,
            initial_backoff: PROTOCOL_READ_BACKOFF,
        }
    }
}

trait ProtocolReadClock {
    fn now(&self) -> Duration;
    fn sleep(&self, duration: Duration);
}

struct SystemProtocolReadClock {
    origin: Instant,
}

impl SystemProtocolReadClock {
    fn new() -> Self {
        Self {
            origin: Instant::now(),
        }
    }
}

impl ProtocolReadClock for SystemProtocolReadClock {
    fn now(&self) -> Duration {
        self.origin.elapsed()
    }

    fn sleep(&self, duration: Duration) {
        std::thread::sleep(duration);
    }
}

thread_local! {
    static PROTOCOL_CAPTURE_ACTIVE: Cell<bool> = const { Cell::new(false) };
    static PROTOCOL_CAPTURE_CAUSE: Cell<Option<CatalogReadRetryCause>> = const { Cell::new(None) };
}

struct ProtocolCaptureScope {
    previous_active: bool,
    previous_cause: Option<CatalogReadRetryCause>,
    is_finished: bool,
}

impl ProtocolCaptureScope {
    fn begin() -> Self {
        let previous_active = PROTOCOL_CAPTURE_ACTIVE.with(|active| active.replace(true));
        let previous_cause = PROTOCOL_CAPTURE_CAUSE.with(|cause| cause.replace(None));
        Self {
            previous_active,
            previous_cause,
            is_finished: false,
        }
    }

    fn finish<T>(mut self, result: Result<T, ScanError>) -> Result<T, CatalogReadAttemptError> {
        let cause = self.restore();
        match (result, cause) {
            (Ok(value), _) => Ok(value),
            (Err(_), Some(cause)) => Err(CatalogReadAttemptError::Protocol(cause)),
            (Err(error), None) => Err(CatalogReadAttemptError::Terminal(error)),
        }
    }

    fn restore(&mut self) -> Option<CatalogReadRetryCause> {
        let cause = PROTOCOL_CAPTURE_CAUSE.with(|cause| cause.replace(self.previous_cause));
        PROTOCOL_CAPTURE_ACTIVE.with(|active| active.set(self.previous_active));
        self.is_finished = true;
        cause
    }
}

impl Drop for ProtocolCaptureScope {
    fn drop(&mut self) {
        if !self.is_finished {
            let _ = self.restore();
        }
    }
}

pub(super) fn record_database_error_code(code: rusqlite::ErrorCode) {
    if code != rusqlite::ErrorCode::FileLockingProtocolFailed {
        return;
    }
    PROTOCOL_CAPTURE_ACTIVE.with(|active| {
        if active.get() {
            PROTOCOL_CAPTURE_CAUSE.with(|cause| {
                cause.set(Some(CatalogReadRetryCause::FileLockingProtocolFailed));
            });
        }
    });
}

fn capture_protocol_attempt<T>(
    attempt: impl FnOnce() -> Result<T, ScanError>,
) -> Result<T, CatalogReadAttemptError> {
    let scope = ProtocolCaptureScope::begin();
    scope.finish(attempt())
}

impl SqliteCatalogReadExecutor {
    pub(crate) fn new(session: SqliteCatalogSession) -> Self {
        Self {
            session,
            policy: SqliteCatalogReadRetryPolicy::default(),
            #[cfg(test)]
            attempt_metrics: std::sync::Arc::new(SqliteCatalogReadAttemptMetrics::default()),
            #[cfg(test)]
            fault_injection: None,
            #[cfg(test)]
            clock: None,
            #[cfg(test)]
            stage_observer: None,
        }
    }

    #[cfg(test)]
    pub(crate) fn open_existing(path: std::path::PathBuf) -> Result<Self, ScanError> {
        Self::open_existing_with(path, SqliteCatalogReadRetryPolicy::default(), None, None)
    }

    #[cfg(test)]
    fn open_existing_with(
        path: std::path::PathBuf,
        policy: SqliteCatalogReadRetryPolicy,
        #[cfg(test)] fault_injection: Option<SqliteCatalogReadFaultInjection>,
        #[cfg(not(test))] _fault_injection: Option<()>,
        #[cfg(test)] clock: Option<std::sync::Arc<dyn ProtocolReadClock>>,
        #[cfg(not(test))] _clock: Option<()>,
    ) -> Result<Self, ScanError> {
        let mut attempt = || {
            SqliteCatalogSession::validate_existing_for_read(path.clone(), |stage| {
                #[cfg(test)]
                if let Some(fault_injection) = &fault_injection {
                    return fault_injection.before_stage(stage);
                }
                let _ = stage;
                Ok(())
            })
        };
        #[cfg(test)]
        let outcome = if let Some(clock) = &clock {
            retry_protocol_operation(
                CatalogReadRetryOperation::SessionValidation,
                policy,
                clock.as_ref(),
                &mut attempt,
            )?
        } else {
            let clock = SystemProtocolReadClock::new();
            retry_protocol_operation(
                CatalogReadRetryOperation::SessionValidation,
                policy,
                &clock,
                &mut attempt,
            )?
        };
        #[cfg(not(test))]
        let outcome = {
            let clock = SystemProtocolReadClock::new();
            retry_protocol_operation(
                CatalogReadRetryOperation::SessionValidation,
                policy,
                &clock,
                &mut attempt,
            )?
        };
        let executor = Self::new(outcome.value);
        #[cfg(test)]
        executor.record_attempts(outcome.attempts);
        #[cfg(test)]
        let executor = executor.with_test_configuration(fault_injection, policy, clock);
        Ok(executor)
    }

    #[cfg(test)]
    fn open_existing_with_test_configuration(
        path: std::path::PathBuf,
        fault_injection: SqliteCatalogReadFaultInjection,
        policy: SqliteCatalogReadRetryPolicy,
        clock: Option<std::sync::Arc<dyn ProtocolReadClock>>,
    ) -> Result<Self, ScanError> {
        Self::open_existing_with(path, policy, Some(fault_injection), clock)
    }

    fn execute_read<T>(
        &self,
        operation: CatalogReadRetryOperation,
        mut read: impl FnMut(&mut SqliteCatalog) -> Result<T, ScanError>,
    ) -> Result<T, ScanError> {
        let mut attempt = || {
            let mut catalog = self
                .session
                .open_read_only_with_read_stage(|stage| self.before_stage(stage))?;
            self.before_stage(SqliteCatalogReadStage::Query)?;
            read(&mut catalog)
        };
        #[cfg(test)]
        let outcome = if let Some(clock) = &self.clock {
            retry_protocol_operation(operation, self.policy, clock.as_ref(), &mut attempt)?
        } else {
            let clock = SystemProtocolReadClock::new();
            retry_protocol_operation(operation, self.policy, &clock, &mut attempt)?
        };
        #[cfg(not(test))]
        let outcome = {
            let clock = SystemProtocolReadClock::new();
            retry_protocol_operation(operation, self.policy, &clock, &mut attempt)?
        };
        #[cfg(test)]
        self.record_attempts(outcome.attempts);
        Ok(outcome.value)
    }

    pub(crate) fn load_snapshot(
        &self,
        max_items: u32,
        query: &GalleryQuery,
        query_id: &str,
        after: Option<&CatalogCursor>,
        before: Option<&CatalogCursor>,
        anchor: Option<&GalleryTimeAnchor>,
    ) -> Result<CatalogSnapshot, ScanError> {
        self.execute_read(CatalogReadRetryOperation::CatalogSnapshot, |catalog| {
            catalog.load_snapshot(max_items, query, query_id, after, before, anchor)
        })
    }

    pub(crate) fn load_query_snapshot(
        &self,
        max_items: u32,
        query: &GalleryQuery,
        query_id: &str,
        anchor: Option<&crate::domain::GalleryQueryAnchor>,
    ) -> Result<crate::domain::GalleryQuerySnapshot, ScanError> {
        self.execute_read(CatalogReadRetryOperation::CatalogSnapshot, |catalog| {
            catalog.load_query_snapshot(max_items, query, query_id, anchor)
        })
    }

    pub(crate) fn load_snapshot_around_location(
        &self,
        max_items: u32,
        query: &GalleryQuery,
        query_id: &str,
        anchor_location_id: &str,
    ) -> Result<CatalogSnapshot, ScanError> {
        self.execute_read(
            CatalogReadRetryOperation::CatalogSnapshotAroundLocation,
            |catalog| {
                catalog.load_snapshot_around_location(
                    max_items,
                    query,
                    query_id,
                    anchor_location_id,
                )
            },
        )
    }

    pub(crate) fn load_snapshot_around_asset(
        &self,
        max_items: u32,
        query: &GalleryQuery,
        query_id: &str,
        requested_location_id: &str,
        anchor_asset_id: &str,
        fallback_ordinal: u64,
    ) -> Result<CatalogSnapshot, ScanError> {
        self.execute_read(
            CatalogReadRetryOperation::CatalogSnapshotAroundAsset,
            |catalog| {
                catalog.load_snapshot_around_asset(
                    max_items,
                    query,
                    query_id,
                    requested_location_id,
                    anchor_asset_id,
                    fallback_ordinal,
                )
            },
        )
    }

    pub(crate) fn load_gallery_timeline(
        &self,
        query: &GalleryQuery,
        query_id: &str,
    ) -> Result<GalleryTimeline, ScanError> {
        self.execute_read(CatalogReadRetryOperation::GalleryTimeline, |catalog| {
            catalog.load_gallery_timeline(query, query_id)
        })
    }

    pub(crate) fn load_gallery_layout_manifest_chunk(
        &self,
        max_items: u32,
        query: &GalleryQuery,
        query_id: &str,
        after: Option<&GalleryLayoutManifestCursor>,
    ) -> Result<GalleryLayoutManifestChunk, ScanError> {
        self.execute_read(
            CatalogReadRetryOperation::GalleryLayoutManifest,
            |catalog| catalog.load_gallery_layout_manifest_chunk(max_items, query, query_id, after),
        )
    }

    pub(crate) fn load_folder_page(
        &self,
        root_id: &str,
        parent_relative_path: &str,
        max_items: u32,
        after: Option<&LibraryFolderCursor>,
    ) -> Result<LibraryFolderPage, ScanError> {
        self.execute_read(CatalogReadRetryOperation::LibraryFolders, |catalog| {
            catalog.load_folder_page(root_id, parent_relative_path, max_items, after)
        })
    }

    pub(crate) fn load_active_location_by_asset_id(
        &self,
        asset_id: &str,
        preferred_location_id: Option<&str>,
    ) -> Result<Option<AssetLocationView>, ScanError> {
        self.execute_read(CatalogReadRetryOperation::CatalogAssetById, |catalog| {
            catalog.load_active_location_by_asset_id(asset_id, preferred_location_id)
        })
    }

    pub(crate) fn load_preview_context(
        &self,
        location_id: &str,
    ) -> Result<(Option<AssetLocationView>, Option<IncrementalCatalogRoot>), ScanError> {
        self.execute_read(CatalogReadRetryOperation::CatalogAssetById, |catalog| {
            let location = catalog.load_active_location(location_id)?;
            let root = match location.as_ref() {
                Some(location) => catalog.load_incremental_catalog_root(&location.root_id)?,
                None => None,
            };
            Ok((location, root))
        })
    }

    #[cfg(test)]
    pub(crate) fn load_watcher_recovery_observation_for_test(
        &self,
        root_id: &str,
    ) -> Result<WatcherRecoveryObservation, ScanError> {
        self.execute_read(
            CatalogReadRetryOperation::WatcherGapAuthorityCount,
            |catalog| catalog.load_watcher_recovery_observation_for_test(root_id),
        )
    }

    #[cfg(test)]
    pub(crate) fn load_incremental_location_by_relative_path_for_test(
        &self,
        root_id: &str,
        relative_path: &str,
    ) -> Result<Option<AssetLocationView>, ScanError> {
        self.execute_read(CatalogReadRetryOperation::IncrementalLocation, |catalog| {
            catalog.load_incremental_location_by_relative_path(root_id, relative_path)
        })
    }

    #[cfg(test)]
    pub(crate) fn load_incremental_locations_by_relative_paths_for_test(
        &self,
        root_id: &str,
        relative_paths: &[String],
    ) -> Result<Vec<AssetLocationView>, ScanError> {
        self.execute_read(CatalogReadRetryOperation::IncrementalLocation, |catalog| {
            catalog.load_incremental_locations_by_relative_paths(root_id, relative_paths)
        })
    }

    #[cfg(test)]
    pub(crate) fn persistent_journal_range_is_completed_for_test(
        &self,
        root_id: &str,
        start_usn: i64,
        end_usn: i64,
    ) -> Result<bool, ScanError> {
        self.execute_read(
            CatalogReadRetryOperation::CompletedPersistentJournalRange,
            |catalog| {
                let count: i64 = catalog
                    .connection
                    .query_row(
                        "SELECT COUNT(*)
                         FROM library_persistent_journal_source_ranges AS ranges
                         JOIN library_persistent_journal_range_lifecycle AS lifecycle
                           ON lifecycle.source_range_id = ranges.id
                         JOIN library_persistent_journal_queue_lineage AS lineage
                           ON lineage.source_range_id = ranges.id
                         JOIN library_change_queue AS queue ON queue.id = lineage.change_id
                         JOIN library_change_queue_lanes AS lane ON lane.change_id = queue.id
                         WHERE ranges.root_id = ?1
                           AND ranges.requested_start_usn = ?2
                           AND ranges.requested_end_usn = ?3
                           AND ranges.status = 'checkpointed'
                           AND lifecycle.lifecycle_state = 'completed'
                           AND queue.status = 'completed'
                           AND lane.lane = 'p1_journal'",
                        (root_id, start_usn.to_string(), end_usn.to_string()),
                        |row| row.get(0),
                    )
                    .map_err(super::database_error)?;
                Ok(count == 1)
            },
        )
    }

    fn before_stage(&self, stage: SqliteCatalogReadStage) -> Result<(), ScanError> {
        #[cfg(test)]
        if let Some(observer) = &self.stage_observer {
            observer(stage);
        }
        #[cfg(test)]
        if let Some(fault_injection) = &self.fault_injection {
            return fault_injection.before_stage(stage);
        }
        let _ = stage;
        Ok(())
    }

    #[cfg(test)]
    fn with_test_configuration(
        mut self,
        fault_injection: Option<SqliteCatalogReadFaultInjection>,
        policy: SqliteCatalogReadRetryPolicy,
        clock: Option<std::sync::Arc<dyn ProtocolReadClock>>,
    ) -> Self {
        self.fault_injection = fault_injection;
        self.policy = policy;
        self.clock = clock;
        self
    }

    #[cfg(test)]
    fn with_stage_observer(mut self, observer: Box<dyn Fn(SqliteCatalogReadStage)>) -> Self {
        self.stage_observer = Some(observer);
        self
    }

    #[cfg(test)]
    fn record_attempts(&self, attempts: usize) {
        use std::sync::atomic::Ordering;

        self.attempt_metrics
            .operations
            .fetch_add(1, Ordering::Relaxed);
        self.attempt_metrics
            .attempts
            .fetch_add(attempts, Ordering::Relaxed);
        self.attempt_metrics
            .maximum_attempts
            .fetch_max(attempts, Ordering::Relaxed);
    }

    #[cfg(test)]
    pub(crate) fn attempt_stats(&self) -> SqliteCatalogReadAttemptStats {
        use std::sync::atomic::Ordering;

        let operations = self.attempt_metrics.operations.load(Ordering::Relaxed);
        let attempts = self.attempt_metrics.attempts.load(Ordering::Relaxed);
        SqliteCatalogReadAttemptStats {
            operations,
            attempts,
            protocol_retries: attempts.saturating_sub(operations),
            maximum_attempts: self
                .attempt_metrics
                .maximum_attempts
                .load(Ordering::Relaxed),
        }
    }
}

fn retry_protocol_operation<T>(
    operation: CatalogReadRetryOperation,
    policy: SqliteCatalogReadRetryPolicy,
    clock: &(impl ProtocolReadClock + ?Sized),
    mut attempt: impl FnMut() -> Result<T, ScanError>,
) -> Result<ProtocolReadOutcome<T>, ScanError> {
    let started = clock.now();
    let mut attempts = 0_usize;
    loop {
        attempts = attempts.saturating_add(1);
        match capture_protocol_attempt(&mut attempt) {
            Ok(result) => {
                return Ok(ProtocolReadOutcome {
                    value: result,
                    #[cfg(test)]
                    attempts,
                });
            }
            Err(CatalogReadAttemptError::Terminal(error)) => return Err(error),
            Err(CatalogReadAttemptError::Protocol(cause)) => {
                let elapsed = clock.now().saturating_sub(started);
                if attempts >= policy.max_attempts || elapsed >= policy.deadline {
                    return Err(protocol_retry_exhausted_error(
                        operation, attempts, elapsed, cause,
                    ));
                }
                let remaining = policy.deadline.saturating_sub(elapsed);
                let backoff_multiplier = 1_u32
                    .checked_shl(u32::try_from(attempts.saturating_sub(1)).unwrap_or(u32::MAX))
                    .unwrap_or(u32::MAX);
                let backoff = policy
                    .initial_backoff
                    .saturating_mul(backoff_multiplier)
                    .min(MAX_PROTOCOL_READ_BACKOFF)
                    .min(remaining);
                if !backoff.is_zero() {
                    clock.sleep(backoff);
                }
                let elapsed = clock.now().saturating_sub(started);
                if elapsed >= policy.deadline {
                    return Err(protocol_retry_exhausted_error(
                        operation, attempts, elapsed, cause,
                    ));
                }
            }
        }
    }
}

fn protocol_retry_exhausted_error(
    operation: CatalogReadRetryOperation,
    attempts: usize,
    elapsed: Duration,
    cause: CatalogReadRetryCause,
) -> ScanError {
    ScanError::new(
        "catalog_read_protocol_retry_exhausted",
        "The catalog read could not acquire SQLite WAL protocol state within the bounded retry policy",
    )
    .with_retry_details(CatalogReadRetryDetails {
        operation,
        attempts: u32::try_from(attempts).unwrap_or(u32::MAX),
        elapsed_ms: u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX),
        cause,
    })
}

#[cfg(test)]
#[derive(Clone)]
struct SqliteCatalogReadFaultInjection {
    state: std::sync::Arc<std::sync::Mutex<SqliteCatalogReadFaultState>>,
}

#[cfg(test)]
struct SqliteCatalogReadFaultState {
    stage: SqliteCatalogReadStage,
    remaining: usize,
    failure: SqliteCatalogReadInjectedFailure,
    open_attempts: usize,
    validation_attempts: usize,
    query_attempts: usize,
}

#[cfg(test)]
#[derive(Clone)]
enum SqliteCatalogReadInjectedFailure {
    Sqlite { code: i32, message: String },
    Terminal(ScanError),
}

#[cfg(test)]
impl SqliteCatalogReadFaultInjection {
    fn new(stage: SqliteCatalogReadStage, remaining: usize, error: ScanError) -> Self {
        Self {
            state: std::sync::Arc::new(std::sync::Mutex::new(SqliteCatalogReadFaultState {
                stage,
                remaining,
                failure: SqliteCatalogReadInjectedFailure::Terminal(error),
                open_attempts: 0,
                validation_attempts: 0,
                query_attempts: 0,
            })),
        }
    }

    fn protocol(stage: SqliteCatalogReadStage, remaining: usize) -> Self {
        Self {
            state: std::sync::Arc::new(std::sync::Mutex::new(SqliteCatalogReadFaultState {
                stage,
                remaining,
                failure: SqliteCatalogReadInjectedFailure::Sqlite {
                    code: rusqlite::ffi::SQLITE_PROTOCOL,
                    message: "injected file-locking protocol failure".to_owned(),
                },
                open_attempts: 0,
                validation_attempts: 0,
                query_attempts: 0,
            })),
        }
    }

    fn protocol_with_message(
        stage: SqliteCatalogReadStage,
        remaining: usize,
        message: String,
    ) -> Self {
        Self {
            state: std::sync::Arc::new(std::sync::Mutex::new(SqliteCatalogReadFaultState {
                stage,
                remaining,
                failure: SqliteCatalogReadInjectedFailure::Sqlite {
                    code: rusqlite::ffi::SQLITE_PROTOCOL,
                    message,
                },
                open_attempts: 0,
                validation_attempts: 0,
                query_attempts: 0,
            })),
        }
    }

    fn sqlite(stage: SqliteCatalogReadStage, remaining: usize, code: i32, message: String) -> Self {
        Self {
            state: std::sync::Arc::new(std::sync::Mutex::new(SqliteCatalogReadFaultState {
                stage,
                remaining,
                failure: SqliteCatalogReadInjectedFailure::Sqlite { code, message },
                open_attempts: 0,
                validation_attempts: 0,
                query_attempts: 0,
            })),
        }
    }

    fn before_stage(&self, stage: SqliteCatalogReadStage) -> Result<(), ScanError> {
        let mut state = self.state.lock().unwrap_or_else(|error| error.into_inner());
        match stage {
            SqliteCatalogReadStage::Open => state.open_attempts += 1,
            SqliteCatalogReadStage::Validation => state.validation_attempts += 1,
            SqliteCatalogReadStage::Query => state.query_attempts += 1,
        }
        if state.stage == stage && state.remaining > 0 {
            state.remaining -= 1;
            let failure = state.failure.clone();
            drop(state);
            return match failure {
                SqliteCatalogReadInjectedFailure::Sqlite { code, message } => {
                    Err(super::database_error(rusqlite::Error::SqliteFailure(
                        rusqlite::ffi::Error::new(code),
                        Some(message),
                    )))
                }
                SqliteCatalogReadInjectedFailure::Terminal(error) => Err(error),
            };
        }
        Ok(())
    }

    fn attempts(&self, stage: SqliteCatalogReadStage) -> usize {
        let state = self.state.lock().unwrap_or_else(|error| error.into_inner());
        match stage {
            SqliteCatalogReadStage::Open => state.open_attempts,
            SqliteCatalogReadStage::Validation => state.validation_attempts,
            SqliteCatalogReadStage::Query => state.query_attempts,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    use flutter_rust_bridge::for_generated::SseSerializer;
    use rusqlite::Connection;
    use syn::visit::Visit;
    use tempfile::TempDir;

    use crate::domain::{
        CatalogReadRetryCause, CatalogReadRetryDetails, CatalogReadRetryOperation,
        LibraryChangeLane, ScanError,
    };
    use crate::frb_generated::SseEncode;

    use super::*;

    const TEST_ATTEMPT_LIMIT_DEADLINE: Duration = Duration::from_secs(30);

    fn fixture_executor() -> (TempDir, SqliteCatalogReadExecutor) {
        let storage = tempfile::tempdir().expect("storage");
        let path = storage.path().join("catalog").join("ame.sqlite3");
        let session = SqliteCatalogSession::validate(path).expect("validated catalog session");
        (storage, SqliteCatalogReadExecutor::new(session))
    }

    fn protocol_error() -> ScanError {
        super::super::database_error(rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_PROTOCOL),
            Some("injected file-locking protocol failure".to_owned()),
        ))
    }

    fn read_one(catalog: &mut SqliteCatalog) -> Result<i64, ScanError> {
        catalog
            .connection
            .query_row("SELECT 1", [], |row| row.get(0))
            .map_err(super::super::database_error)
    }

    fn encoded_scan_error(error: ScanError) -> Vec<u8> {
        let mut serializer = SseSerializer::new();
        error.sse_encode(&mut serializer);
        serializer.cursor.into_inner()
    }

    fn fast_policy(max_attempts: usize) -> SqliteCatalogReadRetryPolicy {
        SqliteCatalogReadRetryPolicy {
            max_attempts,
            deadline: TEST_ATTEMPT_LIMIT_DEADLINE,
            initial_backoff: Duration::ZERO,
        }
    }

    #[derive(Default)]
    struct ManualProtocolReadClock {
        now: Mutex<Duration>,
        sleeps: Mutex<Vec<Duration>>,
        on_sleep: Option<Arc<dyn Fn(Duration) + Send + Sync>>,
    }

    impl ManualProtocolReadClock {
        fn with_sleep_observer(observer: Arc<dyn Fn(Duration) + Send + Sync>) -> Self {
            Self {
                on_sleep: Some(observer),
                ..Self::default()
            }
        }

        fn advance(&self, duration: Duration) {
            let mut now = self.now.lock().unwrap_or_else(|error| error.into_inner());
            *now = now.saturating_add(duration);
        }

        fn observed_sleeps(&self) -> Vec<Duration> {
            self.sleeps
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .clone()
        }
    }

    impl ProtocolReadClock for ManualProtocolReadClock {
        fn now(&self) -> Duration {
            *self.now.lock().unwrap_or_else(|error| error.into_inner())
        }

        fn sleep(&self, duration: Duration) {
            self.sleeps
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .push(duration);
            if let Some(observer) = &self.on_sleep {
                observer(duration);
            }
            self.advance(duration);
        }
    }

    #[derive(Default)]
    struct ReadCapabilityLeakVisitor {
        exposes_callback: bool,
        exposes_catalog_owner: bool,
    }

    impl<'ast> Visit<'ast> for ReadCapabilityLeakVisitor {
        fn visit_trait_bound(&mut self, bound: &'ast syn::TraitBound) {
            if bound.path.segments.last().is_some_and(|segment| {
                matches!(
                    segment.ident.to_string().as_str(),
                    "Fn" | "FnMut" | "FnOnce"
                )
            }) {
                self.exposes_callback = true;
            }
            syn::visit::visit_trait_bound(self, bound);
        }

        fn visit_type_path(&mut self, path: &'ast syn::TypePath) {
            if path.path.segments.last().is_some_and(|segment| {
                matches!(
                    segment.ident.to_string().as_str(),
                    "SqliteCatalog" | "SqliteCatalogReadResult" | "Connection" | "Transaction"
                )
            }) {
                self.exposes_catalog_owner = true;
            }
            syn::visit::visit_type_path(self, path);
        }
    }

    #[derive(Default)]
    struct DirectClockCouplingVisitor {
        instant_now_calls: usize,
        elapsed_calls: usize,
    }

    impl<'ast> Visit<'ast> for DirectClockCouplingVisitor {
        fn visit_expr_call(&mut self, call: &'ast syn::ExprCall) {
            if let syn::Expr::Path(path) = call.func.as_ref() {
                let mut segments = path.path.segments.iter().rev();
                if segments
                    .next()
                    .is_some_and(|segment| segment.ident == "now")
                    && segments
                        .next()
                        .is_some_and(|segment| segment.ident == "Instant")
                {
                    self.instant_now_calls = self.instant_now_calls.saturating_add(1);
                }
            }
            syn::visit::visit_expr_call(self, call);
        }

        fn visit_expr_method_call(&mut self, call: &'ast syn::ExprMethodCall) {
            if call.method == "elapsed" {
                self.elapsed_calls = self.elapsed_calls.saturating_add(1);
            }
            syn::visit::visit_expr_method_call(self, call);
        }
    }

    #[test]
    fn crate_visible_read_capability_is_closed_to_typed_side_effect_free_operations() {
        let syntax = syn::parse_file(include_str!("read_retry.rs")).expect("read retry syntax");
        let mut inspected_methods = 0_usize;
        for item in &syntax.items {
            let syn::Item::Impl(item_impl) = item else {
                continue;
            };
            let syn::Type::Path(self_type) = item_impl.self_ty.as_ref() else {
                continue;
            };
            if !self_type
                .path
                .segments
                .last()
                .is_some_and(|segment| segment.ident == "SqliteCatalogReadExecutor")
            {
                continue;
            }
            for item in &item_impl.items {
                let syn::ImplItem::Fn(method) = item else {
                    continue;
                };
                if matches!(method.vis, syn::Visibility::Inherited) {
                    continue;
                }
                inspected_methods = inspected_methods.saturating_add(1);
                let mut visitor = ReadCapabilityLeakVisitor::default();
                visitor.visit_signature(&method.sig);
                assert!(
                    !visitor.exposes_callback,
                    "crate-visible read method {} exposes a caller-controlled callback",
                    method.sig.ident,
                );
                assert!(
                    !visitor.exposes_catalog_owner,
                    "crate-visible read method {} exposes the SQLite/catalog owner",
                    method.sig.ident,
                );
            }
        }
        assert!(
            inspected_methods > 0,
            "read capability impl was not inspected"
        );
    }

    #[test]
    fn retry_loop_delegates_monotonic_time_to_a_private_clock() {
        let syntax = syn::parse_file(include_str!("read_retry.rs")).expect("read retry syntax");
        let function = syntax
            .items
            .iter()
            .find_map(|item| match item {
                syn::Item::Fn(function) if function.sig.ident == "retry_protocol_operation" => {
                    Some(function)
                }
                _ => None,
            })
            .expect("retry protocol loop");
        let mut visitor = DirectClockCouplingVisitor::default();
        visitor.visit_block(&function.block);

        assert_eq!(
            visitor.instant_now_calls, 0,
            "retry loop constructs Instant directly instead of using its clock owner",
        );
        assert_eq!(
            visitor.elapsed_calls, 0,
            "retry loop reads elapsed wall time directly instead of its clock owner",
        );
    }

    #[test]
    fn forged_protocol_scan_error_is_terminal_and_never_retried() {
        let (_storage, executor) = fixture_executor();
        let faults = SqliteCatalogReadFaultInjection::new(
            SqliteCatalogReadStage::Query,
            1,
            ScanError::new(
                "catalog_database_protocol",
                "forged public error with no rusqlite protocol cause",
            ),
        );
        let executor = executor.with_test_configuration(Some(faults.clone()), fast_policy(5), None);

        let error = executor
            .execute_read(CatalogReadRetryOperation::CatalogSnapshot, read_one)
            .expect_err("a forged public code must remain terminal");

        assert_eq!(error.code, "catalog_database_protocol");
        assert_eq!(faults.attempts(SqliteCatalogReadStage::Open), 1);
        assert_eq!(faults.attempts(SqliteCatalogReadStage::Query), 1);
    }

    #[test]
    fn retry_exhaustion_never_echoes_an_arbitrary_cause_message_or_path() {
        let (_storage, executor) = fixture_executor();
        let secret_path = r"C:\Users\private-owner\Pictures\sensitive.sqlite3";
        let faults = SqliteCatalogReadFaultInjection::protocol_with_message(
            SqliteCatalogReadStage::Query,
            10,
            format!("protocol failure while opening {secret_path}"),
        );
        let executor = executor.with_test_configuration(Some(faults), fast_policy(2), None);

        let error = executor
            .execute_read(CatalogReadRetryOperation::CatalogSnapshot, read_one)
            .expect_err("protocol retry exhaustion");

        assert_eq!(error.code, "catalog_read_protocol_retry_exhausted");
        assert!(!error.message.contains(secret_path));
        assert!(!error.message.contains("private-owner"));
    }

    #[cfg(windows)]
    #[test]
    fn deletion_after_identity_check_never_recreates_a_catalog_database() {
        use std::sync::atomic::AtomicBool;

        let (_storage, executor) = fixture_executor();
        let path = executor.session.path().to_path_buf();
        let delete_path = path.clone();
        let deletion_was_blocked = Arc::new(AtomicBool::new(false));
        let observer_deletion_was_blocked = Arc::clone(&deletion_was_blocked);
        let executor = executor.with_stage_observer(Box::new(move |stage| {
            if stage == SqliteCatalogReadStage::Open
                && !observer_deletion_was_blocked.load(Ordering::SeqCst)
            {
                let deletion = std::fs::remove_file(&delete_path);
                let blocked = deletion.is_err();
                observer_deletion_was_blocked.store(blocked, Ordering::SeqCst);
            }
        }));

        let value = executor
            .execute_read(CatalogReadRetryOperation::CatalogSnapshot, read_one)
            .expect("held identity must keep the original catalog readable");

        assert_eq!(value, 1);
        assert!(
            deletion_was_blocked.load(Ordering::SeqCst),
            "the held catalog identity must deny delete sharing"
        );
        assert!(path.is_file());
    }

    #[cfg(windows)]
    #[test]
    fn replacement_after_held_identity_is_blocked_before_sqlite_open() {
        use std::sync::atomic::AtomicBool;

        let (_storage, executor) = fixture_executor();
        let path = executor.session.path().to_path_buf();
        let retained = path.with_extension("held-original");
        let observer_path = path.clone();
        let observer_retained = retained.clone();
        let replacement_was_blocked = Arc::new(AtomicBool::new(false));
        let observed = Arc::clone(&replacement_was_blocked);
        let executor = executor.with_stage_observer(Box::new(move |stage| {
            if stage != SqliteCatalogReadStage::Open || observed.load(Ordering::SeqCst) {
                return;
            }
            match std::fs::rename(&observer_path, &observer_retained) {
                Err(_) => observed.store(true, Ordering::SeqCst),
                Ok(()) => {
                    std::fs::copy(&observer_retained, &observer_path)
                        .expect("create disposable replacement");
                }
            }
        }));

        assert_eq!(
            executor
                .execute_read(CatalogReadRetryOperation::CatalogSnapshot, read_one)
                .expect("held original catalog read"),
            1,
        );
        assert!(replacement_was_blocked.load(Ordering::SeqCst));
        assert!(!retained.exists());
    }

    #[cfg(windows)]
    #[test]
    fn aba_replacement_after_held_identity_is_blocked() {
        use std::sync::atomic::AtomicBool;

        let (_storage, executor) = fixture_executor();
        let path = executor.session.path().to_path_buf();
        let retained = path.with_extension("aba-original");
        let observer_path = path.clone();
        let observer_retained = retained.clone();
        let aba_was_blocked = Arc::new(AtomicBool::new(false));
        let observed = Arc::clone(&aba_was_blocked);
        let executor = executor.with_stage_observer(Box::new(move |stage| {
            if stage != SqliteCatalogReadStage::Open || observed.load(Ordering::SeqCst) {
                return;
            }
            match std::fs::rename(&observer_path, &observer_retained) {
                Err(_) => observed.store(true, Ordering::SeqCst),
                Ok(()) => {
                    std::fs::copy(&observer_retained, &observer_path)
                        .expect("create disposable ABA replacement");
                    std::fs::remove_file(&observer_path)
                        .expect("remove disposable ABA replacement");
                    std::fs::rename(&observer_retained, &observer_path)
                        .expect("restore disposable ABA original");
                }
            }
        }));

        assert_eq!(
            executor
                .execute_read(CatalogReadRetryOperation::CatalogSnapshot, read_one)
                .expect("held original catalog read"),
            1,
        );
        assert!(aba_was_blocked.load(Ordering::SeqCst));
        assert!(!retained.exists());
    }

    #[cfg(windows)]
    #[test]
    fn held_guard_identity_mismatch_fails_before_opening_sqlite() {
        let (_storage, executor) = fixture_executor();
        let path = executor.session.path().to_path_buf();
        let retained = path.with_extension("expected-identity");
        std::fs::rename(&path, &retained).expect("retain expected catalog identity");
        std::fs::copy(&retained, &path).expect("create replacement catalog identity");

        let error = executor
            .execute_read(CatalogReadRetryOperation::CatalogSnapshot, read_one)
            .expect_err("replacement identity must fail closed");

        assert_eq!(error.code, "catalog_validated_session_stale");
        assert!(error.retry_details.is_none());
    }

    #[cfg(windows)]
    #[test]
    fn held_identity_releases_after_success_and_allows_fixture_cleanup() {
        let (_storage, executor) = fixture_executor();
        let path = executor.session.path().to_path_buf();
        let moved = path.with_extension("after-success");

        assert_eq!(
            executor
                .execute_read(CatalogReadRetryOperation::CatalogSnapshot, read_one)
                .expect("held catalog read"),
            1,
        );
        std::fs::rename(&path, &moved).expect("rename after read owner drop");
        std::fs::rename(&moved, &path).expect("restore after read owner drop");
    }

    #[cfg(windows)]
    #[test]
    fn held_identity_releases_before_protocol_backoff() {
        let (_storage, executor) = fixture_executor();
        let path = executor.session.path().to_path_buf();
        let moved = path.with_extension("during-backoff");
        let sleep_path = path.clone();
        let sleep_moved = moved.clone();
        let faults = SqliteCatalogReadFaultInjection::protocol(SqliteCatalogReadStage::Query, 1);
        let clock = Arc::new(ManualProtocolReadClock::with_sleep_observer(Arc::new(
            move |_| {
                std::fs::rename(&sleep_path, &sleep_moved)
                    .expect("rename after failed attempt owner drop");
                std::fs::rename(&sleep_moved, &sleep_path).expect("restore before retry attempt");
            },
        )));
        let executor = executor.with_test_configuration(
            Some(faults),
            SqliteCatalogReadRetryPolicy::default(),
            Some(clock),
        );

        assert_eq!(
            executor
                .execute_read(CatalogReadRetryOperation::CatalogSnapshot, read_one)
                .expect("retry after identity owner drop"),
            1,
        );
        assert!(path.is_file());
        assert!(!moved.exists());
    }

    #[cfg(windows)]
    #[test]
    fn terminal_query_failure_releases_identity_owner_exactly_once() {
        let (_storage, executor) = fixture_executor();
        let path = executor.session.path().to_path_buf();
        let moved = path.with_extension("after-terminal-error");
        let faults = SqliteCatalogReadFaultInjection::new(
            SqliteCatalogReadStage::Query,
            1,
            ScanError::new("terminal-test-error", "terminal fixture failure"),
        );
        let executor = executor.with_test_configuration(
            Some(faults),
            SqliteCatalogReadRetryPolicy::default(),
            None,
        );

        let error = executor
            .execute_read(CatalogReadRetryOperation::CatalogSnapshot, read_one)
            .expect_err("terminal query failure");
        assert_eq!(error.code, "terminal-test-error");
        std::fs::rename(&path, &moved).expect("rename after terminal owner drop");
        std::fs::rename(&moved, &path).expect("restore after terminal owner drop");
    }

    #[cfg(windows)]
    #[test]
    fn terminal_catalog_reparse_point_is_rejected() {
        use std::os::windows::fs::symlink_file;

        let (storage, executor) = fixture_executor();
        let target = executor.session.path().to_path_buf();
        let link = storage.path().join("catalog-link.sqlite3");
        symlink_file(&target, &link).expect("create disposable catalog symlink");

        let error = SqliteCatalogReadExecutor::open_existing(link)
            .err()
            .expect("terminal catalog reparse must fail closed");

        assert_eq!(error.code, "catalog_identity_unavailable");
        assert!(error.retry_details.is_none());
    }

    #[test]
    fn production_retry_policy_remains_tightly_bounded() {
        let policy = SqliteCatalogReadRetryPolicy::default();

        assert_eq!(policy.max_attempts, 5);
        assert_eq!(policy.deadline, Duration::from_millis(100));
        assert_eq!(policy.initial_backoff, Duration::from_millis(1));
        assert_eq!(MAX_PROTOCOL_READ_BACKOFF, Duration::from_millis(8));
    }

    #[test]
    fn production_policy_recovers_at_attempts_two_through_five_for_every_read_stage() {
        let expected_backoffs = [
            Duration::from_millis(1),
            Duration::from_millis(2),
            Duration::from_millis(4),
            Duration::from_millis(8),
        ];
        for stage in [
            SqliteCatalogReadStage::Open,
            SqliteCatalogReadStage::Validation,
            SqliteCatalogReadStage::Query,
        ] {
            for successful_attempt in 2_usize..=5 {
                let (_storage, executor) = fixture_executor();
                let faults = SqliteCatalogReadFaultInjection::protocol(
                    stage,
                    successful_attempt.saturating_sub(1),
                );
                let clock = Arc::new(ManualProtocolReadClock::default());
                let executor = executor.with_test_configuration(
                    Some(faults.clone()),
                    SqliteCatalogReadRetryPolicy::default(),
                    Some(clock.clone()),
                );

                assert_eq!(
                    executor
                        .execute_read(CatalogReadRetryOperation::CatalogSnapshot, read_one)
                        .expect("production policy protocol recovery"),
                    1,
                );
                assert_eq!(faults.attempts(stage), successful_attempt);
                assert_eq!(
                    clock.observed_sleeps(),
                    expected_backoffs[..successful_attempt.saturating_sub(1)]
                );
            }
        }
    }

    #[test]
    fn production_deadline_prevents_admitting_an_attempt_after_expiry() {
        let (_storage, executor) = fixture_executor();
        let faults = SqliteCatalogReadFaultInjection::protocol(SqliteCatalogReadStage::Query, 10);
        let clock = Arc::new(ManualProtocolReadClock::default());
        let advancing_clock = Arc::clone(&clock);
        let executor = executor
            .with_test_configuration(
                Some(faults.clone()),
                SqliteCatalogReadRetryPolicy::default(),
                Some(clock.clone()),
            )
            .with_stage_observer(Box::new(move |stage| {
                if stage == SqliteCatalogReadStage::Query {
                    advancing_clock.advance(Duration::from_millis(40));
                }
            }));

        let error = executor
            .execute_read(CatalogReadRetryOperation::CatalogSnapshot, read_one)
            .expect_err("deadline exhaustion");

        assert_eq!(faults.attempts(SqliteCatalogReadStage::Query), 3);
        assert_eq!(
            clock.observed_sleeps(),
            vec![Duration::from_millis(1), Duration::from_millis(2),]
        );
        assert_eq!(
            error.retry_details,
            Some(CatalogReadRetryDetails {
                operation: CatalogReadRetryOperation::CatalogSnapshot,
                attempts: 3,
                elapsed_ms: 123,
                cause: CatalogReadRetryCause::FileLockingProtocolFailed,
            })
        );
    }

    #[test]
    fn one_attempt_crossing_the_production_deadline_reports_once_with_actual_elapsed_time() {
        let (_storage, executor) = fixture_executor();
        let faults = SqliteCatalogReadFaultInjection::protocol(SqliteCatalogReadStage::Query, 10);
        let clock = Arc::new(ManualProtocolReadClock::default());
        let advancing_clock = Arc::clone(&clock);
        let executor = executor
            .with_test_configuration(
                Some(faults.clone()),
                SqliteCatalogReadRetryPolicy::default(),
                Some(clock.clone()),
            )
            .with_stage_observer(Box::new(move |stage| {
                if stage == SqliteCatalogReadStage::Query {
                    advancing_clock.advance(Duration::from_millis(101));
                }
            }));

        let error = executor
            .execute_read(CatalogReadRetryOperation::CatalogSnapshot, read_one)
            .expect_err("single attempt deadline exhaustion");

        assert_eq!(faults.attempts(SqliteCatalogReadStage::Query), 1);
        assert!(clock.observed_sleeps().is_empty());
        assert_eq!(
            error.retry_details,
            Some(CatalogReadRetryDetails {
                operation: CatalogReadRetryOperation::CatalogSnapshot,
                attempts: 1,
                elapsed_ms: 101,
                cause: CatalogReadRetryCause::FileLockingProtocolFailed,
            })
        );
    }

    #[test]
    fn uncontended_production_read_has_one_attempt_and_zero_sleep() {
        let (_storage, executor) = fixture_executor();
        let clock = Arc::new(ManualProtocolReadClock::default());
        let executor = executor.with_test_configuration(
            None,
            SqliteCatalogReadRetryPolicy::default(),
            Some(clock.clone()),
        );

        assert_eq!(
            executor
                .execute_read(CatalogReadRetryOperation::CatalogSnapshot, read_one)
                .expect("uncontended read"),
            1,
        );
        assert!(clock.observed_sleeps().is_empty());
        assert_eq!(executor.attempt_stats().attempts, 1);
    }

    #[test]
    fn protocol_failure_during_connection_open_retries_with_a_fresh_attempt() {
        let (_storage, executor) = fixture_executor();
        let faults = SqliteCatalogReadFaultInjection::protocol(SqliteCatalogReadStage::Open, 2);
        let executor = executor.with_test_configuration(Some(faults.clone()), fast_policy(5), None);

        let result = executor
            .execute_read(CatalogReadRetryOperation::CatalogSnapshot, read_one)
            .expect("protocol open failures should recover");

        assert_eq!(result, 1);
        assert_eq!(faults.attempts(SqliteCatalogReadStage::Open), 3);
        assert_eq!(faults.attempts(SqliteCatalogReadStage::Validation), 1);
    }

    #[test]
    fn protocol_failure_during_fast_schema_validation_reopens_the_connection() {
        let (_storage, executor) = fixture_executor();
        let faults =
            SqliteCatalogReadFaultInjection::protocol(SqliteCatalogReadStage::Validation, 2);
        let executor = executor.with_test_configuration(Some(faults.clone()), fast_policy(5), None);

        let result = executor
            .execute_read(CatalogReadRetryOperation::CatalogSnapshot, read_one)
            .expect("protocol validation failures should recover");

        assert_eq!(result, 1);
        assert_eq!(faults.attempts(SqliteCatalogReadStage::Open), 3);
        assert_eq!(faults.attempts(SqliteCatalogReadStage::Validation), 3);
    }

    #[test]
    fn protocol_failure_during_query_reopens_the_connection() {
        let (_storage, executor) = fixture_executor();
        let faults = SqliteCatalogReadFaultInjection::protocol(SqliteCatalogReadStage::Query, 2);
        let executor = executor.with_test_configuration(Some(faults.clone()), fast_policy(5), None);

        let result = executor
            .execute_read(CatalogReadRetryOperation::CatalogSnapshot, read_one)
            .expect("protocol query failures should recover");

        assert_eq!(result, 1);
        assert_eq!(faults.attempts(SqliteCatalogReadStage::Open), 3);
        assert_eq!(faults.attempts(SqliteCatalogReadStage::Validation), 3);
        assert_eq!(faults.attempts(SqliteCatalogReadStage::Query), 3);
    }

    #[test]
    fn non_protocol_failure_is_not_retried() {
        let (_storage, executor) = fixture_executor();
        let faults = SqliteCatalogReadFaultInjection::new(
            SqliteCatalogReadStage::Query,
            1,
            ScanError::new("catalog_test_non_protocol", "injected non-protocol failure"),
        );
        let executor = executor.with_test_configuration(Some(faults.clone()), fast_policy(5), None);

        let error = executor
            .execute_read(CatalogReadRetryOperation::CatalogSnapshot, read_one)
            .expect_err("non-protocol error");

        assert_eq!(error.code, "catalog_test_non_protocol");
        assert_eq!(faults.attempts(SqliteCatalogReadStage::Open), 1);
        assert_eq!(faults.attempts(SqliteCatalogReadStage::Query), 1);
    }

    #[test]
    fn non_protocol_sqlite_failures_are_terminal_without_retry() {
        for (code, expected_code) in [
            (rusqlite::ffi::SQLITE_BUSY, "catalog_database_busy"),
            (rusqlite::ffi::SQLITE_LOCKED, "catalog_database_locked"),
            (rusqlite::ffi::SQLITE_READONLY, "catalog_database_error"),
            (rusqlite::ffi::SQLITE_CORRUPT, "catalog_database_error"),
            (rusqlite::ffi::SQLITE_IOERR, "catalog_database_error"),
        ] {
            let (_storage, executor) = fixture_executor();
            let faults = SqliteCatalogReadFaultInjection::sqlite(
                SqliteCatalogReadStage::Query,
                1,
                code,
                "injected non-protocol locking failure".to_owned(),
            );
            let executor =
                executor.with_test_configuration(Some(faults.clone()), fast_policy(5), None);

            let error = executor
                .execute_read(CatalogReadRetryOperation::CatalogSnapshot, read_one)
                .expect_err("non-protocol locking error");

            assert_eq!(error.code, expected_code);
            assert_eq!(faults.attempts(SqliteCatalogReadStage::Open), 1);
            assert_eq!(faults.attempts(SqliteCatalogReadStage::Query), 1);
        }
    }

    #[test]
    fn attempt_exhaustion_preserves_structured_protocol_diagnostics() {
        let (_storage, executor) = fixture_executor();
        let faults = SqliteCatalogReadFaultInjection::protocol(SqliteCatalogReadStage::Query, 10);
        let clock = Arc::new(ManualProtocolReadClock::default());
        let executor = executor.with_test_configuration(Some(faults), fast_policy(3), Some(clock));

        let error = executor
            .execute_read(CatalogReadRetryOperation::GalleryTimeline, read_one)
            .expect_err("protocol retry exhaustion");

        assert_eq!(error.code, "catalog_read_protocol_retry_exhausted");
        assert_eq!(
            error.retry_details,
            Some(CatalogReadRetryDetails {
                operation: CatalogReadRetryOperation::GalleryTimeline,
                attempts: 3,
                elapsed_ms: 0,
                cause: CatalogReadRetryCause::FileLockingProtocolFailed,
            })
        );
        assert!(!error.message.contains("operation="));
        assert!(!error.message.contains(storage_path_fragment()));
    }

    #[test]
    fn deadline_exhaustion_is_bounded_and_diagnostic() {
        let (_storage, executor) = fixture_executor();
        let faults = SqliteCatalogReadFaultInjection::protocol(SqliteCatalogReadStage::Query, 20);
        let clock = Arc::new(ManualProtocolReadClock::default());
        let executor = executor.with_test_configuration(
            Some(faults.clone()),
            SqliteCatalogReadRetryPolicy {
                max_attempts: 20,
                deadline: Duration::from_millis(1),
                initial_backoff: Duration::from_millis(5),
            },
            Some(clock.clone()),
        );

        let error = executor
            .execute_read(CatalogReadRetryOperation::GalleryLayoutManifest, read_one)
            .expect_err("deadline exhaustion");

        assert_eq!(error.code, "catalog_read_protocol_retry_exhausted");
        assert!(faults.attempts(SqliteCatalogReadStage::Query) < 20);
        assert_eq!(clock.observed_sleeps(), vec![Duration::from_millis(1)]);
        assert_eq!(
            error.retry_details,
            Some(CatalogReadRetryDetails {
                operation: CatalogReadRetryOperation::GalleryLayoutManifest,
                attempts: 1,
                elapsed_ms: 1,
                cause: CatalogReadRetryCause::FileLockingProtocolFailed,
            })
        );
    }

    #[test]
    fn retry_backoff_holds_neither_read_transaction_nor_write_admission() {
        let (_storage, executor) = fixture_executor();
        let session = executor.session.clone();
        let calls = Arc::new(AtomicUsize::new(0));
        let read_calls = Arc::clone(&calls);
        let sleep_calls = Arc::new(AtomicUsize::new(0));
        let observed_sleeps = Arc::clone(&sleep_calls);
        let clock = Arc::new(ManualProtocolReadClock::with_sleep_observer(Arc::new(
            move |_| {
                observed_sleeps.fetch_add(1, Ordering::SeqCst);
                let mut writer = session
                    .open_in_lane(LibraryChangeLane::Recovery)
                    .expect("fresh writer connection during backoff");
                let transaction = writer
                    .begin_write()
                    .expect("write admission during backoff");
                transaction.commit().expect("release write transaction");
            },
        )));
        let executor = executor.with_test_configuration(
            None,
            SqliteCatalogReadRetryPolicy {
                max_attempts: 3,
                deadline: TEST_ATTEMPT_LIMIT_DEADLINE,
                initial_backoff: Duration::from_millis(1),
            },
            Some(clock),
        );

        let result = executor
            .execute_read(CatalogReadRetryOperation::CatalogSnapshot, move |catalog| {
                let attempt = read_calls.fetch_add(1, Ordering::SeqCst);
                if attempt == 0 {
                    let transaction = catalog
                        .connection
                        .transaction()
                        .map_err(super::super::database_error)?;
                    transaction
                        .query_row("SELECT 1", [], |row| row.get::<_, i64>(0))
                        .map_err(super::super::database_error)?;
                    return Err(protocol_error());
                }
                read_one(catalog)
            })
            .expect("retry after releasing read state");

        assert_eq!(result, 1);
        assert_eq!(calls.load(Ordering::SeqCst), 2);
        assert_eq!(sleep_calls.load(Ordering::SeqCst), 1);
    }

    fn storage_path_fragment() -> &'static str {
        if cfg!(windows) {
            "\\catalog\\"
        } else {
            "/catalog/"
        }
    }

    #[test]
    fn fixture_uses_bundled_wal_catalog() {
        let (storage, executor) = fixture_executor();
        let path = executor.session.path().to_path_buf();
        assert!(path.starts_with(storage.path()));
        let connection = Connection::open(path).expect("catalog connection");
        let journal_mode: String = connection
            .query_row("PRAGMA journal_mode", [], |row| row.get(0))
            .expect("journal mode");
        assert_eq!(journal_mode.to_ascii_lowercase(), "wal");
        let mut statement = connection
            .prepare("PRAGMA compile_options")
            .expect("SQLite compile options");
        let options = statement
            .query_map([], |row| row.get::<_, String>(0))
            .expect("query SQLite compile options")
            .collect::<rusqlite::Result<Vec<_>>>()
            .expect("collect SQLite compile options");
        assert!(
            options
                .iter()
                .any(|option| option == "ENABLE_SETLK_TIMEOUT"),
            "bundled SQLite must enable its supported blocking WAL lock path"
        );
    }

    #[test]
    fn read_attempt_uses_bounded_timeout_and_query_only_mode() {
        let (_storage, executor) = fixture_executor();

        let (busy_timeout, query_only): (i64, i64) = executor
            .execute_read(CatalogReadRetryOperation::CatalogSnapshot, |catalog| {
                let busy_timeout = catalog
                    .connection
                    .query_row("PRAGMA busy_timeout", [], |row| row.get(0))
                    .map_err(super::super::database_error)?;
                let query_only = catalog
                    .connection
                    .query_row("PRAGMA query_only", [], |row| row.get(0))
                    .map_err(super::super::database_error)?;
                Ok((busy_timeout, query_only))
            })
            .expect("read-only connection configuration");

        assert_eq!(busy_timeout, 100);
        assert_eq!(query_only, 1);
    }

    #[test]
    fn existing_catalog_validation_retries_without_running_migrations() {
        let (storage, executor) = fixture_executor();
        let path = executor.session.path().to_path_buf();
        crate::adapters::reset_full_schema_validation_count(&path);
        drop(executor);
        let faults =
            SqliteCatalogReadFaultInjection::protocol(SqliteCatalogReadStage::Validation, 2);

        let executor = SqliteCatalogReadExecutor::open_existing_with_test_configuration(
            path.clone(),
            faults.clone(),
            fast_policy(5),
            None,
        )
        .expect("read-only existing schema validation should retry");

        assert!(executor.session.path().starts_with(storage.path()));
        assert_eq!(faults.attempts(SqliteCatalogReadStage::Open), 3);
        assert_eq!(faults.attempts(SqliteCatalogReadStage::Validation), 3);
        assert_eq!(crate::adapters::full_schema_validation_count(&path), 0);
    }

    #[test]
    fn existing_catalog_reader_never_creates_or_migrates_a_missing_database() {
        let storage = tempfile::tempdir().expect("storage");
        let path = storage.path().join("missing").join("ame.sqlite3");
        crate::adapters::reset_full_schema_validation_count(&path);

        let error = SqliteCatalogReadExecutor::open_existing(path.clone())
            .err()
            .expect("missing catalog must require explicit application preparation");

        assert_eq!(error.code, "catalog_identity_unavailable");
        assert!(!path.exists());
        assert_eq!(crate::adapters::full_schema_validation_count(&path), 0);
    }

    #[test]
    fn ordinary_scan_errors_remain_bridge_compatible_without_retry_details() {
        let error = ScanError::new("ordinary_failure", "ordinary failure message");

        assert_eq!(error.code, "ordinary_failure");
        assert_eq!(error.message, "ordinary failure message");
        assert!(error.retry_details.is_none());
    }

    #[test]
    fn scan_error_bridge_serialization_preserves_optional_typed_retry_details() {
        let ordinary = encoded_scan_error(ScanError::new("c", "m"));
        let mut expected_ordinary = Vec::new();
        expected_ordinary.extend_from_slice(&1_i32.to_ne_bytes());
        expected_ordinary.push(b'c');
        expected_ordinary.extend_from_slice(&1_i32.to_ne_bytes());
        expected_ordinary.push(b'm');
        expected_ordinary.push(0);
        assert_eq!(ordinary, expected_ordinary);

        let structured = encoded_scan_error(ScanError::new("c", "m").with_retry_details(
            CatalogReadRetryDetails {
                operation: CatalogReadRetryOperation::GalleryTimeline,
                attempts: 5,
                elapsed_ms: 100,
                cause: CatalogReadRetryCause::FileLockingProtocolFailed,
            },
        ));
        let mut expected_structured = expected_ordinary;
        *expected_structured.last_mut().expect("optional marker") = 1;
        expected_structured.extend_from_slice(&4_i32.to_ne_bytes());
        expected_structured.extend_from_slice(&5_u32.to_ne_bytes());
        expected_structured.extend_from_slice(&100_u64.to_ne_bytes());
        expected_structured.extend_from_slice(&0_i32.to_ne_bytes());
        assert_eq!(structured, expected_structured);
    }

    #[test]
    fn completed_persistent_journal_range_observer_uses_the_typed_read_owner() {
        let (_storage, executor) = fixture_executor();

        assert!(
            !executor
                .persistent_journal_range_is_completed_for_test("root", 10, 11)
                .expect("typed persistent-journal observer")
        );
        assert_eq!(
            executor.attempt_stats(),
            SqliteCatalogReadAttemptStats {
                operations: 1,
                attempts: 1,
                protocol_retries: 0,
                maximum_attempts: 1,
            }
        );
    }

    #[test]
    fn attempt_stats_distinguish_operations_from_protocol_retries() {
        let (_storage, executor) = fixture_executor();
        let faults = SqliteCatalogReadFaultInjection::protocol(SqliteCatalogReadStage::Query, 2);
        let executor = executor.with_test_configuration(Some(faults), fast_policy(5), None);

        executor
            .execute_read(CatalogReadRetryOperation::CatalogSnapshot, read_one)
            .expect("retried read");
        executor
            .execute_read(CatalogReadRetryOperation::CatalogSnapshot, read_one)
            .expect("uncontended read");

        assert_eq!(
            executor.attempt_stats(),
            SqliteCatalogReadAttemptStats {
                operations: 2,
                attempts: 4,
                protocol_retries: 2,
                maximum_attempts: 3,
            }
        );
    }
}
