use super::*;

#[test]
fn live_watcher_gap_promotion_is_atomic_and_idempotent() {
    let directory = tempdir().expect("temporary directory");
    let generation = LibraryRootGeneration::initial();
    let policy = immediate_policy();
    let mut catalog = queue_catalog(directory.path().join("catalog.sqlite3"));
    let opening = seed_current_journal_authority(&mut catalog);
    catalog
        .enqueue_library_change_intents(
            &[intent(
                "root-a",
                generation,
                1,
                1_000,
                LibraryChangeIntentKind::Reconcile,
                LibraryChangeScope::Subtree,
                "album",
            )],
            1_000,
            policy,
        )
        .expect("enqueue bounded watcher work");
    let live = catalog
        .lease_authoritative_library_change("root-a", generation, 1_000, policy)
        .expect("lease bounded watcher work")
        .expect("bounded watcher lease");
    let failure = LibraryChangeFailure {
        code: "metadata_inventory_required".to_owned(),
        message: "The bounded watcher scope could not be reconstructed".to_owned(),
    };

    assert_eq!(
        catalog
            .promote_live_watcher_gap_to_metadata_inventory(
                live.change.id,
                live.lease_generation,
                &failure,
                1_001,
                policy,
            )
            .expect("promote proven watcher gap"),
        LibraryChangeLeaseUpdateOutcome::Applied,
    );
    assert_eq!(
        catalog
            .promote_live_watcher_gap_to_metadata_inventory(
                live.change.id,
                live.lease_generation,
                &failure,
                1_002,
                policy,
            )
            .expect("replay promotion"),
        LibraryChangeLeaseUpdateOutcome::Superseded,
    );

    let evidence: (
        String,
        i64,
        String,
        String,
        String,
        i64,
        Option<i64>,
        String,
        String,
        i64,
    ) = catalog
        .connection
        .query_row(
            "SELECT original.status, original.superseded_by_change_id,
                    recovery.origin, lanes.lane, recovery.status,
                    recovery.lease_generation, recovery.lease_expires_unix_ms,
                    recovery.last_failure_code, authority.reason,
                    (SELECT COUNT(*) FROM library_recovery_authorities)
             FROM library_change_queue AS original
             JOIN library_change_queue AS recovery
               ON recovery.id = original.superseded_by_change_id
             JOIN library_change_queue_lanes AS lanes
               ON lanes.change_id = recovery.id
             JOIN library_recovery_authorities AS authority
               ON authority.change_id = recovery.id
             WHERE original.id = ?1",
            [sqlite_integer(live.change.id.value(), "change ID").expect("change ID")],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                    row.get(7)?,
                    row.get(8)?,
                    row.get(9)?,
                ))
            },
        )
        .expect("atomic promotion evidence");
    assert_eq!(evidence.0, "superseded");
    assert!(evidence.1 > 0);
    assert_eq!(evidence.2, "metadata_inventory");
    assert_eq!(evidence.3, "p2_recovery");
    assert_eq!(evidence.4, "pending");
    assert_eq!(evidence.5, 0);
    assert_eq!(evidence.6, None);
    assert_eq!(evidence.7, "metadata_inventory_required");
    assert_eq!(evidence.8, "watcher_uncovered_gap");
    assert_eq!(evidence.9, 1);
    let recovery_id =
        LibraryChangeId::new(u64::try_from(evidence.1).expect("positive recovery change ID"))
            .expect("recovery change ID");
    let authority = catalog
        .load_metadata_inventory_recovery_authority(recovery_id)
        .expect("load watcher-gap authority")
        .expect("watcher-gap authority");
    let boundary = authority
        .opening_boundary
        .expect("watcher-gap opening boundary");
    assert_eq!(boundary.volume, opening.volume);
    assert_eq!(boundary.root_file_reference, opening.root_file_reference);
    assert_eq!(boundary.journal_id, opening.journal_id);
    assert_eq!(boundary.next_usn, opening.next_unread_usn);
    assert_eq!(boundary.protocol_version, opening.protocol_version);
    assert_eq!(boundary.contract_version, opening.contract_version);
    let window: (String, Option<String>, String, String, String, String) = catalog
        .connection
        .query_row(
            "SELECT baseline.phase, baseline.closing_next_usn,
                    checkpoint.continuity_state, checkpoint.next_unread_usn,
                    root.continuity_state, checkpoint.last_failure_code
             FROM library_persistent_journal_baselines AS baseline
             JOIN library_persistent_journal_checkpoints AS checkpoint
               ON checkpoint.root_id = baseline.root_id
              AND checkpoint.root_generation = baseline.root_generation
             JOIN library_persistent_journal_root_state AS root
               ON root.root_id = baseline.root_id
              AND root.root_generation = baseline.root_generation
             WHERE baseline.change_id = ?1",
            [sqlite_integer(recovery_id.value(), "recovery change ID")
                .expect("recovery change ID")],
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
        .expect("watcher-gap recovery window");
    assert_eq!(window.0, "inventory");
    assert_eq!(window.1, None);
    assert_eq!(window.2, "recovery_required");
    assert_eq!(window.3, opening.next_unread_usn.to_canonical_text());
    assert_eq!(window.4, "recovery_required");
    assert_eq!(window.5, "metadata_inventory_required");
}

