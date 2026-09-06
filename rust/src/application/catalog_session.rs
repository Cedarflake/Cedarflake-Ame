use std::collections::HashMap;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, OnceLock};
use std::time::{Duration, Instant};

use crate::adapters::{SqliteCatalog, SqliteCatalogReadExecutor, SqliteCatalogSession};
use crate::domain::{LibraryChangeLane, ScanError};

const VALIDATION_FAILURE_BACKOFF: Duration = Duration::from_millis(250);

static CATALOG_SESSIONS: OnceLock<Mutex<CatalogSessionRegistry>> = OnceLock::new();

#[derive(Default)]
struct CatalogSessionRegistry {
    entries: HashMap<PathBuf, Arc<CatalogSessionEntry>>,
}

struct CatalogSessionEntry {
    state: Mutex<CatalogSessionState>,
}

enum CatalogSessionState {
    Empty,
    Validating(Arc<CatalogValidationFlight>),
    Ready(Arc<SqliteCatalogSession>),
    Failed {
        error: ScanError,
        retry_after: Instant,
    },
}

#[derive(Default)]
struct CatalogValidationFlight {
    result: Mutex<Option<Result<Arc<SqliteCatalogSession>, ScanError>>>,
    result_ready: Condvar,
    invalidated: AtomicBool,
}

impl CatalogValidationFlight {
    fn wait(&self) -> Result<Arc<SqliteCatalogSession>, ScanError> {
        let mut result = self.result.lock().map_err(|_| registry_unavailable())?;
        loop {
            if let Some(result) = &*result {
                return result.clone();
            }
            result = self
                .result_ready
                .wait(result)
                .map_err(|_| registry_unavailable())?;
        }
    }

    fn complete(
        &self,
        result: Result<Arc<SqliteCatalogSession>, ScanError>,
    ) -> Result<(), ScanError> {
        let mut stored = self.result.lock().map_err(|_| registry_unavailable())?;
        if stored.is_none() {
            *stored = Some(result);
            self.result_ready.notify_all();
        }
        Ok(())
    }
}

impl Default for CatalogSessionEntry {
    fn default() -> Self {
        Self {
            state: Mutex::new(CatalogSessionState::Empty),
        }
    }
}

pub(super) fn open_catalog(
    path: &Path,
    lane: LibraryChangeLane,
) -> Result<SqliteCatalog, ScanError> {
    let session = validated_catalog_session(path)?;
    match session.open_in_lane(lane) {
        Ok(catalog) => Ok(catalog),
        Err(error) if error.code == "catalog_validated_session_stale" => {
            refresh_stale_catalog_session(path, &session)?.open_in_lane(lane)
        }
        Err(error) => Err(error),
    }
}

pub(super) fn open_catalog_for_foreground_scan(
    path: &Path,
    lane: LibraryChangeLane,
    scan_id: &str,
) -> Result<SqliteCatalog, ScanError> {
    loop {
        let session = validated_catalog_session(path)?;
        match session.open_in_lane(lane) {
            Ok(catalog) => {
                let entry = catalog_session_entry(path)?;
                let state = entry.state.lock().map_err(|_| registry_unavailable())?;
                if matches!(
                    &*state,
                    CatalogSessionState::Ready(current) if Arc::ptr_eq(current, &session)
                ) {
                    super::scan_library::protect_scan_catalog_session(scan_id, path)?;
                    return Ok(catalog);
                }
            }
            Err(error) if error.code == "catalog_validated_session_stale" => {
                refresh_stale_catalog_session_for_scan(path, &session, scan_id)?;
            }
            Err(error) => return Err(error),
        }
    }
}

pub(super) fn open_catalog_reader(path: &Path) -> Result<SqliteCatalogReadExecutor, ScanError> {
    open_catalog(path, LibraryChangeLane::Recovery)
        .map(|catalog| SqliteCatalogReadExecutor::new(catalog.validated_session()))
}

