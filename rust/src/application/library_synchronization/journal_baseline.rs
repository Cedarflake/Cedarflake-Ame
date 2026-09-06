use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::adapters::{SqliteCatalog, SqliteCatalogSession};
use crate::domain::{
    JournalFileReference, JournalIdentifier, JournalUsn, LibraryChangeLane,
    LibraryChangeQueuePolicy, LibraryRecoveryAuthorityReason, LibraryRootGeneration,
    LibrarySynchronizationSnapshot, PERSISTENT_JOURNAL_CONTRACT_VERSION, PersistentJournalBaseline,
    PersistentJournalBaselineClosingBoundary, PersistentJournalBaselinePhase,
    PersistentJournalBaselineStartRequest, PersistentJournalCapability,
    PersistentJournalCapabilityState, PersistentJournalContinuityState, PersistentJournalFailure,
    PersistentJournalVolumeIdentity, ScanError,
};
use crate::journal_broker::{
    BrokerResponse, CallerClaim, JournalCapability, PersistentChangeJournalOperationError,
    PersistentChangeJournalSession, ProductionPersistentJournalRoot, QueryJournalRequest,
    RegisterRootRequest, describe_production_persistent_journal_root,
};
use crate::ports::{
    IncrementalCatalogRepository, MetadataInventoryRepository, PersistentJournalRepository,
};

const JOURNAL_BOUNDARY_TIMEOUT_MILLIS: u32 = 10_000;

enum JournalBaselineOpeningAuthority {
    ExistingRoot,
    FirstImport {
        scan_id: String,
        started_unix_ms: i64,
    },
}

pub(super) struct JournalBaselineOpeningWork {
    root_id: String,
    root_generation: LibraryRootGeneration,
    root_path: PathBuf,
    authority: JournalBaselineOpeningAuthority,
}

impl JournalBaselineOpeningWork {
    fn existing_root(
        root_id: String,
        root_generation: LibraryRootGeneration,
        root_path: PathBuf,
    ) -> Self {
        Self {
            root_id,
            root_generation,
            root_path,
            authority: JournalBaselineOpeningAuthority::ExistingRoot,
        }
    }

    fn first_import(
        root_id: String,
        root_generation: LibraryRootGeneration,
        root_path: PathBuf,
        scan_id: String,
        started_unix_ms: i64,
    ) -> Self {
        Self {
            root_id,
            root_generation,
            root_path,
            authority: JournalBaselineOpeningAuthority::FirstImport {
                scan_id,
                started_unix_ms,
            },
        }
    }

    pub(super) fn root_id(&self) -> &str {
        &self.root_id
    }
}

pub(super) struct JournalBaselineClosingWork {
    root_id: String,
    root_generation: LibraryRootGeneration,
    root_path: PathBuf,
    baseline: PersistentJournalBaseline,
}

impl JournalBaselineClosingWork {
    pub(super) fn root_id(&self) -> &str {
        &self.root_id
    }
}

#[derive(Debug, PartialEq, Eq)]
enum JournalBoundaryProbe {
    Supported(SupportedJournalBoundary),
    LiveOnly,
}

#[derive(Debug, PartialEq, Eq)]
struct SupportedJournalBoundary {
    volume: PersistentJournalVolumeIdentity,
    root_file_reference: JournalFileReference,
    journal_id: JournalIdentifier,
    next_usn: JournalUsn,
}

pub(super) fn select_closing_work(
    catalog: &SqliteCatalog,
    snapshot: &LibrarySynchronizationSnapshot,
    root_cursor: Option<&str>,
) -> Result<Option<JournalBaselineClosingWork>, ScanError> {
    let roots = catalog.load_incremental_catalog_roots()?;
    let mut candidates = BTreeMap::new();
    for baseline in catalog.load_persistent_journal_baselines()? {
        if baseline.phase != PersistentJournalBaselinePhase::Inventory
            || !catalog.metadata_inventory_is_waiting_for_closing_boundary(&baseline.run_id)?
        {
            continue;
        }
        let Some(root) = roots.iter().find(|root| {
            root.root_id == baseline.root_id && root.root_generation == baseline.root_generation
        }) else {
            continue;
        };
        if !snapshot_root_is_available_and_healthy(
            snapshot,
            &baseline.root_id,
            baseline.root_generation,
        ) {
            continue;
        }
        candidates.insert(
            baseline.root_id.clone(),
            JournalBaselineClosingWork {
                root_id: baseline.root_id.clone(),
                root_generation: baseline.root_generation,
                root_path: PathBuf::from(&root.root_path),
                baseline,
            },
        );
    }
    Ok(select_candidate(candidates, root_cursor))
}

