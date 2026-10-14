use super::{
    Arc, AtomicBool, CatalogSessionEntry, CatalogSessionState, Condvar, Mutex, Ordering, Path,
    ScanError, catalog_session_entry, registry_unavailable,
};
use crate::domain::CatalogSpaceUsage;
use crate::ports::{CatalogMaintenanceAttempt, CatalogMaintenanceControl, CatalogSpaceRepository};

enum CatalogMaintenanceKind {
    Structural,
    IncrementalReclamation,
}

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
    with_reserved_catalog_maintenance(path, CatalogMaintenanceKind::Structural, attempt)
}

pub(in crate::application) fn reclaim_catalog_pages(
    path: &Path,
    maintenance: &impl CatalogSpaceRepository,
    max_pages: u32,
    control: &CatalogMaintenanceControl,
) -> Result<CatalogMaintenanceAttempt<CatalogSpaceUsage>, ScanError> {
    with_reserved_catalog_maintenance(path, CatalogMaintenanceKind::IncrementalReclamation, || {
        maintenance.try_reclaim_incremental(max_pages, control)
    })
}

fn with_reserved_catalog_maintenance<T>(
    path: &Path,
    kind: CatalogMaintenanceKind,
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
    let retain_previous = match &result {
        Ok(CatalogMaintenanceAttempt::Busy) => true,
        Ok(CatalogMaintenanceAttempt::Completed(_) | CatalogMaintenanceAttempt::Interrupted) => {
            // Incremental vacuum only moves or releases pages. Its normal interruption
            // preserves the schema and rows that the reusable session already validated.
            matches!(kind, CatalogMaintenanceKind::IncrementalReclamation)
                && matches!(&previous, CatalogSessionState::Ready(_))
        }
        Err(_) => false,
    };
    if retain_previous {
        reservation.previous = Some(previous);
    }
    // On unwind the reservation also clears the prior proof and wakes every waiter.
    drop(reservation);
    result
}

#[cfg(test)]
mod tests;
