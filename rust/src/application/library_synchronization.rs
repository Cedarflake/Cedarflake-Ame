use std::collections::{BTreeMap, BTreeSet};
use std::time::{Duration, Instant};

use crate::domain::{
    CatalogFreshnessCause, CatalogFreshnessState, IncrementalCatalogRoot, LibraryChangeIntent,
    LibraryChangeIntentKind, LibraryChangeLane, LibraryChangeOrigin, LibraryChangePlanningIssue,
    LibraryChangePlanningLimits, LibraryChangePlanningResult, LibraryChangeQueueHealth,
    LibraryChangeQueueMetrics, LibraryChangeQueuePolicy, LibraryChangeRestartPolicy,
    LibraryChangeScope, LibraryChangeSourceHealth, LibraryRootAvailability,
    LibraryRootSynchronizationStatus, LibrarySynchronizationPhase, LibrarySynchronizationSnapshot,
    PersistentJournalContinuityState, ScanError,
};
use crate::ports::{
    IncrementalCatalogRepository, LibraryChangeIngress, LibraryChangeSourceRequest,
    LibraryChangeSourceStarter,
};
#[cfg(test)]
use crate::ports::{LibraryChangeSourceFactory, erase_library_change_source_factory};

use super::authoritative_library_changes::process_ready_authoritative_library_change;
use super::library_change_observer::LibraryChangeObserver;
use super::{AuthoritativeRecoveryPolicy, process_ready_library_changes_in_lane};

#[cfg(windows)]
mod admission;
#[cfg(windows)]
mod journal_baseline;
mod observation_diagnostics;
mod observer_handoff;
mod production;

use observation_diagnostics::measure_observation;
use observer_handoff::ObserverHandoff;

#[cfg(test)]
#[path = "../../test_support/production_synchronization_cadence.rs"]
mod production_synchronization_cadence;

#[cfg(not(test))]
pub(crate) use production::poll_production_first_import_change_capture;
pub(crate) use production::{
    poll_production_library_synchronization,
    reserve_production_library_synchronization_start_ticket,
    reserve_production_library_synchronization_stop_fence,
    start_production_library_synchronization, stop_production_library_synchronization,
};

const DEFAULT_INGRESS_CAPACITY: usize = 4_096;
const PERSISTENCE_CONTENTION_GRACE_MILLIS: i64 = 30_000;
const OBSERVER_STOP_TIMEOUT: Duration = Duration::from_secs(2);

struct RootRuntime<Reservation> {
    root: IncrementalCatalogRoot,
    observer: Option<LibraryChangeObserver>,
    availability: LibraryRootAvailability,
    source_health: LibraryChangeSourceHealth,
    last_issue_code: Option<String>,
    blocking_issue_code: Option<String>,
    persistence_contention_started_unix_ms: Option<i64>,
    recovery_contention_started_unix_ms: Option<i64>,
    handoff: ObserverHandoff<Reservation>,
    needs_continuity_gap: bool,
    continuity_revision: u64,
}

struct RetiringObserver {
    root_id: String,
    observer: LibraryChangeObserver,
}

pub(crate) struct LibrarySynchronizationRuntime<Reservation> {
    start_source: LibraryChangeSourceStarter,
    roots: BTreeMap<String, RootRuntime<Reservation>>,
    retiring_observers: Vec<RetiringObserver>,
    planning_limits: LibraryChangePlanningLimits,
    restart_policy: LibraryChangeRestartPolicy,
    queue_policy: LibraryChangeQueuePolicy,
    recovery_policy: AuthoritativeRecoveryPolicy,
    ingress_capacity: usize,
    schedules_initial_metadata_inventory: bool,
    is_running: bool,
    is_stopping: bool,
}

impl<Reservation> LibrarySynchronizationRuntime<Reservation> {
    #[cfg(test)]
    pub(crate) fn new_erased(start_source: LibraryChangeSourceStarter) -> Self {
        Self::new(start_source, true)
    }

