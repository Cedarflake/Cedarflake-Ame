use super::{
    Arc, AtomicBool, CatalogSessionEntry, CatalogSessionState, Condvar, Mutex, Ordering, Path,
    ScanError, catalog_session_entry, registry_unavailable,
};
use crate::ports::CatalogMaintenanceAttempt;

#[derive(Default)]
pub(super) struct CatalogMaintenanceFlight {
    finished: Mutex<bool>,
    ready: Condvar,
    invalidated: AtomicBool,
}

impl CatalogMaintenanceFlight {
    pub(super) fn wait(&self) -> Result<(), ScanError> {
        let mut finished = self.finished.lock().map_err(|_| registry_unavailable())?;
        while !*finished {
            finished = self
                .ready
                .wait(finished)
                .map_err(|_| registry_unavailable())?;
        }
        Ok(())
    }

    #[cfg(test)]
    pub(super) fn invalidate(&self) {
        self.invalidated.store(true, Ordering::Release);
    }

    fn finish(&self) {
        let mut finished = self
            .finished
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        *finished = true;
        self.ready.notify_all();
    }
}

struct CatalogMaintenanceReservation {
    entry: Arc<CatalogSessionEntry>,
    flight: Arc<CatalogMaintenanceFlight>,
    previous: Option<CatalogSessionState>,
}

impl Drop for CatalogMaintenanceReservation {
    fn drop(&mut self) {
        let mut state = self
            .entry
            .state
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if matches!(&*state, CatalogSessionState::Maintaining(current)
            if Arc::ptr_eq(current, &self.flight))
        {
            *state = if self.flight.invalidated.load(Ordering::Acquire) {
                CatalogSessionState::Empty
            } else {
                self.previous.take().unwrap_or(CatalogSessionState::Empty)
            };
        }
        drop(state);
        self.flight.finish();
    }
}

/// Structural maintenance cannot retire a proof that a foreground scan still owns.
/// The reservation covers the operation, but its transition mutex never covers SQLite work.
pub(in crate::application) fn with_catalog_maintenance<T>(
    path: &Path,
    attempt: impl FnOnce() -> Result<CatalogMaintenanceAttempt<T>, ScanError>,
) -> Result<CatalogMaintenanceAttempt<T>, ScanError> {
    let entry = catalog_session_entry(path)?;
    let mut state = entry.state.lock().map_err(|_| registry_unavailable())?;
    if matches!(
        &*state,
        CatalogSessionState::Validating(_) | CatalogSessionState::Maintaining(_)
    ) || crate::application::scan_library::catalog_has_active_scan(path, None)?
    {
        return Ok(CatalogMaintenanceAttempt::Busy);
    }
    let flight = Arc::new(CatalogMaintenanceFlight::default());
    let previous = std::mem::replace(
        &mut *state,
        CatalogSessionState::Maintaining(Arc::clone(&flight)),
    );
    drop(state);
    let mut reservation = CatalogMaintenanceReservation {
        entry,
        flight,
        previous: None,
    };
    let result = attempt();
    if matches!(&result, Ok(CatalogMaintenanceAttempt::Busy)) {
        reservation.previous = Some(previous);
    }
    // On unwind the reservation also clears the prior proof and wakes every waiter.
    drop(reservation);
    result
}

#[cfg(test)]
mod tests;
