use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::domain::{
    IncrementalCatalogRoot, LibraryChangeCatchUpBatch, LibraryChangeCatchUpCheckpoint,
    LibraryChangeCatchUpEvidence, LibraryChangeCatchUpLimits, LibraryChangeCatchUpRootResult,
    LibraryChangeEnqueueReport, LibraryChangeIntent, LibraryChangeObservation,
    LibraryChangeObservationKind, LibraryChangeOrigin, LibraryChangeQueuePolicy,
    LibraryChangeScope, LibraryRootGeneration, ScanError,
};
use crate::ports::{
    LibraryChangeCatchUpRepository, LibraryChangeCatchUpSource, LibraryChangeQueue,
};

use super::{LibraryChangeCatchUpExecution, process_library_change_catch_up};

#[derive(Clone)]
struct FixedSource {
    batch: LibraryChangeCatchUpBatch,
}

impl LibraryChangeCatchUpSource for FixedSource {
    fn read_changes(
        &self,
        _roots: &[IncrementalCatalogRoot],
        _checkpoints: &[LibraryChangeCatchUpCheckpoint],
        _observed_unix_ms: i64,
        _limits: LibraryChangeCatchUpLimits,
        _cancelled: &AtomicBool,
    ) -> Result<LibraryChangeCatchUpBatch, ScanError> {
        Ok(self.batch.clone())
    }
}

#[derive(Default)]
struct RecordingRepository {
    enqueued: Vec<(
        Vec<LibraryChangeIntent>,
        Option<LibraryChangeCatchUpEvidence>,
    )>,
    checkpoints: Vec<LibraryChangeCatchUpCheckpoint>,
    fail_enqueue: bool,
    cancel_after_enqueue: Option<Arc<AtomicBool>>,
}

impl LibraryChangeCatchUpRepository for RecordingRepository {
    fn load_library_change_catch_up_checkpoints(
        &self,
    ) -> Result<Vec<LibraryChangeCatchUpCheckpoint>, ScanError> {
        Ok(self.checkpoints.clone())
    }

    fn save_library_change_catch_up_checkpoint(
        &mut self,
        checkpoint: &LibraryChangeCatchUpCheckpoint,
    ) -> Result<(), ScanError> {
        self.checkpoints.push(checkpoint.clone());
        Ok(())
    }
}

impl LibraryChangeQueue for RecordingRepository {
    fn enqueue_library_change_intents(
        &mut self,
        intents: &[LibraryChangeIntent],
        _enqueued_unix_ms: i64,
        _policy: LibraryChangeQueuePolicy,
    ) -> Result<LibraryChangeEnqueueReport, ScanError> {
        self.enqueued.push((intents.to_vec(), None));
        Ok(LibraryChangeEnqueueReport::default())
    }

    fn enqueue_library_change_intents_with_catch_up(
        &mut self,
        intents: &[LibraryChangeIntent],
        evidence: &LibraryChangeCatchUpEvidence,
        _enqueued_unix_ms: i64,
        _policy: LibraryChangeQueuePolicy,
    ) -> Result<LibraryChangeEnqueueReport, ScanError> {
        if self.fail_enqueue {
            return Err(ScanError::new("database_busy", "busy"));
        }
        self.enqueued
            .push((intents.to_vec(), Some(evidence.clone())));
        if let Some(cancelled) = &self.cancel_after_enqueue {
            cancelled.store(true, Ordering::Release);
        }
        Ok(LibraryChangeEnqueueReport::default())
    }

    fn lease_library_changes(
        &mut self,
        _root_id: &str,
        _root_generation: LibraryRootGeneration,
        _now_unix_ms: i64,
        _policy: LibraryChangeQueuePolicy,
    ) -> Result<Vec<crate::domain::LeasedLibraryChange>, ScanError> {
        unreachable!()
    }

    fn lease_path_library_changes(
        &mut self,
        _root_id: &str,
        _root_generation: LibraryRootGeneration,
        _now_unix_ms: i64,
        _policy: LibraryChangeQueuePolicy,
    ) -> Result<Vec<crate::domain::LeasedLibraryChange>, ScanError> {
        unreachable!()
    }

