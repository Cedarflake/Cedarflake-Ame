use super::super as journal;
use super::*;

#[test]
fn terminal_range_lifecycle_is_bounded_and_survives_reopen() {
    let directory = tempdir().expect("catalog directory");
    let path = directory.path().join("catalog.sqlite3");
    let mut catalog = catalog_with_roots(path.clone(), &["root-a"]);
    support_root(&mut catalog, "root-a");
    let mut start = 10_i64;
    for index in 0..70 {
        let publication_unix_ms = 10_000 + i64::from(index) * 2;
        let end = start + 1;
        let relative_path = format!("page-{index}.jpg");
        let mut volume_batch = atomic_volume_batch(vec![batch(
            &format!("range-{index}"),
            "root-a",
            start,
            end,
            &[&relative_path],
        )]);
        volume_batch.pages[0].checkpoint.updated_unix_ms = publication_unix_ms;
        catalog
            .publish_persistent_journal_volume_batch(&volume_batch, 1_000, policy())
            .expect("publish sequential journal page");
        let transaction = catalog
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .expect("terminal lifecycle transaction");
        transaction
            .execute(
                "UPDATE library_change_queue
                 SET status = 'completed', catalog_revision_at_success = 0,
                     updated_unix_ms = ?1
                 WHERE status = 'pending'",
                [publication_unix_ms + 1],
            )
            .expect("terminalize page queue");
        journal::finalize_persistent_journal_ranges(&transaction, publication_unix_ms + 1)
            .expect("finalize journal page");
        transaction.commit().expect("commit terminal lifecycle");
        start = end;
    }
    let retained: (i64, i64, i64, String) = catalog
        .connection
        .query_row(
            "SELECT
               (SELECT COUNT(*) FROM library_persistent_journal_source_ranges),
               (SELECT COUNT(*) FROM library_persistent_journal_range_lifecycle),
               (SELECT COUNT(*) FROM library_persistent_journal_range_lifecycle
                WHERE lifecycle_state = 'pending'),
               (SELECT continuity_state FROM library_persistent_journal_checkpoints
                WHERE root_id = 'root-a')",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .expect("bounded lifecycle state");
    assert_eq!(retained.0, 64);
    assert_eq!(retained.0, retained.1);
    assert_eq!(retained.2, 0);
    assert_eq!(retained.3, "current");
    drop(catalog);
    let reopened = SqliteCatalog::open(path).expect("reopen bounded lifecycle");
    assert_eq!(
        reopened
            .load_persistent_journal_checkpoint("root-a", LibraryRootGeneration::initial())
            .expect("load reopened checkpoint")
            .expect("checkpoint")
            .next_unread_usn
            .value(),
        80
    );
}

#[test]
fn historical_generation_cleanup_retains_pending_and_latest_terminal_proof() {
    let directory = tempdir().expect("catalog directory");
    let path = directory.path().join("catalog.sqlite3");
    let mut catalog = catalog_with_roots(path.clone(), &["root-a"]);
    support_root(&mut catalog, "root-a");
    let mut terminal_ids = Vec::new();
    for (index, start) in [10_i64, 20].into_iter().enumerate() {
        let mut volume_batch = atomic_volume_batch(vec![batch(
            &format!("terminal-{index}"),
            "root-a",
            start,
            start + 10,
            &[&format!("terminal-{index}.jpg")],
        )]);
        let checkpoint_unix_ms = 2_000 + 200 * i64::try_from(index).expect("bounded index");
        let terminal_unix_ms = checkpoint_unix_ms + 100;
        volume_batch.pages[0].checkpoint.updated_unix_ms = checkpoint_unix_ms;
        terminal_ids.push(volume_batch.pages[0].enrollment.range.batch_id.clone());
        catalog
            .publish_persistent_journal_volume_batch(&volume_batch, 1_000, policy())
            .expect("publish historical terminal page");
        let transaction = catalog
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .expect("terminal lifecycle transaction");
        transaction
            .execute(
                "UPDATE library_change_queue
                 SET status = 'completed', catalog_revision_at_success = 0,
                     updated_unix_ms = ?1
                 WHERE status = 'pending'",
                [terminal_unix_ms],
            )
            .expect("terminalize historical queue");
        journal::finalize_persistent_journal_ranges(&transaction, terminal_unix_ms)
            .expect("finalize historical page");
        transaction
            .commit()
            .expect("commit historical terminal page");
    }

    let mut pending = batch("pending-old", "root-a", 30, 40, &["pending.jpg"]);
    let mut carry = PersistentJournalPendingRename {
        carry_id: String::new(),
        source_range_id: pending.range.batch_id.clone(),
        volume: volume(),
        journal_id: JournalIdentifier::new(9).expect("journal ID"),
        file_reference: JournalFileReference::V3([4; 16]),
        old_usn: JournalUsn::new(35).expect("OLD USN"),
        previous_root_id: "root-a".to_owned(),
        previous_root_generation: LibraryRootGeneration::initial(),
        previous_relative_path: "pending.jpg".to_owned(),
        is_directory: false,
        enrolled_unix_ms: 1_000,
    };
    carry.carry_id = persistent_journal_pending_rename_id(&carry);
    pending.pending_renames.push(carry);
    let mut pending_volume = atomic_volume_batch(vec![pending]);
    pending_volume.pages[0].checkpoint.updated_unix_ms = 2_400;
    let pending_id = pending_volume.pages[0].enrollment.range.batch_id.clone();
    catalog
        .publish_persistent_journal_volume_batch(&pending_volume, 1_000, policy())
        .expect("publish unresolved historical page");

    let next_generation = LibraryRootGeneration::initial()
        .next()
        .expect("next root generation");
    catalog
        .enqueue_library_change_intents(
            &[LibraryChangeIntent {
                root_id: "root-a".to_owned(),
                root_generation: next_generation,
                kind: LibraryChangeIntentKind::Reconcile,
                scope: LibraryChangeScope::Path,
                relative_path: "replacement.jpg".to_owned(),
                previous_relative_path: None,
                origin: LibraryChangeOrigin::LiveNotification,
                first_observed_unix_ms: 3_000,
                most_recent_observed_unix_ms: 3_000,
                first_sequence: 1,
                most_recent_sequence: 1,
                coalesced_observation_count: 1,
            }],
            3_000,
            policy(),
        )
        .expect("replace root generation");
    let transaction = catalog
        .connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .expect("historical cleanup transaction");
    journal::cleanup_completed_persistent_journal_history(&transaction, 3_100, 32)
        .expect("clean historical journal evidence");
    transaction.commit().expect("commit historical cleanup");

    let retained: (i64, i64, i64, i64, i64) = catalog
        .connection
        .query_row(
            "SELECT
               EXISTS(SELECT 1 FROM library_persistent_journal_source_ranges WHERE id = ?1),
               EXISTS(SELECT 1 FROM library_persistent_journal_source_ranges WHERE id = ?2),
               EXISTS(SELECT 1 FROM library_persistent_journal_source_ranges WHERE id = ?3),
               (SELECT COUNT(*) FROM library_persistent_journal_pending_renames),
               (SELECT COUNT(*) FROM library_change_root_state
                WHERE root_id = 'root-a' AND generation = 2 AND is_active = 1)",
            params![terminal_ids[0], terminal_ids[1], pending_id],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            },
        )
        .expect("historical cleanup boundary");
    assert_eq!(retained, (0, 1, 1, 1, 1));
    drop(catalog);
    SqliteCatalog::open(path).expect("reopen historical cleanup boundary");
}

