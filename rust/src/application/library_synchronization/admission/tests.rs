use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use tempfile::TempDir;

use crate::adapters::SqliteCatalog;
use crate::application::StoragePaths;
use crate::application::scan_library::{
    cancel_scan, hold_first_import_capture, pause_scan, suspend_scan,
};
use crate::domain::{
    CatalogFreshnessState, JournalFileReference, JournalIdentifier, JournalUsn,
    LibraryChangeQueuePolicy, LibraryChangeSourceBatch, LibraryChangeSourceError,
    LibraryChangeSourceHealth, LibraryChangeSourceStopReport, LibraryRecoveryAuthorityReason,
    LibraryRootGeneration, LibrarySynchronizationPhase, PERSISTENT_JOURNAL_CONTRACT_VERSION,
    PersistentJournalBaselineStartRequest, PersistentJournalContinuityState,
    PersistentJournalVolumeIdentity, ScanCheckpoint, ScanRequest,
};
use crate::ports::{
    CatalogRepository, IncrementalCatalogRepository, LibraryChangeSource,
    LibraryChangeSourceFactory, LibraryChangeSourceRequest, PersistentJournalRepository,
    erase_library_change_source_factory,
};

use super::super::production::ProductionSynchronizationTestHarness;
use super::SynchronizationAdmissions;

#[derive(Clone, Default)]
struct CountingFactory(Arc<AtomicUsize>);

struct EmptySource;

impl LibraryChangeSourceFactory for CountingFactory {
    type Source = EmptySource;

    fn start(
        &self,
        _: &LibraryChangeSourceRequest,
    ) -> Result<EmptySource, LibraryChangeSourceError> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Ok(EmptySource)
    }
}

impl LibraryChangeSource for EmptySource {
    fn health(&self) -> LibraryChangeSourceHealth {
        LibraryChangeSourceHealth::Healthy
    }

    fn drain(&mut self, _: usize) -> Result<LibraryChangeSourceBatch, LibraryChangeSourceError> {
        Ok(LibraryChangeSourceBatch {
            observations: Vec::new(),
            health: LibraryChangeSourceHealth::Healthy,
            dropped_observation_count: 0,
            ignored_callback_count: 0,
            last_issue_code: None,
        })
    }

    fn stop(&mut self) -> Result<LibraryChangeSourceStopReport, LibraryChangeSourceError> {
        Ok(LibraryChangeSourceStopReport::default())
    }
}

struct Fixture {
    _directory: TempDir,
    storage: StoragePaths,
    request: ScanRequest,
}

impl Fixture {
    fn new(name: &str) -> Self {
        let directory = tempfile::tempdir().expect("fixture directory");
        let source = directory.path().join("source");
        std::fs::create_dir(&source).expect("disposable source");
        let storage = StoragePaths {
            catalog_path: directory.path().join("catalog/ame.sqlite3"),
            preview_root: directory.path().join("previews"),
            preview_budget_bytes: 1024 * 1024,
            settings_path: directory.path().join("settings/storage.sqlite3"),
        };
        let request = ScanRequest {
            scan_id: name.to_owned(),
            root_path: source.to_string_lossy().into_owned(),
            max_items: None,
            max_entries: None,
            preview_edge: 256,
        };
        let mut catalog = SqliteCatalog::open(storage.catalog_path.clone()).expect("new catalog");
        catalog
            .begin_scan(&request, "root", &request.root_path)
            .expect("first import");
        Self {
            _directory: directory,
            storage,
            request,
        }
    }

    fn catalog(&self) -> SqliteCatalog {
        SqliteCatalog::open(self.storage.catalog_path.clone())
            .expect("full catalog contract remains valid")
    }

    fn hold_capture(&self) -> impl Drop {
        crate::application::catalog_session::open_catalog(
            &self.storage.catalog_path,
            crate::domain::LibraryChangeLane::Recovery,
        )
        .expect("prime the production catalog session before protecting its scan");
        hold_first_import_capture(
            &self.request.scan_id,
            &self.storage.catalog_path,
            "root",
            LibraryRootGeneration::initial(),
        )
        .expect("executing first import")
    }

    fn boundary(&self) -> PersistentJournalBaselineStartRequest {
        PersistentJournalBaselineStartRequest {
            run_id: self.request.scan_id.clone(),
            root_id: "root".to_owned(),
            root_generation: LibraryRootGeneration::initial(),
            authority_reason: LibraryRecoveryAuthorityReason::FirstImportBoundary,
            volume: PersistentJournalVolumeIdentity {
                volume_guid: "volume".to_owned(),
                volume_serial: 1,
            },
            root_file_reference: JournalFileReference::V3([3; 16]),
            journal_id: JournalIdentifier::new(1).expect("journal"),
            opening_next_usn: JournalUsn::new(10).expect("USN"),
            protocol_version: crate::journal_broker::PROTOCOL_VERSION,
            contract_version: PERSISTENT_JOURNAL_CONTRACT_VERSION,
            authorized_unix_ms: 1,
        }
    }

