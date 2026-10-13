use super::*;

#[test]
fn repeated_supported_probe_cannot_rewind_replay_or_completed_continuity() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("catalog.sqlite3");
    let request = first_import_scan_request("late-capability-window", "root-a");
    let mut catalog = SqliteCatalog::open(path.clone()).unwrap();
    catalog
        .begin_scan(&request, "root-a", &request.root_path)
        .unwrap();
    let baseline = catalog
        .begin_persistent_journal_baseline(
            &first_import_baseline_request(&request, "root-a", 10),
            policy(),
        )
        .unwrap();
    catalog
        .publish_scan(&request.scan_id, "root-a", 0, 0)
        .unwrap();
    catalog
        .capture_persistent_journal_baseline_closing_boundary(&closing_boundary(
            baseline.change_id,
            10,
        ))
        .unwrap();
    let checkpoint = catalog
        .load_persistent_journal_checkpoint("root-a", LibraryRootGeneration::initial())
        .unwrap()
        .unwrap();
    let mut stale_probe = catalog
        .load_persistent_journal_capabilities()
        .unwrap()
        .remove(0);
    stale_probe.continuity = PersistentJournalContinuityState::BaselineRequired;
    stale_probe.updated_unix_ms = 3_000;
    catalog
        .save_persistent_journal_capability(&stale_probe)
        .unwrap();
    drop(catalog);
    let mut catalog =
        SqliteCatalog::open(path.clone()).expect("late probe preserves active replay");
    assert_eq!(
        catalog.load_persistent_journal_capabilities().unwrap()[0].continuity,
        PersistentJournalContinuityState::CatchingUp
    );
    assert!(
        catalog
            .record_persistent_journal_empty_replay(&checkpoint, 3_000)
            .unwrap()
    );
    assert!(
        catalog
            .finalize_ready_first_import_journal_baseline(3_001)
            .unwrap()
    );
    stale_probe.updated_unix_ms = 3_002;
    catalog
        .save_persistent_journal_capability(&stale_probe)
        .unwrap();
    drop(catalog);
    let catalog = SqliteCatalog::open(path).expect("late probe preserves completed continuity");
    assert_eq!(
        catalog.load_persistent_journal_capabilities().unwrap()[0].continuity,
        PersistentJournalContinuityState::Current
    );
}

#[test]
fn empty_first_import_replay_is_durable_without_a_zero_length_source_range() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("catalog.sqlite3");
    let request = first_import_scan_request("empty-first-import-window", "root-a");
    let mut catalog = SqliteCatalog::open(path.clone()).unwrap();
    catalog
        .begin_scan(&request, "root-a", &request.root_path)
        .unwrap();
    let baseline = catalog
        .begin_persistent_journal_baseline(
            &first_import_baseline_request(&request, "root-a", 10),
            policy(),
        )
        .unwrap();
    catalog
        .publish_scan(&request.scan_id, "root-a", 0, 0)
        .unwrap();
    catalog
        .capture_persistent_journal_baseline_closing_boundary(&closing_boundary(
            baseline.change_id,
            10,
        ))
        .unwrap();
    let checkpoint = catalog
        .load_persistent_journal_checkpoint("root-a", LibraryRootGeneration::initial())
        .unwrap()
        .unwrap();
    assert!(
        !catalog
            .persistent_journal_baseline_closing_is_covered(baseline.change_id)
            .unwrap()
    );
    assert!(
        !catalog
            .finalize_ready_first_import_journal_baseline(3_000)
            .unwrap()
    );
    let mut wrong_journal = checkpoint.clone();
    wrong_journal.journal_id = JournalIdentifier::new(999).unwrap();
    assert!(
        !catalog
            .record_persistent_journal_empty_replay(&wrong_journal, 3_000)
            .unwrap()
    );
    assert_eq!(
        catalog
            .load_persistent_journal_checkpoint("root-a", LibraryRootGeneration::initial(),)
            .unwrap()
            .unwrap(),
        checkpoint
    );
    assert!(
        catalog
            .record_persistent_journal_empty_replay(&checkpoint, 3_000)
            .unwrap()
    );
    assert!(
        catalog
            .record_persistent_journal_empty_replay(&checkpoint, 3_000)
            .unwrap()
    );
    let range_count: i64 = catalog
        .connection
        .query_row(
            "SELECT COUNT(*) FROM library_persistent_journal_source_ranges",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(range_count, 0);
    drop(catalog);
    let mut catalog = SqliteCatalog::open(path).expect("reopen after empty replay receipt");
    assert_eq!(
        catalog.load_persistent_journal_baselines().unwrap()[0].phase,
        PersistentJournalBaselinePhase::Absence
    );
    assert_eq!(
        catalog
            .load_persistent_journal_checkpoint("root-a", LibraryRootGeneration::initial(),)
            .unwrap()
            .unwrap()
            .continuity,
        PersistentJournalContinuityState::CatchingUp
    );
    assert!(
        catalog
            .finalize_ready_first_import_journal_baseline(3_001)
            .unwrap()
    );
    assert_eq!(
        catalog
            .load_persistent_journal_checkpoint("root-a", LibraryRootGeneration::initial(),)
            .unwrap()
            .unwrap()
            .continuity,
        PersistentJournalContinuityState::Current
    );
}

