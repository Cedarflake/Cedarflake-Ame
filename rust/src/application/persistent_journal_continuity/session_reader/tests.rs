use std::collections::VecDeque;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use super::*;
use crate::domain::{JournalFileReference, PersistentJournalVolumeIdentity};
use crate::journal_broker::{
    BrokerFailure, CandidateKind, CandidateScope, PersistentChangeJournalLiveOnlyReason,
    PersistentChangeJournalRead, PersistentChangeJournalSession, ResponseBinding, RootCapability,
};

#[derive(Default)]
struct SessionState {
    register_count: AtomicUsize,
    query_count: AtomicUsize,
    shared_read_count: AtomicUsize,
    close_count: AtomicUsize,
    captured: Mutex<Option<ReadJournalVolumeRequest>>,
    query_journal_ids: Mutex<VecDeque<u64>>,
    query_first_usns: Mutex<VecDeque<i64>>,
    query_next_usns: Mutex<VecDeque<i64>>,
    fail_second_root: AtomicBool,
    emit_pending_old: AtomicBool,
    consume_first_pending: AtomicBool,
    cancel_after_first_query: Mutex<Option<Arc<AtomicBool>>>,
}

struct FakeJournal {
    state: Arc<SessionState>,
}

impl PersistentChangeJournal for FakeJournal {
    fn connect(&self) -> PersistentChangeJournalConnection {
        PersistentChangeJournalConnection::Connected(Arc::new(FakeSession {
            state: Arc::clone(&self.state),
        }))
    }
}

struct FakeSession {
    state: Arc<SessionState>,
}

impl PersistentChangeJournalSession for FakeSession {
    fn register_root(
        &self,
        request: RegisterRootRequest,
    ) -> Result<RootCapability, PersistentChangeJournalOperationError> {
        self.state.register_count.fetch_add(1, Ordering::AcqRel);
        let mut capability = [0_u8; 32];
        capability[..8].copy_from_slice(&request.root.root_generation.to_le_bytes());
        Ok(RootCapability(capability))
    }

    fn query_journal(
        &self,
        request: QueryJournalRequest,
    ) -> Result<BrokerResponse, PersistentChangeJournalOperationError> {
        let query_index = self.state.query_count.fetch_add(1, Ordering::AcqRel);
        if query_index == 0
            && let Some(cancelled) = self
                .state
                .cancel_after_first_query
                .lock()
                .expect("cancellation hook")
                .as_ref()
        {
            cancelled.store(true, Ordering::Release);
        }
        let next_usn = self
            .state
            .query_next_usns
            .lock()
            .expect("query boundaries")
            .pop_front()
            .unwrap_or(20);
        let journal_id = self
            .state
            .query_journal_ids
            .lock()
            .expect("query journal identities")
            .pop_front()
            .unwrap_or(44);
        let first_usn = self
            .state
            .query_first_usns
            .lock()
            .expect("query first USNs")
            .pop_front()
            .unwrap_or(1);
        Ok(BrokerResponse::Journal {
            request_id: request.root.root_generation,
            binding: ResponseBinding::from_request(&request.caller, &request.root),
            capability: JournalCapability::Supported,
            journal_id: Some(journal_id),
            first_usn: Some(first_usn),
            next_usn: Some(next_usn),
        })
    }

    fn begin_read_range(
        &self,
        _request: crate::journal_broker::ReadJournalRangeRequest,
    ) -> Result<Box<dyn PersistentChangeJournalRead>, PersistentChangeJournalOperationError> {
        Err(PersistentChangeJournalOperationError::InvalidRequest)
    }