pub(super) fn with_recovery_catalog_if_idle<T>(
    path: &Path,
    operation: impl FnOnce(&mut SqliteCatalog) -> Result<T, ScanError>,
) -> Result<Option<T>, ScanError> {
    let mut operation = Some(operation);
    loop {
        if super::scan_library::catalog_has_active_scan(path, None)? {
            return Ok(None);
        }
        let session = validated_catalog_session(path)?;
        let entry = catalog_session_entry(path)?;
        let state = entry.state.lock().map_err(|_| registry_unavailable())?;
        if !matches!(
            &*state,
            CatalogSessionState::Ready(current) if Arc::ptr_eq(current, &session)
        ) {
            continue;
        }
        if super::scan_library::catalog_has_active_scan(path, None)? {
            return Ok(None);
        }
        match session.open_in_lane(LibraryChangeLane::Recovery) {
            Ok(mut catalog) => {
                let operation = operation.take().ok_or_else(registry_unavailable)?;
                return operation(&mut catalog).map(Some);
            }
            Err(error) if error.code == "catalog_validated_session_stale" => {
                drop(state);
                match refresh_stale_catalog_session(path, &session) {
                    Ok(_) => {}
                    Err(error)
                        if error.code == "catalog_validated_session_stale_while_scan_active" =>
                    {
                        return Ok(None);
                    }
                    Err(error) => return Err(error),
                }
            }
            Err(error) => return Err(error),
        }
    }
}

pub(super) fn validated_catalog_session(
    path: &Path,
) -> Result<Arc<SqliteCatalogSession>, ScanError> {
    validated_catalog_session_with(path, None, SqliteCatalogSession::validate)
}

pub(super) fn refresh_stale_catalog_session(
    path: &Path,
    stale_session: &Arc<SqliteCatalogSession>,
) -> Result<Arc<SqliteCatalogSession>, ScanError> {
    validated_catalog_session_with(path, Some(stale_session), SqliteCatalogSession::validate)
}

fn refresh_stale_catalog_session_for_scan(
    path: &Path,
    stale_session: &Arc<SqliteCatalogSession>,
    scan_id: &str,
) -> Result<Arc<SqliteCatalogSession>, ScanError> {
    validated_catalog_session_with_exemption(
        path,
        Some(stale_session),
        Some(scan_id),
        SqliteCatalogSession::validate,
    )
}

fn validated_catalog_session_with<F>(
    path: &Path,
    stale_session: Option<&Arc<SqliteCatalogSession>>,
    validate: F,
) -> Result<Arc<SqliteCatalogSession>, ScanError>
where
    F: FnOnce(PathBuf) -> Result<SqliteCatalogSession, ScanError>,
{
    validated_catalog_session_with_exemption(path, stale_session, None, validate)
}

