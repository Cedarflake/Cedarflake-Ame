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
