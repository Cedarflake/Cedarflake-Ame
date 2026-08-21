use std::collections::VecDeque;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use image::{Rgb, RgbImage};
use tempfile::tempdir;

use crate::adapters::SqliteCatalog;
use crate::application::AuthoritativeRecoveryPolicy;
use crate::domain::{
    CatalogFreshnessState, LibraryChangeIntent, LibraryChangeIntentKind, LibraryChangeObservation,
    LibraryChangeObservationKind, LibraryChangeOrigin, LibraryChangePlanningLimits,
    LibraryChangeQueuePolicy, LibraryChangeScope, LibraryChangeSourceBatch,
    LibraryChangeSourceError, LibraryChangeSourceHealth, LibraryChangeSourceStopReport,
    LibraryRootAvailability, LibraryRootGeneration, ScanRequest,
};
use crate::ports::{
    CatalogRepository, IncrementalCatalogRepository, LibraryChangeQueue, LibraryChangeSource,
    LibraryChangeSourceFactory, LibraryChangeSourceRequest,
};

use super::LibrarySynchronizationRuntime;

#[derive(Clone, Default)]
struct FakeFactory {
    state: Arc<Mutex<FakeState>>,
}

#[derive(Default)]
struct FakeState {
    batches: VecDeque<LibraryChangeSourceBatch>,
    start_count: u32,
    stop_count: u32,
}

struct FakeSource {
    state: Arc<Mutex<FakeState>>,
}

impl LibraryChangeSourceFactory for FakeFactory {
    type Source = FakeSource;

    fn start(
        &self,
        _request: &LibraryChangeSourceRequest,
    ) -> Result<Self::Source, LibraryChangeSourceError> {
        self.state.lock().expect("fake state").start_count += 1;
        Ok(FakeSource {
            state: Arc::clone(&self.state),
        })
    }
}

impl LibraryChangeSource for FakeSource {
    fn health(&self) -> LibraryChangeSourceHealth {
        LibraryChangeSourceHealth::Healthy
    }

    fn drain(
        &mut self,
        _max_observations: usize,
    ) -> Result<LibraryChangeSourceBatch, LibraryChangeSourceError> {
        Ok(self
            .state
            .lock()
            .expect("fake state")
            .batches
            .pop_front()
            .unwrap_or_else(empty_batch))
    }

    fn stop(&mut self) -> Result<LibraryChangeSourceStopReport, LibraryChangeSourceError> {
        self.state.lock().expect("fake state").stop_count += 1;
        Ok(LibraryChangeSourceStopReport::default())
    }
}

#[test]
fn cold_start_completes_a_bounded_authoritative_reconciliation_before_claiming_freshness() {
    let fixture = RuntimeFixture::new();
    let factory = FakeFactory::default();
    let mut runtime = runtime(factory.clone());
    let mut catalog = fixture.catalog;

    let snapshot = runtime
        .poll(&mut catalog, 1_000, |_| LibraryRootAvailability::Available)
        .expect("poll synchronization");

    assert_eq!(snapshot.roots.len(), 1);
    assert_eq!(
        snapshot.roots[0].freshness,
        CatalogFreshnessState::Synchronized
    );
    assert_eq!(snapshot.roots[0].freshness_unknown_count, 0);
    assert_eq!(factory.state.lock().expect("fake state").start_count, 1);
}

#[test]
fn live_observation_publishes_a_delta_and_advances_the_shared_revision() {
    let fixture = RuntimeFixture::new();
    let factory = FakeFactory::default();
    let mut runtime = runtime(factory.clone());
    let mut catalog = fixture.catalog;
    runtime
        .poll(&mut catalog, 900, |_| LibraryRootAvailability::Available)
        .expect("complete startup recovery");
    write_png(&fixture.source_root.join("new.png"), 7, 5, [20, 30, 40]);
    factory
        .state
        .lock()
        .expect("fake state")
        .batches
        .push_back(LibraryChangeSourceBatch {
            observations: vec![LibraryChangeObservation {
                root_id: fixture.root_id.clone(),
                root_generation: LibraryRootGeneration::initial(),
                sequence: 1,
                observed_unix_ms: 1_000,
                kind: LibraryChangeObservationKind::Created,
                scope: LibraryChangeScope::Path,
                relative_path: "new.png".to_owned(),
                previous_relative_path: None,
                origin: LibraryChangeOrigin::LiveNotification,
            }],
            health: LibraryChangeSourceHealth::Healthy,
            dropped_observation_count: 0,
            ignored_callback_count: 0,
            last_issue_code: None,
        });
    let snapshot = runtime
        .poll(&mut catalog, 1_000, |_| LibraryRootAvailability::Available)
        .expect("publish live change");

    assert_eq!(snapshot.applied_mutation_count, 1);
    assert_eq!(snapshot.catalog_revision, 2);
    assert_eq!(
        snapshot.roots[0].freshness,
        CatalogFreshnessState::Synchronized
    );
    assert!(
        catalog
            .load_incremental_location_by_relative_path(&fixture.root_id, "new.png")
            .expect("load new location")
            .is_some()
    );
}

