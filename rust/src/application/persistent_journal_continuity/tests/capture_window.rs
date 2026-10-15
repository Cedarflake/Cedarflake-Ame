use std::time::Duration;

use super::*;
use crate::application::persistent_journal_continuity::capture_window::{
    CaptureClock, capture_with_clock, unfinished_checkpoints,
};

struct FixedClock(Duration);

impl CaptureClock for FixedClock {
    fn elapsed(&self) -> Duration {
        self.0
    }

    fn observed_unix_ms(&self) -> Result<i64, ScanError> {
        Ok(2_000)
    }
}

#[derive(Clone, Copy)]
enum PageEffect {
    Empty,
    Candidate,
    Failure,
    Cancel,
}

struct WindowReader {
    reads: AtomicUsize,
    effects: Vec<PageEffect>,
}

impl PersistentJournalVolumeReader for WindowReader {
    fn read_volume(
        &self,
        checkpoints: &[PersistentJournalCheckpoint],
        _pending: &[PersistentJournalPendingRename],
        observed_unix_ms: i64,
        cancelled: &AtomicBool,
    ) -> Result<Vec<PersistentJournalRootReadOutcome>, PersistentJournalReadFailure> {
        let index = self.reads.fetch_add(1, Ordering::SeqCst);
        let effect = self
            .effects
            .get(index)
            .copied()
            .unwrap_or(PageEffect::Empty);
        if matches!(effect, PageEffect::Failure) {
            return Err(PersistentJournalReadFailure {
                kind: PersistentJournalRootFailureKind::Transient,
                failure: PersistentJournalFailure {
                    code: "controlled_read_failure".to_owned(),
                    message: "The controlled read failed".to_owned(),
                },
                opening_boundary: None,
            });
        }
        if matches!(effect, PageEffect::Cancel) {
            cancelled.store(true, Ordering::Release);
        }
        let previous = &checkpoints[0];
        let start = previous.next_unread_usn.value();
        let intents = if matches!(effect, PageEffect::Candidate) {
            vec![intent(
                &previous.root_id,
                LibraryChangeIntentKind::Reconcile,
                "changed.png",
                None,
                u64::try_from(start).unwrap(),
            )]
        } else {
            Vec::new()
        };
        let (mut batch, mut checkpoint) = page("window", &previous.root_id, intents, None);
        batch.range.requested_start_usn = previous.next_unread_usn;
        batch.range.requested_end_usn = JournalUsn::new(200).unwrap();
        batch.range.covered_until_usn = JournalUsn::new(start + 10).unwrap();
        batch.range.is_complete = false;
        batch.range.enrolled_unix_ms = observed_unix_ms;
        batch.range.batch_id = persistent_journal_batch_id(&batch);
        checkpoint.next_unread_usn = batch.range.covered_until_usn;
        checkpoint.captured_exclusive_end = batch.range.requested_end_usn;
        checkpoint.updated_unix_ms = observed_unix_ms;
        Ok(vec![PersistentJournalRootReadOutcome {
            root_id: previous.root_id.clone(),
            root_generation: previous.root_generation,
            opening_boundary: None,
            page: Ok(Some((batch, checkpoint).into())),
        }])
    }
}

fn fixture(
    effects: Vec<PageEffect>,
) -> (
    ControlledRepository,
    WindowReader,
    PersistentJournalCheckpoint,
) {
    let checkpoint = checkpoint("root-a", 10);
    let repository = ControlledRepository {
        capabilities: capabilities(&["root-a"]),
        checkpoints: HashMap::from([("root-a".to_owned(), checkpoint.clone())]),
        ..ControlledRepository::default()
    };
    let reader = WindowReader {
        reads: AtomicUsize::new(0),
        effects,
    };
    (repository, reader, checkpoint)
}

#[test]
fn outside_root_pages_commit_individually_and_yield_at_the_first_candidate() {
    let (mut repository, reader, checkpoint) = fixture(vec![
        PageEffect::Empty,
        PageEffect::Empty,
        PageEffect::Candidate,
    ]);
    let report = capture_with_clock(
        &mut repository,
        &reader,
        &[checkpoint],
        1_000,
        policy(),
        &AtomicBool::new(false),
        &FixedClock(Duration::ZERO),
    )
    .unwrap();
    assert_eq!(reader.reads.load(Ordering::SeqCst), 3);
    assert_eq!(report.reports().len(), 3);
    assert_eq!(repository.enrollment_ids.len(), 3);
    assert_eq!(repository.intents.len(), 1);
    assert_eq!(repository.intents[0].relative_path, "changed.png");
    assert_eq!(repository.checkpoints["root-a"].next_unread_usn.value(), 40);
}

#[test]
fn newly_ready_foreground_work_yields_after_the_committed_empty_page() {
    let (mut repository, reader, checkpoint) = fixture(Vec::new());
    repository.yield_capture_after = Some(2);
    let report = capture_with_clock(
        &mut repository,
        &reader,
        &[checkpoint],
        1_000,
        policy(),
        &AtomicBool::new(false),
        &FixedClock(Duration::ZERO),
    )
    .unwrap();
    assert_eq!(reader.reads.load(Ordering::SeqCst), 2);
    assert_eq!(report.reports().len(), 2);
    assert_eq!(repository.enrollment_ids.len(), 2);
    assert_eq!(repository.checkpoints["root-a"].next_unread_usn.value(), 30);
}

