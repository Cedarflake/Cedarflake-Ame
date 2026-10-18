use std::sync::atomic::AtomicUsize;

use crate::domain::{
    JournalFileReference, LibraryChangeScope, LibraryRootGeneration,
    PersistentJournalContinuityState, PersistentJournalVolumeIdentity,
};
use crate::journal_broker::{
    PersistentChangeJournalOperationError, PersistentChangeJournalRead, ReadJournalRangeRequest,
    ReadJournalVolumeRequest, ResponseBinding, RootAuthorization, RootCapability,
};
use crate::ports::{
    IdentityJournalLocation, IdentityJournalPage, IdentityJournalRecord, IdentityJournalResolution,
};

use super::*;

fn checkpoint() -> PersistentJournalCheckpoint {
    PersistentJournalCheckpoint {
        root_id: "root".to_owned(),
        root_generation: LibraryRootGeneration::initial(),
        volume: PersistentJournalVolumeIdentity {
            volume_guid: "volume".to_owned(),
            volume_serial: 7,
        },
        root_file_reference: JournalFileReference::from_bytes(&[3; 16]).unwrap(),
        journal_id: JournalIdentifier::new(9).unwrap(),
        next_unread_usn: JournalUsn::new(10).unwrap(),
        captured_exclusive_end: JournalUsn::new(10).unwrap(),
        covered_catalog_revision: 1,
        protocol_version: crate::journal_broker::PROTOCOL_VERSION,
        contract_version: 1,
        continuity: PersistentJournalContinuityState::Current,
        failure: None,
        updated_unix_ms: 1,
    }
}

fn reference(id: u64) -> JournalFileReference {
    JournalFileReference::from_bytes(&id.to_le_bytes()).unwrap()
}

fn location(id: u64, path: &str, directory: bool) -> IdentityJournalLocation {
    IdentityJournalLocation {
        identity: identity_for_reference(7, &reference(id)),
        relative_path: path.to_owned(),
        is_directory: directory,
    }
}

fn page(directory: bool) -> IdentityJournalPage {
    IdentityJournalPage {
        covered_until_usn: 20,
        records: vec![IdentityJournalRecord {
            file_reference: reference(2),
            parent_reference: reference(3),
            usn: 12,
            is_directory: directory,
            is_deleted: false,
        }],
    }
}

#[test]
fn one_identity_record_enrolls_every_hardlink_without_fabricating_usn_order() {
    let old = vec![
        location(2, "first.png", false),
        location(2, "album/second.png", false),
    ];
    let current = IdentityJournalResolution {
        locations: old.clone(),
        ..Default::default()
    };
    let (batch, next) =
        translation::translate(&checkpoint(), &page(false), 20, 2, &old, &current).unwrap();
    assert_eq!(batch.intents.len(), 2);
    assert!(
        batch
            .intents
            .iter()
            .all(|intent| intent.first_sequence == 12 && intent.scope == LibraryChangeScope::Path)
    );
    batch.validate().unwrap();
    assert_eq!(next.next_unread_usn.value(), 20);
}