    pub(crate) fn new_production(start_source: LibraryChangeSourceStarter) -> Self {
        Self::new(start_source, false)
    }

    fn new(
        start_source: LibraryChangeSourceStarter,
        schedules_initial_metadata_inventory: bool,
    ) -> Self {
        Self {
            start_source,
            roots: BTreeMap::new(),
            retiring_observers: Vec::new(),
            planning_limits: LibraryChangePlanningLimits::default(),
            restart_policy: LibraryChangeRestartPolicy::default(),
            queue_policy: LibraryChangeQueuePolicy::default(),
            recovery_policy: AuthoritativeRecoveryPolicy::default(),
            ingress_capacity: DEFAULT_INGRESS_CAPACITY,
            schedules_initial_metadata_inventory,
            is_running: true,
            is_stopping: false,
        }
    }

    #[cfg(test)]
    pub(crate) fn with_policy<Factory>(
        factory: Factory,
        planning_limits: LibraryChangePlanningLimits,
        restart_policy: LibraryChangeRestartPolicy,
        queue_policy: LibraryChangeQueuePolicy,
        recovery_policy: AuthoritativeRecoveryPolicy,
        ingress_capacity: usize,
    ) -> Self
    where
        Factory: LibraryChangeSourceFactory,
    {
        Self {
            start_source: erase_library_change_source_factory(factory),
            roots: BTreeMap::new(),
            retiring_observers: Vec::new(),
            planning_limits,
            restart_policy,
            queue_policy,
            recovery_policy,
            ingress_capacity,
            schedules_initial_metadata_inventory: true,
            is_running: true,
            is_stopping: false,
        }
    }

    #[cfg(test)]
    pub(crate) fn poll<Repository>(
        &mut self,
        repository: &mut Repository,
        now_unix_ms: i64,
        inspect_availability: impl FnMut(&str) -> LibraryRootAvailability,
    ) -> Result<LibrarySynchronizationSnapshot, ScanError>
    where
        Repository: IncrementalCatalogRepository + LibraryChangeIngress<Reservation = Reservation>,
    {
        let catalog_roots = repository.load_incremental_catalog_roots()?;
        self.poll_internal(
            repository,
            catalog_roots,
            now_unix_ms,
            inspect_availability,
            true,
            true,
        )
    }

    #[cfg(test)]
    pub(crate) fn poll_without_authoritative_recovery<Repository>(
        &mut self,
        repository: &mut Repository,
        now_unix_ms: i64,
        inspect_availability: impl FnMut(&str) -> LibraryRootAvailability,
    ) -> Result<LibrarySynchronizationSnapshot, ScanError>
    where
        Repository: IncrementalCatalogRepository + LibraryChangeIngress<Reservation = Reservation>,
    {
        let catalog_roots = repository.load_incremental_catalog_roots()?;
        self.poll_internal(
            repository,
            catalog_roots,
            now_unix_ms,
            inspect_availability,
            false,
            false,
        )
    }