#[cfg(windows)]
#[test]
fn new_cloud_placeholder_remains_unresolved_after_a_live_path_event() {
    let fixture = RuntimeFixture::new();
    let factory = FakeFactory::default();
    let mut runtime = runtime(factory.clone());
    let mut catalog = fixture.catalog;
    runtime
        .poll(&mut catalog, 900, |_| LibraryRootAvailability::Available)
        .expect("complete startup recovery");
    let placeholder = fixture.source_root.join("online-only.png");
    std::fs::write(&placeholder, b"must not be hydrated").expect("placeholder fixture");
    set_offline_attribute(&placeholder, true);
    enqueue_live_path_observation(&factory, &fixture.root_id, "online-only.png", 1);

    let snapshot = runtime
        .poll(&mut catalog, 1_000, |_| LibraryRootAvailability::Available)
        .expect("retain unresolved placeholder work");
    set_offline_attribute(&placeholder, false);

    assert_eq!(snapshot.applied_mutation_count, 0);
    assert_eq!(snapshot.roots[0].retry_wait_count, 1);
    assert_eq!(snapshot.roots[0].freshness, CatalogFreshnessState::Updating);
    assert!(
        catalog
            .load_incremental_location_by_relative_path(&fixture.root_id, "online-only.png")
            .expect("load placeholder location")
            .is_none()
    );
}

#[cfg(windows)]
#[test]
fn existing_cloud_placeholder_retains_catalog_evidence_and_remains_unresolved() {
    let fixture = RuntimeFixture::new();
    let factory = FakeFactory::default();
    let mut runtime = runtime(factory.clone());
    let mut catalog = fixture.catalog;
    runtime
        .poll(&mut catalog, 900, |_| LibraryRootAvailability::Available)
        .expect("complete startup recovery");
    let placeholder = fixture.source_root.join("retained.png");
    write_png(&placeholder, 4, 3, [30, 40, 50]);
    enqueue_live_path_observation(&factory, &fixture.root_id, "retained.png", 1);
    runtime
        .poll(&mut catalog, 1_000, |_| LibraryRootAvailability::Available)
        .expect("publish initial local file");
    let prior = catalog
        .load_incremental_location_by_relative_path(&fixture.root_id, "retained.png")
        .expect("load prior location")
        .expect("published prior location");
    set_offline_attribute(&placeholder, true);
    enqueue_live_path_observation(&factory, &fixture.root_id, "retained.png", 2);

    let snapshot = runtime
        .poll(&mut catalog, 1_100, |_| LibraryRootAvailability::Available)
        .expect("retain unresolved placeholder work");
    set_offline_attribute(&placeholder, false);
    let retained = catalog
        .load_incremental_location_by_relative_path(&fixture.root_id, "retained.png")
        .expect("load retained location")
        .expect("last trustworthy location");

    assert_eq!(snapshot.applied_mutation_count, 0);
    assert_eq!(snapshot.roots[0].retry_wait_count, 1);
    assert_eq!(snapshot.roots[0].freshness, CatalogFreshnessState::Updating);
    assert_eq!(retained.location_id, prior.location_id);
    assert_eq!(retained.asset_id, prior.asset_id);
}