#[test]
fn case_sensitive_sibling_paths_remain_distinct_candidates() {
    let old = vec![location(2, "A.jpg", false), location(2, "a.jpg", false)];
    let current = IdentityJournalResolution {
        locations: old.clone(),
        ..Default::default()
    };

    let (batch, next) =
        translation::translate(&checkpoint(), &page(false), 20, 2, &old, &current).unwrap();

    let paths = batch
        .intents
        .iter()
        .map(|intent| intent.relative_path.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(paths, std::collections::BTreeSet::from(["A.jpg", "a.jpg"]));
    assert_eq!(batch.intents.len(), 2);
    assert_eq!(next.next_unread_usn.value(), 20);
    batch.validate().unwrap();
}

#[test]
fn directory_rename_enrolls_only_the_two_affected_subtrees() {
    let old = vec![location(2, "old", true)];
    let current = IdentityJournalResolution {
        locations: vec![location(2, "new", true)],
        ..Default::default()
    };
    let (batch, _) =
        translation::translate(&checkpoint(), &page(true), 20, 2, &old, &current).unwrap();
    assert_eq!(batch.intents.len(), 1);
    assert_eq!(batch.intents[0].relative_path, "new");
    assert_eq!(
        batch.intents[0].previous_relative_path.as_deref(),
        Some("old")
    );
    assert_eq!(
        batch.intents[0].kind,
        crate::domain::LibraryChangeIntentKind::RenameCandidate
    );
    assert!(
        batch
            .intents
            .iter()
            .all(|intent| intent.scope == LibraryChangeScope::Subtree)
    );
    assert_eq!(batch.directory_changes[0].previous_relative_paths, ["old"]);
    assert_eq!(
        batch.directory_changes[0].current_relative_path.as_deref(),
        Some("new")
    );
    let mut unowned_previous = batch.clone();
    unowned_previous.intents[0].previous_relative_path = None;
    assert!(unowned_previous.validate().is_err());
    let mut wrong_kind = batch;
    wrong_kind.intents[0].kind = crate::domain::LibraryChangeIntentKind::Reconcile;
    assert!(wrong_kind.validate().is_err());
}

#[test]
fn failed_identity_open_never_removes_directory_history() {
    let old = vec![location(2, "retained", true)];
    let current = IdentityJournalResolution {
        unresolved: vec![old[0].identity.clone()],
        ..Default::default()
    };
    let (batch, _) =
        translation::translate(&checkpoint(), &page(true), 20, 2, &old, &current).unwrap();
    assert!(batch.directory_changes.is_empty());
    assert_eq!(batch.intents[0].relative_path, "retained");
    let mut deleted = page(true);
    deleted.records[0].is_deleted = true;
    let (batch, _) =
        translation::translate(&checkpoint(), &deleted, 20, 2, &old, &current).unwrap();
    assert_eq!(batch.directory_changes.len(), 1);
    assert!(batch.directory_changes[0].current_relative_path.is_none());
}

#[test]
fn unsupported_alias_enumeration_requires_reconstruction_without_current_authority() {
    let previous = checkpoint();
    let opening = LibraryRecoveryOpeningBoundary {
        volume: previous.volume,
        root_file_reference: previous.root_file_reference,
        journal_id: previous.journal_id,
        next_usn: JournalUsn::new(20).unwrap(),
        protocol_version: previous.protocol_version,
        contract_version: previous.contract_version,
    };
    let failure = from_error(
        ScanError::new(
            "identity_journal_hardlinks_not_supported",
            "Windows metadata operation failed: 50",
        ),
        Some(&opening),
    );
    assert_eq!(
        failure.kind,
        PersistentJournalRootFailureKind::JournalReconstructionFailure
    );
    assert_eq!(
        failure.kind.recovery_reason(),
        Some(crate::domain::LibraryRecoveryAuthorityReason::JournalReconstructionFailure)
    );
    assert_eq!(failure.opening_boundary.as_deref(), Some(&opening));
    assert_eq!(
        failure.failure.code,
        "identity_journal_hardlinks_not_supported"
    );
}

#[test]
fn inaccessible_unknown_identity_recovers_only_its_proven_parent_scope() {
    let old = vec![location(3, "album", true)];
    let current = IdentityJournalResolution {
        access_denied: vec![identity_for_reference(7, &reference(2))],
        ..Default::default()
    };
    let (batch, next) =
        translation::translate(&checkpoint(), &page(false), 20, 2, &old, &current).unwrap();
    assert_eq!(batch.intents.len(), 1);
    assert_eq!(batch.intents[0].relative_path, "album");
    assert_eq!(batch.intents[0].scope, LibraryChangeScope::Subtree);
    assert!(batch.directory_changes.is_empty());
    assert_eq!(next.next_unread_usn.value(), 20);

    let (unrelated, _) =
        translation::translate(&checkpoint(), &page(false), 20, 2, &[], &current).unwrap();
    assert!(unrelated.intents.is_empty());
}

struct Source {
    reads: AtomicUsize,
    resolutions: AtomicUsize,
    observed_first: i64,
    observed_end: i64,
    request: std::sync::Mutex<Option<i64>>,
    records: Vec<IdentityJournalRecord>,
    locations: Vec<IdentityJournalLocation>,
    resolution_failure: Option<ScanError>,
}
impl PersistentIdentityJournalSource for Source {
    fn can_probe_baseline(&self, _root: &IdentityJournalRoot) -> bool {
        true
    }
    fn volume_identity(
        &self,
        _root: &IdentityJournalRoot,
    ) -> Result<PersistentJournalVolumeIdentity, ScanError> {
        Ok(checkpoint().volume)
    }
    fn read_volume(
        &self,
        request: &IdentityJournalRead,
        _cancelled: &AtomicBool,
    ) -> Result<IdentityJournalPage, ScanError> {
        self.reads.fetch_add(1, Ordering::Relaxed);
        *self.request.lock().unwrap() = Some(request.end_usn);
        Ok(IdentityJournalPage {
            covered_until_usn: request.end_usn,
            records: self.records.clone(),
        })
    }
    fn current_locations(
        &self,
        _root: &IdentityJournalRoot,
        references: &[JournalFileReference],
        _cancelled: &AtomicBool,
    ) -> Result<IdentityJournalResolution, ScanError> {
        self.resolutions.fetch_add(1, Ordering::Relaxed);
        assert!(references.len() <= MAX_IDENTITY_JOURNAL_REFERENCES);
        if let Some(failure) = &self.resolution_failure {
            return Err(failure.clone());
        }
        Ok(IdentityJournalResolution {
            locations: self.locations.clone(),
            ..Default::default()
        })
    }
}

struct Namespace;
impl PersistentJournalNamespaceReader for Namespace {
    fn has_snapshot(&self, _root: &IdentityJournalRoot) -> Result<bool, ScanError> {
        Ok(true)
    }
    fn previous_locations(
        &self,
        _root: &IdentityJournalRoot,
        _identities: &[crate::domain::FileIdentityEvidence],
    ) -> Result<Vec<IdentityJournalLocation>, ScanError> {
        Ok(Vec::new())
    }
}

struct Session(Arc<Source>);
impl PersistentChangeJournalSession for Session {
    fn identity_source(&self) -> Option<Arc<dyn PersistentIdentityJournalSource>> {
        Some(self.0.clone())
    }
    fn register_root(
        &self,
        _request: RegisterRootRequest,
    ) -> Result<RootCapability, PersistentChangeJournalOperationError> {
        Ok(RootCapability([1; 32]))
    }
    fn query_journal(
        &self,
        request: QueryJournalRequest,
    ) -> Result<BrokerResponse, PersistentChangeJournalOperationError> {
        Ok(BrokerResponse::Journal {
            request_id: 1,
            binding: ResponseBinding::from_request(&request.caller, &request.root),
            capability: JournalCapability::Supported,
            journal_id: Some(9),
            first_usn: Some(self.0.observed_first),
            next_usn: Some(self.0.observed_end),
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
        _request: ReadJournalVolumeRequest,
    ) -> Result<Box<dyn PersistentChangeJournalRead>, PersistentChangeJournalOperationError> {
        Err(PersistentChangeJournalOperationError::InvalidRequest)
    }
    fn close(&self) -> Result<(), PersistentChangeJournalOperationError> {
        Ok(())
    }
}

#[test]
fn no_change_restart_uses_the_production_reader_without_resolving_or_enumerating_sources() {
    let source = Arc::new(Source {
        reads: AtomicUsize::new(0),
        resolutions: AtomicUsize::new(0),
        observed_first: 0,
        observed_end: 10,
        request: std::sync::Mutex::new(None),
        resolution_failure: None,
        records: Vec::new(),
        locations: Vec::new(),
    });
    let reader = production_reader(source.clone(), Vec::new());
    let outcomes = reader
        .read_volume(&[checkpoint()], &[], 2, &AtomicBool::new(false))
        .unwrap();
    assert!(matches!(outcomes[0].page, Ok(None)));
    assert_eq!(source.reads.load(Ordering::Relaxed), 0);
    assert_eq!(source.resolutions.load(Ordering::Relaxed), 0);
}

#[test]
fn journal_trim_retains_observed_bounds_and_equality_still_allows_replay() {
    for first in [10, 11] {
        let source = Arc::new(Source {
            reads: AtomicUsize::new(0),
            resolutions: AtomicUsize::new(0),
            observed_first: first,
            observed_end: 20,
            request: std::sync::Mutex::new(None),
            resolution_failure: None,
            records: Vec::new(),
            locations: Vec::new(),
        });
        let reader = production_reader(source.clone(), Vec::new());
        let outcomes = reader
            .read_volume(&[checkpoint()], &[], 2, &AtomicBool::new(false))
            .expect("read observed journal boundary");
        if first == 10 {
            assert!(outcomes[0].page.is_ok());
            assert_eq!(source.reads.load(Ordering::Relaxed), 1);
        } else {
            let trim = outcomes[0].page.as_ref().expect_err("expired checkpoint");
            assert_eq!(trim.kind, PersistentJournalRootFailureKind::JournalTrim);
            assert_eq!(trim.failure.code, "identity_journal_trim");
            assert!(trim.failure.message.contains(
                "checkpoint_usn=10, first_usn=11, next_usn=20, checkpoint_updated_unix_ms=1"
            ));
            assert_eq!(source.reads.load(Ordering::Relaxed), 0);
            assert_eq!(source.resolutions.load(Ordering::Relaxed), 0);
        }
    }
}

fn production_reader(
    source: Arc<Source>,
    baselines: Vec<crate::domain::PersistentJournalBaseline>,
) -> super::super::production_reader::ProductionPersistentJournalVolumeReader {
    let caller = CallerClaim {
        process_id: 1,
        session_id: 1,
        client_instance: [1; 16],
    };
    let root = PersistentJournalBrokerRoot {
        authorization: RootAuthorization {
            root_id: "root".to_owned(),
            root_generation: 1,
            volume_id: "volume".to_owned(),
            root_identity: vec![3; 16],
            canonical_root_utf16: "C:\\source".encode_utf16().collect(),
        },
        client_root_handle: 1,
        volume_serial: 7,
    };
    super::super::production_reader::ProductionPersistentJournalVolumeReader::new(
        Arc::new(Session(source.clone())),
        caller,
        vec![root],
        Arc::new(Namespace),
        baselines,
    )
    .unwrap()
}

#[test]
fn large_unrelated_volume_page_reaches_the_root_change_without_an_inventory() {
    let mut records: Vec<_> = (10..522)
        .map(|usn| IdentityJournalRecord {
            file_reference: reference(100),
            parent_reference: reference(101),
            usn,
            is_directory: false,
            is_deleted: false,
        })
        .collect();
    records.push(IdentityJournalRecord {
        file_reference: reference(2),
        parent_reference: reference(3),
        usn: 522,
        is_directory: false,
        is_deleted: false,
    });
    let source = Arc::new(Source {
        reads: AtomicUsize::new(0),
        resolutions: AtomicUsize::new(0),
        observed_first: 0,
        observed_end: 600,
        request: std::sync::Mutex::new(None),
        resolution_failure: None,
        records,
        locations: vec![location(2, "fresh.png", false)],
    });
    let reader = production_reader(source.clone(), Vec::new());
    let outcomes = reader
        .read_volume(&[checkpoint()], &[], 2, &AtomicBool::new(false))
        .unwrap();
    let (batch, next) = outcomes[0]
        .page
        .as_ref()
        .unwrap()
        .as_ref()
        .unwrap()
        .as_enrolled()
        .unwrap();
    assert_eq!(batch.intents.len(), 1);
    assert_eq!(batch.intents[0].relative_path, "fresh.png");
    assert_eq!(batch.intents[0].scope, LibraryChangeScope::Path);
    assert!(batch.directory_changes.is_empty());
    assert_eq!(next.next_unread_usn.value(), 600);
    assert_eq!(source.reads.load(Ordering::Relaxed), 1);
    assert_eq!(source.resolutions.load(Ordering::Relaxed), 1);
}

#[test]
fn over_capacity_identity_page_is_rejected_before_source_resolution() {
    let records = (0..129)
        .map(|index| IdentityJournalRecord {
            file_reference: reference(index * 2),
            parent_reference: reference(index * 2 + 1),
            usn: 10 + index as i64,
            is_directory: false,
            is_deleted: false,
        })
        .collect();
    let source = Arc::new(Source {
        reads: AtomicUsize::new(0),
        resolutions: AtomicUsize::new(0),
        observed_first: 0,
        observed_end: 200,
        request: std::sync::Mutex::new(None),
        resolution_failure: None,
        records,
        locations: Vec::new(),
    });
    let reader = production_reader(source.clone(), Vec::new());
    let failure = reader
        .read_volume(&[checkpoint()], &[], 2, &AtomicBool::new(false))
        .unwrap_err();
    assert_eq!(failure.failure.code, "identity_journal_page_invalid");
    assert_eq!(source.resolutions.load(Ordering::Relaxed), 0);
}

#[test]
fn first_import_replay_keeps_its_closing_boundary_when_the_volume_grows() {
    let source = Arc::new(Source {
        reads: AtomicUsize::new(0),
        resolutions: AtomicUsize::new(0),
        observed_first: 0,
        observed_end: 30,
        request: std::sync::Mutex::new(None),
        resolution_failure: None,
        records: Vec::new(),
        locations: Vec::new(),
    });
    let mut checkpoint = checkpoint();
    checkpoint.continuity = PersistentJournalContinuityState::CatchingUp;
    checkpoint.captured_exclusive_end = JournalUsn::new(20).unwrap();
    let baseline = crate::domain::PersistentJournalBaseline {
        change_id: crate::domain::LibraryChangeId::new(1).unwrap(),
        run_id: "first-import".to_owned(),
        root_id: checkpoint.root_id.clone(),
        root_generation: checkpoint.root_generation,
        volume: checkpoint.volume.clone(),
        root_file_reference: checkpoint.root_file_reference.clone(),
        journal_id: checkpoint.journal_id,
        opening_next_usn: checkpoint.next_unread_usn,
        closing_next_usn: Some(checkpoint.captured_exclusive_end),
        protocol_version: checkpoint.protocol_version,
        contract_version: checkpoint.contract_version,
        phase: crate::domain::PersistentJournalBaselinePhase::Replay,
        authorized_unix_ms: 1,
        updated_unix_ms: 1,
        completed_unix_ms: None,
    };
    let reader = production_reader(source.clone(), vec![baseline]);
    let outcomes = reader
        .read_volume(&[checkpoint], &[], 2, &AtomicBool::new(false))
        .unwrap();
    let (batch, checkpoint) = outcomes[0]
        .page
        .as_ref()
        .unwrap()
        .as_ref()
        .unwrap()
        .as_enrolled()
        .unwrap();
    assert_eq!(batch.range.requested_end_usn.value(), 20);
    assert_eq!(checkpoint.captured_exclusive_end.value(), 20);
    assert_eq!(checkpoint.next_unread_usn.value(), 20);
    assert_eq!(*source.request.lock().unwrap(), Some(20));
    let outcomes = reader
        .read_volume(
            std::slice::from_ref(checkpoint),
            &[],
            3,
            &AtomicBool::new(false),
        )
        .unwrap();
    assert!(matches!(outcomes[0].page, Ok(None)));
    assert_eq!(source.reads.load(Ordering::Relaxed), 1);
}

#[test]
fn empty_baseline_replay_reads_its_boundary_without_resolving_source_names() {
    let source = Arc::new(Source {
        reads: AtomicUsize::new(0),
        resolutions: AtomicUsize::new(0),
        observed_first: 0,
        observed_end: 10,
        request: std::sync::Mutex::new(None),
        resolution_failure: None,
        records: Vec::new(),
        locations: Vec::new(),
    });
    let mut checkpoint = checkpoint();
    checkpoint.continuity = PersistentJournalContinuityState::CatchingUp;
    let mut baseline = crate::domain::PersistentJournalBaseline {
        change_id: crate::domain::LibraryChangeId::new(1).unwrap(),
        run_id: "empty-first-import".to_owned(),
        root_id: checkpoint.root_id.clone(),
        root_generation: checkpoint.root_generation,
        volume: checkpoint.volume.clone(),
        root_file_reference: checkpoint.root_file_reference.clone(),
        journal_id: checkpoint.journal_id,
        opening_next_usn: checkpoint.next_unread_usn,
        closing_next_usn: Some(checkpoint.next_unread_usn),
        protocol_version: checkpoint.protocol_version,
        contract_version: checkpoint.contract_version,
        phase: crate::domain::PersistentJournalBaselinePhase::Replay,
        authorized_unix_ms: 1,
        updated_unix_ms: 1,
        completed_unix_ms: None,
    };
    let reader = production_reader(source.clone(), vec![baseline.clone()]);
    let outcomes = reader
        .read_volume(
            std::slice::from_ref(&checkpoint),
            &[],
            2,
            &AtomicBool::new(false),
        )
        .unwrap();
    assert!(matches!(
        &outcomes[0].page,
        Ok(Some(PersistentJournalReadPage::EmptyReplay(receipt))) if receipt == &checkpoint
    ));
    assert_eq!(source.reads.load(Ordering::Relaxed), 1);
    assert_eq!(source.resolutions.load(Ordering::Relaxed), 0);
    assert_eq!(*source.request.lock().unwrap(), Some(10));

    baseline.phase = crate::domain::PersistentJournalBaselinePhase::Absence;
    let reader = production_reader(source.clone(), vec![baseline]);
    let outcomes = reader
        .read_volume(&[checkpoint], &[], 3, &AtomicBool::new(false))
        .unwrap();
    assert!(matches!(outcomes[0].page, Ok(None)));
    assert_eq!(source.reads.load(Ordering::Relaxed), 1);
}

#[test]
fn partial_catchup_finishes_its_captured_window_before_admitting_later_writes() {
    let source = Arc::new(Source {
        reads: AtomicUsize::new(0),
        resolutions: AtomicUsize::new(0),
        observed_first: 0,
        observed_end: 30,
        request: std::sync::Mutex::new(None),
        resolution_failure: None,
        records: Vec::new(),
        locations: Vec::new(),
    });
    let mut checkpoint = checkpoint();
    checkpoint.continuity = PersistentJournalContinuityState::CatchingUp;
    checkpoint.captured_exclusive_end = JournalUsn::new(20).unwrap();
    let reader = production_reader(source.clone(), Vec::new());
    let outcomes = reader
        .read_volume(&[checkpoint], &[], 2, &AtomicBool::new(false))
        .unwrap();
    let (_, checkpoint) = outcomes[0]
        .page
        .as_ref()
        .unwrap()
        .as_ref()
        .unwrap()
        .as_enrolled()
        .unwrap();
    assert_eq!(checkpoint.next_unread_usn.value(), 20);
    assert_eq!(checkpoint.captured_exclusive_end.value(), 20);
    assert_eq!(*source.request.lock().unwrap(), Some(20));

    let outcomes = reader
        .read_volume(
            std::slice::from_ref(checkpoint),
            &[],
            3,
            &AtomicBool::new(false),
        )
        .unwrap();
    let (_, checkpoint) = outcomes[0]
        .page
        .as_ref()
        .unwrap()
        .as_ref()
        .unwrap()
        .as_enrolled()
        .unwrap();
    assert_eq!(checkpoint.next_unread_usn.value(), 30);
    assert_eq!(checkpoint.captured_exclusive_end.value(), 30);
    assert_eq!(source.reads.load(Ordering::Relaxed), 2);
}

#[test]
fn failed_alias_page_enters_recovery_without_publishing_partial_locations() {
    let source = Arc::new(Source {
        reads: AtomicUsize::new(0),
        resolutions: AtomicUsize::new(0),
        observed_first: 0,
        observed_end: 20,
        request: std::sync::Mutex::new(None),
        records: page(false).records,
        locations: vec![location(2, "first.png", false)],
        resolution_failure: Some(ScanError::new(
            "identity_journal_hardlinks_not_supported",
            "Windows metadata operation failed: 50; operation=FindNextFileNameW; identity=7:2",
        )),
    });
    let reader = production_reader(source.clone(), Vec::new());
    let previous = checkpoint();
    let outcomes = reader
        .read_volume(
            std::slice::from_ref(&previous),
            &[],
            2,
            &AtomicBool::new(false),
        )
        .unwrap();
    assert_eq!(outcomes.len(), 1);
    assert_eq!(outcomes[0].root_id, previous.root_id);
    let failure = outcomes[0]
        .page
        .as_ref()
        .expect_err("no partial page may advance the checkpoint");
    assert_eq!(
        failure.kind,
        PersistentJournalRootFailureKind::JournalReconstructionFailure
    );
    let opening = failure
        .opening_boundary
        .as_deref()
        .expect("fresh recovery opening");
    assert_eq!(opening.next_usn.value(), 20);
    assert_eq!(opening.root_file_reference, previous.root_file_reference);
    assert_eq!(opening.volume, previous.volume);
    assert_eq!(opening.journal_id, previous.journal_id);
    assert!(
        failure
            .failure
            .message
            .contains("FindNextFileNameW; identity=7:2")
    );
    assert_eq!(source.reads.load(Ordering::Relaxed), 1);
    assert_eq!(source.resolutions.load(Ordering::Relaxed), 1);
}

#[test]
fn retained_alias_failure_captures_a_new_opening_without_replaying_the_failed_page() {
    let source = Arc::new(Source {
        reads: AtomicUsize::new(0),
        resolutions: AtomicUsize::new(0),
        observed_first: 0,
        observed_end: 30,
        request: std::sync::Mutex::new(None),
        records: page(false).records,
        locations: Vec::new(),
        resolution_failure: None,
    });
    let reader = production_reader(source.clone(), Vec::new());
    let mut previous = checkpoint();
    previous.continuity = PersistentJournalContinuityState::RecoveryRequired;
    previous.captured_exclusive_end = JournalUsn::new(20).unwrap();
    previous.failure = Some(PersistentJournalFailure {
        code: "identity_journal_hardlinks_not_supported".to_owned(),
        message: "Retained alias lookup failure".to_owned(),
    });
    let outcomes = reader
        .read_volume(&[previous], &[], 2, &AtomicBool::new(false))
        .unwrap();
    let failure = outcomes[0]
        .page
        .as_ref()
        .expect_err("retained failure enters recovery");
    assert_eq!(
        failure.kind,
        PersistentJournalRootFailureKind::JournalReconstructionFailure
    );
    assert_eq!(
        failure.opening_boundary.as_ref().unwrap().next_usn.value(),
        30
    );
    assert_eq!(source.reads.load(Ordering::Relaxed), 0);
    assert_eq!(source.resolutions.load(Ordering::Relaxed), 0);
}
