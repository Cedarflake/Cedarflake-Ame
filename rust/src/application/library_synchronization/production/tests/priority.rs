use super::*;
use std::panic::{AssertUnwindSafe, catch_unwind};

mod connection_lifetime_control;

struct PriorityFixture<G> {
    production: ProductionSynchronization,
    source_gate: Option<G>,
    writer_stop: Arc<AtomicBool>,
    writer: Option<JoinHandle<Result<(), ScanError>>>,
    runtime_drained: bool,
    runtime_stop_deadline: Option<Instant>,
}

impl<G> PriorityFixture<G> {
    fn new(production: ProductionSynchronization, source_gate: G) -> Self {
        Self {
            production,
            source_gate: Some(source_gate),
            writer_stop: Arc::new(AtomicBool::new(false)),
            writer: None,
            runtime_drained: false,
            runtime_stop_deadline: None,
        }
    }

    fn source_gate(&self) -> &G {
        self.source_gate
            .as_ref()
            .expect("active priority source gate")
    }

    fn finish(&mut self) -> Result<(), String> {
        self.writer_stop.store(true, Ordering::Release);
        // The existing gate's Drop unblocks enumeration before runtime drain can join it.
        // Attempt every phase before returning an error, including during assertion unwind.
        let gate = catch_unwind(AssertUnwindSafe(|| drop(self.source_gate.take())));
        let runtime = if self.runtime_drained {
            Ok(Ok(()))
        } else {
            let deadline = *self
                .runtime_stop_deadline
                .get_or_insert_with(|| Instant::now() + Duration::from_secs(2));
            let outcome = catch_unwind(AssertUnwindSafe(|| {
                self.production.finish_stopping_until(deadline)
            }));
            self.runtime_drained = matches!(&outcome, Ok(Ok(())));
            outcome
        };
        let writer = self.writer.take().map(JoinHandle::join);

        gate.map_err(|_| "priority source gate cleanup panicked".to_owned())?;
        runtime
            .map_err(|_| "priority runtime cleanup panicked".to_owned())?
            .map_err(|error| format!("priority runtime cleanup failed: {error:?}"))?;
        if let Some(result) = writer {
            result
                .map_err(|_| "priority writer panicked".to_owned())?
                .map_err(|error| format!("priority writer failed: {error:?}"))?;
        }
        Ok(())
    }
}

impl<G> Drop for PriorityFixture<G> {
    fn drop(&mut self) {
        if self.finish().is_err() && !self.runtime_drained {
            // Retry one recoverable drain panic without granting a new shutdown deadline.
            let _ = self.finish();
        }
    }
}

fn priority_queue_progress(connection: &rusqlite::Connection) -> rusqlite::Result<String> {
    connection.query_row(
        "SELECT COALESCE(GROUP_CONCAT(summary, '; '), '') FROM (
           SELECT lane.lane || ':' || queue.status || ':' ||
                  COALESCE(SUBSTR(queue.last_failure_code, 1, 96), '') || '=' || COUNT(*) ||
                  ',attempts=' || MIN(queue.attempt_count) || '..' || MAX(queue.attempt_count) ||
                  ',lease_generation=' || MIN(queue.lease_generation) || '..' || MAX(queue.lease_generation) ||
                  ',lease_expires=' || COALESCE(MIN(queue.lease_expires_unix_ms), 'none') ||
                      '..' || COALESCE(MAX(queue.lease_expires_unix_ms), 'none') ||
                  ',retry_at=' || COALESCE(MIN(queue.next_retry_unix_ms), 'none') ||
                      '..' || COALESCE(MAX(queue.next_retry_unix_ms), 'none') AS summary
           FROM library_change_queue AS queue
           JOIN library_change_queue_lanes AS lane ON lane.change_id = queue.id
           GROUP BY lane.lane, queue.status, queue.last_failure_code
           ORDER BY lane.lane, queue.status, queue.last_failure_code LIMIT 32
         )",
        [],
        |row| row.get(0),
    )
}

fn priority_worker_progress<'a>(
    worker: Option<&'a JoinHandle<()>>,
    cancelled: &AtomicBool,
) -> (Option<&'a str>, Option<bool>, bool) {
    (
        worker.and_then(|worker| worker.thread().name()),
        worker.map(JoinHandle::is_finished),
        cancelled.load(Ordering::Acquire),
    )
}

fn priority_inventory_progress(
    connection: &rusqlite::Connection,
    root_id: &str,
) -> rusqlite::Result<Option<String>> {
    connection
        .query_row(
            "SELECT 'epoch=' || run.epoch || ',run=' || run.status ||
                ',issue=' || COALESCE(SUBSTR(run.last_issue_code, 1, 96), '') ||
                ',spool=' || COALESCE(spool.state, 'absent') ||
                ',entries=' || (SELECT COUNT(*) FROM library_metadata_inventory_spool_entries
                               WHERE run_id = run.id) ||
                ',directories=' || COALESCE((SELECT GROUP_CONCAT(summary, '; ') FROM (
                    SELECT state || ':' || source_entry_count AS summary
                    FROM library_metadata_inventory_spool_directories
                    WHERE run_id = run.id ORDER BY ordinal LIMIT 8
                )), '')
         FROM library_metadata_inventory_runs AS run
         LEFT JOIN library_metadata_inventory_spools AS spool ON spool.run_id = run.id
         WHERE run.root_id = ?1 ORDER BY run.epoch DESC LIMIT 1",
            [root_id],
            |row| row.get(0),
        )
        .optional()
}

