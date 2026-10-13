use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};

use super::*;

#[test]
fn retained_ingress_defers_other_root_opening_without_self_waiting_in_production_poll() {
    let fixture = ProductionGapFixture::new("ingress-reservation", 0);
    let mut catalog = SqliteCatalog::open(fixture.storage.catalog_path.clone()).unwrap();
    let other_root = publish_other_root(&fixture, &mut catalog);
    let factory = QueuedSourceFactory::default();
    let mut production = fast_gap_runtime(factory.clone(), test_live_only_connection());
    let outcome = catch_unwind(AssertUnwindSafe(|| {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            let snapshot = poll_runtime_with_storage(&mut production, &fixture.storage).unwrap();
            if snapshot.roots.len() == 2
                && snapshot.roots.iter().all(|root| {
                    root.source_health == crate::domain::LibraryChangeSourceHealth::Healthy
                })
            {
                break;
            }
            assert!(Instant::now() < deadline, "both real observers must start");
            thread::sleep(Duration::from_millis(2));
        }
        catalog
            .save_persistent_journal_capability(&PersistentJournalCapability {
                root_id: other_root.clone(),
                root_generation: LibraryRootGeneration::initial(),
                protocol_version: crate::journal_broker::PROTOCOL_VERSION,
                contract_version: PERSISTENT_JOURNAL_CONTRACT_VERSION,
                state: PersistentJournalCapabilityState::Supported,
                continuity: PersistentJournalContinuityState::BaselineRequired,
                failure: None,
                updated_unix_ms: now_unix_ms().unwrap(),
            })
            .expect("root B awaits the existing-root opening protocol");
        drop(
            SqliteCatalog::open(fixture.storage.catalog_path.clone())
                .expect("valid two-root catalog"),
        );
        let image_path = fixture.source_root.join("ingress.png");
        image::RgbImage::from_pixel(2, 2, image::Rgb([20, 40, 80]))
            .save(&image_path)
            .unwrap();
        let original_bytes = std::fs::read(&image_path).unwrap();
        let writer_catalog = SqliteCatalog::open(fixture.storage.catalog_path.clone()).unwrap();
        let mut writer = HeldRecoveryWriter::start(writer_catalog);
        push_gap_batch(
            &factory,
            &fixture.root_id,
            live_path_batch(&fixture.root_id),
        );

        // Establish the real observer handoff before entering unrelated coordinator work.
        // This isolates the retained-registration race from source-thread delivery timing.
        let deadline = Instant::now() + Duration::from_secs(3);
        let snapshot = loop {
            let snapshot = production
                .runtime
                .poll_without_authoritative_recovery(&mut catalog, now_unix_ms().unwrap(), |_| {
                    crate::domain::LibraryRootAvailability::Available
                })
                .unwrap();
            if production.runtime.roots[&fixture.root_id]
                .handoff
                .is_waiting_for_writer()
            {
                break snapshot;
            }
            assert!(
                Instant::now() < deadline,
                "root A must retain its P0 registration"
            );
            thread::sleep(Duration::from_millis(2));
        };
        let opening =
            crate::application::library_synchronization::journal_baseline::select_opening_work(
                &catalog, &snapshot, None,
            )
            .unwrap()
            .expect("root B has actual synchronous LiveOnly opening work");
        assert_eq!(opening.root_id(), other_root);
        assert!(
            production.runtime.roots[&fixture.root_id]
                .handoff
                .has_pending()
        );

        // The old coordinator registers a second writer behind its own retained P0.
        // Abort that registration before Condvar::wait so RED also retires every owner.
        SqliteCatalog::after_next_write_registration_for_test(|| {
            panic!("production poll requested another writer behind its retained ingress")
        });
        let started = Instant::now();
        let poll = catch_unwind(AssertUnwindSafe(|| {
            poll_runtime_with_storage(&mut production, &fixture.storage)
        }));
        let elapsed = started.elapsed();
        SqliteCatalog::after_next_write_registration_for_test(|| {});
        let writer_still_held = writer.is_held();
        writer
            .finish()
            .expect("retire the controlled recovery writer");
        let snapshot = poll
            .expect("coordinator must not synchronously wait behind itself")
            .unwrap();
        assert!(
            writer_still_held,
            "poll must return before the competing writer releases"
        );
        assert!(
            elapsed < Duration::from_millis(200),
            "poll took {elapsed:?}"
        );
        assert_eq!(snapshot.roots.len(), 2);
        assert_eq!(
            snapshot
                .roots
                .iter()
                .find(|root| root.root_id == fixture.root_id)
                .unwrap()
                .freshness,
            crate::domain::CatalogFreshnessState::Updating,
        );
        assert!(
            production.runtime.roots[&fixture.root_id]
                .handoff
                .has_pending()
        );
        assert_eq!(
            catalog
                .load_persistent_journal_capabilities()
                .unwrap()
                .into_iter()
                .find(|capability| capability.root_id == other_root)
                .unwrap()
                .continuity,
            PersistentJournalContinuityState::BaselineRequired,
            "other-root opening remains pending until ingress admission retires",
        );

        let deadline = Instant::now() + Duration::from_secs(8);
        loop {
            let snapshot = poll_runtime_with_storage(&mut production, &fixture.storage).unwrap();
            let visible = catalog
                .load_incremental_location_by_relative_path(&fixture.root_id, "ingress.png")
                .unwrap()
                .is_some();
            let opened = catalog
                .load_persistent_journal_capabilities()
                .unwrap()
                .into_iter()
                .any(|capability| {
                    capability.root_id == other_root
                        && capability.continuity == PersistentJournalContinuityState::LiveOnly
                });
            if visible
                && opened
                && snapshot.roots.iter().all(|root| {
                    root.freshness == crate::domain::CatalogFreshnessState::Synchronized
                })
            {
                break;
            }
            assert!(
                Instant::now() < deadline,
                "ingress and other-root opening must converge"
            );
            thread::sleep(Duration::from_millis(2));
        }
        assert_eq!(std::fs::read(image_path).unwrap(), original_bytes);
    }));
    SqliteCatalog::after_next_write_registration_for_test(|| {});
    let stopped = production.finish_stopping_until(Instant::now() + Duration::from_secs(2));
    stopped.expect("all production owners retire within the original stop budget");
    assert!(
        production.live.is_none() && production.journal.is_none() && production.recovery.is_none()
    );
    assert!(production.core_stopped && production.journal_closed);
    if let Err(panic) = outcome {
        resume_unwind(panic);
    }
    drop(catalog);
    let reopened = SqliteCatalog::open(fixture.storage.catalog_path.clone()).expect("FULL reopen");
    assert!(
        reopened
            .load_incremental_location_by_relative_path(&fixture.root_id, "ingress.png")
            .unwrap()
            .is_some()
    );
    assert_eq!(reopened.load_incremental_catalog_roots().unwrap().len(), 2);
}

