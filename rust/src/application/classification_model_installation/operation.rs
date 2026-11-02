use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::domain::ScanError;
use crate::domain::classification::installation::ModelInstallationProgress;

use super::recovery::{InstallationSource, PendingDownload, pending_source};
use super::{ModelPreparation, PreparedModelInstallation};

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
    source: InstallationSource,
    pending: Mutex<Option<PendingDownload>>,
}

impl ModelInstallationOperation {
    pub fn new() -> Self {
        Self::with_source(InstallationSource::Download)
    }

    fn with_source(source: InstallationSource) -> Self {
        let pending = match &source {
            InstallationSource::Resume(pending) => Some(pending.clone()),
            _ => None,
        };
        Self {
            state: Mutex::new(PreparationState::New),
            progress: Mutex::new(None),
            cancelled: AtomicBool::new(false),
            source,
            pending: Mutex::new(pending),
        }
    }

    pub fn recover() -> Result<Option<Self>, ScanError> {
        Ok(pending_source()?.map(Self::with_source))
    }

    pub fn prepare(&self) -> Result<(), ScanError> {
        self.prepare_with(|cancelled, progress| {
            let preparation = ModelPreparation::begin(self.source.clone(), cancelled)?;
            *self.pending.lock().map_err(|_| state_error())? = preparation.pending.clone();
            preparation.prepare(cancelled, progress)
        })
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

    pub fn discard(&self) -> Result<(), ScanError> {
        let state = self.state.lock().map_err(|_| state_error())?;
        if matches!(
            *state,
            PreparationState::Preparing | PreparationState::Committing
        ) {
            return Err(state_error());
        }
        if let Some(pending) = self.pending.lock().map_err(|_| state_error())?.as_ref() {
            pending.cancel()?;
        }
        Ok(())
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