#[test]
fn failed_live_watcher_gap_promotion_rolls_back_every_record() {
    let directory = tempdir().expect("temporary directory");
    let generation = LibraryRootGeneration::initial();
    let policy = immediate_policy();
    let mut catalog = queue_catalog(directory.path().join("catalog.sqlite3"));
    seed_current_journal_authority(&mut catalog);
    catalog
        .enqueue_library_change_intents(
            &[intent(
                "root-a",
                generation,
                1,
                1_000,
                LibraryChangeIntentKind::Reconcile,
                LibraryChangeScope::Subtree,
                "album",
            )],
            1_000,
            policy,
        )
        .expect("enqueue bounded watcher work");
    let live = catalog
        .lease_authoritative_library_change("root-a", generation, 1_000, policy)
        .expect("lease bounded watcher work")
        .expect("bounded watcher lease");
    let failure = LibraryChangeFailure {
        code: "metadata_inventory_required".to_owned(),
        message: "The bounded watcher scope could not be reconstructed".to_owned(),
    };
    catalog
        .connection
        .execute_batch(
            "CREATE TEMP TRIGGER inject_recovery_authority_failure
             BEFORE INSERT ON library_recovery_authorities
             BEGIN
               SELECT RAISE(ABORT, 'injected promotion failure');
             END;",
        )
        .expect("install transaction failure fixture");

    let error = catalog
        .promote_live_watcher_gap_to_metadata_inventory(
            live.change.id,
            live.lease_generation,
            &failure,
            1_001,
            policy,
        )
        .expect_err("authority failure rolls back promotion");
    assert_eq!(error.code, "metadata_inventory_recovery_authority_rejected");
    let rollback_evidence: (String, Option<i64>, i64, i64, i64, String, String) = catalog
        .connection
        .query_row(
            "SELECT status, superseded_by_change_id,
                    (SELECT COUNT(*) FROM library_change_queue
                     WHERE origin = 'metadata_inventory'),
                    (SELECT COUNT(*) FROM library_recovery_authorities),
                    (SELECT COUNT(*) FROM library_persistent_journal_baselines),
                    (SELECT continuity_state FROM library_persistent_journal_checkpoints
                     WHERE root_id = 'root-a'),
                    (SELECT continuity_state FROM library_persistent_journal_root_state
                     WHERE root_id = 'root-a')
             FROM library_change_queue WHERE id = ?1",
            [sqlite_integer(live.change.id.value(), "change ID").expect("change ID")],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                ))
            },
        )
        .expect("rollback evidence");
    assert_eq!(rollback_evidence.0, "leased");
    assert_eq!(rollback_evidence.1, None);
    assert_eq!(rollback_evidence.2, 0);
    assert_eq!(rollback_evidence.3, 0);
    assert_eq!(rollback_evidence.4, 0);
    assert_eq!(rollback_evidence.5, "current");
    assert_eq!(rollback_evidence.6, "current");

    catalog
        .connection
        .execute_batch("DROP TRIGGER inject_recovery_authority_failure")
        .expect("remove transaction failure fixture");
    assert_eq!(
        catalog
            .promote_live_watcher_gap_to_metadata_inventory(
                live.change.id,
                live.lease_generation,
                &failure,
                1_002,
                policy,
            )
            .expect("retry atomic promotion"),
        LibraryChangeLeaseUpdateOutcome::Applied,
    );
}

