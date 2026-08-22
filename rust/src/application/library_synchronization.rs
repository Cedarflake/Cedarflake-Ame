use std::collections::{BTreeMap, BTreeSet};

use crate::domain::{
    CatalogFreshnessCause, CatalogFreshnessState, IncrementalCatalogRoot, LibraryChangeIntent,
    LibraryChangeIntentKind, LibraryChangeOrigin, LibraryChangePlanningIssue,
    LibraryChangePlanningLimits, LibraryChangePlanningResult, LibraryChangeQueueHealth,
    LibraryChangeQueueMetrics, LibraryChangeQueuePolicy, LibraryChangeRestartPolicy,
    LibraryChangeScope, LibraryChangeSourceHealth, LibraryRootAvailability,
    LibraryRootSynchronizationStatus, LibrarySynchronizationPhase, LibrarySynchronizationSnapshot,
    ScanError,
};
use crate::ports::{
    IncrementalCatalogRepository, LibraryChangeQueue, LibraryChangeSourceRequest,
    LibraryChangeSourceStarter,
};
#[cfg(test)]
use crate::ports::{LibraryChangeSourceFactory, erase_library_change_source_factory};

use super::authoritative_library_changes::process_ready_authoritative_library_change;
use super::library_change_observer::LibraryChangeObserver;
use super::{
    AuthoritativeRecoveryPolicy, enqueue_library_change_plan, process_ready_library_changes,
};

mod production;

pub(crate) use production::{
    poll_production_library_synchronization, start_production_library_synchronization,
    stop_production_library_synchronization,
};

const DEFAULT_INGRESS_CAPACITY: usize = 4_096;
const PERSISTENCE_CONTENTION_GRACE_MILLIS: i64 = 30_000;

struct RootRuntime {
    root: IncrementalCatalogRoot,
    observer: Option<LibraryChangeObserver>,
    availability: LibraryRootAvailability,
    source_health: LibraryChangeSourceHealth,
    last_issue_code: Option<String>,
    blocking_issue_code: Option<String>,
    persistence_contention_started_unix_ms: Option<i64>,
    recovery_contention_started_unix_ms: Option<i64>,
    pending_plan: Option<LibraryChangePlanningResult>,
    needs_continuity_gap: bool,
    continuity_revision: u64,
}

pub(crate) struct LibrarySynchronizationRuntime {
    start_source: LibraryChangeSourceStarter,
    roots: BTreeMap<String, RootRuntime>,
    planning_limits: LibraryChangePlanningLimits,
    restart_policy: LibraryChangeRestartPolicy,
    queue_policy: LibraryChangeQueuePolicy,
    recovery_policy: AuthoritativeRecoveryPolicy,
    ingress_capacity: usize,
    is_running: bool,
}

impl LibrarySynchronizationRuntime {
    #[cfg(test)]
    pub(crate) fn new_erased(start_source: LibraryChangeSourceStarter) -> Self {
        Self::new(start_source)
    }

    pub(crate) fn new_production(start_source: LibraryChangeSourceStarter) -> Self {
        Self::new(start_source)
    }