#[test]
fn enqueue_failure_retains_the_drained_plan_until_persistence_recovers() {
    let fixture = RuntimeFixture::new();
    let factory = FakeFactory::default();
    let mut runtime = runtime(factory.clone());
    let mut catalog = fixture.catalog;
    runtime
        .poll(&mut catalog, 900, |_| LibraryRootAvailability::Available)
        .expect("complete startup recovery");
    write_png(&fixture.source_root.join("new.png"), 7, 5, [20, 30, 40]);
    write_png(&fixture.source_root.join("later.png"), 8, 6, [50, 60, 70]);
    let mut source_state = factory.state.lock().expect("fake state");
    source_state.batches.push_back(LibraryChangeSourceBatch {
        observations: vec![LibraryChangeObservation {
            root_id: fixture.root_id.clone(),
            root_generation: LibraryRootGeneration::initial(),
            sequence: 1,
            observed_unix_ms: 1_000,
            kind: LibraryChangeObservationKind::Created,
            scope: LibraryChangeScope::Path,
            relative_path: "new.png".to_owned(),
            previous_relative_path: None,
            origin: LibraryChangeOrigin::LiveNotification,
        }],
        health: LibraryChangeSourceHealth::Healthy,
        dropped_observation_count: 0,
        ignored_callback_count: 0,
        last_issue_code: None,
    });
    source_state.batches.push_back(LibraryChangeSourceBatch {
        observations: vec![LibraryChangeObservation {
            root_id: fixture.root_id.clone(),
            root_generation: LibraryRootGeneration::initial(),
            sequence: 2,
            observed_unix_ms: 1_050,
            kind: LibraryChangeObservationKind::Created,
            scope: LibraryChangeScope::Path,
            relative_path: "later.png".to_owned(),
            previous_relative_path: None,
            origin: LibraryChangeOrigin::LiveNotification,
        }],
        health: LibraryChangeSourceHealth::Healthy,
        dropped_observation_count: 0,
        ignored_callback_count: 0,
        last_issue_code: None,
    });
    drop(source_state);
    let injector = rusqlite::Connection::open(catalog.catalog_path()).expect("injector connection");
    injector
        .execute_batch(
            "CREATE TRIGGER fail_runtime_enqueue
             BEFORE INSERT ON library_change_queue
             BEGIN
               SELECT RAISE(ABORT, 'injected runtime enqueue failure');
             END;",
        )
        .expect("install enqueue failure");

    let failed_snapshot = runtime
        .poll(&mut catalog, 1_000, |_| LibraryRootAvailability::Available)
        .expect("one root enqueue failure must not abort the synchronization poll");
    assert_eq!(
        failed_snapshot.roots[0].freshness,
        CatalogFreshnessState::NeedsReconciliation
    );
    assert_eq!(
        failed_snapshot.roots[0].last_issue_code.as_deref(),
        Some("catalog_database_error")
    );
    assert!(
        catalog
            .load_incremental_location_by_relative_path(&fixture.root_id, "new.png")
            .expect("load absent location")
            .is_none()
    );
    let repeated_failure = runtime
        .poll(&mut catalog, 1_050, |_| LibraryRootAvailability::Available)
        .expect("retrying the retained plan must remain a per-root failure");
    assert_eq!(
        repeated_failure.roots[0].freshness,
        CatalogFreshnessState::NeedsReconciliation
    );
    injector
        .execute_batch("DROP TRIGGER fail_runtime_enqueue;")
        .expect("remove enqueue failure");

    let first_recovery = runtime
        .poll(&mut catalog, 1_100, |_| LibraryRootAvailability::Available)
        .expect("retry retained plan");
    let snapshot = runtime
        .poll(&mut catalog, 1_600, |_| LibraryRootAvailability::Available)
        .expect("publish the observation retained behind the failed plan");
    assert!(
        catalog
            .load_incremental_location_by_relative_path(&fixture.root_id, "new.png")
            .expect("load recovered location")
            .is_some()
    );
    assert!(
        catalog
            .load_incremental_location_by_relative_path(&fixture.root_id, "later.png")
            .expect("load later recovered location")
            .is_some()
    );
    assert_eq!(
        first_recovery
            .applied_mutation_count
            .saturating_add(snapshot.applied_mutation_count),
        2
    );
    assert_eq!(snapshot.catalog_revision, 2);
    assert_eq!(
        snapshot.roots[0].freshness,
        CatalogFreshnessState::Synchronized
    );
    assert!(snapshot.roots[0].last_issue_code.is_none());
}

