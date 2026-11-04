use std::sync::atomic::AtomicBool;

use crate::domain::ScanError;
use crate::domain::exact_duplicates::{ContentFingerprint, FingerprintCandidate};
use crate::domain::similar_images::{
    SimilarAnalysisProgress, SimilarAnalysisRun, SimilarContentKey, SimilarEvidence, SimilarSource,
};

pub(crate) trait SimilarImageLease {
    fn evidence(&self) -> &SimilarEvidence;
    fn revalidate(&self) -> Result<(), ScanError>;
}

pub(crate) trait SimilarImageReader {
    type Lease: SimilarImageLease;

    fn analyze(
        &self,
        candidate: &FingerprintCandidate,
        content: &SimilarContentKey,
        cancelled: &AtomicBool,
    ) -> Result<Option<Self::Lease>, ScanError>;
}

pub(crate) trait SimilarAnalysisRepository {
    fn begin_similarity(&mut self, run: &SimilarAnalysisRun) -> Result<(), ScanError>;
    fn renew_similarity(&mut self, run: &SimilarAnalysisRun) -> Result<(), ScanError>;
    fn load_similarity_sources(
        &self,
        after_location_id: &str,
    ) -> Result<Vec<SimilarSource>, ScanError>;
    fn has_similarity_evidence(&self, content: &SimilarContentKey) -> Result<bool, ScanError>;
    fn verify_similarity_source(
        &mut self,
        run: &SimilarAnalysisRun,
        candidate: &FingerprintCandidate,
        fingerprint: &ContentFingerprint,
        cancelled: &AtomicBool,
    ) -> Result<bool, ScanError>;
    fn publish_similarity(
        &mut self,
        run: &SimilarAnalysisRun,
        candidate: &FingerprintCandidate,
        content: &SimilarContentKey,
        evidence: &SimilarEvidence,
        cancelled: &AtomicBool,
    ) -> Result<bool, ScanError>;
    fn record_similarity_failure(
        &mut self,
        run: &SimilarAnalysisRun,
        candidate: &FingerprintCandidate,
        issue_code: &str,
    ) -> Result<(), ScanError>;
    fn match_similarity_window(
        &mut self,
        run: &SimilarAnalysisRun,
        cancelled: &AtomicBool,
    ) -> Result<bool, ScanError>;
    fn finish_similarity(
        &mut self,
        run: &SimilarAnalysisRun,
        progress: &SimilarAnalysisProgress,
    ) -> Result<(), ScanError>;
}