fn validated_catalog_session_with_exemption<F>(
    path: &Path,
    stale_session: Option<&Arc<SqliteCatalogSession>>,
    exempt_scan_id: Option<&str>,
    validate: F,
) -> Result<Arc<SqliteCatalogSession>, ScanError>
where
    F: FnOnce(PathBuf) -> Result<SqliteCatalogSession, ScanError>,
{
    let entry = catalog_session_entry(path)?;
    let decision = {
        let mut state = entry.state.lock().map_err(|_| registry_unavailable())?;
        match &*state {
            CatalogSessionState::Ready(session)
                if stale_session.is_none_or(|stale| !Arc::ptr_eq(session, stale)) =>
            {
                CatalogValidationDecision::Ready(Arc::clone(session))
            }
            CatalogSessionState::Validating(flight) => {
                CatalogValidationDecision::Wait(Arc::clone(flight))
            }
            CatalogSessionState::Failed { error, retry_after } if Instant::now() < *retry_after => {
                CatalogValidationDecision::Failed(error.clone())
            }
            CatalogSessionState::Empty
            | CatalogSessionState::Ready(_)
            | CatalogSessionState::Failed { .. } => {
                if super::scan_library::catalog_has_active_scan(path, exempt_scan_id)? {
                    return Err(ScanError::new(
                        "catalog_validated_session_stale_while_scan_active",
                        "Catalog session renewal is deferred until active foreground scans finish",
                    ));
                }
                let flight = Arc::new(CatalogValidationFlight::default());
                *state = CatalogSessionState::Validating(Arc::clone(&flight));
                CatalogValidationDecision::Validate(flight)
            }
        }
    };

    let flight = match decision {
        CatalogValidationDecision::Ready(session) => return Ok(session),
        CatalogValidationDecision::Failed(error) => return Err(error),
        CatalogValidationDecision::Wait(flight) => return flight.wait(),
        CatalogValidationDecision::Validate(flight) => flight,
    };

    let validation = match catch_unwind(AssertUnwindSafe(|| validate(path.to_path_buf()))) {
        Ok(result) => result.map(Arc::new),
        Err(_) => Err(ScanError::new(
            "catalog_session_validation_panicked",
            "Catalog session validation stopped unexpectedly",
        )),
    };
    let installed = match entry.state.lock() {
        Ok(mut state) => {
            if !matches!(
                &*state,
                CatalogSessionState::Validating(current) if Arc::ptr_eq(current, &flight)
            ) {
                Err(registry_unavailable())
            } else if flight.invalidated.load(Ordering::Acquire) {
                *state = CatalogSessionState::Empty;
                Err(ScanError::new(
                    "catalog_validated_session_stale",
                    "The catalog changed while its reusable session was being validated",
                ))
            } else {
                match &validation {
                    Ok(session) => {
                        *state = CatalogSessionState::Ready(Arc::clone(session));
                    }
                    Err(error) => {
                        *state = CatalogSessionState::Failed {
                            error: error.clone(),
                            retry_after: Instant::now() + VALIDATION_FAILURE_BACKOFF,
                        };
                    }
                }
                validation
            }
        }
        Err(_) => Err(registry_unavailable()),
    };
    flight.complete(installed.clone())?;
    installed
}

enum CatalogValidationDecision {
    Ready(Arc<SqliteCatalogSession>),
    Failed(ScanError),
    Wait(Arc<CatalogValidationFlight>),
    Validate(Arc<CatalogValidationFlight>),
}

fn catalog_session_entry(path: &Path) -> Result<Arc<CatalogSessionEntry>, ScanError> {
    let mut registry = catalog_session_registry()
        .lock()
        .map_err(|_| registry_unavailable())?;
    if let Some(entry) = registry.entries.get(path) {
        return Ok(Arc::clone(entry));
    }

    let entry = Arc::new(CatalogSessionEntry::default());
    registry
        .entries
        .insert(path.to_path_buf(), Arc::clone(&entry));
    Ok(entry)
}

fn catalog_session_registry() -> &'static Mutex<CatalogSessionRegistry> {
    CATALOG_SESSIONS.get_or_init(|| Mutex::new(CatalogSessionRegistry::default()))
}

pub(super) fn invalidate_catalog_session(path: &Path) -> Result<(), ScanError> {
    let entry = catalog_session_registry()
        .lock()
        .map_err(|_| registry_unavailable())?
        .entries
        .get(path)
        .cloned();
    if let Some(entry) = entry {
        let mut state = entry.state.lock().map_err(|_| registry_unavailable())?;
        match &*state {
            CatalogSessionState::Validating(flight) => {
                flight.invalidated.store(true, Ordering::Release);
            }
            _ => *state = CatalogSessionState::Empty,
        }
    }
    Ok(())
}

fn registry_unavailable() -> ScanError {
    ScanError::new(
        "catalog_session_registry_unavailable",
        "The reusable catalog session registry is poisoned",
    )
}

#[cfg(test)]
pub(super) fn reset_catalog_session(path: &Path) {
    invalidate_catalog_session(path).expect("catalog session registry");
}

#[cfg(test)]
mod tests {
    use std::sync::{Barrier, mpsc};
    use std::thread;

    use rusqlite::Connection;
    use tempfile::tempdir;

    use super::*;