    fn begin_read_volume(
        &self,
        request: ReadJournalVolumeRequest,
    ) -> Result<Box<dyn PersistentChangeJournalRead>, PersistentChangeJournalOperationError> {
        self.state.shared_read_count.fetch_add(1, Ordering::AcqRel);
        *self.state.captured.lock().expect("captured request") = Some(request.clone());
        let first = &request.roots[0];
        let mut outcomes = vec![SharedJournalRootOutcome {
            binding: ResponseBinding::from_request(&request.caller, &first.root),
            requested_start_usn: first.start_usn,
            covered_until_usn: Some(request.end_usn),
            is_complete: true,
            candidates: vec![BrokerCandidate {
                scope: CandidateScope::RelativePath("created.jpg".to_owned()),
                previous_scope: None,
                file_reference: vec![3; 8],
                usn: 14,
                kind: CandidateKind::Path,
                is_directory: false,
            }],
            failure: None,
        }];
        if let Some(second) = request.roots.get(1) {
            if self.state.fail_second_root.load(Ordering::Acquire) {
                outcomes.push(SharedJournalRootOutcome {
                    binding: ResponseBinding::from_request(&request.caller, &second.root),
                    requested_start_usn: second.start_usn,
                    covered_until_usn: None,
                    is_complete: false,
                    candidates: Vec::new(),
                    failure: Some(BrokerFailure {
                        code: BrokerFailureCode::RootUnauthorized,
                    }),
                });
            } else {
                outcomes.push(SharedJournalRootOutcome {
                    binding: ResponseBinding::from_request(&request.caller, &second.root),
                    requested_start_usn: second.start_usn,
                    covered_until_usn: Some(request.end_usn),
                    is_complete: true,
                    candidates: Vec::new(),
                    failure: None,
                });
            }
        }
        let pending_renames = if self.state.emit_pending_old.load(Ordering::Acquire) {
            vec![SharedJournalPendingRename {
                binding: ResponseBinding::from_request(&request.caller, &first.root),
                file_reference: vec![6; 16],
                old_usn: 15,
                previous_relative_path: "old.jpg".to_owned(),
                is_directory: false,
            }]
        } else {
            Vec::new()
        };
        let handoffs = if self.state.consume_first_pending.load(Ordering::Acquire) {
            request
                .pending_renames
                .first()
                .map(|pending| {
                    vec![crate::journal_broker::SharedJournalHandoff {
                        previous_binding: ResponseBinding::from_request(
                            &request.caller,
                            &pending.root,
                        ),
                        current_binding: ResponseBinding::from_request(
                            &request.caller,
                            &first.root,
                        ),
                        file_reference: pending.file_reference.clone(),
                        usn: 16,
                        previous_relative_path: pending.previous_relative_path.clone(),
                        current_relative_path: "moved.jpg".to_owned(),
                        is_directory: pending.is_directory,
                        previous_carry_id: Some(pending.carry_id.clone()),
                    }]
                })
                .unwrap_or_default()
        } else {
            Vec::new()
        };
        let response = BrokerResponse::ReadVolume {
            request_id: 1,
            client_instance: request.caller.client_instance,
            volume_id: first.root.volume_id.clone(),
            journal_id: request.journal_id,
            requested_end_usn: request.end_usn,
            max_records: request.max_records,
            max_evidence_bytes: request.max_evidence_bytes,
            outcomes,
            handoffs,
            pending_renames,
        };
        Ok(Box::new(FakePending { response }))
    }

    fn close(&self) -> Result<(), PersistentChangeJournalOperationError> {
        self.state.close_count.fetch_add(1, Ordering::AcqRel);
        Ok(())
    }
}

struct FakePending {
    response: BrokerResponse,
}

impl PersistentChangeJournalRead for FakePending {
    fn cancel(&self, _caller: CallerClaim) -> Result<(), PersistentChangeJournalOperationError> {
        Ok(())
    }

    fn wait(self: Box<Self>) -> Result<BrokerResponse, PersistentChangeJournalOperationError> {
        Ok(self.response)
    }
}

