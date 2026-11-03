use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Condvar, Mutex, OnceLock};
use std::time::Duration;

use crate::domain::ScanError;

mod exact_reader;
pub(crate) use exact_reader::AdmittedExactContentReader;

const MAX_WAITERS: usize = 8;
const CANCELLATION_INTERVAL: Duration = Duration::from_millis(25);

#[derive(Default)]
struct AdmissionState {
    next_ticket: u64,
    active: Option<u64>,
    waiting: VecDeque<u64>,
}

#[derive(Default)]
pub(crate) struct BackgroundAnalysisGate {
    state: Mutex<AdmissionState>,
    changed: Condvar,
}

pub(crate) struct BackgroundAnalysisLease<'a> {
    gate: &'a BackgroundAnalysisGate,
    ticket: u64,
}

pub(crate) fn shared() -> &'static BackgroundAnalysisGate {
    static GATE: OnceLock<BackgroundAnalysisGate> = OnceLock::new();
    GATE.get_or_init(BackgroundAnalysisGate::default)
}

impl BackgroundAnalysisGate {
    pub(crate) fn acquire(
        &self,
        cancelled: &AtomicBool,
    ) -> Result<Option<BackgroundAnalysisLease<'_>>, ScanError> {
        if cancelled.load(Ordering::Acquire) {
            return Ok(None);
        }
        let mut state = self.state.lock().map_err(|_| unavailable())?;
        if state.waiting.len() >= MAX_WAITERS {
            return Err(ScanError::new(
                "background_analysis_busy",
                "The bounded background analysis queue is full",
            ));
        }
        state.next_ticket = state.next_ticket.checked_add(1).ok_or_else(unavailable)?;
        let ticket = state.next_ticket;
        state.waiting.push_back(ticket);
        loop {
            if cancelled.load(Ordering::Acquire) {
                state.waiting.retain(|waiting| *waiting != ticket);
                self.changed.notify_all();
                return Ok(None);
            }
            if state.active.is_none() && state.waiting.front() == Some(&ticket) {
                state.waiting.pop_front();
                state.active = Some(ticket);
                return Ok(Some(BackgroundAnalysisLease { gate: self, ticket }));
            }
            let (next, _) = self
                .changed
                .wait_timeout(state, CANCELLATION_INTERVAL)
                .map_err(|_| unavailable())?;
            state = next;
        }
    }
}

impl Drop for BackgroundAnalysisLease<'_> {
    fn drop(&mut self) {
        if let Ok(mut state) = self.gate.state.lock()
            && state.active == Some(self.ticket)
        {
            state.active = None;
            self.gate.changed.notify_all();
        }
    }
}

fn unavailable() -> ScanError {
    ScanError::new(
        "background_analysis_unavailable",
        "Background analysis admission is unavailable",
    )
}

#[cfg(test)]
mod tests;
