use std::time::{Duration, Instant};

use crate::domain::{LibrarySynchronizationSnapshot, ScanError};

pub(super) struct SynchronizationPollStageTimings {
    pub(super) stage: &'static str,
    pub(super) catalog_ms: u128,
    pub(super) observation_ms: u128,
    pub(super) lanes_ms: u128,
    pub(super) scheduling_ms: u128,
    pub(super) projection_ms: u128,
    // Early returns still return the checkout, but do not time that return separately.
    pub(super) checkout_return_ms: Option<u128>,
}

impl Default for SynchronizationPollStageTimings {
    fn default() -> Self {
        Self {
            stage: "not_started",
            catalog_ms: 0,
            observation_ms: 0,
            lanes_ms: 0,
            scheduling_ms: 0,
            projection_ms: 0,
            checkout_return_ms: None,
        }
    }
}

pub(super) struct ElapsedStageTimer<'a> {
    started: Instant,
    elapsed_ms: &'a mut u128,
}

impl<'a> ElapsedStageTimer<'a> {
    pub(super) fn new(elapsed_ms: &'a mut u128) -> Self {
        Self {
            started: Instant::now(),
            elapsed_ms,
        }
    }
}

impl Drop for ElapsedStageTimer<'_> {
    fn drop(&mut self) {
        *self.elapsed_ms = (*self.elapsed_ms).saturating_add(self.started.elapsed().as_millis());
    }
}

pub(super) fn log_synchronization_poll_diagnostic(
    elapsed: Duration,
    timings: &SynchronizationPollStageTimings,
    result: &Result<LibrarySynchronizationSnapshot, ScanError>,
) {
    #[cfg(debug_assertions)]
    {
        const SLOW_POLL: Duration = Duration::from_millis(500);
        if elapsed < SLOW_POLL && result.is_ok() {
            return;
        }
        let (outcome, code, roots, mutations) = match result {
            Ok(snapshot) => (
                "ok",
                "none",
                snapshot.roots.len(),
                snapshot.applied_mutation_count,
            ),
            Err(error) => ("error", error.code.as_str(), 0, 0),
        };
        eprintln!(
            "[Ame sync native] outcome={outcome} code={code} stage={} total_ms={} \
             catalog_ms={} observation_ms={} lanes_ms={} scheduling_ms={} projection_ms={} \
             checkout_return_ms={} checkout_return_measured={} roots={roots} mutations={mutations}",
            timings.stage,
            elapsed.as_millis(),
            timings.catalog_ms,
            timings.observation_ms,
            timings.lanes_ms,
            timings.scheduling_ms,
            timings.projection_ms,
            timings.checkout_return_ms.unwrap_or_default(),
            timings.checkout_return_ms.is_some(),
        );
    }
    #[cfg(not(debug_assertions))]
    let _ = (elapsed, timings, result);
}
