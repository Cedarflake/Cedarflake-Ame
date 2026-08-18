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
    CatalogFreshnessState, LibraryChangeObservation, LibraryChangeObservationKind,
    LibraryChangeOrigin, LibraryChangePlanningLimits, LibraryChangeQueuePolicy, LibraryChangeScope,
    LibraryChangeSourceBatch, LibraryChangeSourceError, LibraryChangeSourceHealth,
    LibraryChangeSourceStopReport, LibraryRootAvailability, LibraryRootGeneration, ScanRequest,
};
use crate::ports::{
    CatalogRepository, IncrementalCatalogRepository, LibraryChangeSource,
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
        });
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

    let error = runtime
        .poll(&mut catalog, 1_000, |_| LibraryRootAvailability::Available)
        .expect_err("enqueue must fail");
    assert_eq!(error.code, "catalog_database_error");
    assert!(
        catalog
            .load_incremental_location_by_relative_path(&fixture.root_id, "new.png")
            .expect("load absent location")
            .is_none()
    );
    injector
        .execute_batch("DROP TRIGGER fail_runtime_enqueue;")
        .expect("remove enqueue failure");

    let snapshot = runtime
        .poll(&mut catalog, 1_100, |_| LibraryRootAvailability::Available)
        .expect("retry retained plan");

    assert_eq!(snapshot.applied_mutation_count, 1);
    assert_eq!(snapshot.catalog_revision, 2);
    assert!(
        catalog
            .load_incremental_location_by_relative_path(&fixture.root_id, "new.png")
            .expect("load recovered location")
            .is_some()
    );
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
        thread::sleep(Duration::from_millis(5));
    }
    let recovered = recovered.expect("restarted observer eventually reconciles the retained gap");

    assert_eq!(
        recovered.roots[0].source_health,
        LibraryChangeSourceHealth::Healthy
    );
    assert_eq!(factory.state.lock().expect("fake state").start_count, 2);
    assert!(
        catalog
            .load_incremental_location_by_relative_path(&fixture.root_id, "during-restart.png",)
            .expect("load recovered location")
            .is_some()
    );
}

#[test]
fn production_poll_mode_leaves_authoritative_work_for_the_background_worker() {
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
    assert_eq!(
        pending.roots[0].freshness,
        CatalogFreshnessState::NeedsReconciliation
    );
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
fn low_frequency_audit_is_persisted_and_does_not_claim_freshness_before_completion() {
    let fixture = RuntimeFixture::new();
    let factory = FakeFactory::default();
    let mut runtime = runtime_with_recovery_policy(
        factory,
        AuthoritativeRecoveryPolicy {
            max_scope_entries: 64,
            max_scope_paths: 32,
            audit_interval_millis: 100,
        },
    );
    let mut catalog = fixture.catalog;
    runtime
        .poll(&mut catalog, 1_000, |_| LibraryRootAvailability::Available)
        .expect("complete startup recovery");

    let scheduled = runtime
        .poll(&mut catalog, 1_100, |_| LibraryRootAvailability::Available)
        .expect("schedule consistency audit");
    assert_ne!(
        scheduled.roots[0].freshness,
        CatalogFreshnessState::Synchronized
    );
    let completed = runtime
        .poll(&mut catalog, 1_100, |_| LibraryRootAvailability::Available)
        .expect("complete consistency audit");
    let root = catalog
        .load_incremental_catalog_root(&fixture.root_id)
        .expect("load audited root")
        .expect("audited root");

    assert_eq!(
        completed.roots[0].freshness,
        CatalogFreshnessState::Synchronized
    );
    assert_eq!(root.last_consistency_audit_unix_ms, Some(1_100));
    let not_due = runtime
        .poll(&mut catalog, 1_150, |_| LibraryRootAvailability::Available)
        .expect("poll before next audit");
    assert_eq!(
        not_due.roots[0].freshness,
        CatalogFreshnessState::Synchronized
    );
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
    }
}

fn write_png(path: &Path, width: u32, height: u32, color: [u8; 3]) {
    RgbImage::from_pixel(width, height, Rgb(color))
        .save(path)
        .expect("write PNG fixture");
}
