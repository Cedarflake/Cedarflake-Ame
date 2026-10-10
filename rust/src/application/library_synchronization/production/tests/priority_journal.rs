use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicUsize, Ordering};

use crate::journal_broker::{
    BrokerCandidate, BrokerResponse, CandidateKind, CandidateScope, JournalCapability,
    PersistentChangeJournalOperationError, PersistentChangeJournalRead,
    PersistentChangeJournalSession, QueryJournalRequest, ReadJournalRangeRequest,
    ReadJournalVolumeRequest, RegisterRootRequest, ResponseBinding, RootCapability,
    SharedJournalRootOutcome,
};

use super::CompletedJournalRead;

pub(super) struct PriorityJournalSession {
    pub(super) p1_root_id: String,
    pub(super) first_usn: i64,
    pub(super) published_next_usn: Arc<AtomicI64>,
    pub(super) candidate_count: usize,
    pub(super) shared_read_count: Arc<AtomicUsize>,
    pub(super) close_count: Arc<AtomicUsize>,
}

impl PersistentChangeJournalSession for PriorityJournalSession {
    fn register_root(
        &self,
        _request: RegisterRootRequest,
    ) -> Result<RootCapability, PersistentChangeJournalOperationError> {
        Ok(RootCapability([9; 32]))
    }