    #[test]
    fn stale_catalog_session_is_revalidated_once_and_reused() {
        let directory = tempdir().expect("catalog directory");
        let path = directory.path().join("catalog").join("ame.sqlite3");
        drop(SqliteCatalog::open(path.clone()).expect("initialize catalog"));
        reset_catalog_session(&path);
        crate::adapters::reset_full_schema_validation_count(&path);

        drop(open_catalog(&path, LibraryChangeLane::Recovery).expect("first cached open"));
        Connection::open(&path)
            .expect("schema-cookie writer")
            .execute_batch("PRAGMA schema_version = 4242")
            .expect("invalidate cached schema cookie");
        drop(open_catalog(&path, LibraryChangeLane::Recovery).expect("revalidated open"));
        drop(open_catalog(&path, LibraryChangeLane::Recovery).expect("reused replacement"));

        assert_eq!(
            crate::adapters::full_schema_validation_count(&path),
            2,
            "one initial validation and one stale-session replacement are expected",
        );
    }

    #[test]
    fn invalidation_retains_the_entry_and_rejects_an_in_flight_validation() {
        let directory = tempdir().expect("catalog directory");
        let path = directory.path().join("ame.sqlite3");
        let entry = catalog_session_entry(&path).expect("stable session entry");
        let (started_tx, started_rx) = mpsc::sync_channel(0);
        let (release_tx, release_rx) = mpsc::sync_channel(0);
        let owner = {
            let path = path.clone();
            thread::spawn(move || {
                validated_catalog_session_with(&path, None, |path| {
                    let session = SqliteCatalogSession::validate(path)?;
                    started_tx.send(()).expect("announce validated flight");
                    release_rx.recv().expect("release validation flight");
                    Ok(session)
                })
            })
        };
        started_rx.recv().expect("validation is in flight");
        let flight = match &*entry.state.lock().expect("session state") {
            CatalogSessionState::Validating(flight) => Arc::clone(flight),
            _ => panic!("validation must remain in flight"),
        };
        let waiter = thread::spawn(move || flight.wait());

        invalidate_catalog_session(&path).expect("invalidate the current flight");
        let retained = catalog_session_entry(&path).expect("retained session entry");
        assert!(Arc::ptr_eq(&entry, &retained));
        assert!(matches!(
            &*retained.state.lock().expect("retained session state"),
            CatalogSessionState::Validating(_),
        ));
        release_tx.send(()).expect("finish invalidated validation");
        let owner_error = owner
            .join()
            .expect("validation owner")
            .err()
            .expect("invalidated success must not become ready");
        let waiter_error = waiter
            .join()
            .expect("validation waiter")
            .err()
            .expect("waiter must share the invalidated flight");
        assert_eq!(owner_error.code, "catalog_validated_session_stale");
        assert_eq!(waiter_error.code, owner_error.code);
        assert!(matches!(
            &*entry.state.lock().expect("invalidated state"),
            CatalogSessionState::Empty,
        ));

        drop(validated_catalog_session(&path).expect("fresh validation after invalidation"));
        assert!(Arc::ptr_eq(
            &entry,
            &catalog_session_entry(&path).expect("same owner after renewal"),
        ));
        assert_eq!(crate::adapters::full_schema_validation_count(&path), 2);
    }

    #[test]
    fn concurrent_validation_failure_is_single_flight_and_shared() {
        const CALLERS: usize = 8;

        let directory = tempdir().expect("catalog directory");
        let path = directory.path().join("catalog").join("ame.sqlite3");
        drop(SqliteCatalog::open(path.clone()).expect("initialize catalog"));
        let connection = Connection::open(&path).expect("schema writer");
        let valid_schema_version: i64 = connection
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .expect("read schema version");
        connection
            .execute_batch("PRAGMA user_version = 999")
            .expect("make schema unsupported");
        drop(connection);
        reset_catalog_session(&path);
        crate::adapters::reset_full_schema_validation_count(&path);
        let start = Arc::new(Barrier::new(CALLERS));

        let callers = (0..CALLERS)
            .map(|_| {
                let path = path.clone();
                let start = Arc::clone(&start);
                thread::spawn(move || {
                    start.wait();
                    validated_catalog_session(&path)
                        .err()
                        .expect("unsupported schema must fail")
                })
            })
            .collect::<Vec<_>>();
        let errors = callers
            .into_iter()
            .map(|caller| caller.join().expect("validation caller"))
            .collect::<Vec<_>>();

        assert_eq!(crate::adapters::full_schema_validation_count(&path), 1);
        for error in &errors[1..] {
            assert_eq!(error.code, errors[0].code);
            assert_eq!(error.message, errors[0].message);
            assert_eq!(error.retry_details, errors[0].retry_details);
        }

        Connection::open(&path)
            .expect("schema repair writer")
            .execute_batch(&format!("PRAGMA user_version = {valid_schema_version}"))
            .expect("repair schema version");
    }