#[test]
fn session_reader_uses_one_shared_request_and_preserves_root_failure_isolation() {
    let state = Arc::new(SessionState::default());
    state.fail_second_root.store(true, Ordering::Release);
    let reader = reader(Arc::clone(&state));
    let checkpoints = vec![checkpoint("root-a", 1, 10), checkpoint("root-b", 2, 12)];
    let outcomes = reader
        .read_volume(&checkpoints, &[], 500, &AtomicBool::new(false))
        .expect("shared volume read");

    assert_eq!(state.register_count.load(Ordering::Acquire), 2);
    assert_eq!(state.query_count.load(Ordering::Acquire), 2);
    assert_eq!(state.shared_read_count.load(Ordering::Acquire), 1);
    assert_eq!(state.close_count.load(Ordering::Acquire), 1);
    let captured = state.captured.lock().expect("captured request");
    let captured = captured.as_ref().expect("one shared request");
    assert_eq!(captured.roots.len(), 2);
    assert_eq!(captured.roots[0].start_usn, 10);
    assert_eq!(captured.roots[1].start_usn, 12);
    assert_eq!(captured.end_usn, 20);
    assert_eq!(captured.max_records, 64);
    let (batch, advanced) = outcomes[0]
        .page
        .as_ref()
        .expect("first root result")
        .as_ref()
        .expect("first root page");
    assert_eq!(batch.intents.len(), 1);
    assert!(batch.cross_root_lineage.is_empty());
    assert_eq!(batch.intents[0].relative_path, "created.jpg");
    assert_eq!(advanced.next_unread_usn.value(), 20);
    assert_eq!(
        outcomes[0]
            .opening_boundary
            .as_ref()
            .expect("fresh opening boundary")
            .next_usn
            .value(),
        20
    );
    assert_eq!(
        outcomes[1]
            .page
            .as_ref()
            .expect_err("isolated root failure")
            .kind,
        PersistentJournalRootFailureKind::ContainmentFailure,
    );
}

#[test]
fn session_reader_cancellation_after_one_root_prevents_all_later_broker_calls() {
    let state = Arc::new(SessionState::default());
    let cancelled = Arc::new(AtomicBool::new(false));
    *state
        .cancel_after_first_query
        .lock()
        .expect("install cancellation hook") = Some(Arc::clone(&cancelled));
    let checkpoints = (0..8)
        .map(|index| {
            checkpoint(
                &format!("root-{index}"),
                u64::try_from(index + 1).expect("root generation"),
                10 + i64::from(index),
            )
        })
        .collect::<Vec<_>>();
    let journal: Arc<dyn PersistentChangeJournal> = Arc::new(FakeJournal {
        state: Arc::clone(&state),
    });
    let reader = SessionBackedPersistentJournalVolumeReader::new(
        journal,
        caller(),
        (0..8)
            .map(|index| {
                broker_root(
                    &format!("root-{index}"),
                    u64::try_from(index + 1).expect("root generation"),
                )
            })
            .collect(),
    )
    .expect("eight-root reader");

    let failure = reader
        .read_volume(&checkpoints, &[], 500, &cancelled)
        .expect_err("cancellation must isolate the later root");

    assert_eq!(failure.kind, PersistentJournalRootFailureKind::Cancelled);
    assert_eq!(state.register_count.load(Ordering::Acquire), 1);
    assert_eq!(state.query_count.load(Ordering::Acquire), 1);
    assert_eq!(state.shared_read_count.load(Ordering::Acquire), 0);
    assert_eq!(state.close_count.load(Ordering::Acquire), 1);
}

#[test]
fn session_reader_classifies_reset_and_trim_from_structured_query_fields() {
    let reset_state = Arc::new(SessionState::default());
    reset_state
        .query_journal_ids
        .lock()
        .expect("reset journal IDs")
        .push_back(45);
    let reset = reader(Arc::clone(&reset_state))
        .read_volume(
            &[checkpoint("root-a", 1, 10)],
            &[],
            500,
            &AtomicBool::new(false),
        )
        .expect("reset outcome");
    let reset = reset[0].page.as_ref().expect_err("journal reset");
    assert_eq!(reset.kind, PersistentJournalRootFailureKind::JournalReset);
    assert_eq!(reset.failure.code, "persistent_journal_reset");
    assert_eq!(
        reset
            .opening_boundary
            .as_ref()
            .expect("reset opening")
            .journal_id
            .value(),
        45
    );
    assert_eq!(reset_state.shared_read_count.load(Ordering::Acquire), 0);

    let trim_state = Arc::new(SessionState::default());
    trim_state
        .query_first_usns
        .lock()
        .expect("trim first USNs")
        .push_back(11);
    let trim = reader(Arc::clone(&trim_state))
        .read_volume(
            &[checkpoint("root-a", 1, 10)],
            &[],
            500,
            &AtomicBool::new(false),
        )
        .expect("trim outcome");
    let trim = trim[0].page.as_ref().expect_err("journal trim");
    assert_eq!(trim.kind, PersistentJournalRootFailureKind::JournalTrim);
    assert_eq!(trim.failure.code, "persistent_journal_trimmed");
    assert_eq!(
        trim.opening_boundary
            .as_ref()
            .expect("trim opening")
            .next_usn
            .value(),
        20
    );
    assert_eq!(trim_state.shared_read_count.load(Ordering::Acquire), 0);
}