#[test]
fn continuous_outside_root_churn_yields_after_eight_pages() {
    let (mut repository, reader, checkpoint) = fixture(Vec::new());
    let report = capture_with_clock(
        &mut repository,
        &reader,
        &[checkpoint],
        1_000,
        policy(),
        &AtomicBool::new(false),
        &FixedClock(Duration::ZERO),
    )
    .unwrap();
    assert_eq!(reader.reads.load(Ordering::SeqCst), 8);
    assert_eq!(report.reports().len(), 8);
    assert_eq!(repository.enrollment_ids.len(), 8);
    assert_eq!(repository.checkpoints["root-a"].next_unread_usn.value(), 90);
}

#[test]
fn slow_read_exhausts_the_quantum_before_another_page_is_started() {
    let (mut repository, reader, checkpoint) = fixture(Vec::new());
    let report = capture_with_clock(
        &mut repository,
        &reader,
        &[checkpoint],
        1_000,
        policy(),
        &AtomicBool::new(false),
        &FixedClock(Duration::from_millis(210)),
    )
    .unwrap();
    assert_eq!(reader.reads.load(Ordering::SeqCst), 1);
    assert_eq!(report.reports().len(), 1);
}

#[test]
fn failure_and_cancellation_preserve_committed_pages_without_starting_another_read() {
    for terminal in [PageEffect::Failure, PageEffect::Cancel] {
        let (mut repository, reader, checkpoint) =
            fixture(vec![PageEffect::Empty, PageEffect::Empty, terminal]);
        let report = capture_with_clock(
            &mut repository,
            &reader,
            &[checkpoint],
            1_000,
            policy(),
            &AtomicBool::new(false),
            &FixedClock(Duration::ZERO),
        )
        .unwrap();
        assert_eq!(reader.reads.load(Ordering::SeqCst), 3);
        assert_eq!(report.reports().len(), 3);
        assert_eq!(repository.enrollment_ids.len(), 2);
        assert_eq!(repository.checkpoints["root-a"].next_unread_usn.value(), 30);
        assert_eq!(
            report.checked_roots().collect::<Vec<_>>(),
            [("root-a", LibraryRootGeneration::initial())]
        );
        if matches!(terminal, PageEffect::Failure) {
            assert_eq!(repository.root_failures.len(), 1);
        }
    }
}

#[test]
fn failed_or_cancelled_first_reads_do_not_confirm_startup() {
    for terminal in [PageEffect::Failure, PageEffect::Cancel] {
        let (mut repository, reader, checkpoint) = fixture(vec![terminal]);
        let report = capture_with_clock(
            &mut repository,
            &reader,
            &[checkpoint],
            1_000,
            policy(),
            &AtomicBool::new(false),
            &FixedClock(Duration::ZERO),
        )
        .unwrap();
        assert_eq!(reader.reads.load(Ordering::SeqCst), 1);
        assert_eq!(report.checked_roots().count(), 0);
        assert!(repository.enrollment_ids.is_empty());
    }
    let counters_only = crate::application::JournalCaptureWindowReport::from(
        PersistentJournalEnrollmentReport::default(),
    );
    assert_eq!(counters_only.checked_roots().count(), 0);
}

#[test]
fn no_change_results_confirm_only_complete_checked_sets_without_rewriting_checkpoints() {
    for omit_second_root in [false, true] {
        let checkpoints = ["root-a", "root-b"].map(|root_id| {
            let mut checkpoint = checkpoint(root_id, 20);
            checkpoint.continuity = PersistentJournalContinuityState::Current;
            checkpoint
        });
        let mut repository = ControlledRepository {
            capabilities: capabilities(&["root-a", "root-b"]),
            checkpoints: checkpoints
                .iter()
                .map(|checkpoint| (checkpoint.root_id.clone(), checkpoint.clone()))
                .collect(),
            ..ControlledRepository::default()
        };
        let retained = repository.checkpoints.clone();
        let reader = ControlledReader::from_outcomes(
            checkpoints
                .iter()
                .filter(|checkpoint| !omit_second_root || checkpoint.root_id != "root-b")
                .map(|checkpoint| PersistentJournalRootReadOutcome {
                    root_id: checkpoint.root_id.clone(),
                    root_generation: checkpoint.root_generation,
                    opening_boundary: None,
                    page: Ok(None),
                })
                .collect(),
        );
        let report = capture_with_clock(
            &mut repository,
            &reader,
            &checkpoints,
            1_000,
            policy(),
            &AtomicBool::new(false),
            &FixedClock(Duration::ZERO),
        )
        .unwrap();
        let expected = if omit_second_root {
            Vec::new()
        } else {
            vec![
                ("root-a", LibraryRootGeneration::initial()),
                ("root-b", LibraryRootGeneration::initial()),
            ]
        };
        assert_eq!(report.checked_roots().collect::<Vec<_>>(), expected);
        assert_eq!(repository.checkpoints, retained);
        assert!(repository.enrollment_ids.is_empty());
    }
}

#[test]
fn missing_changed_or_completed_authority_does_not_continue_the_old_window() {
    let mut previous = checkpoint("root-a", 20);
    previous.captured_exclusive_end = JournalUsn::new(200).unwrap();
    let mut next = previous.clone();
    next.next_unread_usn = JournalUsn::new(30).unwrap();
    assert!(unfinished_checkpoints(&[previous.clone()], &[next.clone()]).is_some());
    assert!(unfinished_checkpoints(&[previous.clone()], &[]).is_none());
    for replacement in 0..4 {
        let mut changed = next.clone();
        match replacement {
            0 => changed.root_generation = LibraryRootGeneration::new(2).unwrap(),
            1 => changed.journal_id = JournalIdentifier::new(10).unwrap(),
            2 => changed.captured_exclusive_end = JournalUsn::new(210).unwrap(),
            _ => changed.continuity = PersistentJournalContinuityState::Current,
        }
        assert!(unfinished_checkpoints(&[previous.clone()], &[changed]).is_none());
    }
}
