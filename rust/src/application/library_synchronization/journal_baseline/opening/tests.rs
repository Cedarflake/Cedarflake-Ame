use crate::application::scan_library::{
    cancel_scan, first_import_capture_lease, hold_first_import_capture, pause_scan, suspend_scan,
};
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::domain::{
    LibraryRootGeneration, PersistentJournalCapabilityState, ScanCheckpoint, ScanRequest,
};
use crate::journal_broker::{
    BrokerResponse, JournalCapability, PersistentChangeJournalOperationError,
    PersistentChangeJournalRead, QueryJournalRequest, ReadJournalRangeRequest,
    ReadJournalVolumeRequest, RegisterRootRequest, ResponseBinding, RootCapability,
};
use crate::ports::{CatalogRepository, IncrementalCatalogRepository};

use super::*;

fn with_first_import<Outcome>(
    scan_id: &str,
    check: impl FnOnce(&mut SqliteCatalog, JournalBaselineOpeningWork) -> Outcome,
) -> Outcome {
    let directory = tempfile::tempdir().expect("isolated first import");
    let source = directory.path().join("source");
    std::fs::create_dir(&source).expect("generated source directory");
    let catalog_path = directory.path().join("catalog/ame.sqlite3");
    let mut catalog = SqliteCatalog::open(catalog_path.clone()).expect("catalog");
    let request = ScanRequest {
        scan_id: scan_id.to_owned(),
        root_path: source.to_string_lossy().into_owned(),
        max_items: None,
        max_entries: None,
        preview_edge: 128,
    };
    catalog
        .begin_scan(&request, "root", &request.root_path)
        .expect("unpublished first import");
    let _registration = hold_first_import_capture(
        scan_id,
        &catalog_path,
        "root",
        LibraryRootGeneration::initial(),
    )
    .expect("executing capture owner");
    let capture =
        first_import_capture_lease(&catalog_path, "root", LibraryRootGeneration::initial())
            .expect("capture registry")
            .expect("current capture");
    let work = JournalBaselineOpeningWork::first_import(
        "root".to_owned(),
        LibraryRootGeneration::initial(),
        source,
        capture,
        1,
    );
    check(&mut catalog, work)
}

#[test]
fn accepted_control_retires_selected_opening_without_failing_or_publishing() {
    for (name, command) in [
        ("pause", pause_scan as fn(&str) -> bool),
        ("cancel", cancel_scan),
        ("suspend", suspend_scan),
    ] {
        let scan_id = format!("opening-retired-{name}");
        with_first_import(&scan_id, |catalog, work| {
            assert!(command(&scan_id));
            assert_eq!(
                persist_unavailable_session(catalog, &work, 2)
                    .expect("selected opening retires after accepted control"),
                OpeningCaptureOutcome::Retired,
            );
            assert_unrecorded(catalog);
        });
    }
}

fn assert_unrecorded(catalog: &SqliteCatalog) {
    let capabilities = catalog
        .load_persistent_journal_capabilities()
        .expect("capability evidence");
    assert_eq!(capabilities.len(), 1);
    assert_eq!(
        capabilities[0].state,
        PersistentJournalCapabilityState::Unknown
    );
    assert!(
        catalog
            .load_persistent_journal_baselines()
            .expect("no baseline was granted")
            .is_empty()
    );
    assert!(
        catalog
            .load_incremental_catalog_root("root")
            .expect("unpublished root")
            .expect("retained root")
            .active_scan_id
            .is_none()
    );
}

#[test]
fn current_live_only_opening_records_capability_without_publishing_catalog() {
    with_first_import("opening-current", |catalog, work| {
        assert_eq!(
            persist_unavailable_session(catalog, &work, 2).expect("record live-only capability"),
            OpeningCaptureOutcome::Recorded,
        );
        let capability = &catalog
            .load_persistent_journal_capabilities()
            .expect("capability evidence")[0];
        assert_eq!(capability.state, PersistentJournalCapabilityState::LiveOnly);
        assert_eq!(capability.updated_unix_ms, 2);
        assert!(
            catalog
                .load_persistent_journal_baselines()
                .expect("baselines")
                .is_empty()
        );
        assert!(
            catalog
                .load_incremental_catalog_root("root")
                .expect("root")
                .expect("retained root")
                .active_scan_id
                .is_none()
        );
    });
}

#[test]
fn control_at_sqlite_admission_retires_the_rejected_write() {
    with_first_import("opening-write-revoked", |catalog, work| {
        let JournalBaselineOpeningAuthority::FirstImport { capture, .. } = &work.authority else {
            panic!("first-import work");
        };
        let result = catalog.save_first_import_journal_capability(
            &live_only_capability("root", work.root_generation, 2),
            capture.scan_id(),
            || {
                assert!(pause_scan(capture.scan_id()));
                capture.acquire_publication()
            },
        );
        assert_eq!(
            work.complete_publication(result)
                .expect("retire rejected publication"),
            OpeningCaptureOutcome::Retired,
        );
        assert_unrecorded(catalog);
    });
}

