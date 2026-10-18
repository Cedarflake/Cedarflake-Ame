use super::super as journal;
use super::*;

#[test]
fn closing_failure_after_checkpoint_write_preserves_the_opening_on_reopen() {
    let directory = tempdir().expect("catalog directory");
    let path = directory.path().join("catalog.sqlite3");
    let mut catalog = catalog_with_roots(path.clone(), &["root-a"]);
    require_baseline_root(&mut catalog, "root-a");
    let baseline = catalog
        .begin_persistent_journal_baseline(&baseline_request("root-a", 10), policy())
        .expect("admit baseline");
    catalog
        .connection
        .execute_batch(
            "CREATE TEMP TRIGGER fail_baseline_closing
             BEFORE UPDATE ON library_persistent_journal_root_state
             WHEN NEW.continuity_state = 'catching_up'
             BEGIN SELECT RAISE(ABORT, 'closing failure after checkpoint'); END;",
        )
        .expect("inject late closing failure");

    let error = catalog
        .capture_persistent_journal_baseline_closing_boundary(&closing_boundary(
            baseline.change_id,
            20,
        ))
        .expect_err("late failure must roll back baseline and checkpoint together");
    assert!(error.message.contains("closing failure after checkpoint"));
    drop(catalog);

    let mut reopened = SqliteCatalog::open(path).expect("reopen rolled back baseline");
    assert_eq!(
        reopened
            .load_persistent_journal_baselines()
            .expect("baseline"),
        vec![baseline.clone()]
    );
    assert!(
        reopened
            .load_persistent_journal_checkpoint("root-a", LibraryRootGeneration::initial())
            .expect("checkpoint after rollback")
            .is_none()
    );
    let state: String = reopened
        .connection
        .query_row(
            "SELECT continuity_state FROM library_persistent_journal_root_state
             WHERE root_id = 'root-a'",
            [],
            |row| row.get(0),
        )
        .expect("root state after rollback");
    assert_eq!(state, "baseline_required");
    let captured = reopened
        .capture_persistent_journal_baseline_closing_boundary(&closing_boundary(
            baseline.change_id,
            20,
        ))
        .expect("retry closing after reopening");
    assert_eq!(captured.phase, PersistentJournalBaselinePhase::Replay);
}

#[test]
fn empty_baseline_interval_requires_a_read_receipt_without_a_source_range() {
    let directory = tempdir().expect("catalog directory");
    let mut catalog = catalog_with_roots(directory.path().join("catalog.sqlite3"), &["root-a"]);
    require_baseline_root(&mut catalog, "root-a");
    let baseline = catalog
        .begin_persistent_journal_baseline(&baseline_request("root-a", 10), policy())
        .expect("admit baseline");
    catalog
        .capture_persistent_journal_baseline_closing_boundary(&closing_boundary(
            baseline.change_id,
            10,
        ))
        .expect("capture empty closing interval");

    assert!(
        !catalog
            .persistent_journal_baseline_closing_is_covered(baseline.change_id)
            .expect("empty interval is not yet read")
    );
    let checkpoint = catalog
        .load_persistent_journal_checkpoint("root-a", LibraryRootGeneration::initial())
        .expect("load empty replay checkpoint")
        .expect("empty replay checkpoint");
    assert!(
        catalog
            .record_persistent_journal_empty_replay(&checkpoint, 3_000)
            .expect("record empty replay receipt")
    );

    assert!(
        catalog
            .persistent_journal_baseline_closing_is_covered(baseline.change_id)
            .expect("empty interval coverage")
    );
    let range_count: i64 = catalog
        .connection
        .query_row(
            "SELECT COUNT(*) FROM library_persistent_journal_source_ranges",
            [],
            |row| row.get(0),
        )
        .expect("source range count");
    assert_eq!(range_count, 0);
}

