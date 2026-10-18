use super::*;

#[cfg(windows)]
#[test]
fn production_native_need_rescan_live_only_preserves_p0_and_converges_through_p2() {
    let fixture = ProductionGapFixture::new_with_media_baseline("native-need-rescan");
    let unchanged_path = fixture.source_root.join("unchanged.png");
    let removed_path = fixture.source_root.join("removed.png");
    let added_path = fixture.source_root.join("added.png");
    let unchanged_before = std::fs::read(&unchanged_path).expect("read unchanged baseline");
    let unchanged_hash_before = blake3::hash(&unchanged_before);
    std::fs::remove_file(&removed_path).expect("remove baseline media");
    image::RgbImage::from_pixel(2, 2, image::Rgb([40, 120, 200]))
        .save_with_format(&added_path, image::ImageFormat::Png)
        .expect("add changed media");
    let added_before = std::fs::read(&added_path).expect("read added source");
    let added_hash_before = blake3::hash(&added_before);
    let root_path = SqliteCatalog::open(fixture.storage.catalog_path.clone())
        .expect("open P2 instrumentation catalog")
        .load_incremental_catalog_root(&fixture.root_id)
        .expect("load P2 instrumentation root")
        .expect("P2 instrumentation root")
        .root_path;
    crate::adapters::reset_source_enumeration_instrumentation(&root_path);
    crate::adapters::reset_source_content_open_instrumentation(&root_path);
    let factory = QueuedSourceFactory::default();
    let mut production = fast_gap_runtime(factory.clone(), test_live_only_connection());
    poll_runtime_with_storage(&mut production, &fixture.storage)
        .expect("start native-gap production observer");
    let batch = crate::adapters::native_need_rescan_batch_for_test(&fixture.observer_request())
        .expect("map native notify need_rescan");
    assert_eq!(batch.observations.len(), 1);
    assert_eq!(
        batch.observations[0].kind,
        crate::domain::LibraryChangeObservationKind::EvidenceGap
    );
    assert_eq!(
        batch.observations[0].origin,
        crate::domain::LibraryChangeOrigin::LiveNotification
    );
    assert_eq!(
        batch.last_issue_code.as_deref(),
        Some("change_source_rescan_required")
    );
    push_gap_batch(&factory, &fixture.root_id, batch);

    let gap = wait_for_durable_gap(&mut production, &fixture);
    assert_p0_live_gap(&gap);
    let snapshot = drive_gap_to_synchronized(&mut production, &fixture);

    assert_eq!(
        snapshot.roots[0].freshness,
        crate::domain::CatalogFreshnessState::Synchronized
    );
    assert_p2_gap_consumer(&fixture, gap.change_id);
    let mut mutation_connection = rusqlite::Connection::open(&fixture.storage.catalog_path)
        .expect("open legacy-contract mutation evidence");
    let mutation = mutation_connection
        .transaction()
        .expect("begin legacy-contract mutation");
    assert_eq!(
        mutation
            .execute(
                "DELETE FROM asset_locations
                 WHERE root_id = ?1 AND relative_path = 'added.png'
                   AND scan_id = (
                     SELECT active_scan_id FROM library_roots WHERE id = ?1
                   )",
                [&fixture.root_id],
            )
            .expect("mutate visible location inside rollback boundary"),
        1,
    );
    let legacy_contract: (String, String, i64, i64, i64) = mutation
        .query_row(
            "SELECT gap.status, claim.consumer_kind,
                    (SELECT COUNT(*) FROM library_change_queue
                     WHERE status IN ('pending', 'leased', 'retry_wait')),
                    (SELECT COUNT(*) FROM scan_runs),
                    (SELECT COUNT(*)
                     FROM library_roots AS root
                     JOIN asset_locations AS location
                       ON location.scan_id = root.active_scan_id
                     WHERE root.id = ?2 AND location.relative_path = 'added.png')
             FROM library_live_gap_recovery_claims AS claim
             JOIN library_change_queue AS gap ON gap.id = claim.gap_change_id
             WHERE claim.gap_change_id = ?1",
            rusqlite::params![gap.change_id, fixture.root_id],
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
        .expect("load legacy-contract mutation evidence");
    assert_eq!(
        legacy_contract,
        (
            "superseded".to_owned(),
            "metadata_inventory_control".to_owned(),
            0,
            fixture.scan_rows_before,
            0,
        ),
        "the former empty-fixture contract stays green even when visible publication is removed"
    );
    mutation
        .rollback()
        .expect("rollback legacy-contract mutation");
    let catalog = SqliteCatalog::open(fixture.storage.catalog_path.clone())
        .expect("open P2 visibility catalog");
    let added = catalog
        .load_incremental_location_by_relative_path(&fixture.root_id, "added.png")
        .expect("load added P2 location")
        .expect("added P2 location must be visible");
    assert_eq!(added.relative_path, "added.png");
    assert_eq!(added.width, 2);
    assert_eq!(added.height, 2);
    assert_eq!(
        added.file_size,
        u64::try_from(added_before.len()).expect("added source size")
    );
    assert!(
        catalog
            .load_incremental_location_by_relative_path(&fixture.root_id, "removed.png")
            .expect("load removed P2 location")
            .is_none(),
        "P2 absence publication must remove the deleted location"
    );
    let unchanged = catalog
        .load_incremental_location_by_relative_path(&fixture.root_id, "unchanged.png")
        .expect("load unchanged P2 location")
        .expect("unchanged baseline media must remain visible");
    assert!(
        !crate::application::media_metadata_is_invalidated(&unchanged),
        "synchronized requires the invalidated baseline metadata to be rebuilt"
    );
    let queue = catalog
        .load_library_change_root_queue_metrics(
            &fixture.root_id,
            LibraryRootGeneration::initial(),
            now_unix_ms().expect("visibility metrics clock"),
            production.runtime.queue_policy(),
        )
        .expect("load terminal P2 visibility queue");
    assert_eq!(queue.pending_count, 0);
    assert_eq!(queue.leased_count, 0);
    assert_eq!(queue.retry_wait_count, 0);
    drop(catalog);
    let terminal: (i64, i64, i64, i64) = rusqlite::Connection::open(&fixture.storage.catalog_path)
        .expect("open terminal P2 visibility evidence")
        .query_row(
            "SELECT
                   (SELECT COUNT(*)
                    FROM library_roots AS root
                    JOIN asset_locations AS location
                      ON location.scan_id = root.active_scan_id
                    WHERE root.id = ?1 AND location.relative_path = 'added.png'),
                   (SELECT COUNT(*)
                    FROM library_live_gap_recovery_claims AS claim
                    WHERE claim.root_id = ?1 AND claim.consumed_unix_ms IS NULL),
                   (SELECT COUNT(*) FROM library_recovery_authorities AS authority
                    WHERE authority.root_id = ?1 AND authority.retired_unix_ms IS NULL),
                   (SELECT COUNT(*) FROM scan_runs)",
            [&fixture.root_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .expect("load terminal P2 visibility evidence");
    assert_eq!(terminal, (1, 0, 0, fixture.scan_rows_before));
    let unchanged_after = std::fs::read(&unchanged_path).expect("reread unchanged source");
    let added_after = std::fs::read(&added_path).expect("reread added source");
    assert_eq!(unchanged_after, unchanged_before);
    assert_eq!(blake3::hash(&unchanged_after), unchanged_hash_before);
    assert_eq!(added_after, added_before);
    assert_eq!(blake3::hash(&added_after), added_hash_before);
    assert!(!removed_path.exists());
    assert_eq!(crate::adapters::source_entry_read_count(&root_path), 2);
    assert_eq!(crate::adapters::source_spool_open_count(&root_path), 1);
    assert_eq!(
        crate::adapters::source_content_open_count(&root_path),
        2,
        "one added image read and one invalidated metadata rebuild"
    );
    fixture.assert_no_automatic_full_scan();
    production
        .stop()
        .expect("stop native-gap production runtime");
}

#[cfg(windows)]
#[test]
fn production_observer_ingress_drop_synthesizes_a_durable_consumed_p0_gap() {
    let fixture = ProductionGapFixture::new("observer-ingress-drop", 0);
    let factory = QueuedSourceFactory::default();
    let mut production = fast_gap_runtime(factory.clone(), test_live_only_connection());
    poll_runtime_with_storage(&mut production, &fixture.storage)
        .expect("start ingress-drop production observer");
    push_gap_batch(
        &factory,
        &fixture.root_id,
        crate::domain::LibraryChangeSourceBatch {
            observations: Vec::new(),
            health: crate::domain::LibraryChangeSourceHealth::Healthy,
            dropped_observation_count: 3,
            ignored_callback_count: 0,
            last_issue_code: Some("change_source_ingress_overflow".to_owned()),
        },
    );

    let gap = wait_for_durable_gap(&mut production, &fixture);
    assert_p0_live_gap(&gap);
    let snapshot = drive_gap_to_synchronized(&mut production, &fixture);

    assert_eq!(snapshot.roots[0].pending_change_count, 0);
    assert_eq!(snapshot.roots[0].retry_wait_count, 0);
    assert_p2_gap_consumer(&fixture, gap.change_id);
    fixture.assert_no_automatic_full_scan();
    production
        .stop()
        .expect("stop ingress-drop production runtime");
}

#[cfg(windows)]
#[test]
fn production_capacity_deferred_gap_wakes_after_one_p2_slot_and_converges() {
    let fixture = ProductionGapFixture::new("capacity-deferred-gap", 0);
    let factory = QueuedSourceFactory::default();
    let policy = crate::domain::LibraryChangeQueuePolicy {
        debounce_millis: 0,
        max_unresolved_changes: 16,
        max_lease_batch: 16,
        lease_duration_millis: crate::domain::LibraryChangeQueuePolicy::MAX_LEASE_DURATION_MILLIS,
        max_attempts: 4,
        retry_initial_delay_millis: 1,
        retry_maximum_delay_millis: 8,
        ..crate::domain::LibraryChangeQueuePolicy::default()
    };
    let p2_capacity =
        usize::try_from(policy.lane_capacity(crate::domain::LibraryChangeLane::Recovery))
            .expect("P2 capacity");
    assert_eq!(p2_capacity, 12);
    let generation = LibraryRootGeneration::initial();
    let base_unix_ms = now_unix_ms().expect("capacity fixture clock");
    let mut catalog = SqliteCatalog::open(fixture.storage.catalog_path.clone())
        .expect("open capacity fixture catalog");
    let fillers = (0..p2_capacity)
        .map(|index| crate::domain::LibraryChangeIntent {
            root_id: fixture.root_id.clone(),
            root_generation: generation,
            kind: crate::domain::LibraryChangeIntentKind::Reconcile,
            scope: crate::domain::LibraryChangeScope::Path,
            relative_path: format!("capacity-{index:02}.jpg"),
            previous_relative_path: None,
            origin: crate::domain::LibraryChangeOrigin::UserRefresh,
            first_observed_unix_ms: base_unix_ms,
            most_recent_observed_unix_ms: base_unix_ms,
            first_sequence: u64::try_from(index + 1).expect("capacity sequence"),
            most_recent_sequence: u64::try_from(index + 1).expect("capacity sequence"),
            coalesced_observation_count: 1,
        })
        .collect::<Vec<_>>();
    catalog
        .enqueue_library_change_intents(&fillers, base_unix_ms, policy)
        .expect("fill P2 lane");
    let mut filler_leases = catalog
        .lease_path_library_changes_in_lane(
            &fixture.root_id,
            generation,
            crate::domain::LibraryChangeLane::Recovery,
            base_unix_ms,
            policy,
        )
        .expect("lease P2 capacity fixtures");
    assert_eq!(filler_leases.len(), p2_capacity);
    drop(catalog);

    let mut production = fast_gap_runtime(factory.clone(), test_live_only_connection());
    production.runtime.queue_policy = policy;
    poll_runtime_with_storage(&mut production, &fixture.storage)
        .expect("start capacity-gap production observer");
    push_gap_batch(
        &factory,
        &fixture.root_id,
        crate::domain::LibraryChangeSourceBatch {
            observations: Vec::new(),
            health: crate::domain::LibraryChangeSourceHealth::Healthy,
            dropped_observation_count: 1,
            ignored_callback_count: 0,
            last_issue_code: Some("change_source_ingress_overflow".to_owned()),
        },
    );
    let gap = wait_for_durable_gap(&mut production, &fixture);
    let attempt_deadline = Instant::now() + Duration::from_secs(5);
    loop {
        poll_runtime_with_storage(&mut production, &fixture.storage)
            .expect("exercise capacity deferral budget");
        let evidence: (Option<String>, i64, i64, Option<i64>) =
            rusqlite::Connection::open(&fixture.storage.catalog_path)
                .expect("open capacity deferral evidence")
                .query_row(
                    "SELECT last_failure_code, attempt_count, lease_generation,
                            next_retry_unix_ms
                     FROM library_change_queue WHERE id = ?1",
                    [gap.change_id],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
                )
                .expect("load capacity deferral evidence");
        if evidence.0.as_deref() == Some("live_gap_p2_capacity_deferred")
            && evidence.1 == 0
            && evidence.2 >= i64::from(policy.max_attempts + 1)
            && evidence.3.is_some()
        {
            break;
        }
        assert!(
            Instant::now() < attempt_deadline,
            "capacity wait exhausted or stopped retrying: {evidence:?}"
        );
        thread::sleep(Duration::from_millis(2));
    }

    let released = filler_leases.pop().expect("one P2 slot owner");
    let released_unix_ms = now_unix_ms().expect("capacity release clock");
    let mut catalog = SqliteCatalog::open(fixture.storage.catalog_path.clone())
        .expect("open capacity release catalog");
    let root = catalog
        .load_incremental_catalog_root(&fixture.root_id)
        .expect("load capacity release root")
        .expect("capacity release root");
    assert_eq!(
        catalog
            .complete_library_change(
                released.change.id,
                released.lease_generation,
                root.catalog_revision,
                released_unix_ms,
            )
            .expect("release one P2 slot"),
        crate::domain::LibraryChangeLeaseUpdateOutcome::Applied,
    );
    drop(catalog);

    let promotion_deadline = Instant::now() + Duration::from_secs(5);
    loop {
        poll_runtime_with_storage(&mut production, &fixture.storage)
            .expect("wake capacity-deferred P0 gap");
        let ownership: i64 = rusqlite::Connection::open(&fixture.storage.catalog_path)
            .expect("open capacity ownership evidence")
            .query_row(
                "SELECT COUNT(*)
                     FROM library_live_gap_recovery_claims AS claim
                     JOIN library_recovery_authorities AS authority
                       ON authority.change_id = claim.recovery_change_id
                     WHERE claim.gap_change_id = ?1
                       AND claim.consumer_kind = 'metadata_inventory_control'
                       AND authority.reason = 'watcher_uncovered_gap'",
                [gap.change_id],
                |row| row.get(0),
            )
            .expect("load capacity ownership evidence");
        if ownership == 1 {
            break;
        }
        assert!(
            Instant::now() < promotion_deadline,
            "one released P2 slot did not wake and promote the P0 gap"
        );
        thread::sleep(Duration::from_millis(2));
    }

    let mut catalog = SqliteCatalog::open(fixture.storage.catalog_path.clone())
        .expect("open remaining capacity catalog");
    let root = catalog
        .load_incremental_catalog_root(&fixture.root_id)
        .expect("reload capacity root")
        .expect("capacity root");
    for filler in filler_leases {
        assert_eq!(
            catalog
                .complete_library_change(
                    filler.change.id,
                    filler.lease_generation,
                    root.catalog_revision,
                    released_unix_ms + 1,
                )
                .expect("release remaining P2 fixture"),
            crate::domain::LibraryChangeLeaseUpdateOutcome::Applied,
        );
    }
    drop(catalog);

    let snapshot = drive_gap_to_synchronized(&mut production, &fixture);
    assert_eq!(
        snapshot.roots[0].freshness,
        crate::domain::CatalogFreshnessState::Synchronized
    );
    assert_p2_gap_consumer(&fixture, gap.change_id);
    let active_authorities: i64 = rusqlite::Connection::open(&fixture.storage.catalog_path)
        .expect("open terminal capacity evidence")
        .query_row(
            "SELECT COUNT(*) FROM library_recovery_authorities
                 WHERE retired_unix_ms IS NULL",
            [],
            |row| row.get(0),
        )
        .expect("count terminal capacity authorities");
    assert_eq!(active_authorities, 0);
    fixture.assert_no_automatic_full_scan();
    production
        .stop()
        .expect("stop capacity-gap production runtime");
}

#[cfg(windows)]
#[test]
fn production_precise_p0_path_stays_visible_while_a_leased_gap_returns_to_capacity_wait() {
    let fixture = ProductionGapFixture::new("leased-capacity-precise-path", 0);
    let factory = QueuedSourceFactory::default();
    let policy = crate::domain::LibraryChangeQueuePolicy {
        debounce_millis: 0,
        max_unresolved_changes: 4,
        max_lease_batch: 4,
        lease_duration_millis: crate::domain::LibraryChangeQueuePolicy::MAX_LEASE_DURATION_MILLIS,
        max_attempts: 4,
        retry_initial_delay_millis: 5_000,
        retry_maximum_delay_millis: 5_000,
        ..crate::domain::LibraryChangeQueuePolicy::default()
    };
    let generation = LibraryRootGeneration::initial();
    let p2_capacity =
        usize::try_from(policy.lane_capacity(crate::domain::LibraryChangeLane::Recovery))
            .expect("P2 capacity");
    assert_eq!(p2_capacity, 2);
    let base_unix_ms = now_unix_ms().expect("leased capacity fixture clock");
    let mut catalog = SqliteCatalog::open(fixture.storage.catalog_path.clone())
        .expect("open leased capacity fixture catalog");
    let fillers = (0..p2_capacity)
        .map(|index| crate::domain::LibraryChangeIntent {
            root_id: fixture.root_id.clone(),
            root_generation: generation,
            kind: crate::domain::LibraryChangeIntentKind::Reconcile,
            scope: crate::domain::LibraryChangeScope::Path,
            relative_path: format!("capacity-{index:02}.jpg"),
            previous_relative_path: None,
            origin: crate::domain::LibraryChangeOrigin::UserRefresh,
            first_observed_unix_ms: base_unix_ms,
            most_recent_observed_unix_ms: base_unix_ms,
            first_sequence: u64::try_from(index + 1).expect("capacity sequence"),
            most_recent_sequence: u64::try_from(index + 1).expect("capacity sequence"),
            coalesced_observation_count: 1,
        })
        .collect::<Vec<_>>();
    catalog
        .enqueue_library_change_intents(&fillers, base_unix_ms, policy)
        .expect("fill P2 lane");
    let filler_leases = catalog
        .lease_path_library_changes_in_lane(
            &fixture.root_id,
            generation,
            crate::domain::LibraryChangeLane::Recovery,
            base_unix_ms,
            policy,
        )
        .expect("lease every P2 capacity fixture");
    assert_eq!(filler_leases.len(), p2_capacity);
    drop(catalog);

    let mut production = fast_gap_runtime(factory.clone(), test_live_only_connection());
    production.runtime.queue_policy = policy;
    poll_runtime_with_storage(&mut production, &fixture.storage)
        .expect("start leased-capacity production observer");
    push_gap_batch(
        &factory,
        &fixture.root_id,
        crate::domain::LibraryChangeSourceBatch {
            observations: Vec::new(),
            health: crate::domain::LibraryChangeSourceHealth::Healthy,
            dropped_observation_count: 1,
            ignored_callback_count: 0,
            last_issue_code: Some("change_source_ingress_overflow".to_owned()),
        },
    );
    let gap = wait_for_durable_gap(&mut production, &fixture);
    let initial_deferral_deadline = Instant::now() + Duration::from_secs(5);
    loop {
        poll_runtime_with_storage(&mut production, &fixture.storage)
            .expect("drive initial capacity deferral");
        let deferred: bool = rusqlite::Connection::open(&fixture.storage.catalog_path)
            .expect("open initial capacity evidence")
            .query_row(
                "SELECT status = 'retry_wait'
                            AND last_failure_code = 'live_gap_p2_capacity_deferred'
                            AND next_retry_unix_ms IS NOT NULL
                     FROM library_change_queue WHERE id = ?1",
                [gap.change_id],
                |row| row.get(0),
            )
            .expect("load initial capacity evidence");
        if deferred && production.live.is_none() {
            break;
        }
        assert!(
            Instant::now() < initial_deferral_deadline,
            "P0 gap did not enter typed capacity wait"
        );
        thread::sleep(Duration::from_millis(2));
    }

    let forced_due_unix_ms = now_unix_ms().expect("forced due clock");
    rusqlite::Connection::open(&fixture.storage.catalog_path)
        .expect("open exact race setup")
        .execute(
            "UPDATE library_change_queue
             SET next_retry_unix_ms = ?1, updated_unix_ms = ?1
             WHERE id = ?2 AND status = 'retry_wait'
               AND last_failure_code = 'live_gap_p2_capacity_deferred'",
            rusqlite::params![forced_due_unix_ms, gap.change_id],
        )
        .expect("make the typed capacity gap due once");
    let pause = install_live_worker_after_lease_pause(&fixture.root_id);
    poll_runtime_with_storage(&mut production, &fixture.storage)
        .expect("schedule exact leased capacity gap");
    pause.wait_until_reached(Duration::from_secs(3));

    let leased_gap: (String, i64, Option<i64>, String) =
        rusqlite::Connection::open(&fixture.storage.catalog_path)
            .expect("open leased capacity evidence")
            .query_row(
                "SELECT status, lease_generation, lease_expires_unix_ms,
                        last_failure_code
                 FROM library_change_queue WHERE id = ?1",
                [gap.change_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .expect("load leased capacity evidence");
    assert_eq!(leased_gap.0, "leased");
    assert!(leased_gap.1 >= 2);
    assert!(leased_gap.2.is_some());
    assert_eq!(leased_gap.3, "live_gap_p2_capacity_deferred");

    let relative_path = "visible-during-capacity.png";
    image::RgbImage::from_pixel(3, 2, image::Rgb([71, 149, 211]))
        .save(fixture.source_root.join(relative_path))
        .expect("write precise P0 media fixture");
    let observed_unix_ms = now_unix_ms().expect("precise P0 observation clock");
    push_gap_batch(
        &factory,
        &fixture.root_id,
        crate::domain::LibraryChangeSourceBatch {
            observations: vec![crate::domain::LibraryChangeObservation {
                root_id: fixture.root_id.clone(),
                root_generation: generation,
                sequence: 2,
                observed_unix_ms,
                kind: crate::domain::LibraryChangeObservationKind::Created,
                scope: crate::domain::LibraryChangeScope::Path,
                relative_path: relative_path.to_owned(),
                previous_relative_path: None,
                origin: crate::domain::LibraryChangeOrigin::LiveNotification,
            }],
            health: crate::domain::LibraryChangeSourceHealth::Healthy,
            dropped_observation_count: 0,
            ignored_callback_count: 0,
            last_issue_code: None,
        },
    );
    poll_runtime_with_storage(&mut production, &fixture.storage)
        .expect("admit precise P0 path while gap worker is leased");
    let concurrent_shape: (String, i64, Option<i64>, i64, String, String) =
        rusqlite::Connection::open(&fixture.storage.catalog_path)
            .expect("open concurrent leased-gap evidence")
            .query_row(
                "SELECT gap.status, gap.lease_generation, gap.lease_expires_unix_ms,
                        COUNT(path.id), MIN(path.scope), MIN(path.intent_kind)
                 FROM library_change_queue AS gap
                 LEFT JOIN library_change_queue AS path
                   ON path.root_id = gap.root_id
                  AND path.root_generation = gap.root_generation
                  AND path.relative_path = ?2
                  AND path.status IN ('pending', 'leased', 'retry_wait', 'completed')
                 WHERE gap.id = ?1",
                rusqlite::params![gap.change_id, relative_path],
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
            .expect("load concurrent leased-gap evidence");
    assert_eq!(concurrent_shape.0, "leased");
    assert_eq!(concurrent_shape.1, leased_gap.1);
    assert_eq!(concurrent_shape.2, leased_gap.2);
    assert_eq!(concurrent_shape.3, 1);
    assert_eq!(concurrent_shape.4, "path");
    assert_eq!(concurrent_shape.5, "reconcile");

    pause.release();
    let visible_deadline = Instant::now() + Duration::from_secs(4);
    let terminal_gap = loop {
        poll_runtime_with_storage(&mut production, &fixture.storage)
            .expect("drive precise P0 publication beside capacity wait");
        let catalog = SqliteCatalog::open(fixture.storage.catalog_path.clone())
            .expect("open precise P0 visibility catalog");
        let is_visible = catalog
            .load_incremental_location_by_relative_path(&fixture.root_id, relative_path)
            .expect("load precise P0 location")
            .is_some();
        drop(catalog);
        let gap_state: (String, Option<String>, Option<i64>, i64) =
            rusqlite::Connection::open(&fixture.storage.catalog_path)
                .expect("open deferred gap state")
                .query_row(
                    "SELECT gap.status, gap.last_failure_code, gap.next_retry_unix_ms,
                            (SELECT COUNT(*)
                             FROM library_live_gap_recovery_claims AS claim
                             WHERE claim.gap_change_id = gap.id)
                     FROM library_change_queue AS gap WHERE gap.id = ?1",
                    [gap.change_id],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
                )
                .expect("load deferred gap state");
        if is_visible {
            break gap_state;
        }
        assert!(
            Instant::now() < visible_deadline,
            "precise P0 path was not visible before the gap retry deadline"
        );
        thread::sleep(Duration::from_millis(5));
    };
    assert_eq!(terminal_gap.0, "retry_wait");
    assert_eq!(
        terminal_gap.1.as_deref(),
        Some("live_gap_p2_capacity_deferred")
    );
    assert!(terminal_gap.2.is_some());
    assert_eq!(terminal_gap.3, 0);
    let precise_status: String = rusqlite::Connection::open(&fixture.storage.catalog_path)
        .expect("open precise completion evidence")
        .query_row(
            "SELECT status FROM library_change_queue
                 WHERE root_id = ?1 AND relative_path = ?2",
            rusqlite::params![fixture.root_id, relative_path],
            |row| row.get(0),
        )
        .expect("load precise completion evidence");
    assert_eq!(precise_status, "completed");
    fixture.assert_no_automatic_full_scan();
    drop(filler_leases);
    drop(pause);
    production
        .stop()
        .expect("stop leased-capacity production runtime");
}

#[cfg(windows)]
#[test]
fn production_restart_recovers_an_expired_live_gap_lease_and_retains_its_consumer_lineage() {
    let fixture = ProductionGapFixture::new("restart-expired-gap", 0);
    let factory = QueuedSourceFactory::default();
    let mut first = fast_gap_runtime(factory.clone(), test_live_only_connection());
    poll_runtime_with_storage(&mut first, &fixture.storage)
        .expect("start pre-crash production observer");
    first.is_stopping = true;
    push_gap_batch(
        &factory,
        &fixture.root_id,
        crate::domain::LibraryChangeSourceBatch {
            observations: Vec::new(),
            health: crate::domain::LibraryChangeSourceHealth::Healthy,
            dropped_observation_count: 1,
            ignored_callback_count: 0,
            last_issue_code: Some("change_source_ingress_overflow".to_owned()),
        },
    );
    let gap = wait_for_durable_gap(&mut first, &fixture);
    assert_p0_live_gap(&gap);
    let connection = rusqlite::Connection::open(&fixture.storage.catalog_path)
        .expect("open crash-boundary catalog");
    let updated = connection
        .execute(
            "UPDATE library_change_queue
             SET status = 'leased', attempt_count = attempt_count + 1,
                 lease_generation = lease_generation + 1,
                 lease_expires_unix_ms = 0, next_retry_unix_ms = NULL
             WHERE id = ?1 AND status = 'pending'",
            [gap.change_id],
        )
        .expect("persist simulated crashed live lease");
    assert_eq!(updated, 1);
    drop(connection);
    drop(first);

    let mut restarted = fast_gap_runtime(factory, test_live_only_connection());
    let snapshot = drive_gap_to_synchronized(&mut restarted, &fixture);

    assert_eq!(snapshot.roots[0].pending_change_count, 0);
    assert_eq!(snapshot.roots[0].retry_wait_count, 0);
    assert_p2_gap_consumer(&fixture, gap.change_id);
    fixture.assert_no_automatic_full_scan();
    restarted.stop().expect("stop restarted production runtime");
}

#[cfg(windows)]
#[test]
fn production_continuous_journal_range_owns_and_consumes_live_gap_before_supersession() {
    let fixture = ProductionGapFixture::new("journal-covered-gap", 0);
    seed_current_gap_journal(&fixture);
    let factory = QueuedSourceFactory::default();
    let query_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let shared_read_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let close_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let session: Arc<dyn PersistentChangeJournalSession> = Arc::new(AdvancingJournalSession {
        boundaries: Mutex::new(std::collections::VecDeque::from([20, 21, 21])),
        query_count: Arc::clone(&query_count),
        shared_read_count: Arc::clone(&shared_read_count),
        read_release: Arc::new(AtomicBool::new(true)),
        close_count: Arc::clone(&close_count),
    });
    let mut production = fast_gap_runtime(
        factory.clone(),
        PersistentChangeJournalConnection::Connected(session),
    );
    production.persistent_change_journal_caller = Some(crate::journal_broker::CallerClaim {
        process_id: std::process::id(),
        session_id: 1,
        client_instance: [17; 16],
    });
    let initial_deadline = Instant::now() + Duration::from_secs(3);
    loop {
        poll_runtime_with_storage(&mut production, &fixture.storage)
            .expect("establish current production journal");
        if query_count.load(Ordering::Acquire) >= 1 && production.journal.is_none() {
            break;
        }
        assert!(
            Instant::now() < initial_deadline,
            "initial current journal query did not settle"
        );
        thread::sleep(Duration::from_millis(2));
    }
    push_gap_batch(
        &factory,
        &fixture.root_id,
        crate::domain::LibraryChangeSourceBatch {
            observations: Vec::new(),
            health: crate::domain::LibraryChangeSourceHealth::Healthy,
            dropped_observation_count: 1,
            ignored_callback_count: 0,
            last_issue_code: Some("change_source_ingress_overflow".to_owned()),
        },
    );

    let gap = wait_for_durable_gap(&mut production, &fixture);
    assert_p0_live_gap(&gap);
    let snapshot = drive_gap_to_synchronized(&mut production, &fixture);

    assert_eq!(
        snapshot.roots[0].freshness,
        crate::domain::CatalogFreshnessState::Synchronized
    );
    assert_journal_gap_consumer(&fixture, gap.change_id);
    assert!(shared_read_count.load(Ordering::Acquire) >= 1);
    fixture.assert_no_automatic_full_scan();
    production
        .stop()
        .expect("stop journal-gap production runtime");
    assert_eq!(close_count.load(Ordering::Acquire), 1);
}