#[test]
fn evidence_gap_runs_bounded_authoritative_reconciliation_before_clearing() {
    let fixture = RuntimeFixture::new();
    let factory = FakeFactory::default();
    factory
        .state
        .lock()
        .expect("fake state")
        .batches
        .push_back(LibraryChangeSourceBatch {
            observations: vec![LibraryChangeObservation {
                root_id: fixture.root_id.clone(),
                root_generation: LibraryRootGeneration::initial(),
                sequence: 1,
                observed_unix_ms: 1_000,
                kind: LibraryChangeObservationKind::EvidenceGap,
                scope: LibraryChangeScope::Root,
                relative_path: String::new(),
                previous_relative_path: None,
                origin: LibraryChangeOrigin::LiveNotification,
            }],
            health: LibraryChangeSourceHealth::Healthy,
            dropped_observation_count: 0,
            ignored_callback_count: 0,
            last_issue_code: Some("change_source_event_incomplete".to_owned()),
        });
    let mut runtime = runtime(factory);
    let mut catalog = fixture.catalog;

    let snapshot = runtime
        .poll(&mut catalog, 1_000, |_| LibraryRootAvailability::Available)
        .expect("retain evidence gap");

    assert_eq!(
        snapshot.roots[0].freshness,
        CatalogFreshnessState::Synchronized
    );
    assert_eq!(snapshot.roots[0].freshness_unknown_count, 0);
    assert_eq!(snapshot.applied_mutation_count, 0);
    assert!(snapshot.roots[0].last_issue_code.is_none());
}

#[test]
fn degraded_source_keeps_the_gap_until_the_restarted_observer_is_healthy() {
    let fixture = RuntimeFixture::new();
    let factory = FakeFactory::default();
    let mut runtime = runtime(factory.clone());
    let mut catalog = fixture.catalog;
    runtime
        .poll(&mut catalog, 900, |_| LibraryRootAvailability::Available)
        .expect("complete startup recovery");
    factory
        .state
        .lock()
        .expect("fake state")
        .batches
        .push_back(LibraryChangeSourceBatch {
            observations: vec![LibraryChangeObservation {
                root_id: fixture.root_id.clone(),
                root_generation: LibraryRootGeneration::initial(),
                sequence: 1,
                observed_unix_ms: 1_000,
                kind: LibraryChangeObservationKind::EvidenceGap,
                scope: LibraryChangeScope::Root,
                relative_path: String::new(),
                previous_relative_path: None,
                origin: LibraryChangeOrigin::LiveNotification,
            }],
            health: LibraryChangeSourceHealth::Degraded,
            dropped_observation_count: 1,
            ignored_callback_count: 0,
            last_issue_code: Some("change_source_rescan_required".to_owned()),
        });

    let degraded = runtime
        .poll(&mut catalog, 1_000, |_| LibraryRootAvailability::Available)
        .expect("retain degraded evidence gap");
    assert_eq!(
        degraded.roots[0].freshness,
        CatalogFreshnessState::NeedsReconciliation
    );
    assert_eq!(
        degraded.roots[0].source_health,
        LibraryChangeSourceHealth::Degraded
    );
    assert_eq!(
        degraded.roots[0].last_issue_code.as_deref(),
        Some("change_source_rescan_required")
    );
    write_png(
        &fixture.source_root.join("during-restart.png"),
        7,
        5,
        [20, 30, 40],
    );
    assert!(
        catalog
            .load_incremental_location_by_relative_path(&fixture.root_id, "during-restart.png",)
            .expect("load absent location")
            .is_none()
    );

    let mut recovered = None;
    for _ in 0..100 {
        let snapshot = runtime
            .poll(&mut catalog, 1_250, |_| LibraryRootAvailability::Available)
            .expect("advance observer restart");
        if snapshot.roots[0].freshness == CatalogFreshnessState::Synchronized {
            recovered = Some(snapshot);
            break;
        }
        assert_eq!(
            snapshot.roots[0].last_issue_code.as_deref(),
            Some("change_source_rescan_required")
        );
        thread::sleep(Duration::from_millis(5));
    }
    let recovered = recovered.expect("restarted observer eventually reconciles the retained gap");

    assert_eq!(
        recovered.roots[0].source_health,
        LibraryChangeSourceHealth::Healthy
    );
    assert!(recovered.roots[0].last_issue_code.is_none());
    assert_eq!(factory.state.lock().expect("fake state").start_count, 2);
    assert!(
        catalog
            .load_incremental_location_by_relative_path(&fixture.root_id, "during-restart.png",)
            .expect("load recovered location")
            .is_some()
    );
}

