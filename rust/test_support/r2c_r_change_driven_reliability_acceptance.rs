#![cfg(windows)]

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use image::{Rgb, RgbImage};
use rusqlite::Connection;
use tempfile::tempdir;

use crate::adapters::{
    FileDiscovery, SqliteCatalog, SqliteCatalogReadExecutor, WatcherRecoveryObservation,
};
use crate::application::scan_library::{run_scan_with_storage, stable_id};
use crate::application::storage::StoragePaths;
use crate::domain::{
    AssetLocationView, JournalFileReference, JournalIdentifier, JournalUsn, LibraryRootGeneration,
    PERSISTENT_JOURNAL_CONTRACT_VERSION, PersistentJournalCapability,
    PersistentJournalCapabilityState, PersistentJournalCheckpoint,
    PersistentJournalContinuityState, PersistentJournalVolumeIdentity, ScanError, ScanRequest,
};
use crate::journal_broker::{
    BrokerCandidate, BrokerFailure, BrokerFailureCode, BrokerResponse, CallerClaim, CandidateKind,
    CandidateScope, JournalCapability, PersistentChangeJournalOperationError,
    PersistentChangeJournalRead, PersistentChangeJournalSession, QueryJournalRequest,
    ReadJournalRangeRequest, ReadJournalVolumeRequest, RegisterRootRequest, ResponseBinding,
    RootCapability, SharedJournalRootOutcome, describe_production_persistent_journal_root,
};
use crate::ports::{IncrementalCatalogRepository, PersistentJournalRepository};

use super::production::ProductionSynchronizationTestHarness;
use super::production_synchronization_cadence::{
    ProductionSynchronizationCadence, capture_wait_intervals_for_contract,
};

const INITIAL_USN: i64 = 20;
const JOURNAL_ID: u64 = 44;
const WORKER_TEST: &str = "application::library_synchronization::change_driven_reliability_acceptance::r2c_r_controlled_process_worker";
const POLL_INTERVAL: Duration = Duration::from_millis(10);
const READY_TIMEOUT: Duration = Duration::from_secs(10);
const VISIBLE_TIMEOUT: Duration = Duration::from_secs(10);
const UNRELATED_BASELINE_ENTRIES: usize = 4_096;

struct ControlledJournalSession {
    root_id: String,
    next_usn: i64,
    candidate: Option<BrokerCandidate>,
    query_count: Arc<AtomicUsize>,
    read_count: Arc<AtomicUsize>,
    close_count: Arc<AtomicUsize>,
}

struct IsolatedVolumeJournalSession {
    failed_root_id: String,
    register_count: Arc<AtomicUsize>,
    query_count: Arc<AtomicUsize>,
    shared_read_count: Arc<AtomicUsize>,
    close_count: Arc<AtomicUsize>,
    captured_root_ids: Arc<Mutex<Vec<String>>>,
}

struct CompletedJournalRead {
    response: BrokerResponse,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct ProductionSynchronizationCadenceInvocationCounts {
    readiness: usize,
    crash_ready: usize,
    recovery: usize,
}

struct CountedProductionSynchronizationCadence {
    cadence: ProductionSynchronizationCadence,
    invocation_counts: ProductionSynchronizationCadenceInvocationCounts,
}

impl CountedProductionSynchronizationCadence {
    fn new(cadence: ProductionSynchronizationCadence) -> Self {
        Self {
            cadence,
            invocation_counts: ProductionSynchronizationCadenceInvocationCounts::default(),
        }
    }

    fn wait_for_readiness(&mut self, timeout: Duration, predicate: impl FnMut() -> bool) {
        self.invocation_counts.readiness += 1;
        self.cadence.wait_until(timeout, predicate);
    }

    fn wait_for_crash_ready(&mut self, timeout: Duration, predicate: impl FnMut() -> bool) {
        self.invocation_counts.crash_ready += 1;
        self.cadence.wait_until(timeout, predicate);
    }

    fn wait_for_recovery(&mut self, timeout: Duration, predicate: impl FnMut() -> bool) {
        self.invocation_counts.recovery += 1;
        self.cadence.wait_until(timeout, predicate);
    }

    fn invocation_counts(&self) -> ProductionSynchronizationCadenceInvocationCounts {
        self.invocation_counts
    }
}

#[test]
fn r2c_r_counted_cadence_composes_shared_tracked_and_875_ms_policies() {
    let tracked_cadence = ProductionSynchronizationCadence::from_shared_policy();
    let tracked_counted_cadence = CountedProductionSynchronizationCadence::new(tracked_cadence);
    assert_eq!(tracked_counted_cadence.cadence, tracked_cadence);

    let shared_cadence = ProductionSynchronizationCadence::try_from_policy_source("875\n")
        .expect("R2c-R shared 875 ms cadence policy");

    let mut cadence = CountedProductionSynchronizationCadence::new(shared_cadence);
    let mut readiness_attempts = 0;
    let mut crash_ready_attempts = 0;
    let mut recovery_attempts = 0;
    let (_, intervals) = capture_wait_intervals_for_contract(|| {
        cadence.wait_for_readiness(Duration::from_secs(1), || {
            readiness_attempts += 1;
            readiness_attempts == 2
        });
        cadence.wait_for_crash_ready(Duration::from_secs(1), || {
            crash_ready_attempts += 1;
            crash_ready_attempts == 2
        });
        cadence.wait_for_recovery(Duration::from_secs(1), || {
            recovery_attempts += 1;
            recovery_attempts == 2
        });
    });
    assert_eq!(intervals, [Duration::from_millis(875); 3]);
    assert_eq!(
        cadence.invocation_counts(),
        ProductionSynchronizationCadenceInvocationCounts {
            readiness: 1,
            crash_ready: 1,
            recovery: 1,
        }
    );
}

impl PersistentChangeJournalSession for ControlledJournalSession {
    fn register_root(
        &self,
        _request: RegisterRootRequest,
    ) -> Result<RootCapability, PersistentChangeJournalOperationError> {
        Ok(RootCapability([0x52; 32]))
    }

    fn query_journal(
        &self,
        request: QueryJournalRequest,
    ) -> Result<BrokerResponse, PersistentChangeJournalOperationError> {
        self.query_count.fetch_add(1, Ordering::AcqRel);
        Ok(BrokerResponse::Journal {
            request_id: 1,
            binding: ResponseBinding::from_request(&request.caller, &request.root),
            capability: JournalCapability::Supported,
            journal_id: Some(JOURNAL_ID),
            first_usn: Some(1),
            next_usn: Some(self.next_usn),
        })
    }

    fn begin_read_range(
        &self,
        _request: ReadJournalRangeRequest,
    ) -> Result<Box<dyn PersistentChangeJournalRead>, PersistentChangeJournalOperationError> {
        Err(PersistentChangeJournalOperationError::InvalidRequest)
    }

    fn begin_read_volume(
        &self,
        request: ReadJournalVolumeRequest,
    ) -> Result<Box<dyn PersistentChangeJournalRead>, PersistentChangeJournalOperationError> {
        self.read_count.fetch_add(1, Ordering::AcqRel);
        let outcomes = request
            .roots
            .iter()
            .map(|root| {
                let candidates = self
                    .candidate
                    .iter()
                    .filter(|candidate| {
                        root.root.root_id == self.root_id
                            && candidate.usn >= root.start_usn
                            && candidate.usn < request.end_usn
                    })
                    .cloned()
                    .collect();
                SharedJournalRootOutcome {
                    binding: ResponseBinding::from_request(&request.caller, &root.root),
                    requested_start_usn: root.start_usn,
                    covered_until_usn: Some(request.end_usn),
                    is_complete: true,
                    candidates,
                    failure: None,
                }
            })
            .collect();
        let volume_id = request
            .roots
            .first()
            .ok_or(PersistentChangeJournalOperationError::InvalidRequest)?
            .root
            .volume_id
            .clone();
        Ok(Box::new(CompletedJournalRead {
            response: BrokerResponse::ReadVolume {
                request_id: 1,
                client_instance: request.caller.client_instance,
                volume_id,
                journal_id: request.journal_id,
                requested_end_usn: request.end_usn,
                max_records: request.max_records,
                max_evidence_bytes: request.max_evidence_bytes,
                outcomes,
                handoffs: Vec::new(),
                pending_renames: Vec::new(),
            },
        }))
    }

    fn close(&self) -> Result<(), PersistentChangeJournalOperationError> {
        self.close_count.fetch_add(1, Ordering::AcqRel);
        Ok(())
    }
}

impl PersistentChangeJournalSession for IsolatedVolumeJournalSession {
    fn register_root(
        &self,
        request: RegisterRootRequest,
    ) -> Result<RootCapability, PersistentChangeJournalOperationError> {
        assert_ne!(request.caller.client_instance, [0; 16]);
        self.register_count.fetch_add(1, Ordering::AcqRel);
        Ok(RootCapability([0x49; 32]))
    }

    fn query_journal(
        &self,
        request: QueryJournalRequest,
    ) -> Result<BrokerResponse, PersistentChangeJournalOperationError> {
        self.query_count.fetch_add(1, Ordering::AcqRel);
        Ok(BrokerResponse::Journal {
            request_id: request.root.root_generation,
            binding: ResponseBinding::from_request(&request.caller, &request.root),
            capability: JournalCapability::Supported,
            journal_id: Some(JOURNAL_ID),
            first_usn: Some(1),
            next_usn: Some(INITIAL_USN + 1),
        })
    }

    fn begin_read_range(
        &self,
        _request: ReadJournalRangeRequest,
    ) -> Result<Box<dyn PersistentChangeJournalRead>, PersistentChangeJournalOperationError> {
        Err(PersistentChangeJournalOperationError::InvalidRequest)
    }

