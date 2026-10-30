use std::sync::atomic::AtomicBool;

use crate::domain::ScanError;
use crate::domain::classification::work::ClassificationWorkLease;
use crate::domain::classification::{ClassificationContent, ClassificationRun, PredictionEvidence};
use crate::domain::exact_duplicates::FingerprintCandidate;

pub(crate) struct ClassificationAnalysisSource {
    pub candidate: FingerprintCandidate,
    pub run: ClassificationRun,
}

pub(crate) trait ClassificationAnalysisRepository {
    fn prepare_classification_analysis(
        &mut self,
        lease: &ClassificationWorkLease,
        cancelled: &AtomicBool,
    ) -> Result<Option<ClassificationAnalysisSource>, ScanError>;

    fn classification_analysis_is_current(
        &mut self,
        lease: &ClassificationWorkLease,
        cancelled: &AtomicBool,
    ) -> Result<bool, ScanError>;

    fn commit_classification_analysis(
        &mut self,
        lease: &ClassificationWorkLease,
        evidence: &PredictionEvidence,
        cancelled: &AtomicBool,
    ) -> Result<bool, ScanError>;
}

pub(crate) trait ClassificationImageReader {
    type Lease: ClassificationImageLease;

    fn open(
        &self,
        candidate: &FingerprintCandidate,
        content: &ClassificationContent,
        cancelled: &AtomicBool,
    ) -> Result<Self::Lease, ScanError>;
}

pub(crate) trait ClassificationImageLease {
    fn prepare_pixels(&mut self, cancelled: &AtomicBool) -> Result<Vec<f32>, ScanError>;
    fn revalidate(&self) -> Result<(), ScanError>;
}
