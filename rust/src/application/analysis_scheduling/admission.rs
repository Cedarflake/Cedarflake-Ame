use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Condvar, Mutex};
use std::time::{Duration, Instant};

use crate::domain::ScanError;
use crate::domain::analysis_capacity::{
    AnalysisCapacity, AnalysisCapacityPolicy, AnalysisConcurrency,
};
use crate::ports::analysis_capacity::AnalysisCapacityProvider;
use crate::ports::classification_encoder::ClassificationEncoderKind;

use super::{Priority, unavailable};

const SAMPLE_INTERVAL: Duration = Duration::from_secs(2);
const CANCELLATION_INTERVAL: Duration = Duration::from_millis(25);
const TEXT_SESSION: Duration = Duration::from_secs(15);

struct State {
    policy: AnalysisCapacityPolicy,
    sampled_at: Option<Instant>,
    capacity: Option<AnalysisCapacity>,
    active: [bool; 2],
    exclusive: bool,
    foreground: Option<ClassificationEncoderKind>,
    foreground_encoded: bool,
    warm_until: Instant,
    closed: bool,
}

pub(super) struct Admission {
    state: Mutex<State>,
    changed: Condvar,
    provider: Box<dyn AnalysisCapacityProvider>,
    gpu: bool,
    started: Instant,
}

pub(super) struct Lease<'a> {
    admission: &'a Admission,
    kind: ClassificationEncoderKind,
    pub serial: bool,
}

pub(crate) struct ForegroundSession<'a>(&'a Admission);

impl Admission {
    pub fn new(provider: Box<dyn AnalysisCapacityProvider>, gpu: bool) -> Self {
        Self {
            state: Mutex::new(State {
                policy: AnalysisCapacityPolicy::default(),
                sampled_at: None,
                capacity: None,
                active: [false; 2],
                exclusive: false,
                foreground: None,
                foreground_encoded: false,
                warm_until: Instant::now(),
                closed: false,
            }),
            changed: Condvar::new(),
            provider,
            gpu,
            started: Instant::now(),
        }
    }

    pub fn foreground(
        &self,
        kind: ClassificationEncoderKind,
    ) -> Result<ForegroundSession<'_>, ScanError> {
        let mut state = self.state.lock().map_err(|_| unavailable())?;
        if state.closed || state.foreground.is_some() {
            return Err(unavailable());
        }
        state.foreground = Some(kind);
        state.foreground_encoded = false;
        self.changed.notify_all();
        Ok(ForegroundSession(self))
    }

    pub fn acquire(
        &self,
        kind: ClassificationEncoderKind,
        priority: Priority,
        cancelled: &AtomicBool,
    ) -> Result<Lease<'_>, ScanError> {
        let mut state = self.state.lock().map_err(|_| unavailable())?;
        loop {
            if state.closed || cancelled.load(Ordering::Acquire) {
                return Err(ScanError::new(
                    "classification_cancelled",
                    "Analysis was stopped",
                ));
            }
            let parallel = self.mode(&mut state) == AnalysisConcurrency::Parallel;
            if priority == Priority::Background && Self::foreground_blocks(&state, kind, parallel) {
                // Yield before waiting: the caller may hold a source-read admission needed by search.
                return Err(ScanError::new(
                    "analysis_yielded",
                    "Foreground analysis has priority",
                ));
            }
            if !state.exclusive
                && !state.active[index(kind)]
                && (parallel || !state.active[other(kind)])
            {
                state.active[index(kind)] = true;
                state.exclusive = !parallel;
                if priority == Priority::Foreground {
                    state.foreground_encoded = true;
                    if kind == ClassificationEncoderKind::Image {
                        state.warm_until = Instant::now();
                    }
                }
                return Ok(Lease {
                    admission: self,
                    kind,
                    serial: !parallel,
                });
            }
            state = self
                .changed
                .wait_timeout(state, CANCELLATION_INTERVAL)
                .map_err(|_| unavailable())?
                .0;
        }
    }

    pub fn background_delay(&self) -> Result<Option<Duration>, ScanError> {
        let mut state = self.state.lock().map_err(|_| unavailable())?;
        if self.mode(&mut state) == AnalysisConcurrency::Parallel {
            return Ok(None);
        }
        if state.foreground.is_some() {
            return Ok(Some(CANCELLATION_INTERVAL));
        }
        Ok(state
            .warm_until
            .checked_duration_since(Instant::now())
            .map(|delay| delay.min(SAMPLE_INTERVAL)))
    }

    pub fn maintenance(
        &self,
        kind: ClassificationEncoderKind,
    ) -> Result<Option<Lease<'_>>, ScanError> {
        let mut state = self.state.lock().map_err(|_| unavailable())?;
        let foreground_owns = state.foreground == Some(kind)
            || kind == ClassificationEncoderKind::Text && Instant::now() < state.warm_until;
        if state.exclusive || state.active[index(kind)] || foreground_owns || state.closed {
            return Ok(None);
        }
        state.active[index(kind)] = true;
        Ok(Some(Lease {
            admission: self,
            kind,
            serial: false,
        }))
    }

    pub fn should_release(&self, kind: ClassificationEncoderKind) -> Result<bool, ScanError> {
        let mut state = self.state.lock().map_err(|_| unavailable())?;
        if self.mode(&mut state) == AnalysisConcurrency::Parallel {
            return Ok(false);
        }
        let retain_text = state.foreground == Some(ClassificationEncoderKind::Text)
            || Instant::now() < state.warm_until;
        Ok(kind
            == if retain_text {
                ClassificationEncoderKind::Image
            } else {
                ClassificationEncoderKind::Text
            })
    }

    pub fn close(&self) -> Result<(), ScanError> {
        let mut state = self.state.lock().map_err(|_| unavailable())?;
        state.closed = true;
        self.changed.notify_all();
        while state.active.iter().any(|active| *active) {
            state = self.changed.wait(state).map_err(|_| unavailable())?;
        }
        Ok(())
    }

    fn mode(&self, state: &mut State) -> AnalysisConcurrency {
        let now = Instant::now();
        if state
            .sampled_at
            .is_none_or(|sample| now.duration_since(sample) >= SAMPLE_INTERVAL)
        {
            state.capacity = self.provider.capacity();
            state.sampled_at = Some(now);
        }
        state.policy.update(
            state.capacity,
            self.gpu,
            self.provider.memory_pressure(),
            self.started.elapsed(),
        )
    }

    fn foreground_blocks(state: &State, kind: ClassificationEncoderKind, parallel: bool) -> bool {
        if !parallel && (state.foreground.is_some() || Instant::now() < state.warm_until) {
            return true;
        }
        state.foreground == Some(kind)
    }
}

pub(super) fn index(kind: ClassificationEncoderKind) -> usize {
    match kind {
        ClassificationEncoderKind::Image => 0,
        ClassificationEncoderKind::Text => 1,
    }
}

pub(super) fn other(kind: ClassificationEncoderKind) -> usize {
    1 - index(kind)
}

impl Drop for Lease<'_> {
    fn drop(&mut self) {
        if let Ok(mut state) = self.admission.state.lock() {
            state.active[index(self.kind)] = false;
            if self.serial {
                state.exclusive = false;
            }
            self.admission.changed.notify_all();
        }
    }
}

impl Drop for ForegroundSession<'_> {
    fn drop(&mut self) {
        if let Ok(mut state) = self.0.state.lock() {
            if state.foreground == Some(ClassificationEncoderKind::Text) && state.foreground_encoded
            {
                state.warm_until = Instant::now() + TEXT_SESSION;
            }
            state.foreground = None;
            self.0.changed.notify_all();
        }
    }
}