    fn poll_internal<Repository>(
        &mut self,
        repository: &mut Repository,
        catalog_roots: Vec<IncrementalCatalogRoot>,
        now_unix_ms: i64,
        mut inspect_availability: impl FnMut(&str) -> LibraryRootAvailability,
        process_authoritative_recovery: bool,
        process_live_changes: bool,
    ) -> Result<LibrarySynchronizationSnapshot, ScanError>
    where
        Repository: IncrementalCatalogRepository + LibraryChangeIngress<Reservation = Reservation>,
    {
        if !self.is_running || self.is_stopping {
            return Err(ScanError::new(
                "library_synchronization_stopped",
                "The library synchronization runtime has already stopped",
            ));
        }
        measure_observation("observer_retirement", || self.reap_retiring_observers())?;
        measure_observation("root_reconciliation", || {
            self.reconcile_roots(&catalog_roots)
        })?;

        let mut statuses = Vec::with_capacity(catalog_roots.len());
        let mut newly_retiring = Vec::new();
        let mut catalog_revision = catalog_roots
            .first()
            .map_or(0, |root| root.catalog_revision);
        let mut applied_mutation_count = 0_u32;
        let mut reserved_ingress_count = self
            .roots
            .values()
            .filter(|root| root.handoff.is_waiting_for_writer())
            .count();
        for root in catalog_roots {
            let availability = inspect_availability(&root.root_path);
            let runtime = self
                .roots
                .get_mut(&root.root_id)
                .expect("catalog roots are reconciled before processing");
            runtime.root = root.clone();
            reserved_ingress_count -= usize::from(runtime.handoff.is_waiting_for_writer());
            let can_drain_observer =
                persist_pending_plan_for_poll(runtime, repository, now_unix_ms, self.queue_policy);
            runtime.availability = availability;
            if availability != LibraryRootAvailability::Available {
                runtime.needs_continuity_gap = true;
                if let Some(mut observer) = runtime.observer.take() {
                    if let Err(error) = observer.request_stop() {
                        runtime.last_issue_code = Some(error.code);
                    }
                    newly_retiring.push(RetiringObserver {
                        root_id: root.root_id.clone(),
                        observer,
                    });
                }
                runtime.source_health = LibraryChangeSourceHealth::Stopped;
            } else {
                let has_retiring_observer = self
                    .retiring_observers
                    .iter()
                    .chain(newly_retiring.iter())
                    .any(|retiring| retiring.root_id == root.root_id);
                if runtime.observer.is_none() && !has_retiring_observer {
                    match measure_observation("observer_start", || {
                        LibraryChangeObserver::start_erased(
                            self.start_source.clone(),
                            source_request(&root, self.ingress_capacity),
                            self.planning_limits,
                            self.restart_policy,
                            now_unix_ms,
                        )
                    }) {
                        Ok(observer) => {
                            runtime.observer = Some(observer);
                            runtime.source_health = LibraryChangeSourceHealth::Starting;
                        }
                        Err(error) => {
                            runtime.source_health = LibraryChangeSourceHealth::Failed;
                            runtime.last_issue_code = Some(error.code);
                        }
                    }
                }
            }

            if can_drain_observer && let Some(observer) = runtime.observer.as_mut() {
                match measure_observation("observer_drain", || observer.poll(now_unix_ms)) {
                    Ok(poll) => {
                        runtime.source_health = poll.source_health;
                        if let Some(code) = poll.last_source_error_code {
                            runtime.last_issue_code = Some(code);
                        }
                        if !poll.planning.intents.is_empty() {
                            if planning_requires_metadata_inventory(&poll.planning) {
                                runtime.needs_continuity_gap = true;
                            } else {
                                runtime.handoff.retain(poll.planning)?;
                                persist_pending_plan_for_poll(
                                    runtime,
                                    repository,
                                    now_unix_ms,
                                    self.queue_policy,
                                );
                            }
                        }
                    }
                    Err(error) => {
                        runtime.source_health = LibraryChangeSourceHealth::Failed;
                        runtime.last_issue_code = Some(error.code);
                    }
                }
            }

            if !runtime.handoff.has_pending()
                && runtime.needs_continuity_gap
                && runtime.source_health == LibraryChangeSourceHealth::Healthy
            {
                runtime.continuity_revision =
                    runtime.continuity_revision.checked_add(1).ok_or_else(|| {
                        ScanError::new(
                            "library_continuity_revision_overflow",
                            "The library continuity revision exceeded the supported range",
                        )
                    })?;
                runtime
                    .handoff
                    .retain(continuity_gap_plan(&root, now_unix_ms))?;
                runtime.needs_continuity_gap = false;
                persist_pending_plan_for_poll(runtime, repository, now_unix_ms, self.queue_policy);
            }

            reserved_ingress_count += usize::from(runtime.handoff.is_waiting_for_writer());
            if process_authoritative_recovery
                && reserved_ingress_count == 0
                && !runtime.handoff.has_pending()
                && availability == LibraryRootAvailability::Available
                && runtime.source_health == LibraryChangeSourceHealth::Healthy
                && !runtime.needs_continuity_gap
            {
                let recovery = process_ready_authoritative_library_change(
                    repository,
                    &root.root_id,
                    root.root_generation,
                    now_unix_ms,
                    self.queue_policy,
                    self.recovery_policy,
                )?;
                catalog_revision = catalog_revision.max(recovery.incremental.catalog_revision);
                applied_mutation_count = applied_mutation_count
                    .checked_add(recovery.incremental.applied_mutation_count)
                    .ok_or_else(|| {
                        ScanError::new(
                            "library_synchronization_count_overflow",
                            "The synchronization mutation count exceeded the supported range",
                        )
                    })?;
            }
            if process_live_changes
                && availability == LibraryRootAvailability::Available
                && reserved_ingress_count == 0
            {
                let report = process_ready_library_changes_in_lane(
                    repository,
                    &root.root_id,
                    root.root_generation,
                    LibraryChangeLane::Live,
                    now_unix_ms,
                    self.queue_policy,
                )?;
                catalog_revision = catalog_revision.max(report.catalog_revision);
                applied_mutation_count = applied_mutation_count
                    .checked_add(report.applied_mutation_count)
                    .ok_or_else(|| {
                        ScanError::new(
                            "library_synchronization_count_overflow",
                            "The synchronization mutation count exceeded the supported range",
                        )
                    })?;
            }
            let metrics = measure_observation("queue_metrics", || {
                repository.load_library_change_root_queue_metrics(
                    &root.root_id,
                    root.root_generation,
                    now_unix_ms,
                    self.queue_policy,
                )
            })?;
            if root_has_converged(runtime, &metrics) {
                runtime.blocking_issue_code = None;
            }
            let mut status = project_root_status(runtime, &metrics);
            if status.freshness == CatalogFreshnessState::Synchronized {
                runtime.last_issue_code = None;
                status.last_issue_code = None;
            }
            statuses.push(status);
        }
        self.retiring_observers.extend(newly_retiring);

        Ok(LibrarySynchronizationSnapshot {
            is_running: true,
            catalog_revision,
            applied_mutation_count,
            roots: statuses,
        })
    }