pub(super) fn select_opening_work(
    catalog: &SqliteCatalog,
    snapshot: &LibrarySynchronizationSnapshot,
    root_cursor: Option<&str>,
) -> Result<Option<JournalBaselineOpeningWork>, ScanError> {
    let capabilities = catalog.load_persistent_journal_capabilities()?;
    let baselines = catalog.load_persistent_journal_baselines()?;
    let mut candidates = BTreeMap::new();
    for root in catalog.load_incremental_catalog_roots()? {
        let Some(status) = snapshot.roots.iter().find(|status| {
            status.root_id == root.root_id
                && status.root_generation == root.root_generation.value()
                && status.availability == crate::domain::LibraryRootAvailability::Available
                && status.source_health == crate::domain::LibraryChangeSourceHealth::Healthy
        }) else {
            continue;
        };
        let root_generation =
            LibraryRootGeneration::new(status.root_generation).ok_or_else(|| {
                ScanError::new(
                    "library_root_generation_invalid",
                    "The synchronization root generation is invalid",
                )
            })?;
        if catalog
            .load_persistent_journal_checkpoint(&root.root_id, root_generation)?
            .is_some()
            || baselines.iter().any(|baseline| {
                baseline.root_id == root.root_id && baseline.root_generation == root_generation
            })
        {
            continue;
        }
        let first_import = if root.active_scan_id.is_none() {
            catalog.pristine_first_import_scan(&root.root_id, root_generation)?
        } else {
            None
        };
        if let Some(capability) = capabilities.iter().find(|capability| {
            capability.root_id == root.root_id && capability.root_generation == root_generation
        }) && !first_import.as_ref().is_some_and(|(_, started_unix_ms)| {
            capability.state == PersistentJournalCapabilityState::LiveOnly
                && capability.continuity == PersistentJournalContinuityState::LiveOnly
                && capability.updated_unix_ms < *started_unix_ms
        }) && (capability.state == PersistentJournalCapabilityState::LiveOnly
            || capability.continuity != PersistentJournalContinuityState::BaselineRequired)
        {
            continue;
        }
        let work = if root.active_scan_id.is_none() {
            let Some((scan_id, started_unix_ms)) = first_import else {
                continue;
            };
            JournalBaselineOpeningWork::first_import(
                root.root_id.clone(),
                root_generation,
                PathBuf::from(&root.root_path),
                scan_id,
                started_unix_ms,
            )
        } else {
            JournalBaselineOpeningWork::existing_root(
                root.root_id.clone(),
                root_generation,
                PathBuf::from(&root.root_path),
            )
        };
        candidates.insert(root.root_id, work);
    }
    Ok(select_candidate(candidates, root_cursor))
}

pub(super) fn persist_unavailable_session(
    catalog: &mut SqliteCatalog,
    work: &JournalBaselineOpeningWork,
    observed_unix_ms: i64,
) -> Result<(), ScanError> {
    catalog.save_persistent_journal_capability(&live_only_capability(
        &work.root_id,
        work.root_generation,
        observed_unix_ms,
    ))
}

