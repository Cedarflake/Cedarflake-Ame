#[cfg(windows)]
mod first_import;
#[cfg(windows)]
mod indexed_startup;
#[cfg(windows)]
mod inventory_pages;
#[cfg(windows)]
mod journal_baseline;
#[cfg(windows)]
mod legacy_recovery_debt;
#[cfg(windows)]
mod live_gap;
#[cfg(windows)]
mod priority_admission;
#[cfg(windows)]
mod publication_guards;
#[cfg(windows)]
mod recovery_authority;
#[cfg(windows)]
mod recovery_fairness;
#[cfg(windows)]
mod recovery_status;
#[cfg(windows)]
mod registration_recovery;
#[cfg(windows)]
mod startup;

#[cfg(windows)]
pub(super) mod live_worker;

#[cfg(windows)]
mod priority;
#[cfg(windows)]
mod retained_gap;

#[cfg(windows)]
mod priority_journal;

#[cfg(windows)]
mod ingress_reservation;

#[cfg(windows)]
mod root_availability;

#[cfg(windows)]
mod root_location_recovery;

#[cfg(windows)]
use super::runtime_test_support::*;
use super::*;
#[cfg(windows)]
use crate::domain::{LibraryChangeQueueHealth, ScanRequest};
#[cfg(windows)]
use crate::ports::{CatalogRepository, LibraryChangeQueue};
#[cfg(windows)]
use priority_journal::PriorityJournalSession;
#[cfg(windows)]
use rusqlite::OptionalExtension;

#[cfg(windows)]
#[derive(Clone, Default)]
struct QueuedSourceFactory {
    batches: Arc<
        Mutex<
            std::collections::BTreeMap<
                String,
                std::collections::VecDeque<crate::domain::LibraryChangeSourceBatch>,
            >,
        >,
    >,
}

#[cfg(windows)]
struct QueuedSource {
    root_id: String,
    batches: Arc<
        Mutex<
            std::collections::BTreeMap<
                String,
                std::collections::VecDeque<crate::domain::LibraryChangeSourceBatch>,
            >,
        >,
    >,
}

#[cfg(windows)]
struct NoChangeJournalSession {
    register_count: Arc<std::sync::atomic::AtomicUsize>,
    query_count: Arc<std::sync::atomic::AtomicUsize>,
    shared_read_count: Arc<std::sync::atomic::AtomicUsize>,
    close_count: Arc<std::sync::atomic::AtomicUsize>,
    client_instances: Arc<Mutex<Vec<[u8; 16]>>>,
}

#[cfg(windows)]
struct AdvancingJournalSession {
    boundaries: Mutex<std::collections::VecDeque<i64>>,
    query_count: Arc<std::sync::atomic::AtomicUsize>,
    shared_read_count: Arc<std::sync::atomic::AtomicUsize>,
    read_release: Arc<AtomicBool>,
    close_count: Arc<std::sync::atomic::AtomicUsize>,
}

#[cfg(windows)]
struct CompletedJournalRead {
    response: BrokerResponse,
    release: Arc<AtomicBool>,
}

#[cfg(windows)]
#[derive(Clone)]
struct OrderingProbeFactory {
    events: Arc<Mutex<Vec<&'static str>>>,
}

#[cfg(windows)]
struct OrderingProbeSource;

#[cfg(windows)]
impl PersistentChangeJournal for OrderingProbeFactory {
    fn connect(&self) -> PersistentChangeJournalConnection {
        self.events.lock().expect("ordering events").push("journal");
        PersistentChangeJournalConnection::LiveOnly(
            PersistentChangeJournalLiveOnlyReason::BrokerAbsent,
        )
    }
}

