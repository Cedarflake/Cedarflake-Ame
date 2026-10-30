use std::sync::atomic::AtomicBool;

use crate::domain::ScanError;
use crate::domain::classification::work::ClassificationWorkLease;
use crate::domain::exact_duplicates::{ContentFingerprint, FingerprintCandidate};

pub(crate) enum ClassificationSourcePreparation {
    Completed,
    Read(Box<FingerprintCandidate>),
    Superseded,
}

pub(crate) trait ClassificationSourceRepository {
    fn prepare_classification_source(
        &mut self,
        lease: &ClassificationWorkLease,
        cancelled: &AtomicBool,
    ) -> Result<ClassificationSourcePreparation, ScanError>;

    fn commit_classification_source(
        &mut self,
        lease: &ClassificationWorkLease,
        candidate: &FingerprintCandidate,
        fingerprint: &ContentFingerprint,
        now_unix_ms: u64,
        cancelled: &AtomicBool,
    ) -> Result<bool, ScanError>;
}
