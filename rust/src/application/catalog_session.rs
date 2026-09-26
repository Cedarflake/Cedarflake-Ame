use std::collections::HashMap;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, OnceLock};
use std::time::{Duration, Instant};

use crate::adapters::{SqliteCatalog, SqliteCatalogReadExecutor, SqliteCatalogSession};
use crate::domain::{LibraryChangeLane, ScanError};

mod maintenance;

use maintenance::CatalogMaintenanceFlight;
pub(super) use maintenance::with_catalog_maintenance;

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
    Maintaining(Arc<CatalogMaintenanceFlight>),
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
        let session = validated_catalog_session_with_exemption(
            path,
            None,
            Some(scan_id),
            SqliteCatalogSession::validate,
        )?;
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
    let decision = loop {
        let mut state = entry.state.lock().map_err(|_| registry_unavailable())?;
        let decision = match &*state {
            CatalogSessionState::Maintaining(flight) => {
                let flight = Arc::clone(flight);
                drop(state);
                if exempt_scan_id.is_some() {
                    super::catalog_reclamation::preempt_catalog_reclamation(path);
                }
                flight.wait()?;
                continue;
            }
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
        };
        break decision;
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

#[cfg(test)]
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
            CatalogSessionState::Maintaining(flight) => flight.invalidate(),
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
mod tests;