    fn begin_read_volume(
        &self,
        request: ReadJournalVolumeRequest,
    ) -> Result<Box<dyn PersistentChangeJournalRead>, PersistentChangeJournalOperationError> {
        self.shared_read_count.fetch_add(1, Ordering::AcqRel);
        assert_eq!(
            request.roots.len(),
            2,
            "same-volume roots must share one request"
        );
        assert!(
            request
                .roots
                .iter()
                .all(|root| root.root.volume_id == request.roots[0].root.volume_id),
            "the shared request must contain one physical volume"
        );
        *self
            .captured_root_ids
            .lock()
            .expect("captured same-volume roots") = request
            .roots
            .iter()
            .map(|root| root.root.root_id.clone())
            .collect();
        let outcomes = request
            .roots
            .iter()
            .map(|root| {
                let failed = root.root.root_id == self.failed_root_id;
                SharedJournalRootOutcome {
                    binding: ResponseBinding::from_request(&request.caller, &root.root),
                    requested_start_usn: root.start_usn,
                    covered_until_usn: (!failed).then_some(request.end_usn),
                    is_complete: !failed,
                    candidates: Vec::new(),
                    failure: failed.then_some(BrokerFailure {
                        code: BrokerFailureCode::RootUnauthorized,
                    }),
                }
            })
            .collect();
        Ok(Box::new(CompletedJournalRead {
            response: BrokerResponse::ReadVolume {
                request_id: 1,
                client_instance: request.caller.client_instance,
                volume_id: request.roots[0].root.volume_id.clone(),
                journal_id: request.journal_id,
                requested_end_usn: request.end_usn,
                max_records: request.max_records,
                max_evidence_bytes: request.max_evidence_bytes,
                outcomes,
                handoffs: Vec::new(),
                pending_renames: Vec::new(),
            },
        }))
    }

    fn close(&self) -> Result<(), PersistentChangeJournalOperationError> {
        self.close_count.fetch_add(1, Ordering::AcqRel);
        Ok(())
    }
}

impl PersistentChangeJournalRead for CompletedJournalRead {
    fn cancel(&self, _caller: CallerClaim) -> Result<(), PersistentChangeJournalOperationError> {
        Ok(())
    }

    fn wait(self: Box<Self>) -> Result<BrokerResponse, PersistentChangeJournalOperationError> {
        Ok(self.response)
    }
}

#[derive(Clone, Copy)]
enum ClosedOperationKind {
    Create,
    Modify,
    Rename,
    Move,
    Replace,
    Delete,
}

impl ClosedOperationKind {
    const fn label(self) -> &'static str {
        match self {
            Self::Create => "create",
            Self::Modify => "modify",
            Self::Rename => "rename",
            Self::Move => "move",
            Self::Replace => "replace",
            Self::Delete => "delete",
        }
    }
}

struct ClosedOperation {
    kind: ClosedOperationKind,
    previous_relative_path: Option<String>,
    current_relative_path: String,
    prior: Option<AssetLocationView>,
}

struct ControlledFixture {
    _directory: tempfile::TempDir,
    storage: StoragePaths,
    source_root: PathBuf,
    root_path: String,
    root_id: String,
    report_path: PathBuf,
}

impl ControlledFixture {
    fn new() -> Self {
        Self::with_unrelated_baseline(0)
    }

    fn with_unrelated_baseline(unrelated_entries: usize) -> Self {
        let directory = owned_test_directory();
        let source_root = directory.path().join("source");
        fs::create_dir_all(source_root.join("baseline")).expect("R2c-R baseline directory");
        for (name, size, seed) in [
            ("modify.png", 4, 11),
            ("rename.png", 5, 23),
            ("move.png", 6, 37),
            ("replace.png", 7, 53),
            ("delete.png", 8, 71),
        ] {
            write_png(
                &source_root.join("baseline").join(name),
                size,
                size + 1,
                seed,
            );
        }
        for index in 0..unrelated_entries {
            let directory_index = index / 64;
            let path = source_root.join(format!(
                "unrelated-baseline/bucket-{directory_index:03}/entry-{index:05}.bin"
            ));
            fs::create_dir_all(path.parent().expect("unrelated baseline parent"))
                .expect("create unrelated baseline directory");
            fs::write(path, []).expect("write unrelated baseline entry");
        }
        let root_path = FileDiscovery::new(&source_root.to_string_lossy())
            .expect("R2c-R root discovery")
            .canonical_root()
            .expect("R2c-R canonical root")
            .to_string_lossy()
            .into_owned();
        let root_id = stable_id("library-root-v1", &root_path);
        let storage = StoragePaths {
            catalog_path: directory.path().join("catalog").join("ame.sqlite3"),
            preview_root: directory.path().join("previews"),
            preview_budget_bytes: 64 * 1024 * 1024,
            settings_path: directory.path().join("settings").join("storage.sqlite3"),
        };
        run_scan_with_storage(
            ScanRequest {
                scan_id: "r2c-r-controlled-baseline".to_owned(),
                root_path: root_path.clone(),
                max_items: None,
                max_entries: None,
                preview_edge: 128,
            },
            |_| true,
            storage.clone(),
        )
        .expect("R2c-R initial controlled baseline");
        seed_current_checkpoint(&storage, &source_root, &root_id, INITIAL_USN);
        Self {
            report_path: directory.path().join("r2c-r-report.log"),
            _directory: directory,
            storage,
            source_root,
            root_path,
            root_id,
        }
    }

    fn location(&self, relative_path: &str) -> Option<AssetLocationView> {
        SqliteCatalog::open(self.storage.catalog_path.clone())
            .expect("R2c-R catalog")
            .load_incremental_location_by_relative_path(&self.root_id, relative_path)
            .expect("R2c-R location query")
    }

    fn prepare_operation(&self, kind: ClosedOperationKind) -> ClosedOperation {
        match kind {
            ClosedOperationKind::Create => {
                let current = "关闭期间/新增图片.png".to_owned();
                let path = self.source_root.join(&current);
                fs::create_dir_all(path.parent().expect("create parent"))
                    .expect("create Chinese path parent");
                write_png(&path, 13, 17, 89);
                ClosedOperation {
                    kind,
                    previous_relative_path: None,
                    current_relative_path: current,
                    prior: None,
                }
            }
            ClosedOperationKind::Modify => {
                let current = "baseline/modify.png".to_owned();
                let prior = self.location(&current).expect("modify prior");
                write_png(&self.source_root.join(&current), 19, 23, 101);
                ClosedOperation {
                    kind,
                    previous_relative_path: None,
                    current_relative_path: current,
                    prior: Some(prior),
                }
            }
            ClosedOperationKind::Rename => {
                let previous = "baseline/rename.png".to_owned();
                let current = "重命名/完成.png".to_owned();
                let prior = self.location(&previous).expect("rename prior");
                let destination = self.source_root.join(&current);
                fs::create_dir_all(destination.parent().expect("rename parent"))
                    .expect("create rename parent");
                fs::rename(self.source_root.join(&previous), destination)
                    .expect("rename while runtime is closed");
                ClosedOperation {
                    kind,
                    previous_relative_path: Some(previous),
                    current_relative_path: current,
                    prior: Some(prior),
                }
            }
            ClosedOperationKind::Move => {
                let previous = "baseline/move.png".to_owned();
                let long_component = "long-path-component-abcdefghijklmnopqrstuvwxyz0123456789";
                let current = format!(
                    "moved/{long_component}/{long_component}/{long_component}/{long_component}/移动.png"
                );
                assert!(self.source_root.join(&current).as_os_str().len() > 260);
                let prior = self.location(&previous).expect("move prior");
                let destination = self.source_root.join(&current);
                fs::create_dir_all(destination.parent().expect("move parent"))
                    .expect("create long-path parent");
                fs::rename(self.source_root.join(&previous), destination)
                    .expect("move into long path while runtime is closed");
                ClosedOperation {
                    kind,
                    previous_relative_path: Some(previous),
                    current_relative_path: current,
                    prior: Some(prior),
                }
            }
            ClosedOperationKind::Replace => {
                let current = "baseline/replace.png".to_owned();
                let prior = self.location(&current).expect("replacement prior");
                let path = self.source_root.join(&current);
                fs::remove_file(&path).expect("remove replacement predecessor");
                write_png(&path, 29, 31, 127);
                ClosedOperation {
                    kind,
                    previous_relative_path: None,
                    current_relative_path: current,
                    prior: Some(prior),
                }
            }
            ClosedOperationKind::Delete => {
                let current = "baseline/delete.png".to_owned();
                let prior = self.location(&current).expect("delete prior");
                fs::remove_file(self.source_root.join(&current))
                    .expect("delete while runtime is closed");
                ClosedOperation {
                    kind,
                    previous_relative_path: None,
                    current_relative_path: current,
                    prior: Some(prior),
                }
            }
        }
    }

    fn assert_operation_visible(&self, operation: &ClosedOperation) {
        let current = self.location(&operation.current_relative_path);
        match operation.kind {
            ClosedOperationKind::Create => assert!(current.is_some()),
            ClosedOperationKind::Modify => {
                let current = current.expect("modified location");
                let prior = operation.prior.as_ref().expect("modify prior");
                assert_eq!(current.asset_id, prior.asset_id);
                assert!(file_state_changed(prior, &current));
            }
            ClosedOperationKind::Rename | ClosedOperationKind::Move => {
                let current = current.expect("renamed or moved location");
                let prior = operation.prior.as_ref().expect("rename prior");
                assert_eq!(current.asset_id, prior.asset_id);
                assert!(
                    self.location(
                        operation
                            .previous_relative_path
                            .as_deref()
                            .expect("previous rename path")
                    )
                    .is_none()
                );
            }
            ClosedOperationKind::Replace => {
                let current = current.expect("replacement location");
                let prior = operation.prior.as_ref().expect("replacement prior");
                assert_ne!(current.asset_id, prior.asset_id);
                assert!(file_state_changed(prior, &current));
            }
            ClosedOperationKind::Delete => assert!(current.is_none()),
        }
    }

    fn operation_is_visible(&self, operation: &ClosedOperation) -> bool {
        let current = self.location(&operation.current_relative_path);
        match operation.kind {
            ClosedOperationKind::Create => current.is_some(),
            ClosedOperationKind::Modify => current.is_some_and(|current| {
                operation.prior.as_ref().is_some_and(|prior| {
                    prior.asset_id == current.asset_id && file_state_changed(prior, &current)
                })
            }),
            ClosedOperationKind::Rename | ClosedOperationKind::Move => {
                current.is_some_and(|current| {
                    operation
                        .prior
                        .as_ref()
                        .is_some_and(|prior| prior.asset_id == current.asset_id)
                }) && operation
                    .previous_relative_path
                    .as_deref()
                    .is_some_and(|previous| self.location(previous).is_none())
            }
            ClosedOperationKind::Replace => current.is_some_and(|current| {
                operation.prior.as_ref().is_some_and(|prior| {
                    prior.asset_id != current.asset_id && file_state_changed(prior, &current)
                })
            }),
            ClosedOperationKind::Delete => current.is_none(),
        }
    }
}

