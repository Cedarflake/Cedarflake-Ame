use std::cell::Cell;
use std::collections::HashSet;

use crate::domain::exact_duplicates::{ContentFingerprint, FingerprintCandidate};
use crate::domain::{
    AssetLocationView, ExactDuplicateGroupCursor, ExactDuplicateGroupPage, IncrementalCatalogRoot,
    LibraryRootGeneration, PreviewStatus,
};

use super::*;

struct Repository {
    candidate_count: u64,
    fingerprints: HashSet<u64>,
    published_batches: Vec<Vec<u64>>,
    outcomes: Vec<ExactAnalysisProgress>,
}

impl Default for Repository {
    fn default() -> Self {
        Self {
            candidate_count: 3,
            fingerprints: HashSet::new(),
            published_batches: Vec::new(),
            outcomes: Vec::new(),
        }
    }
}

impl ExactDuplicateRepository for Repository {
    fn begin_exact_analysis(&mut self, _: &ExactAnalysisRun) -> Result<(), ScanError> {
        Ok(())
    }
    fn finish_exact_analysis(&mut self, progress: &ExactAnalysisProgress) -> Result<(), ScanError> {
        self.outcomes.push(progress.clone());
        Ok(())
    }
    fn load_fingerprint_candidates(
        &self,
        after: &str,
    ) -> Result<Vec<FingerprintCandidate>, ScanError> {
        Ok((1..=self.candidate_count)
            .filter(|generation| !self.fingerprints.contains(generation))
            .map(candidate)
            .filter(|candidate| candidate.location.location_id.as_str() > after)
            .take(64)
            .collect())
    }
    fn publish_fingerprint(
        &mut self,
        _: &str,
        candidate: &FingerprintCandidate,
        _: &ContentFingerprint,
        _: &AtomicBool,
    ) -> Result<bool, ScanError> {
        Ok(self
            .fingerprints
            .insert(candidate.location.source_generation))
    }
    fn publish_exact_analysis_batch(
        &mut self,
        _: &str,
        generations: &[u64],
    ) -> Result<(), ScanError> {
        assert!(self.outcomes.is_empty());
        assert!(
            generations
                .iter()
                .all(|value| self.fingerprints.contains(value))
        );
        self.published_batches.push(generations.to_vec());
        Ok(())
    }
    fn load_duplicate_groups(
        &self,
        _: Option<&ExactDuplicateGroupCursor>,
    ) -> Result<ExactDuplicateGroupPage, ScanError> {
        unreachable!("execution never loads presentation groups")
    }
    fn load_duplicate_locations(
        &self,
        _: &str,
        _: u64,
        _: &str,
    ) -> Result<Vec<FingerprintCandidate>, ScanError> {
        unreachable!("execution never loads presentation locations")
    }
}

struct Reader {
    reads: Cell<usize>,
    cancel_on: usize,
    stale_generation: u64,
}
struct Lease {
    fingerprint: ContentFingerprint,
    stale: bool,
}

impl FingerprintReadLease for Lease {
    fn fingerprint(&self) -> &ContentFingerprint {
        &self.fingerprint
    }
    fn revalidate(&self) -> Result<(), ScanError> {
        if self.stale {
            Err(ScanError::new("fixture_source_changed", "source changed"))
        } else {
            Ok(())
        }
    }
}

impl ExactContentReader for Reader {
    type Lease = Lease;
    fn read_fingerprint(
        &self,
        candidate: &FingerprintCandidate,
        cancelled: &AtomicBool,
    ) -> Result<Option<Lease>, ScanError> {
        let count = self.reads.get() + 1;
        self.reads.set(count);
        if count == self.cancel_on {
            cancelled.store(true, Ordering::Release);
            return Ok(None);
        }
        Ok(Some(Lease {
            fingerprint: ContentFingerprint {
                digest: "a".repeat(64),
                byte_length: 20,
            },
            stale: candidate.location.source_generation == self.stale_generation,
        }))
    }
}

fn run(repository: &mut Repository, reader: &Reader) {
    analyze(
        repository,
        reader,
        &ExactAnalysisRun {
            run_id: "run".to_owned(),
            started_unix_ms: 1,
        },
        &AtomicBool::new(false),
        &mut |_| true,
    )
    .unwrap();
}

#[test]
fn retry_reuses_completed_content_after_cancellation() {
    let mut repository = Repository::default();
    run(
        &mut repository,
        &Reader {
            reads: Cell::new(0),
            cancel_on: 2,
            stale_generation: 0,
        },
    );
    assert_eq!(repository.fingerprints, HashSet::from([1]));
    assert_eq!(repository.outcomes[0].phase, ExactAnalysisPhase::Cancelled);
    let retry = Reader {
        reads: Cell::new(0),
        cancel_on: 0,
        stale_generation: 0,
    };
    run(&mut repository, &retry);
    assert_eq!(retry.reads.get(), 2);
    assert_eq!(repository.fingerprints.len(), 3);
    assert_eq!(repository.outcomes[1].phase, ExactAnalysisPhase::Completed);
}

#[test]
fn changed_handle_is_skipped_without_losing_other_results() {
    let mut repository = Repository::default();
    run(
        &mut repository,
        &Reader {
            reads: Cell::new(0),
            cancel_on: 0,
            stale_generation: 2,
        },
    );
    assert_eq!(repository.fingerprints, HashSet::from([1, 3]));
    assert_eq!(repository.outcomes[0].skipped_files, 1);
    assert_eq!(repository.outcomes[0].phase, ExactAnalysisPhase::Completed);
}

#[test]
fn completed_evidence_is_published_before_a_run_retires() {
    let mut repository = Repository {
        candidate_count: 300,
        ..Repository::default()
    };
    run(
        &mut repository,
        &Reader {
            reads: Cell::new(0),
            cancel_on: 290,
            stale_generation: 2,
        },
    );
    assert_eq!(
        repository.published_batches.concat(),
        (1..=257).filter(|value| *value != 2).collect::<Vec<_>>()
    );
    assert_eq!(repository.outcomes[0].phase, ExactAnalysisPhase::Cancelled);
    assert_eq!(repository.outcomes[0].fingerprinted_files, 288);
}

fn candidate(generation: u64) -> FingerprintCandidate {
    FingerprintCandidate {
        location: AssetLocationView {
            asset_id: format!("asset-{generation}"),
            location_id: format!("location-{generation:04}"),
            root_id: "root".to_owned(),
            scan_id: "scan".to_owned(),
            absolute_path: "C:\\Fixture\\image.png".to_owned(),
            display_path: "C:\\Fixture\\image.png".to_owned(),
            relative_path: "image.png".to_owned(),
            preview_path: String::new(),
            file_size: 20,
            created_unix_ms: None,
            modified_unix_ms: 1,
            file_identity: None,
            source_revision: None,
            source_generation: generation,
            width: 20,
            height: 30,
            preview_status: PreviewStatus::Pending,
            preview_issue_code: None,
            preview_issue_message: None,
            metadata_engine_id: "fixture".to_owned(),
            metadata_engine_version: "1".to_owned(),
            capture_time: None,
        },
        root: IncrementalCatalogRoot {
            root_id: "root".to_owned(),
            root_path: "C:\\Fixture".to_owned(),
            root_generation: LibraryRootGeneration::initial(),
            active_scan_id: Some("scan".to_owned()),
            has_running_scan: false,
            catalog_revision: 0,
            last_consistency_audit_unix_ms: None,
            publication_root_identity: None,
        },
    }
}
