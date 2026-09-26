use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use crate::domain::{CatalogReclamationPhase, CatalogReclamationSnapshot, ScanError};
use crate::ports::CatalogMaintenanceControl;

static REGISTRY: OnceLock<CatalogReclamationRegistry> = OnceLock::new();

pub(super) fn registry() -> &'static CatalogReclamationRegistry {
    REGISTRY.get_or_init(CatalogReclamationRegistry::default)
}

#[derive(Default)]
pub(super) struct CatalogReclamationRegistry {
    entries: Mutex<HashMap<PathBuf, Arc<Entry>>>,
}

pub(super) struct ReclamationAdmission {
    pub snapshot: CatalogReclamationSnapshot,
    pub worker: Option<CatalogReclamationOperation>,
}

struct Entry {
    state: Mutex<State>,
}

struct State {
    snapshot: CatalogReclamationSnapshot,
    // Admission and retirement share this state; a display phase never proves worker ownership.
    run: Option<Arc<RunToken>>,
    pending_operation_id: Option<String>,
    active_control: Option<CatalogMaintenanceControl>,
}

struct RunToken {
    cancelled: Arc<AtomicBool>,
}

#[derive(Clone)]
pub(super) struct CatalogReclamationOperation {
    entry: Arc<Entry>,
    token: Arc<RunToken>,
}

#[derive(Clone, Copy)]
pub(super) enum ReclamationCompletion {
    Completed,
    Cancelled,
}

impl CatalogReclamationRegistry {
    pub fn admit(
        &self,
        key: PathBuf,
        operation_id: String,
        replace_terminal: bool,
    ) -> Result<ReclamationAdmission, ScanError> {
        let mut entries = self.entries.lock().map_err(|_| registry_error())?;
        let is_new = !entries.contains_key(&key);
        let entry = entries.entry(key).or_insert_with(|| {
            Arc::new(Entry {
                state: Mutex::new(State {
                    snapshot: CatalogReclamationSnapshot::idle(),
                    run: None,
                    pending_operation_id: None,
                    active_control: None,
                }),
            })
        });
        let mut state = entry.state.lock().map_err(|_| registry_error())?;
        let worker = if state.run.is_some() {
            if replace_terminal {
                state.pending_operation_id = Some(operation_id);
            }
            None
        } else if is_new || replace_terminal {
            Some(state.start(Arc::clone(entry), operation_id))
        } else {
            None
        };
        Ok(ReclamationAdmission {
            snapshot: state.snapshot.clone(),
            worker,
        })
    }

    pub fn snapshot(&self, key: &Path) -> Result<Option<CatalogReclamationSnapshot>, ScanError> {
        let entry = self
            .entries
            .lock()
            .map_err(|_| registry_error())?
            .get(key)
            .cloned();
        entry.map(|entry| entry.snapshot()).transpose()
    }

    pub fn preempt(&self, key: &Path) {
        let entry = self
            .entries
            .lock()
            .ok()
            .and_then(|entries| entries.get(key).cloned());
        let control = entry.and_then(|entry| {
            entry
                .state
                .lock()
                .ok()
                .and_then(|state| state.active_control.clone())
        });
        if let Some(control) = control {
            // The retained attempt is safe to interrupt after retirement; it cannot target its successor.
            control.preempt();
        }
    }

    pub fn cancel(&self, operation_id: &str) -> bool {
        let control = self.entries.lock().ok().and_then(|entries| {
            entries.values().find_map(|entry| {
                let state = entry.state.lock().ok()?;
                if state.snapshot.operation_id.as_deref() != Some(operation_id) {
                    return None;
                }
                let run = state.run.as_ref()?;
                run.cancelled.store(true, Ordering::Release);
                Some(state.active_control.clone())
            })
        });
        match control {
            Some(control) => {
                if let Some(control) = control {
                    control.cancel();
                }
                true
            }
            None => false,
        }
    }
}

impl Entry {
    fn snapshot(&self) -> Result<CatalogReclamationSnapshot, ScanError> {
        self.state
            .lock()
            .map(|state| state.snapshot.clone())
            .map_err(|_| registry_error())
    }
}