#[test]
fn production_poll_mode_projects_automatic_authoritative_work_as_updating() {
    let fixture = RuntimeFixture::new();
    let factory = FakeFactory::default();
    let mut runtime = runtime(factory);
    let mut catalog = fixture.catalog;
    write_png(
        &fixture.source_root.join("background.png"),
        7,
        5,
        [20, 30, 40],
    );

    let pending = runtime
        .poll_without_authoritative_recovery(&mut catalog, 1_000, |_| {
            LibraryRootAvailability::Available
        })
        .expect("production-mode poll");

    assert_eq!(pending.applied_mutation_count, 0);
    assert_eq!(pending.roots[0].freshness, CatalogFreshnessState::Updating);
    assert_eq!(pending.roots[0].freshness_unknown_count, 1);
    assert!(
        catalog
            .load_incremental_location_by_relative_path(&fixture.root_id, "background.png")
            .expect("load pending location")
            .is_none()
    );

    let completed = runtime
        .poll(&mut catalog, 1_100, |_| LibraryRootAvailability::Available)
        .expect("test-only inline recovery");
    assert_eq!(completed.applied_mutation_count, 1);
    assert_eq!(
        completed.roots[0].freshness,
        CatalogFreshnessState::Synchronized
    );
}

#[test]
fn production_starts_the_observer_before_persisting_startup_inventory_work() {
    let fixture = RuntimeFixture::new();
    let factory = FakeFactory::default();
    let mut runtime = LibrarySynchronizationRuntime::new_production(
        crate::ports::erase_library_change_source_factory(factory.clone()),
    );
    let mut catalog = fixture.catalog;

    let pending = runtime
        .poll_without_authoritative_recovery(&mut catalog, 1_000, |_| {
            LibraryRootAvailability::Available
        })
        .expect("watcher-first poll");

    assert_eq!(factory.state.lock().expect("fake state").start_count, 1);
    assert_eq!(pending.roots[0].freshness, CatalogFreshnessState::Updating);
    assert_eq!(pending.roots[0].freshness_unknown_count, 1);
    assert!(
        catalog
            .load_recoverable_scan()
            .expect("recoverable scan")
            .is_none()
    );
}

#[test]
fn recovery_writer_contention_blocks_only_after_the_bounded_grace_period() {
    let fixture = RuntimeFixture::new();
    let factory = FakeFactory::default();
    let mut runtime = LibrarySynchronizationRuntime::new_production(
        crate::ports::erase_library_change_source_factory(factory),
    );
    let mut catalog = fixture.catalog;
    runtime
        .poll_without_authoritative_recovery(&mut catalog, 1_000, |_| {
            LibraryRootAvailability::Available
        })
        .expect("establish runtime root");

    runtime.record_recovery_failure(&fixture.root_id, "catalog_database_busy", 1_000);
    let root = runtime.roots.get(&fixture.root_id).expect("runtime root");
    assert_eq!(root.recovery_contention_started_unix_ms, Some(1_000));
    assert!(root.blocking_issue_code.is_none());

    runtime.record_recovery_failure(&fixture.root_id, "catalog_database_busy", 30_999);
    assert!(
        runtime
            .roots
            .get(&fixture.root_id)
            .expect("runtime root")
            .blocking_issue_code
            .is_none()
    );

    runtime.record_recovery_failure(&fixture.root_id, "catalog_database_busy", 31_000);
    assert_eq!(
        runtime
            .roots
            .get(&fixture.root_id)
            .expect("runtime root")
            .blocking_issue_code
            .as_deref(),
        Some("catalog_database_busy")
    );
}

#[test]
fn non_contention_recovery_failure_blocks_immediately() {
    let fixture = RuntimeFixture::new();
    let factory = FakeFactory::default();
    let mut runtime = LibrarySynchronizationRuntime::new_production(
        crate::ports::erase_library_change_source_factory(factory),
    );
    let mut catalog = fixture.catalog;
    runtime
        .poll_without_authoritative_recovery(&mut catalog, 1_000, |_| {
            LibraryRootAvailability::Available
        })
        .expect("establish runtime root");

    runtime.record_recovery_failure(&fixture.root_id, "catalog_database_error", 1_000);

    assert_eq!(
        runtime
            .roots
            .get(&fixture.root_id)
            .expect("runtime root")
            .blocking_issue_code
            .as_deref(),
        Some("catalog_database_error")
    );
}