pub(super) fn capture_opening_boundary(
    work: JournalBaselineOpeningWork,
    session: &dyn PersistentChangeJournalSession,
    caller: &CallerClaim,
    observed_unix_ms: i64,
    catalog_session: &SqliteCatalogSession,
    queue_policy: LibraryChangeQueuePolicy,
    cancelled: &AtomicBool,
) -> Result<(), ScanError> {
    ensure_boundary_not_cancelled(cancelled, "opening")?;
    let registration = describe_production_persistent_journal_root(
        &work.root_id,
        work.root_generation.value(),
        &work.root_path,
    )
    .map_err(map_journal_operation_error)?;
    let boundary = probe_journal_boundary(session, caller, &registration)?;
    let mut catalog = catalog_session.open_in_lane(LibraryChangeLane::Journal)?;
    match boundary {
        JournalBoundaryProbe::Supported(supported) => {
            catalog.begin_persistent_journal_baseline(
                &opening_request(work, supported, observed_unix_ms),
                queue_policy,
            )?;
        }
        JournalBoundaryProbe::LiveOnly => {
            let observed_unix_ms = work.authorized_unix_ms(observed_unix_ms);
            catalog.save_persistent_journal_capability(&live_only_capability(
                &work.root_id,
                work.root_generation,
                observed_unix_ms,
            ))?;
        }
    }
    drop(registration);
    Ok(())
}

pub(super) fn capture_closing_boundary(
    work: JournalBaselineClosingWork,
    session: &dyn PersistentChangeJournalSession,
    caller: &CallerClaim,
    observed_unix_ms: i64,
    catalog_session: &SqliteCatalogSession,
    cancelled: &AtomicBool,
) -> Result<(), ScanError> {
    ensure_boundary_not_cancelled(cancelled, "closing")?;
    let registration = describe_production_persistent_journal_root(
        &work.root_id,
        work.root_generation.value(),
        &work.root_path,
    )
    .map_err(map_journal_operation_error)?;
    let boundary = probe_journal_boundary(session, caller, &registration)?;
    let closing = closing_request(&work, boundary, observed_unix_ms)?;
    let mut catalog = catalog_session.open_in_lane(LibraryChangeLane::Journal)?;
    catalog.capture_persistent_journal_baseline_closing_boundary(&closing)?;
    drop(registration);
    Ok(())
}

impl JournalBaselineOpeningWork {
    fn authorized_unix_ms(&self, observed_unix_ms: i64) -> i64 {
        match &self.authority {
            JournalBaselineOpeningAuthority::ExistingRoot => observed_unix_ms,
            JournalBaselineOpeningAuthority::FirstImport {
                started_unix_ms, ..
            } => observed_unix_ms.max(*started_unix_ms),
        }
    }
}

fn opening_request(
    work: JournalBaselineOpeningWork,
    boundary: SupportedJournalBoundary,
    observed_unix_ms: i64,
) -> PersistentJournalBaselineStartRequest {
    let SupportedJournalBoundary {
        volume,
        root_file_reference,
        journal_id,
        next_usn,
    } = boundary;
    let authorized_unix_ms = work.authorized_unix_ms(observed_unix_ms);
    let (run_id, authority_reason) = match work.authority {
        JournalBaselineOpeningAuthority::ExistingRoot => (
            crate::application::scan_library::stable_id(
                "persistent-journal-baseline-v1",
                &format!(
                    "{}\0{}\0{}\0{}\0{}\0{}",
                    work.root_id,
                    work.root_generation.value(),
                    volume.volume_guid,
                    volume.volume_serial,
                    journal_id.value(),
                    next_usn.value(),
                ),
            ),
            LibraryRecoveryAuthorityReason::ExistingRootBaseline,
        ),
        JournalBaselineOpeningAuthority::FirstImport { scan_id, .. } => {
            (scan_id, LibraryRecoveryAuthorityReason::FirstImportBoundary)
        }
    };
    PersistentJournalBaselineStartRequest {
        run_id,
        root_id: work.root_id,
        root_generation: work.root_generation,
        authority_reason,
        volume,
        root_file_reference,
        journal_id,
        opening_next_usn: next_usn,
        protocol_version: crate::journal_broker::PROTOCOL_VERSION,
        contract_version: PERSISTENT_JOURNAL_CONTRACT_VERSION,
        authorized_unix_ms,
    }
}

