use std::sync::atomic::AtomicBool;

use crate::domain::exact_duplicates::{ContentFingerprint, ExactAnalysisRun, FingerprintCandidate};
use crate::domain::{
    ExactAnalysisProgress, ExactDuplicateGroupCursor, ExactDuplicateGroupPage, ScanError,
};

pub(crate) trait GalleryDuplicateRepository {
    fn load_gallery_duplicates(
        &mut self,
        revision: u64,
        location_ids: &[String],
    ) -> Result<crate::domain::GalleryDuplicatePage, ScanError>;
}

pub(crate) trait FingerprintReadLease {
    fn fingerprint(&self) -> &ContentFingerprint;
    fn revalidate(&self) -> Result<(), ScanError>;
}

pub(crate) trait ExactContentReader {
    type Lease: FingerprintReadLease;

    fn read_fingerprint(
        &self,
        candidate: &FingerprintCandidate,
        cancelled: &AtomicBool,
    ) -> Result<Option<Self::Lease>, ScanError>;
}

pub(crate) trait ExactDuplicateRepository {
    fn begin_exact_analysis(&mut self, run: &ExactAnalysisRun) -> Result<(), ScanError>;
    fn finish_exact_analysis(&mut self, progress: &ExactAnalysisProgress) -> Result<(), ScanError>;
    fn load_fingerprint_candidates(
        &self,
        after_location_id: &str,
    ) -> Result<Vec<FingerprintCandidate>, ScanError>;
    fn publish_fingerprint(
        &mut self,
        run_id: &str,
        candidate: &FingerprintCandidate,
        fingerprint: &ContentFingerprint,
        cancelled: &AtomicBool,
    ) -> Result<bool, ScanError>;
    fn load_duplicate_groups(
        &self,
        after: Option<&ExactDuplicateGroupCursor>,
    ) -> Result<ExactDuplicateGroupPage, ScanError>;
    fn load_duplicate_locations(
        &self,
        digest: &str,
        byte_length: u64,
        after_location_id: &str,
    ) -> Result<Vec<FingerprintCandidate>, ScanError>;
}