#[test]
fn live_watcher_gap_promotion_respects_lower_lane_reserve() {
    let directory = tempdir().expect("temporary directory");
    let generation = LibraryRootGeneration::initial();
    let policy = LibraryChangeQueuePolicy {
        max_unresolved_changes: 2,
        max_lease_batch: 1,
        ..immediate_policy()
    };
    let mut catalog = queue_catalog(directory.path().join("catalog.sqlite3"));
    seed_current_journal_authority(&mut catalog);
    let mut journal = path_intent("root-a", generation, 1, 1_000, "journal.jpg");
    journal.origin = LibraryChangeOrigin::StartupCatchUp;
    catalog
        .enqueue_library_change_intents(&[journal], 1_000, policy)
        .expect("fill the lower-lane allowance");
    assert_eq!(
        catalog
            .lease_path_library_changes_in_lane(
                "root-a",
                generation,
                LibraryChangeLane::Journal,
                1_000,
                policy,
            )
            .expect("lease lower-lane work")
            .len(),
        1,
    );
    catalog
        .enqueue_library_change_intents(
            &[intent(
                "root-a",
                generation,
                2,
                1_001,
                LibraryChangeIntentKind::Reconcile,
                LibraryChangeScope::Subtree,
                "album",
            )],
            1_001,
            policy,
        )
        .expect("use reserved live admission");
    let live = catalog
        .lease_authoritative_library_change("root-a", generation, 1_001, policy)
        .expect("lease reserved live work")
        .expect("reserved live lease");
    let error = catalog
        .promote_live_watcher_gap_to_metadata_inventory(
            live.change.id,
            live.lease_generation,
            &LibraryChangeFailure {
                code: "metadata_inventory_required".to_owned(),
                message: "The bounded watcher scope could not be reconstructed".to_owned(),
            },
            1_002,
            policy,
        )
        .expect_err("P2 promotion cannot consume P0 reserve");
    assert_eq!(error.code, "change_queue_backpressure");

    let evidence: (String, Option<i64>, i64, i64, String, String) = catalog
        .connection
        .query_row(
            "SELECT status, superseded_by_change_id,
                    (SELECT COUNT(*) FROM library_change_queue
                     WHERE origin = 'metadata_inventory'),
                    (SELECT COUNT(*) FROM library_recovery_authorities),
                    (SELECT continuity_state FROM library_persistent_journal_checkpoints
                     WHERE root_id = 'root-a'),
                    (SELECT continuity_state FROM library_persistent_journal_root_state
                     WHERE root_id = 'root-a')
             FROM library_change_queue WHERE id = ?1",
            [sqlite_integer(live.change.id.value(), "change ID").expect("change ID")],
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
        .expect("reserved admission evidence");
    assert_eq!(evidence.0, "leased");
    assert_eq!(evidence.1, None);
    assert_eq!(evidence.2, 0);
    assert_eq!(evidence.3, 0);
    assert_eq!(evidence.4, "current");
    assert_eq!(evidence.5, "current");
    assert_eq!(
        catalog
            .defer_library_change_for_capacity(
                live.change.id,
                live.lease_generation,
                LibraryChangeCapacityDeferral::MetadataInventoryLane,
                1_003,
                policy,
            )
            .expect("subtree waits for lower-lane capacity"),
        LibraryChangeLeaseUpdateOutcome::Applied
    );
    assert!(
        !catalog
            .has_ready_live_authoritative_library_change("root-a", generation, 1_003, policy)
            .expect("capacity wait must not spin")
    );
}

#[test]
fn live_watcher_gap_without_current_checkpoint_fails_closed_without_p2() {
    let directory = tempdir().expect("temporary directory");
    let generation = LibraryRootGeneration::initial();
    let policy = immediate_policy();
    let mut catalog = queue_catalog(directory.path().join("catalog.sqlite3"));
    catalog
        .save_persistent_journal_capability(&PersistentJournalCapability {
            root_id: "root-a".to_owned(),
            root_generation: generation,
            protocol_version: 5,
            contract_version: 1,
            state: PersistentJournalCapabilityState::Supported,
            continuity: PersistentJournalContinuityState::Current,
            failure: None,
            updated_unix_ms: 900,
        })
        .expect("current journal root without checkpoint");
    catalog
        .enqueue_library_change_intents(
            &[intent(
                "root-a",
                generation,
                1,
                1_000,
                LibraryChangeIntentKind::Reconcile,
                LibraryChangeScope::Subtree,
                "album",
            )],
            1_000,
            policy,
        )
        .expect("enqueue bounded watcher work");
    let live = catalog
        .lease_authoritative_library_change("root-a", generation, 1_000, policy)
        .expect("lease bounded watcher work")
        .expect("bounded watcher lease");
    let failure = LibraryChangeFailure {
        code: "metadata_inventory_required".to_owned(),
        message: "The bounded watcher scope could not be reconstructed".to_owned(),
    };

    assert_eq!(
        catalog
            .promote_live_watcher_gap_to_metadata_inventory(
                live.change.id,
                live.lease_generation,
                &failure,
                1_001,
                policy,
            )
            .expect("persist fail-closed retry"),
        LibraryChangeLeaseUpdateOutcome::Applied,
    );
    let evidence: (String, String, i64, i64, i64, String, Option<String>) = catalog
        .connection
        .query_row(
            "SELECT queue.status, queue.last_failure_code,
                    (SELECT COUNT(*) FROM library_change_queue
                     WHERE origin = 'metadata_inventory'),
                    (SELECT COUNT(*) FROM library_recovery_authorities),
                    (SELECT COUNT(*) FROM library_persistent_journal_baselines),
                    root.continuity_state, root.last_failure_code
             FROM library_change_queue AS queue
             JOIN library_persistent_journal_root_state AS root
               ON root.root_id = queue.root_id
              AND root.root_generation = queue.root_generation
             WHERE queue.id = ?1",
            [sqlite_integer(live.change.id.value(), "change ID").expect("change ID")],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                ))
            },
        )
        .expect("fail-closed promotion evidence");
    assert_eq!(evidence.0, "retry_wait");
    assert_eq!(evidence.1, "metadata_inventory_required");
    assert_eq!(evidence.2, 0);
    assert_eq!(evidence.3, 0);
    assert_eq!(evidence.4, 0);
    assert_eq!(evidence.5, "recovery_required");
    assert_eq!(evidence.6, None);
    assert_eq!(
        catalog
            .promote_live_watcher_gap_to_metadata_inventory(
                live.change.id,
                live.lease_generation,
                &failure,
                1_002,
                policy,
            )
            .expect("idempotent stale promotion"),
        LibraryChangeLeaseUpdateOutcome::LeaseMismatch,
    );
}