    pub(crate) fn stop(&mut self) -> Result<(), ScanError> {
        self.request_stop()?;
        self.finish_stop_until(Instant::now() + OBSERVER_STOP_TIMEOUT)
    }

    pub(crate) fn request_stop(&mut self) -> Result<(), ScanError> {
        if !self.is_running && !self.is_stopping {
            return Ok(());
        }
        self.is_stopping = true;
        self.release_ingress_admissions();
        let mut first_error = None;
        for runtime in self.roots.values_mut() {
            if let Some(observer) = runtime.observer.as_mut()
                && let Err(error) = observer.request_stop()
                && first_error.is_none()
            {
                first_error = Some(ScanError::new(error.code, error.message));
            }
        }
        for retiring in &mut self.retiring_observers {
            if let Err(error) = retiring.observer.request_stop()
                && first_error.is_none()
            {
                first_error = Some(ScanError::new(error.code, error.message));
            }
        }
        if let Some(error) = first_error {
            return Err(error);
        }
        Ok(())
    }

    pub(crate) fn finish_stop_until(&mut self, deadline: Instant) -> Result<(), ScanError> {
        self.request_stop()?;
        for runtime in self.roots.values_mut() {
            if let Some(observer) = runtime.observer.as_mut() {
                observer
                    .finish_stop_until(deadline)
                    .map_err(|error| ScanError::new(error.code, error.message))?;
            }
        }
        for retiring in &mut self.retiring_observers {
            retiring
                .observer
                .finish_stop_until(deadline)
                .map_err(|error| ScanError::new(error.code, error.message))?;
        }
        for runtime in self.roots.values_mut() {
            runtime.observer = None;
            runtime.source_health = LibraryChangeSourceHealth::Stopped;
        }
        self.retiring_observers.clear();
        self.is_running = false;
        self.is_stopping = false;
        Ok(())
    }

