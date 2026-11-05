use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use crate::domain::ScanError;
use crate::domain::similar_images::{
    SimilarAnalysisPhase, SimilarAnalysisProgress, SimilarAnalysisRun,
};
use crate::ports::exact_duplicates::ExactContentReader;
use crate::ports::similar_images::{SimilarAnalysisRepository, SimilarImageReader};

use super::AnalysisControl;

mod source;

pub(super) fn analyze(
    repository: &mut impl SimilarAnalysisRepository,
    fingerprints: &impl ExactContentReader,
    images: &impl SimilarImageReader,
    run: &SimilarAnalysisRun,
    control: &AnalysisControl,
    emit: &mut impl FnMut(SimilarAnalysisProgress),
) -> Result<SimilarAnalysisProgress, ScanError> {
    let mut progress = SimilarAnalysisProgress::running(run.run_id.clone());
    repository.begin_similarity(run)?;
    emit(progress.clone());
    let result = execute(
        repository,
        fingerprints,
        images,
        run,
        control,
        &mut progress,
        emit,
    );
    progress.current_path = None;
    progress.phase = match &result {
        Err(issue) => {
            progress.issue_code = Some(issue.code.clone());
            SimilarAnalysisPhase::Failed
        }
        Ok(()) if control.cancelled.load(Ordering::Acquire) => SimilarAnalysisPhase::Cancelled,
        Ok(()) => SimilarAnalysisPhase::Completed,
    };
    if let Err(issue) = repository.finish_similarity(run, &progress) {
        progress.phase = SimilarAnalysisPhase::Failed;
        progress.issue_code = Some(issue.code.clone());
        emit(progress);
        return Err(issue);
    }
    emit(progress.clone());
    result.map(|()| progress)
}

fn execute(
    repository: &mut impl SimilarAnalysisRepository,
    fingerprints: &impl ExactContentReader,
    images: &impl SimilarImageReader,
    run: &SimilarAnalysisRun,
    control: &AnalysisControl,
    progress: &mut SimilarAnalysisProgress,
    emit: &mut impl FnMut(SimilarAnalysisProgress),
) -> Result<(), ScanError> {
    let mut after = String::new();
    let mut renewal = Instant::now();
    let mut last_emit = Instant::now();
    loop {
        if !admit(repository, run, control, &mut renewal)? {
            return Ok(());
        }
        let sources = repository.load_similarity_sources(&after)?;
        if sources.is_empty() {
            break;
        }
        for source in sources {
            if !admit(repository, run, control, &mut renewal)? {
                return Ok(());
            }
            after.clone_from(&source.candidate.location.location_id);
            progress.current_path = Some(source.candidate.location.display_path.clone());
            if !source::analyze(
                repository,
                fingerprints,
                images,
                run,
                source,
                &control.cancelled,
                progress,
            )? {
                return Ok(());
            }
            // Share the existing background-reader admission; only bounded matching runs outside it.
            repository.match_similarity_window(run, &control.cancelled)?;
            if last_emit.elapsed() >= Duration::from_millis(100) {
                emit(progress.clone());
                last_emit = Instant::now();
            }
        }
    }
    progress.current_path = None;
    while admit(repository, run, control, &mut renewal)?
        && repository.match_similarity_window(run, &control.cancelled)?
    {
        if last_emit.elapsed() >= Duration::from_millis(100) {
            emit(progress.clone());
            last_emit = Instant::now();
        }
    }
    Ok(())
}

fn admit(
    repository: &mut impl SimilarAnalysisRepository,
    run: &SimilarAnalysisRun,
    control: &AnalysisControl,
    renewal: &mut Instant,
) -> Result<bool, ScanError> {
    loop {
        if control.cancelled.load(Ordering::Acquire) {
            return Ok(false);
        }
        if renewal.elapsed() >= Duration::from_secs(5) {
            repository.renew_similarity(run)?;
            *renewal = Instant::now();
        }
        if !control.suspended.load(Ordering::Acquire) {
            return Ok(true);
        }
        std::thread::sleep(Duration::from_millis(25));
    }
}
