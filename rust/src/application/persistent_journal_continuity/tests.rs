use std::collections::{HashMap, HashSet};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use crate::domain::{
    JournalFileReference, JournalIdentifier, JournalUsn, LibraryChangeIntent,
    LibraryChangeIntentKind, LibraryChangeOrigin, LibraryChangeQueuePolicy, LibraryChangeScope,
    LibraryRootGeneration, PersistentJournalCapability, PersistentJournalCheckpoint,
    PersistentJournalContinuityState, PersistentJournalCrossRootLineage,
    PersistentJournalEnrollmentBatch, PersistentJournalEnrollmentReport, PersistentJournalFailure,
    PersistentJournalLineageState, PersistentJournalPendingRename, PersistentJournalRangeState,
    PersistentJournalReadFailure, PersistentJournalRootFailure, PersistentJournalRootFailureKind,
    PersistentJournalRootReadOutcome, PersistentJournalSourceRange,
    PersistentJournalVolumeIdentity, ScanError, persistent_journal_batch_id,
    persistent_journal_pending_rename_id,
};
use crate::ports::{PersistentJournalRepository, PersistentJournalVolumeReader};

use super::catch_up_persistent_journal_volume;

#[test]
fn one_physical_volume_read_distributes_final_state_candidates_to_independent_roots() {
    let reader = ControlledReader::new(vec![
        page(
            "range-a",
            "root-a",
            vec![
                intent(
                    "root-a",
                    LibraryChangeIntentKind::Reconcile,
                    "created.jpg",
                    None,
                    1,
                ),
                intent(
                    "root-a",
                    LibraryChangeIntentKind::Reconcile,
                    "modified.jpg",
                    None,
                    2,
                ),
                intent(
                    "root-a",
                    LibraryChangeIntentKind::Reconcile,
                    "deleted.jpg",
                    None,
                    3,
                ),
                intent(
                    "root-a",
                    LibraryChangeIntentKind::RenameCandidate,
                    "renamed.jpg",
                    Some("old-name.jpg"),
                    4,
                ),
                intent(
                    "root-a",
                    LibraryChangeIntentKind::RenameCandidate,
                    "replacement.jpg",
                    Some("replaced.jpg"),
                    5,
                ),
            ],
            Some(lineage("range-a")),
        ),
        page(
            "range-b",
            "root-b",
            vec![intent(
                "root-b",
                LibraryChangeIntentKind::RenameCandidate,
                "moved.jpg",
                Some("source.jpg"),
                6,
            )],
            Some(lineage("range-b")),
        ),
    ]);
    let mut repository = ControlledRepository {
        capabilities: capabilities(&["root-a", "root-b"]),
        ..ControlledRepository::default()
    };
    let checkpoints = vec![checkpoint("root-a", 10), checkpoint("root-b", 10)];

    let report = catch_up_persistent_journal_volume(
        &mut repository,
        &reader,
        &checkpoints,
        1_000,
        policy(),
        &AtomicBool::new(false),
    )
    .expect("controlled volume catch-up");

    assert_eq!(reader.physical_reads.load(Ordering::SeqCst), 1);
    assert_eq!(reader.root_enumerations.load(Ordering::SeqCst), 0);
    assert_eq!(reader.media_reads.load(Ordering::SeqCst), 0);
    assert_eq!(reader.source_mutations.load(Ordering::SeqCst), 0);
    assert_eq!(report.enrolled_root_count, 2);
    assert_eq!(report.failed_root_count, 0);
    assert_eq!(report.observation_count, 6);
    assert_eq!(report.advanced_checkpoint_count, 2);
    assert_eq!(repository.enrollments, vec!["root-a", "root-b"]);
    assert_eq!(repository.checkpoints.len(), 2);
    assert!(repository.intents.iter().all(|intent| matches!(
        intent.kind,
        LibraryChangeIntentKind::Reconcile | LibraryChangeIntentKind::RenameCandidate
    )));
}