#[cfg(windows)]
impl PersistentChangeJournalSession for NoChangeJournalSession {
    fn register_root(
        &self,
        request: RegisterRootRequest,
    ) -> Result<RootCapability, PersistentChangeJournalOperationError> {
        self.register_count.fetch_add(1, Ordering::AcqRel);
        self.client_instances
            .lock()
            .expect("no-change client instances")
            .push(request.caller.client_instance);
        Ok(RootCapability([7; 32]))
    }

    fn query_journal(
        &self,
        request: QueryJournalRequest,
    ) -> Result<BrokerResponse, PersistentChangeJournalOperationError> {
        self.query_count.fetch_add(1, Ordering::AcqRel);
        Ok(BrokerResponse::Journal {
            request_id: 1,
            binding: crate::journal_broker::ResponseBinding::from_request(
                &request.caller,
                &request.root,
            ),
            capability: crate::journal_broker::JournalCapability::Supported,
            journal_id: Some(44),
            first_usn: Some(1),
            next_usn: Some(20),
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
        let root = request
            .roots
            .first()
            .ok_or(PersistentChangeJournalOperationError::InvalidRequest)?;
        if request
            .roots
            .iter()
            .any(|root| root.start_usn != request.end_usn)
        {
            return Err(PersistentChangeJournalOperationError::InvalidRequest);
        }
        Ok(Box::new(CompletedJournalRead {
            response: BrokerResponse::ReadVolume {
                request_id: 1,
                client_instance: request.caller.client_instance,
                volume_id: root.root.volume_id.clone(),
                journal_id: request.journal_id,
                requested_end_usn: request.end_usn,
                max_records: request.max_records,
                max_evidence_bytes: request.max_evidence_bytes,
                outcomes: request
                    .roots
                    .iter()
                    .map(|root| crate::journal_broker::SharedJournalRootOutcome {
                        binding: crate::journal_broker::ResponseBinding::from_request(
                            &request.caller,
                            &root.root,
                        ),
                        requested_start_usn: root.start_usn,
                        covered_until_usn: Some(request.end_usn),
                        is_complete: true,
                        candidates: Vec::new(),
                        failure: None,
                    })
                    .collect(),
                handoffs: Vec::new(),
                pending_renames: Vec::new(),
            },
            release: Arc::new(AtomicBool::new(true)),
        }))
    }

    fn close(&self) -> Result<(), PersistentChangeJournalOperationError> {
        self.close_count.fetch_add(1, Ordering::AcqRel);
        Ok(())
    }
}

#[cfg(windows)]
impl PersistentChangeJournalSession for AdvancingJournalSession {
    fn register_root(
        &self,
        _request: RegisterRootRequest,
    ) -> Result<RootCapability, PersistentChangeJournalOperationError> {
        Ok(RootCapability([6; 32]))
    }

    fn query_journal(
        &self,
        request: QueryJournalRequest,
    ) -> Result<BrokerResponse, PersistentChangeJournalOperationError> {
        self.query_count.fetch_add(1, Ordering::AcqRel);
        let next_usn = self
            .boundaries
            .lock()
            .expect("journal boundaries")
            .pop_front()
            .unwrap_or(21);
        Ok(BrokerResponse::Journal {
            request_id: 1,
            binding: crate::journal_broker::ResponseBinding::from_request(
                &request.caller,
                &request.root,
            ),
            capability: JournalCapability::Supported,
            journal_id: Some(44),
            first_usn: Some(1),
            next_usn: Some(next_usn),
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
        let root = request
            .roots
            .first()
            .ok_or(PersistentChangeJournalOperationError::InvalidRequest)?;
        Ok(Box::new(CompletedJournalRead {
            response: BrokerResponse::ReadVolume {
                request_id: 1,
                client_instance: request.caller.client_instance,
                volume_id: root.root.volume_id.clone(),
                journal_id: request.journal_id,
                requested_end_usn: request.end_usn,
                max_records: request.max_records,
                max_evidence_bytes: request.max_evidence_bytes,
                outcomes: vec![crate::journal_broker::SharedJournalRootOutcome {
                    binding: crate::journal_broker::ResponseBinding::from_request(
                        &request.caller,
                        &root.root,
                    ),
                    requested_start_usn: root.start_usn,
                    covered_until_usn: Some(request.end_usn),
                    is_complete: true,
                    candidates: Vec::new(),
                    failure: None,
                }],
                handoffs: Vec::new(),
                pending_renames: Vec::new(),
            },
            release: Arc::clone(&self.read_release),
        }))
    }

    fn close(&self) -> Result<(), PersistentChangeJournalOperationError> {
        self.close_count.fetch_add(1, Ordering::AcqRel);
        Ok(())
    }
}

#[cfg(windows)]
impl PersistentChangeJournalRead for CompletedJournalRead {
    fn cancel(
        &self,
        _caller: crate::journal_broker::CallerClaim,
    ) -> Result<(), PersistentChangeJournalOperationError> {
        self.release.store(true, Ordering::Release);
        Ok(())
    }

    fn wait(self: Box<Self>) -> Result<BrokerResponse, PersistentChangeJournalOperationError> {
        while !self.release.load(Ordering::Acquire) {
            std::thread::yield_now();
        }
        Ok(self.response)
    }
}

#[cfg(windows)]
impl crate::ports::LibraryChangeSourceFactory for QueuedSourceFactory {
    type Source = QueuedSource;

    fn start(
        &self,
        request: &crate::ports::LibraryChangeSourceRequest,
    ) -> Result<Self::Source, crate::domain::LibraryChangeSourceError> {
        Ok(QueuedSource {
            root_id: request.root_id.clone(),
            batches: Arc::clone(&self.batches),
        })
    }
}

#[cfg(windows)]
impl crate::ports::LibraryChangeSourceFactory for OrderingProbeFactory {
    type Source = OrderingProbeSource;

    fn start(
        &self,
        _request: &crate::ports::LibraryChangeSourceRequest,
    ) -> Result<Self::Source, crate::domain::LibraryChangeSourceError> {
        self.events.lock().expect("ordering events").push("watcher");
        Ok(OrderingProbeSource)
    }
}

#[cfg(windows)]
impl crate::ports::LibraryChangeSource for QueuedSource {
    fn health(&self) -> crate::domain::LibraryChangeSourceHealth {
        crate::domain::LibraryChangeSourceHealth::Healthy
    }

    fn drain(
        &mut self,
        _max_observations: usize,
    ) -> Result<crate::domain::LibraryChangeSourceBatch, crate::domain::LibraryChangeSourceError>
    {
        Ok(self
            .batches
            .lock()
            .expect("queued source batches")
            .entry(self.root_id.clone())
            .or_default()
            .pop_front()
            .unwrap_or(crate::domain::LibraryChangeSourceBatch {
                observations: Vec::new(),
                health: crate::domain::LibraryChangeSourceHealth::Healthy,
                dropped_observation_count: 0,
                ignored_callback_count: 0,
                last_issue_code: None,
            }))
    }

    fn stop(
        &mut self,
    ) -> Result<crate::domain::LibraryChangeSourceStopReport, crate::domain::LibraryChangeSourceError>
    {
        Ok(crate::domain::LibraryChangeSourceStopReport::default())
    }
}

#[cfg(windows)]
impl crate::ports::LibraryChangeSource for OrderingProbeSource {
    fn health(&self) -> crate::domain::LibraryChangeSourceHealth {
        crate::domain::LibraryChangeSourceHealth::Healthy
    }

    fn drain(
        &mut self,
        _max_observations: usize,
    ) -> Result<crate::domain::LibraryChangeSourceBatch, crate::domain::LibraryChangeSourceError>
    {
        Ok(crate::domain::LibraryChangeSourceBatch {
            observations: Vec::new(),
            health: crate::domain::LibraryChangeSourceHealth::Healthy,
            dropped_observation_count: 0,
            ignored_callback_count: 0,
            last_issue_code: None,
        })
    }

    fn stop(
        &mut self,
    ) -> Result<crate::domain::LibraryChangeSourceStopReport, crate::domain::LibraryChangeSourceError>
    {
        Ok(crate::domain::LibraryChangeSourceStopReport::default())
    }
}

#[cfg(windows)]
pub(super) struct ProductionGapFixture {
    _directory: tempfile::TempDir,
    pub(super) source_root: std::path::PathBuf,
    pub(super) root_id: String,
    pub(super) storage: crate::application::storage::StoragePaths,
    scan_rows_before: i64,
}

#[cfg(windows)]
#[derive(Debug)]
struct DurableGapRow {
    change_id: i64,
    intent_kind: String,
    scope: String,
    relative_path: String,
    origin: String,
    lane: String,
    status: String,
    first_sequence: String,
    most_recent_sequence: String,
    coalesced_observation_count: i64,
}

#[cfg(windows)]
impl ProductionGapFixture {
    fn new(name: &str, source_entry_count: usize) -> Self {
        let directory = tempfile::tempdir().expect("production gap test directory");
        let source_root = directory.path().join("source");
        std::fs::create_dir_all(&source_root).expect("production gap source root");
        for index in 0..source_entry_count {
            std::fs::write(
                source_root.join(format!("sentinel-{index:04}.txt")),
                b"metadata-only recovery fixture",
            )
            .expect("production gap source entry");
        }
        let discovery = crate::adapters::FileDiscovery::new(&source_root.to_string_lossy())
            .expect("production gap root discovery");
        let root_path = source_root.to_string_lossy().into_owned();
        let publication_identity = discovery
            .metadata_inventory_root_identity()
            .expect("production gap root identity query")
            .expect("production gap stable root identity");
        let root_id = crate::application::scan_library::stable_id("library-root-v1", &root_path);
        let storage = crate::application::storage::StoragePaths {
            catalog_path: directory.path().join("catalog").join("ame.sqlite3"),
            preview_root: directory.path().join("previews"),
            preview_budget_bytes: 64 * 1024 * 1024,
            settings_path: directory.path().join("settings").join("storage.sqlite3"),
        };
        let request = ScanRequest {
            scan_id: format!("initial-{name}"),
            root_path: root_path.clone(),
            max_items: None,
            max_entries: None,
            preview_edge: 512,
        };
        let mut catalog =
            SqliteCatalog::open(storage.catalog_path.clone()).expect("production gap catalog");
        catalog
            .begin_scan_with_publication_namespace(
                &request,
                &root_id,
                &root_path,
                &publication_identity,
            )
            .expect("begin production gap fixture scan");
        catalog
            .prove_live_only_first_import_handoff_for_test(&request.scan_id)
            .expect("prove production gap fixture first-import handoff");
        catalog
            .publish_scan(&request.scan_id, &root_id, 0, 0)
            .expect("publish production gap fixture scan");
        drop(catalog);
        let connection = rusqlite::Connection::open(&storage.catalog_path)
            .expect("open production gap assertion catalog");
        let scan_rows_before = connection
            .query_row("SELECT COUNT(*) FROM scan_runs", [], |row| row.get(0))
            .expect("count production gap scan rows");
        drop(connection);
        Self {
            _directory: directory,
            source_root,
            root_id,
            storage,
            scan_rows_before,
        }
    }

    pub(super) fn new_with_media_baseline(name: &str) -> Self {
        let directory = tempfile::tempdir().expect("media gap test directory");
        let source_root = directory.path().join("source");
        std::fs::create_dir_all(&source_root).expect("media gap source root");
        image::RgbImage::from_pixel(2, 2, image::Rgb([20, 40, 80]))
            .save_with_format(source_root.join("unchanged.png"), image::ImageFormat::Png)
            .expect("unchanged media baseline");
        image::RgbImage::from_pixel(2, 2, image::Rgb([80, 40, 20]))
            .save_with_format(source_root.join("removed.png"), image::ImageFormat::Png)
            .expect("removed media baseline");
        let discovery = crate::adapters::FileDiscovery::new(&source_root.to_string_lossy())
            .expect("media gap root discovery");
        let root_path = discovery
            .canonical_root()
            .expect("canonical media gap root")
            .to_string_lossy()
            .into_owned();
        let root_id = crate::application::scan_library::stable_id("library-root-v1", &root_path);
        let storage = crate::application::storage::StoragePaths {
            catalog_path: directory.path().join("catalog").join("ame.sqlite3"),
            preview_root: directory.path().join("previews"),
            preview_budget_bytes: 64 * 1024 * 1024,
            settings_path: directory.path().join("settings").join("storage.sqlite3"),
        };
        crate::application::scan_library::run_scan_with_storage(
            ScanRequest {
                scan_id: format!("initial-{name}"),
                root_path,
                max_items: None,
                max_entries: None,
                preview_edge: 512,
            },
            |_| true,
            storage.clone(),
        )
        .expect("publish explicit media baseline");
        let catalog =
            SqliteCatalog::open(storage.catalog_path.clone()).expect("open media baseline catalog");
        assert!(
            catalog
                .load_incremental_location_by_relative_path(&root_id, "unchanged.png")
                .expect("load unchanged media baseline")
                .is_some()
        );
        assert!(
            catalog
                .load_incremental_location_by_relative_path(&root_id, "removed.png")
                .expect("load removable media baseline")
                .is_some()
        );
        drop(catalog);
        let connection = rusqlite::Connection::open(&storage.catalog_path)
            .expect("open media baseline assertion catalog");
        let scan_rows_before = connection
            .query_row("SELECT COUNT(*) FROM scan_runs", [], |row| row.get(0))
            .expect("count media baseline scan rows");
        drop(connection);
        Self {
            _directory: directory,
            source_root,
            root_id,
            storage,
            scan_rows_before,
        }
    }

    fn observer_request(&self) -> crate::ports::LibraryChangeSourceRequest {
        crate::ports::LibraryChangeSourceRequest {
            root_id: self.root_id.clone(),
            root_generation: LibraryRootGeneration::initial(),
            root_path: self.source_root.clone(),
            ingress_capacity: 16,
        }
    }

    fn assert_no_automatic_full_scan(&self) {
        let connection = rusqlite::Connection::open(&self.storage.catalog_path)
            .expect("open full-scan assertion catalog");
        let scan_rows_after: i64 = connection
            .query_row("SELECT COUNT(*) FROM scan_runs", [], |row| row.get(0))
            .expect("count final production gap scan rows");
        assert_eq!(scan_rows_after, self.scan_rows_before);
    }
}

#[cfg(windows)]
fn fast_gap_runtime(
    factory: QueuedSourceFactory,
    connection: PersistentChangeJournalConnection,
) -> ProductionSynchronization {
    let mut production = new_production_synchronization_with_connection(
        crate::ports::erase_library_change_source_factory(factory),
        connection,
    );
    production.runtime.queue_policy = crate::domain::LibraryChangeQueuePolicy {
        debounce_millis: 0,
        retry_initial_delay_millis: 1,
        retry_maximum_delay_millis: 8,
        ..crate::domain::LibraryChangeQueuePolicy::default()
    };
    production.runtime.restart_policy = crate::domain::LibraryChangeRestartPolicy {
        initial_delay_millis: 1,
        maximum_delay_millis: 8,
    };
    production
}

#[cfg(windows)]
fn push_gap_batch(
    factory: &QueuedSourceFactory,
    root_id: &str,
    batch: crate::domain::LibraryChangeSourceBatch,
) {
    factory
        .batches
        .lock()
        .expect("queued production gap batches")
        .entry(root_id.to_owned())
        .or_default()
        .push_back(batch);
}

#[cfg(windows)]
fn wait_for_durable_gap(
    production: &mut ProductionSynchronization,
    fixture: &ProductionGapFixture,
) -> DurableGapRow {
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        poll_runtime_with_storage(production, &fixture.storage)
            .expect("poll production gap ingress");
        let connection = rusqlite::Connection::open(&fixture.storage.catalog_path)
            .expect("open durable gap assertion catalog");
        let row = connection
            .query_row(
                "SELECT queue.id, queue.intent_kind, queue.scope, queue.relative_path,
                        queue.origin, lanes.lane, queue.status, queue.first_sequence,
                        queue.most_recent_sequence, queue.coalesced_observation_count
                 FROM library_change_queue AS queue
                 JOIN library_change_queue_lanes AS lanes ON lanes.change_id = queue.id
                 WHERE queue.root_id = ?1 AND queue.root_generation = 1
                   AND queue.intent_kind = 'freshness_unknown' AND queue.scope = 'root'
                   AND queue.origin = 'live_notification' AND lanes.lane = 'p0_live'
                 ORDER BY queue.id DESC LIMIT 1",
                [&fixture.root_id],
                |row| {
                    Ok(DurableGapRow {
                        change_id: row.get(0)?,
                        intent_kind: row.get(1)?,
                        scope: row.get(2)?,
                        relative_path: row.get(3)?,
                        origin: row.get(4)?,
                        lane: row.get(5)?,
                        status: row.get(6)?,
                        first_sequence: row.get(7)?,
                        most_recent_sequence: row.get(8)?,
                        coalesced_observation_count: row.get(9)?,
                    })
                },
            )
            .optional()
            .expect("load durable production gap");
        if let Some(row) = row {
            return row;
        }
        assert!(
            Instant::now() < deadline,
            "production observer did not persist its evidence gap"
        );
        thread::sleep(Duration::from_millis(2));
    }
}

#[cfg(windows)]
fn assert_p0_live_gap(row: &DurableGapRow) {
    assert_eq!(row.intent_kind, "freshness_unknown");
    assert_eq!(row.scope, "root");
    assert_eq!(row.relative_path, "");
    assert_eq!(row.origin, "live_notification");
    assert_eq!(row.lane, "p0_live");
    assert!(matches!(
        row.status.as_str(),
        "pending" | "leased" | "retry_wait" | "superseded"
    ));
    let first_sequence = row
        .first_sequence
        .parse::<u64>()
        .expect("canonical first sequence");
    let most_recent_sequence = row
        .most_recent_sequence
        .parse::<u64>()
        .expect("canonical most-recent sequence");
    assert!(first_sequence > 0);
    assert!(most_recent_sequence >= first_sequence);
    assert!(row.coalesced_observation_count > 0);
}

#[cfg(windows)]
fn drive_gap_to_synchronized(
    production: &mut ProductionSynchronization,
    fixture: &ProductionGapFixture,
) -> LibrarySynchronizationSnapshot {
    let deadline = Instant::now() + Duration::from_secs(8);
    loop {
        let snapshot = poll_runtime_with_storage(production, &fixture.storage)
            .expect("drive production gap recovery");
        let status = snapshot.roots.first().expect("production gap root status");
        let catalog = SqliteCatalog::open(fixture.storage.catalog_path.clone())
            .expect("open production gap metrics catalog");
        let metrics = catalog
            .load_library_change_root_queue_metrics(
                &fixture.root_id,
                LibraryRootGeneration::initial(),
                now_unix_ms().expect("production gap metrics time"),
                production.runtime.queue_policy(),
            )
            .expect("load production gap queue metrics");
        if status.freshness == crate::domain::CatalogFreshnessState::Synchronized
            && metrics.pending_count == 0
            && metrics.leased_count == 0
            && metrics.retry_wait_count == 0
        {
            return snapshot;
        }
        if Instant::now() >= deadline {
            let connection = rusqlite::Connection::open(&fixture.storage.catalog_path)
                .expect("open production gap timeout diagnostics");
            let queue = connection
                .prepare(
                    "SELECT id, origin, status, attempt_count, last_failure_code,
                            last_failure_message
                     FROM library_change_queue WHERE root_id = ?1 ORDER BY id",
                )
                .and_then(|mut statement| {
                    statement
                        .query_map([&fixture.root_id], |row| {
                            Ok((
                                row.get::<_, i64>(0)?,
                                row.get::<_, String>(1)?,
                                row.get::<_, String>(2)?,
                                row.get::<_, i64>(3)?,
                                row.get::<_, Option<String>>(4)?,
                                row.get::<_, Option<String>>(5)?,
                            ))
                        })?
                        .collect::<Result<Vec<_>, _>>()
                })
                .expect("load production gap timeout queue");
            let inventory = connection
                .query_row(
                    "SELECT COUNT(*),
                            COALESCE(GROUP_CONCAT(status || ':' || enumeration_complete || ':' || absence_authority), '')
                     FROM library_metadata_inventory_runs",
                    [],
                    |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)),
                )
                .expect("load production gap timeout inventory");
            panic!(
                "production gap did not converge: freshness={:?} pending={} leased={} retry_wait={} last_issue={:?} queue={queue:?} inventory={inventory:?}",
                status.freshness,
                metrics.pending_count,
                metrics.leased_count,
                metrics.retry_wait_count,
                status.last_issue_code,
            );
        }
        thread::sleep(Duration::from_millis(2));
    }
}

#[cfg(windows)]
fn assert_p2_gap_consumer(fixture: &ProductionGapFixture, gap_change_id: i64) {
    let connection = rusqlite::Connection::open(&fixture.storage.catalog_path)
        .expect("open P2 gap consumer catalog");
    let evidence: (String, Option<i64>, String, String, String, String, String) = connection
        .query_row(
            "SELECT lineage.consumer_kind, lineage.recovery_change_id,
                    gap.status, recovery.origin, recovery_lanes.lane,
                    recovery.intent_kind, authority.reason
             FROM library_live_gap_recovery_claims AS lineage
             JOIN library_change_queue AS gap ON gap.id = lineage.gap_change_id
             JOIN library_change_queue AS recovery ON recovery.id = lineage.recovery_change_id
             JOIN library_change_queue_lanes AS recovery_lanes
               ON recovery_lanes.change_id = recovery.id
             JOIN library_recovery_authorities AS authority
               ON authority.change_id = recovery.id
             WHERE lineage.gap_change_id = ?1",
            [gap_change_id],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                ))
            },
        )
        .expect("load P2 live-gap lineage and consumer");
    assert_eq!(evidence.0, "metadata_inventory_control");
    assert!(evidence.1.is_some());
    assert_eq!(evidence.2, "superseded");
    assert_eq!(evidence.3, "metadata_inventory");
    assert_eq!(evidence.4, "p2_recovery");
    assert_eq!(evidence.5, "freshness_unknown");
    assert_eq!(evidence.6, "watcher_uncovered_gap");
}