#[test]
fn session_reader_rejects_cross_volume_set_before_connecting() {
    let state = Arc::new(SessionState::default());
    let reader = reader(Arc::clone(&state));
    let mut other = checkpoint("root-b", 2, 12);
    other.volume.volume_guid = "volume-other".to_owned();
    assert!(
        reader
            .read_volume(
                &[checkpoint("root-a", 1, 10), other],
                &[],
                500,
                &AtomicBool::new(false),
            )
            .is_err()
    );
    assert_eq!(state.register_count.load(Ordering::Acquire), 0);
}

#[test]
fn equal_checkpoints_are_successful_no_ops_without_a_shared_read() {
    let state = Arc::new(SessionState::default());
    let reader = reader(Arc::clone(&state));
    let outcomes = reader
        .read_volume(
            &[checkpoint("root-a", 1, 20), checkpoint("root-b", 2, 20)],
            &[],
            500,
            &AtomicBool::new(false),
        )
        .expect("no-change read");

    assert_eq!(state.shared_read_count.load(Ordering::Acquire), 0);
    assert!(
        outcomes
            .iter()
            .all(|outcome| matches!(&outcome.page, Ok(None)))
    );
}

#[test]
fn retained_session_no_change_read_does_not_close_or_issue_a_shared_read() {
    let state = Arc::new(SessionState::default());
    let session: Arc<dyn PersistentChangeJournalSession> = Arc::new(FakeSession {
        state: Arc::clone(&state),
    });
    let reader = SessionBackedPersistentJournalVolumeReader::with_retained_session(
        Arc::clone(&session),
        caller(),
        vec![broker_root("root-a", 1), broker_root("root-b", 2)],
    )
    .expect("retained reader");

    let outcomes = reader
        .read_volume(
            &[checkpoint("root-a", 1, 20), checkpoint("root-b", 2, 20)],
            &[],
            500,
            &AtomicBool::new(false),
        )
        .expect("retained no-change read");

    assert_eq!(state.shared_read_count.load(Ordering::Acquire), 0);
    assert_eq!(state.close_count.load(Ordering::Acquire), 0);
    assert!(
        outcomes
            .iter()
            .all(|outcome| matches!(&outcome.page, Ok(None)))
    );
    session.close().expect("close retained session once");
    assert_eq!(state.close_count.load(Ordering::Acquire), 1);
}

#[test]
fn root_at_the_common_end_does_not_block_a_progressing_sibling() {
    let state = Arc::new(SessionState::default());
    let reader = reader(Arc::clone(&state));
    let outcomes = reader
        .read_volume(
            &[checkpoint("root-a", 1, 10), checkpoint("root-b", 2, 20)],
            &[],
            500,
            &AtomicBool::new(false),
        )
        .expect("partitioned read");

    assert_eq!(state.shared_read_count.load(Ordering::Acquire), 1);
    let captured = state.captured.lock().expect("captured request");
    assert_eq!(captured.as_ref().expect("shared request").roots.len(), 1);
    assert!(matches!(&outcomes[0].page, Ok(Some(_))));
    assert!(matches!(&outcomes[1].page, Ok(None)));
}