#[test]
fn root_publication_failure_does_not_roll_back_independent_sibling_checkpoint() {
    let reader = ControlledReader::new(vec![
        page(
            "range-a",
            "root-a",
            vec![intent(
                "root-a",
                LibraryChangeIntentKind::Reconcile,
                "a.jpg",
                None,
                1,
            )],
            None,
        ),
        page(
            "range-b",
            "root-b",
            vec![intent(
                "root-b",
                LibraryChangeIntentKind::Reconcile,
                "b.jpg",
                None,
                2,
            )],
            None,
        ),
    ]);
    let mut repository = ControlledRepository {
        capabilities: capabilities(&["root-a", "root-b"]),
        fail_enrollment_for: Some("root-b"),
        ..ControlledRepository::default()
    };

    let report = catch_up_persistent_journal_volume(
        &mut repository,
        &reader,
        &[checkpoint("root-a", 10), checkpoint("root-b", 10)],
        1_000,
        policy(),
        &AtomicBool::new(false),
    )
    .expect("isolated root publication failure");

    assert_eq!(reader.physical_reads.load(Ordering::SeqCst), 1);
    assert_eq!(report.enrolled_root_count, 1);
    assert_eq!(report.failed_root_count, 1);
    assert_eq!(report.advanced_checkpoint_count, 1);
    assert!(repository.checkpoints.contains_key("root-a"));
    assert!(!repository.checkpoints.contains_key("root-b"));
}

#[test]
fn broker_root_failure_does_not_block_unrelated_sibling_publication() {
    let healthy = page(
        "range-a",
        "root-a",
        vec![intent(
            "root-a",
            LibraryChangeIntentKind::Reconcile,
            "a.jpg",
            None,
            1,
        )],
        None,
    );
    let reader = ControlledReader::from_outcomes(vec![
        PersistentJournalRootReadOutcome {
            root_id: "root-a".to_owned(),
            root_generation: LibraryRootGeneration::initial(),
            opening_boundary: None,
            page: Ok(Some(healthy)),
        },
        PersistentJournalRootReadOutcome {
            root_id: "root-b".to_owned(),
            root_generation: LibraryRootGeneration::initial(),
            opening_boundary: None,
            page: Err(PersistentJournalReadFailure {
                kind: PersistentJournalRootFailureKind::BrokerAfterCurrentFailure,
                failure: PersistentJournalFailure {
                    code: "controlled_broker_failure".to_owned(),
                    message: "The broker rejected only this root".to_owned(),
                },
                opening_boundary: None,
            }),
        },
    ]);
    let mut repository = ControlledRepository {
        capabilities: capabilities(&["root-a", "root-b"]),
        ..ControlledRepository::default()
    };

    let report = catch_up_persistent_journal_volume(
        &mut repository,
        &reader,
        &[checkpoint("root-a", 10), checkpoint("root-b", 10)],
        1_000,
        policy(),
        &AtomicBool::new(false),
    )
    .expect("isolated broker failure");

    assert_eq!(reader.physical_reads.load(Ordering::SeqCst), 1);
    assert_eq!(report.enrolled_root_count, 1);
    assert_eq!(report.failed_root_count, 1);
    assert_eq!(report.advanced_checkpoint_count, 1);
    assert!(repository.checkpoints.contains_key("root-a"));
    assert!(!repository.checkpoints.contains_key("root-b"));
}

#[test]
fn extra_and_missing_root_outcomes_are_counted_independently() {
    let valid = page(
        "range-a",
        "root-a",
        vec![intent(
            "root-a",
            LibraryChangeIntentKind::Reconcile,
            "a.jpg",
            None,
            1,
        )],
        None,
    );
    let extra = page("range-extra", "root-extra", Vec::new(), None);
    let reader = ControlledReader::from_outcomes(vec![
        PersistentJournalRootReadOutcome {
            root_id: "root-a".to_owned(),
            root_generation: LibraryRootGeneration::initial(),
            opening_boundary: None,
            page: Ok(Some(valid)),
        },
        PersistentJournalRootReadOutcome {
            root_id: "root-extra".to_owned(),
            root_generation: LibraryRootGeneration::initial(),
            opening_boundary: None,
            page: Ok(Some(extra)),
        },
    ]);
    let mut repository = ControlledRepository {
        capabilities: capabilities(&["root-a", "root-b"]),
        ..ControlledRepository::default()
    };

    let report = catch_up_persistent_journal_volume(
        &mut repository,
        &reader,
        &[checkpoint("root-a", 10), checkpoint("root-b", 10)],
        1_000,
        policy(),
        &AtomicBool::new(false),
    )
    .expect("extra and missing outcomes");

    assert_eq!(report.enrolled_root_count, 1);
    assert_eq!(report.failed_root_count, 2);
    assert_eq!(report.advanced_checkpoint_count, 1);
}