impl State {
    fn start(&mut self, entry: Arc<Entry>, operation_id: String) -> CatalogReclamationOperation {
        let token = Arc::new(RunToken {
            cancelled: Arc::new(AtomicBool::new(false)),
        });
        self.snapshot = CatalogReclamationSnapshot::idle();
        self.snapshot.operation_id = Some(operation_id);
        self.snapshot.phase = CatalogReclamationPhase::Queued;
        self.run = Some(Arc::clone(&token));
        self.active_control = None;
        CatalogReclamationOperation { entry, token }
    }

    fn owns(&self, token: &Arc<RunToken>) -> bool {
        self.run
            .as_ref()
            .is_some_and(|current| Arc::ptr_eq(current, token))
    }
}

impl CatalogReclamationOperation {
    #[cfg(test)]
    pub fn new(operation_id: String) -> Self {
        CatalogReclamationRegistry::default()
            .admit(PathBuf::new(), operation_id, true)
            .expect("test reclamation admission")
            .worker
            .expect("new test worker")
    }

    pub fn snapshot(&self) -> Result<CatalogReclamationSnapshot, ScanError> {
        self.entry.snapshot()
    }

    pub fn update(&self, update: impl FnOnce(&mut CatalogReclamationSnapshot)) {
        if let Ok(mut state) = self.entry.state.lock()
            && state.owns(&self.token)
        {
            update(&mut state.snapshot);
        }
    }

    pub fn is_cancelled(&self) -> bool {
        self.token.cancelled.load(Ordering::Acquire)
    }

    #[cfg(test)]
    pub fn cancel(&self) {
        self.token.cancelled.store(true, Ordering::Release);
    }

    pub fn begin_attempt(&self) -> Result<CatalogMaintenanceControl, ScanError> {
        let mut state = self.entry.state.lock().map_err(|_| registry_error())?;
        if !state.owns(&self.token) {
            return Err(stale_run_error());
        }
        let control = CatalogMaintenanceControl::new(Arc::clone(&self.token.cancelled));
        state.active_control = Some(control.clone());
        Ok(control)
    }

    pub fn finish_attempt(&self) {
        if let Ok(mut state) = self.entry.state.lock()
            && state.owns(&self.token)
        {
            state.active_control = None;
        }
    }

    pub fn finish(
        &self,
        result: Result<ReclamationCompletion, ScanError>,
    ) -> Result<Option<Self>, ScanError> {
        let mut state = self.entry.state.lock().map_err(|_| registry_error())?;
        if !state.owns(&self.token) {
            return Err(stale_run_error());
        }
        state.active_control = None;
        // A request is either handed to this worker or admitted after its atomic retirement.
        if let Some(operation_id) = state.pending_operation_id.take() {
            return Ok(Some(state.start(Arc::clone(&self.entry), operation_id)));
        }
        state.run = None;
        match result {
            Ok(ReclamationCompletion::Completed) => {
                state.snapshot.phase = CatalogReclamationPhase::Completed
            }
            Ok(ReclamationCompletion::Cancelled) => {
                state.snapshot.phase = CatalogReclamationPhase::Cancelled
            }
            Err(error) => publish_failure(&mut state.snapshot, error),
        }
        Ok(None)
    }

    pub fn worker_unavailable(&self, error: ScanError) {
        if let Ok(mut state) = self.entry.state.lock()
            && state.owns(&self.token)
        {
            if let Some(operation_id) = state.pending_operation_id.take() {
                state.snapshot = CatalogReclamationSnapshot::idle();
                state.snapshot.operation_id = Some(operation_id);
            }
            state.run = None;
            state.active_control = None;
            publish_failure(&mut state.snapshot, error);
        }
    }
}

fn publish_failure(snapshot: &mut CatalogReclamationSnapshot, error: ScanError) {
    snapshot.phase = CatalogReclamationPhase::Failed;
    snapshot.error_code = Some(error.code);
    snapshot.error_message = Some(error.message);
}

fn registry_error() -> ScanError {
    ScanError::new(
        "catalog_reclamation_registry_unavailable",
        "The catalog reclamation task registry is unavailable",
    )
}

fn stale_run_error() -> ScanError {
    ScanError::new(
        "catalog_reclamation_run_superseded",
        "The catalog reclamation worker no longer owns this request",
    )
}

#[cfg(test)]
mod tests;