#[test]
fn growing_query_boundary_is_shared_before_any_root_advances_past_it() {
    let state = Arc::new(SessionState::default());
    state
        .query_next_usns
        .lock()
        .expect("query boundaries")
        .extend([20, 30, 30, 30]);
    let reader = reader(Arc::clone(&state));

    let first = reader
        .read_volume(
            &[checkpoint("root-a", 1, 20), checkpoint("root-b", 2, 10)],
            &[],
            500,
            &AtomicBool::new(false),
        )
        .expect("first bounded read");
    let captured = state.captured.lock().expect("captured request");
    let request = captured.as_ref().expect("first shared request");
    assert_eq!(request.end_usn, 20);
    assert_eq!(request.roots.len(), 1);
    assert_eq!(request.roots[0].root.root_id, "root-b");
    assert!(matches!(&first[0].page, Ok(None)));
    assert_eq!(
        first[1]
            .page
            .as_ref()
            .expect("second root result")
            .as_ref()
            .expect("second root page")
            .1
            .next_unread_usn
            .value(),
        20
    );
    drop(captured);

    let second = reader
        .read_volume(
            &[checkpoint("root-a", 1, 20), checkpoint("root-b", 2, 20)],
            &[],
            501,
            &AtomicBool::new(false),
        )
        .expect("second bounded read");
    let captured = state.captured.lock().expect("captured request");
    let request = captured.as_ref().expect("second shared request");
    assert_eq!(request.end_usn, 30);
    assert_eq!(request.roots.len(), 2);
    assert_eq!(state.shared_read_count.load(Ordering::Acquire), 2);
    assert!(second.iter().all(|outcome| outcome.page.is_ok()));
}

#[test]
fn session_carries_pending_old_into_a_later_cross_root_page() {
    let state = Arc::new(SessionState::default());
    state.emit_pending_old.store(true, Ordering::Release);
    let reader = reader(Arc::clone(&state));
    let first = reader
        .read_volume(
            &[checkpoint("root-a", 1, 10), checkpoint("root-b", 2, 20)],
            &[],
            500,
            &AtomicBool::new(false),
        )
        .expect("OLD page");
    let old_page = first[0]
        .page
        .as_ref()
        .expect("OLD root result")
        .as_ref()
        .expect("OLD root page");
    assert_eq!(old_page.0.pending_renames.len(), 1);
    let carry = old_page.0.pending_renames[0].clone();
    assert_eq!(carry.source_range_id, old_page.0.range.batch_id);
    carry.validate().expect("durable carry");

    state.emit_pending_old.store(false, Ordering::Release);
    state.consume_first_pending.store(true, Ordering::Release);
    let second = reader
        .read_volume(
            &[checkpoint("root-a", 1, 20), checkpoint("root-b", 2, 10)],
            std::slice::from_ref(&carry),
            501,
            &AtomicBool::new(false),
        )
        .expect("NEW page");
    let new_page = second[1]
        .page
        .as_ref()
        .expect("NEW root result")
        .as_ref()
        .expect("NEW root page");
    assert_eq!(
        new_page.0.consumed_pending_rename_ids,
        std::slice::from_ref(&carry.carry_id)
    );
    assert_eq!(new_page.0.carried_cross_root_lineage.len(), 1);
    assert_eq!(new_page.0.cross_root_lineage.len(), 1);
    assert_eq!(
        new_page.0.carried_cross_root_lineage[0].owner_source_range_id,
        carry.source_range_id
    );
    assert_eq!(
        new_page.0.cross_root_lineage[0].owner_source_range_id,
        new_page.0.range.batch_id
    );
    assert_eq!(
        new_page.0.carried_cross_root_lineage[0].lineage_id,
        new_page.0.cross_root_lineage[0].lineage_id
    );
    new_page.0.validate().expect("carried NEW batch");
}

