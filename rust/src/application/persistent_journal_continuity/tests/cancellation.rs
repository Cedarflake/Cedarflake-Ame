use super::*;

#[test]
fn cancellation_after_read_preserves_every_unenrolled_checkpoint() {
    let reader = CancellingReader(ControlledReader::new(vec![
        page("range-a", "root-a", Vec::new(), None),
        page("range-b", "root-b", Vec::new(), None),
    ]));
    let mut repository = ControlledRepository {
        capabilities: capabilities(&["root-a", "root-b"]),
        ..ControlledRepository::default()
    };
    let cancelled = AtomicBool::new(false);

    let report = catch_up_persistent_journal_volume(
        &mut repository,
        &reader,
        &[checkpoint("root-a", 10), checkpoint("root-b", 10)],
        1_000,
        policy(),
        &cancelled,
    )
    .expect("cancelled read preserves durable state");

    assert_eq!(reader.0.physical_reads.load(Ordering::SeqCst), 1);
    assert_eq!(report.enrolled_root_count, 0);
    assert_eq!(report.advanced_checkpoint_count, 0);
    assert_eq!(report.failed_root_count, 0);
    assert!(repository.enrollments.is_empty());
    assert!(repository.checkpoints.is_empty());
    assert!(repository.root_failures.is_empty());
}

struct CancellingReader(ControlledReader);

impl PersistentJournalVolumeReader for CancellingReader {
    fn read_volume(
        &self,
        checkpoints: &[PersistentJournalCheckpoint],
        pending_renames: &[PersistentJournalPendingRename],
        observed_unix_ms: i64,
        cancelled: &AtomicBool,
    ) -> Result<Vec<PersistentJournalRootReadOutcome>, PersistentJournalReadFailure> {
        let pages =
            self.0
                .read_volume(checkpoints, pending_renames, observed_unix_ms, cancelled)?;
        cancelled.store(true, Ordering::Release);
        Ok(pages)
    }
}