#[test]
fn watcher_gap_closing_identity_and_transaction_failure_preserve_opening_window() {
    let directory = tempdir().expect("catalog directory");
    let path = directory.path().join("catalog.sqlite3");
    let mut catalog = catalog_with_roots(path.clone(), &["root-a"]);
    current_root(&mut catalog, "root-a");
    let baseline = promote_watcher_gap(&mut catalog, "root-a");
    let mut mismatch = closing_boundary(baseline.change_id, 20);
    mismatch.volume.volume_serial = 99;
    let identity_error = catalog
        .capture_persistent_journal_baseline_closing_boundary(&mismatch)
        .expect_err("identity drift must not close the watcher window");
    assert_eq!(
        identity_error.code,
        "persistent_journal_baseline_closing_mismatch"
    );
    let initial: (String, Option<String>, String, String, String, String) = catalog
        .connection
        .query_row(
            "SELECT baseline.phase, baseline.closing_next_usn,
                    checkpoint.next_unread_usn, checkpoint.captured_exclusive_end,
                    checkpoint.continuity_state, root.continuity_state
             FROM library_persistent_journal_baselines AS baseline
             JOIN library_persistent_journal_checkpoints AS checkpoint
               ON checkpoint.root_id = baseline.root_id
              AND checkpoint.root_generation = baseline.root_generation
             JOIN library_persistent_journal_root_state AS root
               ON root.root_id = baseline.root_id
              AND root.root_generation = baseline.root_generation
             WHERE baseline.change_id = ?1",
            [
                sqlite_integer(baseline.change_id.value(), "baseline change ID")
                    .expect("baseline change ID"),
            ],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                ))
            },
        )
        .expect("opening window state");
    assert_eq!(
        initial,
        (
            "inventory".to_owned(),
            None,
            "10".to_owned(),
            "10".to_owned(),
            "recovery_required".to_owned(),
            "recovery_required".to_owned(),
        )
    );

    catalog
        .connection
        .execute_batch(
            "CREATE TEMP TRIGGER inject_watcher_closing_failure
             BEFORE UPDATE OF closing_next_usn ON library_persistent_journal_baselines
             BEGIN
               SELECT RAISE(ABORT, 'injected watcher closing failure');
             END;",
        )
        .expect("install closing transaction failure");
    let closing = closing_boundary(baseline.change_id, 20);
    let transaction_error = catalog
        .capture_persistent_journal_baseline_closing_boundary(&closing)
        .expect_err("closing failure must roll back checkpoint and window");
    assert_eq!(transaction_error.code, "catalog_database_error");
    let rolled_back: (String, Option<String>, String, String, String, String) = catalog
        .connection
        .query_row(
            "SELECT baseline.phase, baseline.closing_next_usn,
                    checkpoint.next_unread_usn, checkpoint.captured_exclusive_end,
                    checkpoint.continuity_state, root.continuity_state
             FROM library_persistent_journal_baselines AS baseline
             JOIN library_persistent_journal_checkpoints AS checkpoint
               ON checkpoint.root_id = baseline.root_id
              AND checkpoint.root_generation = baseline.root_generation
             JOIN library_persistent_journal_root_state AS root
               ON root.root_id = baseline.root_id
              AND root.root_generation = baseline.root_generation
             WHERE baseline.change_id = ?1",
            [
                sqlite_integer(baseline.change_id.value(), "baseline change ID")
                    .expect("baseline change ID"),
            ],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                ))
            },
        )
        .expect("rolled back closing state");
    assert_eq!(rolled_back, initial);
    catalog
        .connection
        .execute_batch("DROP TRIGGER inject_watcher_closing_failure")
        .expect("remove closing transaction failure");

    let captured = catalog
        .capture_persistent_journal_baseline_closing_boundary(&closing)
        .expect("capture watcher closing boundary");
    assert_eq!(captured.phase, PersistentJournalBaselinePhase::Replay);
    drop(catalog);
    let catalog = SqliteCatalog::open(path).expect("reopen watcher closing window");
    let reopened = catalog
        .load_persistent_journal_baselines()
        .expect("load reopened watcher window");
    assert_eq!(reopened, vec![captured]);
    let state: (String, String, String, String) = catalog
        .connection
        .query_row(
            "SELECT checkpoint.next_unread_usn, checkpoint.captured_exclusive_end,
                    checkpoint.continuity_state, root.continuity_state
             FROM library_persistent_journal_checkpoints AS checkpoint
             JOIN library_persistent_journal_root_state AS root
               ON root.root_id = checkpoint.root_id
              AND root.root_generation = checkpoint.root_generation
             WHERE checkpoint.root_id = 'root-a'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .expect("reopened watcher catch-up state");
    assert_eq!(
        state,
        (
            "10".to_owned(),
            "20".to_owned(),
            "catching_up".to_owned(),
            "catching_up".to_owned(),
        )
    );
}

