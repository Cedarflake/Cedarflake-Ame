use std::time::{Duration, Instant};

use crate::domain::exact_duplicates::EXACT_PUBLICATION_BATCH_SIZE;

/// Complete evidence becomes visible in bounded batches without ending the run.
pub(super) struct ExactAnalysisPublication {
    pending: Vec<u64>,
    last_publication: Instant,
}

impl ExactAnalysisPublication {
    pub(super) fn new(now: Instant) -> Self {
        Self {
            pending: Vec::new(),
            last_publication: now,
        }
    }

    pub(super) fn retain(&mut self, source_generation: u64) {
        self.pending.push(source_generation);
    }

    pub(super) fn take_due(&mut self, now: Instant) -> Option<Vec<u64>> {
        if self.pending.is_empty() {
            return None;
        }
        if self.pending.len() < EXACT_PUBLICATION_BATCH_SIZE
            && now.duration_since(self.last_publication) < Duration::from_secs(1)
        {
            return None;
        }
        self.last_publication = now;
        Some(std::mem::take(&mut self.pending))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slow_reads_publish_completed_sources_without_waiting_for_a_full_batch() {
        let now = Instant::now();
        let mut publication = ExactAnalysisPublication::new(now);
        publication.retain(7);
        assert!(
            publication
                .take_due(now + Duration::from_millis(999))
                .is_none()
        );
        assert_eq!(
            publication.take_due(now + Duration::from_secs(1)),
            Some(vec![7])
        );
        assert!(publication.take_due(now + Duration::from_secs(2)).is_none());
    }

    #[test]
    fn fast_reads_never_retain_more_than_the_publication_bound() {
        let now = Instant::now();
        let mut publication = ExactAnalysisPublication::new(now);
        for generation in 1..EXACT_PUBLICATION_BATCH_SIZE {
            publication.retain(generation as u64);
            assert!(publication.take_due(now).is_none());
        }
        publication.retain(EXACT_PUBLICATION_BATCH_SIZE as u64);
        let batch = publication.take_due(now).unwrap();
        assert_eq!(batch.len(), EXACT_PUBLICATION_BATCH_SIZE);
        assert!(publication.take_due(now).is_none());
    }
}