#[test]
fn production_live_root_gap_persists_metadata_inventory_work_without_starting_a_scan() {
    let fixture = RuntimeFixture::new();
    let factory = FakeFactory::default();
    let mut runtime = LibrarySynchronizationRuntime::new_production(
        crate::ports::erase_library_change_source_factory(factory.clone()),
    );
    let mut catalog = fixture.catalog;
    runtime
        .poll(&mut catalog, 900, |_| LibraryRootAvailability::Available)
        .expect("complete startup inventory");
    factory
        .state
        .lock()
        .expect("fake state")
        .batches
        .push_back(LibraryChangeSourceBatch {
            observations: vec![LibraryChangeObservation {
                root_id: fixture.root_id.clone(),
                root_generation: LibraryRootGeneration::initial(),
                sequence: 1,
                observed_unix_ms: 1_000,
                kind: LibraryChangeObservationKind::EvidenceGap,
                scope: LibraryChangeScope::Root,
                relative_path: String::new(),
                previous_relative_path: None,
                origin: LibraryChangeOrigin::LiveNotification,
            }],
            health: LibraryChangeSourceHealth::Healthy,
            dropped_observation_count: 1,
            ignored_callback_count: 0,
            last_issue_code: Some("change_source_event_incomplete".to_owned()),
        });

    let snapshot = runtime
        .poll_without_authoritative_recovery(&mut catalog, 1_000, |_| {
            LibraryRootAvailability::Available
        })
        .expect("persist live gap as metadata inventory work");

    assert_eq!(snapshot.roots[0].freshness, CatalogFreshnessState::Updating);
    assert_eq!(snapshot.roots[0].freshness_unknown_count, 1);
    let metrics = catalog
        .load_library_change_root_queue_metrics(
            &fixture.root_id,
            LibraryRootGeneration::initial(),
            1_000,
            LibraryChangeQueuePolicy::default(),
        )
        .expect("load queue metrics");
    assert_eq!(metrics.pending_count, 1);
    assert!(
        catalog
            .load_recoverable_scan()
            .expect("recoverable scan")
            .is_none()
    );
}

#[test]
fn active_authoritative_recovery_projects_updating_before_its_freshness_gap() {
    let fixture = RuntimeFixture::new();
    let factory = FakeFactory::default();
    let mut runtime = runtime(factory);
    let mut catalog = fixture.catalog;
    catalog
        .enqueue_library_change_intents(
            &[LibraryChangeIntent {
                root_id: fixture.root_id.clone(),
                root_generation: LibraryRootGeneration::initial(),
                kind: LibraryChangeIntentKind::FreshnessUnknown,
                scope: LibraryChangeScope::Root,
                relative_path: String::new(),
                previous_relative_path: None,
                origin: LibraryChangeOrigin::StartupCatchUp,
                first_observed_unix_ms: 1_000,
                most_recent_observed_unix_ms: 1_000,
                first_sequence: 1,
                most_recent_sequence: 1,
                coalesced_observation_count: 1,
            }],
            1_000,
            LibraryChangeQueuePolicy::default(),
        )
        .expect("enqueue freshness gap");
    let request = ScanRequest {
        scan_id: "active-authoritative-recovery".to_owned(),
        root_path: fixture.source_root.to_string_lossy().into_owned(),
        max_items: None,
        max_entries: None,
        preview_edge: 512,
    };
    catalog
        .begin_authoritative_scan(&request, &fixture.root_id, &request.root_path)
        .expect("begin authoritative recovery");

    let snapshot = runtime
        .poll_without_authoritative_recovery(&mut catalog, 1_100, |_| {
            LibraryRootAvailability::Available
        })
        .expect("project active recovery");

    assert_eq!(snapshot.roots[0].freshness_unknown_count, 1);
    assert_eq!(snapshot.roots[0].freshness, CatalogFreshnessState::Updating);
    catalog
        .abandon_scan(&request.scan_id, "cancelled", 0)
        .expect("abandon fixture scan");
}