fn closing_request(
    work: &JournalBaselineClosingWork,
    boundary: JournalBoundaryProbe,
    observed_unix_ms: i64,
) -> Result<PersistentJournalBaselineClosingBoundary, ScanError> {
    let SupportedJournalBoundary {
        volume,
        root_file_reference,
        journal_id,
        next_usn,
    } = match boundary {
        JournalBoundaryProbe::Supported(supported) => supported,
        JournalBoundaryProbe::LiveOnly => {
            return Err(ScanError::new(
                "persistent_journal_baseline_closing_unavailable",
                "The closing change boundary is unavailable",
            ));
        }
    };
    Ok(PersistentJournalBaselineClosingBoundary {
        change_id: work.baseline.change_id,
        volume,
        root_file_reference,
        journal_id,
        closing_next_usn: next_usn,
        protocol_version: crate::journal_broker::PROTOCOL_VERSION,
        captured_unix_ms: observed_unix_ms,
    })
}

fn probe_journal_boundary(
    session: &dyn PersistentChangeJournalSession,
    caller: &CallerClaim,
    registration: &ProductionPersistentJournalRoot,
) -> Result<JournalBoundaryProbe, ScanError> {
    let capability = session
        .register_root(RegisterRootRequest {
            caller: caller.clone(),
            root: registration.authorization.clone(),
            client_root_handle: registration.client_root_handle,
            timeout_ms: JOURNAL_BOUNDARY_TIMEOUT_MILLIS,
        })
        .map_err(map_journal_operation_error)?;
    match session
        .query_journal(QueryJournalRequest {
            caller: caller.clone(),
            root: registration.authorization.clone(),
            root_capability: capability,
            timeout_ms: JOURNAL_BOUNDARY_TIMEOUT_MILLIS,
        })
        .map_err(map_journal_operation_error)?
    {
        BrokerResponse::Journal {
            capability: JournalCapability::Supported,
            journal_id: Some(journal_id),
            first_usn: Some(first_usn),
            next_usn: Some(next_usn),
            ..
        } if first_usn >= 0 && next_usn >= first_usn => {
            Ok(JournalBoundaryProbe::Supported(SupportedJournalBoundary {
                volume: PersistentJournalVolumeIdentity {
                    volume_guid: registration.authorization.volume_id.clone(),
                    volume_serial: registration.volume_serial,
                },
                root_file_reference: JournalFileReference::from_bytes(
                    &registration.authorization.root_identity,
                )?,
                journal_id: JournalIdentifier::new(journal_id)?,
                next_usn: JournalUsn::new(next_usn)?,
            }))
        }
        BrokerResponse::Journal {
            capability: JournalCapability::LiveOnly,
            journal_id: None,
            first_usn: None,
            next_usn: None,
            ..
        } => Ok(JournalBoundaryProbe::LiveOnly),
        _ => Err(ScanError::new(
            "persistent_journal_boundary_invalid",
            "The retained change boundary response is incomplete or inconsistent",
        )),
    }
}

fn ensure_boundary_not_cancelled(
    cancelled: &AtomicBool,
    boundary_name: &str,
) -> Result<(), ScanError> {
    if cancelled.load(Ordering::Acquire) {
        return Err(ScanError::new(
            "persistent_journal_boundary_cancelled",
            format!("The {boundary_name} change boundary was cancelled"),
        ));
    }
    Ok(())
}

fn map_journal_operation_error(error: PersistentChangeJournalOperationError) -> ScanError {
    ScanError::new(
        "persistent_journal_boundary_unavailable",
        format!("The retained change boundary is unavailable: {error}"),
    )
}

