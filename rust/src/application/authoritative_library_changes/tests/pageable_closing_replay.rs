use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use crate::adapters::SqliteCatalog;
use crate::application::persistent_journal_continuity::catch_up_persistent_journal_volume;
use crate::domain::{
    IncrementalCatalogRoot, PersistentJournalBaselinePhase, PersistentJournalCheckpoint,
    PersistentJournalPendingRename, PersistentJournalReadFailure, PersistentJournalReadPage,
    PersistentJournalRootReadOutcome,
};
use crate::ports::{PersistentJournalRepository, PersistentJournalVolumeReader};

struct ClosingReadFixture {
    checkpoint: PersistentJournalCheckpoint,
    reads: AtomicUsize,
}

impl PersistentJournalVolumeReader for ClosingReadFixture {
    fn read_volume(
        &self,
        checkpoints: &[PersistentJournalCheckpoint],
        pending_renames: &[PersistentJournalPendingRename],
        _observed_unix_ms: i64,
        cancelled: &AtomicBool,
    ) -> Result<Vec<PersistentJournalRootReadOutcome>, PersistentJournalReadFailure> {
        assert!(!cancelled.load(Ordering::Acquire));
        assert!(pending_renames.is_empty());
        assert_eq!(checkpoints, std::slice::from_ref(&self.checkpoint));
        self.reads.fetch_add(1, Ordering::AcqRel);
        Ok(vec![PersistentJournalRootReadOutcome {
            root_id: self.checkpoint.root_id.clone(),
            root_generation: self.checkpoint.root_generation,
            opening_boundary: None,
            page: Ok(Some(PersistentJournalReadPage::EmptyReplay(
                self.checkpoint.clone(),
            ))),
        }])
    }
}

pub(super) fn complete(catalog: &mut SqliteCatalog, root: &IncrementalCatalogRoot, now: i64) {
    let baseline = catalog
        .load_persistent_journal_baselines()
        .expect("closing baseline")
        .into_iter()
        .find(|baseline| {
            baseline.root_id == root.root_id
                && baseline.root_generation == root.root_generation
                && baseline.phase == PersistentJournalBaselinePhase::Replay
        })
        .expect("root baseline");
    assert_eq!(baseline.closing_next_usn, Some(baseline.opening_next_usn));
    assert!(
        !catalog
            .persistent_journal_baseline_closing_is_covered(baseline.change_id)
            .expect("unread closing window")
    );
    let checkpoint = catalog
        .load_persistent_journal_checkpoint(&root.root_id, root.root_generation)
        .expect("closing checkpoint")
        .expect("retained checkpoint");
    assert_eq!(checkpoint.next_unread_usn, baseline.opening_next_usn);
    assert_eq!(
        checkpoint.captured_exclusive_end,
        checkpoint.next_unread_usn
    );
    let reader = ClosingReadFixture {
        checkpoint: checkpoint.clone(),
        reads: AtomicUsize::new(0),
    };
    let report = catch_up_persistent_journal_volume(
        catalog,
        &reader,
        &[checkpoint],
        now,
        super::immediate_queue_policy(),
        &AtomicBool::new(false),
    )
    .expect("consume successful empty replay receipt");
    assert_eq!(reader.reads.load(Ordering::Acquire), 1);
    assert_eq!(report.failed_root_count, 0);
    assert_eq!(report.advanced_checkpoint_count, 0);
    assert!(
        catalog
            .persistent_journal_baseline_closing_is_covered(baseline.change_id)
            .expect("covered closing window")
    );
}