#[test]
fn r2c_r_controlled_journal_evidence_uses_the_production_coordinator() {
    let directory = tempdir().expect("R2c-R evidence directory");
    let source_root = directory.path().join("source");
    fs::create_dir_all(&source_root).expect("R2c-R source root");
    let root_path = source_root.to_string_lossy().into_owned();
    let storage = StoragePaths {
        catalog_path: directory.path().join("catalog").join("ame.sqlite3"),
        preview_root: directory.path().join("previews"),
        preview_budget_bytes: 64 * 1024 * 1024,
        settings_path: directory.path().join("settings").join("storage.sqlite3"),
    };
    crate::adapters::reset_source_content_open_instrumentation(&root_path);
    let session = controlled_session("unused", INITIAL_USN, None);
    let mut runtime = ProductionSynchronizationTestHarness::with_persistent_journal_session(
        storage,
        session.0,
        caller_claim(1),
    );

    runtime.poll().expect("production coordinator poll");
    runtime.stop().expect("production coordinator stop");
    assert_eq!(
        crate::adapters::source_content_open_count(&root_path),
        0,
        "an empty controlled root must open no media content"
    );
}

#[test]
fn r2c_r_same_volume_roots_share_one_retained_session_read_and_isolate_failure() {
    let directory = tempdir().expect("R2c-R same-volume directory");
    let storage = StoragePaths {
        catalog_path: directory.path().join("catalog").join("ame.sqlite3"),
        preview_root: directory.path().join("previews"),
        preview_budget_bytes: 64 * 1024 * 1024,
        settings_path: directory.path().join("settings").join("storage.sqlite3"),
    };
    let mut roots = Vec::new();
    for label in ["healthy", "isolated-failure"] {
        let source_root = directory.path().join(format!("source-{label}"));
        fs::create_dir_all(&source_root).expect("create same-volume source root");
        let root_path = FileDiscovery::new(&source_root.to_string_lossy())
            .expect("same-volume root discovery")
            .canonical_root()
            .expect("canonical same-volume root")
            .to_string_lossy()
            .into_owned();
        let root_id = stable_id("library-root-v1", &root_path);
        run_scan_with_storage(
            ScanRequest {
                scan_id: format!("r2c-r-same-volume-{label}"),
                root_path: root_path.clone(),
                max_items: None,
                max_entries: None,
                preview_edge: 128,
            },
            |_| true,
            storage.clone(),
        )
        .expect("publish same-volume root baseline");
        seed_current_checkpoint(&storage, &source_root, &root_id, INITIAL_USN);
        crate::adapters::reset_source_enumeration_instrumentation(&root_path);
        crate::adapters::reset_source_root_entry_enumeration_count(&root_path);
        crate::adapters::reset_source_content_open_instrumentation(&root_path);
        crate::adapters::reset_observer_root_handle_open_count(&root_path);
        roots.push((root_id, root_path, source_root));
    }
    let scan_rows_before = scan_row_count(&storage.catalog_path);
    let source_before = roots
        .iter()
        .map(|(_, _, source_root)| source_snapshot(source_root))
        .collect::<Vec<_>>();
    let healthy_root_id = roots[0].0.clone();
    let failed_root_id = roots[1].0.clone();
    let failed_recovery_gate = crate::adapters::gate_source_enumeration(&roots[1].1);
    let register_count = Arc::new(AtomicUsize::new(0));
    let query_count = Arc::new(AtomicUsize::new(0));
    let shared_read_count = Arc::new(AtomicUsize::new(0));
    let close_count = Arc::new(AtomicUsize::new(0));
    let captured_root_ids = Arc::new(Mutex::new(Vec::new()));
    let session: Arc<dyn PersistentChangeJournalSession> = Arc::new(IsolatedVolumeJournalSession {
        failed_root_id: failed_root_id.clone(),
        register_count: Arc::clone(&register_count),
        query_count: Arc::clone(&query_count),
        shared_read_count: Arc::clone(&shared_read_count),
        close_count: Arc::clone(&close_count),
        captured_root_ids: Arc::clone(&captured_root_ids),
    });
    let mut runtime = ProductionSynchronizationTestHarness::with_persistent_journal_session(
        storage.clone(),
        session,
        caller_claim(0x61),
    );

    wait_until(READY_TIMEOUT, || {
        runtime
            .poll()
            .expect("same-volume production coordinator poll");
        if shared_read_count.load(Ordering::Acquire) != 1 {
            return false;
        }
        let catalog =
            SqliteCatalog::open(storage.catalog_path.clone()).expect("inspect same-volume catalog");
        let healthy = catalog
            .load_persistent_journal_checkpoint(&healthy_root_id, LibraryRootGeneration::initial())
            .expect("load healthy checkpoint");
        let failed = catalog
            .load_persistent_journal_checkpoint(&failed_root_id, LibraryRootGeneration::initial())
            .expect("load failed checkpoint");
        let failed_capability = catalog
            .load_persistent_journal_capabilities()
            .expect("load same-volume capabilities")
            .into_iter()
            .find(|capability| capability.root_id == failed_root_id);
        healthy.is_some_and(|checkpoint| {
            checkpoint.next_unread_usn.value() == INITIAL_USN + 1
                && checkpoint.continuity == PersistentJournalContinuityState::Current
                && checkpoint.failure.is_none()
        }) && failed.is_some_and(|checkpoint| {
            checkpoint.next_unread_usn.value() == INITIAL_USN
                && checkpoint.continuity == PersistentJournalContinuityState::RecoveryRequired
                && checkpoint.failure.is_some()
        }) && failed_capability.is_some_and(|capability| {
            capability.continuity == PersistentJournalContinuityState::RecoveryRequired
        })
    });
    failed_recovery_gate.release();
    runtime
        .stop()
        .expect("stop same-volume production coordinator");

    assert_eq!(register_count.load(Ordering::Acquire), 2);
    assert_eq!(query_count.load(Ordering::Acquire), 2);
    assert_eq!(shared_read_count.load(Ordering::Acquire), 1);
    assert_eq!(close_count.load(Ordering::Acquire), 1);
    let captured = captured_root_ids
        .lock()
        .expect("captured same-volume roots")
        .iter()
        .cloned()
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        captured,
        [healthy_root_id.clone(), failed_root_id.clone()]
            .into_iter()
            .collect()
    );
    assert_eq!(
        roots
            .iter()
            .map(|(_, root_path, _)| {
                crate::adapters::observer_root_handle_open_count(root_path)
            })
            .sum::<u64>(),
        2,
        "both roots must use the production observer"
    );
    assert_eq!(scan_row_count(&storage.catalog_path), scan_rows_before);
    assert_eq!(
        roots
            .iter()
            .map(|(_, _, source_root)| source_snapshot(source_root))
            .collect::<Vec<_>>(),
        source_before,
        "same-volume catch-up must not mutate either source"
    );
    eprintln!(
        "R2c-R same-volume roots=2 production_observer_handles=2 retained_session_registers=2 retained_session_queries=2 shared_volume_reads=1 healthy_checkpoint_advanced=1 failed_checkpoint_isolated=1 shutdown_closes=1 full_scan_rows_added=0 source_snapshots_unchanged=true"
    );
}

