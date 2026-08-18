use std::collections::VecDeque;
use std::path::Path;
use std::sync::{Arc, Mutex};

use image::{Rgb, RgbImage};
use tempfile::tempdir;

use crate::adapters::SqliteCatalog;
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
fn starts_configured_roots_and_reports_synchronized_after_an_idle_poll() {
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
    assert_eq!(factory.state.lock().expect("fake state").start_count, 1);
}

#[test]
fn live_observation_publishes_a_delta_and_advances_the_shared_revision() {
    let fixture = RuntimeFixture::new();
    write_png(&fixture.source_root.join("new.png"), 7, 5, [20, 30, 40]);
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
    let mut runtime = runtime(factory);
    let mut catalog = fixture.catalog;

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
fn evidence_gap_is_retained_for_authoritative_reconciliation() {
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
        CatalogFreshnessState::NeedsReconciliation
    );
    assert_eq!(snapshot.roots[0].freshness_unknown_count, 1);
    assert_eq!(snapshot.applied_mutation_count, 0);
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
    LibrarySynchronizationRuntime::with_policy(
        factory,
        LibraryChangePlanningLimits::default(),
        crate::domain::LibraryChangeRestartPolicy::default(),
        LibraryChangeQueuePolicy {
            debounce_millis: 0,
            ..LibraryChangeQueuePolicy::default()
        },
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