#[test]
fn metadata_inventory_readiness_is_isolated_per_root() {
    let fixture = RuntimeFixture::new();
    let factory = FakeFactory::default();
    let mut runtime = LibrarySynchronizationRuntime::new_production(
        crate::ports::erase_library_change_source_factory(factory),
    );
    let mut catalog = fixture.catalog;
    let other_root_path = fixture._storage.path().join("other-source");
    std::fs::create_dir_all(&other_root_path).expect("other source root");
    let request = ScanRequest {
        scan_id: "other-runtime-scan".to_owned(),
        root_path: other_root_path.to_string_lossy().into_owned(),
        max_items: None,
        max_entries: None,
        preview_edge: 512,
    };
    let checkpoint = catalog
        .begin_scan(&request, "other-runtime-root", &request.root_path)
        .expect("begin other root scan");
    catalog
        .publish_scan(
            &request.scan_id,
            "other-runtime-root",
            checkpoint.accepted_items,
            checkpoint.issue_count,
        )
        .expect("publish other root");
    runtime
        .poll_without_authoritative_recovery(&mut catalog, 1_000, |root_path| {
            if Path::new(root_path) == other_root_path {
                LibraryRootAvailability::Offline
            } else {
                LibraryRootAvailability::Available
            }
        })
        .expect("establish both observers");

    assert!(runtime.root_is_ready_for_authoritative_recovery(&fixture.root_id));
    assert!(!runtime.root_is_ready_for_authoritative_recovery("other-runtime-root"));
}

#[test]
fn unavailable_root_retains_catalog_state_without_starting_an_observer() {
    let fixture = RuntimeFixture::new();
    let factory = FakeFactory::default();
    let mut runtime = runtime(factory.clone());
    let mut catalog = fixture.catalog;

    let snapshot = runtime
        .poll(&mut catalog, 1_000, |_| LibraryRootAvailability::Offline)
        .expect("poll unavailable root");

    assert_eq!(
        snapshot.roots[0].freshness,
        CatalogFreshnessState::Unavailable
    );
    assert_eq!(factory.state.lock().expect("fake state").start_count, 0);
    assert!(
        catalog
            .load_incremental_catalog_root(&fixture.root_id)
            .expect("load retained root")
            .is_some()
    );
}

#[test]
fn returning_available_root_reconciles_the_continuity_gap() {
    let fixture = RuntimeFixture::new();
    let factory = FakeFactory::default();
    let mut runtime = runtime(factory.clone());
    let mut catalog = fixture.catalog;
    runtime
        .poll(&mut catalog, 1_000, |_| LibraryRootAvailability::Offline)
        .expect("poll unavailable root");

    let snapshot = runtime
        .poll(&mut catalog, 1_100, |_| LibraryRootAvailability::Available)
        .expect("poll recovered root");

    assert_eq!(
        snapshot.roots[0].freshness,
        CatalogFreshnessState::Synchronized
    );
    assert_eq!(snapshot.roots[0].freshness_unknown_count, 0);
    assert_eq!(factory.state.lock().expect("fake state").start_count, 1);
}

#[test]
fn removing_a_root_stops_and_forgets_its_observer() {
    let fixture = RuntimeFixture::new();
    let factory = FakeFactory::default();
    let mut runtime = runtime(factory.clone());
    let mut catalog = fixture.catalog;
    runtime
        .poll(&mut catalog, 1_000, |_| LibraryRootAvailability::Available)
        .expect("start observer");

    assert!(
        catalog
            .unregister_root(&fixture.root_id)
            .expect("remove root")
    );
    let snapshot = runtime
        .poll(&mut catalog, 1_100, |_| LibraryRootAvailability::Available)
        .expect("reconcile removed root");

    assert!(snapshot.roots.is_empty());
    assert_eq!(factory.state.lock().expect("fake state").stop_count, 1);
}

#[test]
fn shutdown_is_idempotent_and_stops_each_observer_once() {
    let fixture = RuntimeFixture::new();
    let factory = FakeFactory::default();
    let mut runtime = runtime(factory.clone());
    let mut catalog = fixture.catalog;
    runtime
        .poll(&mut catalog, 1_000, |_| LibraryRootAvailability::Available)
        .expect("start observer");

    runtime.stop().expect("first stop");
    runtime.stop().expect("second stop");

    assert_eq!(factory.state.lock().expect("fake state").stop_count, 1);
}

