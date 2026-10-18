use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use crate::domain::exact_duplicates::ExactAnalysisRun;
use crate::domain::{ExactAnalysisPhase, ExactAnalysisProgress, ScanError};
use crate::ports::exact_duplicates::{
    ExactContentReader, ExactDuplicateRepository, FingerprintReadLease,
};

mod publication;

pub(super) fn analyze(
    repository: &mut impl ExactDuplicateRepository,
    reader: &impl ExactContentReader,
    run: &ExactAnalysisRun,
    cancelled: &AtomicBool,
    emit: &mut impl FnMut(ExactAnalysisProgress) -> bool,
) -> Result<(), ScanError> {
    let mut progress = ExactAnalysisProgress::running(run.run_id.clone());
    if !emit(progress.clone()) {
        cancelled.store(true, Ordering::Release);
    }
    repository.begin_exact_analysis(run)?;
    let result = analyze_candidates(repository, reader, cancelled, &mut progress, emit);
    progress.current_path = None;
    progress.phase = match &result {
        Err(error) => {
            progress.issue_code = Some(error.code.clone());
            ExactAnalysisPhase::Failed
        }
        Ok(()) if cancelled.load(Ordering::Acquire) => ExactAnalysisPhase::Cancelled,
        Ok(()) => ExactAnalysisPhase::Completed,
    };
    if let Err(error) = repository.finish_exact_analysis(&progress) {
        progress.phase = ExactAnalysisPhase::Failed;
        progress.issue_code = Some(error.code.clone());
        emit(progress);
        return Err(error);
    }
    emit(progress);
    result
}

fn analyze_candidates(
    repository: &mut impl ExactDuplicateRepository,
    reader: &impl ExactContentReader,
    cancelled: &AtomicBool,
    progress: &mut ExactAnalysisProgress,
    emit: &mut impl FnMut(ExactAnalysisProgress) -> bool,
) -> Result<(), ScanError> {
    let mut after = String::new();
    let mut last_progress = Instant::now();
    let mut publication = publication::ExactAnalysisPublication::new(last_progress);
    while !cancelled.load(Ordering::Acquire) {
        let candidates = repository.load_fingerprint_candidates(&after)?;
        if candidates.is_empty() {
            break;
        }
        let mut published_generations = HashSet::new();
        for candidate in candidates {
            if cancelled.load(Ordering::Acquire) {
                return Ok(());
            }
            after.clone_from(&candidate.location.location_id);
            if published_generations.contains(&candidate.location.source_generation) {
                continue;
            }
            progress.current_path = Some(candidate.location.display_path.clone());
            let lease = match reader.read_fingerprint(&candidate, cancelled) {
                Ok(Some(lease)) => Some(lease),
                Ok(None) => return Ok(()),
                Err(issue) => {
                    progress.issue_code = Some(issue.code);
                    None
                }
            };
            progress.checked_files += 1;
            if let Some(lease) = lease {
                progress.bytes_read += lease.fingerprint().byte_length;
                match lease.revalidate() {
                    Ok(()) => {
                        // The source and namespace remain held until the publication transaction ends.
                        if repository.publish_fingerprint(
                            &progress.run_id,
                            &candidate,
                            lease.fingerprint(),
                            cancelled,
                        )? {
                            progress.fingerprinted_files += 1;
                            published_generations.insert(candidate.location.source_generation);
                            publication.retain(candidate.location.source_generation);
                        } else if !cancelled.load(Ordering::Acquire) {
                            progress.skipped_files += 1;
                            progress.issue_code = Some("exact_source_superseded".to_owned());
                        }
                    }
                    Err(issue) => {
                        progress.skipped_files += 1;
                        progress.issue_code = Some(issue.code);
                    }
                }
            } else {
                progress.skipped_files += 1;
            }
            if let Some(generations) = publication.take_due(Instant::now()) {
                repository.publish_exact_analysis_batch(&progress.run_id, &generations)?;
            }
            if last_progress.elapsed() >= Duration::from_millis(100) {
                if !emit(progress.clone()) {
                    cancelled.store(true, Ordering::Release);
                }
                last_progress = Instant::now();
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
