use std::sync::atomic::{AtomicBool, Ordering};

use crate::domain::exact_duplicates::{ContentFingerprint, FingerprintCandidate};
use crate::domain::similar_images::{
    SimilarAnalysisProgress, SimilarContentKey, SimilarEvidence, SimilarSource,
};

use super::*;

#[test]
fn matching_advances_between_sources_and_yields_with_pending_work() {
    let control = AnalysisControl::default();
    let mut repository = PendingMatching::default();
    let mut renewal = Instant::now() - Duration::from_secs(6);
    assert!(advance(&mut repository, &run(), &control, &mut renewal).unwrap());
    assert!(repository.windows > 1);
    assert!(repository.windows <= MAX_WINDOWS);
    assert_eq!(repository.renewals, 1);

    repository.exhausted = true;
    assert!(!advance(&mut repository, &run(), &control, &mut renewal).unwrap());
}

#[test]
fn cancellation_retires_a_matching_slice_before_another_window() {
    let control = AnalysisControl::default();
    let mut repository = PendingMatching {
        cancel_after: Some(2),
        ..Default::default()
    };
    assert!(!advance(&mut repository, &run(), &control, &mut Instant::now()).unwrap());
    assert!(control.cancelled.load(Ordering::Acquire));
    assert_eq!(repository.windows, 2);
}

fn run() -> SimilarAnalysisRun {
    SimilarAnalysisRun {
        run_id: "matching-slice".into(),
        source_run_id: "matching-source".into(),
        started_unix_ms: 1,
    }
}

#[derive(Default)]
struct PendingMatching {
    windows: usize,
    renewals: usize,
    cancel_after: Option<usize>,
    exhausted: bool,
}

impl SimilarAnalysisRepository for PendingMatching {
    fn renew_similarity(&mut self, _: &SimilarAnalysisRun) -> Result<(), ScanError> {
        self.renewals += 1;
        Ok(())
    }

    fn match_similarity_window(
        &mut self,
        _: &SimilarAnalysisRun,
        cancelled: &AtomicBool,
    ) -> Result<bool, ScanError> {
        if self.exhausted {
            return Ok(false);
        }
        self.windows += 1;
        if self.cancel_after == Some(self.windows) {
            cancelled.store(true, Ordering::Release);
        }
        Ok(true)
    }

    fn begin_similarity(&mut self, _: &SimilarAnalysisRun) -> Result<(), ScanError> {
        unreachable!()
    }

    fn load_similarity_sources(&self, _: &str) -> Result<Vec<SimilarSource>, ScanError> {
        unreachable!()
    }

    fn has_similarity_evidence(&self, _: &SimilarContentKey) -> Result<bool, ScanError> {
        unreachable!()
    }

    fn verify_similarity_source(
        &mut self,
        _: &SimilarAnalysisRun,
        _: &FingerprintCandidate,
        _: &ContentFingerprint,
        _: &AtomicBool,
    ) -> Result<bool, ScanError> {
        unreachable!()
    }

    fn publish_similarity(
        &mut self,
        _: &SimilarAnalysisRun,
        _: &FingerprintCandidate,
        _: &SimilarContentKey,
        _: &SimilarEvidence,
        _: &AtomicBool,
    ) -> Result<bool, ScanError> {
        unreachable!()
    }

    fn record_similarity_failure(
        &mut self,
        _: &SimilarAnalysisRun,
        _: &FingerprintCandidate,
        _: &str,
    ) -> Result<(), ScanError> {
        unreachable!()
    }

    fn finish_similarity(
        &mut self,
        _: &SimilarAnalysisRun,
        _: &SimilarAnalysisProgress,
    ) -> Result<(), ScanError> {
        unreachable!()
    }
}