#[test]
fn control_after_write_admission_preserves_the_committed_receipt() {
    with_first_import("opening-write-admitted", |catalog, work| {
        let JournalBaselineOpeningAuthority::FirstImport { capture, .. } = &work.authority else {
            panic!("first-import work");
        };
        let result = catalog.save_first_import_journal_capability(
            &live_only_capability("root", work.root_generation, 2),
            capture.scan_id(),
            || {
                let permit = capture.acquire_publication()?;
                assert!(pause_scan(capture.scan_id()));
                Ok(permit)
            },
        );
        assert!(!capture.is_current());
        assert_eq!(
            work.complete_publication(result)
                .expect("preserve committed receipt"),
            OpeningCaptureOutcome::Recorded,
        );
        assert_eq!(
            catalog
                .load_persistent_journal_capabilities()
                .expect("capability")[0]
                .state,
            PersistentJournalCapabilityState::LiveOnly
        );
        catalog
            .pause_scan(capture.scan_id(), &ScanCheckpoint::default())
            .expect("terminal pause follows committed capture");
    });
}

#[test]
fn inactive_catalog_authority_with_current_execution_is_still_an_error() {
    with_first_import("opening-catalog-retired", |catalog, work| {
        catalog
            .pause_scan("opening-catalog-retired", &ScanCheckpoint::default())
            .expect("persist retired scan");
        assert!(work.is_current(), "execution token was not controlled");
        let error = persist_unavailable_session(catalog, &work, 2)
            .expect_err("do not hide catalog authority mismatch");
        assert_eq!(error.code, "persistent_journal_first_import_inactive");
        assert_unrecorded(catalog);
    });
}

#[test]
fn revoked_execution_does_not_hide_unrelated_publication_errors() {
    with_first_import("opening-other-error", |_, work| {
        assert!(cancel_scan("opening-other-error"));
        for code in [
            "catalog_database_error",
            "persistent_journal_root_authority_stale",
        ] {
            let error = work
                .complete_publication(Err(ScanError::new(code, "original failure")))
                .expect_err("unrelated failure remains visible");
            assert_eq!(error.code, code);
            assert_eq!(error.message, "original failure");
        }
    });
}

#[derive(Clone, Copy)]
enum ProbeResponse {
    Supported,
    LiveOnly,
    Unavailable,
}

struct ControlledSession<'a> {
    scan_id: &'a str,
    command: Option<fn(&str) -> bool>,
    response: ProbeResponse,
    calls: AtomicUsize,
}

impl PersistentChangeJournalSession for ControlledSession<'_> {
    fn register_root(
        &self,
        _: RegisterRootRequest,
    ) -> Result<RootCapability, PersistentChangeJournalOperationError> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        Ok(RootCapability([7; 32]))
    }

    fn query_journal(
        &self,
        request: QueryJournalRequest,
    ) -> Result<BrokerResponse, PersistentChangeJournalOperationError> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        if let Some(command) = self.command {
            assert!(command(self.scan_id));
        }
        let (capability, journal_id, first_usn, next_usn) = match self.response {
            ProbeResponse::Supported => (JournalCapability::Supported, Some(44), Some(1), Some(20)),
            ProbeResponse::LiveOnly => (JournalCapability::LiveOnly, None, None, None),
            ProbeResponse::Unavailable => {
                return Err(PersistentChangeJournalOperationError::TransportUnavailable);
            }
        };
        Ok(BrokerResponse::Journal {
            request_id: 1,
            binding: ResponseBinding::from_request(&request.caller, &request.root),
            capability,
            journal_id,
            first_usn,
            next_usn,
        })
    }

    fn begin_read_range(
        &self,
        _: ReadJournalRangeRequest,
    ) -> Result<Box<dyn PersistentChangeJournalRead>, PersistentChangeJournalOperationError> {
        Err(PersistentChangeJournalOperationError::InvalidRequest)
    }

    fn begin_read_volume(
        &self,
        _: ReadJournalVolumeRequest,
    ) -> Result<Box<dyn PersistentChangeJournalRead>, PersistentChangeJournalOperationError> {
        Err(PersistentChangeJournalOperationError::InvalidRequest)
    }

    fn close(&self) -> Result<(), PersistentChangeJournalOperationError> {
        Ok(())
    }
}

fn capture_with_session(
    catalog: &SqliteCatalog,
    work: JournalBaselineOpeningWork,
    session: &ControlledSession<'_>,
) -> Result<OpeningCaptureOutcome, ScanError> {
    let catalog_session = SqliteCatalogSession::validate(catalog.catalog_path().to_path_buf())
        .expect("validated catalog session");
    capture_opening_boundary(
        work,
        session,
        &CallerClaim {
            process_id: std::process::id(),
            session_id: 1,
            client_instance: [8; 16],
        },
        2,
        &catalog_session,
        LibraryChangeQueuePolicy::default(),
        &AtomicBool::new(false),
    )
}