#[test]
fn p0_event_to_visible_p95_stays_below_one_second_with_p1_and_p2_active() {
    let p95 = run_priority_workload(poll_runtime_with_storage);
    assert!(p95 <= Duration::from_secs(1), "P0 P95 was {p95:?}");
}

fn run_priority_workload(
    mut poll: impl FnMut(
        &mut ProductionSynchronization,
        &crate::application::storage::StoragePaths,
    ) -> Result<LibrarySynchronizationSnapshot, ScanError>,
) -> Duration {
    const P1_CANDIDATE_COUNT: usize = 2_048;
    const P2_SOURCE_ENTRIES: usize = 10_000;
    const SAMPLE_COUNT: usize = 25;

    let directory = tempfile::tempdir().expect("test directory");
    let p1_source_root = directory.path().join("p1-source");
    let p2_source_root = directory.path().join("p2-source");
    std::fs::create_dir_all(&p1_source_root).expect("P1 source root");
    std::fs::create_dir_all(&p2_source_root).expect("P2 source root");
    let p1_template = p1_source_root.join("journal-template.png");
    image::RgbImage::from_pixel(2, 2, image::Rgb([40, 80, 160]))
        .save(&p1_template)
        .expect("P1 image template");
    let p1_image_bytes = std::fs::read(&p1_template).expect("read P1 image template");
    for index in 0..P1_CANDIDATE_COUNT {
        std::fs::write(
            p1_source_root.join(format!("journal-{index:04}.png")),
            &p1_image_bytes,
        )
        .expect("P1 source entry");
    }
    for index in 0..P2_SOURCE_ENTRIES {
        std::fs::write(
            p2_source_root.join(format!("recovery-{index:05}.jpg")),
            b"metadata-only recovery fixture",
        )
        .expect("target-scale recovery entry");
    }
    let p1_root_path = crate::adapters::FileDiscovery::new(&p1_source_root.to_string_lossy())
        .expect("P1 root discovery")
        .canonical_root()
        .expect("canonical P1 root")
        .to_string_lossy()
        .into_owned();
    let p2_root_path = crate::adapters::FileDiscovery::new(&p2_source_root.to_string_lossy())
        .expect("P2 root discovery")
        .canonical_root()
        .expect("canonical P2 root")
        .to_string_lossy()
        .into_owned();
    let p1_root_id = crate::application::scan_library::stable_id("library-root-v1", &p1_root_path);
    let p2_root_id = crate::application::scan_library::stable_id("library-root-v1", &p2_root_path);
    let storage = crate::application::storage::StoragePaths {
        catalog_path: directory.path().join("catalog").join("ame.sqlite3"),
        preview_root: directory.path().join("previews"),
        preview_budget_bytes: 64 * 1024 * 1024,
        settings_path: directory.path().join("settings").join("storage.sqlite3"),
    };
    let policy = crate::domain::LibraryChangeQueuePolicy {
        debounce_millis: 0,
        max_unresolved_changes: 4_096,
        max_lease_batch: 64,
        ..crate::domain::LibraryChangeQueuePolicy::default()
    };
    let generation = LibraryRootGeneration::initial();
    let base_unix_ms = now_unix_ms().expect("fixture clock");
    let mut catalog = SqliteCatalog::open(storage.catalog_path.clone()).expect("fixture catalog");
    for (root_id, root_path, scan_id) in [
        (&p1_root_id, &p1_root_path, "initial-priority-p1"),
        (&p2_root_id, &p2_root_path, "initial-priority-p2"),
    ] {
        let discovery =
            crate::adapters::FileDiscovery::new(root_path).expect("priority root discovery");
        let canonical_root = discovery
            .canonical_root()
            .expect("canonical priority root")
            .to_string_lossy()
            .into_owned();
        let publication_identity = discovery
            .metadata_inventory_root_identity()
            .expect("priority root identity query")
            .expect("priority root stable identity");
        catalog
            .begin_scan_with_publication_namespace(
                &ScanRequest {
                    scan_id: scan_id.to_owned(),
                    root_path: canonical_root.clone(),
                    max_items: None,
                    max_entries: None,
                    preview_edge: 512,
                },
                root_id,
                &canonical_root,
                &publication_identity,
            )
            .expect("begin priority root scan");
        catalog
            .prove_live_only_first_import_handoff_for_test(scan_id)
            .expect("prove priority fixture first-import handoff");
        catalog
            .publish_scan(scan_id, root_id, 0, 0)
            .expect("publish priority root scan");
    }
    let mut seed_current_checkpoint =
        |root_id: &str, root_path: &std::path::Path| -> PersistentJournalCheckpoint {
            let registration =
                describe_production_persistent_journal_root(root_id, generation.value(), root_path)
                    .expect("describe priority root");
            let root_reference =
                JournalFileReference::from_bytes(&registration.authorization.root_identity)
                    .expect("priority root reference");
            catalog
                .save_persistent_journal_capability(&PersistentJournalCapability {
                    root_id: root_id.to_owned(),
                    root_generation: generation,
                    protocol_version: crate::journal_broker::PROTOCOL_VERSION,
                    contract_version: PERSISTENT_JOURNAL_CONTRACT_VERSION,
                    state: PersistentJournalCapabilityState::Supported,
                    continuity: PersistentJournalContinuityState::Current,
                    failure: None,
                    updated_unix_ms: base_unix_ms,
                })
                .expect("seed current priority capability");
            let root = catalog
                .load_incremental_catalog_root(root_id)
                .expect("load priority root")
                .expect("published priority root");
            let checkpoint = PersistentJournalCheckpoint {
                root_id: root_id.to_owned(),
                root_generation: generation,
                volume: PersistentJournalVolumeIdentity {
                    volume_guid: registration.authorization.volume_id.clone(),
                    volume_serial: registration.volume_serial,
                },
                root_file_reference: root_reference,
                journal_id: JournalIdentifier::new(44).expect("priority journal ID"),
                next_unread_usn: JournalUsn::new(20).expect("priority next USN"),
                captured_exclusive_end: JournalUsn::new(20).expect("priority captured end"),
                covered_catalog_revision: root.catalog_revision,
                protocol_version: crate::journal_broker::PROTOCOL_VERSION,
                contract_version: PERSISTENT_JOURNAL_CONTRACT_VERSION,
                continuity: PersistentJournalContinuityState::Current,
                failure: None,
                updated_unix_ms: base_unix_ms,
            };
            catalog
                .seed_persistent_journal_checkpoint_for_test(&checkpoint)
                .expect("seed current priority checkpoint");
            checkpoint
        };
    let _p1_checkpoint = seed_current_checkpoint(&p1_root_id, &p1_source_root);
    let p2_checkpoint = seed_current_checkpoint(&p2_root_id, &p2_source_root);
    catalog
        .persist_persistent_journal_root_failure(
            &crate::domain::PersistentJournalRootFailure {
                root_id: p2_root_id.clone(),
                root_generation: generation,
                kind: crate::domain::PersistentJournalRootFailureKind::ContainmentFailure,
                failure: PersistentJournalFailure {
                    code: "priority-fixture-containment".to_owned(),
                    message: "Priority fixture containment recovery".to_owned(),
                },
                opening_boundary: Some(crate::domain::LibraryRecoveryOpeningBoundary {
                    volume: p2_checkpoint.volume.clone(),
                    root_file_reference: p2_checkpoint.root_file_reference.clone(),
                    journal_id: p2_checkpoint.journal_id,
                    next_usn: p2_checkpoint.next_unread_usn,
                    protocol_version: p2_checkpoint.protocol_version,
                    contract_version: p2_checkpoint.contract_version,
                }),
            },
            base_unix_ms + 1,
            policy,
        )
        .expect("admit target-scale P2 recovery")
        .expect("containment recovery is allowlisted");
    drop(catalog);

    let source_factory = QueuedSourceFactory::default();
    let p1_shared_read_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let p1_published_next_usn = Arc::new(std::sync::atomic::AtomicI64::new(20));
    let journal_close_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let session: Arc<dyn PersistentChangeJournalSession> = Arc::new(PriorityJournalSession {
        p1_root_id: p1_root_id.clone(),
        first_usn: 20,
        published_next_usn: Arc::clone(&p1_published_next_usn),
        candidate_count: P1_CANDIDATE_COUNT,
        shared_read_count: Arc::clone(&p1_shared_read_count),
        close_count: Arc::clone(&journal_close_count),
    });
    let mut production = new_production_synchronization_with_connection(
        crate::ports::erase_library_change_source_factory(source_factory.clone()),
        PersistentChangeJournalConnection::Connected(session),
    );
    production.persistent_change_journal_caller = Some(crate::journal_broker::CallerClaim {
        process_id: std::process::id(),
        session_id: 1,
        client_instance: [4; 16],
    });
    production.runtime.queue_policy = policy;
    assert_eq!(
        production.metadata_inventory_page_entries,
        METADATA_INVENTORY_WORK_PAGE_ENTRIES
    );
    crate::adapters::reset_source_enumeration_instrumentation(&p2_root_path);
    let source_gate = crate::adapters::gate_source_enumeration(&p2_root_path);
    let mut fixture = PriorityFixture::new(production, source_gate);
    let progress_connection = rusqlite::Connection::open(&storage.catalog_path)
        .expect("retained priority progress catalog");
    let cold_enumeration_deadline = std::time::Instant::now() + Duration::from_secs(10);
    loop {
        poll(&mut fixture.production, &storage).expect("start the cold P2 source page");
        if fixture.production.recovery.is_some()
            && fixture
                .source_gate()
                .wait_until_blocked(Duration::from_millis(10))
        {
            break;
        }
        assert!(
            std::time::Instant::now() < cold_enumeration_deadline,
            "production did not start the gated cold P2 source page"
        );
        std::thread::yield_now();
    }
    assert_eq!(crate::adapters::source_entry_read_count(&p2_root_path), 0);
    let p2_staged_before: i64 = progress_connection
        .query_row(
            "SELECT COALESCE(MAX(staged_entry_count), 0)
             FROM library_metadata_inventory_runs WHERE root_id = ?1",
            [&p2_root_id],
            |row| row.get(0),
        )
        .expect("load cold P2 progress");
    assert!(
        p2_staged_before < i64::from(METADATA_INVENTORY_WORK_PAGE_ENTRIES),
        "the first P0 sample must precede completion of the first real P2 source page"
    );
    let p1_completed_before = 0_i64;

    let visibility_catalog =
        SqliteCatalog::open(storage.catalog_path.clone()).expect("retained visible catalog");
    let low_writer_operations = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let writer_stop = Arc::clone(&fixture.writer_stop);
    let writer_operations = Arc::clone(&low_writer_operations);
    let writer_catalog_path = storage.catalog_path.clone();
    fixture.writer = Some(thread::spawn(move || -> Result<(), ScanError> {
        let mut catalog = SqliteCatalog::open(writer_catalog_path)?;
        while !writer_stop.load(Ordering::Acquire) {
            catalog.cleanup_terminal_library_changes(0, 1)?;
            writer_operations.fetch_add(1, Ordering::AcqRel);
            std::thread::yield_now();
        }
        Ok(())
    }));
    let writer_deadline = std::time::Instant::now() + Duration::from_secs(5);
    while low_writer_operations.load(Ordering::Acquire) == 0 {
        assert!(
            std::time::Instant::now() < writer_deadline,
            "low-priority writer did not enter SQLite admission"
        );
        std::thread::yield_now();
    }

    let mut latencies = Vec::with_capacity(SAMPLE_COUNT);
    let mut queue_admission_latencies = Vec::with_capacity(SAMPLE_COUNT);
    let mut worker_admission_latencies = Vec::with_capacity(SAMPLE_COUNT);
    let mut visible_query_latencies = Vec::with_capacity(SAMPLE_COUNT);
    let mut p1_completed_samples = Vec::with_capacity(SAMPLE_COUNT);
    let mut p2_source_read_samples = Vec::with_capacity(SAMPLE_COUNT);
    let mut p1_active_sample_count = 0_usize;
    let mut p2_active_sample_count = 0_usize;
    let mut p1_progress_sample_count = 0_usize;
    let mut p2_progress_sample_count = 0_usize;
    let writer_operations_before = low_writer_operations.load(Ordering::Acquire);
    for index in 0..SAMPLE_COUNT {
        let p2_ready_deadline = std::time::Instant::now() + Duration::from_secs(5);
        loop {
            if fixture.production.recovery.is_some()
                && fixture
                    .source_gate()
                    .wait_until_blocked(Duration::from_millis(10))
            {
                break;
            }
            poll(&mut fixture.production, &storage).expect("start the next bounded P2 source page");
            assert!(
                std::time::Instant::now() < p2_ready_deadline,
                "sample {index} did not start a bounded P2 source page"
            );
            std::thread::yield_now();
        }
        let p1_completed_at_start: i64 = progress_connection
            .query_row(
                "SELECT COUNT(*)
                 FROM library_change_queue AS queue
                 JOIN library_change_queue_lanes AS lane ON lane.change_id = queue.id
                 WHERE queue.root_id = ?1 AND lane.lane = 'p1_journal'
                   AND queue.status = 'completed'",
                [&p1_root_id],
                |row| row.get(0),
            )
            .expect("load P1 sample baseline");
        let p2_source_reads_at_start = crate::adapters::source_entry_read_count(&p2_root_path);
        if index == 0 {
            assert_eq!(p2_source_reads_at_start, 0);
            assert_eq!(p2_staged_before, 0);
        }
        let published_candidate_count = (index + 1)
            .checked_mul(usize::try_from(policy.max_lease_batch).expect("P1 page size"))
            .expect("P1 published candidate count")
            .min(P1_CANDIDATE_COUNT);
        p1_published_next_usn.store(
            20 + i64::try_from(published_candidate_count).expect("P1 published end"),
            Ordering::Release,
        );
        let p1_ready_deadline = std::time::Instant::now() + Duration::from_secs(5);
        let pending_p1 = loop {
            poll(&mut fixture.production, &storage).expect("publish the next bounded P1 page");
            let pending_p1: i64 = progress_connection
                .query_row(
                    "SELECT COUNT(*)
                     FROM library_change_queue AS queue
                     JOIN library_change_queue_lanes AS lane ON lane.change_id = queue.id
                     WHERE queue.root_id = ?1 AND lane.lane = 'p1_journal'
                       AND queue.status IN ('pending', 'leased', 'retry_wait')",
                    [&p1_root_id],
                    |row| row.get(0),
                )
                .expect("load active P1 sample backlog");
            if pending_p1 > 0 && fixture.production.journal.is_some() {
                break pending_p1;
            }
            assert!(
                std::time::Instant::now() < p1_ready_deadline,
                "sample {index} did not publish and activate its bounded P1 page"
            );
            std::thread::yield_now();
        };
        assert!(pending_p1 <= i64::from(policy.max_lease_batch));
        assert!(
            fixture.production.recovery.is_some(),
            "sample {index} started a new P2 page before P1 and must retain it while P1 starts"
        );
        p1_active_sample_count = p1_active_sample_count.saturating_add(1);
        p2_active_sample_count = p2_active_sample_count.saturating_add(1);

        let relative_path = format!("live-{index:02}.png");
        let absolute_path = p1_source_root.join(&relative_path);
        image::RgbImage::from_pixel(
            2,
            2,
            image::Rgb([u8::try_from(index).expect("sample color"), 80, 160]),
        )
        .save(&absolute_path)
        .expect("live image fixture");
        let observed_unix_ms = now_unix_ms().expect("event time");
        let started = std::time::Instant::now();
        source_factory
            .batches
            .lock()
            .expect("queued source batches")
            .entry(p1_root_id.clone())
            .or_default()
            .push_back(crate::domain::LibraryChangeSourceBatch {
                observations: vec![crate::domain::LibraryChangeObservation {
                    root_id: p1_root_id.clone(),
                    root_generation: generation,
                    sequence: u64::try_from(index + 1).expect("live sequence"),
                    observed_unix_ms,
                    kind: crate::domain::LibraryChangeObservationKind::Created,
                    scope: crate::domain::LibraryChangeScope::Path,
                    relative_path: relative_path.clone(),
                    previous_relative_path: None,
                    origin: crate::domain::LibraryChangeOrigin::LiveNotification,
                }],
                health: crate::domain::LibraryChangeSourceHealth::Healthy,
                dropped_observation_count: 0,
                ignored_callback_count: 0,
                last_issue_code: None,
            });
        fixture.source_gate().allow_entries(128);

        let visible_deadline = started + Duration::from_secs(5);
        let mut queue_admission_latency = None;
        let mut worker_admission_latency = None;
        let mut poll_count = 0_u64;
        let mut poll_total = Duration::ZERO;
        let mut poll_maximum = Duration::ZERO;
        let location = loop {
            let poll_started = std::time::Instant::now();
            let poll_result = poll(&mut fixture.production, &storage);
            let poll_elapsed = poll_started.elapsed();
            poll_count = poll_count.saturating_add(1);
            poll_total = poll_total.saturating_add(poll_elapsed);
            poll_maximum = poll_maximum.max(poll_elapsed);
            poll_result.expect("drive reserved P0 publication");
            if worker_admission_latency.is_none() && fixture.production.live.is_some() {
                worker_admission_latency = Some(started.elapsed());
            }
            if queue_admission_latency.is_none() {
                let queued: i64 = progress_connection
                    .query_row(
                        "SELECT COUNT(*)
                         FROM library_change_queue AS queue
                         JOIN library_change_queue_lanes AS lane
                           ON lane.change_id = queue.id
                         WHERE queue.root_id = ?1 AND queue.relative_path = ?2
                           AND lane.lane = 'p0_live'",
                        rusqlite::params![p1_root_id, relative_path],
                        |row| row.get(0),
                    )
                    .expect("load P0 queue admission evidence");
                if queued == 1 {
                    queue_admission_latency = Some(started.elapsed());
                }
            }
            let visible_query_started = std::time::Instant::now();
            let visible = visibility_catalog
                .load_incremental_location_by_relative_path(&p1_root_id, &relative_path)
                .expect("load visible P0 location");
            let visible_query_wall = visible_query_started.elapsed();
            if let Some(location) = visible {
                visible_query_latencies.push(visible_query_wall);
                break location;
            }
            if std::time::Instant::now() >= visible_deadline {
                let queue_evidence = progress_connection
                    .query_row(
                        "SELECT COALESCE(MAX(queue.status), 'missing'),
                                MAX(queue.last_failure_code), COUNT(*)
                         FROM library_change_queue AS queue
                         JOIN library_change_queue_lanes AS lane
                           ON lane.change_id = queue.id
                         WHERE queue.root_id = ?1 AND queue.relative_path = ?2
                           AND lane.lane = 'p0_live'",
                        rusqlite::params![p1_root_id, relative_path],
                        |row| {
                            Ok((
                                row.get::<_, String>(0)?,
                                row.get::<_, Option<String>>(1)?,
                                row.get::<_, i64>(2)?,
                            ))
                        },
                    )
                    .expect("load timed-out P0 queue evidence");
                panic!(
                    "P0 location did not become visible within five seconds: queue={queue_evidence:?} queue_admission={queue_admission_latency:?} worker_admission={worker_admission_latency:?} poll_count={poll_count} poll_total={poll_total:?} poll_maximum={poll_maximum:?} live_active={} journal_active={} recovery_active={} low_writer_ops={}",
                    fixture.production.live.is_some(),
                    fixture.production.journal.is_some(),
                    fixture.production.recovery.is_some(),
                    low_writer_operations.load(Ordering::Acquire),
                );
            }
            std::thread::sleep(Duration::from_millis(1));
        };
        queue_admission_latencies
            .push(queue_admission_latency.expect("visible P0 work has durable queue evidence"));
        worker_admission_latencies
            .push(worker_admission_latency.expect("visible P0 work admitted the reserved worker"));
        let visible_latency = started.elapsed();
        latencies.push(visible_latency);
        assert!(matches!(
            location.preview_status,
            crate::domain::PreviewStatus::Pending
        ));
        assert!(location.preview_path.is_empty());
        let lane_progress_deadline = std::time::Instant::now() + Duration::from_secs(5);
        let (p1_completed, p2_source_reads) = loop {
            poll(&mut fixture.production, &storage)
                .expect("complete the bounded P1 and P2 sample pages");
            let p1_completed: i64 = progress_connection
                .query_row(
                    "SELECT COUNT(*)
                     FROM library_change_queue AS queue
                     JOIN library_change_queue_lanes AS lane ON lane.change_id = queue.id
                     WHERE queue.root_id = ?1 AND lane.lane = 'p1_journal'
                       AND queue.status = 'completed'",
                    [&p1_root_id],
                    |row| row.get::<_, i64>(0),
                )
                .expect("load P1 sample progress");
            let p2_source_reads = crate::adapters::source_entry_read_count(&p2_root_path);
            if p1_completed > p1_completed_at_start && p2_source_reads > p2_source_reads_at_start {
                break (p1_completed, p2_source_reads);
            }
            assert!(
                std::time::Instant::now() < lane_progress_deadline,
                "sample {index} did not advance both active lower-priority lanes: p1={p1_completed_at_start}->{p1_completed} p2={p2_source_reads_at_start}->{p2_source_reads} visible={visible_latency:?} poll_count={poll_count} poll_total={poll_total:?} poll_maximum={poll_maximum:?} queue_admission={queue_admission_latency:?} worker_admission={worker_admission_latency:?} queue={:?} live_worker={:?} journal_worker={:?} recovery_worker={:?} runtime_stopping={} stop_requested={} low_writer_ops={} now_unix_ms={:?}",
                priority_queue_progress(&progress_connection),
                fixture.production.live.as_ref().map(|task| {
                    priority_worker_progress(task.worker.as_ref(), &task.cancelled)
                }),
                fixture.production.journal.as_ref().map(|task| {
                    priority_worker_progress(task.worker.as_ref(), &task.cancelled)
                }),
                fixture.production.recovery.as_ref().map(|task| {
                    priority_worker_progress(task.worker.as_ref(), &task.cancelled)
                }),
                fixture.production.is_stopping,
                fixture.production.stop_requested.load(Ordering::Acquire),
                low_writer_operations.load(Ordering::Acquire),
                now_unix_ms(),
            );
            std::thread::yield_now();
        };
        assert_eq!(
            p1_completed - p1_completed_at_start,
            i64::from(policy.max_lease_batch),
            "each measured P1 page must remain bounded"
        );
        assert_eq!(
            p2_source_reads - p2_source_reads_at_start,
            128,
            "each measured P2 raw source page must remain bounded"
        );
        p1_progress_sample_count = p1_progress_sample_count.saturating_add(1);
        p2_progress_sample_count = p2_progress_sample_count.saturating_add(1);
        p1_completed_samples.push(p1_completed);
        p2_source_read_samples.push(p2_source_reads);
        eprintln!(
            "controlled P0 sample index={index} queue_admission_ms={} worker_admission_ms={} visible_ms={} poll_count={poll_count} poll_total_ms={} poll_max_ms={}",
            queue_admission_latency
                .expect("sample queue admission")
                .as_millis(),
            worker_admission_latency
                .expect("sample worker admission")
                .as_millis(),
            visible_latency.as_millis(),
            poll_total.as_millis(),
            poll_maximum.as_millis(),
        );
    }

    p1_published_next_usn.store(
        20 + i64::try_from(P1_CANDIDATE_COUNT).expect("P1 final end"),
        Ordering::Release,
    );
    fixture.source_gate().release();
    let progress_deadline = std::time::Instant::now() + Duration::from_secs(60);
    let (p1_completed_after, p2_staged_after) = loop {
        poll(&mut fixture.production, &storage).expect("drive post-measurement P1 and P2 progress");
        let completed_p1 = progress_connection
            .query_row(
                "SELECT COUNT(*)
                 FROM library_change_queue AS queue
                 JOIN library_change_queue_lanes AS lane ON lane.change_id = queue.id
                 WHERE queue.root_id = ?1 AND lane.lane = 'p1_journal'
                   AND queue.status = 'completed'",
                [&p1_root_id],
                |row| row.get::<_, i64>(0),
            )
            .expect("load post-measurement P1 progress");
        let staged_p2 = progress_connection
            .query_row(
                "SELECT COALESCE(MAX(staged_entry_count), 0)
                 FROM library_metadata_inventory_runs WHERE root_id = ?1",
                [&p2_root_id],
                |row| row.get::<_, i64>(0),
            )
            .expect("load post-measurement P2 progress");
        if completed_p1 == i64::try_from(P1_CANDIDATE_COUNT).expect("P1 total")
            && staged_p2 >= i64::from(METADATA_INVENTORY_WORK_PAGE_ENTRIES)
            && crate::adapters::source_entry_read_count(&p2_root_path)
                == u64::try_from(P2_SOURCE_ENTRIES).expect("P2 total")
        {
            break (completed_p1, staged_p2);
        }
        if std::time::Instant::now() >= progress_deadline {
            eprintln!(
                "controlled inventory deadline worker={:?} runtime_stopping={} stop_requested={}",
                fixture.production.recovery.as_ref().map(|task| {
                    priority_worker_progress(task.worker.as_ref(), &task.cancelled)
                }),
                fixture.production.is_stopping,
                fixture.production.stop_requested.load(Ordering::Acquire),
            );
        }
        assert!(
            std::time::Instant::now() < progress_deadline,
            "P1 did not finish all 2048 candidates or P2 did not publish its real default 4095-entry page: completed_p1={completed_p1} staged_p2={staged_p2} source_reads={} spool_opens={} inventory={:?} queue={:?} active_p1={} active_p2={}",
            crate::adapters::source_entry_read_count(&p2_root_path),
            crate::adapters::source_spool_open_count(&p2_root_path),
            priority_inventory_progress(&progress_connection, &p2_root_id),
            priority_queue_progress(&progress_connection),
            fixture.production.journal.is_some(),
            fixture.production.recovery.is_some()
        );
        std::thread::yield_now();
    };

    latencies.sort_unstable();
    queue_admission_latencies.sort_unstable();
    worker_admission_latencies.sort_unstable();
    visible_query_latencies.sort_unstable();
    let p50_index = (latencies.len() * 50).div_ceil(100) - 1;
    let p95_index = (latencies.len() * 95).div_ceil(100) - 1;
    let p50 = latencies[p50_index];
    let p95 = latencies[p95_index];
    let maximum = *latencies.last().expect("at least one P0 latency sample");
    let over_one_second = latencies
        .iter()
        .filter(|latency| **latency > Duration::from_secs(1))
        .count();
    let queue_p95 = queue_admission_latencies[p95_index];
    let worker_admission_p95 = worker_admission_latencies[p95_index];
    let visible_query_p95 = visible_query_latencies[p95_index];
    let writer_operations_after = low_writer_operations.load(Ordering::Acquire);
    let p1_completed_first = p1_completed_samples.first().copied().unwrap_or_default();
    let p1_completed_last = p1_completed_samples.last().copied().unwrap_or_default();
    let p2_source_reads_first = p2_source_read_samples.first().copied().unwrap_or_default();
    let p2_source_reads_last = p2_source_read_samples.last().copied().unwrap_or_default();
    let p1_statuses = {
        let mut statement = progress_connection
            .prepare(
                "SELECT queue.status, COALESCE(queue.last_failure_code, ''), COUNT(*)
                 FROM library_change_queue AS queue
                 JOIN library_change_queue_lanes AS lane ON lane.change_id = queue.id
                 WHERE queue.root_id = ?1 AND lane.lane = 'p1_journal'
                 GROUP BY queue.status, queue.last_failure_code
                 ORDER BY queue.status, queue.last_failure_code",
            )
            .expect("prepare P1 status evidence");
        statement
            .query_map([&p1_root_id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            })
            .expect("query P1 status evidence")
            .collect::<Result<Vec<_>, _>>()
            .expect("collect P1 status evidence")
    };
    eprintln!(
        "controlled P0 priority fixture samples={SAMPLE_COUNT} p1_candidates={P1_CANDIDATE_COUNT} p1_completed={p1_completed_before}->{p1_completed_first}->{p1_completed_last}->{p1_completed_after} p1_active_samples={p1_active_sample_count} p1_progress_samples={p1_progress_sample_count} p1_statuses={p1_statuses:?} p2_entries={P2_SOURCE_ENTRIES} p2_cold_staged={p2_staged_before} p2_source_reads={p2_source_reads_first}->{p2_source_reads_last}->{} p2_staged_after={p2_staged_after} p2_active_samples={p2_active_sample_count} p2_progress_samples={p2_progress_sample_count} low_writer_ops={writer_operations_before}->{writer_operations_after} queue_p95_ms={} worker_admission_p95_ms={} visible_query_p95_ms={} visible_p50_ms={} visible_p95_ms={} visible_max_ms={} visible_over_one_second={over_one_second}",
        crate::adapters::source_entry_read_count(&p2_root_path),
        queue_p95.as_millis(),
        worker_admission_p95.as_millis(),
        visible_query_p95.as_millis(),
        p50.as_millis(),
        p95.as_millis(),
        maximum.as_millis()
    );
    fixture.finish().expect("stop priority fixture");
    assert!(p1_shared_read_count.load(Ordering::Acquire) > 0);
    assert_eq!(
        p1_active_sample_count, SAMPLE_COUNT,
        "P1 must be active in every measured sample"
    );
    assert_eq!(
        p1_progress_sample_count, SAMPLE_COUNT,
        "P1 must make durable progress in every measured sample"
    );
    assert_eq!(
        p1_completed_after,
        i64::try_from(P1_CANDIDATE_COUNT).expect("P1 total"),
        "all real P1 candidates must complete"
    );
    assert_eq!(
        p2_active_sample_count, SAMPLE_COUNT,
        "P2 must be active in every measured sample"
    );
    assert_eq!(
        p2_progress_sample_count, SAMPLE_COUNT,
        "P2 must consume real source entries in every measured sample"
    );
    assert_eq!(
        crate::adapters::source_entry_read_count(&p2_root_path),
        u64::try_from(P2_SOURCE_ENTRIES).expect("P2 total")
    );
    assert!(
        p2_staged_after >= i64::from(METADATA_INVENTORY_WORK_PAGE_ENTRIES),
        "P2 must publish the production default page after real enumeration"
    );
    assert!(
        writer_operations_after > writer_operations_before,
        "the lower-priority writer did not compete during P0 measurement"
    );
    assert_eq!(journal_close_count.load(Ordering::Acquire), 1);
    p95
}

