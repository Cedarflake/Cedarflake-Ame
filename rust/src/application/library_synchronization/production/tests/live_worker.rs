use super::*;

pub(in super::super) fn enqueue_deletions(
    fixture: &ProductionGapFixture,
    storage: &crate::application::storage::StoragePaths,
    paths: &[&str],
) {
    let policy = crate::domain::LibraryChangeQueuePolicy::default();
    let observed_unix_ms = now_unix_ms().expect("fixture clock") - 1_000;
    let intents = paths
        .iter()
        .enumerate()
        .map(|(index, path)| {
            std::fs::remove_file(fixture.source_root.join(path))
                .expect("remove only generated media");
            let sequence = u64::try_from(index + 1).expect("fixture sequence");
            crate::domain::LibraryChangeIntent {
                root_id: fixture.root_id.clone(),
                root_generation: LibraryRootGeneration::initial(),
                kind: crate::domain::LibraryChangeIntentKind::Reconcile,
                scope: crate::domain::LibraryChangeScope::Subtree,
                relative_path: (*path).to_owned(),
                previous_relative_path: None,
                origin: crate::domain::LibraryChangeOrigin::LiveNotification,
                first_observed_unix_ms: observed_unix_ms,
                most_recent_observed_unix_ms: observed_unix_ms,
                first_sequence: sequence,
                most_recent_sequence: sequence,
                coalesced_observation_count: 1,
            }
        })
        .collect::<Vec<_>>();
    let mut catalog = SqliteCatalog::open(storage.catalog_path.clone()).expect("enqueue catalog");
    catalog
        .enqueue_library_change_intents(&intents, observed_unix_ms, policy)
        .expect("enqueue independent, already-ready deletion scopes");
}

fn live_runtime() -> ProductionSynchronization {
    new_production_synchronization_with_connection(
        crate::ports::erase_library_change_source_factory(HealthyFactory),
        test_live_only_connection(),
    )
}

fn finish_live_work(production: &mut ProductionSynchronization) -> u32 {
    let task = production.live.as_ref().expect("admitted Live worker");
    let deadline = Instant::now() + Duration::from_secs(5);
    while task.progress().1 != Some(true) && Instant::now() < deadline {
        thread::sleep(Duration::from_millis(1));
    }
    let worker_finished = task.progress().1 == Some(true);
    if !worker_finished {
        production.stop().expect("retire timed-out fixture");
        panic!("the bounded Live worker must finish");
    }
    production.poll_live(now_unix_ms().expect("completion clock"))
}

#[test]
fn presentation_poll_admits_a_bounded_ready_deletion_worker() {
    let fixture = ProductionGapFixture::new_with_media_baseline("live-deletion-cadence");
    let paths = ["removed.png", "unchanged.png"];
    enqueue_deletions(&fixture, &fixture.storage, &paths);
    let mut production = live_runtime();
    poll_runtime_with_storage(&mut production, &fixture.storage)
        .expect("the only presentation poll admits ready Live work");
    let mutations = finish_live_work(&mut production);
    production.stop().expect("retire every owned worker");

    let catalog = SqliteCatalog::open(fixture.storage.catalog_path.clone())
        .expect("independent catalog inspection after retirement");
    let retained_paths = paths
        .iter()
        .filter(|path| {
            catalog
                .load_incremental_location_by_relative_path(&fixture.root_id, path)
                .expect("inspect exact active location")
                .is_some()
        })
        .copied()
        .collect::<Vec<_>>();
    fixture.assert_no_automatic_full_scan();
    assert!(
        (1..=2).contains(&mutations),
        "one real quantum publishes bounded work"
    );
    assert_eq!(retained_paths.len(), 2 - mutations as usize);
}

