use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::application::scan_library::{FirstImportCaptureLease, first_import_capture_lease};
use crate::domain::{
    LibraryChangeLane, LibraryRootGeneration, LibrarySynchronizationSnapshot,
    PERSISTENT_JOURNAL_CONTRACT_VERSION, PersistentJournalBaseline,
    PersistentJournalBaselineClosingBoundary, PersistentJournalBaselinePhase,
    PersistentJournalCapability, PersistentJournalCapabilityState,
    PersistentJournalContinuityState, PersistentJournalFailure, ScanError,
};
use crate::ports::catalog_session::CatalogLaneSession;
use crate::ports::journal_boundary::{
    JournalBoundaryProbe, JournalBoundaryReader, JournalBoundarySource, SupportedJournalBoundary,
};
use crate::ports::scan_admission::FirstImportReadiness;
use crate::ports::{
    CatalogRepository, IncrementalCatalogRepository, MetadataInventoryRepository,
    PersistentJournalRepository,
};

mod opening;

pub(super) use opening::{capture_opening_boundary, persist_unavailable_session};

enum JournalBaselineOpeningAuthority {
    ExistingRoot,
    FirstImport {
        capture: FirstImportCaptureLease,
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
        capture: FirstImportCaptureLease,
        started_unix_ms: i64,
    ) -> Self {
        Self {
            root_id,
            root_generation,
            root_path,
            authority: JournalBaselineOpeningAuthority::FirstImport {
                capture,
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
    pub(super) fn root_generation(&self) -> LibraryRootGeneration {
        self.root_generation
    }

    pub(super) fn root_id(&self) -> &str {
        &self.root_id
    }
}

pub(super) fn select_closing_work(
    catalog: &(
         impl IncrementalCatalogRepository + PersistentJournalRepository + MetadataInventoryRepository
     ),
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
    catalog: &(
         impl CatalogRepository
         + IncrementalCatalogRepository
         + PersistentJournalRepository
         + FirstImportReadiness
     ),
    snapshot: &LibrarySynchronizationSnapshot,
    root_cursor: Option<&str>,
    identity_source: Option<&dyn crate::ports::PersistentIdentityJournalSource>,
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
        }) {
            if capability.state == PersistentJournalCapabilityState::LiveOnly {
                let first_import_is_newer = first_import.as_ref().is_some_and(|(_, started)| {
                    capability.continuity == PersistentJournalContinuityState::LiveOnly
                        && capability.updated_unix_ms < *started
                });
                let new_source_can_probe = identity_source.is_some_and(|source| {
                    source.can_probe_baseline(&crate::ports::IdentityJournalRoot {
                        root_id: root.root_id.clone(),
                        root_generation,
                    })
                });
                if !first_import_is_newer && !new_source_can_probe {
                    continue;
                }
            } else if capability.continuity != PersistentJournalContinuityState::BaselineRequired {
                continue;
            }
        }
        let work = if root.active_scan_id.is_none() {
            let Some((scan_id, started_unix_ms)) = first_import else {
                continue;
            };
            let Some(capture) =
                first_import_capture_lease(catalog.catalog_path(), &root.root_id, root_generation)?
            else {
                continue;
            };
            if capture.scan_id() != scan_id {
                continue;
            }
            JournalBaselineOpeningWork::first_import(
                root.root_id.clone(),
                root_generation,
                PathBuf::from(&root.root_path),
                capture,
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

pub(super) fn capture_closing_boundary<Session>(
    work: JournalBaselineClosingWork,
    boundary_source: &impl JournalBoundarySource,
    observed_unix_ms: i64,
    catalog_session: &Session,
    cancelled: &AtomicBool,
) -> Result<(), ScanError>
where
    Session: CatalogLaneSession,
    Session::Catalog: PersistentJournalRepository,
{
    ensure_boundary_not_cancelled(cancelled, "closing")?;
    let registration =
        boundary_source.open_root(&work.root_id, work.root_generation, &work.root_path)?;
    let boundary = registration.read_boundary()?;
    let closing = closing_request(&work, boundary, observed_unix_ms)?;
    let mut catalog = catalog_session.open_in_lane(LibraryChangeLane::Journal)?;
    catalog.capture_persistent_journal_baseline_closing_boundary(&closing)?;
    drop(registration);
    Ok(())
}

fn closing_request(
    work: &JournalBaselineClosingWork,
    boundary: JournalBoundaryProbe,
    observed_unix_ms: i64,
) -> Result<PersistentJournalBaselineClosingBoundary, ScanError> {
    let SupportedJournalBoundary {
        protocol_version,
        volume,
        root_file_reference,
        journal_id,
        next_usn,
        ..
    } = match boundary {
        JournalBoundaryProbe::Supported(supported) => supported,
        JournalBoundaryProbe::LiveOnly { .. } => {
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
        protocol_version,
        captured_unix_ms: observed_unix_ms,
    })
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

pub(super) fn live_only_capability(
    root_id: &str,
    root_generation: LibraryRootGeneration,
    observed_unix_ms: i64,
    protocol_version: u16,
) -> PersistentJournalCapability {
    PersistentJournalCapability {
        root_id: root_id.to_owned(),
        root_generation,
        protocol_version,
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
    use crate::adapters::SqliteCatalogSession;
    use std::path::PathBuf;

    use super::opening::opening_request;
    use super::*;
    use crate::domain::{
        JournalFileReference, JournalIdentifier, JournalUsn, PersistentJournalVolumeIdentity,
    };
    use crate::domain::{
        LibraryChangeId, LibraryChangeQueuePolicy, LibraryRecoveryAuthorityReason,
        PersistentJournalBaselinePhase,
    };

    struct UnopenedBoundarySource;

    impl JournalBoundarySource for UnopenedBoundarySource {
        type Root = Self;

        fn open_root(
            &self,
            _: &str,
            _: LibraryRootGeneration,
            _: &std::path::Path,
        ) -> Result<Self::Root, ScanError> {
            panic!("cancelled opening must not acquire a root")
        }
    }

    impl JournalBoundaryReader for UnopenedBoundarySource {
        fn read_boundary(&self) -> Result<JournalBoundaryProbe, ScanError> {
            panic!("cancelled opening must not read a boundary")
        }
    }

    fn supported_boundary() -> JournalBoundaryProbe {
        JournalBoundaryProbe::Supported(SupportedJournalBoundary {
            protocol_version: 7,
            volume: PersistentJournalVolumeIdentity {
                volume_guid: "volume-a".to_owned(),
                volume_serial: 7,
            },
            root_file_reference: JournalFileReference::V3([3; 16]),
            journal_id: JournalIdentifier::new(44).expect("journal ID"),
            first_usn: JournalUsn::new(0).expect("first journal USN"),
            next_usn: JournalUsn::new(20).expect("journal USN"),
        })
    }

    fn supported_opening_boundary() -> SupportedJournalBoundary {
        match supported_boundary() {
            JournalBoundaryProbe::Supported(boundary) => boundary,
            JournalBoundaryProbe::LiveOnly { .. } => unreachable!("test boundary is supported"),
        }
    }

    fn test_capture(scan_id: &str) -> (impl Drop, FirstImportCaptureLease) {
        let catalog_path = PathBuf::from(format!("unused-{scan_id}"));
        let registration = crate::application::scan_library::hold_first_import_capture(
            scan_id,
            &catalog_path,
            "root-a",
            LibraryRootGeneration::initial(),
        )
        .expect("capture registration");
        let capture =
            first_import_capture_lease(&catalog_path, "root-a", LibraryRootGeneration::initial())
                .expect("registry")
                .expect("capture");
        (registration, capture)
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
                    capture,
                    started_unix_ms,
                } => format!("first-import:{}:{started_unix_ms}", capture.scan_id()),
            }
        }

        assert_eq!(
            assert_total(JournalBaselineOpeningAuthority::ExistingRoot),
            "existing"
        );
        let (_registration, capture) = test_capture("authority-api-scan");
        assert_eq!(
            assert_total(JournalBaselineOpeningAuthority::FirstImport {
                capture,
                started_unix_ms: 100,
            }),
            "first-import:authority-api-scan:100"
        );
    }

    #[test]
    fn existing_root_opening_maps_to_deterministic_baseline_request() {
        let work = JournalBaselineOpeningWork::existing_root(
            "root-a".to_owned(),
            LibraryRootGeneration::initial(),
            PathBuf::from("unused"),
        );
        let request = opening_request(&work, supported_opening_boundary(), 100);
        let expected_run_id = crate::domain::stable_id(
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
        assert_eq!(request.protocol_version, 7);
    }

    #[test]
    fn first_import_opening_maps_to_scan_authority_and_start_time() {
        let (_registration, capture) = test_capture("first-import-scan");
        let work = JournalBaselineOpeningWork::first_import(
            "root-a".to_owned(),
            LibraryRootGeneration::initial(),
            PathBuf::from("unused"),
            capture,
            150,
        );
        let request = opening_request(&work, supported_opening_boundary(), 100);

        request.validate().expect("valid first-import request");
        assert_eq!(request.run_id, "first-import-scan");
        assert_eq!(
            request.authority_reason,
            LibraryRecoveryAuthorityReason::FirstImportBoundary
        );
        assert_eq!(request.authorized_unix_ms, 150);
    }

    #[test]
    fn cancelled_opening_stops_before_boundary_registration() {
        let directory = tempfile::tempdir().expect("test directory");
        let catalog_session =
            SqliteCatalogSession::validate(directory.path().join("catalog").join("ame.sqlite3"))
                .expect("catalog session");
        let cancelled = AtomicBool::new(true);
        let work = JournalBaselineOpeningWork::existing_root(
            "root-a".to_owned(),
            LibraryRootGeneration::initial(),
            directory.path().join("missing-source"),
        );

        let error = capture_opening_boundary(
            work,
            &UnopenedBoundarySource,
            100,
            &catalog_session,
            LibraryChangeQueuePolicy::default(),
            &cancelled,
        )
        .expect_err("cancelled opening must stop");

        assert_eq!(error.code, "persistent_journal_boundary_cancelled");
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
        assert_eq!(closing.protocol_version, 7);

        let error = closing_request(
            &work,
            JournalBoundaryProbe::LiveOnly {
                protocol_version: 7,
            },
            200,
        )
        .expect_err("live-only cannot close a retained baseline");
        assert_eq!(
            error.code,
            "persistent_journal_baseline_closing_unavailable"
        );
    }
}