#[test]
fn priority_fixture_unwind_releases_gate_stops_runtime_and_joins_writer() {
    let directory = tempfile::tempdir().expect("priority cleanup directory");
    std::fs::write(directory.path().join("entry.jpg"), b"metadata only").expect("entry");
    let root_path = directory.path().to_string_lossy().into_owned();
    let close_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let writer_finished = Arc::new(AtomicBool::new(false));
    let writer_stop = Arc::new(AtomicBool::new(false));
    crate::adapters::reset_source_enumeration_instrumentation(&root_path);
    let panic = catch_unwind(AssertUnwindSafe(|| {
        let source_gate = crate::adapters::gate_source_enumeration(&root_path);
        let mut fixture = PriorityFixture::new(
            runtime_with_close_probe(Arc::clone(&close_count)),
            source_gate,
        );
        fixture.writer_stop = Arc::clone(&writer_stop);
        let stopped = Arc::clone(&writer_stop);
        let closed = Arc::clone(&close_count);
        let finished = Arc::clone(&writer_finished);
        let worker_root = root_path.clone();
        fixture.writer = Some(thread::spawn(move || {
            let discovery = crate::adapters::PublicationGuardedFileDiscovery::
                new_metadata_inventory_source_guard(&worker_root, None)
                .expect("guarded priority cleanup source");
            discovery
                .streaming_metadata_inventory_entries_in_directory("")
                .expect("priority cleanup entries")
                .next()
                .expect("priority cleanup entry")
                .expect("read priority cleanup entry");
            assert!(
                stopped.load(Ordering::Acquire),
                "stop intent precedes gate release"
            );
            let deadline = Instant::now() + Duration::from_secs(5);
            while closed.load(Ordering::Acquire) == 0 {
                assert!(
                    Instant::now() < deadline,
                    "runtime must stop before writer join"
                );
                thread::yield_now();
            }
            finished.store(true, Ordering::Release);
            Ok(())
        }));
        assert!(
            fixture
                .source_gate()
                .wait_until_blocked(Duration::from_secs(5))
        );
        assert_eq!(crate::adapters::source_entry_read_count(&root_path), 0);
        panic!("injected priority assertion");
    }));

    assert_eq!(
        panic.expect_err("injected panic").downcast_ref::<&str>(),
        Some(&"injected priority assertion")
    );
    assert!(writer_stop.load(Ordering::Acquire));
    assert!(writer_finished.load(Ordering::Acquire));
    assert_eq!(close_count.load(Ordering::Acquire), 1);
    assert_eq!(crate::adapters::source_entry_read_count(&root_path), 1);
    std::fs::rename(
        directory.path().join("entry.jpg"),
        directory.path().join("released.jpg"),
    )
    .expect("source handles released");
}

