use std::sync::mpsc;
use std::time::Instant;

use super::*;
use crate::ports::erase_library_change_source_factory;

#[test]
fn observer_poll_defers_while_recovery_holds_the_catalog_writer() {
    let fixture = RuntimeFixture::new();
    let factory = FakeFactory::default();
    let mut runtime = LibrarySynchronizationRuntime::new_production(
        erase_library_change_source_factory(factory.clone()),
    );
    let mut writer = SqliteCatalog::open(fixture.catalog.catalog_path().to_path_buf())
        .expect("open the competing catalog before holding its transaction");
    let mut catalog = fixture.catalog;
    runtime
        .poll_without_authoritative_recovery(&mut catalog, 900, |_| {
            LibraryRootAvailability::Available
        })
        .expect("establish the production observer");
    enqueue_live_path_observation(&factory, &fixture.root_id, "new.png", 1);

    let (held_tx, held_rx) = mpsc::sync_channel(1);
    let (release_tx, release_rx) = mpsc::sync_channel(1);
    let writer_thread = thread::spawn(move || {
        writer.with_recovery_write_for_test(|| {
            held_tx.send(()).expect("report the held recovery writer");
            release_rx.recv_timeout(Duration::from_secs(5))
        })
    });
    let held = held_rx.recv_timeout(Duration::from_secs(5));
    let (registered_tx, registered_rx) = mpsc::sync_channel(1);
    let (finished_tx, finished_rx) = mpsc::sync_channel(1);
    let poll_thread = thread::spawn(move || {
        SqliteCatalog::after_next_write_registration_for_test(move || {
            registered_tx
                .send(())
                .expect("observe actual queue admission");
        });
        let snapshot = runtime.poll_without_authoritative_recovery(&mut catalog, 1_000, |_| {
            LibraryRootAvailability::Available
        });
        finished_tx.send(()).expect("report observer completion");
        (runtime, catalog, snapshot)
    });
    let registered = registered_rx.recv_timeout(Duration::from_secs(5));
    let returned_while_held = finished_rx.recv_timeout(Duration::from_millis(100));
    let released = release_tx.send(());
    let writer_result = writer_thread.join();
    let poll_result = poll_thread.join();
    let (mut runtime, catalog, snapshot) = poll_result.expect("retire the observer poll");
    let stopped = runtime.stop();
    held.expect("recovery acquired the actual catalog transaction");
    registered.expect("observer reached actual writer admission");
    released.expect("release the owned recovery transaction");
    writer_result
        .expect("retire the recovery writer")
        .expect("release before the writer deadline");
    stopped.expect("retire the observer source");
    assert!(
        returned_while_held.is_ok(),
        "observer poll waited for the recovery writer to release"
    );
    let snapshot = snapshot.expect("contention is a per-root deferred handoff");
    assert_eq!(snapshot.roots[0].freshness, CatalogFreshnessState::Updating);
    assert_eq!(snapshot.roots[0].pending_change_count, 0);
    assert_eq!(
        catalog
            .load_library_change_root_queue_metrics(
                &fixture.root_id,
                LibraryRootGeneration::initial(),
                1_000,
                LibraryChangeQueuePolicy::default(),
            )
            .expect("read the unchanged durable queue")
            .pending_count,
        0,
    );
}

#[test]
fn external_sqlite_contention_retains_observations_and_does_not_claim_synchronized() {
    let fixture = RuntimeFixture::new();
    let factory = FakeFactory::default();
    let mut runtime = Runtime::new_production(erase_library_change_source_factory(factory.clone()));
    let mut catalog = fixture.catalog;
    poll_observer(&mut runtime, &mut catalog, 900);
    write_png(&fixture.source_root.join("new.png"), 7, 5, [20, 30, 40]);
    write_png(&fixture.source_root.join("later.png"), 8, 6, [50, 60, 70]);
    enqueue_live_path_observation(&factory, &fixture.root_id, "new.png", 1);
    enqueue_live_path_observation(&factory, &fixture.root_id, "later.png", 2);
    let external = rusqlite::Connection::open(catalog.catalog_path()).expect("external writer");
    external
        .execute_batch("BEGIN IMMEDIATE")
        .expect("hold an external SQLite write lock");
    let started = Instant::now();
    let deferred = poll_observer(&mut runtime, &mut catalog, 1_000);
    let elapsed = started.elapsed();
    let again = poll_observer(&mut runtime, &mut catalog, 1_001);
    external
        .execute_batch("ROLLBACK")
        .expect("release external writer");
    assert!(
        elapsed < Duration::from_millis(200),
        "observer waited on SQLite: {elapsed:?}"
    );
    for snapshot in [deferred, again] {
        assert_eq!(snapshot.roots[0].freshness, CatalogFreshnessState::Updating);
        assert_eq!(snapshot.roots[0].pending_change_count, 0);
        assert_eq!(
            snapshot.roots[0].last_issue_code.as_deref(),
            Some("catalog_database_busy")
        );
    }
    assert_eq!(
        factory.state.lock().expect("source batches").batches.len(),
        1,
        "do not drain over uncommitted observations"
    );
    assert!(!runtime.root_is_ready_for_authoritative_recovery(&fixture.root_id));
    let published = poll_observer(&mut runtime, &mut catalog, 1_100);
    assert_eq!(published.roots[0].pending_change_count, 2);
    let synchronized = runtime
        .poll(&mut catalog, 2_000, |_| LibraryRootAvailability::Available)
        .expect("publish both retained path changes");
    assert_eq!(synchronized.applied_mutation_count, 2);
    assert_eq!(
        synchronized.roots[0].freshness,
        CatalogFreshnessState::Synchronized
    );
    for path in ["new.png", "later.png"] {
        assert!(
            catalog
                .load_incremental_location_by_relative_path(&fixture.root_id, path)
                .expect("published path")
                .is_some()
        );
    }
    runtime.stop().expect("stop after convergence");
    drop(catalog);
    SqliteCatalog::open(external.path().expect("catalog path").into())
        .expect("FULL reopen after ingress and publication");
}

