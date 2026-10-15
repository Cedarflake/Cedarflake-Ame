use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use super::*;

#[test]
fn checkpoint_only_publication_preserves_an_active_recovery_writer() {
    assert_publication_admission(&[], 0);
}

#[test]
fn source_change_publication_still_preempts_an_active_recovery_writer() {
    assert_publication_admission(&["changed.jpg"], 1);
}

#[test]
fn directory_only_changes_keep_journal_priority_without_image_intents() {
    let mut enrollment = batch("range", "root-a", 10, 20, &["new-folder"]);
    enrollment.intents[0].scope = LibraryChangeScope::Subtree;
    enrollment.intents[0].kind = LibraryChangeIntentKind::RenameCandidate;
    enrollment.intents[0].previous_relative_path = Some("old-folder".to_owned());
    enrollment
        .directory_changes
        .push(crate::domain::PersistentJournalDirectoryChange {
            identity: crate::domain::FileIdentityEvidence {
                scheme: "windows-file-id-128-v1".to_owned(),
                value: format!("{:016x}:{:032x}", volume().volume_serial, 1),
            },
            previous_relative_paths: vec!["old-folder".to_owned()],
            current_relative_path: Some("new-folder".to_owned()),
        });
    let volume_batch = atomic_volume_batch(vec![enrollment]);
    volume_batch.validate().expect("valid directory-only page");
    assert_eq!(
        super::super::publication_admission::write_lane(&volume_batch),
        LibraryChangeLane::Journal,
    );
}

fn assert_publication_admission(paths: &[&str], expected_preemptions: usize) {
    let directory = tempdir().expect("catalog directory");
    let path = directory.path().join("catalog.sqlite3");
    let mut blocker = catalog_with_roots(path.clone(), &["root-a"]);
    current_root(&mut blocker, "root-a");
    let mut publisher = SqliteCatalog::open(path.clone()).expect("publication connection");
    let batch = atomic_volume_batch(vec![batch("range", "root-a", 10, 20, paths)]);
    let admission = Arc::clone(&blocker.write_admission);
    let callbacks = Arc::new(AtomicUsize::new(0));
    let callback_count = Arc::clone(&callbacks);
    thread::scope(|scope| {
        let transaction = blocker
            .begin_preemptible_write_in_lane(
                LibraryChangeLane::Recovery,
                Arc::new(move || {
                    callback_count.fetch_add(1, Ordering::SeqCst);
                }),
            )
            .expect("hold actual recovery publication transaction");
        let worker = scope.spawn(move || {
            publisher.publish_persistent_journal_volume_batch(&batch, 1_000, policy())
        });
        let deadline = Instant::now() + Duration::from_secs(2);
        let registered = loop {
            let waiting = admission
                .state
                .lock()
                .expect("admission state")
                .waiting
                .iter()
                .any(|queue| !queue.is_empty());
            if waiting || Instant::now() >= deadline {
                break waiting;
            }
            thread::sleep(Duration::from_millis(1));
        };
        let cursor: String = transaction
            .query_row(
                "SELECT next_unread_usn FROM library_persistent_journal_checkpoints
                 WHERE root_id = 'root-a'",
                [],
                |row| row.get(0),
            )
            .expect("uncommitted cursor");
        drop(transaction);
        let result = worker.join().expect("journal publication worker");
        assert!(registered, "journal publication must join writer admission");
        assert_eq!(
            cursor, "10",
            "waiting publication cannot advance its cursor"
        );
        let report = result.expect("publish after recovery releases its writer");
        assert_eq!(report.advanced_checkpoint_count, 1);
        assert_eq!(
            report.observation_count,
            u64::try_from(paths.len()).expect("bounded observation count"),
        );
    });
    assert_eq!(callbacks.load(Ordering::SeqCst), expected_preemptions);
    drop(blocker);
    let reopened = SqliteCatalog::open(path).expect("reopen completed journal publication");
    let checkpoint = reopened
        .load_persistent_journal_checkpoint("root-a", LibraryRootGeneration::initial())
        .expect("load committed checkpoint")
        .expect("checkpoint retained");
    assert_eq!(checkpoint.next_unread_usn.value(), 20);
}
