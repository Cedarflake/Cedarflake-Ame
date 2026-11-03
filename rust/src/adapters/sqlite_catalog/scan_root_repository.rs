use crate::domain::{
    FileIdentityEvidence, IncrementalCatalogRoot, LibraryRootGeneration, PersistentJournalBaseline,
    ScanCheckpoint, ScanError, ScanRequest, ScanResumeCoverage,
};
use crate::ports::RootConsolidationRegistration;
use crate::ports::scan_admission::ScanAdmissionRepository;

use super::SqliteCatalog;

impl ScanAdmissionRepository for SqliteCatalog {
    fn first_import_change_capture_is_ready(
        &self,
        scan_id: &str,
        root_id: &str,
        root_generation: LibraryRootGeneration,
    ) -> Result<bool, ScanError> {
        self.first_import_change_capture_is_ready(scan_id, root_id, root_generation)
    }

    fn has_registered_root_id(&self, root_id: &str) -> Result<bool, ScanError> {
        self.has_registered_root_id(root_id)
    }

    fn load_registered_root_id(&self, root_path: &str) -> Result<Option<String>, ScanError> {
        self.load_registered_root_id(root_path)
    }

    fn load_published_root_id_by_identity(
        &self,
        identity: &FileIdentityEvidence,
    ) -> Result<Option<String>, ScanError> {
        self.load_published_root_id_by_identity(identity)
    }

    fn begin_identity_recovered_scan(
        &mut self,
        request: &ScanRequest,
        expected: &IncrementalCatalogRoot,
        root_path: &str,
        identity: &FileIdentityEvidence,
    ) -> Result<ScanCheckpoint, ScanError> {
        self.begin_identity_recovered_scan(request, expected, root_path, identity)
    }

    fn begin_relocated_scan(
        &mut self,
        request: &ScanRequest,
        expected: &IncrementalCatalogRoot,
        root_path: &str,
        identity: &FileIdentityEvidence,
    ) -> Result<ScanCheckpoint, ScanError> {
        self.begin_relocated_scan(request, expected, root_path, identity)
    }

    fn begin_scan_with_publication_namespace(
        &mut self,
        request: &ScanRequest,
        root_id: &str,
        root_path: &str,
        identity: &FileIdentityEvidence,
    ) -> Result<ScanCheckpoint, ScanError> {
        self.begin_scan_with_publication_namespace(request, root_id, root_path, identity)
    }

    fn resume_scan_with_publication_namespace(
        &mut self,
        request: &ScanRequest,
        root_id: &str,
        root_path: &str,
        identity: &FileIdentityEvidence,
        coverage: Option<&ScanResumeCoverage>,
    ) -> Result<ScanCheckpoint, ScanError> {
        self.resume_scan_with_publication_namespace(request, root_id, root_path, identity, coverage)
    }

    fn load_first_import_resume_baseline(
        &self,
        scan_id: &str,
        root_id: &str,
    ) -> Result<Option<PersistentJournalBaseline>, ScanError> {
        self.load_first_import_resume_baseline(scan_id, root_id)
    }

    fn load_root_consolidation_registrations(
        &self,
        root_id: &str,
    ) -> Result<Vec<RootConsolidationRegistration>, ScanError> {
        self.load_root_consolidation_registrations(root_id)
    }
}