fn publish_other_root(fixture: &ProductionGapFixture, catalog: &mut SqliteCatalog) -> String {
    let source = fixture._directory.path().join("other-source");
    std::fs::create_dir(&source).unwrap();
    let discovery = crate::adapters::FileDiscovery::new(&source.to_string_lossy()).unwrap();
    let root_path = discovery
        .canonical_root()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    let root_id = crate::application::scan_library::stable_id("library-root-v1", &root_path);
    let identity = discovery
        .metadata_inventory_root_identity()
        .unwrap()
        .unwrap();
    let request = ScanRequest {
        scan_id: "initial-ingress-other-root".to_owned(),
        root_path: root_path.clone(),
        max_items: None,
        max_entries: None,
        preview_edge: 512,
    };
    catalog
        .begin_scan_with_publication_namespace(&request, &root_id, &root_path, &identity)
        .unwrap();
    catalog
        .prove_live_only_first_import_handoff_for_test(&request.scan_id)
        .unwrap();
    catalog
        .publish_scan(&request.scan_id, &root_id, 0, 0)
        .unwrap();
    root_id
}

fn live_path_batch(root_id: &str) -> crate::domain::LibraryChangeSourceBatch {
    crate::domain::LibraryChangeSourceBatch {
        observations: vec![crate::domain::LibraryChangeObservation {
            root_id: root_id.to_owned(),
            root_generation: LibraryRootGeneration::initial(),
            sequence: 1,
            observed_unix_ms: now_unix_ms().unwrap(),
            kind: crate::domain::LibraryChangeObservationKind::Created,
            scope: crate::domain::LibraryChangeScope::Path,
            relative_path: "ingress.png".to_owned(),
            previous_relative_path: None,
            origin: crate::domain::LibraryChangeOrigin::LiveNotification,
        }],
        health: crate::domain::LibraryChangeSourceHealth::Healthy,
        dropped_observation_count: 0,
        ignored_callback_count: 0,
        last_issue_code: None,
    }
}

struct HeldRecoveryWriter {
    release: Option<mpsc::SyncSender<()>>,
    worker: Option<JoinHandle<Result<(), mpsc::RecvTimeoutError>>>,
    held: Arc<AtomicBool>,
    deadline: Option<Instant>,
}

impl HeldRecoveryWriter {
    fn start(mut catalog: SqliteCatalog) -> Self {
        let (held_tx, held_rx) = mpsc::sync_channel(1);
        let (release_tx, release_rx) = mpsc::sync_channel(1);
        let held = Arc::new(AtomicBool::new(false));
        let worker_held = Arc::clone(&held);
        let worker = thread::spawn(move || {
            catalog.with_recovery_write_for_test(|| {
                worker_held.store(true, Ordering::Release);
                held_tx
                    .send(())
                    .expect("report actual recovery transaction");
                let released = release_rx.recv_timeout(Duration::from_secs(10));
                worker_held.store(false, Ordering::Release);
                released
            })
        });
        let owner = Self {
            release: Some(release_tx),
            worker: Some(worker),
            held,
            deadline: None,
        };
        held_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("acquire competing writer");
        owner
    }

    fn is_held(&self) -> bool {
        self.release.is_some()
            && self.held.load(Ordering::Acquire)
            && self
                .worker
                .as_ref()
                .is_some_and(|worker| !worker.is_finished())
    }

    fn finish(&mut self) -> Result<(), String> {
        if let Some(release) = self.release.take() {
            let _ = release.send(());
        }
        let deadline = *self
            .deadline
            .get_or_insert_with(|| Instant::now() + Duration::from_secs(5));
        let Some(worker) = &self.worker else {
            return Ok(());
        };
        while !worker.is_finished() {
            if Instant::now() >= deadline {
                return Err("recovery writer retirement remains unconfirmed".to_owned());
            }
            thread::sleep(Duration::from_millis(1));
        }
        self.worker
            .take()
            .unwrap()
            .join()
            .map_err(|_| "recovery writer panicked".to_owned())?
            .map_err(|error| format!("recovery writer release failed: {error}"))
    }
}

impl Drop for HeldRecoveryWriter {
    fn drop(&mut self) {
        let _ = self.finish();
    }
}
