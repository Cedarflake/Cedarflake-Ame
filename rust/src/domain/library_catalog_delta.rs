use super::{
    AssetLocationView, DerivedEvidenceDisposition, FileIdentityEvidence,
    IncrementalReconciliationOutcome, LibraryChangeFailure, LibraryChangeId, LibraryRootGeneration,
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
    pub publication_root_identity: Option<FileIdentityEvidence>,
}

#[derive(Clone, Debug)]
pub struct CatalogDeltaMutation {
    pub change_id: LibraryChangeId,
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TerminalMediaEvidence {
    pub relative_path: String,
    pub file_size: u64,
    pub modified_unix_ms: i64,
    pub file_identity: Option<FileIdentityEvidence>,
    pub inspection_engine_id: String,
    pub inspection_engine_version: u32,
    pub issue: LibraryChangeFailure,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TerminalMediaEvidenceUpdate {
    pub change_id: LibraryChangeId,
    pub evidence: TerminalMediaEvidence,
}

#[derive(Clone, Debug)]
pub struct CatalogDeltaBatch {
    pub root_id: String,
    pub root_generation: LibraryRootGeneration,
    pub expected_catalog_revision: u64,
    pub mutations: Vec<CatalogDeltaMutation>,
    pub terminal_media_evidence: Vec<TerminalMediaEvidenceUpdate>,
    pub completions: Vec<LibraryChangeCompletion>,
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum CatalogDeltaPublicationStatus {
    Applied,
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
