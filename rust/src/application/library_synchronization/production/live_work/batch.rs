use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use crate::adapters::{SqliteCatalog, SqliteCatalogSession};
use crate::application::{
    AuthoritativeRecoveryPolicy, process_leased_authoritative_library_change_cancellable,
    process_ready_library_changes_in_lane_cancellable,
};
use crate::domain::{
    IncrementalLibraryChangeReport, LibraryChangeLane, LibraryChangeQueuePolicy,
    LibraryRootGeneration, ScanError,
};
use crate::ports::IncrementalCatalogRepository;

use super::super::now_unix_ms;

const ADMISSION_QUANTUM: Duration = Duration::from_millis(100);

#[derive(Default)]
pub(in super::super) struct LiveWorkOutcome {
    pub(in super::super) report: IncrementalLibraryChangeReport,
    pub(in super::super) failure: Option<ScanError>,
}

impl LiveWorkOutcome {
    pub(in super::super) fn failed(error: ScanError) -> Self {
        Self {
            failure: Some(error),
            ..Self::default()
        }
    }
}

enum LiveStep {
    Scope(IncrementalLibraryChangeReport),
    PathBatch(IncrementalLibraryChangeReport),
    Idle,
}

struct LiveReconciliation<'a> {
    catalog: &'a mut SqliteCatalog,
    root_id: &'a str,
    root_generation: LibraryRootGeneration,
    queue_policy: LibraryChangeQueuePolicy,
    recovery_policy: AuthoritativeRecoveryPolicy,
    cancelled: &'a AtomicBool,
}

impl LiveReconciliation<'_> {
    fn next(&mut self, first: bool, now: i64) -> Result<LiveStep, ScanError> {
        let Some(root) = self.catalog.load_incremental_catalog_root(self.root_id)? else {
            return Ok(LiveStep::Idle);
        };
        if root.root_generation != self.root_generation || root.active_scan_id.is_none() {
            return Ok(LiveStep::Idle);
        }
        if let Some(leased) = self.catalog.lease_live_authoritative_library_change(
            self.root_id,
            self.root_generation,
            now,
            self.queue_policy,
        )? {
            #[cfg(test)]
            super::super::pause_live_worker_after_lease(self.root_id);
            let report = process_leased_authoritative_library_change_cancellable(
                self.catalog,
                &root,
                &leased,
                now,
                self.queue_policy,
                self.recovery_policy,
                self.cancelled,
            )?;
            Ok(LiveStep::Scope(report.incremental))
        } else if first {
            process_ready_library_changes_in_lane_cancellable(
                self.catalog,
                self.root_id,
                self.root_generation,
                LibraryChangeLane::Live,
                now,
                self.queue_policy,
                self.cancelled,
            )
            .map(LiveStep::PathBatch)
        } else {
            Ok(LiveStep::Idle)
        }
    }
}

pub(super) fn run(
    session: Arc<SqliteCatalogSession>,
    root_id: &str,
    root_generation: LibraryRootGeneration,
    queue_policy: LibraryChangeQueuePolicy,
    recovery_policy: AuthoritativeRecoveryPolicy,
    cancelled: &AtomicBool,
) -> LiveWorkOutcome {
    let mut catalog = match session.open_in_lane(LibraryChangeLane::Live) {
        Ok(catalog) => catalog,
        Err(error) => return LiveWorkOutcome::failed(error),
    };
    let mut reconciliation = LiveReconciliation {
        catalog: &mut catalog,
        root_id,
        root_generation,
        queue_policy,
        recovery_policy,
        cancelled,
    };
    let started = Instant::now();
    run_quantum(
        queue_policy.max_lease_batch,
        cancelled,
        || started.elapsed(),
        |first| reconciliation.next(first, now_unix_ms()?),
    )
}

fn run_quantum(
    max_leases: u32,
    cancelled: &AtomicBool,
    mut elapsed: impl FnMut() -> Duration,
    mut next: impl FnMut(bool) -> Result<LiveStep, ScanError>,
) -> LiveWorkOutcome {
    let mut outcome = LiveWorkOutcome::default();
    while outcome.report.leased_count < max_leases
        && !cancelled.load(Ordering::Acquire)
        && elapsed() < ADMISSION_QUANTUM
    {
        let step = match next(outcome.report.leased_count == 0) {
            Ok(step) => step,
            Err(error) => {
                outcome.failure = Some(error);
                break;
            }
        };
        let (report, can_continue) = match step {
            LiveStep::Scope(report) => {
                let can_continue = report.completed_count == 1
                    && report.deferred_count == 0
                    && report.retried_count == 0
                    && report.superseded_count == 0;
                (report, can_continue)
            }
            LiveStep::PathBatch(report) => (report, false),
            LiveStep::Idle => break,
        };
        if let Err(error) = append_report(&mut outcome.report, report) {
            outcome.failure = Some(error);
            break;
        }
        if !can_continue || report.leased_count == 0 {
            break;
        }
    }
    outcome
}

fn append_report(
    total: &mut IncrementalLibraryChangeReport,
    next: IncrementalLibraryChangeReport,
) -> Result<(), ScanError> {
    fn add(left: u32, right: u32) -> Result<u32, ScanError> {
        left.checked_add(right).ok_or_else(|| {
            ScanError::new(
                "library_synchronization_count_overflow",
                "The synchronization mutation count exceeded the supported range",
            )
        })
    }
    *total = IncrementalLibraryChangeReport {
        leased_count: add(total.leased_count, next.leased_count)?,
        completed_count: add(total.completed_count, next.completed_count)?,
        retried_count: add(total.retried_count, next.retried_count)?,
        deferred_count: add(total.deferred_count, next.deferred_count)?,
        superseded_count: add(total.superseded_count, next.superseded_count)?,
        applied_mutation_count: add(total.applied_mutation_count, next.applied_mutation_count)?,
        catalog_revision: total.catalog_revision.max(next.catalog_revision),
    };
    Ok(())
}

#[cfg(test)]
mod tests;