#[test]
fn cancellation_after_lease_returns_work_without_deleting_or_acquiring_the_next_scope() {
    let fixture = ProductionGapFixture::new_with_media_baseline("live-cancelled-lease");
    enqueue_deletions(
        &fixture,
        &fixture.storage,
        &["removed.png", "unchanged.png"],
    );
    let pause = install_live_worker_after_lease_pause(&fixture.root_id);
    let mut production = live_runtime();
    poll_runtime_with_storage(&mut production, &fixture.storage).expect("admit first lease");
    pause.wait_until_reached(Duration::from_secs(5));
    production.live.as_ref().expect("Live owner").request_stop();
    pause.release();
    assert_eq!(finish_live_work(&mut production), 0);
    production.stop().expect("retire all workers");

    let connection = rusqlite::Connection::open(&fixture.storage.catalog_path).expect("evidence");
    let state: (i64, i64, i64, i64) = connection
        .query_row(
            "SELECT COUNT(*), SUM(attempt_count), SUM(lease_generation),
            (SELECT COUNT(*) FROM asset_locations WHERE root_id = ?1)
         FROM library_change_queue WHERE root_id = ?1 AND status = 'pending'",
            [&fixture.root_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .expect("returned lease and untouched catalog");
    assert_eq!(state, (2, 0, 1, 2));
}

#[test]
fn a_replaced_root_generation_cannot_publish_or_acquire_more_work_from_the_old_worker() {
    let fixture = ProductionGapFixture::new_with_media_baseline("live-replaced-root");
    enqueue_deletions(
        &fixture,
        &fixture.storage,
        &["removed.png", "unchanged.png"],
    );
    let pause = install_live_worker_after_lease_pause(&fixture.root_id);
    let mut production = live_runtime();
    poll_runtime_with_storage(&mut production, &fixture.storage).expect("admit old generation");
    pause.wait_until_reached(Duration::from_secs(5));
    let now = now_unix_ms().expect("replacement clock");
    let mut catalog =
        SqliteCatalog::open(fixture.storage.catalog_path.clone()).expect("replacement catalog");
    let replacement = catalog
        .enqueue_library_change_intents(
            &[crate::domain::LibraryChangeIntent {
                root_id: fixture.root_id.clone(),
                root_generation: LibraryRootGeneration::initial()
                    .next()
                    .expect("next generation"),
                kind: crate::domain::LibraryChangeIntentKind::Reconcile,
                scope: crate::domain::LibraryChangeScope::Subtree,
                relative_path: "new-generation.png".to_owned(),
                previous_relative_path: None,
                origin: crate::domain::LibraryChangeOrigin::LiveNotification,
                first_observed_unix_ms: now - 1_000,
                most_recent_observed_unix_ms: now - 1_000,
                first_sequence: 3,
                most_recent_sequence: 3,
                coalesced_observation_count: 1,
            }],
            now - 1_000,
            crate::domain::LibraryChangeQueuePolicy::default(),
        )
        .expect("retire previous root generation atomically");
    assert_eq!(replacement.superseded_count, 2);
    drop(catalog);
    pause.release();
    assert_eq!(finish_live_work(&mut production), 0);
    production.stop().expect("retire old worker");
    let connection = rusqlite::Connection::open(&fixture.storage.catalog_path).expect("evidence");
    let state: (String, i64, i64) = connection
        .query_row(
            "SELECT status, lease_generation,
            (SELECT COUNT(*) FROM asset_locations WHERE root_id = ?1)
         FROM library_change_queue WHERE root_id = ?1 AND root_generation = 2",
            [&fixture.root_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("new generation remains untouched");
    assert_eq!(state, ("pending".to_owned(), 0, 2));
}

#[test]
fn bounded_live_work_yields_to_the_next_ready_root() {
    let first = ProductionGapFixture::new_with_media_baseline("live-fair-first");
    let second = ProductionGapFixture::new_with_media_baseline("live-fair-second");
    let second_root_path =
        crate::adapters::FileDiscovery::new(&second.source_root.to_string_lossy())
            .expect("second root discovery")
            .canonical_root()
            .expect("second canonical root")
            .to_string_lossy()
            .into_owned();
    crate::application::scan_library::run_scan_with_storage(
        ScanRequest {
            scan_id: "second-root-shared-catalog".to_owned(),
            root_path: second_root_path,
            max_items: None,
            max_entries: None,
            preview_edge: 512,
        },
        |_| true,
        first.storage.clone(),
    )
    .expect("publish peer in the same catalog");
    enqueue_deletions(&first, &first.storage, &["removed.png", "unchanged.png"]);
    enqueue_deletions(&second, &first.storage, &["removed.png", "unchanged.png"]);
    let mut production = live_runtime();
    production.runtime.queue_policy.max_lease_batch = 1;
    poll_runtime_with_storage(&mut production, &first.storage).expect("first root admission");
    let first_admitted = production
        .live
        .as_ref()
        .expect("first owner")
        .root_id()
        .to_owned();
    assert_eq!(finish_live_work(&mut production), 1);
    poll_runtime_with_storage(&mut production, &first.storage).expect("second root admission");
    let second_admitted = production
        .live
        .as_ref()
        .expect("second owner")
        .root_id()
        .to_owned();
    assert_eq!(finish_live_work(&mut production), 1);
    production.stop().expect("retire shared runtime");
    assert_ne!(
        first_admitted, second_admitted,
        "ready peer must receive the next quantum"
    );
    let connection = rusqlite::Connection::open(&first.storage.catalog_path).expect("evidence");
    for root_id in [&first.root_id, &second.root_id] {
        let remaining: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM asset_locations WHERE root_id = ?1",
                [root_id],
                |row| row.get(0),
            )
            .expect("remaining peer membership");
        assert_eq!(
            remaining, 1,
            "each root receives exactly one bounded quantum"
        );
    }
}
