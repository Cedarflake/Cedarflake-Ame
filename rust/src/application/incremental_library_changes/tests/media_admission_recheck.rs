use super::*;

fn retained_failures(count: usize) -> CatalogFixture {
    let source = tempdir().unwrap();
    let names = (0..count)
        .map(|index| format!("old-{index}.png"))
        .collect::<Vec<_>>();
    for (index, name) in names.iter().enumerate() {
        write_png(&source.path().join(name), 2 + index as u32, 3, [4, 9, 18]);
    }
    write_png(&source.path().join("valid.png"), 13, 7, [21, 40, 89]);
    let mut paths = names.iter().map(String::as_str).collect::<Vec<_>>();
    paths.push("valid.png");
    let mut fixture = seed_catalog(source, &paths);
    for (index, name) in names.iter().enumerate() {
        fs::write(
            fixture.source.path().join(name),
            br#"{"error":"download failed"}"#,
        )
        .unwrap();
        fixture.enqueue(&[intent(&fixture.root_id, name, None, index as u64 + 1)]);
        assert_eq!(fixture.process().completed_count, 1);
    }
    let path = fixture.catalog.catalog_path().to_path_buf();
    drop(fixture.catalog);
    let connection = Connection::open(&path).unwrap();
    connection
        .execute_batch(
            "DELETE FROM library_terminal_media_evidence;
        DROP TABLE library_media_admission_rechecks;
        UPDATE schema_info SET version = 37; PRAGMA user_version = 37;",
        )
        .unwrap();
    drop(connection);
    fixture.catalog = SqliteCatalog::open(path).expect("upgrade retained failures");
    fixture
}

fn recover(
    fixture: &mut CatalogFixture,
    now: i64,
) -> crate::domain::IncrementalLibraryChangeReport {
    crate::application::process_ready_unowned_recovery_paths_cancellable(
        &mut fixture.catalog,
        &fixture.root_id,
        LibraryRootGeneration::initial(),
        now,
        policy(),
        &AtomicBool::new(false),
    )
    .expect("targeted path recovery")
}