#[test]
#[ignore = "requires the serial Windows R2c-R local reliability wrapper"]
fn r2c_r_live_operations_and_event_storm_remain_p0_bounded() {
    const STORM_PATHS: usize = 96;
    const EXPECTED_MEDIA_CONTENT_OPENS: u64 = STORM_PATHS as u64 + 3;

    let fixture = ControlledFixture::with_unrelated_baseline(UNRELATED_BASELINE_ENTRIES);
    let scan_rows_before = scan_row_count(&fixture.storage.catalog_path);
    let unrelated_baseline_snapshot =
        source_snapshot(&fixture.source_root.join("unrelated-baseline"));
    crate::adapters::reset_source_enumeration_instrumentation(&fixture.root_path);
    crate::adapters::reset_source_root_entry_enumeration_count(&fixture.root_path);
    crate::adapters::reset_source_content_open_instrumentation(&fixture.root_path);
    let (session, query_count, read_count, close_count) =
        controlled_session(&fixture.root_id, INITIAL_USN, None);
    let mut runtime = ProductionSynchronizationTestHarness::with_persistent_journal_session(
        fixture.storage.clone(),
        session,
        caller_claim(0x50),
    );
    wait_until(READY_TIMEOUT, || {
        runtime.poll().is_ok_and(|snapshot| {
            query_count.load(Ordering::Acquire) >= 1
                && snapshot.roots.iter().any(|root| {
                    root.root_id == fixture.root_id
                        && root.source_health == crate::domain::LibraryChangeSourceHealth::Healthy
                })
        })
    });

    let mut live_latencies = Vec::with_capacity(6);
    for kind in [
        ClosedOperationKind::Create,
        ClosedOperationKind::Modify,
        ClosedOperationKind::Rename,
        ClosedOperationKind::Move,
        ClosedOperationKind::Replace,
        ClosedOperationKind::Delete,
    ] {
        let started = Instant::now();
        let operation = fixture.prepare_operation(kind);
        let source_after_fixture_change =
            operation_source_snapshot(&fixture.source_root, &operation);
        wait_until(VISIBLE_TIMEOUT, || {
            runtime.poll().expect("R2c-R live production poll");
            fixture.operation_is_visible(&operation)
        });
        live_latencies.push(started.elapsed());
        fixture.assert_operation_visible(&operation);
        assert_eq!(
            operation_source_snapshot(&fixture.source_root, &operation),
            source_after_fixture_change,
            "Ame changed an affected source entry or its bytes after the live fixture mutation"
        );
    }

    let storm_queue_before = p0_queue_evidence(&fixture.storage.catalog_path, &fixture.root_id);
    let storm_started = Instant::now();
    let mut storm_relative_paths = Vec::with_capacity(STORM_PATHS);
    for index in 0..STORM_PATHS {
        let relative_path = format!("event-storm/item-{index:04}.png");
        let path = fixture.source_root.join(&relative_path);
        write_png(&path, 2, 2, index as u8);
        write_png(&path, 3, 3, index.wrapping_add(31) as u8);
        write_png(&path, 4, 5, index.wrapping_add(97) as u8);
        storm_relative_paths.push(relative_path);
    }
    let source_after_storm = source_paths_snapshot(&fixture.source_root, &storm_relative_paths);
    wait_until(Duration::from_secs(30), || {
        runtime.poll().expect("R2c-R storm production poll");
        (0..STORM_PATHS).all(|index| {
            let relative_path = format!("event-storm/item-{index:04}.png");
            let expected_size = fs::metadata(fixture.source_root.join(&relative_path))
                .expect("R2c-R storm source metadata")
                .len();
            fixture
                .location(&relative_path)
                .is_some_and(|location| location.file_size == expected_size)
        })
    });
    let storm_elapsed = storm_started.elapsed();
    assert_eq!(
        source_paths_snapshot(&fixture.source_root, &storm_relative_paths),
        source_after_storm
    );
    let storm_queue_after = p0_queue_evidence(&fixture.storage.catalog_path, &fixture.root_id);
    let storm_queue_rows = storm_queue_after.0.saturating_sub(storm_queue_before.0);
    let storm_observations = storm_queue_after.1.saturating_sub(storm_queue_before.1);

    runtime.stop().expect("R2c-R live production stop");
    live_latencies.sort_unstable();
    let p50 = percentile(&live_latencies, 50);
    let p95 = percentile(&live_latencies, 95);
    let maximum = *live_latencies.last().expect("R2c-R live samples");
    let over_one_second = live_latencies
        .iter()
        .filter(|latency| **latency > Duration::from_secs(1))
        .count();
    let affected_path_entry_reads = crate::adapters::source_entry_read_count(&fixture.root_path);
    let media_content_opens = crate::adapters::source_content_open_count(&fixture.root_path);
    assert_eq!(
        source_snapshot(&fixture.source_root.join("unrelated-baseline")),
        unrelated_baseline_snapshot,
        "P0 processing mutated the unrelated baseline"
    );
    eprintln!(
        "R2c-R live samples=6 unrelated_baseline_entries={UNRELATED_BASELINE_ENTRIES} p50_ms={} p95_ms={} max_ms={} over_one_second={} event_storm_paths={STORM_PATHS} storm_queue_rows={storm_queue_rows} storm_observations={storm_observations} storm_visible_ms={} journal_queries={} journal_reads={} media_content_opens={} exact_expected_media_content_opens={EXPECTED_MEDIA_CONTENT_OPENS} affected_path_entry_reads={} affected_path_read_limit={} root_inventory_entry_reads={} inventory_runs=0 p2_rows={} full_scan_rows_added=0 affected_source_snapshots_unchanged=true unrelated_baseline_snapshot_unchanged=true shutdown_closes={}",
        p50.as_millis(),
        p95.as_millis(),
        maximum.as_millis(),
        over_one_second,
        storm_elapsed.as_millis(),
        query_count.load(Ordering::Acquire),
        read_count.load(Ordering::Acquire),
        media_content_opens,
        affected_path_entry_reads,
        STORM_PATHS * 2,
        crate::adapters::source_root_entry_enumeration_count(&fixture.root_path),
        p2_queue_count(&fixture.storage.catalog_path, &fixture.root_id),
        close_count.load(Ordering::Acquire),
    );
    assert!(p95 <= Duration::from_secs(1), "live P0 P95 was {p95:?}");
    assert_eq!(over_one_second, 0);
    assert!(storm_queue_rows <= STORM_PATHS as u64);
    assert!(storm_observations >= STORM_PATHS as u64);
    assert_eq!(read_count.load(Ordering::Acquire), 0);
    assert_eq!(close_count.load(Ordering::Acquire), 1);
    assert_eq!(
        media_content_opens, EXPECTED_MEDIA_CONTENT_OPENS,
        "P0 must open each content-bearing affected path exactly once"
    );
    assert!(
        affected_path_entry_reads
            <= u64::try_from(STORM_PATHS * 2).expect("bounded storm entry reads"),
        "ordinary event-storm reconciliation must stay within the affected subtree"
    );
    assert!(
        affected_path_entry_reads
            < u64::try_from(UNRELATED_BASELINE_ENTRIES).expect("unrelated baseline size"),
        "affected-path reads must not scale with the unrelated library baseline"
    );
    assert_eq!(
        crate::adapters::source_root_entry_enumeration_count(&fixture.root_path),
        0
    );
    assert_eq!(inventory_run_count(&fixture.storage.catalog_path), 0);
    assert_eq!(
        p2_queue_count(&fixture.storage.catalog_path, &fixture.root_id),
        0
    );
    assert_eq!(
        scan_row_count(&fixture.storage.catalog_path),
        scan_rows_before,
        "ordinary P0 changes must not hide a full scan"
    );
}

#[test]
#[ignore = "requires the serial Windows R2c-R local reliability wrapper"]
fn r2c_r_watcher_overflow_recovers_while_twenty_five_real_p0_changes_remain_fast()
-> Result<(), ScanError> {
    const OVERFLOW_PATHS: usize = 256;
    const P0_SAMPLES: usize = 25;

    let fixture = ControlledFixture::new();
    let watcher_observer =
        SqliteCatalogReadExecutor::open_existing(fixture.storage.catalog_path.clone())
            .expect("R2c-R watcher-gap read observer");
    let scan_rows_before = scan_row_count(&fixture.storage.catalog_path);
    crate::adapters::reset_source_enumeration_instrumentation(&fixture.root_path);
    crate::adapters::reset_source_root_entry_enumeration_count(&fixture.root_path);
    crate::adapters::reset_source_content_open_instrumentation(&fixture.root_path);
    let (session, query_count, read_count, close_count) =
        controlled_session(&fixture.root_id, INITIAL_USN, None);
    let mut runtime = ProductionSynchronizationTestHarness::with_persistent_journal_session(
        fixture.storage.clone(),
        session,
        caller_claim(0x51),
    );
    wait_until(READY_TIMEOUT, || {
        runtime.poll().is_ok_and(|snapshot| {
            query_count.load(Ordering::Acquire) >= 1
                && snapshot.roots.iter().any(|root| {
                    root.root_id == fixture.root_id
                        && root.source_health == crate::domain::LibraryChangeSourceHealth::Healthy
                })
        })
    });

    let source_gate = crate::adapters::gate_source_enumeration(&fixture.root_path);
    for index in 0..OVERFLOW_PATHS {
        let path = fixture
            .source_root
            .join(format!("overflow/item-{index:04}.png"));
        write_png(&path, 2, 2, index as u8);
        write_png(&path, 3, 3, index.wrapping_add(31) as u8);
        write_png(&path, 4, 5, index.wrapping_add(97) as u8);
    }
    let overflow_started = Instant::now();
    wait_until_result(Duration::from_secs(15), || {
        runtime.poll()?;
        let observation = watcher_recovery_observation(&watcher_observer, &fixture.root_id)?;
        Ok(observation.authority_count >= 1
            && observation.active_inventory_run_count >= 1
            && source_gate.wait_until_blocked(Duration::from_millis(10)))
    })?;
    let active_authority_change_id =
        watcher_recovery_observation(&watcher_observer, &fixture.root_id)?
            .active_authority_change_id
            .ok_or_else(|| {
                ScanError::new(
                    "r2c_r_active_watcher_gap_authority_missing",
                    "R2c-R active watcher-gap recovery authority was not visible",
                )
            })?;
    let source_reads_before_p2 = crate::adapters::source_entry_read_count(&fixture.root_path);
    assert!(
        source_reads_before_p2 <= u64::try_from(OVERFLOW_PATHS * 2).expect("bounded P0 reads"),
        "pre-recovery P0 subtree reconciliation was not bounded"
    );

    let priority_path = fixture.source_root.join("overflow-priority.png");
    let mut p0_latencies = Vec::with_capacity(P0_SAMPLES);
    let mut p0_active_authority_samples = 0_usize;
    for index in 0..P0_SAMPLES {
        assert!(
            source_gate.wait_until_blocked(Duration::from_millis(10)),
            "P2 must remain active for every measured P0 sample"
        );
        let observation = watcher_recovery_observation(&watcher_observer, &fixture.root_id)?;
        assert!(
            observation.active_inventory_run_count >= 1,
            "the overflow inventory must remain active for every P0 sample"
        );
        assert_eq!(
            observation.active_authority_change_id,
            Some(active_authority_change_id),
            "every P0 sample must start under the same unretired watcher-gap authority"
        );
        let width = 40 + u32::try_from(index).expect("P0 sample width");
        let started = Instant::now();
        write_png(
            &priority_path,
            width,
            width + 1,
            u8::try_from(index).expect("P0 sample seed"),
        );
        wait_until_result(VISIBLE_TIMEOUT, || {
            runtime.poll()?;
            Ok(
                observer_location(&watcher_observer, &fixture.root_id, "overflow-priority.png")?
                    .is_some_and(|location| location.width == width),
            )
        })?;
        let observation = watcher_recovery_observation(&watcher_observer, &fixture.root_id)?;
        assert_eq!(
            observation.active_authority_change_id,
            Some(active_authority_change_id),
            "every P0 sample must become visible before its P2 authority retires"
        );
        p0_latencies.push(started.elapsed());
        p0_active_authority_samples += 1;
    }
    assert_eq!(
        crate::adapters::source_entry_read_count(&fixture.root_path),
        source_reads_before_p2,
        "P0 must not release or consume the blocked P2 source page"
    );
    let source_after_fixture_changes = source_snapshot(&fixture.source_root);
    source_gate.release();
    wait_until_result(Duration::from_secs(60), || {
        runtime.poll()?;
        let observation = watcher_recovery_observation(&watcher_observer, &fixture.root_id)?;
        Ok(observation.active_inventory_run_count == 0
            && all_overflow_locations_visible(&watcher_observer, &fixture.root_id, OVERFLOW_PATHS)?)
    })?;
    let overflow_convergence = overflow_started.elapsed();
    assert_eq!(
        source_snapshot(&fixture.source_root),
        source_after_fixture_changes,
        "overflow recovery changed source entries or bytes"
    );
    runtime.stop().expect("R2c-R overflow production stop");

    p0_latencies.sort_unstable();
    let p50 = percentile(&p0_latencies, 50);
    let p95 = percentile(&p0_latencies, 95);
    let maximum = *p0_latencies.last().expect("overflow P0 samples");
    let over_one_second = p0_latencies
        .iter()
        .filter(|latency| **latency > Duration::from_secs(1))
        .count();
    let inventory = inventory_metrics(&fixture.storage.catalog_path, &fixture.root_id);
    let p2_queue = p2_queue_metrics(&fixture.storage.catalog_path, &fixture.root_id);
    let source_reads = crate::adapters::source_entry_read_count(&fixture.root_path);
    let p2_source_reads = source_reads.saturating_sub(source_reads_before_p2);
    let watcher_authority_count =
        watcher_recovery_observation(&watcher_observer, &fixture.root_id)?.authority_count;
    let watcher_read_attempts = watcher_observer.attempt_stats();
    eprintln!(
        "R2c-R watcher-overflow paths={OVERFLOW_PATHS} writes={} typed_authorities={} inventory_runs={} max_staged_entries={} max_next_page_index={} total_inventory_candidates={} p2_control_rows={} p2_owned_candidate_rows={} p2_total_rows={} pre_recovery_p0_subtree_reads={} p2_source_reads={} source_entry_reads_total={} p0_active_authority_samples={}/{} p0_p50_ms={} p0_p95_ms={} p0_max_ms={} p0_over_one_second={} convergence_ms={} journal_reads={} media_content_opens={} watcher_read_operations={} watcher_read_attempts={} watcher_protocol_retries={} watcher_max_attempts={} scan_rows_unchanged=true source_post_fixture_snapshot_unchanged=true shutdown_closes={}",
        OVERFLOW_PATHS * 3,
        watcher_authority_count,
        inventory.run_count,
        inventory.maximum_staged_entries,
        inventory.maximum_next_page_index,
        inventory.total_candidates,
        p2_queue.control_rows,
        p2_queue.owned_candidate_rows,
        p2_queue.total_rows,
        source_reads_before_p2,
        p2_source_reads,
        source_reads,
        p0_active_authority_samples,
        P0_SAMPLES,
        p50.as_millis(),
        p95.as_millis(),
        maximum.as_millis(),
        over_one_second,
        overflow_convergence.as_millis(),
        read_count.load(Ordering::Acquire),
        crate::adapters::source_content_open_count(&fixture.root_path),
        watcher_read_attempts.operations,
        watcher_read_attempts.attempts,
        watcher_read_attempts.protocol_retries,
        watcher_read_attempts.maximum_attempts,
        close_count.load(Ordering::Acquire),
    );
    assert_eq!(p0_latencies.len(), P0_SAMPLES);
    assert_eq!(p0_active_authority_samples, P0_SAMPLES);
    assert!(p95 <= Duration::from_secs(1), "overflow P0 P95 was {p95:?}");
    assert_eq!(over_one_second, 0);
    assert_eq!(watcher_authority_count, 1);
    assert_eq!(inventory.run_count, 1);
    assert_eq!(inventory.maximum_staged_entries, 257);
    assert_eq!(inventory.maximum_next_page_index, 2);
    assert_eq!(inventory.total_candidates, 256);
    assert!(
        p2_source_reads <= u64::from(inventory.run_count).saturating_mul(4_095),
        "overflow source reads exceeded bounded production pages"
    );
    assert_eq!(p2_queue.control_rows, 1);
    assert_eq!(p2_queue.owned_candidate_rows, 256);
    assert_eq!(p2_queue.total_rows, 257);
    assert!(p2_queue.total_rows <= 3_072);
    assert_eq!(read_count.load(Ordering::Acquire), 0);
    assert_eq!(close_count.load(Ordering::Acquire), 1);
    assert!(
        watcher_read_attempts.operations <= 640,
        "watcher recovery observation must use bounded typed read snapshots"
    );
    assert_eq!(
        scan_row_count(&fixture.storage.catalog_path),
        scan_rows_before
    );
    Ok(())
}

