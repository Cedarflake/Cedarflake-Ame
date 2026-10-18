use super::super as journal;
use super::*;

#[test]
fn second_checkpoint_failure_rolls_back_all_owners_before_reopen_and_retry() {
    let directory = tempdir().expect("catalog directory");
    let path = directory.path().join("catalog.sqlite3");
    let mut catalog = catalog_with_roots(path.clone(), &["root-a", "root-b"]);
    support_root(&mut catalog, "root-a");
    support_root(&mut catalog, "root-b");
    let before = ["root-a", "root-b"].map(|root| {
        catalog
            .load_persistent_journal_checkpoint(root, LibraryRootGeneration::initial())
            .expect("initial checkpoint")
            .expect("checkpoint")
    });
    let volume_batch = atomic_volume_batch(vec![
        batch("range-a", "root-a", 10, 20, &["a.jpg"]),
        batch("range-b", "root-b", 10, 20, &["b.jpg"]),
    ]);
    catalog
        .connection
        .execute_batch(
            "CREATE TEMP TRIGGER fail_second_checkpoint
             BEFORE UPDATE ON library_persistent_journal_checkpoints
             WHEN NEW.root_id = 'root-b' AND NEW.next_unread_usn = '20'
             BEGIN SELECT RAISE(ABORT, 'second checkpoint failure'); END;",
        )
        .expect("inject second checkpoint failure");

    let error = catalog
        .publish_persistent_journal_volume_batch(&volume_batch, 1_000, policy())
        .expect_err("second checkpoint failure must roll back every owner");
    assert!(error.message.contains("second checkpoint failure"));
    drop(catalog);

    let mut reopened = SqliteCatalog::open(path).expect("reopen rolled back volume");
    for checkpoint in before {
        let actual = reopened
            .load_persistent_journal_checkpoint(&checkpoint.root_id, checkpoint.root_generation)
            .expect("reopened checkpoint");
        assert_eq!(actual, Some(checkpoint));
    }
    let counts: (i64, i64, i64) = reopened
        .connection
        .query_row(
            "SELECT
               (SELECT COUNT(*) FROM library_persistent_journal_source_ranges),
               (SELECT COUNT(*) FROM library_persistent_journal_range_lifecycle),
               (SELECT COUNT(*) FROM library_change_queue)",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("publication state after rollback");
    assert_eq!(counts, (0, 0, 0));
    let report = reopened
        .publish_persistent_journal_volume_batch(&volume_batch, 1_000, policy())
        .expect("retry full volume publication");
    assert_eq!(report.enrolled_root_count, 2);
    assert_eq!(report.advanced_checkpoint_count, 2);
}

#[test]
fn atomic_volume_publication_rolls_back_before_the_second_owner() {
    let directory = tempdir().expect("catalog directory");
    let mut catalog = catalog_with_roots(
        directory.path().join("catalog.sqlite3"),
        &["root-a", "root-b"],
    );
    support_root(&mut catalog, "root-a");
    support_root(&mut catalog, "root-b");
    let volume_batch = atomic_volume_batch(vec![
        batch("range-a", "root-a", 10, 20, &["a.jpg"]),
        batch("range-b", "root-b", 10, 20, &["b.jpg", "c.jpg"]),
    ]);

    let error =
        journal::publish_volume_batch(&mut catalog, &volume_batch, 1_000, policy(), Some(0))
            .expect_err("injected crash must roll back the whole volume");
    let state: (i64, i64, i64, String, String) = catalog
        .connection
        .query_row(
            "SELECT
               (SELECT COUNT(*) FROM library_persistent_journal_source_ranges),
               (SELECT COUNT(*) FROM library_persistent_journal_range_lifecycle),
               (SELECT COUNT(*) FROM library_change_queue),
               (SELECT next_unread_usn FROM library_persistent_journal_checkpoints
                WHERE root_id = 'root-a'),
               (SELECT next_unread_usn FROM library_persistent_journal_checkpoints
                WHERE root_id = 'root-b')",
            [],
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
        .expect("rolled back volume state");
    assert_eq!(error.code, "persistent_journal_test_crash");
    assert_eq!(state, (0, 0, 0, "10".to_owned(), "10".to_owned()));
}

#[test]
fn atomic_volume_capacity_failure_preserves_every_checkpoint() {
    let directory = tempdir().expect("catalog directory");
    let mut catalog = catalog_with_roots(
        directory.path().join("catalog.sqlite3"),
        &["root-a", "root-b"],
    );
    support_root(&mut catalog, "root-a");
    support_root(&mut catalog, "root-b");
    let volume_batch = atomic_volume_batch(vec![
        batch("range-a", "root-a", 10, 20, &["a.jpg"]),
        batch("range-b", "root-b", 10, 20, &["b.jpg", "c.jpg"]),
    ]);
    let constrained = LibraryChangeQueuePolicy {
        max_unresolved_changes: 1,
        ..policy()
    };

    let error = catalog
        .publish_persistent_journal_volume_batch(&volume_batch, 1_000, constrained)
        .expect_err("second owner capacity must roll back every owner");
    let state: (i64, i64, i64) = catalog
        .connection
        .query_row(
            "SELECT
               (SELECT COUNT(*) FROM library_persistent_journal_source_ranges),
               (SELECT COUNT(*) FROM library_change_queue),
               (SELECT COUNT(*) FROM library_persistent_journal_checkpoints
                WHERE next_unread_usn = '10')",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("capacity rollback state");
    assert_eq!(error.code, "persistent_journal_queue_capacity");
    assert_eq!(state, (0, 0, 2));
}

#[test]
fn atomic_volume_replay_binds_full_normalized_content() {
    let directory = tempdir().expect("catalog directory");
    let mut catalog = catalog_with_roots(directory.path().join("catalog.sqlite3"), &["root-a"]);
    support_root(&mut catalog, "root-a");
    let volume_batch = atomic_volume_batch(vec![batch("range-a", "root-a", 10, 20, &["a.jpg"])]);

    let first = catalog
        .publish_persistent_journal_volume_batch(&volume_batch, 1_000, policy())
        .expect("initial atomic publication");
    let replay = catalog
        .publish_persistent_journal_volume_batch(&volume_batch, 1_000, policy())
        .expect("same-content crash replay");
    let mut changed = volume_batch.clone();
    changed.pages[0].enrollment.intents[0].relative_path = "changed.jpg".to_owned();
    rebind_volume_batch_content(&mut changed);
    let error = catalog
        .publish_persistent_journal_volume_batch(&changed, 1_000, policy())
        .expect_err("changed normalized intent must conflict");
    let mut changed_observation = volume_batch.clone();
    changed_observation.pages[0].enrollment.intents[0].first_observed_unix_ms = 999;
    rebind_volume_batch_content(&mut changed_observation);
    let observation_error = catalog
        .publish_persistent_journal_volume_batch(&changed_observation, 1_000, policy())
        .expect_err("changed normalized intent timing must conflict");
    let checkpoint = catalog
        .load_persistent_journal_checkpoint("root-a", LibraryRootGeneration::initial())
        .expect("retained checkpoint")
        .expect("checkpoint");

    assert_eq!(first.advanced_checkpoint_count, 1);
    assert_eq!(replay.advanced_checkpoint_count, 0);
    assert_eq!(replay.observation_count, 0);
    assert_eq!(error.code, "persistent_journal_enrollment_invalid");
    assert_eq!(
        observation_error.code,
        "persistent_journal_enrollment_invalid"
    );
    assert_eq!(checkpoint.next_unread_usn.value(), 20);
    assert_eq!(
        checkpoint.continuity,
        PersistentJournalContinuityState::CatchingUp
    );
}

#[test]
fn atomic_volume_replay_rejects_added_or_changed_handoff_content() {
    let directory = tempdir().expect("catalog directory");
    let mut catalog = catalog_with_roots(
        directory.path().join("catalog.sqlite3"),
        &["root-a", "root-b"],
    );
    support_root(&mut catalog, "root-a");
    support_root(&mut catalog, "root-b");
    let mut previous = batch("range-a", "root-a", 10, 20, &["old.jpg"]);
    previous
        .cross_root_lineage
        .push(lineage(&previous.range.batch_id));
    let mut current = batch("range-b", "root-b", 10, 20, &["new.jpg"]);
    current
        .cross_root_lineage
        .push(lineage(&current.range.batch_id));
    rebind_enrollment_content(&mut previous);
    rebind_enrollment_content(&mut current);
    let volume_batch = atomic_volume_batch(vec![previous, current]);

    catalog
        .publish_persistent_journal_volume_batch(&volume_batch, 1_000, policy())
        .expect("publish normalized handoff batch");
    let replay = catalog
        .publish_persistent_journal_volume_batch(&volume_batch, 1_000, policy())
        .expect("same-content handoff crash replay");
    assert_eq!(replay.advanced_checkpoint_count, 0);

    let mut changed = volume_batch.clone();
    for page in &mut changed.pages {
        page.enrollment.cross_root_lineage[0].current_relative_path = "changed.jpg".to_owned();
    }
    rebind_volume_batch_content(&mut changed);
    let changed_error = catalog
        .publish_persistent_journal_volume_batch(&changed, 1_000, policy())
        .expect_err("changed handoff content must conflict");

    let mut added = volume_batch.clone();
    for page in &mut added.pages {
        let mut extra = page.enrollment.cross_root_lineage[0].clone();
        extra.lineage_id = "move-lineage-extra".to_owned();
        extra.file_reference = JournalFileReference::V3([3; 16]);
        extra.previous_relative_path = "old-extra.jpg".to_owned();
        extra.current_relative_path = "new-extra.jpg".to_owned();
        page.enrollment.cross_root_lineage.push(extra);
    }
    rebind_volume_batch_content(&mut added);
    let added_error = catalog
        .publish_persistent_journal_volume_batch(&added, 1_000, policy())
        .expect_err("added handoff content must conflict");

    let durable: (i64, i64, i64) = catalog
        .connection
        .query_row(
            "SELECT
               (SELECT COUNT(*) FROM library_persistent_journal_source_ranges),
               (SELECT COUNT(*) FROM library_persistent_journal_cross_root_lineage),
               (SELECT COUNT(*) FROM library_persistent_journal_checkpoints
                WHERE next_unread_usn = '20')",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("unchanged durable handoff replay state");
    assert_eq!(changed_error.code, "persistent_journal_enrollment_invalid");
    assert_eq!(added_error.code, "persistent_journal_enrollment_invalid");
    assert_eq!(durable, (2, 1, 2));
}

#[test]
fn atomic_cross_root_owners_publish_in_either_page_order() {
    for reverse in [false, true] {
        let directory = tempdir().expect("catalog directory");
        let mut catalog = catalog_with_roots(
            directory.path().join("catalog.sqlite3"),
            &["root-a", "root-b"],
        );
        support_root(&mut catalog, "root-a");
        support_root(&mut catalog, "root-b");
        let mut previous = batch("range-a", "root-a", 10, 20, &["old.jpg"]);
        previous
            .cross_root_lineage
            .push(lineage(&previous.range.batch_id));
        let mut current = batch("range-b", "root-b", 10, 20, &["new.jpg"]);
        current
            .cross_root_lineage
            .push(lineage(&current.range.batch_id));
        let pages = if reverse {
            vec![current, previous]
        } else {
            vec![previous, current]
        };
        let volume_batch = atomic_volume_batch(pages);

        let report = catalog
            .publish_persistent_journal_volume_batch(&volume_batch, 1_000, policy())
            .expect("atomic cross-root publication");
        let owners: Vec<String> = catalog
            .connection
            .prepare(
                "SELECT participant_role
                 FROM library_persistent_journal_cross_root_ranges
                 ORDER BY participant_role",
            )
            .expect("owner statement")
            .query_map([], |row| row.get(0))
            .expect("owner rows")
            .collect::<Result<_, _>>()
            .expect("owners");
        assert_eq!(report.advanced_checkpoint_count, 2);
        assert_eq!(owners, vec!["current".to_owned(), "previous".to_owned()]);
    }
}

#[test]
fn durable_pending_rename_reopens_and_consumes_with_atomic_rollback() {
    let directory = tempdir().expect("catalog directory");
    let path = directory.path().join("catalog.sqlite3");
    let mut catalog = catalog_with_roots(path.clone(), &["root-a", "root-b"]);
    support_root(&mut catalog, "root-a");
    support_root(&mut catalog, "root-b");

    let mut old = batch("range-old", "root-a", 10, 20, &["old.jpg"]);
    let mut carry = PersistentJournalPendingRename {
        carry_id: String::new(),
        source_range_id: old.range.batch_id.clone(),
        volume: volume(),
        journal_id: JournalIdentifier::new(9).expect("journal ID"),
        file_reference: JournalFileReference::V3([2; 16]),
        old_usn: JournalUsn::new(15).expect("OLD USN"),
        previous_root_id: "root-a".to_owned(),
        previous_root_generation: LibraryRootGeneration::initial(),
        previous_relative_path: "old.jpg".to_owned(),
        is_directory: false,
        enrolled_unix_ms: 1_000,
    };
    carry.carry_id = persistent_journal_pending_rename_id(&carry);
    old.pending_renames.push(carry);
    let old_volume = atomic_volume_batch(vec![old]);
    let durable_carry = old_volume.pages[0].enrollment.pending_renames[0].clone();
    catalog
        .publish_persistent_journal_volume_batch(&old_volume, 1_000, policy())
        .expect("publish durable OLD carry");
    drop(catalog);

    let mut catalog = SqliteCatalog::open(path.clone()).expect("reopen durable OLD carry");
    assert_eq!(
        catalog
            .load_persistent_journal_pending_renames(
                &volume(),
                JournalIdentifier::new(9).expect("journal ID"),
            )
            .expect("load reopened OLD carry"),
        std::slice::from_ref(&durable_carry)
    );
    let old_replay = catalog
        .publish_persistent_journal_volume_batch(&old_volume, 1_000, policy())
        .expect("replay durable OLD page");
    assert_eq!(old_replay.advanced_checkpoint_count, 0);

    let mut current = batch("range-new", "root-b", 10, 20, &["new.jpg", "sibling.jpg"]);
    let bind_carry = |mut lineage: PersistentJournalCrossRootLineage| {
        lineage.journal_id = durable_carry.journal_id;
        lineage.old_usn = Some(durable_carry.old_usn);
        lineage.new_usn = Some(JournalUsn::new(18).expect("NEW USN"));
        lineage.previous_carry_id = Some(durable_carry.carry_id.clone());
        lineage
    };
    current
        .cross_root_lineage
        .push(bind_carry(lineage(&current.range.batch_id)));
    current
        .carried_cross_root_lineage
        .push(bind_carry(lineage(&durable_carry.source_range_id)));
    current
        .consumed_pending_rename_ids
        .push(durable_carry.carry_id.clone());
    let current_volume = atomic_volume_batch(vec![current]);

    let crash =
        journal::publish_volume_batch(&mut catalog, &current_volume, 1_000, policy(), Some(0))
            .expect_err("crash before carry consumption must roll back");
    assert_eq!(crash.code, "persistent_journal_test_crash");
    assert_pending_carry_rollback(&catalog, &durable_carry);

    let constrained = LibraryChangeQueuePolicy {
        max_unresolved_changes: 1,
        ..policy()
    };
    let capacity = catalog
        .publish_persistent_journal_volume_batch(&current_volume, 1_000, constrained)
        .expect_err("capacity failure must retain OLD carry and checkpoint");
    assert_eq!(capacity.code, "persistent_journal_queue_capacity");
    assert_pending_carry_rollback(&catalog, &durable_carry);

    let mut assert_mismatch = |mut mismatched: PersistentJournalVolumeBatch, name: &str| {
        rebind_volume_batch_content(&mut mismatched);
        let mismatch = catalog
            .publish_persistent_journal_volume_batch(&mismatched, 1_000, policy())
            .expect_err(name);
        assert!(
            matches!(
                mismatch.code.as_str(),
                "persistent_journal_enrollment_invalid" | "persistent_journal_batch_conflict"
            ),
            "unexpected mismatch code for {name}: {}",
            mismatch.code
        );
        assert_pending_carry_rollback(&catalog, &durable_carry);
    };

    let mut missing_owner = current_volume.clone();
    missing_owner.pages[0]
        .enrollment
        .carried_cross_root_lineage
        .clear();
    assert_mismatch(missing_owner, "missing carried owner must roll back");

    let mut duplicate_owner = current_volume.clone();
    let duplicate = duplicate_owner.pages[0].enrollment.cross_root_lineage[0].clone();
    duplicate_owner.pages[0]
        .enrollment
        .cross_root_lineage
        .push(duplicate);
    assert_mismatch(duplicate_owner, "duplicate carried owner must roll back");

    let mut wrong_generation = current_volume.clone();
    let enrollment = &mut wrong_generation.pages[0].enrollment;
    for lineage in enrollment
        .cross_root_lineage
        .iter_mut()
        .chain(&mut enrollment.carried_cross_root_lineage)
    {
        lineage.previous_root_generation = LibraryRootGeneration::new(2).expect("wrong generation");
    }
    assert_mismatch(wrong_generation, "wrong carry generation must roll back");

    let mut wrong_path = current_volume.clone();
    let enrollment = &mut wrong_path.pages[0].enrollment;
    for lineage in enrollment
        .cross_root_lineage
        .iter_mut()
        .chain(&mut enrollment.carried_cross_root_lineage)
    {
        lineage.previous_relative_path = "wrong.jpg".to_owned();
    }
    assert_mismatch(wrong_path, "wrong carry path must roll back");

    let mut wrong_reference = current_volume.clone();
    let enrollment = &mut wrong_reference.pages[0].enrollment;
    for lineage in enrollment
        .cross_root_lineage
        .iter_mut()
        .chain(&mut enrollment.carried_cross_root_lineage)
    {
        lineage.file_reference = JournalFileReference::V3([8; 16]);
    }
    assert_mismatch(wrong_reference, "wrong carry reference must roll back");

    let mut wrong_range = current_volume.clone();
    wrong_range.pages[0].enrollment.carried_cross_root_lineage[0].owner_source_range_id =
        "0".repeat(64);
    assert_mismatch(wrong_range, "wrong carry source range must roll back");

    let report = catalog
        .publish_persistent_journal_volume_batch(&current_volume, 1_000, policy())
        .expect("atomically publish carried handoff");
    assert_eq!(report.advanced_checkpoint_count, 1);
    assert!(
        catalog
            .load_persistent_journal_pending_renames(
                &volume(),
                JournalIdentifier::new(9).expect("journal ID"),
            )
            .expect("load consumed carries")
            .is_empty()
    );
    let durable: (i64, i64, String, String) = catalog
        .connection
        .query_row(
            "SELECT
               (SELECT COUNT(*) FROM library_persistent_journal_cross_root_ranges),
               (SELECT COUNT(DISTINCT participant_role)
                FROM library_persistent_journal_cross_root_ranges),
               (SELECT next_unread_usn FROM library_persistent_journal_checkpoints
                WHERE root_id = 'root-a'),
               (SELECT next_unread_usn FROM library_persistent_journal_checkpoints
                WHERE root_id = 'root-b')",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .expect("durable carried handoff state");
    assert_eq!(durable, (2, 2, "20".to_owned(), "20".to_owned()));

    let current_replay = catalog
        .publish_persistent_journal_volume_batch(&current_volume, 1_000, policy())
        .expect("same-content carried handoff replay");
    let old_after_consume = catalog
        .publish_persistent_journal_volume_batch(&old_volume, 1_000, policy())
        .expect("same-content OLD replay after carry cleanup");
    assert_eq!(current_replay.advanced_checkpoint_count, 0);
    assert_eq!(old_after_consume.advanced_checkpoint_count, 0);
    drop(catalog);

    let consumed = SqliteCatalog::open(path.clone()).expect("reopen consumed carry proof");
    consumed
        .connection
        .execute(
            "DELETE FROM library_persistent_journal_cross_root_lineage",
            [],
        )
        .expect("delete consumed lineage proof with owner cascade");
    assert_eq!(
        consumed
            .connection
            .query_row(
                "SELECT COUNT(*) FROM library_persistent_journal_cross_root_ranges",
                [],
                |row| row.get::<_, i64>(0),
            )
            .expect("consumed proof owner count"),
        0
    );
    drop(consumed);
    assert_contract_reopen_fails(path, "deleted consumed carry proof");
}