pub(super) fn live_only_capability(
    root_id: &str,
    root_generation: LibraryRootGeneration,
    observed_unix_ms: i64,
) -> PersistentJournalCapability {
    PersistentJournalCapability {
        root_id: root_id.to_owned(),
        root_generation,
        protocol_version: crate::journal_broker::PROTOCOL_VERSION,
        contract_version: PERSISTENT_JOURNAL_CONTRACT_VERSION,
        state: PersistentJournalCapabilityState::LiveOnly,
        continuity: PersistentJournalContinuityState::LiveOnly,
        failure: Some(PersistentJournalFailure {
            code: "retained_change_history_unavailable".to_owned(),
            message: "Updates remain continuous only while Ame is open".to_owned(),
        }),
        updated_unix_ms: observed_unix_ms,
    }
}

fn snapshot_root_is_available_and_healthy(
    snapshot: &LibrarySynchronizationSnapshot,
    root_id: &str,
    root_generation: LibraryRootGeneration,
) -> bool {
    snapshot.roots.iter().any(|status| {
        status.root_id == root_id
            && status.root_generation == root_generation.value()
            && status.availability == crate::domain::LibraryRootAvailability::Available
            && status.source_health == crate::domain::LibraryChangeSourceHealth::Healthy
    })
}

fn select_candidate<Work>(
    mut candidates: BTreeMap<String, Work>,
    cursor: Option<&str>,
) -> Option<Work> {
    let key = select_rotated_key(candidates.keys(), cursor)?;
    candidates.remove(&key)
}

