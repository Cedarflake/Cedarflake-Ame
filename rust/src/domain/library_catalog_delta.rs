use super::{
    AssetLocationView, DerivedEvidenceDisposition, IncrementalReconciliationOutcome,
    LibraryChangeFailure, LibraryChangeId, LibraryRootGeneration,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IncrementalCatalogRoot {
    pub root_id: String,
    pub root_path: String,
    pub root_generation: LibraryRootGeneration,
    pub active_scan_id: Option<String>,
    pub has_running_scan: bool,
    pub catalog_revision: u64,
    pub last_consistency_audit_unix_ms: Option<i64>,
}

#[derive(Clone, Debug)]
pub struct CatalogDeltaMutation {
    pub outcome: IncrementalReconciliationOutcome,
    pub evidence_disposition: DerivedEvidenceDisposition,
    pub remove_location_ids: Vec<String>,
    pub upsert_location: Option<AssetLocationView>,
    pub retained_preview_expectation: Option<RetainedPreviewExpectation>,
}

#[derive(Clone, Debug)]
pub struct RetainedPreviewExpectation {
    pub location_id: String,
    pub preview_path: String,
    pub preview_status: super::PreviewStatus,
    pub preview_issue_code: Option<String>,
    pub preview_issue_message: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LibraryChangeCompletion {
    pub change_id: LibraryChangeId,
    pub lease_generation: u64,
    pub issue: Option<LibraryChangeFailure>,
}

#[derive(Clone, Debug)]
pub struct CatalogDeltaBatch {
    pub root_id: String,
    pub root_generation: LibraryRootGeneration,
    pub expected_catalog_revision: u64,
    pub mutations: Vec<CatalogDeltaMutation>,
    pub completions: Vec<LibraryChangeCompletion>,
    pub catch_up_handoff_dependencies: Vec<LibraryChangeId>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LibraryChangeCatchUpPeer {
    pub change_id: LibraryChangeId,
    pub root_id: String,
    pub root_generation: LibraryRootGeneration,
    pub relative_path: String,
    pub requires_authoritative_reconciliation: bool,
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum CatalogDeltaPublicationStatus {
    Applied,
    CatchUpHandoffPending,
    StaleLease,
    StaleCatalogRevision,
    StalePreviewState,
    RootGenerationChanged,
    RootScanInProgress,
    NoPublishedCatalog,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CatalogDeltaPublication {
    pub status: CatalogDeltaPublicationStatus,
    pub catalog_revision: u64,
    pub applied_mutation_count: u32,
    pub completed_change_count: u32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct IncrementalLibraryChangeReport {
    pub leased_count: u32,
    pub completed_count: u32,
    pub retried_count: u32,
    pub deferred_count: u32,
    pub superseded_count: u32,
    pub applied_mutation_count: u32,
    pub catalog_revision: u64,
}
