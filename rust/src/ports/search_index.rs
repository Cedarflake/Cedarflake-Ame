use std::sync::atomic::AtomicBool;

use crate::domain::{
    FileIdentityEvidence, IncrementalCatalogRoot, LeasedLibraryChange, LibraryChangeIntent,
    LibraryChangeQueuePolicy, MetadataInventoryRun, ScanError,
};

use super::{IncrementalCatalogRepository, MetadataInventoryRepository};

pub(crate) struct IndexedFileHint {
    pub relative_path: String,
    pub file_size: Option<u64>,
    pub modified_unix_ms: Option<i64>,
    pub is_image_candidate: bool,
}

pub(crate) trait SearchIndex {
    fn read(
        &self,
        root_path: &str,
        cancelled: &AtomicBool,
        accept: &mut dyn FnMut(&[IndexedFileHint]) -> Result<(), ScanError>,
    ) -> Result<u64, ScanError>;
}

pub(crate) trait IndexedRecoveryRepository:
    IncrementalCatalogRepository + MetadataInventoryRepository
{
    fn begin_search_index_hints(&self) -> Result<(), ScanError>;
    fn stage_search_index_hints(&mut self, hints: &[IndexedFileHint]) -> Result<(), ScanError>;
    fn changed_search_index_paths(
        &self,
        root: &IncrementalCatalogRoot,
        limit: u32,
    ) -> Result<Vec<String>, ScanError>;
    fn bind_indexed_recovery_namespace(
        &mut self,
        run: &MetadataInventoryRun,
        lease: &LeasedLibraryChange,
        identity: &FileIdentityEvidence,
        now_unix_ms: i64,
    ) -> Result<(), ScanError>;
    fn publish_indexed_recovery_candidates(
        &mut self,
        lease: &LeasedLibraryChange,
        run_id: &str,
        intents: &[LibraryChangeIntent],
        now_unix_ms: i64,
        policy: LibraryChangeQueuePolicy,
    ) -> Result<u32, ScanError>;
}