#[test]
fn carried_handoff_must_exactly_match_the_durable_pending_rename() {
    let mut carry = PersistentJournalPendingRename {
        carry_id: String::new(),
        source_range_id: "a".repeat(64),
        volume: volume(),
        journal_id: JournalIdentifier::new(9).expect("journal"),
        file_reference: JournalFileReference::V3([2; 16]),
        old_usn: JournalUsn::new(12).expect("OLD USN"),
        previous_root_id: "root-a".to_owned(),
        previous_root_generation: LibraryRootGeneration::initial(),
        previous_relative_path: "source.jpg".to_owned(),
        is_directory: false,
        enrolled_unix_ms: 900,
    };
    carry.carry_id = persistent_journal_pending_rename_id(&carry);

    let mut current_page = page("range-new", "root-b", Vec::new(), None);
    let current_range_id = current_page.0.range.batch_id.clone();
    let mut current_owner = lineage(&current_range_id);
    current_owner.old_usn = Some(carry.old_usn);
    current_owner.new_usn = Some(JournalUsn::new(18).expect("NEW USN"));
    current_owner.previous_carry_id = Some(carry.carry_id.clone());
    current_owner.previous_relative_path = "tampered-source.jpg".to_owned();
    let mut previous_owner = current_owner.clone();
    previous_owner.owner_source_range_id = carry.source_range_id.clone();
    current_page.0.cross_root_lineage.push(current_owner);
    current_page
        .0
        .carried_cross_root_lineage
        .push(previous_owner);
    current_page
        .0
        .consumed_pending_rename_ids
        .push(carry.carry_id.clone());
    let canonical_id = persistent_journal_batch_id(&current_page.0);
    current_page.0.range.batch_id.clone_from(&canonical_id);
    current_page.0.cross_root_lineage[0]
        .owner_source_range_id
        .clone_from(&canonical_id);

    let reader = ControlledReader::new(vec![current_page]);
    let mut repository = ControlledRepository {
        capabilities: capabilities(&["root-b"]),
        pending_renames: vec![carry],
        ..ControlledRepository::default()
    };
    let report = catch_up_persistent_journal_volume(
        &mut repository,
        &reader,
        &[checkpoint("root-b", 10)],
        1_000,
        policy(),
        &AtomicBool::new(false),
    )
    .expect("malformed carried handoff remains a root outcome");

    assert_eq!(report.enrolled_root_count, 0);
    assert_eq!(report.failed_root_count, 1);
    assert_eq!(report.advanced_checkpoint_count, 0);
    assert!(repository.enrollments.is_empty());
}

struct ControlledReader {
    pages: Mutex<Vec<PersistentJournalRootReadOutcome>>,
    physical_reads: AtomicUsize,
    root_enumerations: AtomicUsize,
    media_reads: AtomicUsize,
    source_mutations: AtomicUsize,
}

impl ControlledReader {
    fn new(
        pages: Vec<(
            PersistentJournalEnrollmentBatch,
            PersistentJournalCheckpoint,
        )>,
    ) -> Self {
        Self::from_outcomes(
            pages
                .into_iter()
                .map(|page| PersistentJournalRootReadOutcome {
                    root_id: page.0.range.root_id.clone(),
                    root_generation: page.0.range.root_generation,
                    opening_boundary: None,
                    page: Ok(Some(page)),
                })
                .collect(),
        )
    }

    fn from_outcomes(pages: Vec<PersistentJournalRootReadOutcome>) -> Self {
        Self {
            pages: Mutex::new(pages),
            physical_reads: AtomicUsize::new(0),
            root_enumerations: AtomicUsize::new(0),
            media_reads: AtomicUsize::new(0),
            source_mutations: AtomicUsize::new(0),
        }
    }
}

impl PersistentJournalVolumeReader for ControlledReader {
    fn read_volume(
        &self,
        _checkpoints: &[PersistentJournalCheckpoint],
        _pending_renames: &[crate::domain::PersistentJournalPendingRename],
        _observed_unix_ms: i64,
        _cancelled: &AtomicBool,
    ) -> Result<Vec<PersistentJournalRootReadOutcome>, PersistentJournalReadFailure> {
        self.physical_reads.fetch_add(1, Ordering::SeqCst);
        Ok(self.pages.lock().expect("controlled pages").clone())
    }
}

#[derive(Default)]
struct ControlledRepository {
    capabilities: Vec<PersistentJournalCapability>,
    pending_renames: Vec<PersistentJournalPendingRename>,
    enrollments: Vec<String>,
    enrollment_ids: HashSet<String>,
    intents: Vec<LibraryChangeIntent>,
    checkpoints: HashMap<String, PersistentJournalCheckpoint>,
    fail_enrollment_for: Option<&'static str>,
    root_failures: Vec<PersistentJournalRootFailure>,
}