#[test]
fn watcher_gap_replay_and_candidate_barriers_atomically_publish_current() {
    let directory = tempdir().expect("catalog directory");
    let mut catalog = catalog_with_roots(directory.path().join("catalog.sqlite3"), &["root-a"]);
    let source_root = directory.path().join("root-a-source");
    std::fs::create_dir(&source_root).expect("watcher recovery source root");
    let source_root_path = source_root.to_string_lossy().into_owned();
    catalog
        .connection
        .execute(
            "UPDATE library_roots SET path = ?1 WHERE id = 'root-a'",
            [&source_root_path],
        )
        .expect("bind watcher recovery source root");
    current_root(&mut catalog, "root-a");
    let baseline = promote_watcher_gap(&mut catalog, "root-a");
    let authority = catalog
        .load_metadata_inventory_recovery_authority(baseline.change_id)
        .expect("load watcher recovery authority")
        .expect("watcher recovery authority");
    catalog
        .begin_metadata_inventory(&MetadataInventoryRunRequest {
            run_id: authority.run_id.clone(),
            root_id: "root-a".to_owned(),
            root_generation: LibraryRootGeneration::initial(),
            epoch: 1,
            scope: MetadataInventoryScope::Subtree {
                relative_path: "album".to_owned(),
            },
            started_unix_ms: 1_002,
        })
        .expect("begin watcher inventory");
    let root_identity = crate::adapters::FileDiscovery::new(&source_root_path)
        .expect("pin watcher recovery source root")
        .metadata_inventory_root_identity()
        .expect("read watcher recovery root identity")
        .expect("stable watcher recovery root identity");
    catalog
        .connection
        .execute(
            "INSERT INTO library_metadata_inventory_spools(
               run_id, authority_change_id, root_id, root_generation,
               root_identity_scheme, root_identity_value,
               scope_kind, scope_relative_path, state,
               created_unix_ms, updated_unix_ms
             ) VALUES (?1, ?2, 'root-a', 1, ?3, ?4, 'subtree', 'album',
                       'ready', 1002, 1002)",
            rusqlite::params![
                authority.run_id,
                sqlite_integer(authority.change_id.value(), "recovery change ID")
                    .expect("recovery change ID"),
                root_identity.scheme,
                root_identity.value,
            ],
        )
        .expect("persist watcher recovery root proof");
    catalog
        .stage_metadata_inventory_page(
            &authority.run_id,
            &MetadataInventoryPage {
                page_index: 1,
                entries: Vec::new(),
                cursor: None,
                is_complete: true,
                frontier: vec![MetadataInventoryFrontierEntry::completed("", None)],
            },
            1_003,
        )
        .expect("complete watcher inventory enumeration");
    catalog
        .capture_persistent_journal_baseline_closing_boundary(&closing_boundary(
            baseline.change_id,
            20,
        ))
        .expect("capture watcher closing boundary");
    let absence_before_replay = catalog
        .authorize_metadata_inventory_absence(&authority.run_id, 1_004)
        .expect_err("absence must wait for closing replay");
    assert_eq!(
        absence_before_replay.code,
        "persistent_journal_baseline_closing_incomplete"
    );

    let mut closing_replay = atomic_volume_batch(vec![batch(
        "watcher-gap-range",
        "root-a",
        10,
        20,
        &["during-gap.jpg"],
    )]);
    closing_replay.pages[0].enrollment.range.enrolled_unix_ms = 1_005;
    rebind_volume_batch_content(&mut closing_replay);
    catalog
        .publish_persistent_journal_volume_batch(&closing_replay, 1_005, policy())
        .expect("persist closing replay range");
    assert!(
        !catalog
            .persistent_journal_baseline_closing_is_covered(baseline.change_id)
            .expect("pending replay coverage")
    );
    let absence_while_p1_pending = catalog
        .authorize_metadata_inventory_absence(&authority.run_id, 1_006)
        .expect_err("absence must wait for terminal P1 ownership");
    assert_eq!(
        absence_while_p1_pending.code,
        "persistent_journal_baseline_closing_incomplete"
    );
    let journal = catalog
        .lease_path_library_changes_in_lane(
            "root-a",
            LibraryRootGeneration::initial(),
            crate::domain::LibraryChangeLane::Journal,
            1_006,
            policy(),
        )
        .expect("lease closing replay work");
    assert_eq!(journal.len(), 1);
    let catalog_revision = catalog
        .connection
        .query_row("SELECT revision FROM catalog_state", [], |row| {
            row.get::<_, i64>(0)
        })
        .expect("catalog revision");
    let catalog_revision = u64::try_from(catalog_revision).expect("nonnegative catalog revision");
    assert_eq!(
        catalog
            .complete_library_change(
                journal[0].change.id,
                journal[0].lease_generation,
                catalog_revision,
                1_007,
            )
            .expect("complete closing replay work"),
        crate::domain::LibraryChangeLeaseUpdateOutcome::Applied,
    );
    let transaction = catalog
        .connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .expect("finalize replay transaction");
    journal::finalize_persistent_journal_ranges(&transaction, 1_008)
        .expect("finalize closing replay range");
    transaction.commit().expect("commit replay finalization");
    assert!(
        catalog
            .persistent_journal_baseline_closing_is_covered(baseline.change_id)
            .expect("terminal replay coverage")
    );
    catalog
        .authorize_metadata_inventory_absence(&authority.run_id, 1_009)
        .expect("authorize absence after closing replay");
    let recovery = catalog
        .lease_metadata_inventory_recovery(
            "root-a",
            LibraryRootGeneration::initial(),
            1_010,
            policy(),
        )
        .expect("lease watcher recovery")
        .expect("watcher recovery lease");

    catalog
        .connection
        .execute_batch(
            "CREATE TEMP TRIGGER inject_watcher_completion_failure
             BEFORE UPDATE OF retired_unix_ms ON library_recovery_authorities
             BEGIN
               SELECT RAISE(ABORT, 'injected watcher completion failure');
             END;",
        )
        .expect("install completion transaction failure");
    let completion_error = catalog
        .finish_metadata_inventory_recovery(
            recovery.change.id,
            recovery.lease_generation,
            catalog_revision,
            1_011,
        )
        .expect_err("final authority failure must roll back Current publication");
    assert_eq!(
        completion_error.code,
        "metadata_inventory_recovery_authority_rejected"
    );
    let rolled_back: (String, String, String, String, String, Option<i64>) = catalog
        .connection
        .query_row(
            "SELECT queue.status, run.status, baseline.phase,
                    checkpoint.continuity_state, root.continuity_state,
                    authority.retired_unix_ms
             FROM library_change_queue AS queue
             JOIN library_recovery_authorities AS authority
               ON authority.change_id = queue.id
             JOIN library_metadata_inventory_runs AS run
               ON run.id = authority.run_id
             JOIN library_persistent_journal_baselines AS baseline
               ON baseline.change_id = authority.change_id
             JOIN library_persistent_journal_checkpoints AS checkpoint
               ON checkpoint.root_id = baseline.root_id
              AND checkpoint.root_generation = baseline.root_generation
             JOIN library_persistent_journal_root_state AS root
               ON root.root_id = baseline.root_id
              AND root.root_generation = baseline.root_generation
             WHERE queue.id = ?1",
            [
                sqlite_integer(recovery.change.id.value(), "recovery change ID")
                    .expect("recovery change ID"),
            ],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                ))
            },
        )
        .expect("rolled back Current state");
    assert_eq!(rolled_back.0, "leased");
    assert_eq!(rolled_back.1, "comparing");
    assert_eq!(rolled_back.2, "absence");
    assert_eq!(rolled_back.3, "catching_up");
    assert_eq!(rolled_back.4, "catching_up");
    assert_eq!(rolled_back.5, None);
    catalog
        .connection
        .execute_batch("DROP TRIGGER inject_watcher_completion_failure")
        .expect("remove completion transaction failure");
    assert_eq!(
        catalog
            .finish_metadata_inventory_recovery(
                recovery.change.id,
                recovery.lease_generation,
                catalog_revision,
                1_012,
            )
            .expect("atomically finish watcher recovery"),
        Some(crate::domain::LibraryChangeLeaseUpdateOutcome::Applied),
    );
    let completed: (String, String, String, String, String, Option<i64>) = catalog
        .connection
        .query_row(
            "SELECT queue.status, run.status, baseline.phase,
                    checkpoint.continuity_state, root.continuity_state,
                    authority.retired_unix_ms
             FROM library_change_queue AS queue
             JOIN library_recovery_authorities AS authority
               ON authority.change_id = queue.id
             JOIN library_metadata_inventory_runs AS run
               ON run.id = authority.run_id
             JOIN library_persistent_journal_baselines AS baseline
               ON baseline.change_id = authority.change_id
             JOIN library_persistent_journal_checkpoints AS checkpoint
               ON checkpoint.root_id = baseline.root_id
              AND checkpoint.root_generation = baseline.root_generation
             JOIN library_persistent_journal_root_state AS root
               ON root.root_id = baseline.root_id
              AND root.root_generation = baseline.root_generation
             WHERE queue.id = ?1",
            [
                sqlite_integer(recovery.change.id.value(), "recovery change ID")
                    .expect("recovery change ID"),
            ],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                ))
            },
        )
        .expect("completed watcher recovery state");
    assert_eq!(completed.0, "completed");
    assert_eq!(completed.1, "completed");
    assert_eq!(completed.2, "completed");
    assert_eq!(completed.3, "current");
    assert_eq!(completed.4, "current");
    assert_eq!(completed.5, Some(1_012));
}