    fn reconcile_roots(
        &mut self,
        catalog_roots: &[IncrementalCatalogRoot],
    ) -> Result<(), ScanError> {
        let current = catalog_roots
            .iter()
            .map(|root| root.root_id.as_str())
            .collect::<BTreeSet<_>>();
        let removed_root_ids = self
            .roots
            .keys()
            .filter(|root_id| !current.contains(root_id.as_str()))
            .cloned()
            .collect::<Vec<_>>();
        for root_id in removed_root_ids {
            if let Some(mut runtime) = self.roots.remove(&root_id)
                && let Some(observer) = runtime.observer.take()
            {
                self.retire_observer(root_id, observer)?;
            }
        }
        for root in catalog_roots {
            let must_replace = self.roots.get(&root.root_id).is_some_and(|runtime| {
                runtime.root.root_generation != root.root_generation
                    || runtime.root.root_path != root.root_path
            });
            if must_replace
                && let Some(mut runtime) = self.roots.remove(&root.root_id)
                && let Some(observer) = runtime.observer.take()
            {
                self.retire_observer(root.root_id.clone(), observer)?;
            }
            self.roots
                .entry(root.root_id.clone())
                .or_insert_with(|| RootRuntime {
                    root: root.clone(),
                    observer: None,
                    availability: LibraryRootAvailability::Unknown,
                    source_health: LibraryChangeSourceHealth::Starting,
                    last_issue_code: None,
                    blocking_issue_code: None,
                    persistence_contention_started_unix_ms: None,
                    recovery_contention_started_unix_ms: None,
                    handoff: ObserverHandoff::new(),
                    needs_continuity_gap: self.schedules_initial_metadata_inventory,
                    continuity_revision: 0,
                });
        }
        Ok(())
    }

    fn retire_observer(
        &mut self,
        root_id: String,
        mut observer: LibraryChangeObserver,
    ) -> Result<(), ScanError> {
        let stop = observer
            .request_stop()
            .map_err(|error| ScanError::new(error.code, error.message));
        self.retiring_observers
            .push(RetiringObserver { root_id, observer });
        stop
    }

    fn reap_retiring_observers(&mut self) -> Result<(), ScanError> {
        let mut index = 0;
        while index < self.retiring_observers.len() {
            match self.retiring_observers[index].observer.try_finish_stop() {
                Ok(Some(_)) => {
                    self.retiring_observers.swap_remove(index);
                }
                Ok(None) => index += 1,
                Err(error) => {
                    self.retiring_observers.swap_remove(index);
                    return Err(ScanError::new(error.code, error.message));
                }
            }
        }
        Ok(())
    }

    pub(crate) fn root_is_ready_for_authoritative_recovery(&self, root_id: &str) -> bool {
        self.roots.get(root_id).is_some_and(|runtime| {
            !runtime.needs_continuity_gap
                && !runtime.handoff.has_pending()
                && runtime.availability == LibraryRootAvailability::Available
                && runtime.source_health == LibraryChangeSourceHealth::Healthy
        })
    }

    pub(crate) fn has_reserved_ingress(&self) -> bool {
        self.roots
            .values()
            .any(|root| root.handoff.is_waiting_for_writer())
    }

    pub(crate) fn release_ingress_admissions(&mut self) {
        for root in self.roots.values_mut() {
            root.handoff.release_admission();
        }
    }

    pub(crate) fn root_continuity_revision(&self, root_id: &str) -> Option<u64> {
        self.roots
            .get(root_id)
            .map(|runtime| runtime.continuity_revision)
    }