impl PersistentJournalRepository for ControlledRepository {
    fn persist_persistent_journal_root_failure(
        &mut self,
        failure: &PersistentJournalRootFailure,
        _failed_unix_ms: i64,
        _policy: LibraryChangeQueuePolicy,
    ) -> Result<Option<crate::domain::LibraryChangeId>, ScanError> {
        self.root_failures.push(failure.clone());
        Ok(None)
    }

    fn load_persistent_journal_capabilities(
        &self,
    ) -> Result<Vec<PersistentJournalCapability>, ScanError> {
        Ok(self.capabilities.clone())
    }

    fn load_persistent_journal_checkpoint(
        &self,
        root_id: &str,
        _root_generation: LibraryRootGeneration,
    ) -> Result<Option<PersistentJournalCheckpoint>, ScanError> {
        Ok(self.checkpoints.get(root_id).cloned())
    }

    fn save_persistent_journal_capability(
        &mut self,
        capability: &PersistentJournalCapability,
    ) -> Result<(), ScanError> {
        self.capabilities.push(capability.clone());
        Ok(())
    }

    fn load_persistent_journal_pending_renames(
        &self,
        _volume: &PersistentJournalVolumeIdentity,
        _journal_id: JournalIdentifier,
    ) -> Result<Vec<crate::domain::PersistentJournalPendingRename>, ScanError> {
        Ok(self.pending_renames.clone())
    }

    fn publish_persistent_journal_volume_batch(
        &mut self,
        batch: &crate::domain::PersistentJournalVolumeBatch,
        _enqueued_unix_ms: i64,
        _policy: LibraryChangeQueuePolicy,
    ) -> Result<PersistentJournalEnrollmentReport, ScanError> {
        if batch
            .pages
            .iter()
            .any(|page| self.fail_enrollment_for == Some(page.enrollment.range.root_id.as_str()))
        {
            return Err(ScanError::new(
                "controlled_root_failure",
                "The controlled volume rejected its atomic enrollment",
            ));
        }
        let mut report = PersistentJournalEnrollmentReport::default();
        for page in &batch.pages {
            let enrollment =
                self.enroll_persistent_journal_batch(&page.enrollment, _enqueued_unix_ms, _policy)?;
            report.enrolled_root_count = report
                .enrolled_root_count
                .saturating_add(enrollment.enrolled_root_count);
            report.observation_count = report
                .observation_count
                .saturating_add(enrollment.observation_count);
            if self.advance_persistent_journal_checkpoint(
                &page.enrollment.range.batch_id,
                &page.checkpoint,
            )? {
                report.advanced_checkpoint_count =
                    report.advanced_checkpoint_count.saturating_add(1);
            }
        }
        Ok(report)
    }

    fn enroll_persistent_journal_batch(
        &mut self,
        batch: &PersistentJournalEnrollmentBatch,
        _enqueued_unix_ms: i64,
        _policy: LibraryChangeQueuePolicy,
    ) -> Result<PersistentJournalEnrollmentReport, ScanError> {
        if self.fail_enrollment_for == Some(batch.range.root_id.as_str()) {
            return Err(ScanError::new(
                "controlled_root_failure",
                "The controlled root rejected its enrollment",
            ));
        }
        let is_new = self.enrollment_ids.insert(batch.range.batch_id.clone());
        if is_new {
            self.enrollments.push(batch.range.root_id.clone());
            self.intents.extend(batch.intents.clone());
        }
        Ok(PersistentJournalEnrollmentReport {
            enrolled_root_count: 1,
            observation_count: if is_new {
                batch
                    .intents
                    .iter()
                    .map(|intent| u64::from(intent.coalesced_observation_count))
                    .sum()
            } else {
                0
            },
            ..PersistentJournalEnrollmentReport::default()
        })
    }

    fn advance_persistent_journal_checkpoint(
        &mut self,
        _source_range_id: &str,
        checkpoint: &PersistentJournalCheckpoint,
    ) -> Result<bool, ScanError> {
        Ok(self
            .checkpoints
            .insert(checkpoint.root_id.clone(), checkpoint.clone())
            .as_ref()
            != Some(checkpoint))
    }
}