#[cfg(windows)]
fn assert_journal_gap_consumer(fixture: &ProductionGapFixture, gap_change_id: i64) {
    let connection = rusqlite::Connection::open(&fixture.storage.catalog_path)
        .expect("open journal gap consumer catalog");
    let evidence: (
        String,
        Option<String>,
        String,
        String,
        String,
        String,
        String,
        String,
    ) = connection
        .query_row(
            "SELECT lineage.consumer_kind, lineage.source_range_id, gap.status,
                        ranges.root_id, ranges.requested_start_usn,
                        ranges.covered_until_usn, lifecycle.lifecycle_state,
                        lineage.opening_next_usn
                 FROM library_live_gap_recovery_claims AS lineage
                 JOIN library_change_queue AS gap ON gap.id = lineage.gap_change_id
                 JOIN library_persistent_journal_source_ranges AS ranges
                   ON ranges.id = lineage.source_range_id
                 JOIN library_persistent_journal_range_lifecycle AS lifecycle
                   ON lifecycle.source_range_id = ranges.id
                 WHERE lineage.gap_change_id = ?1",
            [gap_change_id],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                    row.get(7)?,
                ))
            },
        )
        .expect("load journal live-gap lineage and consumer");
    assert_eq!(evidence.0, "journal_source_range");
    assert!(evidence.1.is_some());
    assert_eq!(evidence.2, "superseded");
    assert_eq!(evidence.3, fixture.root_id);
    assert_eq!(evidence.4, evidence.7);
    assert!(
        evidence.5.parse::<i64>().expect("covered journal USN")
            >= evidence.4.parse::<i64>().expect("opening journal USN")
    );
    assert_eq!(evidence.6, "completed");
    let p2_count: i64 = connection
        .query_row(
            "SELECT COUNT(*)
             FROM library_live_gap_recovery_claims
             WHERE gap_change_id = ?1 AND recovery_change_id IS NOT NULL",
            [gap_change_id],
            |row| row.get(0),
        )
        .expect("count unexpected P2 live-gap consumer");
    assert_eq!(p2_count, 0);
}