    pub(crate) const fn queue_policy(&self) -> LibraryChangeQueuePolicy {
        self.queue_policy
    }

    pub(crate) const fn recovery_policy(&self) -> AuthoritativeRecoveryPolicy {
        self.recovery_policy
    }

    pub(crate) fn acknowledge_recovery_success(&mut self, root_id: &str) {
        if let Some(runtime) = self.roots.get_mut(root_id) {
            clear_recovery_contention(runtime);
        }
    }

    pub(crate) fn record_recovery_failure(&mut self, root_id: &str, code: &str, now_unix_ms: i64) {
        if let Some(runtime) = self.roots.get_mut(root_id) {
            record_recovery_issue(runtime, code, now_unix_ms);
        }
    }
}

fn record_recovery_issue<Reservation>(
    runtime: &mut RootRuntime<Reservation>,
    code: &str,
    now_unix_ms: i64,
) {
    runtime.last_issue_code = Some(code.to_owned());
    if is_transient_persistence_contention(code) {
        let started = runtime
            .recovery_contention_started_unix_ms
            .get_or_insert(now_unix_ms);
        if now_unix_ms.saturating_sub(*started) >= PERSISTENCE_CONTENTION_GRACE_MILLIS {
            runtime.blocking_issue_code = Some(code.to_owned());
        }
    } else {
        runtime.recovery_contention_started_unix_ms = None;
        runtime.blocking_issue_code = Some(code.to_owned());
    }
}

fn clear_recovery_contention<Reservation>(runtime: &mut RootRuntime<Reservation>) {
    runtime.recovery_contention_started_unix_ms = None;
    if runtime.blocking_issue_code.is_none()
        && runtime
            .last_issue_code
            .as_deref()
            .is_some_and(is_transient_persistence_contention)
    {
        runtime.last_issue_code = None;
    }
}

fn persist_pending_plan_for_poll<Repository, Reservation>(
    runtime: &mut RootRuntime<Reservation>,
    repository: &mut Repository,
    now_unix_ms: i64,
    queue_policy: LibraryChangeQueuePolicy,
) -> bool
where
    Repository: LibraryChangeIngress<Reservation = Reservation>,
{
    match measure_observation("queue_persistence", || {
        runtime
            .handoff
            .persist(repository, now_unix_ms, queue_policy)
    }) {
        Ok(()) => {
            runtime.persistence_contention_started_unix_ms = None;
            if runtime.blocking_issue_code.is_none()
                && runtime
                    .last_issue_code
                    .as_deref()
                    .is_some_and(is_retryable_plan_persistence)
            {
                runtime.last_issue_code = None;
            }
            true
        }
        Err(error) => {
            runtime.last_issue_code = Some(error.code.clone());
            let is_transient = is_transient_persistence_contention(&error.code);
            let is_backpressured = error.code == "change_queue_backpressure";
            if is_backpressured {
                runtime.persistence_contention_started_unix_ms = None;
            } else if is_transient {
                let started = runtime
                    .persistence_contention_started_unix_ms
                    .get_or_insert(now_unix_ms);
                if now_unix_ms.saturating_sub(*started) >= PERSISTENCE_CONTENTION_GRACE_MILLIS {
                    runtime.blocking_issue_code = Some(error.code.clone());
                }
            } else {
                runtime.persistence_contention_started_unix_ms = None;
                runtime.blocking_issue_code = Some(error.code.clone());
            }
            #[cfg(debug_assertions)]
            eprintln!(
                "[Ame sync] queue persistence result={} root={} code={} message={}",
                if is_backpressured || is_transient && runtime.blocking_issue_code.is_none() {
                    "retrying"
                } else {
                    "failed"
                },
                runtime.root.root_id,
                error.code,
                error.message.replace(['\r', '\n'], " ")
            );
            false
        }
    }
}

fn is_transient_persistence_contention(code: &str) -> bool {
    matches!(code, "catalog_database_busy" | "catalog_database_locked")
}

