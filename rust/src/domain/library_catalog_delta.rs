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
}

#[derive(Clone, Debug)]
pub struct CatalogDeltaMutation {
    pub outcome: IncrementalReconciliationOutcome,
    pub evidence_disposition: DerivedEvidenceDisposition,
    pub remove_location_ids: Vec<String>,
    pub upsert_location: Option<AssetLocationView>,
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
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum CatalogDeltaPublicationStatus {
    Applied,
    StaleLease,
    StaleCatalogRevision,
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
    pub superseded_count: u32,
    pub applied_mutation_count: u32,
    pub catalog_revision: u64,
}