#[test]
fn old_failures_are_removed_in_bounded_batches_without_enumerating_sources() {
    let mut fixture = retained_failures(40);
    let valid = fixture.location("valid.png").unwrap();
    reset_source_enumeration_instrumentation(&fixture.root_path);
    reset_source_content_open_instrumentation(&fixture.root_path);
    assert_eq!(
        fixture
            .catalog
            .enqueue_media_admission_rechecks(3_000, policy())
            .unwrap(),
        32
    );
    assert_eq!(
        fixture
            .catalog
            .enqueue_media_admission_rechecks(3_000, policy())
            .unwrap(),
        8
    );
    assert_eq!(
        fixture
            .catalog
            .enqueue_media_admission_rechecks(3_000, policy())
            .unwrap(),
        0
    );
    assert_eq!(recover(&mut fixture, 4_000).completed_count, 40);
    assert_eq!(source_entry_read_count(&fixture.root_path), 0);
    assert_eq!(source_directory_open_count(&fixture.root_path), 0);
    assert!((40..=80).contains(&source_content_open_count(&fixture.root_path)));
    assert_eq!(
        fixture.location("valid.png").unwrap().asset_id,
        valid.asset_id
    );
    for index in 0..40 {
        let name = format!("old-{index}.png");
        assert!(fixture.location(&name).is_none());
        assert_eq!(
            fs::read(fixture.source.path().join(name)).unwrap(),
            br#"{"error":"download failed"}"#
        );
    }
    let path = fixture.catalog.catalog_path().to_path_buf();
    drop(fixture.catalog);
    fixture.catalog = SqliteCatalog::open(path).expect("reopen cleaned catalog");
    reset_source_enumeration_instrumentation(&fixture.root_path);
    reset_source_content_open_instrumentation(&fixture.root_path);
    assert_eq!(
        fixture
            .catalog
            .enqueue_media_admission_rechecks(5_000, policy())
            .unwrap(),
        0
    );
    assert_eq!(recover(&mut fixture, 6_000).leased_count, 0);
    assert_eq!(source_content_open_count(&fixture.root_path), 0);
    assert_eq!(source_entry_read_count(&fixture.root_path), 0);
    let connection = Connection::open(fixture.catalog.catalog_path()).unwrap();
    assert_eq!(
        connection
            .query_row("SELECT count(*) FROM scan_runs", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        1
    );
    assert_eq!(
        connection
            .query_row(
                "SELECT count(*) FROM library_metadata_inventory_runs",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
        0
    );
    assert_eq!(
        connection
            .query_row(
                "SELECT asset_count FROM scan_runs WHERE id = 'baseline-scan'",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
        1
    );
}

#[test]
fn repaired_file_keeps_its_identity_before_or_after_automatic_cleanup() {
    let mut fixture = retained_failures(2);
    let before = fixture.location("old-0.png").unwrap();
    let after = fixture.location("old-1.png").unwrap();
    write_png(
        &fixture.source.path().join("old-0.png"),
        15,
        9,
        [12, 24, 48],
    );
    fixture
        .catalog
        .enqueue_media_admission_rechecks(3_000, policy())
        .unwrap();
    assert_eq!(recover(&mut fixture, 4_000).completed_count, 2);
    let repaired = fixture.location("old-0.png").unwrap();
    assert_eq!(repaired.asset_id, before.asset_id);
    assert_eq!((repaired.width, repaired.height), (15, 9));
    assert!(fixture.location("old-1.png").is_none());
    write_png(
        &fixture.source.path().join("old-1.png"),
        7,
        19,
        [32, 64, 128],
    );
    fixture.enqueue(&[intent(&fixture.root_id, "old-1.png", None, 20)]);
    assert_eq!(fixture.process().completed_count, 1);
    let repaired = fixture.location("old-1.png").unwrap();
    assert_eq!(repaired.asset_id, after.asset_id);
    assert_eq!((repaired.width, repaired.height), (7, 19));
}

#[test]
fn queued_rechecks_resume_after_close_and_locked_files_remain_retryable() {
    let mut fixture = retained_failures(2);
    fixture
        .catalog
        .enqueue_media_admission_rechecks(3_000, policy())
        .unwrap();
    let path = fixture.catalog.catalog_path().to_path_buf();
    drop(fixture.catalog);
    fixture.catalog = SqliteCatalog::open(path).unwrap();
    let locked = fs::OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(fixture.source.path().join("old-0.png"))
        .unwrap();
    let report = recover(&mut fixture, 4_000);
    assert_eq!(report.completed_count, 1);
    assert_eq!(report.retried_count, 1);
    assert!(fixture.location("old-0.png").is_some());
    assert!(fixture.location("old-1.png").is_none());
    drop(locked);
    assert_eq!(recover(&mut fixture, 100_000).completed_count, 1);
    assert!(fixture.location("old-0.png").is_none());
}

#[test]
fn paused_imports_and_queue_pressure_never_expand_cleanup_into_a_scan() {
    let mut fixture = retained_failures(3);
    let connection = Connection::open(fixture.catalog.catalog_path()).unwrap();
    connection
        .execute(
            "UPDATE scan_runs SET status = 'paused' WHERE id = 'baseline-scan'",
            [],
        )
        .unwrap();
    assert_eq!(
        fixture
            .catalog
            .enqueue_media_admission_rechecks(3_000, policy())
            .unwrap(),
        0
    );
    connection
        .execute(
            "UPDATE scan_runs SET status = 'completed' WHERE id = 'baseline-scan'",
            [],
        )
        .unwrap();
    let bounded_policy = LibraryChangeQueuePolicy {
        max_unresolved_changes: 2,
        max_lease_batch: 2,
        ..policy()
    };
    assert_eq!(
        fixture
            .catalog
            .enqueue_media_admission_rechecks(3_000, bounded_policy)
            .unwrap(),
        1
    );
    assert_eq!(
        fixture
            .catalog
            .enqueue_media_admission_rechecks(3_000, bounded_policy)
            .unwrap(),
        0
    );
    assert_eq!(
        connection
            .query_row(
                "SELECT count(*) FROM library_change_queue WHERE scope <> 'path'",
                [],
                |row| row.get::<_, i64>(0)
            )
            .unwrap(),
        0
    );
    assert_eq!(recover(&mut fixture, 4_000).completed_count, 1);
    assert_eq!(
        fixture
            .catalog
            .enqueue_media_admission_rechecks(5_000, policy())
            .unwrap(),
        2
    );
    assert_eq!(recover(&mut fixture, 6_000).completed_count, 2);
}

#[test]
fn replacing_a_legacy_failure_with_another_image_does_not_claim_its_identity() {
    let mut fixture = retained_failures(1);
    let original = fixture.location("old-0.png").unwrap();
    // Keep the old inode allocated so the filesystem cannot immediately reuse its ID.
    fs::rename(
        fixture.source.path().join("old-0.png"),
        fixture.source.path().join("old-error.txt"),
    )
    .unwrap();
    write_png(
        &fixture.source.path().join("old-0.png"),
        17,
        9,
        [19, 38, 76],
    );
    fixture
        .catalog
        .enqueue_media_admission_rechecks(3_000, policy())
        .unwrap();
    assert_eq!(recover(&mut fixture, 4_000).completed_count, 1);
    let replacement = fixture.location("old-0.png").unwrap();
    assert_ne!(replacement.file_identity, original.file_identity);
    assert_ne!(replacement.asset_id, original.asset_id);
    assert_eq!((replacement.width, replacement.height), (17, 9));
    assert_eq!(
        fixture
            .catalog
            .enqueue_media_admission_rechecks(5_000, policy())
            .unwrap(),
        0
    );
}

#[test]
fn a_rename_arriving_before_cleanup_cannot_retain_an_invalid_card() {
    let mut fixture = retained_failures(1);
    let original = fixture.location("old-0.png").unwrap();
    fs::rename(
        fixture.source.path().join("old-0.png"),
        fixture.source.path().join("renamed.png"),
    )
    .unwrap();
    fixture.enqueue(&[intent(
        &fixture.root_id,
        "renamed.png",
        Some("old-0.png"),
        20,
    )]);
    assert_eq!(fixture.process().completed_count, 1);
    assert!(fixture.location("old-0.png").is_none());
    assert!(fixture.location("renamed.png").is_none());
    assert_eq!(
        fixture
            .catalog
            .enqueue_media_admission_rechecks(3_000, policy())
            .unwrap(),
        0
    );
    write_png(
        &fixture.source.path().join("renamed.png"),
        9,
        17,
        [33, 66, 99],
    );
    fixture.enqueue(&[intent(&fixture.root_id, "renamed.png", None, 21)]);
    assert_eq!(fixture.process().completed_count, 1);
    assert_eq!(
        fixture.location("renamed.png").unwrap().asset_id,
        original.asset_id
    );
}

#[test]
fn automatic_cleanup_defers_without_blocking_when_an_external_writer_is_active() {
    let mut fixture = retained_failures(1);
    let mut connection = Connection::open(fixture.catalog.catalog_path()).unwrap();
    let held = connection
        .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
        .unwrap();
    let started = std::time::Instant::now();
    assert_eq!(
        fixture
            .catalog
            .enqueue_media_admission_rechecks(3_000, policy())
            .unwrap(),
        0
    );
    assert!(started.elapsed() < std::time::Duration::from_secs(2));
    assert!(fixture.location("old-0.png").is_some());
    drop(held);
    assert_eq!(
        fixture
            .catalog
            .enqueue_media_admission_rechecks(4_000, policy())
            .unwrap(),
        1
    );
    assert_eq!(recover(&mut fixture, 5_000).completed_count, 1);
}
