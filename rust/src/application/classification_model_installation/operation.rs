use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::domain::ScanError;
use crate::domain::classification::installation::ModelInstallationProgress;

use super::{PreparedModelInstallation, prepare_managed_classification_installation};

enum PreparationState {
    New,
    Preparing,
    Prepared(Box<PreparedModelInstallation>),
    Committing,
    Closed,
}

pub(crate) struct ModelInstallationOperation {
    state: Mutex<PreparationState>,
    progress: Mutex<Option<ModelInstallationProgress>>,
    cancelled: AtomicBool,
}

impl ModelInstallationOperation {
    pub fn new() -> Self {
        Self {
            state: Mutex::new(PreparationState::New),
            progress: Mutex::new(None),
            cancelled: AtomicBool::new(false),
        }
    }

    pub fn prepare(&self) -> Result<(), ScanError> {
        self.prepare_with(prepare_managed_classification_installation)
    }

    fn prepare_with(
        &self,
        prepare: impl FnOnce(
            &AtomicBool,
            &mut dyn FnMut(ModelInstallationProgress),
        ) -> Result<PreparedModelInstallation, ScanError>,
    ) -> Result<(), ScanError> {
        {
            let mut state = self.state.lock().map_err(|_| state_error())?;
            super::check_cancelled(&self.cancelled)?;
            if !matches!(*state, PreparationState::New) {
                return Err(state_error());
            }
            *state = PreparationState::Preparing;
        }
        let mut progress_error = None;
        let prepared = prepare(&self.cancelled, &mut |value| match self.progress.lock() {
            Ok(mut progress) => *progress = Some(value),
            Err(_) => {
                progress_error = Some(state_error());
                self.cancelled.store(true, Ordering::Release);
            }
        });
        let mut state = self.state.lock().map_err(|_| state_error())?;
        *state = PreparationState::Closed;
        if let Some(error) = progress_error {
            return Err(error);
        }
        super::check_cancelled(&self.cancelled)?;
        *state = PreparationState::Prepared(Box::new(prepared?));
        Ok(())
    }

    pub fn progress(&self) -> Result<Option<ModelInstallationProgress>, ScanError> {
        self.progress
            .lock()
            .map(|progress| *progress)
            .map_err(|_| state_error())
    }

    pub fn cancel(&self) -> Result<(), ScanError> {
        self.cancelled.store(true, Ordering::Release);
        let mut state = self.state.lock().map_err(|_| state_error())?;
        if matches!(
            *state,
            PreparationState::New | PreparationState::Prepared(_)
        ) {
            *state = PreparationState::Closed;
        }
        Ok(())
    }

    pub fn commit(&self) -> Result<(), ScanError> {
        let prepared = {
            let mut state = self.state.lock().map_err(|_| state_error())?;
            if !matches!(*state, PreparationState::Prepared(_)) {
                return Err(state_error());
            }
            match std::mem::replace(&mut *state, PreparationState::Committing) {
                PreparationState::Prepared(prepared) => prepared,
                _ => return Err(state_error()),
            }
        };
        let result = prepared.commit(&self.cancelled);
        *self.state.lock().map_err(|_| state_error())? = PreparationState::Closed;
        result
    }
}

impl Default for ModelInstallationOperation {
    fn default() -> Self {
        Self::new()
    }
}

fn state_error() -> ScanError {
    ScanError::new(
        "classification_installation_unavailable",
        "This installation operation is no longer available",
    )
}

#[cfg(test)]
mod tests;