    #[test]
    fn concurrent_recovery_after_backoff_revalidates_once() {
        const CALLERS: usize = 8;

        let directory = tempdir().expect("catalog directory");
        let path = directory.path().join("catalog").join("ame.sqlite3");
        drop(SqliteCatalog::open(path.clone()).expect("initialize catalog"));
        let connection = Connection::open(&path).expect("schema writer");
        let valid_schema_version: i64 = connection
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .expect("read schema version");
        connection
            .execute_batch("PRAGMA user_version = 999")
            .expect("make schema unsupported");
        drop(connection);
        reset_catalog_session(&path);
        crate::adapters::reset_full_schema_validation_count(&path);
        validated_catalog_session(&path)
            .err()
            .expect("prime cached validation failure");
        Connection::open(&path)
            .expect("schema repair writer")
            .execute_batch(&format!("PRAGMA user_version = {valid_schema_version}"))
            .expect("repair schema version");
        thread::sleep(VALIDATION_FAILURE_BACKOFF + Duration::from_millis(50));
        let start = Arc::new(Barrier::new(CALLERS));

        let callers = (0..CALLERS)
            .map(|_| {
                let path = path.clone();
                let start = Arc::clone(&start);
                thread::spawn(move || {
                    start.wait();
                    validated_catalog_session(&path).expect("recovered validation")
                })
            })
            .collect::<Vec<_>>();
        for caller in callers {
            drop(caller.join().expect("recovery caller"));
        }

        assert_eq!(
            crate::adapters::full_schema_validation_count(&path),
            2,
            "one failed validation and one shared recovery validation are expected",
        );
    }

    #[test]
    fn delayed_waiter_observes_its_original_failed_flight_after_retry() {
        let directory = tempdir().expect("catalog directory");
        let path = directory.path().join("catalog").join("ame.sqlite3");
        reset_catalog_session(&path);
        let original_error = ScanError::new(
            "catalog_original_validation_failed",
            "The original validation failed",
        );
        let replacement_error = ScanError::new(
            "catalog_replacement_validation_failed",
            "The replacement validation failed",
        );
        let (started_tx, started_rx) = mpsc::sync_channel(0);
        let (release_tx, release_rx) = mpsc::sync_channel(0);
        let owner = {
            let path = path.clone();
            let original_error = original_error.clone();
            thread::spawn(move || {
                validated_catalog_session_with(&path, None, |_| {
                    started_tx.send(()).expect("announce validation owner");
                    release_rx.recv().expect("release validation owner");
                    Err(original_error)
                })
            })
        };
        started_rx.recv().expect("validation owner started");
        let entry = catalog_session_entry(&path).expect("catalog session entry");
        let original_flight = {
            let state = entry.state.lock().expect("catalog session state");
            match &*state {
                CatalogSessionState::Validating(flight) => Arc::clone(flight),
                _ => panic!("the original validation flight must be active"),
            }
        };
        let delayed_result_guard = original_flight
            .result
            .lock()
            .expect("hold original flight result publication");
        let waiter = {
            let original_flight = Arc::clone(&original_flight);
            thread::spawn(move || original_flight.wait())
        };
        release_tx.send(()).expect("release validation owner");

        let observation_deadline = Instant::now() + Duration::from_secs(5);
        let retry_after = loop {
            let state = entry.state.lock().expect("catalog session state");
            if let CatalogSessionState::Failed { retry_after, .. } = &*state {
                break *retry_after;
            }
            assert!(
                Instant::now() < observation_deadline,
                "the original failure was not installed"
            );
            drop(state);
            thread::yield_now();
        };
        let retry_delay = retry_after.saturating_duration_since(Instant::now());
        if !retry_delay.is_zero() {
            thread::sleep(retry_delay + Duration::from_millis(10));
        }

        let replacement = validated_catalog_session_with(&path, None, {
            let replacement_error = replacement_error.clone();
            move |_| Err(replacement_error)
        })
        .err()
        .expect("the replacement validation must fail independently");
        drop(delayed_result_guard);
        let owner_result = owner
            .join()
            .expect("validation owner")
            .err()
            .expect("the original validation must fail");
        let waiter_result = waiter
            .join()
            .expect("validation waiter")
            .err()
            .expect("the original waiter must share the original failure");

        assert_eq!(owner_result.code, original_error.code);
        assert_eq!(waiter_result.code, original_error.code);
        assert_eq!(replacement.code, replacement_error.code);
    }

