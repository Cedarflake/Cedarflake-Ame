use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, Weak};

use crate::domain::classification::work::ClassificationRecoveryRoot;
use crate::domain::{LibraryRootGeneration, ScanError};

use super::super::classification_dispatch::ClassificationForegroundState;
use super::super::classification_runtime::{
    ClassificationRuntimeControl, NativeClassificationRuntime,
};
use super::maintenance::ClassificationDesktopMaintenance;

static CONNECTION: Mutex<Weak<Connection>> = Mutex::new(Weak::new());

type Completion = Result<Option<ClassificationDesktopMaintenance>, ScanError>;

struct State {
    generation: u64,
    pending: Option<NativeClassificationRuntime>,
    running: bool,
    completion: Option<Completion>,
}

struct Connection {
    database: PathBuf,
    control: ClassificationRuntimeControl,
    state: Mutex<State>,
    changed: Condvar,
}

pub(crate) struct ClassificationDesktopSession {
    connection: Arc<Connection>,
    generation: u64,
    started: AtomicBool,
}

impl ClassificationDesktopSession {
    pub(super) fn open(
        database: PathBuf,
        create: impl FnOnce() -> Result<Option<NativeClassificationRuntime>, ScanError>,
    ) -> Result<Option<Self>, ScanError> {
        let mut current = CONNECTION.lock().map_err(|_| unavailable())?;
        if let Some(connection) = current.upgrade() {
            let mut state = connection.lock()?;
            if connection.database == database
                && (state.pending.is_some() || state.running)
                && !connection.control.signals().is_closed()?
            {
                state.generation = state.generation.checked_add(1).ok_or_else(unavailable)?;
                let generation = state.generation;
                connection.changed.notify_all();
                drop(state);
                return Ok(Some(Self {
                    connection,
                    generation,
                    started: AtomicBool::new(false),
                }));
            }
        }
        // Opening only admits saved configuration; source/model work starts in run().
        let Some(runtime) = create()? else {
            return Ok(None);
        };
        let connection = Arc::new(Connection {
            database,
            control: runtime.control(),
            state: Mutex::new(State {
                generation: 1,
                pending: Some(runtime),
                running: false,
                completion: None,
            }),
            changed: Condvar::new(),
        });
        *current = Arc::downgrade(&connection);
        Ok(Some(Self {
            connection,
            generation: 1,
            started: AtomicBool::new(false),
        }))
    }

    pub fn run(&self) -> Completion {
        if self.started.swap(true, Ordering::AcqRel) {
            return Err(already_started());
        }
        let connection = &self.connection;
        let mut state = connection.lock()?;
        if state.generation != self.generation {
            return Ok(None);
        }
        if let Some(mut runtime) = state.pending.take() {
            state.running = true;
            drop(state);
            // The original bridge call keeps executing; replacement calls only observe it.
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                runtime.run_until_maintenance()
            }))
            .unwrap_or_else(|_| {
                Err(ScanError::new(
                    "classification_runtime_panicked",
                    "The classification runtime terminated unexpectedly",
                ))
            });
            drop(runtime);
            state = connection.lock()?;
            state.running = false;
            state.completion = Some(result.map(|plan| {
                plan.map(|plan| {
                    ClassificationDesktopMaintenance::new(connection.database.clone(), plan)
                })
            }));
            connection.changed.notify_all();
        }
        while state.generation == self.generation && state.running {
            state = connection.changed.wait(state).map_err(|_| unavailable())?;
        }
        if state.generation != self.generation {
            return Ok(None);
        }
        state
            .completion
            .take()
            .unwrap_or_else(|| Err(already_started()))
    }

    pub fn foreground_changed(
        &self,
        state: ClassificationForegroundState,
    ) -> Result<(), ScanError> {
        self.control(|control| control.signals().foreground_changed(state))
    }

    pub fn browsing_activity(&self) -> Result<(), ScanError> {
        self.control(|control| control.signals().browsing_activity())
    }

    pub fn catalog_changed(&self) -> Result<(), ScanError> {
        self.control(|control| control.signals().catalog_changed())
    }

    pub fn root_available(&self, root_id: String, generation: u64) -> Result<(), ScanError> {
        let root = ClassificationRecoveryRoot {
            id: root_id,
            generation: LibraryRootGeneration::new(generation).ok_or_else(|| {
                ScanError::new(
                    "classification_root_invalid",
                    "The source generation must be positive",
                )
            })?,
        };
        let mut cursor = None;
        loop {
            {
                let state = self.connection.lock()?;
                if state.generation != self.generation
                    || self.connection.control.signals().is_closed()?
                {
                    return Ok(());
                }
            }
            let page = self
                .connection
                .control
                .root_available(&root, cursor.as_ref())?;
            cursor = page.next;
            if cursor.is_none() {
                return Ok(());
            }
        }
    }

    pub fn request_close(&self) -> Result<(), ScanError> {
        self.control(|control| control.signals().close())
    }

    pub fn close(&self) -> Result<(), ScanError> {
        let pending = {
            let mut state = self.connection.lock()?;
            if state.generation != self.generation {
                return Ok(());
            }
            self.connection.control.signals().close()?;
            state.pending.take()
        };
        drop(pending);
        Ok(())
    }

    fn control(
        &self,
        action: impl FnOnce(&ClassificationRuntimeControl) -> Result<(), ScanError>,
    ) -> Result<(), ScanError> {
        let state = self.connection.lock()?;
        if state.generation != self.generation {
            return Ok(());
        }
        action(&self.connection.control)
    }
}

impl Connection {
    fn lock(&self) -> Result<MutexGuard<'_, State>, ScanError> {
        self.state.lock().map_err(|_| unavailable())
    }
}

impl Drop for ClassificationDesktopSession {
    fn drop(&mut self) {
        let _ = self.request_close();
    }
}

fn already_started() -> ScanError {
    ScanError::new(
        "classification_session_started",
        "The classification session has already started or closed",
    )
}

fn unavailable() -> ScanError {
    ScanError::new(
        "classification_session_unavailable",
        "The classification session ownership is unavailable",
    )
}

#[cfg(test)]
mod tests;