#[test]
fn session_rejects_pending_or_carried_rename_outside_durable_request_evidence() {
    let previous = checkpoint("root-a", 1, 10);
    let previous_page =
        translate_root_outcome(&previous, successful_outcome(&previous, 10, 20), 20, 500)
            .expect("previous page");
    let mut previous_outcomes = vec![(
        (previous.root_id.clone(), previous.root_generation),
        Ok(Some(previous_page)),
    )];
    let outside = SharedJournalPendingRename {
        binding: first_binding(&previous),
        file_reference: vec![6; 16],
        old_usn: 9,
        previous_relative_path: "old.jpg".to_owned(),
        is_directory: false,
    };
    assert!(
        attach_pending_renames(
            &mut previous_outcomes,
            &[outside],
            &caller(),
            &previous.volume,
            44,
            500,
        )
        .is_err()
    );

    attach_pending_renames(
        &mut previous_outcomes,
        &[SharedJournalPendingRename {
            binding: first_binding(&previous),
            file_reference: vec![6; 16],
            old_usn: 15,
            previous_relative_path: "old.jpg".to_owned(),
            is_directory: false,
        }],
        &caller(),
        &previous.volume,
        44,
        500,
    )
    .expect("attach valid pending OLD");
    finalize_batch_ids(&mut previous_outcomes).expect("finalize OLD range");
    let carry = previous_outcomes[0]
        .1
        .as_ref()
        .expect("successful OLD page")
        .as_ref()
        .expect("OLD page")
        .0
        .pending_renames[0]
        .clone();

    let current = checkpoint("root-b", 2, 12);
    let current_page =
        translate_root_outcome(&current, successful_outcome(&current, 12, 20), 20, 501)
            .expect("current page");
    let mut current_outcomes = vec![(
        (current.root_id.clone(), current.root_generation),
        Ok(Some(current_page)),
    )];
    let mut request = handoff_request(&[&current]);
    request
        .pending_renames
        .push(SharedJournalPendingRenameRequest {
            carry_id: carry.carry_id.clone(),
            source_range_id: carry.source_range_id.clone(),
            root: broker_root("root-a", 1).authorization,
            root_capability: Some(RootCapability([7; 32])),
            file_reference: carry.file_reference.as_bytes().to_vec(),
            old_usn: carry.old_usn.value(),
            previous_relative_path: carry.previous_relative_path.clone(),
            is_directory: carry.is_directory,
        });
    let malformed = crate::journal_broker::SharedJournalHandoff {
        previous_binding: first_binding(&previous),
        current_binding: first_binding(&current),
        file_reference: carry.file_reference.as_bytes().to_vec(),
        usn: 16,
        previous_relative_path: "tampered.jpg".to_owned(),
        current_relative_path: "moved.jpg".to_owned(),
        is_directory: false,
        previous_carry_id: Some(carry.carry_id.clone()),
    };
    assert!(
        attach_cross_root_lineage(
            &mut current_outcomes,
            &[malformed],
            &[carry],
            &request,
            &current.volume,
            44,
        )
        .is_err()
    );
}

#[test]
fn half_lineage_is_rejected_and_successful_endpoints_share_one_identity() {
    let first = checkpoint("root-a", 1, 10);
    let second = checkpoint("root-b", 2, 12);
    let first_page = translate_root_outcome(&first, successful_outcome(&first, 10, 20), 20, 500)
        .expect("first page");
    let second_page = translate_root_outcome(&second, successful_outcome(&second, 12, 20), 20, 500)
        .expect("second page");
    let handoff = crate::journal_broker::SharedJournalHandoff {
        previous_binding: first_binding(&first),
        current_binding: first_binding(&second),
        file_reference: vec![7; 16],
        usn: 15,
        previous_relative_path: "source.jpg".to_owned(),
        current_relative_path: "moved.jpg".to_owned(),
        is_directory: false,
        previous_carry_id: None,
    };
    let request = handoff_request(&[&first, &second]);
    let mut half = vec![
        (
            (first.root_id.clone(), first.root_generation),
            Ok(Some(first_page.clone())),
        ),
        (
            (second.root_id.clone(), second.root_generation),
            Err(reconstruction_failure("failed endpoint", None)),
        ),
    ];
    assert!(
        attach_cross_root_lineage(
            &mut half,
            std::slice::from_ref(&handoff),
            &[],
            &request,
            &first.volume,
            44,
        )
        .is_err()
    );

    let mut complete = vec![
        (
            (first.root_id.clone(), first.root_generation),
            Ok(Some(first_page)),
        ),
        (
            (second.root_id.clone(), second.root_generation),
            Ok(Some(second_page)),
        ),
    ];
    attach_cross_root_lineage(
        &mut complete,
        std::slice::from_ref(&handoff),
        &[],
        &request,
        &first.volume,
        44,
    )
    .expect("complete lineage");
    let lineage_ids = complete
        .iter()
        .map(|(_, page)| {
            &page
                .as_ref()
                .expect("successful page")
                .as_ref()
                .expect("page")
                .0
                .cross_root_lineage[0]
                .lineage_id
        })
        .collect::<Vec<_>>();
    assert_eq!(lineage_ids[0], lineage_ids[1]);
}