    #[test]
    fn validation_panic_completes_waiters_and_allows_bounded_retry() {
        let directory = tempdir().expect("catalog directory");
        let path = directory.path().join("catalog").join("ame.sqlite3");
        reset_catalog_session(&path);

        let error =
            validated_catalog_session_with(&path, None, |_| panic!("injected validation panic"))
                .err()
                .expect("a validation panic must become a structured failure");
        assert_eq!(error.code, "catalog_session_validation_panicked");
        let cached = validated_catalog_session_with(&path, None, |_| {
            panic!("the backoff must reuse the structured failure")
        })
        .err()
        .expect("the validation failure remains cached during backoff");
        assert_eq!(cached.code, error.code);

        thread::sleep(VALIDATION_FAILURE_BACKOFF + Duration::from_millis(10));
        let recovered = validated_catalog_session_with(&path, None, |_| {
            Err(ScanError::new(
                "catalog_retry_reached",
                "The bounded retry was admitted",
            ))
        })
        .err()
        .expect("the retry validator must run after backoff");
        assert_eq!(recovered.code, "catalog_retry_reached");
    }

    #[test]
    fn validation_for_one_path_does_not_hold_the_registry_lock() {
        let directory = tempdir().expect("catalog directory");
        let blocked_path = directory.path().join("blocked").join("ame.sqlite3");
        let other_path = directory.path().join("other").join("ame.sqlite3");
        drop(SqliteCatalog::open(blocked_path.clone()).expect("initialize blocked catalog"));
        drop(SqliteCatalog::open(other_path.clone()).expect("initialize other catalog"));
        reset_catalog_session(&blocked_path);
        reset_catalog_session(&other_path);
        let (blocked_tx, blocked_rx) = mpsc::sync_channel(0);
        let (release_tx, release_rx) = mpsc::sync_channel(0);
        let blocked_thread = {
            let blocked_path = blocked_path.clone();
            thread::spawn(move || {
                validated_catalog_session_with(&blocked_path, None, |path| {
                    blocked_tx.send(()).expect("announce blocked validation");
                    release_rx.recv().expect("release blocked validation");
                    SqliteCatalogSession::validate(path)
                })
            })
        };
        blocked_rx.recv().expect("blocked validation started");
        let (other_tx, other_rx) = mpsc::sync_channel(0);
        let other_thread = thread::spawn(move || {
            let result = validated_catalog_session(&other_path);
            other_tx
                .send(result.is_ok())
                .expect("report other validation");
            result
        });

        assert!(
            other_rx
                .recv_timeout(Duration::from_secs(5))
                .expect("other path must finish before the blocked path is released"),
        );
        release_tx.send(()).expect("release blocked validation");
        drop(
            blocked_thread
                .join()
                .expect("blocked validation thread")
                .expect("blocked validation succeeds"),
        );
        drop(
            other_thread
                .join()
                .expect("other validation thread")
                .expect("other validation succeeds"),
        );
    }
}
