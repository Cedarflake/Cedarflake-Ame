use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use crate::adapters::{SqliteCatalog, SqliteCatalogSession};
use crate::application::catalog_session::{
    refresh_stale_catalog_session, validated_catalog_session,
};
use crate::domain::{LibraryChangeLane, ScanError};

#[derive(Clone, Default)]
pub(super) struct PollCatalogOwner {
    slot: Arc<Mutex<PollCatalogState>>,
    #[cfg(test)]
    counters: Arc<ConnectionCounters>,
}

#[derive(Default)]
struct PollCatalogState {
    path: Option<PathBuf>,
    connection: PollCatalogSlot,
}

enum PollCatalogSlot {
    Idle(Option<Box<RetainedCatalog>>),
    CheckedOut,
    Retiring,
    Retired,
}

impl Default for PollCatalogSlot {
    fn default() -> Self {
        Self::Idle(None)
    }
}

struct RetainedCatalog {
    catalog: Option<SqliteCatalog>,
    session: Arc<SqliteCatalogSession>,
    #[cfg(test)]
    counters: Arc<ConnectionCounters>,
}

impl Drop for RetainedCatalog {
    fn drop(&mut self) {
        let catalog = self.catalog.take();
        #[cfg(test)]
        {
            let hook = self.counters.before_close.lock().unwrap().take();
            if let Some(hook) = hook {
                hook();
            }
            if let Some(catalog) = catalog {
                catalog.retire_with_poll_diagnostics();
            }
            self.counters
                .closes
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        }
        #[cfg(not(test))]
        drop(catalog);
    }
}

pub(super) struct PollCatalogCheckout {
    owner: PollCatalogOwner,
    retained: Option<Box<RetainedCatalog>>,
}

impl PollCatalogOwner {
    pub(super) fn checkout(&self, path: &Path) -> Result<PollCatalogCheckout, ScanError> {
        let retained = {
            let mut state = self.slot.lock().map_err(|_| ownership_error())?;
            if state.path.as_ref().is_some_and(|bound| bound != path) {
                return Err(ScanError::new(
                    "library_synchronization_catalog_changed",
                    "The production runtime cannot switch catalogs within one epoch",
                ));
            }
            if !matches!(state.connection, PollCatalogSlot::Idle(_)) {
                return Err(ownership_error());
            }
            state.path.get_or_insert_with(|| path.to_path_buf());
            let PollCatalogSlot::Idle(retained) =
                std::mem::replace(&mut state.connection, PollCatalogSlot::CheckedOut)
            else {
                unreachable!("the idle slot was checked under its lock")
            };
            retained
        };
        let mut checkout = PollCatalogCheckout {
            owner: self.clone(),
            retained,
        };
        checkout.prepare(path)?;
        Ok(checkout)
    }

    pub(super) fn retire(&self) -> Result<(), ScanError> {
        let retained = {
            let mut state = self.slot.lock().map_err(|_| ownership_error())?;
            match &state.connection {
                PollCatalogSlot::Retired => return Ok(()),
                PollCatalogSlot::Idle(_) => {}
                PollCatalogSlot::CheckedOut | PollCatalogSlot::Retiring => {
                    return Err(ownership_error());
                }
            }
            let PollCatalogSlot::Idle(retained) =
                std::mem::replace(&mut state.connection, PollCatalogSlot::Retiring)
            else {
                unreachable!("retirement only takes an idle connection")
            };
            retained
        };
        // SQLite close can block. No transition lock covers it and Retired is not visible early.
        drop(retained);
        self.slot.lock().map_err(|_| ownership_error())?.connection = PollCatalogSlot::Retired;
        Ok(())
    }
}

impl PollCatalogCheckout {
    fn prepare(&mut self, path: &Path) -> Result<(), ScanError> {
        let mut has_current_proof = false;
        if let Some(entry) = &self.retained {
            match entry.session.revalidate_connection(self) {
                Ok(()) => has_current_proof = true,
                Err(error) if error.code == "catalog_validated_session_stale" => {}
                Err(error) => return Err(error),
            }
        }
        let session = validated_catalog_session(path)?;
        if self
            .retained
            .as_ref()
            .is_none_or(|entry| !Arc::ptr_eq(&entry.session, &session))
        {
            return self.install(session);
        }
        if has_current_proof {
            return Ok(());
        }
        // Renewal is exceptional and still belongs to the process-session/scan protection owner.
        let replacement = refresh_stale_catalog_session(path, &session)?;
        self.install(replacement)
    }

    fn install(&mut self, mut session: Arc<SqliteCatalogSession>) -> Result<(), ScanError> {
        drop(self.retained.take());
        let catalog = match session.open_in_lane(LibraryChangeLane::Live) {
            Ok(catalog) => catalog,
            Err(error) if error.code == "catalog_validated_session_stale" => {
                session = refresh_stale_catalog_session(session.path(), &session)?;
                session.open_in_lane(LibraryChangeLane::Live)?
            }
            Err(error) => return Err(error),
        };
        #[cfg(test)]
        self.owner
            .counters
            .opens
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        self.retained = Some(Box::new(RetainedCatalog {
            catalog: Some(catalog),
            session,
            #[cfg(test)]
            counters: Arc::clone(&self.owner.counters),
        }));
        Ok(())
    }

    pub(super) fn session(&self) -> Arc<SqliteCatalogSession> {
        Arc::clone(&self.retained.as_ref().expect("prepared checkout").session)
    }
}

impl Deref for PollCatalogCheckout {
    type Target = SqliteCatalog;

    fn deref(&self) -> &Self::Target {
        self.retained
            .as_ref()
            .and_then(|entry| entry.catalog.as_ref())
            .expect("prepared checkout")
    }
}

impl DerefMut for PollCatalogCheckout {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.retained
            .as_mut()
            .and_then(|entry| entry.catalog.as_mut())
            .expect("prepared checkout")
    }
}

impl Drop for PollCatalogCheckout {
    fn drop(&mut self) {
        let mut state = self
            .owner
            .slot
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        state.connection = PollCatalogSlot::Idle(self.retained.take());
    }
}

fn ownership_error() -> ScanError {
    ScanError::new(
        "library_synchronization_catalog_ownership_unavailable",
        "The poll catalog is already checked out, retiring, or retired",
    )
}

#[cfg(test)]
#[derive(Default)]
struct ConnectionCounters {
    opens: std::sync::atomic::AtomicUsize,
    closes: std::sync::atomic::AtomicUsize,
    before_close: Mutex<Option<Box<dyn FnOnce() + Send>>>,
}

#[cfg(test)]
impl PollCatalogOwner {
    pub(super) fn connection_counts(&self) -> (usize, usize) {
        use std::sync::atomic::Ordering;
        (
            self.counters.opens.load(Ordering::SeqCst),
            self.counters.closes.load(Ordering::SeqCst),
        )
    }

    pub(super) fn before_close(&self, hook: impl FnOnce() + Send + 'static) {
        *self.counters.before_close.lock().unwrap() = Some(Box::new(hook));
    }
}

#[cfg(test)]
mod tests;