#[test]
fn durable_range_identity_changes_with_volume_or_root_generation() {
    let original = checkpoint("root-a", 1, 10);
    let original_id =
        translate_root_outcome(&original, successful_outcome(&original, 10, 20), 20, 500)
            .expect("original page")
            .0
            .range
            .batch_id;
    let mut changed_volume = original.clone();
    changed_volume.volume.volume_guid = "other-volume".to_owned();
    let changed_volume_id = translate_root_outcome(
        &changed_volume,
        successful_outcome(&changed_volume, 10, 20),
        20,
        500,
    )
    .expect("changed volume page")
    .0
    .range
    .batch_id;
    let changed_generation = checkpoint("root-a", 2, 10);
    let changed_generation_id = translate_root_outcome(
        &changed_generation,
        successful_outcome(&changed_generation, 10, 20),
        20,
        500,
    )
    .expect("changed generation page")
    .0
    .range
    .batch_id;

    assert_ne!(original_id, changed_volume_id);
    assert_ne!(original_id, changed_generation_id);

    let second = checkpoint("root-b", 2, 12);
    let original_handoff_id = attached_lineage_id(&original, &second);
    let changed_range = checkpoint("root-a", 1, 9);
    let changed_handoff_id = attached_lineage_id(&changed_range, &second);
    assert_ne!(original_handoff_id, changed_handoff_id);
    let replaced_root = checkpoint("root-a", 2, 10);
    let replaced_root_handoff_id = attached_lineage_id(&replaced_root, &second);
    assert_ne!(original_handoff_id, replaced_root_handoff_id);
    let mut moved_volume_previous = original.clone();
    moved_volume_previous.volume.volume_guid = "other-volume".to_owned();
    let mut moved_volume_current = second;
    moved_volume_current.volume.volume_guid = "other-volume".to_owned();
    let moved_volume_handoff_id =
        attached_lineage_id(&moved_volume_previous, &moved_volume_current);
    assert_ne!(original_handoff_id, moved_volume_handoff_id);
}

fn attached_lineage_id(
    previous: &PersistentJournalCheckpoint,
    current: &PersistentJournalCheckpoint,
) -> String {
    let previous_page = translate_root_outcome(
        previous,
        successful_outcome(previous, previous.next_unread_usn.value(), 20),
        20,
        500,
    )
    .expect("previous page");
    let current_page = translate_root_outcome(
        current,
        successful_outcome(current, current.next_unread_usn.value(), 20),
        20,
        500,
    )
    .expect("current page");
    let mut outcomes = vec![
        (
            (previous.root_id.clone(), previous.root_generation),
            Ok(Some(previous_page)),
        ),
        (
            (current.root_id.clone(), current.root_generation),
            Ok(Some(current_page)),
        ),
    ];
    let request = handoff_request(&[previous, current]);
    attach_cross_root_lineage(
        &mut outcomes,
        &[crate::journal_broker::SharedJournalHandoff {
            previous_binding: first_binding(previous),
            current_binding: first_binding(current),
            file_reference: vec![7; 16],
            usn: 15,
            previous_relative_path: "source.jpg".to_owned(),
            current_relative_path: "moved.jpg".to_owned(),
            is_directory: false,
            previous_carry_id: None,
        }],
        &[],
        &request,
        &previous.volume,
        44,
    )
    .expect("attached lineage");
    outcomes[0]
        .1
        .as_ref()
        .expect("successful page")
        .as_ref()
        .expect("page")
        .0
        .cross_root_lineage[0]
        .lineage_id
        .clone()
}

fn handoff_request(checkpoints: &[&PersistentJournalCheckpoint]) -> ReadJournalVolumeRequest {
    ReadJournalVolumeRequest {
        caller: caller(),
        roots: checkpoints
            .iter()
            .map(|checkpoint| SharedJournalRootRequest {
                root: RootAuthorization {
                    root_id: checkpoint.root_id.clone(),
                    root_generation: checkpoint.root_generation.value(),
                    volume_id: checkpoint.volume.volume_guid.clone(),
                    root_identity: match checkpoint.root_file_reference {
                        JournalFileReference::V2(reference) => reference.to_vec(),
                        JournalFileReference::V3(reference) => reference.to_vec(),
                    },
                    canonical_root_utf16: format!("C:\\{}", checkpoint.root_id)
                        .encode_utf16()
                        .collect(),
                },
                root_capability: RootCapability([7; 32]),
                start_usn: checkpoint.next_unread_usn.value(),
            })
            .collect(),
        pending_renames: Vec::new(),
        journal_id: 44,
        end_usn: 20,
        max_records: DEFAULT_MAX_RECORDS,
        max_evidence_bytes: DEFAULT_MAX_EVIDENCE_BYTES,
        timeout_ms: DEFAULT_TIMEOUT_MS,
    }
}