    fn seed_boundary(&self) {
        self.catalog()
            .begin_persistent_journal_baseline(
                &self.boundary(),
                LibraryChangeQueuePolicy::default(),
            )
            .expect("first import opening");
    }

    fn runtime(&self, factory: &CountingFactory) -> ProductionSynchronizationTestHarness {
        ProductionSynchronizationTestHarness::with_change_source_for_contract(
            self.storage.clone(),
            erase_library_change_source_factory(factory.clone()),
        )
    }

    fn assert_dormant(&self) {
        let factory = CountingFactory::default();
        let mut runtime = self.runtime(&factory);
        for _ in 0..3 {
            let snapshot = runtime.poll().expect("dormant production poll");
            assert_eq!(snapshot.roots.len(), 1);
            let root = &snapshot.roots[0];
            assert_eq!(
                root.last_issue_code.as_deref(),
                Some("library_first_import_required")
            );
            assert_eq!(
                root.continuity,
                PersistentJournalContinuityState::BaselineRequired
            );
            assert_eq!(root.phase, LibrarySynchronizationPhase::Blocked);
            assert_eq!(root.source_health, LibraryChangeSourceHealth::Stopped);
            assert_ne!(root.freshness, CatalogFreshnessState::Updating);
            assert_eq!(snapshot.applied_mutation_count, 0);
            assert!(
                !runtime.has_scheduled_work(),
                "no P0/P1/P2 worker may be admitted"
            );
        }
        assert_eq!(factory.0.load(Ordering::SeqCst), 0);
        runtime.stop().expect("stop dormant runtime");
        let connection =
            rusqlite::Connection::open(&self.storage.catalog_path).expect("fixture evidence");
        assert_eq!(
            connection
                .query_row("SELECT COUNT(*) FROM scan_runs", [], |row| row
                    .get::<_, u32>(0))
                .expect("scan count"),
            1,
            "polls cannot create replacement scans"
        );
        assert_eq!(
            connection
                .query_row("SELECT COUNT(*) FROM library_roots", [], |row| row
                    .get::<_, u32>(0))
                .expect("root count"),
            1,
            "configured root remains available for manual action"
        );
    }
}

#[test]
fn interrupted_first_import_waits_for_manual_resume_and_keeps_opening_boundary() {
    let fixture = Fixture::new("admission-interrupted");
    fixture.seed_boundary();
    let capture = fixture.hold_capture();
    assert_eq!(
        SynchronizationAdmissions::load(&fixture.catalog())
            .expect("admission")
            .observing_roots()
            .len(),
        1
    );
    drop(capture);
    fixture.assert_dormant();
    assert_eq!(
        fixture
            .catalog()
            .load_persistent_journal_baselines()
            .expect("retained boundary")
            .len(),
        1
    );
    fixture
        .catalog()
        .resume_scan(&fixture.request, "root", &fixture.request.root_path)
        .expect("explicit resume");
    let _capture = fixture.hold_capture();
    assert_eq!(
        SynchronizationAdmissions::load(&fixture.catalog())
            .expect("manual admission")
            .observing_roots()
            .len(),
        1
    );
}

#[test]
fn paused_first_import_does_not_resume_or_start_lanes_after_reopen() {
    let fixture = Fixture::new("admission-paused");
    fixture.seed_boundary();
    fixture
        .catalog()
        .pause_scan(&fixture.request.scan_id, &ScanCheckpoint::default())
        .expect("pause");
    fixture.assert_dormant();
    assert_eq!(
        fixture
            .catalog()
            .load_persistent_journal_baselines()
            .expect("retained boundary")
            .len(),
        1
    );
    fixture
        .catalog()
        .resume_scan(&fixture.request, "root", &fixture.request.root_path)
        .expect("manual resume");
    let _capture = fixture.hold_capture();
    assert_eq!(
        SynchronizationAdmissions::load(&fixture.catalog())
            .expect("resumed admission")
            .observing_roots()
            .len(),
        1
    );
}

