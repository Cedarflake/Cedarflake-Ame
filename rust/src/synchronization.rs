//! Platform-independent continuous-library synchronization contracts.
//!
//! Metadata inventory is an internal, authority-bound recovery operation; the public facade does
//! not provide a proofless inventory entrypoint.
//!
//! ```compile_fail
//! use rust_lib_cedarflake_ame::synchronization::run_local_metadata_inventory;
//! ```

pub use crate::application::{
    enqueue_library_change_plan, plan_library_changes, process_ready_library_changes,
    reconcile_path_evidence,
};
pub use crate::domain::{
    CatalogDeltaBatch, CatalogDeltaMutation, CatalogDeltaPublication,
    CatalogDeltaPublicationStatus, CatalogFreshnessCause, CatalogFreshnessState,
    DerivedEvidenceDisposition, FileIdentityEvidence, IncrementalCatalogRoot,
    IncrementalLibraryChangeReport, IncrementalReconciliationDecision,
    IncrementalReconciliationOutcome, LibraryChangeCompletion, LibraryChangeIntent,
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
    LibraryChangeSourceStopReport, MetadataInventoryComparisonStatus,
    MetadataInventoryComparisonUpdate, MetadataInventoryEntry, MetadataInventoryEntryKind,
    MetadataInventoryPage, MetadataInventoryPlaceholderState, MetadataInventoryReport,
    MetadataInventoryRun, MetadataInventoryRunRequest, MetadataInventoryRunStatus,
    MetadataInventoryScope, ScanError,
};
pub use crate::ports::{
    IncrementalCatalogRepository, LibraryChangeQueue, MetadataInventoryRepository,
};

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
