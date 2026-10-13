use super::*;

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
