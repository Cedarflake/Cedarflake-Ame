use std::collections::{BTreeMap, BTreeSet};

use crate::domain::{
    CatalogFreshnessCause, CatalogFreshnessState, IncrementalCatalogRoot,
    LibraryChangePlanningLimits, LibraryChangeQueueHealth, LibraryChangeQueueMetrics,
    LibraryChangeQueuePolicy, LibraryChangeRestartPolicy, LibraryChangeSourceHealth,
    LibraryRootAvailability, LibraryRootSynchronizationStatus, LibrarySynchronizationSnapshot,
    ScanError,
};
use crate::ports::{
    IncrementalCatalogRepository, LibraryChangeQueue, LibraryChangeSourceRequest,
    LibraryChangeSourceStarter,
};
#[cfg(test)]
use crate::ports::{LibraryChangeSourceFactory, erase_library_change_source_factory};

use super::library_change_observer::LibraryChangeObserver;
use super::{enqueue_library_change_plan, process_ready_library_changes};

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
}

pub(crate) struct LibrarySynchronizationRuntime {
    start_source: LibraryChangeSourceStarter,
    roots: BTreeMap<String, RootRuntime>,
    planning_limits: LibraryChangePlanningLimits,
    restart_policy: LibraryChangeRestartPolicy,
    queue_policy: LibraryChangeQueuePolicy,
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
            runtime.availability = availability;
            if availability != LibraryRootAvailability::Available {
                if let Some(mut observer) = runtime.observer.take()
                    && let Err(error) = observer.stop()
                {
                    runtime.last_issue_code = Some(error.code);
                }
                runtime.source_health = LibraryChangeSourceHealth::Stopped;
            } else if runtime.observer.is_none() {
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

            if let Some(observer) = runtime.observer.as_mut() {
                match observer.poll(now_unix_ms) {
                    Ok(poll) => {
                        runtime.source_health = poll.source_health;
                        runtime.last_issue_code = poll.last_source_error_code.clone();
                        if !poll.planning.intents.is_empty() {
                            enqueue_library_change_plan(
                                repository,
                                &poll.planning,
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
                });
        }
    }
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
