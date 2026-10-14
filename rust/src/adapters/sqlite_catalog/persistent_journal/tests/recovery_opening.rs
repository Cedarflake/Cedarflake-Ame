use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, mpsc};
use std::thread;
use std::time::Duration;

use super::*;

const ROOT: &str = "opening-root";
const DEADLINE: Duration = Duration::from_secs(2);

#[test]
fn current_root_opening_reads_do_not_preempt_or_wait_for_publication() {
    assert_read_only_opening(false);
}

#[test]
fn attached_recovery_opening_replay_does_not_preempt_or_wait_for_publication() {
    assert_read_only_opening(true);
}

fn assert_read_only_opening(attach_recovery: bool) {
    let directory = tempdir().expect("catalog directory");
    let mut catalog = catalog_with_roots(directory.path().join("catalog.sqlite3"), &[ROOT]);
    current_root(&mut catalog, ROOT);
    let boundary = opening_boundary();
    let expected = if attach_recovery {
        let change_id = request_recovery(&mut catalog);
        assert_eq!(
            catalog
                .attach_persistent_journal_recovery_opening_boundary(
                    ROOT,
                    LibraryRootGeneration::initial(),
                    &boundary,
                    1_001,
                )
                .expect("attach recovery window"),
            Some(change_id),
        );
        Some(change_id)
    } else {
        None
    };
    let mut poller = catalog
        .validated_session()
        .open_in_lane(LibraryChangeLane::Journal)
        .expect("journal reader");
    let callbacks = Arc::new(AtomicUsize::new(0));
    let callback_count = Arc::clone(&callbacks);
    let epoch = catalog.completed_write_epoch();
    thread::scope(|scope| {
        let transaction = catalog
            .begin_preemptible_write_in_lane(
                LibraryChangeLane::Recovery,
                Arc::new(move || {
                    callback_count.fetch_add(1, Ordering::SeqCst);
                }),
            )
            .expect("hold publication writer");
        let (sender, receiver) = mpsc::sync_channel(1);
        let worker = scope.spawn(move || {
            let outcomes = (2_000..2_003)
                .map(|now| {
                    poller.attach_persistent_journal_recovery_opening_boundary(
                        ROOT,
                        LibraryRootGeneration::initial(),
                        &boundary,
                        now,
                    )
                })
                .collect::<Vec<_>>();
            let _ = sender.send(outcomes);
        });
        let outcomes = receiver.recv_timeout(DEADLINE);
        let observed_epoch = transaction._permit.admission.completed_write_epoch();
        drop(transaction);
        worker.join().expect("opening reader");
        for outcome in outcomes.expect("no-op reads finish while publication retains its writer") {
            assert_eq!(outcome.expect("read opening state"), expected);
        }
        assert_eq!(callbacks.load(Ordering::SeqCst), 0, "no false preemption");
        assert_eq!(observed_epoch, epoch, "no no-op writer admission");
    });
    drop(catalog);
    SqliteCatalog::open(directory.path().join("catalog.sqlite3")).expect("valid retained state");
}

#[test]
fn missing_recovery_opening_still_preempts_and_attaches() {
    assert_write_rechecks_opening(false);
}

#[test]
fn recovery_opening_rechecks_conflicting_attachment_after_admission() {
    assert_write_rechecks_opening(true);
}

fn assert_write_rechecks_opening(competing_attachment: bool) {
    let directory = tempdir().expect("catalog directory");
    let mut catalog = catalog_with_roots(directory.path().join("catalog.sqlite3"), &[ROOT]);
    current_root(&mut catalog, ROOT);
    let change_id = request_recovery(&mut catalog);
    let boundary = opening_boundary();
    let requested = boundary.clone();
    let mut poller = catalog
        .validated_session()
        .open_in_lane(LibraryChangeLane::Journal)
        .expect("journal writer");
    thread::scope(|scope| {
        let (sender, receiver) = mpsc::sync_channel(1);
        let transaction = catalog
            .begin_preemptible_write_in_lane(
                LibraryChangeLane::Recovery,
                Arc::new(move || {
                    let _ = sender.try_send(());
                }),
            )
            .expect("hold writer until opening hint completes");
        let worker = scope.spawn(move || {
            poller.attach_persistent_journal_recovery_opening_boundary(
                ROOT,
                LibraryRootGeneration::initial(),
                &requested,
                2_000,
            )
        });
        let preempted = receiver.recv_timeout(DEADLINE);
        if preempted.is_ok() && competing_attachment {
            let mut competing = boundary;
            competing.next_usn = JournalUsn::new(21).expect("different opening");
            attach_recovery_window_if_available(
                &transaction,
                change_id,
                ROOT,
                LibraryRootGeneration::initial(),
                Some(&competing),
                1_500,
            )
            .expect("concurrent valid attachment");
        }
        let committed = transaction.commit();
        let outcome = worker.join().expect("opening writer");
        preempted.expect("needed attachment retains Journal priority");
        committed.expect("release publication writer");
        if competing_attachment {
            assert_eq!(
                outcome
                    .expect_err("stale hint cannot overwrite another window")
                    .code,
                "persistent_journal_recovery_opening_conflict",
            );
        } else {
            assert_eq!(outcome.expect("attach missing window"), Some(change_id));
        }
    });
    let (count, opening): (i64, String) = catalog
        .connection
        .query_row(
            "SELECT COUNT(*), MIN(opening_next_usn)
             FROM library_persistent_journal_baselines WHERE root_id = ?1",
            [ROOT],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("single durable opening");
    assert_eq!(count, 1);
    let expected_usn = if competing_attachment { 21 } else { 20 };
    assert_eq!(
        opening,
        JournalUsn::new(expected_usn)
            .expect("expected opening USN")
            .to_canonical_text()
    );
    drop(catalog);
    SqliteCatalog::open(directory.path().join("catalog.sqlite3"))
        .expect("valid recovered authority");
}

fn opening_boundary() -> LibraryRecoveryOpeningBoundary {
    LibraryRecoveryOpeningBoundary {
        volume: volume(),
        root_file_reference: JournalFileReference::V2([1; 8]),
        journal_id: JournalIdentifier::new(9).expect("journal ID"),
        next_usn: JournalUsn::new(20).expect("opening USN"),
        protocol_version: 1,
        contract_version: 1,
    }
}

fn request_recovery(catalog: &mut SqliteCatalog) -> LibraryChangeId {
    catalog
        .persist_persistent_journal_root_failure(
            &PersistentJournalRootFailure {
                root_id: ROOT.to_owned(),
                root_generation: LibraryRootGeneration::initial(),
                kind: PersistentJournalRootFailureKind::BrokerAfterCurrentFailure,
                failure: PersistentJournalFailure {
                    code: "persistent_journal_broker_after_current_failure".to_owned(),
                    message: "Controlled journal connection failure".to_owned(),
                },
                opening_boundary: None,
            },
            1_000,
            policy(),
        )
        .expect("request recovery")
        .expect("recovery authority")
}