#[cfg(windows)]
fn seed_current_gap_journal(fixture: &ProductionGapFixture) {
    let registration = describe_production_persistent_journal_root(
        &fixture.root_id,
        LibraryRootGeneration::initial().value(),
        &fixture.source_root,
    )
    .expect("describe production gap journal root");
    let root_reference =
        crate::domain::JournalFileReference::from_bytes(&registration.authorization.root_identity)
            .expect("production gap root file reference");
    let volume = crate::domain::PersistentJournalVolumeIdentity {
        volume_guid: registration.authorization.volume_id.clone(),
        volume_serial: registration.volume_serial,
    };
    let journal_id = crate::domain::JournalIdentifier::new(44).expect("production gap journal ID");
    let usn = crate::domain::JournalUsn::new(20).expect("production gap journal USN");
    let mut catalog = SqliteCatalog::open(fixture.storage.catalog_path.clone())
        .expect("open production gap journal catalog");
    let root = catalog
        .load_incremental_catalog_root(&fixture.root_id)
        .expect("load production gap journal root")
        .expect("published production gap journal root");
    catalog
        .save_persistent_journal_capability(&PersistentJournalCapability {
            root_id: fixture.root_id.clone(),
            root_generation: LibraryRootGeneration::initial(),
            protocol_version: crate::journal_broker::PROTOCOL_VERSION,
            contract_version: PERSISTENT_JOURNAL_CONTRACT_VERSION,
            state: PersistentJournalCapabilityState::Supported,
            continuity: PersistentJournalContinuityState::Current,
            failure: None,
            updated_unix_ms: 1_000,
        })
        .expect("save production gap journal capability");
    catalog
        .seed_persistent_journal_checkpoint_for_test(&PersistentJournalCheckpoint {
            root_id: fixture.root_id.clone(),
            root_generation: LibraryRootGeneration::initial(),
            volume,
            root_file_reference: root_reference,
            journal_id,
            next_unread_usn: usn,
            captured_exclusive_end: usn,
            covered_catalog_revision: root.catalog_revision,
            protocol_version: crate::journal_broker::PROTOCOL_VERSION,
            contract_version: PERSISTENT_JOURNAL_CONTRACT_VERSION,
            continuity: PersistentJournalContinuityState::Current,
            failure: None,
            updated_unix_ms: 1_000,
        })
        .expect("seed production gap journal checkpoint");
}

#[test]
fn current_time_is_representable() {
    assert!(now_unix_ms().expect("current time") > 0);
}