#[test]
#[ignore = "requires the serial Windows R2c-R local reliability wrapper"]
fn r2c_r_closed_process_changes_converge_through_durable_p1() {
    let fixture = ControlledFixture::new();
    let observer = SqliteCatalogReadExecutor::open_existing(fixture.storage.catalog_path.clone())
        .expect("R2c-R closed-process read observer");
    let scan_rows_before = scan_row_count(&fixture.storage.catalog_path);
    crate::adapters::reset_source_enumeration_instrumentation(&fixture.root_path);
    crate::adapters::reset_source_root_entry_enumeration_count(&fixture.root_path);
    crate::adapters::reset_source_content_open_instrumentation(&fixture.root_path);

    run_worker(&fixture, "crash-ready", INITIAL_USN, None);

    let mut next_usn = INITIAL_USN;
    for kind in [
        ClosedOperationKind::Create,
        ClosedOperationKind::Modify,
        ClosedOperationKind::Rename,
        ClosedOperationKind::Move,
        ClosedOperationKind::Replace,
        ClosedOperationKind::Delete,
    ] {
        let operation = fixture.prepare_operation(kind);
        let safety_before = source_snapshot(&fixture.source_root);
        let start_usn = next_usn;
        next_usn += 1;
        run_worker(&fixture, "recover", next_usn, Some((&operation, start_usn)));
        fixture.assert_operation_visible(&operation);
        assert_eq!(source_snapshot(&fixture.source_root), safety_before);
        assert_p1_range_completed(&observer, &fixture.root_id, start_usn, next_usn);
    }

    let mut latencies = report_latencies(&fixture.report_path);
    assert_eq!(latencies.len(), 6);
    latencies.sort_unstable();
    let p50 = percentile(&latencies, 50);
    let p95 = percentile(&latencies, 95);
    let maximum = *latencies.last().expect("closed-process latency samples");
    let over_two_seconds = latencies
        .iter()
        .filter(|latency| **latency > Duration::from_secs(2))
        .count();
    let source_entry_reads = report_metric_sum(&fixture.report_path, "source_entry_reads");
    let inventory_entry_reads = report_metric_sum(&fixture.report_path, "inventory_entry_reads");
    let content_opens = report_metric_sum(&fixture.report_path, "content_opens");
    let availability_metadata_probe_samples =
        report_metric_values(&fixture.report_path, "availability_metadata_probes");
    let availability_readiness_probe_samples =
        report_metric_values(&fixture.report_path, "availability_readiness_probes");
    let availability_metadata_probes = availability_metadata_probe_samples.iter().sum::<u64>();
    let discovery_root_handles = report_metric_sum(&fixture.report_path, "discovery_root_handles");
    let publication_guard_root_handles =
        report_metric_sum(&fixture.report_path, "publication_guard_root_handles");
    eprintln!(
        "R2c-R closed-process samples={} p50_ms={} p95_ms={} max_ms={} over_two_seconds={} source_ranges=6 queue_lineage=6 inventory_runs=0 source_entry_reads={} inventory_entry_reads={} media_content_opens={} availability_metadata_probes={} availability_readiness_probe_samples={availability_readiness_probe_samples:?} availability_metadata_probe_samples={availability_metadata_probe_samples:?} discovery_root_handles={} publication_guard_root_handles={} scan_rows_unchanged=true source_post_fixture_snapshots_unchanged=true",
        latencies.len(),
        p50.as_millis(),
        p95.as_millis(),
        maximum.as_millis(),
        over_two_seconds,
        source_entry_reads,
        inventory_entry_reads,
        content_opens,
        availability_metadata_probes,
        discovery_root_handles,
        publication_guard_root_handles,
    );
    assert!(
        p95 <= Duration::from_secs(2),
        "closed-process P95 was {p95:?}"
    );
    assert_eq!(over_two_seconds, 0);
    assert_eq!(
        source_entry_reads, 0,
        "P1 must not enumerate the source root"
    );
    assert_eq!(
        inventory_entry_reads, 0,
        "P1 must not run metadata inventory"
    );
    assert!(
        content_opens <= 6,
        "P1 must open content only for bounded changed-file inspection"
    );
    assert!(
        publication_guard_root_handles >= 6,
        "every P1 sample must use the guarded production publication path"
    );
    assert!(
        (6..=60).contains(&availability_metadata_probes),
        "closed-process recovery must use bounded metadata-only availability probes"
    );
    assert_eq!(inventory_run_count(&fixture.storage.catalog_path), 0);
    assert_eq!(
        scan_row_count(&fixture.storage.catalog_path),
        scan_rows_before
    );
}

