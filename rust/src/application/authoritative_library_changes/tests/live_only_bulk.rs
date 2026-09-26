use super::*;
use crate::domain::LibraryChangeLane;

#[test]
fn live_only_bulk_removal_crosses_old_catalog_page_limit() {
    recover_bulk_removal(false);
}

#[test]
fn live_only_bulk_removal_recovers_exhausted_debt_after_reopen() {
    recover_bulk_removal(true);
}

fn recover_bulk_removal(retained: bool) {
    let source = tempdir().expect("source");
    let storage = tempdir().expect("storage");
    let album = source.path().join("album");
    fs::create_dir(&album).expect("album");
    for index in 0..129 {
        write_png(&album.join(format!("{index:03}.png")), [10, 20, 30, 255]);
    }
    write_png(&source.path().join("sibling.png"), [40, 50, 60, 255]);
    let paths = fixture_storage(&storage);
    publish_initial_scan(
        &source,
        paths.clone(),
        if retained {
            "live-only-bulk-retained"
        } else {
            "live-only-bulk-fresh"
        },
    );
    let mut catalog = SqliteCatalog::open(paths.catalog_path.clone()).expect("catalog");
    let root = only_root(&catalog);
    let sibling_before = catalog
        .load_incremental_location_by_relative_path(&root.root_id, "sibling.png")
        .expect("sibling query")
        .expect("sibling");
    let kept_bytes = fs::read(album.join("128.png")).expect("retained bytes");
    for index in 0..128 {
        fs::remove_file(album.join(format!("{index:03}.png"))).expect("owned deletion");
    }
    enqueue_intent(
        &mut catalog,
        subtree_intent(&root, LibraryChangeIntentKind::Reconcile, "album", None),
        3_000,
    );
    let policy = immediate_queue_policy();
    if retained {
        let connection = Connection::open(&paths.catalog_path).expect("evidence");
        connection
            .execute(
                "UPDATE library_change_queue SET status='retry_wait', attempt_count=?1,
               next_retry_unix_ms=NULL, lease_expires_unix_ms=NULL,
               last_failure_code='metadata_inventory_required',
               last_failure_message='The authoritative scope exceeds one bounded metadata page'
             WHERE root_id=?2 AND relative_path='album' AND origin='live_notification'",
                rusqlite::params![i64::from(policy.max_attempts), root.root_id],
            )
            .expect("retained old-version debt");
        drop(catalog);
        catalog = SqliteCatalog::open(paths.catalog_path.clone()).expect("reopen retained debt");
        assert!(
            catalog
                .has_ready_live_authoritative_library_change(
                    &root.root_id,
                    root.root_generation,
                    3_010,
                    policy
                )
                .expect("retained readiness")
        );
        connection
            .execute_batch(
                "CREATE TRIGGER reject_retained_gap BEFORE INSERT ON library_recovery_authorities
             BEGIN SELECT RAISE(ABORT, 'injected retained transfer failure'); END;",
            )
            .expect("rollback injection");
        for state in [
            "capability_state='supported', continuity_state='current'",
            "capability_state='live_only', continuity_state='unavailable'",
        ] {
            connection
                .execute(
                    &format!(
                        "UPDATE library_persistent_journal_root_state SET {state}, last_failure_code=NULL, last_failure_message=NULL WHERE root_id=?1"
                    ),
                    [&root.root_id],
                )
                .expect("non-LiveOnly state");
            assert!(
                !catalog
                    .has_ready_live_authoritative_library_change(
                        &root.root_id,
                        root.root_generation,
                        3_010,
                        policy
                    )
                    .expect("non-LiveOnly refusal")
            );
        }
        connection.execute("UPDATE library_persistent_journal_root_state SET capability_state='live_only', continuity_state='live_only' WHERE root_id=?1", [&root.root_id]).expect("restore LiveOnly");
        connection
            .execute(
                "UPDATE library_change_root_state SET is_active=0 WHERE root_id=?1",
                [&root.root_id],
            )
            .expect("inactive root");
        assert!(
            !catalog
                .has_ready_live_authoritative_library_change(
                    &root.root_id,
                    root.root_generation,
                    3_010,
                    policy
                )
                .expect("inactive refusal")
        );
        connection
            .execute(
                "UPDATE library_change_root_state SET is_active=1 WHERE root_id=?1",
                [&root.root_id],
            )
            .expect("reactivate fixture");
        let capacity_policy = LibraryChangeQueuePolicy {
            max_unresolved_changes: 2,
            max_lease_batch: 1,
            ..policy
        };
        let mut journal = subtree_intent(&root, LibraryChangeIntentKind::Reconcile, "other", None);
        journal.scope = LibraryChangeScope::Path;
        journal.origin = LibraryChangeOrigin::StartupCatchUp;
        catalog
            .enqueue_library_change_intents(&[journal], 3_000, capacity_policy)
            .expect("fill lower lane");
        assert!(
            !catalog
                .has_ready_live_authoritative_library_change(
                    &root.root_id,
                    root.root_generation,
                    3_010,
                    capacity_policy
                )
                .expect("capacity preserves lower-lane opportunity")
        );
        let lower = catalog
            .lease_path_library_changes_in_lane(
                &root.root_id,
                root.root_generation,
                LibraryChangeLane::Journal,
                3_010,
                capacity_policy,
            )
            .expect("lower lane lease");
        assert_eq!(lower.len(), 1);
        catalog
            .complete_library_change(
                lower[0].change.id,
                lower[0].lease_generation,
                root.catalog_revision,
                3_010,
            )
            .expect("release capacity");
        assert!(
            catalog
                .has_ready_live_authoritative_library_change(
                    &root.root_id,
                    root.root_generation,
                    3_010,
                    capacity_policy
                )
                .expect("capacity releases retained gap")
        );
        assert!(
            catalog
                .lease_live_authoritative_library_change(
                    &root.root_id,
                    root.root_generation,
                    3_010,
                    policy
                )
                .is_err()
        );
        assert_eq!(connection.query_row(
            "SELECT COUNT(*) FROM library_change_queue WHERE root_id=?1 AND status='retry_wait'
             AND last_failure_code='metadata_inventory_required' AND attempt_count=?2",
            rusqlite::params![root.root_id, i64::from(policy.max_attempts)], |row| row.get::<_, u32>(0),
        ).expect("original debt preserved"), 1);
        connection
            .execute_batch("DROP TRIGGER reject_retained_gap")
            .expect("end injection");
        for condition in [
            "last_failure_code='unrelated_failure'",
            "scope='path'",
            "origin='consistency_audit'",
        ] {
            connection.execute(&format!("UPDATE library_change_queue SET {condition} WHERE root_id=?1 AND relative_path='album'"), [&root.root_id]).expect("negative shape");
            assert!(
                !catalog
                    .has_ready_live_authoritative_library_change(
                        &root.root_id,
                        root.root_generation,
                        3_010,
                        policy
                    )
                    .expect("negative readiness")
            );
            connection.execute("UPDATE library_change_queue SET last_failure_code='metadata_inventory_required', scope='subtree', origin='live_notification' WHERE root_id=?1 AND relative_path='album'", [&root.root_id]).expect("restore shape");
        }
        assert!(
            catalog
                .lease_live_authoritative_library_change(
                    &root.root_id,
                    root.root_generation,
                    3_010,
                    policy
                )
                .expect("retained transfer")
                .is_none()
        );
        assert!(
            !catalog
                .has_ready_live_authoritative_library_change(
                    &root.root_id,
                    root.root_generation,
                    3_010,
                    policy
                )
                .expect("transferred readiness")
        );
        let old_attempts: u32 = connection
            .query_row(
                "SELECT attempt_count FROM library_change_queue WHERE root_id=?1
             AND origin='live_notification' AND relative_path='album'",
                [&root.root_id],
                |row| row.get(0),
            )
            .expect("attempt evidence");
        assert_eq!(old_attempts, policy.max_attempts);
        drop(catalog);
        catalog =
            SqliteCatalog::open(paths.catalog_path.clone()).expect("reopen transferred authority");
    } else {
        let bounded = process_ready_authoritative_library_change(
            &mut catalog,
            &root.root_id,
            root.root_generation,
            3_000,
            policy,
            AuthoritativeRecoveryPolicy::default(),
        )
        .expect("bounded live reconciliation");
        assert_eq!(bounded.incremental.applied_mutation_count, 0);
        assert_eq!(bounded.incremental.retried_count, 1);
    }
    let mut retained_source = None;
    let mut complete = false;
    let cancellation = AtomicBool::new(false);
    for step in 0..24 {
        let now = 4_000 + step * 100;
        let Some(lease) = catalog
            .lease_authoritative_library_change(&root.root_id, root.root_generation, now, policy)
            .expect("recovery lease")
        else {
            continue;
        };
        assert!(leased_change_requires_metadata_inventory(&lease));
        let page = process_leased_metadata_inventory_change_with_retained_source(
            &mut catalog,
            &root,
            &lease,
            MetadataInventoryRecoveryExecution::without_progress(now, 64, policy, &cancellation),
            retained_source.take(),
        )
        .expect("scoped metadata recovery");
        retained_source = page.retained_source;
        crate::application::process_ready_library_changes(
            &mut catalog,
            &root.root_id,
            root.root_generation,
            now,
            policy,
        )
        .expect("candidate publication");
        if page.report.inventory.is_complete {
            complete = true;
            break;
        }
    }
    assert!(complete, "bounded LiveOnly inventory must complete");
    for index in 0..128 {
        assert!(
            catalog
                .load_incremental_location_by_relative_path(
                    &root.root_id,
                    &format!("album/{index:03}.png")
                )
                .expect("removed query")
                .is_none()
        );
    }
    assert!(
        catalog
            .load_incremental_location_by_relative_path(&root.root_id, "album/128.png")
            .expect("kept query")
            .is_some()
    );
    assert_eq!(
        catalog
            .load_incremental_location_by_relative_path(&root.root_id, "sibling.png")
            .expect("sibling query")
            .expect("sibling"),
        sibling_before
    );
    assert_eq!(
        fs::read(album.join("128.png")).expect("source bytes"),
        kept_bytes
    );
    let metrics = catalog
        .load_library_change_root_queue_metrics(&root.root_id, root.root_generation, 9_000, policy)
        .expect("settled metrics");
    assert_eq!(
        metrics.pending_count + metrics.leased_count + metrics.retry_wait_count,
        0
    );
    drop(catalog);
    let reopened = SqliteCatalog::open(paths.catalog_path).expect("FULL reopen after removal");
    assert!(
        !reopened
            .has_ready_live_authoritative_library_change(
                &root.root_id,
                root.root_generation,
                9_000,
                policy
            )
            .expect("no new work on reopen")
    );
}