#[test]
fn first_import_rejects_overshoot_atomically_and_reopens_after_bounded_replay() {
    let directory = tempdir().expect("catalog directory");
    let path = directory.path().join("catalog.sqlite3");
    let request = first_import_scan_request("first-import-window", "root-a");
    let mut catalog = SqliteCatalog::open(path.clone()).expect("catalog");
    catalog
        .begin_scan(&request, "root-a", &request.root_path)
        .expect("begin scan");
    let baseline = catalog
        .begin_persistent_journal_baseline(
            &first_import_baseline_request(&request, "root-a", 10),
            policy(),
        )
        .expect("baseline");
    catalog
        .publish_scan(&request.scan_id, "root-a", 0, 0)
        .expect("publish first import");
    catalog
        .capture_persistent_journal_baseline_closing_boundary(&closing_boundary(
            baseline.change_id,
            20,
        ))
        .expect("closing boundary");
    let previous = catalog
        .load_persistent_journal_checkpoint("root-a", LibraryRootGeneration::initial())
        .unwrap()
        .unwrap();

    for (end, succeeds) in [(30, false), (20, true)] {
        let mut enrollment = batch("bounded", "root-a", 10, end, &[]);
        enrollment.range.enrolled_unix_ms = 3_000;
        enrollment.range.batch_id = persistent_journal_batch_id(&enrollment);
        let mut advanced = previous.clone();
        advanced.next_unread_usn = JournalUsn::new(end).unwrap();
        advanced.captured_exclusive_end = JournalUsn::new(end).unwrap();
        advanced.updated_unix_ms = 3_000;
        let volume_batch = PersistentJournalVolumeBatch {
            pages: vec![PersistentJournalVolumePage {
                enrollment,
                checkpoint: advanced,
            }],
        };
        let result =
            catalog.publish_persistent_journal_volume_batch(&volume_batch, 3_000, policy());
        if succeeds {
            result.expect("bounded replay publication");
            assert!(
                catalog
                    .finalize_ready_first_import_journal_baseline(3_001)
                    .unwrap()
            );
        } else {
            assert_eq!(
                result.unwrap_err().code,
                "persistent_journal_baseline_read_window_invalid"
            );
            assert_eq!(
                catalog
                    .load_persistent_journal_checkpoint("root-a", LibraryRootGeneration::initial(),)
                    .unwrap()
                    .unwrap(),
                previous
            );
            assert_eq!(
                catalog
                    .connection
                    .query_row(
                        "SELECT COUNT(*) FROM library_persistent_journal_source_ranges",
                        [],
                        |row| row.get::<_, i64>(0),
                    )
                    .unwrap(),
                0
            );
        }
        drop(catalog);
        catalog = SqliteCatalog::open(path.clone()).expect("reopen preserves baseline authority");
    }
    assert_eq!(
        catalog
            .load_persistent_journal_checkpoint("root-a", LibraryRootGeneration::initial(),)
            .unwrap()
            .unwrap()
            .continuity,
        PersistentJournalContinuityState::Current
    );
}