#[test]
fn cancelled_and_failed_first_import_retire_capture_atomically_without_removing_root() {
    for status in ["cancelled", "failed"] {
        let fixture = Fixture::new(&format!("admission-{status}"));
        fixture.seed_boundary();
        fixture
            .catalog()
            .abandon_scan(&fixture.request.scan_id, status, 0)
            .expect("terminal first import");
        assert!(
            fixture
                .catalog()
                .load_persistent_journal_baselines()
                .expect("retired capture")
                .is_empty()
        );
        assert!(
            fixture
                .catalog()
                .load_single_recoverable_foreground_scan()
                .expect("recoverable")
                .is_none()
        );
        assert!(
            fixture
                .catalog()
                .load_single_paused_foreground_scan()
                .expect("paused")
                .is_none()
        );
        fixture.assert_dormant();
        assert!(
            fixture
                .catalog()
                .resume_scan(&fixture.request, "root", &fixture.request.root_path)
                .is_err()
        );
    }
}

#[test]
fn executing_first_import_is_observer_first_without_publishing_a_baseline() {
    let fixture = Fixture::new("admission-executing");
    let _capture = fixture.hold_capture();
    let factory = CountingFactory::default();
    let mut runtime = fixture.runtime(&factory);
    let snapshot = runtime.poll().expect("active import capture");
    assert_eq!(
        snapshot.roots[0].source_health,
        LibraryChangeSourceHealth::Healthy
    );
    assert_eq!(factory.0.load(Ordering::SeqCst), 1);
    assert!(
        fixture
            .catalog()
            .load_incremental_catalog_root("root")
            .expect("root")
            .expect("configured")
            .active_scan_id
            .is_none()
    );
    assert!(
        fixture
            .catalog()
            .first_import_change_capture_is_ready(
                &fixture.request.scan_id,
                "root",
                LibraryRootGeneration::initial()
            )
            .expect("capture readiness")
    );
    runtime.stop().expect("stop observer");
}

#[test]
fn cancelling_replacement_keeps_published_baseline_and_live_observer() {
    let fixture = Fixture::new("admission-published");
    let mut catalog = fixture.catalog();
    catalog
        .prove_live_only_first_import_handoff_for_test(&fixture.request.scan_id)
        .expect("capture");
    catalog
        .publish_scan(&fixture.request.scan_id, "root", 0, 0)
        .expect("published baseline");
    let replacement = ScanRequest {
        scan_id: "admission-replacement".to_owned(),
        ..fixture.request.clone()
    };
    catalog
        .begin_scan(&replacement, "root", &replacement.root_path)
        .expect("manual replacement");
    catalog
        .abandon_scan(&replacement.scan_id, "cancelled", 0)
        .expect("cancel replacement");
    drop(catalog);
    let factory = CountingFactory::default();
    let mut runtime = fixture.runtime(&factory);
    let snapshot = runtime.poll().expect("published root still syncs");
    assert_eq!(
        snapshot.roots[0].source_health,
        LibraryChangeSourceHealth::Healthy
    );
    assert_ne!(
        snapshot.roots[0].last_issue_code.as_deref(),
        Some("library_first_import_required")
    );
    assert_eq!(factory.0.load(Ordering::SeqCst), 1);
    runtime.stop().expect("stop");
}

#[test]
fn delayed_journal_opening_cannot_reacquire_paused_or_cancelled_first_import() {
    for status in ["paused", "cancelled"] {
        let fixture = Fixture::new(&format!("admission-late-{status}"));
        fixture.seed_boundary();
        let mut catalog = fixture.catalog();
        if status == "paused" {
            catalog
                .pause_scan(&fixture.request.scan_id, &ScanCheckpoint::default())
                .expect("pause");
        } else {
            catalog
                .abandon_scan(&fixture.request.scan_id, status, 0)
                .expect("cancel");
        }
        let capability = super::super::journal_baseline::live_only_capability(
            "root",
            LibraryRootGeneration::initial(),
            2,
        );
        assert_eq!(
            catalog
                .save_first_import_journal_capability(&capability, &fixture.request.scan_id, || Ok(
                    ()
                ))
                .expect_err("late LiveOnly write rejected")
                .code,
            "persistent_journal_first_import_inactive"
        );
        assert!(
            catalog
                .begin_persistent_journal_baseline(
                    &fixture.boundary(),
                    LibraryChangeQueuePolicy::default()
                )
                .is_err(),
            "even idempotent opening must recheck scan ownership"
        );
        drop(catalog);
        fixture.assert_dormant();
    }
}