#[test]
fn completed_lineage_cleanup_deletes_only_a_closed_range_proof_cluster() {
    let directory = tempdir().expect("catalog directory");
    let path = directory.path().join("catalog.sqlite3");
    let (mut catalog, previous, current) = catalog_with_pending_lineage_batches(path.clone());
    let old_range_ids = [
        previous.range.batch_id.clone(),
        current.range.batch_id.clone(),
    ];
    let transaction = catalog
        .connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .expect("terminal lineage transaction");
    transaction
        .execute(
            "UPDATE library_change_queue
             SET status = 'completed', catalog_revision_at_success = 0,
                 updated_unix_ms = 2_100
             WHERE status = 'pending'",
            [],
        )
        .expect("terminalize lineage queue");
    journal::finalize_persistent_journal_ranges(&transaction, 2_100)
        .expect("terminalize lineage ranges");
    transaction.commit().expect("commit terminal lineage");

    let mut newer = atomic_volume_batch(vec![
        batch("newer-a", "root-a", 20, 30, &[]),
        batch("newer-b", "root-b", 20, 30, &[]),
    ]);
    for page in &mut newer.pages {
        page.enrollment.range.enrolled_unix_ms = 2_200;
        page.checkpoint.updated_unix_ms = 2_200;
    }
    rebind_volume_batch_content(&mut newer);
    let newer_range_ids = newer
        .pages
        .iter()
        .map(|page| page.enrollment.range.batch_id.clone())
        .collect::<Vec<_>>();
    catalog
        .publish_persistent_journal_volume_batch(&newer, 2_200, policy())
        .expect("publish newer terminal ranges");
    let next_generation = LibraryRootGeneration::initial()
        .next()
        .expect("next root generation");
    for root_id in ["root-a", "root-b"] {
        catalog
            .enqueue_library_change_intents(
                &[LibraryChangeIntent {
                    root_id: root_id.to_owned(),
                    root_generation: next_generation,
                    kind: LibraryChangeIntentKind::Reconcile,
                    scope: LibraryChangeScope::Path,
                    relative_path: format!("replacement-{root_id}.jpg"),
                    previous_relative_path: None,
                    origin: LibraryChangeOrigin::LiveNotification,
                    first_observed_unix_ms: 2_300,
                    most_recent_observed_unix_ms: 2_300,
                    first_sequence: 1,
                    most_recent_sequence: 1,
                    coalesced_observation_count: 1,
                }],
                2_300,
                policy(),
            )
            .expect("retire journal root generation");
    }
    let transaction = catalog
        .connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .expect("closed cleanup transaction");
    journal::cleanup_completed_persistent_journal_history(&transaction, 2_400, 32)
        .expect("delete closed range proof cluster");
    crate::adapters::sqlite_catalog::migrations::journal_contract::validate_payload_children(
        &transaction,
    )
    .expect("cleanup leaves payload-child equivalence");
    transaction.commit().expect("commit closed cleanup");

    for range_id in old_range_ids {
        assert!(
            !catalog
                .connection
                .query_row(
                    "SELECT EXISTS(
                       SELECT 1 FROM library_persistent_journal_source_ranges WHERE id = ?1
                     )",
                    [&range_id],
                    |row| row.get::<_, bool>(0),
                )
                .expect("old range retention")
        );
    }
    for range_id in newer_range_ids {
        assert!(
            catalog
                .connection
                .query_row(
                    "SELECT EXISTS(
                       SELECT 1 FROM library_persistent_journal_source_ranges WHERE id = ?1
                     )",
                    [&range_id],
                    |row| row.get::<_, bool>(0),
                )
                .expect("newer range retention")
        );
    }
    assert_eq!(
        catalog
            .connection
            .query_row(
                "SELECT COUNT(*) FROM library_persistent_journal_cross_root_lineage",
                [],
                |row| row.get::<_, i64>(0),
            )
            .expect("cleaned lineage count"),
        0
    );
    drop(catalog);
    SqliteCatalog::open(path).expect("reopen after closed proof cleanup");
}