#[test]
#[ignore = "launched as an isolated child by the R2c-R controlled acceptance"]
fn r2c_r_controlled_process_worker() {
    let Ok(phase) = std::env::var("CEDARFLAKE_AME_R2C_R_WORKER_PHASE") else {
        return;
    };
    assert!(matches!(phase.as_str(), "crash-ready" | "recover"));
    let run_nonce = environment("CEDARFLAKE_AME_R2C_R_RUN_NONCE");
    assert!(
        run_nonce.len() == 32
            && run_nonce
                .bytes()
                .all(|value| value.is_ascii_digit() || (b'a'..=b'f').contains(&value)),
        "R2c-R worker nonce is not a lowercase GUID payload"
    );
    let runner_pid = environment_u32("CEDARFLAKE_AME_R2C_R_RUNNER_PID");
    let parent_pid = environment_u32("CEDARFLAKE_AME_R2C_R_PARENT_PID");
    let worker_pid = std::process::id();
    assert_ne!(runner_pid, parent_pid);
    assert_ne!(runner_pid, worker_pid);
    assert_ne!(parent_pid, worker_pid);
    let catalog_path = environment_path("CEDARFLAKE_AME_R2C_R_CATALOG");
    let preview_root = environment_path("CEDARFLAKE_AME_R2C_R_PREVIEWS");
    let settings_path = environment_path("CEDARFLAKE_AME_R2C_R_SETTINGS");
    let source_root = environment_path("CEDARFLAKE_AME_R2C_R_ROOT_PATH");
    let report_path = environment_path("CEDARFLAKE_AME_R2C_R_REPORT");
    for path in [
        catalog_path.as_path(),
        preview_root.as_path(),
        settings_path.as_path(),
        source_root.as_path(),
        report_path.as_path(),
    ] {
        assert_worker_path_is_owned(path);
    }
    let storage = StoragePaths {
        catalog_path,
        preview_root,
        preview_budget_bytes: 64 * 1024 * 1024,
        settings_path,
    };
    let root_id = environment("CEDARFLAKE_AME_R2C_R_ROOT_ID");
    let root_path = environment("CEDARFLAKE_AME_R2C_R_ROOT_PATH");
    let next_usn = environment("CEDARFLAKE_AME_R2C_R_NEXT_USN")
        .parse::<i64>()
        .expect("R2c-R next USN");
    crate::adapters::reset_source_enumeration_instrumentation(&root_path);
    crate::adapters::reset_source_root_entry_enumeration_count(&root_path);
    crate::adapters::reset_source_content_open_instrumentation(&root_path);
    crate::adapters::reset_configured_root_open_instrumentation(&root_path);
    crate::adapters::reset_root_availability_metadata_probe_instrumentation(&root_path);
    let candidate = worker_candidate();
    let (session, query_count, read_count, close_count) =
        controlled_session(&root_id, next_usn, candidate);
    let mut runtime = ProductionSynchronizationTestHarness::with_persistent_journal_session(
        storage.clone(),
        session,
        caller_claim(u8::try_from(next_usn).unwrap_or(0x52)),
    );
    let mut synchronization_cadence =
        CountedProductionSynchronizationCadence::new(runtime.production_cadence());

    synchronization_cadence.wait_for_readiness(READY_TIMEOUT, || {
        runtime.poll().is_ok_and(|snapshot| {
            snapshot.roots.iter().any(|root| {
                root.root_id == root_id
                    && root.source_health == crate::domain::LibraryChangeSourceHealth::Healthy
            })
        })
    });
    let availability_readiness_probes =
        crate::adapters::root_availability_metadata_probe_count(&root_path);
    if phase == "crash-ready" {
        synchronization_cadence.wait_for_crash_ready(READY_TIMEOUT, || {
            runtime.poll().expect("R2c-R no-change boundary poll");
            query_count.load(Ordering::Acquire) >= 1
        });
        assert_eq!(read_count.load(Ordering::Acquire), 0);
        assert_eq!(crate::adapters::source_entry_read_count(&root_path), 0);
        assert_eq!(
            crate::adapters::source_root_entry_enumeration_count(&root_path),
            0
        );
        assert_eq!(crate::adapters::source_content_open_count(&root_path), 0);
        let cadence_counts = synchronization_cadence.invocation_counts();
        assert_eq!(
            cadence_counts,
            ProductionSynchronizationCadenceInvocationCounts {
                readiness: 1,
                crash_ready: 1,
                recovery: 0,
            }
        );
        append_report(
            &report_path,
            &format!(
                "status=passed nonce={run_nonce} runner_pid={runner_pid} parent_pid={parent_pid} worker_pid={worker_pid} phase=crash-ready queries={} reads=0 closes=0 source_entry_reads=0 inventory_entry_reads=0 content_opens=0 availability_metadata_probes={} discovery_root_handles=0 publication_guard_root_handles=0 cadence_readiness_waits={} cadence_crash_ready_waits={} cadence_recovery_waits={}\n",
                query_count.load(Ordering::Acquire),
                crate::adapters::root_availability_metadata_probe_count(&root_path),
                cadence_counts.readiness,
                cadence_counts.crash_ready,
                cadence_counts.recovery,
            ),
        );
        std::mem::forget(runtime);
        return;
    }
    assert_eq!(phase, "recover");
    let observer = SqliteCatalogReadExecutor::open_existing(storage.catalog_path.clone())
        .expect("R2c-R worker read observer");
    let ready = Instant::now();
    synchronization_cadence.wait_for_recovery(VISIBLE_TIMEOUT, || {
        runtime.poll().expect("R2c-R production recovery poll");
        worker_operation_visible(&observer, &root_id).expect("R2c-R visible operation observer")
            && p1_range_completed_from_environment(&observer, &root_id)
                .expect("R2c-R completed P1 observer")
    });
    let ready_to_visible = ready.elapsed();
    runtime.stop().expect("R2c-R production runtime stop");
    assert!(query_count.load(Ordering::Acquire) >= 1);
    assert_eq!(read_count.load(Ordering::Acquire), 1);
    assert_eq!(close_count.load(Ordering::Acquire), 1);
    let cadence_counts = synchronization_cadence.invocation_counts();
    assert_eq!(
        cadence_counts,
        ProductionSynchronizationCadenceInvocationCounts {
            readiness: 1,
            crash_ready: 0,
            recovery: 1,
        }
    );
    append_report(
        &report_path,
        &format!(
            "status=passed nonce={run_nonce} runner_pid={runner_pid} parent_pid={parent_pid} worker_pid={worker_pid} phase=recover kind={} ready_to_visible_us={} queries={} reads={} closes={} source_entry_reads={} inventory_entry_reads={} content_opens={} availability_readiness_probes={} availability_metadata_probes={} discovery_root_handles={} publication_guard_root_handles={} cadence_readiness_waits={} cadence_crash_ready_waits={} cadence_recovery_waits={}\n",
            environment("CEDARFLAKE_AME_R2C_R_OPERATION"),
            ready_to_visible.as_micros(),
            query_count.load(Ordering::Acquire),
            read_count.load(Ordering::Acquire),
            close_count.load(Ordering::Acquire),
            crate::adapters::source_entry_read_count(&root_path),
            crate::adapters::source_root_entry_enumeration_count(&root_path),
            crate::adapters::source_content_open_count(&root_path),
            availability_readiness_probes,
            crate::adapters::root_availability_metadata_probe_count(&root_path),
            crate::adapters::configured_root_open_count(&root_path, true),
            crate::adapters::configured_root_open_count(&root_path, false),
            cadence_counts.readiness,
            cadence_counts.crash_ready,
            cadence_counts.recovery,
        ),
    );
}

fn controlled_session(
    root_id: &str,
    next_usn: i64,
    candidate: Option<BrokerCandidate>,
) -> (
    Arc<dyn PersistentChangeJournalSession>,
    Arc<AtomicUsize>,
    Arc<AtomicUsize>,
    Arc<AtomicUsize>,
) {
    let query_count = Arc::new(AtomicUsize::new(0));
    let read_count = Arc::new(AtomicUsize::new(0));
    let close_count = Arc::new(AtomicUsize::new(0));
    (
        Arc::new(ControlledJournalSession {
            root_id: root_id.to_owned(),
            next_usn,
            candidate,
            query_count: Arc::clone(&query_count),
            read_count: Arc::clone(&read_count),
            close_count: Arc::clone(&close_count),
        }),
        query_count,
        read_count,
        close_count,
    )
}

fn seed_current_checkpoint(
    storage: &StoragePaths,
    source_root: &Path,
    root_id: &str,
    next_usn: i64,
) {
    let generation = LibraryRootGeneration::initial();
    let registration =
        describe_production_persistent_journal_root(root_id, generation.value(), source_root)
            .expect("describe controlled R2c-R root");
    let root_reference =
        JournalFileReference::from_bytes(&registration.authorization.root_identity)
            .expect("R2c-R root file reference");
    let mut catalog =
        SqliteCatalog::open(storage.catalog_path.clone()).expect("R2c-R seed catalog");
    let root = catalog
        .load_incremental_catalog_root(root_id)
        .expect("load R2c-R root")
        .expect("R2c-R root is published");
    let observed_unix_ms = now_unix_ms();
    catalog
        .save_persistent_journal_capability(&PersistentJournalCapability {
            root_id: root_id.to_owned(),
            root_generation: generation,
            protocol_version: crate::journal_broker::PROTOCOL_VERSION,
            contract_version: PERSISTENT_JOURNAL_CONTRACT_VERSION,
            state: PersistentJournalCapabilityState::Supported,
            continuity: PersistentJournalContinuityState::Current,
            failure: None,
            updated_unix_ms: observed_unix_ms,
        })
        .expect("seed R2c-R capability");
    let next_usn = JournalUsn::new(next_usn).expect("R2c-R checkpoint USN");
    catalog
        .seed_persistent_journal_checkpoint_for_test(&PersistentJournalCheckpoint {
            root_id: root_id.to_owned(),
            root_generation: generation,
            volume: PersistentJournalVolumeIdentity {
                volume_guid: registration.authorization.volume_id,
                volume_serial: registration.volume_serial,
            },
            root_file_reference: root_reference,
            journal_id: JournalIdentifier::new(JOURNAL_ID).expect("R2c-R journal ID"),
            next_unread_usn: next_usn,
            captured_exclusive_end: next_usn,
            covered_catalog_revision: root.catalog_revision,
            protocol_version: crate::journal_broker::PROTOCOL_VERSION,
            contract_version: PERSISTENT_JOURNAL_CONTRACT_VERSION,
            continuity: PersistentJournalContinuityState::Current,
            failure: None,
            updated_unix_ms: observed_unix_ms,
        })
        .expect("seed R2c-R checkpoint");
}