fn select_rotated_key<'a>(
    keys: impl Iterator<Item = &'a String>,
    cursor: Option<&str>,
) -> Option<String> {
    let mut keys = keys.cloned().collect::<Vec<_>>();
    keys.sort();
    let start = cursor
        .and_then(|cursor| keys.iter().position(|key| key == cursor))
        .map_or(0, |index| (index + 1) % keys.len().max(1));
    keys.get(start).cloned()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::Mutex;

    use super::*;
    use crate::domain::{LibraryChangeId, PersistentJournalBaselinePhase};
    use crate::journal_broker::{
        PersistentChangeJournalRead, ReadJournalRangeRequest, ReadJournalVolumeRequest,
        ResponseBinding, RootCapability,
    };

    struct RecordingBoundarySession {
        registered: Mutex<Option<RegisterRootRequest>>,
        queried: Mutex<Option<QueryJournalRequest>>,
    }

    impl RecordingBoundarySession {
        fn new() -> Self {
            Self {
                registered: Mutex::new(None),
                queried: Mutex::new(None),
            }
        }
    }

    impl PersistentChangeJournalSession for RecordingBoundarySession {
        fn register_root(
            &self,
            request: RegisterRootRequest,
        ) -> Result<RootCapability, PersistentChangeJournalOperationError> {
            *self.registered.lock().expect("record register request") = Some(request);
            Ok(RootCapability([7; 32]))
        }

        fn query_journal(
            &self,
            request: QueryJournalRequest,
        ) -> Result<BrokerResponse, PersistentChangeJournalOperationError> {
            *self.queried.lock().expect("record query request") = Some(request.clone());
            Ok(BrokerResponse::Journal {
                request_id: 1,
                binding: ResponseBinding::from_request(&request.caller, &request.root),
                capability: JournalCapability::Supported,
                journal_id: Some(44),
                first_usn: Some(1),
                next_usn: Some(20),
            })
        }

        fn begin_read_range(
            &self,
            _request: ReadJournalRangeRequest,
        ) -> Result<Box<dyn PersistentChangeJournalRead>, PersistentChangeJournalOperationError>
        {
            Err(PersistentChangeJournalOperationError::InvalidRequest)
        }

        fn begin_read_volume(
            &self,
            _request: ReadJournalVolumeRequest,
        ) -> Result<Box<dyn PersistentChangeJournalRead>, PersistentChangeJournalOperationError>
        {
            Err(PersistentChangeJournalOperationError::InvalidRequest)
        }

        fn close(&self) -> Result<(), PersistentChangeJournalOperationError> {
            Ok(())
        }
    }

    fn supported_boundary() -> JournalBoundaryProbe {
        JournalBoundaryProbe::Supported(SupportedJournalBoundary {
            volume: PersistentJournalVolumeIdentity {
                volume_guid: "volume-a".to_owned(),
                volume_serial: 7,
            },
            root_file_reference: JournalFileReference::V3([3; 16]),
            journal_id: JournalIdentifier::new(44).expect("journal ID"),
            next_usn: JournalUsn::new(20).expect("journal USN"),
        })
    }

    fn supported_opening_boundary() -> SupportedJournalBoundary {
        match supported_boundary() {
            JournalBoundaryProbe::Supported(boundary) => boundary,
            JournalBoundaryProbe::LiveOnly => unreachable!("test boundary is supported"),
        }
    }

    fn inventory_baseline() -> PersistentJournalBaseline {
        PersistentJournalBaseline {
            change_id: LibraryChangeId::new(41).expect("change ID"),
            run_id: "baseline-run".to_owned(),
            root_id: "root-a".to_owned(),
            root_generation: LibraryRootGeneration::initial(),
            volume: PersistentJournalVolumeIdentity {
                volume_guid: "volume-a".to_owned(),
                volume_serial: 7,
            },
            root_file_reference: JournalFileReference::V3([3; 16]),
            journal_id: JournalIdentifier::new(44).expect("journal ID"),
            opening_next_usn: JournalUsn::new(10).expect("opening USN"),
            closing_next_usn: None,
            protocol_version: crate::journal_broker::PROTOCOL_VERSION,
            contract_version: PERSISTENT_JOURNAL_CONTRACT_VERSION,
            phase: PersistentJournalBaselinePhase::Inventory,
            authorized_unix_ms: 90,
            updated_unix_ms: 90,
            completed_unix_ms: None,
        }
    }

    #[test]
    fn opening_authority_api_has_no_partial_first_import_state() {
        fn assert_total(authority: JournalBaselineOpeningAuthority) -> String {
            match authority {
                JournalBaselineOpeningAuthority::ExistingRoot => "existing".to_owned(),
                JournalBaselineOpeningAuthority::FirstImport {
                    scan_id,
                    started_unix_ms,
                } => format!("first-import:{scan_id}:{started_unix_ms}"),
            }
        }

        assert_eq!(
            assert_total(JournalBaselineOpeningAuthority::ExistingRoot),
            "existing"
        );
        assert_eq!(
            assert_total(JournalBaselineOpeningAuthority::FirstImport {
                scan_id: "scan-a".to_owned(),
                started_unix_ms: 100,
            }),
            "first-import:scan-a:100"
        );
    }

    #[test]
    fn existing_root_opening_maps_to_deterministic_baseline_request() {
        let work = JournalBaselineOpeningWork::existing_root(
            "root-a".to_owned(),
            LibraryRootGeneration::initial(),
            PathBuf::from("unused"),
        );
        let request = opening_request(work, supported_opening_boundary(), 100);
        let expected_run_id = crate::application::scan_library::stable_id(
            "persistent-journal-baseline-v1",
            &format!("{}\0{}\0{}\0{}\0{}\0{}", "root-a", 1, "volume-a", 7, 44, 20),
        );

        request.validate().expect("valid existing-root request");
        assert_eq!(request.run_id, expected_run_id);
        assert_eq!(request.root_id, "root-a");
        assert_eq!(
            request.authority_reason,
            LibraryRecoveryAuthorityReason::ExistingRootBaseline
        );
        assert_eq!(request.authorized_unix_ms, 100);
        assert_eq!(request.opening_next_usn.value(), 20);
    }

    #[test]
    fn first_import_opening_maps_to_scan_authority_and_start_time() {
        let work = JournalBaselineOpeningWork::first_import(
            "root-a".to_owned(),
            LibraryRootGeneration::initial(),
            PathBuf::from("unused"),
            "first-import-scan".to_owned(),
            150,
        );
        let request = opening_request(work, supported_opening_boundary(), 100);

        request.validate().expect("valid first-import request");
        assert_eq!(request.run_id, "first-import-scan");
        assert_eq!(
            request.authority_reason,
            LibraryRecoveryAuthorityReason::FirstImportBoundary
        );
        assert_eq!(request.authorized_unix_ms, 150);
    }

    #[test]
    fn boundary_probe_binds_register_and_query_requests_to_the_same_root() {
        let directory = tempfile::tempdir().expect("test directory");
        let source_root = directory.path().join("source");
        std::fs::create_dir_all(&source_root).expect("source root");
        let registration = describe_production_persistent_journal_root(
            "root-a",
            LibraryRootGeneration::initial().value(),
            &source_root,
        )
        .expect("describe journal root");
        let caller = CallerClaim {
            process_id: std::process::id(),
            session_id: 1,
            client_instance: [9; 16],
        };
        let session = RecordingBoundarySession::new();

        let boundary = probe_journal_boundary(&session, &caller, &registration)
            .expect("probe journal boundary");

        let register = session
            .registered
            .lock()
            .expect("registered request")
            .clone()
            .expect("register request");
        let query = session
            .queried
            .lock()
            .expect("queried request")
            .clone()
            .expect("query request");
        assert_eq!(register.caller, caller);
        assert_eq!(register.root, registration.authorization);
        assert_eq!(register.client_root_handle, registration.client_root_handle);
        assert_eq!(register.timeout_ms, JOURNAL_BOUNDARY_TIMEOUT_MILLIS);
        assert_eq!(query.caller, caller);
        assert_eq!(query.root, register.root);
        assert_eq!(query.root_capability, RootCapability([7; 32]));
        assert_eq!(query.timeout_ms, JOURNAL_BOUNDARY_TIMEOUT_MILLIS);
        assert!(matches!(
            boundary,
            JournalBoundaryProbe::Supported(SupportedJournalBoundary { next_usn, .. })
                if next_usn.value() == 20
        ));
    }

    #[test]
    fn cancelled_opening_stops_before_boundary_registration() {
        let directory = tempfile::tempdir().expect("test directory");
        let catalog_session =
            SqliteCatalogSession::validate(directory.path().join("catalog").join("ame.sqlite3"))
                .expect("catalog session");
        let session = RecordingBoundarySession::new();
        let cancelled = AtomicBool::new(true);
        let work = JournalBaselineOpeningWork::existing_root(
            "root-a".to_owned(),
            LibraryRootGeneration::initial(),
            directory.path().join("missing-source"),
        );

        let error = capture_opening_boundary(
            work,
            &session,
            &CallerClaim {
                process_id: std::process::id(),
                session_id: 1,
                client_instance: [8; 16],
            },
            100,
            &catalog_session,
            LibraryChangeQueuePolicy::default(),
            &cancelled,
        )
        .expect_err("cancelled opening must stop");

        assert_eq!(error.code, "persistent_journal_boundary_cancelled");
        assert!(
            session
                .registered
                .lock()
                .expect("registered request")
                .is_none()
        );
        assert!(session.queried.lock().expect("queried request").is_none());
    }

    #[test]
    fn closing_work_carries_its_baseline_and_rejects_live_only_boundary() {
        let baseline = inventory_baseline();
        let work = JournalBaselineClosingWork {
            root_id: baseline.root_id.clone(),
            root_generation: baseline.root_generation,
            root_path: PathBuf::from("unused"),
            baseline,
        };
        let closing =
            closing_request(&work, supported_boundary(), 200).expect("supported closing boundary");
        assert_eq!(closing.change_id, work.baseline.change_id);
        assert_eq!(closing.closing_next_usn.value(), 20);
        assert_eq!(closing.captured_unix_ms, 200);

        let error = closing_request(&work, JournalBoundaryProbe::LiveOnly, 200)
            .expect_err("live-only cannot close a retained baseline");
        assert_eq!(
            error.code,
            "persistent_journal_baseline_closing_unavailable"
        );
    }

    #[test]
    fn broker_failure_maps_to_boundary_unavailable() {
        let error = map_journal_operation_error(
            PersistentChangeJournalOperationError::TransportUnavailable,
        );

        assert_eq!(error.code, "persistent_journal_boundary_unavailable");
        assert!(error.message.contains("transport is unavailable"));
    }
}
