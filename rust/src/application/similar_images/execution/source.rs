use std::sync::atomic::{AtomicBool, Ordering};

use crate::domain::ScanError;
use crate::domain::exact_duplicates::EXACT_FINGERPRINT_SCHEME;
use crate::domain::similar_images::{
    SimilarAnalysisProgress, SimilarAnalysisRun, SimilarContentKey, SimilarSource,
};
use crate::ports::exact_duplicates::{ExactContentReader, FingerprintReadLease};
use crate::ports::similar_images::{
    SimilarAnalysisRepository, SimilarImageLease, SimilarImageReader,
};

pub(super) fn analyze(
    repository: &mut impl SimilarAnalysisRepository,
    fingerprints: &impl ExactContentReader,
    images: &impl SimilarImageReader,
    run: &SimilarAnalysisRun,
    source: SimilarSource,
    cancelled: &AtomicBool,
    progress: &mut SimilarAnalysisProgress,
) -> Result<bool, ScanError> {
    let Some(_admission) = crate::application::background_analysis::shared().acquire(cancelled)?
    else {
        return Ok(false);
    };
    let candidate = &source.candidate;
    progress.checked_sources += 1;
    let content = match source.content {
        Some(content) => content,
        None => {
            let lease = match fingerprints.read_fingerprint(candidate, cancelled) {
                Ok(Some(lease)) => lease,
                Ok(None) => return Ok(false),
                Err(issue) => {
                    return skipped(repository, run, candidate, issue, cancelled, progress);
                }
            };
            if let Err(issue) = lease.revalidate() {
                return skipped(repository, run, candidate, issue, cancelled, progress);
            }
            if !repository.verify_similarity_source(
                run,
                candidate,
                lease.fingerprint(),
                cancelled,
            )? {
                return Ok(!cancelled.load(Ordering::Acquire));
            }
            SimilarContentKey {
                scheme: EXACT_FINGERPRINT_SCHEME.into(),
                digest: lease.fingerprint().digest.clone(),
                byte_length: lease.fingerprint().byte_length,
            }
        }
    };
    if repository.has_similarity_evidence(&content)? {
        return Ok(true);
    }
    let lease = match images.analyze(candidate, &content, cancelled) {
        Ok(Some(lease)) => lease,
        Ok(None) => return Ok(false),
        Err(issue) => return skipped(repository, run, candidate, issue, cancelled, progress),
    };
    if let Err(issue) = lease.revalidate() {
        return skipped(repository, run, candidate, issue, cancelled, progress);
    }
    if repository.publish_similarity(run, candidate, &content, lease.evidence(), cancelled)? {
        progress.analyzed_contents += 1;
    }
    Ok(!cancelled.load(Ordering::Acquire))
}

fn skipped(
    repository: &mut impl SimilarAnalysisRepository,
    run: &SimilarAnalysisRun,
    candidate: &crate::domain::exact_duplicates::FingerprintCandidate,
    issue: ScanError,
    cancelled: &AtomicBool,
    progress: &mut SimilarAnalysisProgress,
) -> Result<bool, ScanError> {
    if cancelled.load(Ordering::Acquire) {
        return Ok(false);
    }
    repository.record_similarity_failure(run, candidate, &issue.code)?;
    progress.skipped_sources += 1;
    progress.issue_code = Some(issue.code);
    Ok(true)
}