fn run_worker(
    fixture: &ControlledFixture,
    phase: &str,
    next_usn: i64,
    operation: Option<(&ClosedOperation, i64)>,
) {
    const WORKER_TIMEOUT: Duration = Duration::from_secs(30);

    let report_line_count_before = report_lines(&fixture.report_path).len();
    let parent_pid = std::process::id();
    let run_nonce = environment("CEDARFLAKE_AME_R2C_R_RUN_NONCE");
    let runner_pid = environment_u32("CEDARFLAKE_AME_R2C_R_RUNNER_PID");
    let mut command = Command::new(std::env::current_exe().expect("R2c-R current test executable"));
    command
        .args([
            "--exact",
            WORKER_TEST,
            "--ignored",
            "--nocapture",
            "--test-threads=1",
        ])
        .env("CEDARFLAKE_AME_R2C_R_WORKER_PHASE", phase)
        .env(
            "CEDARFLAKE_AME_R2C_R_CATALOG",
            &fixture.storage.catalog_path,
        )
        .env(
            "CEDARFLAKE_AME_R2C_R_PREVIEWS",
            &fixture.storage.preview_root,
        )
        .env(
            "CEDARFLAKE_AME_R2C_R_SETTINGS",
            &fixture.storage.settings_path,
        )
        .env("CEDARFLAKE_AME_R2C_R_ROOT_ID", &fixture.root_id)
        .env("CEDARFLAKE_AME_R2C_R_ROOT_PATH", &fixture.root_path)
        .env("CEDARFLAKE_AME_R2C_R_NEXT_USN", next_usn.to_string())
        .env("CEDARFLAKE_AME_R2C_R_REPORT", &fixture.report_path)
        .env("CEDARFLAKE_AME_R2C_R_PARENT_PID", parent_pid.to_string());
    if let Some((operation, start_usn)) = operation {
        let expected_size =
            fs::metadata(fixture.source_root.join(&operation.current_relative_path))
                .map_or(0, |metadata| metadata.len());
        command
            .env("CEDARFLAKE_AME_R2C_R_OPERATION", operation.kind.label())
            .env("CEDARFLAKE_AME_R2C_R_START_USN", start_usn.to_string())
            .env(
                "CEDARFLAKE_AME_R2C_R_CURRENT_PATH",
                &operation.current_relative_path,
            )
            .env(
                "CEDARFLAKE_AME_R2C_R_PREVIOUS_PATH",
                operation.previous_relative_path.as_deref().unwrap_or(""),
            )
            .env(
                "CEDARFLAKE_AME_R2C_R_EXPECTED_SIZE",
                expected_size.to_string(),
            );
    }
    let mut child = command.spawn().expect("launch R2c-R controlled worker");
    let worker_pid = child.id();
    let deadline = Instant::now() + WORKER_TIMEOUT;
    let status = loop {
        if let Some(status) = child.try_wait().expect("poll R2c-R controlled worker") {
            break status;
        }
        if Instant::now() >= deadline {
            child.kill().expect("terminate timed-out R2c-R worker");
            let _ = child.wait();
            panic!(
                "R2c-R controlled worker {worker_pid} exceeded {WORKER_TIMEOUT:?}; the public runner job owns any descendants"
            );
        }
        std::thread::sleep(POLL_INTERVAL);
    };
    assert!(status.success(), "R2c-R controlled worker failed");
    let lines = report_lines(&fixture.report_path);
    let new_lines = lines
        .get(report_line_count_before..)
        .expect("R2c-R worker report append boundary");
    assert_eq!(
        new_lines.len(),
        1,
        "one worker must append exactly one report"
    );
    let line = &new_lines[0];
    assert_eq!(report_field(line, "status"), "passed");
    assert_eq!(report_field(line, "nonce"), run_nonce);
    assert_eq!(report_field(line, "runner_pid"), runner_pid.to_string());
    assert_eq!(report_field(line, "parent_pid"), parent_pid.to_string());
    assert_eq!(report_field(line, "worker_pid"), worker_pid.to_string());
    assert_eq!(report_field(line, "phase"), phase);
    let expected_crash_ready_waits = usize::from(phase == "crash-ready");
    let expected_recovery_waits = usize::from(phase == "recover");
    assert_eq!(report_field(line, "cadence_readiness_waits"), "1");
    assert_eq!(
        report_field(line, "cadence_crash_ready_waits"),
        expected_crash_ready_waits.to_string(),
    );
    assert_eq!(
        report_field(line, "cadence_recovery_waits"),
        expected_recovery_waits.to_string(),
    );
}

fn worker_candidate() -> Option<BrokerCandidate> {
    let operation = std::env::var("CEDARFLAKE_AME_R2C_R_OPERATION").ok()?;
    let start_usn = environment("CEDARFLAKE_AME_R2C_R_START_USN")
        .parse::<i64>()
        .expect("R2c-R start USN");
    let current = environment("CEDARFLAKE_AME_R2C_R_CURRENT_PATH");
    let previous = environment("CEDARFLAKE_AME_R2C_R_PREVIOUS_PATH");
    let is_rename = matches!(operation.as_str(), "rename" | "move");
    Some(BrokerCandidate {
        scope: CandidateScope::RelativePath(current),
        previous_scope: is_rename.then_some(CandidateScope::RelativePath(previous)),
        file_reference: u64::try_from(start_usn)
            .expect("R2c-R file reference")
            .to_le_bytes()
            .to_vec(),
        usn: start_usn,
        kind: if is_rename {
            CandidateKind::Rename
        } else {
            CandidateKind::Path
        },
        is_directory: false,
    })
}

fn worker_operation_visible(
    observer: &SqliteCatalogReadExecutor,
    root_id: &str,
) -> Result<bool, ScanError> {
    let operation = environment("CEDARFLAKE_AME_R2C_R_OPERATION");
    let current_path = environment("CEDARFLAKE_AME_R2C_R_CURRENT_PATH");
    let previous_path = environment("CEDARFLAKE_AME_R2C_R_PREVIOUS_PATH");
    let expected_size = environment("CEDARFLAKE_AME_R2C_R_EXPECTED_SIZE")
        .parse::<u64>()
        .expect("R2c-R expected size");
    let current = observer_location(observer, root_id, &current_path)?;
    if operation == "delete" {
        return Ok(current.is_none());
    }
    let Some(current) = current else {
        return Ok(false);
    };
    Ok(current.file_size == expected_size
        && (previous_path.is_empty()
            || observer_location(observer, root_id, &previous_path)?.is_none()))
}

fn p1_range_completed_from_environment(
    observer: &SqliteCatalogReadExecutor,
    root_id: &str,
) -> Result<bool, ScanError> {
    let start_usn = environment("CEDARFLAKE_AME_R2C_R_START_USN")
        .parse::<i64>()
        .expect("R2c-R start USN");
    let end_usn = environment("CEDARFLAKE_AME_R2C_R_NEXT_USN")
        .parse::<i64>()
        .expect("R2c-R end USN");
    p1_range_completed(observer, root_id, start_usn, end_usn)
}

fn p1_range_completed(
    observer: &SqliteCatalogReadExecutor,
    root_id: &str,
    start_usn: i64,
    end_usn: i64,
) -> Result<bool, ScanError> {
    observer.persistent_journal_range_is_completed_for_test(root_id, start_usn, end_usn)
}

fn assert_p1_range_completed(
    observer: &SqliteCatalogReadExecutor,
    root_id: &str,
    start_usn: i64,
    end_usn: i64,
) {
    assert!(
        p1_range_completed(observer, root_id, start_usn, end_usn)
            .expect("R2c-R completed P1 evidence"),
        "P1 range {start_usn}..{end_usn} did not retain completed source-range and queue lineage"
    );
}

fn source_snapshot(root: &Path) -> Vec<(String, Vec<u8>)> {
    fn visit(root: &Path, directory: &Path, entries: &mut Vec<(String, Vec<u8>)>) {
        for child in fs::read_dir(directory).expect("R2c-R source snapshot directory") {
            let child = child.expect("R2c-R source snapshot entry");
            let path = child.path();
            let metadata = child.metadata().expect("R2c-R source snapshot metadata");
            if metadata.is_dir() {
                visit(root, &path, entries);
            } else if metadata.is_file() {
                entries.push((
                    path.strip_prefix(root)
                        .expect("R2c-R snapshot containment")
                        .to_string_lossy()
                        .replace('\\', "/"),
                    fs::read(path).expect("R2c-R source snapshot bytes"),
                ));
            }
        }
    }
    let mut entries = Vec::new();
    visit(root, root, &mut entries);
    entries.sort_by(|left, right| left.0.cmp(&right.0));
    entries
}

fn operation_source_snapshot(
    root: &Path,
    operation: &ClosedOperation,
) -> Vec<(String, Option<Vec<u8>>)> {
    let mut relative_paths = vec![operation.current_relative_path.clone()];
    if let Some(previous_relative_path) = &operation.previous_relative_path {
        relative_paths.push(previous_relative_path.clone());
    }
    relative_paths.sort_unstable();
    relative_paths.dedup();
    source_paths_snapshot(root, &relative_paths)
}

fn source_paths_snapshot(root: &Path, relative_paths: &[String]) -> Vec<(String, Option<Vec<u8>>)> {
    let mut entries = relative_paths
        .iter()
        .map(|relative_path| {
            let path = root.join(relative_path);
            let bytes = match fs::read(&path) {
                Ok(bytes) => Some(bytes),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
                Err(error) => panic!("R2c-R targeted source snapshot {relative_path}: {error}"),
            };
            (relative_path.clone(), bytes)
        })
        .collect::<Vec<_>>();
    entries.sort_unstable_by(|left, right| left.0.cmp(&right.0));
    entries
}

fn report_latencies(report_path: &Path) -> Vec<Duration> {
    report_lines(report_path)
        .into_iter()
        .filter(|line| report_field(line, "phase") == "recover")
        .map(|line| {
            let micros = line
                .split_whitespace()
                .find_map(|field| field.strip_prefix("ready_to_visible_us="))
                .expect("R2c-R latency field")
                .parse::<u64>()
                .expect("R2c-R latency value");
            Duration::from_micros(micros)
        })
        .collect()
}

fn report_metric_sum(report_path: &Path, metric: &str) -> u64 {
    report_metric_values(report_path, metric).into_iter().sum()
}

fn report_metric_values(report_path: &Path, metric: &str) -> Vec<u64> {
    let prefix = format!("{metric}=");
    report_lines(report_path)
        .into_iter()
        .filter(|line| report_field(line, "phase") == "recover")
        .map(|line| {
            line.split_whitespace()
                .find_map(|field| field.strip_prefix(&prefix))
                .unwrap_or_else(|| panic!("missing R2c-R report metric {metric}"))
                .parse::<u64>()
                .unwrap_or_else(|_| panic!("invalid R2c-R report metric {metric}"))
        })
        .collect()
}

fn report_lines(report_path: &Path) -> Vec<String> {
    match fs::read_to_string(report_path) {
        Ok(contents) => contents.lines().map(str::to_owned).collect(),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Vec::new(),
        Err(error) => panic!("read R2c-R report: {error}"),
    }
}

fn report_field<'a>(line: &'a str, name: &str) -> &'a str {
    let prefix = format!("{name}=");
    let mut values = line
        .split_whitespace()
        .filter_map(|field| field.strip_prefix(&prefix));
    let value = values
        .next()
        .unwrap_or_else(|| panic!("missing R2c-R report field {name}"));
    assert!(
        values.next().is_none(),
        "ambiguous R2c-R report field {name}"
    );
    value
}

#[test]
fn r2c_r_worker_report_fields_reject_missing_or_ambiguous_bindings() {
    assert!(
        std::panic::catch_unwind(|| report_field("status=passed phase=recover", "nonce")).is_err()
    );
    assert!(
        std::panic::catch_unwind(|| {
            report_field(
                "status=passed nonce=first nonce=second phase=recover",
                "nonce",
            )
        })
        .is_err()
    );
    assert_eq!(
        report_field("status=passed nonce=only phase=recover", "nonce"),
        "only"
    );
}