#[test]
fn priority_fixture_finish_reports_writer_failure_after_cleanup() {
    let close_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let mut fixture = PriorityFixture::new(runtime_with_close_probe(Arc::clone(&close_count)), ());
    fixture.writer = Some(thread::spawn(|| {
        Err(ScanError::new(
            "priority_writer_injected_failure",
            "controlled writer failure",
        ))
    }));

    let failure = fixture
        .finish()
        .expect_err("writer failure must remain visible");
    assert!(failure.contains("priority_writer_injected_failure"));
    assert!(fixture.writer_stop.load(Ordering::Acquire));
    assert!(fixture.writer.is_none());
    assert!(fixture.source_gate.is_none());
    fixture.finish().expect("idempotent cleanup");
    drop(fixture);
    assert_eq!(close_count.load(Ordering::Acquire), 1);
}

#[test]
fn priority_fixture_unwind_preserves_assertion_when_writer_panics() {
    let close_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let panic = catch_unwind(AssertUnwindSafe(|| {
        let mut fixture =
            PriorityFixture::new(runtime_with_close_probe(Arc::clone(&close_count)), ());
        fixture.writer = Some(thread::spawn(|| panic!("injected writer panic")));
        panic!("original priority assertion");
    }));

    assert_eq!(
        panic.expect_err("original panic").downcast_ref::<&str>(),
        Some(&"original priority assertion")
    );
    assert_eq!(close_count.load(Ordering::Acquire), 1);
}