fn is_retryable_plan_persistence(code: &str) -> bool {
    code == "change_queue_backpressure" || is_transient_persistence_contention(code)
}

fn root_has_converged<Reservation>(
    runtime: &RootRuntime<Reservation>,
    metrics: &LibraryChangeQueueMetrics,
) -> bool {
    runtime.blocking_issue_code.is_some()
        && runtime.availability == LibraryRootAvailability::Available
        && runtime.source_health == LibraryChangeSourceHealth::Healthy
        && !runtime.needs_continuity_gap
        && !runtime.handoff.has_pending()
        && !runtime.root.has_running_scan
        && runtime.root.active_scan_id.is_some()
        && metrics.health != LibraryChangeQueueHealth::Degraded
        && metrics.pending_count == 0
        && metrics.leased_count == 0
        && metrics.retry_wait_count == 0
}

fn continuity_gap_plan(
    root: &IncrementalCatalogRoot,
    observed_unix_ms: i64,
) -> LibraryChangePlanningResult {
    LibraryChangePlanningResult {
        root_id: root.root_id.clone(),
        root_generation: root.root_generation,
        freshness: CatalogFreshnessState::NeedsReconciliation,
        freshness_cause: CatalogFreshnessCause::EvidenceGap,
        intents: vec![LibraryChangeIntent {
            root_id: root.root_id.clone(),
            root_generation: root.root_generation,
            kind: LibraryChangeIntentKind::FreshnessUnknown,
            scope: LibraryChangeScope::Root,
            relative_path: String::new(),
            previous_relative_path: None,
            origin: LibraryChangeOrigin::LiveNotification,
            first_observed_unix_ms: observed_unix_ms,
            most_recent_observed_unix_ms: observed_unix_ms,
            first_sequence: 1,
            most_recent_sequence: 1,
            coalesced_observation_count: 1,
        }],
        issues: vec![LibraryChangePlanningIssue::ChangeEvidenceGap],
        received_observation_count: 1,
        superseded_observation_count: 0,
    }
}

fn planning_requires_metadata_inventory(plan: &LibraryChangePlanningResult) -> bool {
    plan.issues
        .contains(&LibraryChangePlanningIssue::ChangeEvidenceGap)
        && plan.intents.iter().any(|intent| {
            intent.kind == LibraryChangeIntentKind::FreshnessUnknown
                && intent.scope == LibraryChangeScope::Root
        })
}