fn page(
    range_id: &str,
    root_id: &str,
    intents: Vec<LibraryChangeIntent>,
    lineage: Option<PersistentJournalCrossRootLineage>,
) -> (
    PersistentJournalEnrollmentBatch,
    PersistentJournalCheckpoint,
) {
    let mut enrollment = PersistentJournalEnrollmentBatch {
        range: PersistentJournalSourceRange {
            batch_id: range_id.to_owned(),
            root_id: root_id.to_owned(),
            root_generation: LibraryRootGeneration::initial(),
            volume: volume(),
            journal_id: JournalIdentifier::new(9).expect("journal ID"),
            requested_start_usn: JournalUsn::new(10).expect("start USN"),
            requested_end_usn: JournalUsn::new(20).expect("end USN"),
            covered_until_usn: JournalUsn::new(20).expect("covered USN"),
            is_complete: true,
            protocol_version: 1,
            contract_version: 1,
            state: PersistentJournalRangeState::Enrolled,
            enrolled_unix_ms: 1_000,
            checkpointed_unix_ms: None,
        },
        intents,
        cross_root_lineage: lineage.into_iter().collect(),
        carried_cross_root_lineage: Vec::new(),
        pending_renames: Vec::new(),
        consumed_pending_rename_ids: Vec::new(),
    };
    let canonical_id = persistent_journal_batch_id(&enrollment);
    enrollment.range.batch_id.clone_from(&canonical_id);
    for item in &mut enrollment.cross_root_lineage {
        if item.owner_source_range_id == range_id {
            item.owner_source_range_id.clone_from(&canonical_id);
        }
    }
    (enrollment, checkpoint_at(root_id, 20))
}

fn checkpoint(root_id: &str, usn: i64) -> PersistentJournalCheckpoint {
    checkpoint_at(root_id, usn)
}

fn checkpoint_at(root_id: &str, usn: i64) -> PersistentJournalCheckpoint {
    PersistentJournalCheckpoint {
        root_id: root_id.to_owned(),
        root_generation: LibraryRootGeneration::initial(),
        volume: volume(),
        root_file_reference: JournalFileReference::V2([1; 8]),
        journal_id: JournalIdentifier::new(9).expect("journal ID"),
        next_unread_usn: JournalUsn::new(usn).expect("next USN"),
        captured_exclusive_end: JournalUsn::new(usn).expect("end USN"),
        covered_catalog_revision: 1,
        protocol_version: 1,
        contract_version: 1,
        continuity: PersistentJournalContinuityState::CatchingUp,
        failure: None,
        updated_unix_ms: 1_000,
    }
}

fn intent(
    root_id: &str,
    kind: LibraryChangeIntentKind,
    relative_path: &str,
    previous_relative_path: Option<&str>,
    sequence: u64,
) -> LibraryChangeIntent {
    LibraryChangeIntent {
        root_id: root_id.to_owned(),
        root_generation: LibraryRootGeneration::initial(),
        kind,
        scope: LibraryChangeScope::Path,
        relative_path: relative_path.to_owned(),
        previous_relative_path: previous_relative_path.map(str::to_owned),
        origin: LibraryChangeOrigin::StartupCatchUp,
        first_observed_unix_ms: 1_000,
        most_recent_observed_unix_ms: 1_000,
        first_sequence: sequence,
        most_recent_sequence: sequence,
        coalesced_observation_count: 1,
    }
}

fn lineage(owner_source_range_id: &str) -> PersistentJournalCrossRootLineage {
    PersistentJournalCrossRootLineage {
        lineage_id: "move-lineage".to_owned(),
        owner_source_range_id: owner_source_range_id.to_owned(),
        volume: volume(),
        journal_id: JournalIdentifier::new(9).expect("journal"),
        file_reference: JournalFileReference::V3([2; 16]),
        old_usn: None,
        new_usn: None,
        previous_carry_id: None,
        previous_root_id: "root-a".to_owned(),
        previous_root_generation: LibraryRootGeneration::initial(),
        previous_relative_path: "source.jpg".to_owned(),
        current_root_id: "root-b".to_owned(),
        current_root_generation: LibraryRootGeneration::initial(),
        current_relative_path: "moved.jpg".to_owned(),
        state: PersistentJournalLineageState::Pending,
    }
}

fn volume() -> PersistentJournalVolumeIdentity {
    PersistentJournalVolumeIdentity {
        volume_guid: "volume-a".to_owned(),
        volume_serial: u64::MAX,
    }
}

fn policy() -> LibraryChangeQueuePolicy {
    LibraryChangeQueuePolicy {
        debounce_millis: 0,
        ..LibraryChangeQueuePolicy::default()
    }
}

fn capabilities(root_ids: &[&str]) -> Vec<PersistentJournalCapability> {
    root_ids
        .iter()
        .map(|root_id| PersistentJournalCapability {
            root_id: (*root_id).to_owned(),
            root_generation: LibraryRootGeneration::initial(),
            protocol_version: 1,
            contract_version: 1,
            state: crate::domain::PersistentJournalCapabilityState::Supported,
            continuity: PersistentJournalContinuityState::Current,
            failure: None,
            updated_unix_ms: 1_000,
        })
        .collect()
}