#[test]
fn explicit_cancel_cannot_be_downgraded_to_pause_or_suspend() {
    let fixture = Fixture::new("admission-control-order");
    let _capture = fixture.hold_capture();
    assert!(cancel_scan(&fixture.request.scan_id));
    assert!(!pause_scan(&fixture.request.scan_id));
    assert!(!suspend_scan(&fixture.request.scan_id));
    assert!(
        SynchronizationAdmissions::load(&fixture.catalog())
            .expect("cancelled admission")
            .observing_roots()
            .is_empty()
    );
}

#[test]
fn first_import_publication_reclassifies_retired_capture_without_stopping_sync() {
    let fixture = Fixture::new("admission-publish-race");
    let capture = fixture.hold_capture();
    let mut admissions = SynchronizationAdmissions::load(&fixture.catalog()).expect("capturing");
    let mut catalog = fixture.catalog();
    catalog
        .prove_live_only_first_import_handoff_for_test(&fixture.request.scan_id)
        .expect("capture");
    catalog
        .publish_scan(&fixture.request.scan_id, "root", 0, 0)
        .expect("publish empty baseline");
    drop(capture);
    let mut runtime = super::super::LibrarySynchronizationRuntime::new_production(
        erase_library_change_source_factory(CountingFactory::default()),
    );
    let mut snapshot = crate::domain::LibrarySynchronizationSnapshot {
        is_running: true,
        catalog_revision: 0,
        applied_mutation_count: 0,
        roots: Vec::new(),
    };
    admissions
        .revalidate(&catalog, &mut runtime, &mut snapshot)
        .expect("refresh published admission");
    admissions.append_dormant_statuses(&mut snapshot);
    assert_eq!(admissions.observing_roots().len(), 1);
    assert!(
        snapshot.roots.is_empty(),
        "a completed baseline cannot be projected as first-import-required"
    );
    runtime.stop().expect("stop");
}

#[test]
fn first_import_write_admission_rechecks_live_lease_before_any_catalog_changes() {
    for supported in [false, true] {
        let fixture = Fixture::new(&format!("admission-fence-{supported}"));
        let _registration = fixture.hold_capture();
        let capture = crate::application::scan_library::first_import_capture_lease(
            &fixture.storage.catalog_path,
            "root",
            LibraryRootGeneration::initial(),
        )
        .expect("lease")
        .expect("owner");
        let acquire = || {
            assert!(pause_scan(&fixture.request.scan_id));
            capture.acquire_publication()
        };
        let mut catalog = fixture.catalog();
        let error = if supported {
            catalog
                .begin_journal_baseline_with_admission(
                    &fixture.boundary(),
                    LibraryChangeQueuePolicy::default(),
                    acquire,
                )
                .expect_err("revoked supported write")
        } else {
            catalog
                .save_first_import_journal_capability(
                    &super::super::journal_baseline::live_only_capability(
                        "root",
                        LibraryRootGeneration::initial(),
                        2,
                    ),
                    &fixture.request.scan_id,
                    acquire,
                )
                .expect_err("revoked LiveOnly write")
        };
        assert_eq!(error.code, "persistent_journal_first_import_inactive");
        assert!(
            catalog
                .load_persistent_journal_baselines()
                .expect("no boundary")
                .is_empty()
        );
        assert_eq!(
            catalog
                .load_persistent_journal_capabilities()
                .expect("capabilities")[0]
                .state,
            crate::domain::PersistentJournalCapabilityState::Unknown
        );
    }
}

#[test]
fn first_import_reserved_publication_does_not_block_control_and_old_lease_cannot_revive() {
    let fixture = Fixture::new("admission-nonblocking-permit");
    let registration = fixture.hold_capture();
    let old = crate::application::scan_library::first_import_capture_lease(
        &fixture.storage.catalog_path,
        "root",
        LibraryRootGeneration::initial(),
    )
    .expect("lease")
    .expect("owner");
    let mut catalog = fixture.catalog();
    catalog
        .begin_journal_baseline_with_admission(
            &fixture.boundary(),
            LibraryChangeQueuePolicy::default(),
            || {
                let permit = old.acquire_publication()?;
                assert!(
                    pause_scan(&fixture.request.scan_id),
                    "control registration cannot wait on the reserved publication"
                );
                Ok(permit)
            },
        )
        .expect("already reserved opening commits before pause persistence");
    catalog
        .pause_scan(&fixture.request.scan_id, &ScanCheckpoint::default())
        .expect("terminal pause follows opening in the same writer order");
    drop(registration);
    catalog
        .resume_scan(&fixture.request, "root", &fixture.request.root_path)
        .expect("explicit rebuild");
    assert!(
        catalog
            .load_persistent_journal_baselines()
            .expect("old opening discarded")
            .is_empty()
    );
    assert!(
        !catalog
            .first_import_change_capture_is_ready(
                &fixture.request.scan_id,
                "root",
                LibraryRootGeneration::initial()
            )
            .expect("new capture required")
    );
    drop(catalog);
    let _replacement_registration = fixture.hold_capture();
    assert!(
        old.acquire_publication().is_err(),
        "same scan ID cannot reactivate the old token"
    );
    assert_eq!(
        SynchronizationAdmissions::load(&fixture.catalog())
            .expect("new executing registration")
            .observing_roots()
            .len(),
        1
    );
}

