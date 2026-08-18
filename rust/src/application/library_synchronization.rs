use std::collections::{BTreeMap, BTreeSet};

use crate::domain::{
    CatalogFreshnessCause, CatalogFreshnessState, IncrementalCatalogRoot, LibraryChangeIntent,
    LibraryChangeIntentKind, LibraryChangeOrigin, LibraryChangePlanningIssue,
    LibraryChangePlanningLimits, LibraryChangePlanningResult, LibraryChangeQueueHealth,
    LibraryChangeQueueMetrics, LibraryChangeQueuePolicy, LibraryChangeRestartPolicy,
    LibraryChangeScope, LibraryChangeSourceHealth, LibraryRootAvailability,
    LibraryRootSynchronizationStatus, LibrarySynchronizationSnapshot, ScanError,
};
use crate::ports::{
    IncrementalCatalogRepository, LibraryChangeQueue, LibraryChangeSourceRequest,
    LibraryChangeSourceStarter,
};
#[cfg(test)]
use crate::ports::{LibraryChangeSourceFactory, erase_library_change_source_factory};

use super::library_change_observer::LibraryChangeObserver;
use super::{
    AuthoritativeRecoveryPolicy, FullScanRecoveryRequest, enqueue_library_change_plan,
    process_ready_authoritative_library_change, process_ready_library_changes,
};

mod production;

pub(crate) use production::{
    poll_production_library_synchronization, start_production_library_synchronization,
    stop_production_library_synchronization,
};

const DEFAULT_INGRESS_CAPACITY: usize = 4_096;

struct RootRuntime {
    root: IncrementalCatalogRoot,
    observer: Option<LibraryChangeObserver>,
    availability: LibraryRootAvailability,
    source_health: LibraryChangeSourceHealth,
    last_issue_code: Option<String>,
    pending_plan: Option<LibraryChangePlanningResult>,
    needs_continuity_gap: bool,
    pending_full_scan: Option<FullScanRecoveryRequest>,
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
    pub(crate) fn new_erased(start_source: LibraryChangeSourceStarter) -> Self {
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

    pub(crate) fn poll<Repository>(
        &mut self,
        repository: &mut Repository,
        now_unix_ms: i64,
        mut inspect_availability: impl FnMut(&str) -> LibraryRootAvailability,
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
            persist_pending_plan(runtime, repository, now_unix_ms, self.queue_policy)?;
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
                if runtime.needs_continuity_gap {
                    runtime.pending_plan = Some(continuity_gap_plan(&root, now_unix_ms));
                    runtime.needs_continuity_gap = false;
                    persist_pending_plan(runtime, repository, now_unix_ms, self.queue_policy)?;
                }
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
                            runtime.last_issue_code = None;
                        }
                        Err(error) => {
                            runtime.source_health = LibraryChangeSourceHealth::Failed;
                            runtime.last_issue_code = Some(error.code);
                        }
                    }
                }
            }

            if let Some(observer) = runtime.observer.as_mut() {
                match observer.poll(now_unix_ms) {
                    Ok(poll) => {
                        runtime.source_health = poll.source_health;
                        runtime.last_issue_code = poll.last_source_error_code.clone();
                        if !poll.planning.intents.is_empty() {
                            runtime.pending_plan = Some(poll.planning);
                            persist_pending_plan(
                                runtime,
                                repository,
                                now_unix_ms,
                                self.queue_policy,
                            )?;
                        }
                    }
                    Err(error) => {
                        runtime.source_health = LibraryChangeSourceHealth::Failed;
                        runtime.last_issue_code = Some(error.code);
                    }
                }
            }

            if availability == LibraryRootAvailability::Available {
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
                if recovery.full_scan.is_some() {
                    runtime.pending_full_scan = recovery.full_scan;
                }
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
            let mut metrics = repository.load_library_change_root_queue_metrics(
                &root.root_id,
                root.root_generation,
                now_unix_ms,
                self.queue_policy,
            )?;
            if availability == LibraryRootAvailability::Available
                && runtime.source_health == LibraryChangeSourceHealth::Healthy
                && metrics.pending_count == 0
                && metrics.leased_count == 0
                && metrics.retry_wait_count == 0
                && runtime.pending_full_scan.is_none()
                && let Some(refreshed_root) =
                    repository.load_incremental_catalog_root(&root.root_id)?
                && consistency_audit_is_due(
                    &refreshed_root,
                    now_unix_ms,
                    self.recovery_policy.audit_interval_millis,
                )
            {
                runtime.pending_plan = Some(consistency_audit_plan(&refreshed_root, now_unix_ms));
                persist_pending_plan(runtime, repository, now_unix_ms, self.queue_policy)?;
                metrics = repository.load_library_change_root_queue_metrics(
                    &root.root_id,
                    root.root_generation,
                    now_unix_ms,
                    self.queue_policy,
                )?;
            }
            statuses.push(project_root_status(runtime, &metrics));
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
                    pending_plan: None,
                    needs_continuity_gap: true,
                    pending_full_scan: None,
                });
        }
    }

    pub(crate) fn pending_full_scan_requests(&self) -> Vec<FullScanRecoveryRequest> {
        self.roots
            .values()
            .filter_map(|runtime| runtime.pending_full_scan.clone())
            .collect()
    }

    pub(crate) fn acknowledge_full_scan_started(
        &mut self,
        root_id: &str,
        queue_high_watermark: crate::domain::LibraryChangeId,
    ) {
        let Some(runtime) = self.roots.get_mut(root_id) else {
            return;
        };
        if runtime
            .pending_full_scan
            .as_ref()
            .is_some_and(|request| request.queue_high_watermark == queue_high_watermark)
        {
            runtime.pending_full_scan = None;
        }
    }

    pub(crate) fn record_full_scan_failure(&mut self, root_id: &str, code: String) {
        if let Some(runtime) = self.roots.get_mut(root_id) {
            runtime.last_issue_code = Some(code);
        }
    }
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