    fn lease_authoritative_library_change(
        &mut self,
        _root_id: &str,
        _root_generation: LibraryRootGeneration,
        _now_unix_ms: i64,
        _policy: LibraryChangeQueuePolicy,
    ) -> Result<Option<crate::domain::LeasedLibraryChange>, ScanError> {
        unreachable!()
    }

    fn complete_library_change(
        &mut self,
        _change_id: crate::domain::LibraryChangeId,
        _lease_generation: u64,
        _catalog_revision_at_success: u64,
        _completed_unix_ms: i64,
    ) -> Result<crate::domain::LibraryChangeLeaseUpdateOutcome, ScanError> {
        unreachable!()
    }

    fn retry_library_change(
        &mut self,
        _change_id: crate::domain::LibraryChangeId,
        _lease_generation: u64,
        _failure: &crate::domain::LibraryChangeFailure,
        _failed_unix_ms: i64,
        _policy: LibraryChangeQueuePolicy,
    ) -> Result<crate::domain::LibraryChangeLeaseUpdateOutcome, ScanError> {
        unreachable!()
    }

    fn defer_library_change(
        &mut self,
        _change_id: crate::domain::LibraryChangeId,
        _lease_generation: u64,
        _deferred_unix_ms: i64,
    ) -> Result<crate::domain::LibraryChangeLeaseUpdateOutcome, ScanError> {
        unreachable!()
    }

    fn load_library_change_queue_metrics(
        &self,
        _now_unix_ms: i64,
        _policy: LibraryChangeQueuePolicy,
    ) -> Result<crate::domain::LibraryChangeQueueMetrics, ScanError> {
        unreachable!()
    }

    fn load_library_change_root_queue_metrics(
        &self,
        _root_id: &str,
        _root_generation: LibraryRootGeneration,
        _now_unix_ms: i64,
        _policy: LibraryChangeQueuePolicy,
    ) -> Result<crate::domain::LibraryChangeQueueMetrics, ScanError> {
        unreachable!()
    }

    fn cleanup_terminal_library_changes(
        &mut self,
        _terminal_before_unix_ms: i64,
        _limit: u32,
    ) -> Result<u32, ScanError> {
        unreachable!()
    }
}

#[test]
fn journal_work_is_enqueued_before_checkpoint_advances() {
    let root = root("root-a");
    let checkpoint = checkpoint();
    let source = FixedSource {
        batch: LibraryChangeCatchUpBatch {
            roots: vec![LibraryChangeCatchUpRootResult {
                root_id: root.root_id.clone(),
                root_generation: root.root_generation,
                observations: vec![observation(&root, "album/new.jpg")],
                fallback_code: None,
                evidence: Some(evidence()),
            }],
            checkpoints: vec![checkpoint.clone()],
        },
    };
    let mut repository = RecordingRepository::default();

    let report = process_library_change_catch_up(
        &source,
        &mut repository,
        &[root],
        LibraryChangeCatchUpExecution::at(50, Default::default()),
        &AtomicBool::new(false),
    )
    .expect("catch up");

    assert_eq!(repository.enqueued.len(), 1);
    assert_eq!(repository.enqueued[0].1, Some(evidence()));
    assert_eq!(repository.checkpoints, vec![checkpoint]);
    assert_eq!(report.observation_count, 1);
    assert_eq!(report.checkpoint_count, 1);
}

#[test]
fn enqueue_failure_does_not_advance_checkpoint() {
    let root = root("root-a");
    let source = FixedSource {
        batch: LibraryChangeCatchUpBatch {
            roots: vec![LibraryChangeCatchUpRootResult {
                root_id: root.root_id.clone(),
                root_generation: root.root_generation,
                observations: vec![observation(&root, "changed.jpg")],
                fallback_code: None,
                evidence: Some(evidence()),
            }],
            checkpoints: vec![checkpoint()],
        },
    };
    let mut repository = RecordingRepository {
        fail_enqueue: true,
        ..Default::default()
    };

    let error = process_library_change_catch_up(
        &source,
        &mut repository,
        &[root],
        LibraryChangeCatchUpExecution::at(50, Default::default()),
        &AtomicBool::new(false),
    )
    .expect_err("enqueue failure");

    assert_eq!(error.code, "database_busy");
    assert!(repository.checkpoints.is_empty());
}