#[test]
fn elapsed_time_does_not_schedule_a_full_root_consistency_scan() {
    let fixture = RuntimeFixture::new();
    let factory = FakeFactory::default();
    let mut runtime = runtime_with_recovery_policy(
        factory,
        AuthoritativeRecoveryPolicy {
            max_scope_entries: 64,
            max_scope_paths: 32,
        },
    );
    let mut catalog = fixture.catalog;
    let initial = runtime
        .poll(&mut catalog, 1_000, |_| LibraryRootAvailability::Available)
        .expect("complete startup recovery");
    let prior_authoritative_pass = catalog
        .load_incremental_catalog_root(&fixture.root_id)
        .expect("load root")
        .expect("root")
        .last_consistency_audit_unix_ms;
    let after_thirty_days = runtime
        .poll(&mut catalog, 1_000 + 30 * 24 * 60 * 60 * 1_000, |_| {
            LibraryRootAvailability::Available
        })
        .expect("poll after elapsed time");
    let root = catalog
        .load_incremental_catalog_root(&fixture.root_id)
        .expect("reload root")
        .expect("root");

    assert_eq!(
        initial.roots[0].freshness,
        CatalogFreshnessState::Synchronized
    );
    assert_eq!(
        after_thirty_days.roots[0].freshness,
        CatalogFreshnessState::Synchronized
    );
    assert_eq!(
        root.last_consistency_audit_unix_ms,
        prior_authoritative_pass
    );
    assert_eq!(after_thirty_days.roots[0].pending_change_count, 0);
}

struct RuntimeFixture {
    _storage: tempfile::TempDir,
    catalog: SqliteCatalog,
    source_root: std::path::PathBuf,
    root_id: String,
}

impl RuntimeFixture {
    fn new() -> Self {
        let storage = tempdir().expect("temporary runtime storage");
        let source_root = storage.path().join("source");
        std::fs::create_dir_all(&source_root).expect("source root");
        let mut catalog =
            SqliteCatalog::open(storage.path().join("catalog.sqlite3")).expect("catalog");
        let request = ScanRequest {
            scan_id: "runtime-scan".to_owned(),
            root_path: source_root.to_string_lossy().into_owned(),
            max_items: None,
            max_entries: None,
            preview_edge: 512,
        };
        let checkpoint = catalog
            .begin_scan(&request, "runtime-root", &request.root_path)
            .expect("begin scan");
        catalog
            .publish_scan(
                &request.scan_id,
                "runtime-root",
                checkpoint.accepted_items,
                checkpoint.issue_count,
            )
            .expect("publish empty root");
        Self {
            _storage: storage,
            catalog,
            source_root,
            root_id: "runtime-root".to_owned(),
        }
    }
}

fn runtime(factory: FakeFactory) -> LibrarySynchronizationRuntime {
    runtime_with_recovery_policy(factory, AuthoritativeRecoveryPolicy::default())
}

fn runtime_with_recovery_policy(
    factory: FakeFactory,
    recovery_policy: AuthoritativeRecoveryPolicy,
) -> LibrarySynchronizationRuntime {
    LibrarySynchronizationRuntime::with_policy(
        factory,
        LibraryChangePlanningLimits::default(),
        crate::domain::LibraryChangeRestartPolicy::default(),
        LibraryChangeQueuePolicy {
            debounce_millis: 0,
            ..LibraryChangeQueuePolicy::default()
        },
        recovery_policy,
        64,
    )
}

fn empty_batch() -> LibraryChangeSourceBatch {
    LibraryChangeSourceBatch {
        observations: Vec::new(),
        health: LibraryChangeSourceHealth::Healthy,
        dropped_observation_count: 0,
        ignored_callback_count: 0,
        last_issue_code: None,
    }
}

#[cfg(windows)]
fn enqueue_live_path_observation(
    factory: &FakeFactory,
    root_id: &str,
    relative_path: &str,
    sequence: u64,
) {
    factory
        .state
        .lock()
        .expect("fake state")
        .batches
        .push_back(LibraryChangeSourceBatch {
            observations: vec![LibraryChangeObservation {
                root_id: root_id.to_owned(),
                root_generation: LibraryRootGeneration::initial(),
                sequence,
                observed_unix_ms: 1_000,
                kind: LibraryChangeObservationKind::Modified,
                scope: LibraryChangeScope::Path,
                relative_path: relative_path.to_owned(),
                previous_relative_path: None,
                origin: LibraryChangeOrigin::LiveNotification,
            }],
            health: LibraryChangeSourceHealth::Healthy,
            dropped_observation_count: 0,
            ignored_callback_count: 0,
            last_issue_code: None,
        });
}

#[cfg(windows)]
fn set_offline_attribute(path: &Path, is_offline: bool) {
    let status = std::process::Command::new("attrib.exe")
        .arg(if is_offline { "+O" } else { "-O" })
        .arg(path)
        .status()
        .expect("attrib executable");
    assert!(status.success());
}

fn write_png(path: &Path, width: u32, height: u32, color: [u8; 3]) {
    RgbImage::from_pixel(width, height, Rgb(color))
        .save(path)
        .expect("write PNG fixture");
}
