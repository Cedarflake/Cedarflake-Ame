use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, mpsc};
use std::thread;
use std::time::Duration;

use tempfile::{TempDir, tempdir};

use crate::domain::{
    JournalFileReference, JournalIdentifier, JournalUsn, LibraryChangeIntent,
    LibraryChangeIntentKind, LibraryChangeOrigin, LibraryChangeQueuePolicy, LibraryChangeScope,
    LibraryRecoveryAuthorityReason, LibraryRootGeneration,
    PersistentJournalBaselineClosingBoundary, PersistentJournalBaselineStartRequest,
    PersistentJournalVolumeIdentity, ScanRequest,
};
use crate::ports::{CatalogRepository, PersistentJournalRepository};

use super::*;

const ROOT: &str = "completion-root";
const DEADLINE: Duration = Duration::from_secs(2);

#[test]
fn no_candidate_completion_does_not_preempt_or_wait_for_recovery_publication() {
    assert_read_only_poll(None);
}

#[test]
fn running_first_import_completion_does_not_preempt_or_wait_for_its_publication() {
    assert_read_only_poll(Some(None));
}

#[test]
fn uncovered_first_import_replay_does_not_preempt_recovery_publication() {
    assert_read_only_poll(Some(Some(20)));
}

fn assert_read_only_poll(stage: Option<Option<i64>>) {
    let fixture = CompletionFixture::new(stage);
    let mut blocker = fixture.open();
    let mut poller = fixture.open();
    let callbacks = Arc::new(AtomicUsize::new(0));
    let callback_count = Arc::clone(&callbacks);
    let epoch = blocker.completed_write_epoch();
    thread::scope(|scope| {
        let transaction = blocker
            .begin_preemptible_write_in_lane(
                LibraryChangeLane::Recovery,
                Arc::new(move || {
                    callback_count.fetch_add(1, Ordering::SeqCst);
                }),
            )
            .expect("hold actual recovery publication transaction");
        let (sender, receiver) = mpsc::sync_channel(1);
        let worker = scope.spawn(move || {
            let mut outcomes = Vec::new();
            for now in 3_000..3_003 {
                outcomes.push(poller.finalize_ready_first_import_journal_baseline(now));
            }
            let _ = sender.send(outcomes);
        });
        let outcomes = receiver.recv_timeout(DEADLINE);
        let epoch_during_poll = transaction._permit.admission.completed_write_epoch();
        drop(transaction);
        worker.join().expect("completion poll worker");
        let outcomes = outcomes.expect("empty poll must finish while recovery keeps its writer");
        for outcome in outcomes {
            assert!(!outcome.expect("no-op completion succeeds"));
        }
        assert_eq!(
            callbacks.load(Ordering::SeqCst),
            0,
            "no-op must not preempt"
        );
        assert_eq!(
            epoch_during_poll, epoch,
            "no-op must not acquire any write permit"
        );
    });
    fixture.open();
}

#[test]
fn ready_first_import_completion_still_preempts_and_atomically_finishes() {
    assert_candidate_rechecked(false);
}

#[test]
fn first_import_completion_rechecks_live_work_arriving_after_read_hint() {
    assert_candidate_rechecked(true);
}

fn assert_candidate_rechecked(enqueue_live: bool) {
    let fixture = CompletionFixture::new(Some(Some(10)));
    let mut blocker = fixture.open();
    let mut poller = fixture.open();
    assert!(
        select_ready(&poller.connection)
            .expect("ready baseline")
            .is_some()
    );
    thread::scope(|scope| {
        let (preempt_sender, preempt_receiver) = mpsc::sync_channel(1);
        let transaction = blocker
            .begin_preemptible_write_in_lane(
                LibraryChangeLane::Recovery,
                Arc::new(move || {
                    let _ = preempt_sender.try_send(());
                }),
            )
            .expect("hold writer until hint has completed");
        let worker =
            scope.spawn(move || poller.finalize_ready_first_import_journal_baseline(3_000));
        let preempted = preempt_receiver.recv_timeout(DEADLINE);
        if preempted.is_ok() && enqueue_live {
            super::super::enqueue_intents_in_transaction(
                &transaction,
                &[LibraryChangeIntent {
                    root_id: ROOT.to_owned(),
                    root_generation: LibraryRootGeneration::initial(),
                    kind: LibraryChangeIntentKind::Reconcile,
                    scope: LibraryChangeScope::Path,
                    relative_path: "new.png".to_owned(),
                    previous_relative_path: None,
                    origin: LibraryChangeOrigin::LiveNotification,
                    first_observed_unix_ms: 2_500,
                    most_recent_observed_unix_ms: 2_500,
                    first_sequence: 1,
                    most_recent_sequence: 1,
                    coalesced_observation_count: 1,
                }],
                None,
                2_500,
                LibraryChangeQueuePolicy::default(),
            )
            .expect("commit real live work after the unlocked hint");
        }
        let committed = transaction.commit();
        let completed = worker.join().expect("completion worker");
        preempted.expect("a real completion still requests Journal priority");
        committed.expect("release writer with intervening state");
        assert_eq!(completed.expect("completion result"), !enqueue_live);
    });
    let catalog = fixture.open();
    let state: (String, String, String, String, bool) = catalog.connection.query_row(
        "SELECT baseline.phase, control.status, checkpoint.continuity_state,
                root.continuity_state, authority.retired_unix_ms IS NOT NULL
         FROM library_persistent_journal_baselines AS baseline
         JOIN library_change_queue AS control ON control.id = baseline.change_id
         JOIN library_recovery_authorities AS authority ON authority.change_id = baseline.change_id
         JOIN library_persistent_journal_checkpoints AS checkpoint ON checkpoint.root_id = baseline.root_id
         JOIN library_persistent_journal_root_state AS root ON root.root_id = baseline.root_id
         WHERE baseline.root_id = ?1",
        [ROOT],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
    ).expect("reopened atomic baseline state");
    if enqueue_live {
        assert_eq!(
            state,
            (
                "replay".into(),
                "pending".into(),
                "catching_up".into(),
                "catching_up".into(),
                false
            )
        );
    } else {
        assert_eq!(
            state,
            (
                "completed".into(),
                "completed".into(),
                "current".into(),
                "current".into(),
                true
            )
        );
    }
}

