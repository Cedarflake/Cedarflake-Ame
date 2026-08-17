//! Platform-independent continuous-library synchronization contracts.

#[cfg(windows)]
use std::path::PathBuf;

#[cfg(windows)]
use crate::adapters::WindowsLibraryChangeSourceFactory;
#[cfg(windows)]
use crate::application::LibraryChangeObserver;
#[cfg(windows)]
use crate::ports::LibraryChangeSourceRequest;

pub use crate::application::{
    enqueue_library_change_plan, plan_library_changes, reconcile_path_evidence,
};
pub use crate::domain::{
    CatalogFreshnessCause, CatalogFreshnessState, DerivedEvidenceDisposition, FileIdentityEvidence,
    IncrementalReconciliationDecision, IncrementalReconciliationOutcome, LibraryChangeIntent,
    LibraryChangeIntentKind, LibraryChangeObservation, LibraryChangeObservationKind,
    LibraryChangeOrigin, LibraryChangePlanningContext, LibraryChangePlanningError,
    LibraryChangePlanningIssue, LibraryChangePlanningLimits, LibraryChangePlanningResult,
    LibraryChangeScope, LibraryChangeSourceHealth, LibraryRootAvailability, LibraryRootGeneration,
    ReconciliationFileEvidence, ReconciliationObservedState,
};
pub use crate::domain::{
    DurableLibraryChange, LeasedLibraryChange, LibraryChangeEnqueueReport, LibraryChangeFailure,
    LibraryChangeId, LibraryChangeLeaseUpdateOutcome, LibraryChangeObserverPoll,
    LibraryChangeQueueHealth, LibraryChangeQueueMetrics, LibraryChangeQueuePolicy,
    LibraryChangeQueueStatus, LibraryChangeRestartPolicy, LibraryChangeSourceError,
    LibraryChangeSourceStopReport, ScanError,
};
pub use crate::ports::LibraryChangeQueue;

#[cfg(windows)]
pub struct WindowsLibraryChangeObserver {
    inner: LibraryChangeObserver<WindowsLibraryChangeSourceFactory>,
}

#[cfg(windows)]
impl WindowsLibraryChangeObserver {
    pub fn start(
        root_id: String,
        root_generation: LibraryRootGeneration,
        root_path: String,
        ingress_capacity: usize,
        planning_limits: LibraryChangePlanningLimits,
        restart_policy: LibraryChangeRestartPolicy,
        now_unix_ms: i64,
    ) -> Result<Self, LibraryChangeSourceError> {
        let inner = LibraryChangeObserver::start(
            WindowsLibraryChangeSourceFactory,
            LibraryChangeSourceRequest {
                root_id,
                root_generation,
                root_path: PathBuf::from(root_path),
                ingress_capacity,
            },
            planning_limits,
            restart_policy,
            now_unix_ms,
        )?;
        Ok(Self { inner })
    }

    pub fn poll(
        &mut self,
        now_unix_ms: i64,
    ) -> Result<LibraryChangeObserverPoll, LibraryChangeSourceError> {
        self.inner.poll(now_unix_ms)
    }

    pub fn stop(&mut self) -> Result<LibraryChangeSourceStopReport, LibraryChangeSourceError> {
        self.inner.stop()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CatalogFreshnessState, LibraryChangeIntentKind, LibraryChangeObservation,
        LibraryChangeObservationKind, LibraryChangeOrigin, LibraryChangePlanningContext,
        LibraryChangePlanningLimits, LibraryChangeScope, LibraryChangeSourceHealth,
        LibraryRootAvailability, LibraryRootGeneration, plan_library_changes,
    };

    #[test]
    fn facade_plans_platform_independent_observations() {
        let generation = LibraryRootGeneration::initial();
        let context = LibraryChangePlanningContext {
            root_id: "root-a".to_owned(),
            root_generation: generation,
            availability: LibraryRootAvailability::Available,
            source_health: LibraryChangeSourceHealth::Healthy,
        };
        let observation = LibraryChangeObservation {
            root_id: context.root_id.clone(),
            root_generation: generation,
            sequence: 1,
            observed_unix_ms: 1_000,
            kind: LibraryChangeObservationKind::Created,
            scope: LibraryChangeScope::Path,
            relative_path: "相册\\照片.jpg".to_owned(),
            previous_relative_path: None,
            origin: LibraryChangeOrigin::LiveNotification,
        };

        let result = plan_library_changes(
            &context,
            [observation],
            LibraryChangePlanningLimits::default(),
        )
        .expect("plan library changes");

        assert_eq!(result.freshness, CatalogFreshnessState::Updating);
        assert_eq!(result.intents.len(), 1);
        assert_eq!(result.intents[0].kind, LibraryChangeIntentKind::Reconcile);
        assert_eq!(result.intents[0].relative_path, "相册/照片.jpg");
    }
}