impl<Reservation> Drop for LibrarySynchronizationRuntime<Reservation> {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

fn source_request(
    root: &IncrementalCatalogRoot,
    ingress_capacity: usize,
) -> LibraryChangeSourceRequest {
    LibraryChangeSourceRequest {
        root_id: root.root_id.clone(),
        root_generation: root.root_generation,
        root_path: root.root_path.clone().into(),
        ingress_capacity,
    }
}

fn project_root_status<Reservation>(
    runtime: &RootRuntime<Reservation>,
    metrics: &LibraryChangeQueueMetrics,
) -> LibraryRootSynchronizationStatus {
    let unresolved = metrics
        .pending_count
        .saturating_add(metrics.leased_count)
        .saturating_add(metrics.retry_wait_count);
    let (freshness, freshness_cause, phase) = if runtime.availability
        != LibraryRootAvailability::Available
    {
        (
            CatalogFreshnessState::Unavailable,
            CatalogFreshnessCause::RootUnavailable,
            LibrarySynchronizationPhase::Unavailable,
        )
    } else if matches!(
        runtime.source_health,
        LibraryChangeSourceHealth::Degraded
            | LibraryChangeSourceHealth::Failed
            | LibraryChangeSourceHealth::Stopped
            | LibraryChangeSourceHealth::Unsupported
    ) {
        (
            CatalogFreshnessState::NeedsReconciliation,
            CatalogFreshnessCause::ChangeSourceUnhealthy,
            LibrarySynchronizationPhase::Blocked,
        )
    } else if runtime.blocking_issue_code.is_some() || metrics.explicit_recovery_required_count > 0
    {
        (
            CatalogFreshnessState::NeedsReconciliation,
            CatalogFreshnessCause::EvidenceGap,
            LibrarySynchronizationPhase::Blocked,
        )
    } else if runtime.root.has_running_scan {
        (
            CatalogFreshnessState::Updating,
            CatalogFreshnessCause::PendingChanges,
            LibrarySynchronizationPhase::FullScan,
        )
    } else if metrics.health == LibraryChangeQueueHealth::Degraded {
        (
            CatalogFreshnessState::NeedsReconciliation,
            CatalogFreshnessCause::EvidenceGap,
            LibrarySynchronizationPhase::Blocked,
        )
    } else if runtime.needs_continuity_gap
        || runtime.source_health == LibraryChangeSourceHealth::Starting
    {
        (
            CatalogFreshnessState::Updating,
            CatalogFreshnessCause::PendingChanges,
            LibrarySynchronizationPhase::WatcherStartup,
        )
    } else if metrics.retry_wait_count > 0
        && metrics.pending_count == 0
        && metrics.leased_count == 0
    {
        (
            CatalogFreshnessState::Updating,
            CatalogFreshnessCause::PendingChanges,
            LibrarySynchronizationPhase::RetryWait,
        )
    } else if runtime.root.active_scan_id.is_none()
        || unresolved > 0
        || runtime.handoff.has_pending()
    {
        (
            CatalogFreshnessState::Updating,
            CatalogFreshnessCause::PendingChanges,
            LibrarySynchronizationPhase::QueuePublication,
        )
    } else {
        (
            CatalogFreshnessState::Synchronized,
            CatalogFreshnessCause::NoPendingChanges,
            LibrarySynchronizationPhase::Synchronized,
        )
    };
    let last_issue_code = if metrics.explicit_recovery_required_count > 0 {
        Some("live_gap_v30_explicit_recovery_required".to_owned())
    } else if metrics.exhausted_retry_count > 0 {
        metrics
            .latest_exhausted_failure_code
            .clone()
            .or_else(|| runtime.blocking_issue_code.clone())
            .or_else(|| runtime.last_issue_code.clone())
    } else {
        runtime.last_issue_code.clone()
    };
    LibraryRootSynchronizationStatus {
        root_id: runtime.root.root_id.clone(),
        root_generation: runtime.root.root_generation.value(),
        availability: runtime.availability,
        freshness,
        freshness_cause,
        continuity: match freshness {
            CatalogFreshnessState::Synchronized => PersistentJournalContinuityState::Current,
            CatalogFreshnessState::Updating => PersistentJournalContinuityState::CatchingUp,
            CatalogFreshnessState::NeedsReconciliation => {
                PersistentJournalContinuityState::RecoveryRequired
            }
            CatalogFreshnessState::Unavailable => PersistentJournalContinuityState::Unavailable,
        },
        phase,
        source_health: runtime.source_health,
        queue_health: metrics.health,
        pending_change_count: metrics.pending_count.saturating_add(metrics.leased_count),
        retry_wait_count: metrics.retry_wait_count,
        freshness_unknown_count: metrics.freshness_unknown_count,
        recovery_blocked: runtime.blocking_issue_code.is_some()
            || metrics.exhausted_retry_count > 0
            || metrics.explicit_recovery_required_count > 0,
        last_issue_code,
    }
}

#[cfg(test)]
#[path = "../../test_support/r2c_r_change_driven_reliability_acceptance.rs"]
mod change_driven_reliability_acceptance;
#[cfg(test)]
#[path = "../../test_support/r2c_h_reliability_acceptance.rs"]
mod reliability_acceptance;
#[cfg(test)]
#[path = "../../test_support/r2c_m_replacement_reliability_acceptance.rs"]
mod replacement_reliability_acceptance;
#[cfg(test)]
mod tests;