#[test]
fn baseline_closing_boundary_rejects_identity_drift_and_is_idempotent() {
    let directory = tempdir().expect("catalog directory");
    let mut catalog = catalog_with_roots(directory.path().join("catalog.sqlite3"), &["root-a"]);
    require_baseline_root(&mut catalog, "root-a");
    let baseline = catalog
        .begin_persistent_journal_baseline(&baseline_request("root-a", 10), policy())
        .expect("admit baseline");
    let mut mismatch = closing_boundary(baseline.change_id, 20);
    mismatch.journal_id = JournalIdentifier::new(10).expect("other journal ID");
    let error = catalog
        .capture_persistent_journal_baseline_closing_boundary(&mismatch)
        .expect_err("journal drift must fail closed");
    let checkpoint_count: i64 = catalog
        .connection
        .query_row(
            "SELECT COUNT(*) FROM library_persistent_journal_checkpoints",
            [],
            |row| row.get(0),
        )
        .expect("checkpoint count after mismatch");
    assert_eq!(error.code, "persistent_journal_baseline_closing_mismatch");
    assert_eq!(checkpoint_count, 0);

    let boundary = closing_boundary(baseline.change_id, 20);
    let captured = catalog
        .capture_persistent_journal_baseline_closing_boundary(&boundary)
        .expect("capture closing boundary");
    let replay = catalog
        .capture_persistent_journal_baseline_closing_boundary(&boundary)
        .expect("replay closing capture");
    let checkpoint = catalog
        .load_persistent_journal_checkpoint("root-a", LibraryRootGeneration::initial())
        .expect("load provisional checkpoint")
        .expect("provisional checkpoint");

    assert_eq!(captured, replay);
    assert_eq!(captured.phase, PersistentJournalBaselinePhase::Replay);
    assert_eq!(
        captured.closing_next_usn,
        Some(JournalUsn::new(20).expect("USN"))
    );
    assert_eq!(
        checkpoint.next_unread_usn,
        JournalUsn::new(10).expect("USN")
    );
    assert_eq!(
        checkpoint.captured_exclusive_end,
        JournalUsn::new(20).expect("USN")
    );
    assert!(
        !catalog
            .persistent_journal_baseline_closing_is_covered(baseline.change_id)
            .expect("coverage state")
    );
}
