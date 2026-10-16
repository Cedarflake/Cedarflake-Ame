use super::super::recovery_checkpoint::replay_start;
use super::*;

#[test]
fn subtree_recovery_preserves_unread_changes_outside_the_inventory() {
    let directory = tempdir().unwrap();
    let mut catalog = catalog_with_roots(directory.path().join("catalog.sqlite3"), &["root-a"]);
    current_root(&mut catalog, "root-a");
    let mut baseline = promote_watcher_gap(&mut catalog, "root-a");
    baseline.opening_next_usn = JournalUsn::new(20).unwrap();
    let mut checkpoint = catalog
        .load_persistent_journal_checkpoint("root-a", LibraryRootGeneration::initial())
        .unwrap()
        .unwrap();
    checkpoint.captured_exclusive_end = JournalUsn::new(15).unwrap();

    assert_eq!(
        replay_start(&checkpoint, &baseline, "watcher_uncovered_gap", "subtree").unwrap(),
        JournalUsn::new(10).unwrap()
    );
    assert_eq!(
        replay_start(&checkpoint, &baseline, "journal_trim", "root").unwrap(),
        JournalUsn::new(20).unwrap()
    );
    for reason in [
        "watcher_uncovered_gap",
        "journal_gap",
        "containment_failure",
        "broker_after_current_failure",
        "journal_reconstruction_failure",
    ] {
        assert_eq!(
            replay_start(&checkpoint, &baseline, reason, "root").unwrap(),
            JournalUsn::new(10).unwrap(),
            "retain readable pre-inventory changes for {reason}"
        );
    }
    assert!(replay_start(&checkpoint, &baseline, "journal_trim", "subtree").is_err());
    checkpoint.failure = Some(PersistentJournalFailure {
        code: "identity_journal_hardlinks_not_supported".to_owned(),
        message: "Windows metadata operation failed: 50".to_owned(),
    });
    assert_eq!(
        replay_start(
            &checkpoint,
            &baseline,
            "journal_reconstruction_failure",
            "root"
        )
        .unwrap(),
        JournalUsn::new(20).unwrap()
    );
    assert!(
        replay_start(
            &checkpoint,
            &baseline,
            "journal_reconstruction_failure",
            "subtree"
        )
        .is_err()
    );
    checkpoint.captured_exclusive_end = JournalUsn::new(21).unwrap();
    assert!(replay_start(&checkpoint, &baseline, "journal_trim", "root").is_err());
}

#[test]
fn journal_replacement_requires_whole_root_reconstruction() {
    let directory = tempdir().unwrap();
    let mut catalog = catalog_with_roots(directory.path().join("catalog.sqlite3"), &["root-a"]);
    current_root(&mut catalog, "root-a");
    let mut baseline = promote_watcher_gap(&mut catalog, "root-a");
    let checkpoint = catalog
        .load_persistent_journal_checkpoint("root-a", LibraryRootGeneration::initial())
        .unwrap()
        .unwrap();
    baseline.journal_id = JournalIdentifier::new(checkpoint.journal_id.value() + 1).unwrap();
    baseline.opening_next_usn = JournalUsn::new(2).unwrap();
    assert_eq!(
        replay_start(&checkpoint, &baseline, "journal_reset", "root").unwrap(),
        JournalUsn::new(2).unwrap()
    );
    for scope in ["subtree", "path", "unknown"] {
        assert!(replay_start(&checkpoint, &baseline, "journal_reset", scope).is_err());
    }
    baseline.root_generation = LibraryRootGeneration::new(2).unwrap();
    assert!(replay_start(&checkpoint, &baseline, "journal_reset", "root").is_err());
}
