use std::time::{Duration, Instant};

use crate::domain::ScanError;
use crate::domain::similar_images::SimilarAnalysisRun;
use crate::ports::similar_images::SimilarAnalysisRepository;

use super::super::AnalysisControl;

// One content may need 64 empty band windows before a peer can be found.
// Bound each slice while allowing matching to keep pace with source analysis.
const MAX_WINDOWS: usize = 128;
const TIME_BUDGET: Duration = Duration::from_millis(100);

pub(super) fn advance(
    repository: &mut impl SimilarAnalysisRepository,
    run: &SimilarAnalysisRun,
    control: &AnalysisControl,
    renewal: &mut Instant,
) -> Result<bool, ScanError> {
    let started = Instant::now();
    for _ in 0..MAX_WINDOWS {
        if !super::admit(repository, run, control, renewal)?
            || !repository.match_similarity_window(run, &control.cancelled)?
        {
            return Ok(false);
        }
        if started.elapsed() >= TIME_BUDGET {
            break;
        }
    }
    Ok(true)
}

#[cfg(test)]
mod tests;