    fn query_journal(
        &self,
        request: QueryJournalRequest,
    ) -> Result<BrokerResponse, PersistentChangeJournalOperationError> {
        Ok(BrokerResponse::Journal {
            request_id: 1,
            binding: ResponseBinding::from_request(&request.caller, &request.root),
            capability: JournalCapability::Supported,
            journal_id: Some(44),
            first_usn: Some(1),
            next_usn: Some(self.published_next_usn.load(Ordering::Acquire)),
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
        let outcomes = request
            .roots
            .iter()
            .map(|root| {
                let available_count = request
                    .end_usn
                    .saturating_sub(root.start_usn)
                    .try_into()
                    .unwrap_or(usize::MAX);
                let first_index = if root.root.root_id == self.p1_root_id {
                    usize::try_from(root.start_usn.saturating_sub(self.first_usn))
                        .unwrap_or(usize::MAX)
                        .min(self.candidate_count)
                } else {
                    0
                };
                let candidate_count = if root.root.root_id == self.p1_root_id {
                    self.candidate_count
                        .saturating_sub(first_index)
                        .min(available_count)
                        .min(request.max_records as usize)
                } else {
                    0
                };
                let candidates = (0..candidate_count)
                    .map(|page_index| {
                        let candidate_index = first_index
                            .checked_add(page_index)
                            .expect("candidate index range");
                        let sequence = root
                            .start_usn
                            .checked_add(i64::try_from(page_index).expect("candidate sequence"))
                            .expect("candidate sequence range");
                        BrokerCandidate {
                            scope: CandidateScope::RelativePath(format!(
                                "journal-{candidate_index:04}.png"
                            )),
                            previous_scope: None,
                            file_reference: u64::try_from(candidate_index + 1)
                                .expect("candidate file reference")
                                .to_le_bytes()
                                .to_vec(),
                            usn: sequence,
                            kind: CandidateKind::Path,
                            is_directory: false,
                        }
                    })
                    .collect();
                let covered_until_usn = if root.root.root_id == self.p1_root_id {
                    root.start_usn
                        .checked_add(i64::try_from(candidate_count).expect("covered count"))
                        .expect("covered USN range")
                } else {
                    request.end_usn
                };
                SharedJournalRootOutcome {
                    binding: ResponseBinding::from_request(&request.caller, &root.root),
                    requested_start_usn: root.start_usn,
                    covered_until_usn: Some(covered_until_usn),
                    is_complete: covered_until_usn == request.end_usn,
                    candidates,
                    failure: None,
                }
            })
            .collect();
        Ok(Box::new(CompletedJournalRead {
            response: BrokerResponse::ReadVolume {
                request_id: 1,
                client_instance: request.caller.client_instance,
                volume_id: request
                    .roots
                    .first()
                    .ok_or(PersistentChangeJournalOperationError::InvalidRequest)?
                    .root
                    .volume_id
                    .clone(),
                journal_id: request.journal_id,
                requested_end_usn: request.end_usn,
                max_records: request.max_records,
                max_evidence_bytes: request.max_evidence_bytes,
                outcomes,
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

mod tests {
    use super::*;
    use crate::journal_broker::{CallerClaim, RootAuthorization, SharedJournalRootRequest};

    fn session() -> PriorityJournalSession {
        PriorityJournalSession {
            p1_root_id: "p1".to_owned(),
            first_usn: 20,
            published_next_usn: Arc::new(AtomicI64::new(23)),
            candidate_count: 3,
            shared_read_count: Arc::new(AtomicUsize::new(0)),
            close_count: Arc::new(AtomicUsize::new(0)),
        }
    }

    fn caller() -> CallerClaim {
        CallerClaim {
            process_id: std::process::id(),
            session_id: 1,
            client_instance: [4; 16],
        }
    }

    fn root(root_id: &str, identity: u8) -> RootAuthorization {
        RootAuthorization {
            root_id: root_id.to_owned(),
            root_generation: 1,
            volume_id: "fixture-volume".to_owned(),
            root_identity: vec![identity; 16],
            canonical_root_utf16: format!("C:\\fixture\\{root_id}").encode_utf16().collect(),
        }
    }

    #[test]
    fn same_volume_roots_share_the_published_journal_upper_bound() {
        let session = session();
        for expected_next_usn in [23, 26] {
            session
                .published_next_usn
                .store(expected_next_usn, Ordering::Release);
            for root in [root("p1", 1), root("p2", 2)] {
                let request = QueryJournalRequest {
                    caller: caller(),
                    root,
                    root_capability: RootCapability([9; 32]),
                    timeout_ms: 1_000,
                };
                request.validate().unwrap();
                assert_eq!(
                    session.query_journal(request.clone()).unwrap(),
                    BrokerResponse::Journal {
                        request_id: 1,
                        binding: ResponseBinding::from_request(&request.caller, &request.root),
                        capability: JournalCapability::Supported,
                        journal_id: Some(44),
                        first_usn: Some(1),
                        next_usn: Some(expected_next_usn),
                    }
                );
            }
        }
    }

    #[test]
    fn empty_p2_candidates_cover_a_nonempty_usn_window_without_changing_p1_paging() {
        let session = session();
        let mut request = ReadJournalVolumeRequest {
            caller: caller(),
            roots: [root("p1", 1), root("p2", 2)]
                .into_iter()
                .map(|root| SharedJournalRootRequest {
                    root,
                    root_capability: RootCapability([9; 32]),
                    start_usn: 20,
                })
                .collect(),
            pending_renames: Vec::new(),
            journal_id: 44,
            end_usn: 23,
            max_records: 2,
            max_evidence_bytes: 4_096,
            timeout_ms: 1_000,
        };
        request.validate().unwrap();
        let BrokerResponse::ReadVolume { outcomes, .. } = session
            .begin_read_volume(request.clone())
            .unwrap()
            .wait()
            .unwrap()
        else {
            panic!("expected shared journal response");
        };
        assert_eq!(outcomes.len(), 2);
        let p1 = &outcomes[0];
        assert_eq!(p1.requested_start_usn, 20);
        assert_eq!(p1.covered_until_usn, Some(22));
        assert!(!p1.is_complete);
        assert_eq!(p1.candidates.len(), 2);
        assert_eq!(p1.candidates[0].usn, 20);
        assert_eq!(p1.candidates[1].usn, 21);
        assert_eq!(
            p1.candidates[1].scope,
            CandidateScope::RelativePath("journal-0001.png".to_owned())
        );
        assert_eq!(
            outcomes[1],
            SharedJournalRootOutcome {
                binding: ResponseBinding::from_request(&request.caller, &request.roots[1].root),
                requested_start_usn: 20,
                covered_until_usn: Some(23),
                is_complete: true,
                candidates: Vec::new(),
                failure: None,
            }
        );

        request.roots.truncate(1);
        request.roots[0].start_usn = 22;
        request.validate().unwrap();
        let BrokerResponse::ReadVolume { outcomes, .. } =
            session.begin_read_volume(request).unwrap().wait().unwrap()
        else {
            panic!("expected shared journal response");
        };
        assert_eq!(outcomes.len(), 1);
        let p1 = &outcomes[0];
        assert_eq!(p1.requested_start_usn, 22);
        assert_eq!(p1.covered_until_usn, Some(23));
        assert!(p1.is_complete);
        assert_eq!(p1.candidates.len(), 1);
        assert_eq!(p1.candidates[0].usn, 22);
        assert_eq!(
            p1.candidates[0].scope,
            CandidateScope::RelativePath("journal-0002.png".to_owned())
        );
        assert_eq!(session.shared_read_count.load(Ordering::Acquire), 2);
        assert_eq!(session.close_count.load(Ordering::Acquire), 0);
        session.close().unwrap();
        assert_eq!(session.close_count.load(Ordering::Acquire), 1);
    }
}
