use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::domain::ScanError;

const INITIAL_QUIET: Duration = Duration::from_secs(3);

pub trait ClassificationDispatchClock: Send + Sync {
    fn elapsed(&self) -> Duration;
    fn unix_ms(&self) -> u64;
}

pub struct MonotonicDispatchClock(Instant);

impl Default for MonotonicDispatchClock {
    fn default() -> Self {
        Self(Instant::now())
    }
}

impl ClassificationDispatchClock for MonotonicDispatchClock {
    fn elapsed(&self) -> Duration {
        self.0.elapsed()
    }

    fn unix_ms(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
            .min(i64::MAX as u128) as u64
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ClassificationForegroundState {
    pub reading_sources: bool,
    pub importing: bool,
    pub suspended: bool,
    pub memory_pressure: bool,
}

impl ClassificationForegroundState {
    fn busy(self) -> bool {
        self.reading_sources || self.importing || self.suspended || self.memory_pressure
    }
}

struct State {
    notification: u64,
    catalog_revision: u64,
    foreground: ClassificationForegroundState,
    quiet_since: Duration,
    admitted: bool,
    closed: bool,
    active: Option<Arc<AtomicBool>>,
}

struct Shared<C> {
    state: Mutex<State>,
    changed: Condvar,
    clock: C,
}

pub struct ClassificationDispatchSignals<C = MonotonicDispatchClock> {
    shared: Arc<Shared<C>>,
}

impl<C> Clone for ClassificationDispatchSignals<C> {
    fn clone(&self) -> Self {
        Self {
            shared: self.shared.clone(),
        }
    }
}

impl<C: ClassificationDispatchClock> ClassificationDispatchSignals<C> {
    pub fn new(clock: C) -> Self {
        Self {
            shared: Arc::new(Shared {
                state: Mutex::new(State {
                    notification: 0,
                    catalog_revision: 1,
                    foreground: ClassificationForegroundState::default(),
                    quiet_since: clock.elapsed(),
                    admitted: false,
                    closed: false,
                    active: None,
                }),
                changed: Condvar::new(),
                clock,
            }),
        }
    }

    pub fn foreground_changed(
        &self,
        foreground: ClassificationForegroundState,
    ) -> Result<(), ScanError> {
        let mut state = self.shared.state.lock().map_err(|_| unavailable())?;
        if state.foreground != foreground {
            state.foreground = foreground;
            self.interrupt(&mut state);
        }
        Ok(())
    }

    pub fn browsing_activity(&self) -> Result<(), ScanError> {
        let mut state = self.shared.state.lock().map_err(|_| unavailable())?;
        if !state.admitted {
            state.quiet_since = self.shared.clock.elapsed();
            self.notify(&mut state);
        }
        Ok(())
    }

    pub fn catalog_changed(&self) -> Result<(), ScanError> {
        let mut state = self.shared.state.lock().map_err(|_| unavailable())?;
        state.catalog_revision = state.catalog_revision.wrapping_add(1);
        self.notify(&mut state);
        Ok(())
    }

    pub fn dispatch_changed(&self) -> Result<(), ScanError> {
        let mut state = self.shared.state.lock().map_err(|_| unavailable())?;
        state.catalog_revision = state.catalog_revision.wrapping_add(1);
        self.interrupt(&mut state);
        Ok(())
    }

    pub fn close(&self) -> Result<(), ScanError> {
        let mut state = self.shared.state.lock().map_err(|_| unavailable())?;
        state.closed = true;
        self.interrupt(&mut state);
        Ok(())
    }

    fn interrupt(&self, state: &mut State) {
        if let Some(active) = &state.active {
            active.store(true, Ordering::Release);
        }
        state.admitted = false;
        state.quiet_since = self.shared.clock.elapsed();
        self.notify(state);
    }

    fn notify(&self, state: &mut State) {
        state.notification = state.notification.wrapping_add(1);
        self.shared.changed.notify_all();
    }

    pub(super) fn admission(&self, automatic: bool) -> Result<Admission<C>, ScanError> {
        let mut state = self.shared.state.lock().map_err(|_| unavailable())?;
        if state.closed {
            return Ok(Admission::Closed);
        }
        if !automatic || state.foreground.busy() {
            state.admitted = false;
            return Ok(Admission::Wait {
                delay: None,
                retire: true,
            });
        }
        let quiet = self
            .shared
            .clock
            .elapsed()
            .saturating_sub(state.quiet_since);
        if !state.admitted && quiet < INITIAL_QUIET {
            return Ok(Admission::Wait {
                delay: Some(INITIAL_QUIET - quiet),
                retire: false,
            });
        }
        if state.active.is_some() {
            return Err(unavailable());
        }
        let cancelled = Arc::new(AtomicBool::new(false));
        state.active = Some(cancelled.clone());
        state.admitted = true;
        Ok(Admission::Ready(ActiveWork {
            cancelled,
            catalog_revision: state.catalog_revision,
            signals: self.clone(),
        }))
    }

    pub(super) fn idle(&self, catalog_revision: u64) -> Result<(), ScanError> {
        let mut state = self.shared.state.lock().map_err(|_| unavailable())?;
        if state.catalog_revision == catalog_revision {
            state.admitted = false;
        }
        Ok(())
    }

    pub(super) fn notification(&self) -> Result<u64, ScanError> {
        Ok(self
            .shared
            .state
            .lock()
            .map_err(|_| unavailable())?
            .notification)
    }

    pub(super) fn wait(&self, observed: u64, delay: Option<Duration>) -> Result<(), ScanError> {
        let state = self.shared.state.lock().map_err(|_| unavailable())?;
        if state.notification != observed || state.closed {
            return Ok(());
        }
        match delay {
            Some(delay) => {
                drop(
                    self.shared
                        .changed
                        .wait_timeout(state, delay)
                        .map_err(|_| unavailable())?,
                );
            }
            None => {
                drop(self.shared.changed.wait(state).map_err(|_| unavailable())?);
            }
        }
        Ok(())
    }

    pub(crate) fn unix_ms(&self) -> u64 {
        self.shared.clock.unix_ms()
    }
}

pub(super) enum Admission<C: ClassificationDispatchClock> {
    Closed,
    Wait {
        delay: Option<Duration>,
        retire: bool,
    },
    Ready(ActiveWork<C>),
}

pub(super) struct ActiveWork<C: ClassificationDispatchClock> {
    pub cancelled: Arc<AtomicBool>,
    pub catalog_revision: u64,
    signals: ClassificationDispatchSignals<C>,
}

impl<C: ClassificationDispatchClock> Drop for ActiveWork<C> {
    fn drop(&mut self) {
        if let Ok(mut state) = self.signals.shared.state.lock()
            && state
                .active
                .as_ref()
                .is_some_and(|active| Arc::ptr_eq(active, &self.cancelled))
        {
            state.active = None;
        }
    }
}

fn unavailable() -> ScanError {
    ScanError::new(
        "classification_dispatch_unavailable",
        "Classification dispatch ownership is unavailable",
    )
}

#[cfg(test)]
mod tests;