#[test]
fn continuity_gap_cannot_overwrite_a_deferred_path_plan() {
    let fixture = RuntimeFixture::new();
    let factory = FakeFactory::default();
    let mut runtime = Runtime::new_production(erase_library_change_source_factory(factory.clone()));
    let mut catalog = fixture.catalog;
    poll_observer(&mut runtime, &mut catalog, 900);
    runtime
        .roots
        .get_mut(&fixture.root_id)
        .expect("root")
        .needs_continuity_gap = true;
    enqueue_live_path_observation(&factory, &fixture.root_id, "precise.png", 1);
    let external = rusqlite::Connection::open(catalog.catalog_path()).expect("external writer");
    external
        .execute_batch("BEGIN IMMEDIATE")
        .expect("hold writer");
    let deferred = poll_observer(&mut runtime, &mut catalog, 1_000);
    external.execute_batch("ROLLBACK").expect("release writer");
    assert_eq!(deferred.roots[0].freshness, CatalogFreshnessState::Updating);
    assert_eq!(
        runtime.root_continuity_revision(&fixture.root_id),
        Some(0),
        "gap revision cannot overtake pending precise evidence"
    );
    assert!(
        runtime
            .roots
            .get(&fixture.root_id)
            .expect("root")
            .needs_continuity_gap
    );
    poll_observer(&mut runtime, &mut catalog, 1_100);
    let precise: i64 = external
        .query_row(
            "SELECT count(*) FROM library_change_queue WHERE relative_path = 'precise.png'",
            [],
            |row| row.get(0),
        )
        .expect("retained precise path was durably submitted");
    assert_eq!(precise, 1);
    assert_eq!(runtime.root_continuity_revision(&fixture.root_id), Some(1));
    runtime.stop().expect("stop gap fixture");
    drop(catalog);
    SqliteCatalog::open(external.path().expect("catalog path").into())
        .expect("FULL reopen preserves gap and path lineage");
}

#[test]
fn stopping_deferred_ingress_releases_writer_position_without_claiming_publication() {
    let fixture = RuntimeFixture::new();
    let factory = FakeFactory::default();
    let mut runtime = Runtime::new_production(erase_library_change_source_factory(factory.clone()));
    let mut catalog = fixture.catalog;
    poll_observer(&mut runtime, &mut catalog, 900);
    enqueue_live_path_observation(&factory, &fixture.root_id, "pending.png", 1);
    let external = rusqlite::Connection::open(catalog.catalog_path()).expect("external writer");
    external
        .execute_batch("BEGIN IMMEDIATE")
        .expect("hold writer");
    poll_observer(&mut runtime, &mut catalog, 1_000);
    let before = runtime
        .roots
        .get(&fixture.root_id)
        .expect("root")
        .handoff
        .is_waiting_for_writer();
    runtime
        .request_stop()
        .expect("stop admission does not wait for the writer");
    let root = runtime.roots.get(&fixture.root_id).expect("retained root");
    let released = !root.handoff.is_waiting_for_writer();
    let unpublished = root.handoff.has_pending();
    external.execute_batch("ROLLBACK").expect("release writer");
    assert!(before && released && unpublished);
    catalog.with_recovery_write_for_test(|| ());
    runtime
        .finish_stop_until(Instant::now() + Duration::from_secs(2))
        .expect("owned observer retirement");
}

fn poll_observer(
    runtime: &mut Runtime,
    catalog: &mut SqliteCatalog,
    now: i64,
) -> LibrarySynchronizationSnapshot {
    runtime
        .poll_without_authoritative_recovery(catalog, now, |_| LibraryRootAvailability::Available)
        .expect("observer-only production path")
}

#[test]
fn removing_a_root_retires_its_unpublished_ingress_reservation() {
    let fixture = RuntimeFixture::new();
    let factory = FakeFactory::default();
    let mut runtime = Runtime::new_production(erase_library_change_source_factory(factory.clone()));
    let mut catalog = fixture.catalog;
    poll_observer(&mut runtime, &mut catalog, 900);
    enqueue_live_path_observation(&factory, &fixture.root_id, "pending.png", 1);
    let external = rusqlite::Connection::open(catalog.catalog_path()).expect("external writer");
    external
        .execute_batch("BEGIN IMMEDIATE")
        .expect("hold writer");
    poll_observer(&mut runtime, &mut catalog, 1_000);
    external.execute_batch("ROLLBACK").expect("release writer");
    assert!(runtime.has_reserved_ingress());
    assert!(
        catalog
            .unregister_root(&fixture.root_id)
            .expect("interactive removal outranks pending ingress")
    );
    let snapshot = poll_observer(&mut runtime, &mut catalog, 1_100);
    assert!(snapshot.roots.is_empty());
    assert!(!runtime.has_reserved_ingress());
    catalog.with_recovery_write_for_test(|| ());
    runtime.stop().expect("retire removed observer");
    drop(catalog);
    SqliteCatalog::open(external.path().expect("catalog path").into())
        .expect("FULL reopen after removing uncommitted root");
}