fn percentile(samples: &[Duration], percent: usize) -> Duration {
    let index = (samples.len() * percent).div_ceil(100).saturating_sub(1);
    samples[index]
}

fn scan_row_count(catalog_path: &Path) -> u64 {
    let connection = Connection::open(catalog_path).expect("R2c-R scan-row catalog");
    let count: i64 = connection
        .query_row("SELECT COUNT(*) FROM scan_runs", [], |row| row.get(0))
        .expect("R2c-R scan-row count");
    u64::try_from(count).expect("R2c-R nonnegative scan-row count")
}

fn p0_queue_evidence(catalog_path: &Path, root_id: &str) -> (u64, u64) {
    let connection = Connection::open(catalog_path).expect("R2c-R P0 evidence catalog");
    let evidence = connection
        .query_row(
            "SELECT COUNT(*), COALESCE(SUM(queue.coalesced_observation_count), 0)
             FROM library_change_queue AS queue
             JOIN library_change_queue_lanes AS lane ON lane.change_id = queue.id
             WHERE queue.root_id = ?1 AND lane.lane = 'p0_live'",
            [root_id],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
        )
        .expect("R2c-R P0 queue evidence");
    (
        u64::try_from(evidence.0).expect("R2c-R nonnegative P0 rows"),
        u64::try_from(evidence.1).expect("R2c-R nonnegative P0 observations"),
    )
}

fn p2_queue_count(catalog_path: &Path, root_id: &str) -> u64 {
    let connection = Connection::open(catalog_path).expect("R2c-R P2 evidence catalog");
    let count: i64 = connection
        .query_row(
            "SELECT COUNT(*)
             FROM library_change_queue AS queue
             JOIN library_change_queue_lanes AS lane ON lane.change_id = queue.id
             WHERE queue.root_id = ?1 AND lane.lane = 'p2_recovery'",
            [root_id],
            |row| row.get(0),
        )
        .expect("R2c-R P2 queue count");
    u64::try_from(count).expect("R2c-R nonnegative P2 count")
}

struct P2QueueMetrics {
    control_rows: u64,
    owned_candidate_rows: u64,
    total_rows: u64,
}

fn p2_queue_metrics(catalog_path: &Path, root_id: &str) -> P2QueueMetrics {
    let connection = Connection::open(catalog_path).expect("R2c-R P2 metrics catalog");
    let metrics = connection
        .query_row(
            "SELECT
               COUNT(DISTINCT control.change_id),
               COUNT(DISTINCT CASE
                 WHEN owner_authority.change_id IS NOT NULL THEN owner.change_id
               END),
               COUNT(DISTINCT queue.id)
             FROM library_change_queue AS queue
             JOIN library_change_queue_lanes AS lane ON lane.change_id = queue.id
             LEFT JOIN library_recovery_authorities AS control
               ON control.change_id = queue.id
              AND control.reason = 'watcher_uncovered_gap'
             LEFT JOIN library_metadata_inventory_candidate_owners AS owner
               ON owner.change_id = queue.id
             LEFT JOIN library_recovery_authorities AS owner_authority
               ON owner_authority.run_id = owner.run_id
              AND owner_authority.root_id = queue.root_id
              AND owner_authority.root_generation = queue.root_generation
              AND owner_authority.reason = 'watcher_uncovered_gap'
             WHERE queue.root_id = ?1 AND lane.lane = 'p2_recovery'",
            [root_id],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            },
        )
        .expect("R2c-R P2 queue metrics");
    P2QueueMetrics {
        control_rows: u64::try_from(metrics.0).expect("R2c-R nonnegative P2 controls"),
        owned_candidate_rows: u64::try_from(metrics.1)
            .expect("R2c-R nonnegative P2 owned candidates"),
        total_rows: u64::try_from(metrics.2).expect("R2c-R nonnegative P2 total"),
    }
}

fn inventory_run_count(catalog_path: &Path) -> u64 {
    let connection = Connection::open(catalog_path).expect("R2c-R inventory catalog");
    let count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM library_metadata_inventory_runs",
            [],
            |row| row.get(0),
        )
        .expect("R2c-R inventory count");
    u64::try_from(count).expect("R2c-R nonnegative inventory count")
}

struct InventoryMetrics {
    run_count: u32,
    maximum_staged_entries: u64,
    maximum_next_page_index: u64,
    total_candidates: u64,
}

fn inventory_metrics(catalog_path: &Path, root_id: &str) -> InventoryMetrics {
    let connection = Connection::open(catalog_path).expect("R2c-R inventory metrics catalog");
    let metrics = connection
        .query_row(
            "SELECT COUNT(*), COALESCE(MAX(staged_entry_count), 0),
                    COALESCE(MAX(next_page_index), 0), COALESCE(SUM(candidate_count), 0)
             FROM library_metadata_inventory_runs WHERE root_id = ?1",
            [root_id],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                ))
            },
        )
        .expect("R2c-R inventory metrics");
    InventoryMetrics {
        run_count: u32::try_from(metrics.0).expect("R2c-R bounded inventory run count"),
        maximum_staged_entries: u64::try_from(metrics.1).expect("R2c-R nonnegative staged entries"),
        maximum_next_page_index: u64::try_from(metrics.2)
            .expect("R2c-R nonnegative inventory page index"),
        total_candidates: u64::try_from(metrics.3).expect("R2c-R nonnegative inventory candidates"),
    }
}

fn watcher_recovery_observation(
    observer: &SqliteCatalogReadExecutor,
    root_id: &str,
) -> Result<WatcherRecoveryObservation, ScanError> {
    observer.load_watcher_recovery_observation_for_test(root_id)
}

fn observer_location(
    observer: &SqliteCatalogReadExecutor,
    root_id: &str,
    relative_path: &str,
) -> Result<Option<AssetLocationView>, ScanError> {
    observer.load_incremental_location_by_relative_path_for_test(root_id, relative_path)
}

fn all_overflow_locations_visible(
    observer: &SqliteCatalogReadExecutor,
    root_id: &str,
    count: usize,
) -> Result<bool, ScanError> {
    let relative_paths = (0..count)
        .map(|index| format!("overflow/item-{index:04}.png"))
        .collect::<Vec<_>>();
    Ok(observer
        .load_incremental_locations_by_relative_paths_for_test(root_id, &relative_paths)?
        .len()
        == count)
}

fn file_state_changed(previous: &AssetLocationView, current: &AssetLocationView) -> bool {
    previous.file_size != current.file_size
        || previous.modified_unix_ms != current.modified_unix_ms
        || previous.file_identity != current.file_identity
}

fn append_report(path: &Path, line: &str) {
    let mut report = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .expect("open R2c-R report");
    report
        .write_all(line.as_bytes())
        .expect("append R2c-R report");
    report.flush().expect("flush R2c-R report");
}

fn write_png(path: &Path, width: u32, height: u32, seed: u8) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("R2c-R PNG parent");
    }
    RgbImage::from_pixel(width, height, Rgb([seed, seed.wrapping_add(37), 211]))
        .save(path)
        .expect("write R2c-R PNG");
}

fn wait_until(timeout: Duration, mut predicate: impl FnMut() -> bool) {
    let deadline = Instant::now() + timeout;
    while !predicate() {
        assert!(Instant::now() < deadline, "R2c-R wait exceeded {timeout:?}");
        std::thread::sleep(POLL_INTERVAL);
    }
}

fn wait_until_result(
    timeout: Duration,
    mut predicate: impl FnMut() -> Result<bool, ScanError>,
) -> Result<(), ScanError> {
    let deadline = Instant::now() + timeout;
    while !predicate()? {
        if Instant::now() >= deadline {
            return Err(ScanError::new(
                "r2c_r_wait_timeout",
                format!("R2c-R result wait exceeded {timeout:?}"),
            ));
        }
        std::thread::sleep(POLL_INTERVAL);
    }
    Ok(())
}

fn caller_claim(seed: u8) -> CallerClaim {
    CallerClaim {
        process_id: std::process::id(),
        session_id: 1,
        client_instance: [seed.max(1); 16],
    }
}

fn now_unix_ms() -> i64 {
    i64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("R2c-R system clock")
            .as_millis(),
    )
    .expect("R2c-R representable system clock")
}

fn environment(name: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| panic!("missing R2c-R environment value {name}"))
}

fn environment_path(name: &str) -> PathBuf {
    PathBuf::from(environment(name))
}

fn environment_u32(name: &str) -> u32 {
    environment(name)
        .parse::<u32>()
        .unwrap_or_else(|_| panic!("invalid R2c-R process identifier {name}"))
}

fn assert_worker_path_is_owned(path: &Path) {
    assert!(path.is_absolute(), "R2c-R worker path must be absolute");
    assert!(
        !path
            .components()
            .any(|component| matches!(component, std::path::Component::ParentDir)),
        "R2c-R worker path must not contain a parent traversal"
    );
    let owned_root = environment_path("CEDARFLAKE_AME_R2C_R_OWNED_ROOT")
        .canonicalize()
        .expect("canonical R2c-R runner-owned root");
    let existing_ancestor = path
        .ancestors()
        .find(|ancestor| ancestor.exists())
        .expect("R2c-R worker path existing ancestor")
        .canonicalize()
        .expect("canonical R2c-R worker path ancestor");
    assert_ne!(existing_ancestor, owned_root);
    assert!(
        existing_ancestor.starts_with(&owned_root),
        "R2c-R worker path escaped the runner-owned root"
    );
    if path.exists() {
        let canonical = path.canonicalize().expect("canonical R2c-R worker path");
        assert_ne!(canonical, owned_root);
        assert!(
            canonical.starts_with(&owned_root),
            "R2c-R worker path physically escaped the runner-owned root"
        );
    }
}

fn owned_test_directory() -> tempfile::TempDir {
    let owned_root = environment_path("CEDARFLAKE_AME_R2C_R_OWNED_ROOT")
        .canonicalize()
        .expect("canonical R2c-R runner-owned root");
    let directory = tempfile::tempdir_in(&owned_root).expect("R2c-R owned disposable directory");
    let canonical_directory = directory
        .path()
        .canonicalize()
        .expect("canonical R2c-R disposable directory");
    assert_ne!(canonical_directory, owned_root);
    assert!(
        canonical_directory.starts_with(&owned_root),
        "R2c-R fixture escaped the runner-owned root"
    );
    directory
}