fn consistency_audit_plan(
    root: &IncrementalCatalogRoot,
    observed_unix_ms: i64,
) -> LibraryChangePlanningResult {
    LibraryChangePlanningResult {
        root_id: root.root_id.clone(),
        root_generation: root.root_generation,
        freshness: CatalogFreshnessState::Updating,
        freshness_cause: CatalogFreshnessCause::PendingChanges,
        intents: vec![LibraryChangeIntent {
            root_id: root.root_id.clone(),
            root_generation: root.root_generation,
            kind: LibraryChangeIntentKind::Reconcile,
            scope: LibraryChangeScope::Root,
            relative_path: String::new(),
            previous_relative_path: None,
            origin: LibraryChangeOrigin::ConsistencyAudit,
            first_observed_unix_ms: observed_unix_ms,
            most_recent_observed_unix_ms: observed_unix_ms,
            first_sequence: 1,
            most_recent_sequence: 1,
            coalesced_observation_count: 1,
        }],
        issues: Vec::new(),
        received_observation_count: 1,
        superseded_observation_count: 0,
    }
}

fn consistency_audit_is_due(
    root: &IncrementalCatalogRoot,
    now_unix_ms: i64,
    interval_millis: u64,
) -> bool {
    if root.has_running_scan || root.active_scan_id.is_none() {
        return false;
    }
    let interval = i64::try_from(interval_millis).unwrap_or(i64::MAX);
    root.last_consistency_audit_unix_ms
        .is_none_or(|last| now_unix_ms.saturating_sub(last) >= interval)
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
    let (freshness, freshness_cause) = if runtime.availability != LibraryRootAvailability::Available
    {
        (
            CatalogFreshnessState::Unavailable,
            CatalogFreshnessCause::RootUnavailable,
        )
    } else if metrics.freshness_unknown_count > 0
        || metrics.health == LibraryChangeQueueHealth::Degraded
    {
        (
            CatalogFreshnessState::NeedsReconciliation,
            CatalogFreshnessCause::EvidenceGap,
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
        )
    } else if runtime.root.has_running_scan
        || runtime.root.active_scan_id.is_none()
        || unresolved > 0
        || runtime.source_health == LibraryChangeSourceHealth::Starting
    {
        (
            CatalogFreshnessState::Updating,
            CatalogFreshnessCause::PendingChanges,
        )
    } else {
        (
            CatalogFreshnessState::Synchronized,
            CatalogFreshnessCause::NoPendingChanges,
        )
    };
    LibraryRootSynchronizationStatus {
        root_id: runtime.root.root_id.clone(),
        root_generation: runtime.root.root_generation.value(),
        availability: runtime.availability,
        freshness,
        freshness_cause,
        source_health: runtime.source_health,
        queue_health: metrics.health,
        pending_change_count: metrics.pending_count.saturating_add(metrics.leased_count),
        retry_wait_count: metrics.retry_wait_count,
        freshness_unknown_count: metrics.freshness_unknown_count,
        last_issue_code: runtime.last_issue_code.clone(),
    }
}

#[cfg(test)]
mod tests;