#[test]
fn terminalizer_refuses_to_publish_current_after_owner_loss() {
    let directory = tempdir().expect("catalog directory");
    let mut catalog = catalog_with_pending_lineage(directory.path().join("catalog.sqlite3"));
    catalog
        .connection
        .execute(
            "DELETE FROM library_persistent_journal_cross_root_ranges
             WHERE participant_role = 'previous'",
            [],
        )
        .expect("remove previous owner");
    let transaction = catalog
        .connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .expect("terminal transaction");
    let error = journal::finalize_persistent_journal_ranges(&transaction, 2_000)
        .expect_err("owner loss must block terminal publication");
    assert_eq!(
        error.code,
        "catalog_persistent_journal_contract_unverifiable"
    );
    transaction.rollback().expect("roll back terminal attempt");
    let states = catalog
        .connection
        .prepare(
            "SELECT continuity_state FROM library_persistent_journal_checkpoints
             ORDER BY root_id",
        )
        .expect("checkpoint state statement")
        .query_map([], |row| row.get::<_, String>(0))
        .expect("checkpoint states")
        .collect::<Result<Vec<_>, _>>()
        .expect("checkpoint state rows");
    assert_eq!(states, ["catching_up", "catching_up"]);

    let mut parent_loss =
        catalog_with_pending_lineage(directory.path().join("deleted-parent-terminal.sqlite3"));
    parent_loss
        .connection
        .execute(
            "DELETE FROM library_persistent_journal_cross_root_lineage",
            [],
        )
        .expect("delete lineage parent with owner cascade");
    let transaction = parent_loss
        .connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .expect("parent-loss terminal transaction");
    let error = journal::finalize_persistent_journal_ranges(&transaction, 2_000)
        .expect_err("missing payload child parent must block terminal publication");
    assert_eq!(
        error.code,
        "catalog_persistent_journal_contract_unverifiable"
    );
    transaction
        .rollback()
        .expect("roll back parent-loss terminal attempt");
    let states = parent_loss
        .connection
        .prepare(
            "SELECT continuity_state FROM library_persistent_journal_checkpoints
             ORDER BY root_id",
        )
        .expect("parent-loss checkpoint statement")
        .query_map([], |row| row.get::<_, String>(0))
        .expect("parent-loss checkpoint states")
        .collect::<Result<Vec<_>, _>>()
        .expect("parent-loss checkpoint rows");
    assert_eq!(states, ["catching_up", "catching_up"]);
}
