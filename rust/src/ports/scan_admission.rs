use crate::domain::{
    FileIdentityEvidence, IncrementalCatalogRoot, LibraryRootGeneration, PersistentJournalBaseline,
    ScanCheckpoint, ScanError, ScanRequest, ScanResumeCoverage,
};

use super::RootConsolidationRegistration;
use super::scan_staging::ScanStagingRepository;

pub(crate) trait FirstImportReadiness {
    fn pristine_first_import_scan(
        &self,
        root_id: &str,
        root_generation: LibraryRootGeneration,
    ) -> Result<Option<(String, i64)>, ScanError>;

    fn load_inactive_first_import_roots(&self) -> Result<Vec<IncrementalCatalogRoot>, ScanError>;
}

pub(crate) trait ScanAdmissionRepository: ScanStagingRepository {
    fn first_import_change_capture_is_ready(
        &self,
        scan_id: &str,
        root_id: &str,
        root_generation: LibraryRootGeneration,
    ) -> Result<bool, ScanError>;
    fn has_registered_root_id(&self, root_id: &str) -> Result<bool, ScanError>;
    fn load_registered_root_id(&self, root_path: &str) -> Result<Option<String>, ScanError>;
    fn load_published_root_id_by_identity(
        &self,
        identity: &FileIdentityEvidence,
    ) -> Result<Option<String>, ScanError>;
    fn begin_identity_recovered_scan(
        &mut self,
        request: &ScanRequest,
        expected: &IncrementalCatalogRoot,
        root_path: &str,
        identity: &FileIdentityEvidence,
    ) -> Result<ScanCheckpoint, ScanError>;
    fn begin_relocated_scan(
        &mut self,
        request: &ScanRequest,
        expected: &IncrementalCatalogRoot,
        root_path: &str,
        identity: &FileIdentityEvidence,
    ) -> Result<ScanCheckpoint, ScanError>;
    fn begin_scan_with_publication_namespace(
        &mut self,
        request: &ScanRequest,
        root_id: &str,
        root_path: &str,
        identity: &FileIdentityEvidence,
    ) -> Result<ScanCheckpoint, ScanError>;
    fn resume_scan_with_publication_namespace(
        &mut self,
        request: &ScanRequest,
        root_id: &str,
        root_path: &str,
        identity: &FileIdentityEvidence,
        coverage: Option<&ScanResumeCoverage>,
    ) -> Result<ScanCheckpoint, ScanError>;
    fn load_first_import_resume_baseline(
        &self,
        scan_id: &str,
        root_id: &str,
    ) -> Result<Option<PersistentJournalBaseline>, ScanError>;
    fn load_root_consolidation_registrations(
        &self,
        root_id: &str,
    ) -> Result<Vec<RootConsolidationRegistration>, ScanError>;
}