#[test]
fn priority_fixture_retries_runtime_panic_with_the_original_deadline() {
    let close_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let mut fixture = PriorityFixture::new(runtime_with_close_probe(Arc::clone(&close_count)), ());
    fixture
        .production
        .inject_drain_panic(DrainPanicPoint::RequestStop, Some(1));

    let failure = fixture
        .finish()
        .expect_err("runtime panic must remain visible");
    assert!(failure.contains("runtime cleanup panicked"));
    assert!(!fixture.runtime_drained);
    assert_eq!(close_count.load(Ordering::Acquire), 0);
    let deadline = fixture.runtime_stop_deadline.expect("original deadline");
    fixture.finish().expect("retry the same runtime owner");
    assert!(fixture.runtime_drained);
    assert_eq!(fixture.runtime_stop_deadline, Some(deadline));
    drop(fixture);
    assert_eq!(close_count.load(Ordering::Acquire), 1);
}

#[test]
fn priority_fixture_failure_diagnostics_match_the_current_catalog_schema() {
    let directory = tempfile::tempdir().expect("priority diagnostics directory");
    let catalog_path = directory.path().join("catalog.sqlite3");
    drop(SqliteCatalog::open(catalog_path.clone()).expect("priority diagnostics catalog"));
    let connection = rusqlite::Connection::open(catalog_path).expect("priority diagnostics reader");

    assert_eq!(
        priority_queue_progress(&connection).expect("queue diagnostics"),
        ""
    );
    assert_eq!(
        priority_inventory_progress(&connection, "unregistered-root")
            .expect("inventory diagnostics"),
        None
    );
}

#[test]
fn priority_fixture_unwind_retries_one_runtime_panic_without_replacing_the_assertion() {
    let close_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let panic = catch_unwind(AssertUnwindSafe(|| {
        let mut fixture =
            PriorityFixture::new(runtime_with_close_probe(Arc::clone(&close_count)), ());
        fixture
            .production
            .inject_drain_panic(DrainPanicPoint::RequestStop, Some(1));
        panic!("original priority assertion");
    }));

    assert_eq!(
        panic.expect_err("original panic").downcast_ref::<&str>(),
        Some(&"original priority assertion")
    );
    assert_eq!(close_count.load(Ordering::Acquire), 1);
}