#[test]
fn live_only_watcher_gap_creates_scoped_recovery_authority() {
    let directory = tempdir().expect("temporary directory");
    let generation = LibraryRootGeneration::initial();
    let policy = immediate_policy();
    let mut catalog = queue_catalog(directory.path().join("catalog.sqlite3"));
    catalog
        .save_persistent_journal_capability(&PersistentJournalCapability {
            root_id: "root-a".to_owned(),
            root_generation: generation,
            protocol_version: 5,
            contract_version: 1,
            state: PersistentJournalCapabilityState::LiveOnly,
            continuity: PersistentJournalContinuityState::LiveOnly,
            failure: None,
            updated_unix_ms: 900,
        })
        .expect("live-only journal root");
    catalog
        .enqueue_library_change_intents(
            &[intent(
                "root-a",
                generation,
                1,
                1_000,
                LibraryChangeIntentKind::Reconcile,
                LibraryChangeScope::Subtree,
                "album",
            )],
            1_000,
            policy,
        )
        .expect("enqueue bounded watcher work");
    let live = catalog
        .lease_authoritative_library_change("root-a", generation, 1_000, policy)
        .expect("lease bounded watcher work")
        .expect("bounded watcher lease");

    assert_eq!(
        catalog
            .promote_live_watcher_gap_to_metadata_inventory(
                live.change.id,
                live.lease_generation,
                &LibraryChangeFailure {
                    code: "metadata_inventory_required".to_owned(),
                    message: "The bounded watcher scope could not be reconstructed".to_owned(),
                },
                1_001,
                policy,
            )
            .expect("persist live-only retry"),
        LibraryChangeLeaseUpdateOutcome::Applied,
    );
    let evidence: (String, i64, i64, String) = catalog
        .connection
        .query_row(
            "SELECT queue.status,
                    (SELECT COUNT(*) FROM library_change_queue
                     WHERE origin = 'metadata_inventory'),
                    (SELECT COUNT(*) FROM library_recovery_authorities),
                    root.continuity_state
             FROM library_change_queue AS queue
             JOIN library_persistent_journal_root_state AS root
               ON root.root_id = queue.root_id
              AND root.root_generation = queue.root_generation
             WHERE queue.id = ?1",
            [sqlite_integer(live.change.id.value(), "change ID").expect("change ID")],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .expect("live-only fail-closed evidence");
    assert_eq!(evidence.0, "superseded");
    assert_eq!(evidence.1, 1);
    assert_eq!(evidence.2, 1);
    assert_eq!(evidence.3, "live_only");
}