    fn new(start_source: LibraryChangeSourceStarter) -> Self {
        Self {
            start_source,
            roots: BTreeMap::new(),
            planning_limits: LibraryChangePlanningLimits::default(),
            restart_policy: LibraryChangeRestartPolicy::default(),
            queue_policy: LibraryChangeQueuePolicy::default(),
            recovery_policy: AuthoritativeRecoveryPolicy::default(),
            ingress_capacity: DEFAULT_INGRESS_CAPACITY,
            is_running: true,
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
            planning_limits,
            restart_policy,
            queue_policy,
            recovery_policy,
            ingress_capacity,
            is_running: true,
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
        Repository: IncrementalCatalogRepository + LibraryChangeQueue,
    {
        self.poll_internal(repository, now_unix_ms, inspect_availability, true)
    }

    pub(crate) fn poll_without_authoritative_recovery<Repository>(
        &mut self,
        repository: &mut Repository,
        now_unix_ms: i64,
        inspect_availability: impl FnMut(&str) -> LibraryRootAvailability,
    ) -> Result<LibrarySynchronizationSnapshot, ScanError>
    where
        Repository: IncrementalCatalogRepository + LibraryChangeQueue,
    {
        self.poll_internal(repository, now_unix_ms, inspect_availability, false)
    }

    fn poll_internal<Repository>(
        &mut self,
        repository: &mut Repository,
        now_unix_ms: i64,
        mut inspect_availability: impl FnMut(&str) -> LibraryRootAvailability,
        process_authoritative_recovery: bool,
    ) -> Result<LibrarySynchronizationSnapshot, ScanError>
    where
        Repository: IncrementalCatalogRepository + LibraryChangeQueue,
    {
        if !self.is_running {
            return Err(ScanError::new(
                "library_synchronization_stopped",
                "The library synchronization runtime has already stopped",
            ));
        }
        let catalog_roots = repository.load_incremental_catalog_roots()?;
        self.reconcile_roots(&catalog_roots);

        let mut statuses = Vec::with_capacity(catalog_roots.len());
        let mut catalog_revision = catalog_roots
            .first()
            .map_or(0, |root| root.catalog_revision);
        let mut applied_mutation_count = 0_u32;
        for root in catalog_roots {
            let availability = inspect_availability(&root.root_path);
            let runtime = self
                .roots
                .get_mut(&root.root_id)
                .expect("catalog roots are reconciled before processing");
            runtime.root = root.clone();
            let can_drain_observer =
                persist_pending_plan_for_poll(runtime, repository, now_unix_ms, self.queue_policy);
            runtime.availability = availability;
            if availability != LibraryRootAvailability::Available {
                runtime.needs_continuity_gap = true;
                if let Some(mut observer) = runtime.observer.take()
                    && let Err(error) = observer.stop()
                {
                    runtime.last_issue_code = Some(error.code);
                }
                runtime.source_health = LibraryChangeSourceHealth::Stopped;
            } else {
                if runtime.observer.is_none() {
                    match LibraryChangeObserver::start_erased(
                        self.start_source.clone(),
                        source_request(&root, self.ingress_capacity),
                        self.planning_limits,
                        self.restart_policy,
                        now_unix_ms,
                    ) {
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
                match observer.poll(now_unix_ms) {
                    Ok(poll) => {
                        runtime.source_health = poll.source_health;
                        if let Some(code) = poll.last_source_error_code {
                            runtime.last_issue_code = Some(code);
                        }
                        if !poll.planning.intents.is_empty() {
                            if planning_requires_metadata_inventory(&poll.planning) {
                                runtime.needs_continuity_gap = true;
                            } else {
                                runtime.pending_plan = Some(poll.planning);
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

            if can_drain_observer
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
                runtime.pending_plan = Some(continuity_gap_plan(&root, now_unix_ms));
                runtime.needs_continuity_gap = false;
                persist_pending_plan_for_poll(runtime, repository, now_unix_ms, self.queue_policy);
            }

            if process_authoritative_recovery
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
            if availability == LibraryRootAvailability::Available {
                let report = process_ready_library_changes(
                    repository,
                    &root.root_id,
                    root.root_generation,
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
            let metrics = repository.load_library_change_root_queue_metrics(
                &root.root_id,
                root.root_generation,
                now_unix_ms,
                self.queue_policy,
            )?;
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

        Ok(LibrarySynchronizationSnapshot {
            is_running: true,
            catalog_revision,
            applied_mutation_count,
            roots: statuses,
        })
    }

    pub(crate) fn stop(&mut self) -> Result<(), ScanError> {
        if !self.is_running {
            return Ok(());
        }
        self.is_running = false;
        let mut first_error = None;
        for runtime in self.roots.values_mut() {
            if let Some(mut observer) = runtime.observer.take()
                && let Err(error) = observer.stop()
                && first_error.is_none()
            {
                first_error = Some(ScanError::new(error.code, error.message));
            }
            runtime.source_health = LibraryChangeSourceHealth::Stopped;
        }
        if let Some(error) = first_error {
            return Err(error);
        }
        Ok(())
    }

    fn reconcile_roots(&mut self, catalog_roots: &[IncrementalCatalogRoot]) {
        let current = catalog_roots
            .iter()
            .map(|root| root.root_id.as_str())
            .collect::<BTreeSet<_>>();
        self.roots.retain(|root_id, runtime| {
            if current.contains(root_id.as_str()) {
                return true;
            }
            if let Some(mut observer) = runtime.observer.take() {
                let _ = observer.stop();
            }
            false
        });
        for root in catalog_roots {
            let must_replace = self.roots.get(&root.root_id).is_some_and(|runtime| {
                runtime.root.root_generation != root.root_generation
                    || runtime.root.root_path != root.root_path
            });
            if must_replace
                && let Some(mut runtime) = self.roots.remove(&root.root_id)
                && let Some(mut observer) = runtime.observer.take()
            {
                let _ = observer.stop();
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
                    pending_plan: None,
                    needs_continuity_gap: true,
                    continuity_revision: 0,
                });
        }
    }

    pub(crate) fn root_is_ready_for_authoritative_recovery(&self, root_id: &str) -> bool {
        self.roots.get(root_id).is_some_and(|runtime| {
            !runtime.needs_continuity_gap
                && runtime.availability == LibraryRootAvailability::Available
                && runtime.source_health == LibraryChangeSourceHealth::Healthy
        })
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

fn record_recovery_issue(runtime: &mut RootRuntime, code: &str, now_unix_ms: i64) {
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

fn clear_recovery_contention(runtime: &mut RootRuntime) {
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

fn persist_pending_plan_for_poll<Repository>(
    runtime: &mut RootRuntime,
    repository: &mut Repository,
    now_unix_ms: i64,
    queue_policy: LibraryChangeQueuePolicy,
) -> bool
where
    Repository: LibraryChangeQueue,
{
    match persist_pending_plan(runtime, repository, now_unix_ms, queue_policy) {
        Ok(()) => {
            runtime.persistence_contention_started_unix_ms = None;
            if runtime.blocking_issue_code.is_none()
                && runtime
                    .last_issue_code
                    .as_deref()
                    .is_some_and(is_transient_persistence_contention)
            {
                runtime.last_issue_code = None;
            }
            true
        }
        Err(error) => {
            let is_transient = is_transient_persistence_contention(&error.code);
            if is_transient {
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
                if is_transient && runtime.blocking_issue_code.is_none() {
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

fn root_has_converged(runtime: &RootRuntime, metrics: &LibraryChangeQueueMetrics) -> bool {
    runtime.blocking_issue_code.is_some()
        && runtime.availability == LibraryRootAvailability::Available
        && runtime.source_health == LibraryChangeSourceHealth::Healthy
        && !runtime.needs_continuity_gap
        && runtime.pending_plan.is_none()
        && !runtime.root.has_running_scan
        && runtime.root.active_scan_id.is_some()
        && metrics.health != LibraryChangeQueueHealth::Degraded
        && metrics.pending_count == 0
        && metrics.leased_count == 0
        && metrics.retry_wait_count == 0
}

fn persist_pending_plan<Repository>(
    runtime: &mut RootRuntime,
    repository: &mut Repository,
    now_unix_ms: i64,
    queue_policy: LibraryChangeQueuePolicy,
) -> Result<(), ScanError>
where
    Repository: LibraryChangeQueue,
{
    let Some(plan) = runtime.pending_plan.take() else {
        return Ok(());
    };
    match enqueue_library_change_plan(repository, &plan, now_unix_ms, queue_policy) {
        Ok(_) => Ok(()),
        Err(error) => {
            runtime.last_issue_code = Some(error.code.clone());
            runtime.pending_plan = Some(plan);
            Err(error)
        }
    }
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
            origin: LibraryChangeOrigin::StartupCatchUp,
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

impl Drop for LibrarySynchronizationRuntime {
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

fn project_root_status(
    runtime: &RootRuntime,
    metrics: &LibraryChangeQueueMetrics,
) -> LibraryRootSynchronizationStatus {
    let unresolved = metrics
        .pending_count
        .saturating_add(metrics.leased_count)
        .saturating_add(metrics.retry_wait_count);
    let (freshness, freshness_cause, phase) =
        if runtime.availability != LibraryRootAvailability::Available {
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
        } else if runtime.blocking_issue_code.is_some() {
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
        } else if runtime.root.active_scan_id.is_none() || unresolved > 0 {
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
    let last_issue_code = if metrics.exhausted_retry_count > 0 {
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
        phase,
        source_health: runtime.source_health,
        queue_health: metrics.health,
        pending_change_count: metrics.pending_count.saturating_add(metrics.leased_count),
        retry_wait_count: metrics.retry_wait_count,
        freshness_unknown_count: metrics.freshness_unknown_count,
        recovery_blocked: runtime.blocking_issue_code.is_some()
            || metrics.exhausted_retry_count > 0,
        last_issue_code,
    }
}

#[cfg(test)]
#[path = "../../test_support/r2c_h_reliability_acceptance.rs"]
mod reliability_acceptance;
#[cfg(test)]
#[path = "../../test_support/r2c_m_replacement_reliability_acceptance.rs"]
mod replacement_reliability_acceptance;
#[cfg(test)]
mod tests;