#[test]
fn first_import_resume_failure_rolls_back_opening_frontier_and_paused_checkpoint() {
    let fixture = Fixture::new("admission-resume-rollback");
    fixture.seed_boundary();
    let mut catalog = fixture.catalog();
    catalog
        .pause_scan(
            &fixture.request.scan_id,
            &ScanCheckpoint {
                visited_entries: 2,
                ..ScanCheckpoint::default()
            },
        )
        .expect("pause checkpoint");
    let connection =
        rusqlite::Connection::open(&fixture.storage.catalog_path).expect("fixture fault injection");
    connection.execute_batch("CREATE TRIGGER fixture_abort_resume BEFORE UPDATE OF visited_entries ON scan_runs WHEN NEW.visited_entries = 0 BEGIN SELECT RAISE(ABORT, 'fixture resume abort'); END;").expect("inject reset failure");
    assert!(
        catalog
            .resume_scan(&fixture.request, "root", &fixture.request.root_path)
            .is_err()
    );
    connection
        .execute_batch("DROP TRIGGER fixture_abort_resume;")
        .expect("remove owned injection");
    let evidence: (String, i64, i64, i64) = connection.query_row(
        "SELECT status, visited_entries, (SELECT COUNT(*) FROM library_persistent_journal_baselines), (SELECT COUNT(*) FROM scan_directory_frontier) FROM scan_runs WHERE id = ?1",
        [&fixture.request.scan_id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
    ).expect("rollback evidence");
    assert_eq!(evidence, ("paused".to_owned(), 2, 1, 1));
    drop(catalog);
    fixture.catalog();
}

#[test]
fn first_import_publication_after_classification_never_projects_a_new_dormant_claim() {
    let fixture = Fixture::new("admission-late-publication");
    let capture = fixture.hold_capture();
    let mut catalog = fixture.catalog();
    let stale_roster = catalog
        .load_incremental_catalog_roots()
        .expect("old SQL roster");
    let admissions = SynchronizationAdmissions::load(&catalog).expect("capturing classification");
    catalog
        .prove_live_only_first_import_handoff_for_test(&fixture.request.scan_id)
        .expect("opening");
    catalog
        .publish_scan(&fixture.request.scan_id, "root", 0, 0)
        .expect("publish");
    drop(capture);
    let mut snapshot = crate::domain::LibrarySynchronizationSnapshot {
        is_running: true,
        catalog_revision: 0,
        applied_mutation_count: 0,
        roots: Vec::new(),
    };
    admissions.append_dormant_statuses(&mut snapshot);
    assert!(snapshot.roots.is_empty());
    assert_eq!(
        admissions.observing_roots().len(),
        1,
        "an admitted poll does not misinterpret normal completion as observer retirement"
    );
    let refreshed = SynchronizationAdmissions::classify_roots(&catalog, stale_roster)
        .expect("reclassify after missing lease");
    assert!(refreshed.observing_roots()[0].active_scan_id.is_some());
}

#[test]
fn pausing_executing_first_import_retires_observation_before_the_next_poll() {
    let fixture = Fixture::new("admission-active-pause");
    let capture = fixture.hold_capture();
    let factory = CountingFactory::default();
    let mut runtime = fixture.runtime(&factory);
    runtime.poll().expect("observer-first poll");
    assert!(pause_scan(&fixture.request.scan_id));
    fixture
        .catalog()
        .pause_scan(&fixture.request.scan_id, &ScanCheckpoint::default())
        .expect("persist pause");
    drop(capture);
    for _ in 0..3 {
        let snapshot = runtime.poll().expect("paused poll");
        assert_eq!(
            snapshot.roots[0].source_health,
            LibraryChangeSourceHealth::Stopped
        );
        assert_eq!(
            snapshot.roots[0].last_issue_code.as_deref(),
            Some("library_first_import_required")
        );
        assert!(!runtime.has_scheduled_work());
    }
    assert_eq!(
        factory.0.load(Ordering::SeqCst),
        1,
        "no new observer after terminal pause"
    );
    runtime.stop().expect("retire source");
}