#[test]
fn cancellation_after_enqueue_does_not_advance_checkpoint() {
    let root = root("root-a");
    let source = FixedSource {
        batch: LibraryChangeCatchUpBatch {
            roots: vec![LibraryChangeCatchUpRootResult {
                root_id: root.root_id.clone(),
                root_generation: root.root_generation,
                observations: vec![observation(&root, "changed.jpg")],
                fallback_code: None,
                evidence: Some(evidence()),
            }],
            checkpoints: vec![checkpoint()],
        },
    };
    let cancelled = Arc::new(AtomicBool::new(false));
    let mut repository = RecordingRepository {
        cancel_after_enqueue: Some(Arc::clone(&cancelled)),
        ..Default::default()
    };

    let error = process_library_change_catch_up(
        &source,
        &mut repository,
        &[root],
        LibraryChangeCatchUpExecution::at(50, Default::default()),
        cancelled.as_ref(),
    )
    .expect_err("cancel after durable enqueue");

    assert_eq!(error.code, "library_change_catch_up_cancelled");
    assert_eq!(repository.enqueued.len(), 1);
    assert!(repository.checkpoints.is_empty());
}

#[test]
fn explicit_fallback_becomes_durable_root_evidence_gap() {
    let root = root("root-a");
    let source = FixedSource {
        batch: LibraryChangeCatchUpBatch {
            roots: vec![LibraryChangeCatchUpRootResult {
                root_id: root.root_id.clone(),
                root_generation: root.root_generation,
                observations: Vec::new(),
                fallback_code: Some("usn_journal_recreated".to_owned()),
                evidence: Some(evidence()),
            }],
            checkpoints: vec![checkpoint()],
        },
    };
    let mut repository = RecordingRepository::default();

    let report = process_library_change_catch_up(
        &source,
        &mut repository,
        &[root],
        LibraryChangeCatchUpExecution::at(50, Default::default()),
        &AtomicBool::new(false),
    )
    .expect("fallback");

    assert_eq!(report.fallback_count, 1);
    let intent = &repository.enqueued[0].0[0];
    assert_eq!(intent.scope, LibraryChangeScope::Root);
    assert_eq!(intent.origin, LibraryChangeOrigin::StartupCatchUp);
}

fn root(root_id: &str) -> IncrementalCatalogRoot {
    IncrementalCatalogRoot {
        root_id: root_id.to_owned(),
        root_path: format!("C:/fixtures/{root_id}"),
        root_generation: LibraryRootGeneration::initial(),
        active_scan_id: Some("scan".to_owned()),
        has_running_scan: false,
        catalog_revision: 7,
        last_consistency_audit_unix_ms: None,
    }
}

fn observation(root: &IncrementalCatalogRoot, relative_path: &str) -> LibraryChangeObservation {
    LibraryChangeObservation {
        root_id: root.root_id.clone(),
        root_generation: root.root_generation,
        sequence: 8,
        observed_unix_ms: 40,
        kind: LibraryChangeObservationKind::Modified,
        scope: LibraryChangeScope::Path,
        relative_path: relative_path.to_owned(),
        previous_relative_path: None,
        origin: LibraryChangeOrigin::StartupCatchUp,
    }
}

fn evidence() -> LibraryChangeCatchUpEvidence {
    LibraryChangeCatchUpEvidence {
        source: "windows_usn_v1".to_owned(),
        watermark: "volume:12:34".to_owned(),
    }
}

fn checkpoint() -> LibraryChangeCatchUpCheckpoint {
    LibraryChangeCatchUpCheckpoint {
        volume_id: "volume".to_owned(),
        journal_id: "12".to_owned(),
        next_usn: "34".to_owned(),
        root_set_fingerprint: "a".repeat(64),
        catalog_revision: 7,
        updated_unix_ms: 50,
    }
}