fn successful_outcome(
    checkpoint: &PersistentJournalCheckpoint,
    requested_start_usn: i64,
    covered_until_usn: i64,
) -> SharedJournalRootOutcome {
    SharedJournalRootOutcome {
        binding: first_binding(checkpoint),
        requested_start_usn,
        covered_until_usn: Some(covered_until_usn),
        is_complete: true,
        candidates: Vec::new(),
        failure: None,
    }
}

fn first_binding(checkpoint: &PersistentJournalCheckpoint) -> ResponseBinding {
    ResponseBinding {
        client_instance: caller().client_instance,
        root_id: checkpoint.root_id.clone(),
        root_generation: checkpoint.root_generation.value(),
        volume_id: checkpoint.volume.volume_guid.clone(),
    }
}

fn reader(state: Arc<SessionState>) -> SessionBackedPersistentJournalVolumeReader {
    let journal: Arc<dyn PersistentChangeJournal> = Arc::new(FakeJournal { state });
    SessionBackedPersistentJournalVolumeReader::new(
        journal,
        caller(),
        vec![broker_root("root-a", 1), broker_root("root-b", 2)],
    )
    .expect("reader")
}

fn broker_root(root_id: &str, generation: u64) -> PersistentJournalBrokerRoot {
    PersistentJournalBrokerRoot {
        authorization: RootAuthorization {
            root_id: root_id.to_owned(),
            root_generation: generation,
            volume_id: "volume-guid".to_owned(),
            root_identity: vec![generation as u8; 16],
            canonical_root_utf16: format!("C:\\{root_id}").encode_utf16().collect(),
        },
        client_root_handle: generation,
        volume_serial: 77,
    }
}

fn checkpoint(root_id: &str, generation: u64, start: i64) -> PersistentJournalCheckpoint {
    PersistentJournalCheckpoint {
        root_id: root_id.to_owned(),
        root_generation: LibraryRootGeneration::new(generation).expect("generation"),
        volume: PersistentJournalVolumeIdentity {
            volume_guid: "volume-guid".to_owned(),
            volume_serial: 77,
        },
        root_file_reference: JournalFileReference::V3([generation as u8; 16]),
        journal_id: JournalIdentifier::new(44).expect("journal"),
        next_unread_usn: JournalUsn::new(start).expect("start"),
        captured_exclusive_end: JournalUsn::new(start).expect("captured"),
        covered_catalog_revision: 9,
        protocol_version: PROTOCOL_VERSION,
        contract_version: PERSISTENT_JOURNAL_CONTRACT_VERSION,
        continuity: PersistentJournalContinuityState::CatchingUp,
        failure: None,
        updated_unix_ms: 400,
    }
}

fn caller() -> CallerClaim {
    CallerClaim {
        process_id: 10,
        session_id: 2,
        client_instance: [9; 16],
    }
}

#[test]
fn live_only_connection_fails_closed() {
    struct LiveOnlyJournal;
    impl PersistentChangeJournal for LiveOnlyJournal {
        fn connect(&self) -> PersistentChangeJournalConnection {
            PersistentChangeJournalConnection::LiveOnly(
                PersistentChangeJournalLiveOnlyReason::BrokerAbsent,
            )
        }
    }
    let reader = SessionBackedPersistentJournalVolumeReader::new(
        Arc::new(LiveOnlyJournal),
        caller(),
        vec![broker_root("root-a", 1)],
    )
    .expect("reader");
    assert!(
        reader
            .read_volume(
                &[checkpoint("root-a", 1, 10)],
                &[],
                500,
                &AtomicBool::new(false),
            )
            .is_err()
    );
}