#[test]
fn completion_rejects_an_existing_transaction_before_candidate_reads() {
    let fixture = CompletionFixture::new(None);
    let mut catalog = fixture.open();
    catalog
        .connection
        .execute_batch("BEGIN DEFERRED")
        .expect("existing read transaction");
    let error = catalog
        .finalize_ready_first_import_journal_baseline(3_000)
        .expect_err("do not infer committed readiness from an existing transaction");
    assert_eq!(
        error.code,
        "persistent_journal_first_import_completion_transaction_active"
    );
    catalog
        .connection
        .execute_batch("ROLLBACK")
        .expect("release caller transaction");
    assert!(
        !catalog
            .finalize_ready_first_import_journal_baseline(3_000)
            .expect("standalone retry")
    );
}

struct CompletionFixture {
    directory: TempDir,
}

impl CompletionFixture {
    fn new(stage: Option<Option<i64>>) -> Self {
        let fixture = Self {
            directory: tempdir().expect("isolated catalog storage"),
        };
        let mut catalog = fixture.open();
        let Some(closing) = stage else {
            return fixture;
        };
        let request = ScanRequest {
            scan_id: "completion-first-import".to_owned(),
            root_path: format!("C:/{ROOT}"),
            max_items: None,
            max_entries: None,
            preview_edge: 128,
        };
        catalog
            .begin_scan(&request, ROOT, &request.root_path)
            .expect("begin real first import");
        let volume = PersistentJournalVolumeIdentity {
            volume_guid: "completion-volume".to_owned(),
            volume_serial: 7,
        };
        let reference = JournalFileReference::V2([1; 8]);
        let journal = JournalIdentifier::new(9).expect("journal ID");
        let baseline = catalog
            .begin_persistent_journal_baseline(
                &PersistentJournalBaselineStartRequest {
                    run_id: request.scan_id.clone(),
                    root_id: ROOT.to_owned(),
                    root_generation: LibraryRootGeneration::initial(),
                    authority_reason: LibraryRecoveryAuthorityReason::FirstImportBoundary,
                    volume: volume.clone(),
                    root_file_reference: reference.clone(),
                    journal_id: journal,
                    opening_next_usn: JournalUsn::new(10).expect("opening USN"),
                    protocol_version: 1,
                    contract_version: 1,
                    authorized_unix_ms: 1_000,
                },
                LibraryChangeQueuePolicy::default(),
            )
            .expect("admit real opening boundary");
        if let Some(closing) = closing {
            catalog
                .publish_scan(&request.scan_id, ROOT, 0, 0)
                .expect("publish first inventory");
            catalog
                .capture_persistent_journal_baseline_closing_boundary(
                    &PersistentJournalBaselineClosingBoundary {
                        change_id: baseline.change_id,
                        volume,
                        root_file_reference: reference,
                        journal_id: journal,
                        closing_next_usn: JournalUsn::new(closing).expect("closing USN"),
                        protocol_version: 1,
                        captured_unix_ms: 2_000,
                    },
                )
                .expect("capture real closing boundary");
        }
        fixture
    }

    fn open(&self) -> SqliteCatalog {
        SqliteCatalog::open(self.directory.path().join("catalog.sqlite3"))
            .expect("open and validate catalog")
    }
}