#[test]
fn revoked_opening_does_not_access_source_or_broker() {
    with_first_import("opening-before-probe", |catalog, mut work| {
        assert!(pause_scan("opening-before-probe"));
        work.root_path = work.root_path.join("missing-root");
        let session = ControlledSession {
            scan_id: "opening-before-probe",
            command: None,
            response: ProbeResponse::Unavailable,
            calls: AtomicUsize::new(0),
        };
        assert_eq!(
            capture_with_session(catalog, work, &session).expect("retire before source admission"),
            OpeningCaptureOutcome::Retired
        );
        assert_eq!(session.calls.load(Ordering::Relaxed), 0);
        assert_unrecorded(catalog);
    });
}

#[test]
fn control_during_supported_or_live_only_probe_retires_without_publishing() {
    for (name, command) in [
        ("pause", pause_scan as fn(&str) -> bool),
        ("cancel", cancel_scan),
        ("suspend", suspend_scan),
    ] {
        for (probe, response) in [
            ("supported", ProbeResponse::Supported),
            ("live-only", ProbeResponse::LiveOnly),
        ] {
            let scan_id = format!("opening-probe-{name}-{probe}");
            with_first_import(&scan_id, |catalog, work| {
                let session = ControlledSession {
                    scan_id: &scan_id,
                    command: Some(command),
                    response,
                    calls: AtomicUsize::new(0),
                };
                assert_eq!(
                    capture_with_session(catalog, work, &session).expect("retire controlled probe"),
                    OpeningCaptureOutcome::Retired
                );
                assert_eq!(session.calls.load(Ordering::Relaxed), 2);
                assert_unrecorded(catalog);
            });
        }
    }
}

#[test]
fn probe_failure_remains_visible_when_control_was_accepted() {
    with_first_import("opening-probe-error", |catalog, work| {
        let session = ControlledSession {
            scan_id: "opening-probe-error",
            command: Some(pause_scan),
            response: ProbeResponse::Unavailable,
            calls: AtomicUsize::new(0),
        };
        let error = capture_with_session(catalog, work, &session)
            .expect_err("transport failure is not retirement");
        assert_eq!(error.code, "persistent_journal_boundary_unavailable");
        assert_eq!(session.calls.load(Ordering::Relaxed), 2);
        assert_unrecorded(catalog);
    });
}

#[test]
fn current_probe_records_its_supported_or_live_only_boundary() {
    for (name, response, expected_capability, expected_baselines) in [
        (
            "supported",
            ProbeResponse::Supported,
            PersistentJournalCapabilityState::Supported,
            1,
        ),
        (
            "live-only",
            ProbeResponse::LiveOnly,
            PersistentJournalCapabilityState::LiveOnly,
            0,
        ),
    ] {
        let scan_id = format!("opening-current-probe-{name}");
        with_first_import(&scan_id, |catalog, work| {
            let session = ControlledSession {
                scan_id: &scan_id,
                command: None,
                response,
                calls: AtomicUsize::new(0),
            };
            assert_eq!(
                capture_with_session(catalog, work, &session).expect("record current boundary"),
                OpeningCaptureOutcome::Recorded
            );
            assert_eq!(session.calls.load(Ordering::Relaxed), 2);
            assert_eq!(
                catalog
                    .load_persistent_journal_capabilities()
                    .expect("capability")[0]
                    .state,
                expected_capability
            );
            let baselines = catalog
                .load_persistent_journal_baselines()
                .expect("baselines");
            assert_eq!(baselines.len(), expected_baselines);
            if let Some(baseline) = baselines.first() {
                assert_eq!(baseline.run_id, scan_id);
                assert_eq!(baseline.root_id, "root");
            }
        });
    }
}

#[test]
fn retired_work_cannot_borrow_replacement_execution_authority() {
    let scan_id = "opening-replaced-registration";
    let (old_work, catalog_path) = with_first_import(scan_id, |catalog, work| {
        (work, catalog.catalog_path().to_path_buf())
    });
    let _replacement = hold_first_import_capture(
        scan_id,
        &catalog_path,
        "root",
        LibraryRootGeneration::initial(),
    )
    .expect("new same-ID execution");
    let current =
        first_import_capture_lease(&catalog_path, "root", LibraryRootGeneration::initial())
            .expect("registry")
            .expect("new lease");
    assert!(current.is_current());
    let JournalBaselineOpeningAuthority::FirstImport { capture, .. } = &old_work.authority else {
        panic!("old first-import work");
    };
    assert!(!capture.is_current());
    let rejected = capture.acquire_publication().map(|_| ());
    assert!(
        rejected.is_err(),
        "old work cannot use the new execution's permit"
    );
    assert_eq!(
        old_work
            .complete_publication(rejected)
            .expect("retired old result"),
        OpeningCaptureOutcome::Retired
    );
}
