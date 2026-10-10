use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Barrier, Mutex};
use std::time::{Duration, Instant};

use super::contracts::RootNameSemanticsProof;
use super::*;
use crate::journal_broker::client::{
    AdapterPoll, BrokerClientError, BrokerClientTransport, JournalBrokerClient,
    OwnedBrokerConnection, sealed,
};
use crate::journal_broker::framing::{decode_frame, encode_frame};
use crate::journal_broker::wire::{
    BrokerRequest, CandidateScope, NameComparisonSemantics, RootCapability, decode_request,
    decode_response, encode_request, encode_response, inspect_request_header,
};

fn owned_connection<Transport>(transport: Transport) -> OwnedBrokerConnection<Transport> {
    static NEXT_CONNECTION_NONCE: AtomicUsize = AtomicUsize::new(1);
    let nonce = NEXT_CONNECTION_NONCE.fetch_add(1, Ordering::AcqRel) as u64;
    let mut connection_id = [0_u8; 16];
    connection_id[..8].copy_from_slice(&nonce.to_le_bytes());
    connection_id[8..].copy_from_slice(&(!nonce).to_le_bytes());
    OwnedBrokerConnection::from_adapter(transport, connection_id, 1, nonce)
        .expect("unique service test connection")
}

fn broker_client<Transport>(transport: Transport) -> JournalBrokerClient<Transport>
where
    Transport: BrokerClientTransport,
{
    JournalBrokerClient::new(owned_connection(transport))
}

#[derive(Clone)]
struct FakeAuthorizer {
    allow_root: bool,
    name_semantics: Vec<NameComparisonSemantics>,
}

impl CallerAuthorizer for FakeAuthorizer {
    fn verify_claim(
        &self,
        authenticated: &AuthenticatedConnection,
        claim: &CallerClaim,
    ) -> Result<CallerAuthorization, AuthorizerError> {
        if claim.process_id != authenticated.observed_process_id
            || claim.session_id != authenticated.observed_session_id
            || authenticated.opaque_token_binding == [0; 32]
        {
            return Err(AuthorizerError::CallerRejected);
        }
        CallerAuthorization::from_verified_claim(authenticated, claim)
    }

    fn register_root(
        &self,
        caller: &CallerAuthorization,
        root: &RootAuthorization,
        client_root_handle: u64,
        context: &mut RequestContext<'_>,
    ) -> Result<RootCapability, AuthorizerError> {
        if client_root_handle == 0 {
            return Err(AuthorizerError::RootUnauthorized);
        }
        let _ = self.authorize_for_test(caller, root, context)?;
        Ok(RootCapability([7; 32]))
    }

    fn authorize_registered_root(
        &self,
        caller: &CallerAuthorization,
        root: &RootAuthorization,
        capability: RootCapability,
        context: &mut RequestContext<'_>,
    ) -> Result<AuthorizedPinnedRoot, AuthorizerError> {
        if capability != RootCapability([7; 32]) {
            return Err(AuthorizerError::RootUnauthorized);
        }
        self.authorize_for_test(caller, root, context)
    }

    fn filter_disclosable_candidates(
        &self,
        _root: &AuthorizedPinnedRoot,
        candidates: Vec<BackendJournalCandidate>,
        context: &mut RequestContext<'_>,
    ) -> Result<Vec<BackendJournalCandidate>, AuthorizerError> {
        context
            .bounded_step()
            .map_err(|_| AuthorizerError::RootUnauthorized)?;
        Ok(candidates)
    }
}

impl FakeAuthorizer {
    fn authorize_for_test(
        &self,
        caller: &CallerAuthorization,
        root: &RootAuthorization,
        context: &mut RequestContext<'_>,
    ) -> Result<AuthorizedPinnedRoot, AuthorizerError> {
        if !self.allow_root
            || !matches!(root.root_id.as_str(), "root-a" | "root-b")
            || caller.opaque_namespace == [0; 32]
        {
            return Err(AuthorizerError::RootUnauthorized);
        }
        context
            .bounded_step()
            .map_err(|_| AuthorizerError::RootIdentityMismatch)?;
        let declared = DeclaredRootAuthorization::after_access_check(caller, root)?;
        let semantics =
            RootNameSemanticsProof::from_pinned_directory_handles(&self.name_semantics)?;
        let pinned = PinnedBrokerRoot::from_caller_pinned_handle(
            &declared.root.volume_id,
            &declared.root.root_identity,
            &declared.root.canonical_root_utf16,
            semantics,
        )
        .map_err(|_| AuthorizerError::RootIdentityMismatch)?;
        AuthorizedPinnedRoot::after_identity_check(caller, &declared, pinned)
    }
}

#[derive(Clone)]
struct EndpointTranslationAuthorizer {
    inner: FakeAuthorizer,
    failing_root_id: &'static str,
}

impl CallerAuthorizer for EndpointTranslationAuthorizer {
    fn verify_claim(
        &self,
        authenticated: &AuthenticatedConnection,
        claim: &CallerClaim,
    ) -> Result<CallerAuthorization, AuthorizerError> {
        self.inner.verify_claim(authenticated, claim)
    }

    fn register_root(
        &self,
        caller: &CallerAuthorization,
        root: &RootAuthorization,
        client_root_handle: u64,
        context: &mut RequestContext<'_>,
    ) -> Result<RootCapability, AuthorizerError> {
        self.inner
            .register_root(caller, root, client_root_handle, context)
    }

    fn authorize_registered_root(
        &self,
        caller: &CallerAuthorization,
        root: &RootAuthorization,
        capability: RootCapability,
        context: &mut RequestContext<'_>,
    ) -> Result<AuthorizedPinnedRoot, AuthorizerError> {
        self.inner
            .authorize_registered_root(caller, root, capability, context)
    }

    fn filter_disclosable_candidates(
        &self,
        root: &AuthorizedPinnedRoot,
        candidates: Vec<BackendJournalCandidate>,
        context: &mut RequestContext<'_>,
    ) -> Result<Vec<BackendJournalCandidate>, AuthorizerError> {
        if !candidates.is_empty() && root.declared_root.root_id == self.failing_root_id {
            return Err(AuthorizerError::RootIdentityMismatch);
        }
        self.inner
            .filter_disclosable_candidates(root, candidates, context)
    }
}

#[derive(Clone, Copy)]
enum ReadMode {
    Complete,
    Partial,
    Reset,
    Trimmed,
    BackendUnavailable,
    BlockingUntilCancelled,
    BlockingQueryUntilCancelled,
}

#[derive(Clone)]
struct FakeBackend {
    pin_calls: Arc<AtomicUsize>,
    state: ExistingJournalState,
    mode: ReadMode,
    candidates: Vec<BackendJournalCandidate>,
    read_started: Arc<AtomicUsize>,
}

impl ExistingJournalBackend for FakeBackend {
    fn query_existing_journal(
        &self,
        root: &AuthorizedPinnedRoot,
        context: &mut RequestContext<'_>,
    ) -> Result<ExistingJournalState, BackendError> {
        let _ = context.bounded_step()?;
        self.pin_calls.fetch_add(1, Ordering::SeqCst);
        if root.caller.opaque_namespace == [0; 32] || root.declared_root.root_id != "root-a" {
            return Err(BackendError::RootUnavailable);
        }
        if matches!(self.mode, ReadMode::BackendUnavailable) {
            return Err(BackendError::Unavailable);
        }
        if matches!(self.mode, ReadMode::BlockingQueryUntilCancelled) {
            self.read_started.store(1, Ordering::Release);
            loop {
                context.bounded_step()?;
                std::thread::yield_now();
            }
        }
        Ok(self.state)
    }

    fn read_existing_journal_page(
        &self,
        _root: &AuthorizedPinnedRoot,
        journal: ExistingJournalMetadata,
        range: JournalRange,
        page: &mut BoundedJournalPageBuilder,
        context: &mut RequestContext<'_>,
    ) -> Result<JournalReadProof, BackendError> {
        self.read_started.store(1, Ordering::Release);
        if matches!(self.mode, ReadMode::BlockingUntilCancelled) {
            loop {
                context.bounded_step()?;
                std::thread::yield_now();
            }
        }
        for candidate in &self.candidates {
            let _ = context.bounded_step()?;
            page.push(candidate.clone())?;
        }
        let (covered_until_usn, is_complete, after) = match self.mode {
            ReadMode::Complete => (range.end_usn, true, journal),
            ReadMode::Partial => (range.start_usn + 5, false, journal),
            ReadMode::Reset => (
                range.end_usn,
                true,
                ExistingJournalMetadata {
                    journal_id: journal.journal_id + 1,
                    ..journal
                },
            ),
            ReadMode::Trimmed => (
                range.end_usn,
                true,
                ExistingJournalMetadata {
                    first_usn: range.start_usn + 1,
                    ..journal
                },
            ),
            ReadMode::BackendUnavailable => return Err(BackendError::Unavailable),
            ReadMode::BlockingUntilCancelled => unreachable!("blocking mode exits by cancellation"),
            ReadMode::BlockingQueryUntilCancelled => {
                unreachable!("query-blocking mode never reaches page reads")
            }
        };
        JournalReadProof::after_read(covered_until_usn, is_complete, after)
    }
}

struct ComparisonDowngradeBackend {
    inner: FakeBackend,
    attempted_profile: NameComparisonSemantics,
}

impl ExistingJournalBackend for ComparisonDowngradeBackend {
    fn query_existing_journal(
        &self,
        root: &AuthorizedPinnedRoot,
        context: &mut RequestContext<'_>,
    ) -> Result<ExistingJournalState, BackendError> {
        let _untrusted_attempt = self.attempted_profile;
        self.inner.query_existing_journal(root, context)
    }

    fn read_existing_journal_page(
        &self,
        root: &AuthorizedPinnedRoot,
        journal: ExistingJournalMetadata,
        range: JournalRange,
        page: &mut BoundedJournalPageBuilder,
        context: &mut RequestContext<'_>,
    ) -> Result<JournalReadProof, BackendError> {
        self.inner
            .read_existing_journal_page(root, journal, range, page, context)
    }
}

#[test]
fn query_journal_supports_initial_baseline_and_live_only() {
    let supported = service(
        ExistingJournalState::Supported(metadata()),
        ReadMode::Complete,
        vec![],
        true,
    );
    let response = round_trip(
        &supported,
        BrokerRequest::QueryJournal {
            request_id: 41,
            request: query_request(),
        },
        &connection(1),
    );
    assert!(matches!(
        response,
        BrokerResponse::Journal {
            journal_id: Some(7),
            first_usn: Some(0),
            next_usn: Some(20),
            ..
        }
    ));

    let live_only = service(
        ExistingJournalState::LiveOnly,
        ReadMode::Complete,
        vec![],
        true,
    );
    let response = round_trip(
        &live_only,
        BrokerRequest::QueryJournal {
            request_id: 42,
            request: query_request(),
        },
        &connection(1),
    );
    assert!(matches!(
        response,
        BrokerResponse::Journal {
            capability: JournalCapability::LiveOnly,
            journal_id: None,
            ..
        }
    ));
}

#[test]
fn unsupported_filesystem_maps_to_structured_journal_unavailable() {
    assert_eq!(
        map_authorizer_failure(AuthorizerError::UnsupportedFilesystem).code,
        BrokerFailureCode::JournalUnavailable
    );
}

#[test]
fn query_finish_selects_cancel_or_timeout_as_one_terminal_across_races() {
    let read_started = Arc::new(AtomicUsize::new(0));
    let service = Arc::new(BrokerService::new(
        FakeBackend {
            pin_calls: Arc::new(AtomicUsize::new(0)),
            state: ExistingJournalState::Supported(metadata()),
            mode: ReadMode::BlockingQueryUntilCancelled,
            candidates: vec![],
            read_started: Arc::clone(&read_started),
        },
        FakeAuthorizer {
            allow_root: true,
            name_semantics: vec![NameComparisonSemantics::OrdinalCaseSensitive],
        },
    ));
    let authenticated = connection(14);
    service
        .open_connection(&authenticated)
        .expect("open query connection");
    for iteration in 0..100_u64 {
        read_started.store(0, Ordering::Release);
        let request_id = 10_000 + iteration;
        let mut request = query_request();
        request.timeout_ms = if iteration % 2 == 0 { 1_000 } else { 1 };
        let payload = encode_request(&BrokerRequest::QueryJournal {
            request_id,
            request,
        })
        .expect("encode query");
        let frame = encode_frame(&payload).expect("frame query");
        if iteration % 2 == 0 {
            let query_service = Arc::clone(&service);
            let query_connection = authenticated.clone();
            let query =
                std::thread::spawn(move || query_service.handle_frame(&frame, &query_connection));
            wait_until_started(&read_started);
            let cancel_payload = encode_request(&BrokerRequest::Cancel {
                request_id: 20_000 + iteration,
                caller: caller(),
                target_request_id: request_id,
            })
            .expect("encode query cancel");
            let cancel_frame = encode_frame(&cancel_payload).expect("frame query cancel");
            let cancel_response = service
                .handle_frame(&cancel_frame, &authenticated)
                .expect("cancel response");
            assert!(matches!(
                decode_response(decode_frame(&cancel_response).expect("cancel frame"))
                    .expect("cancel payload"),
                BrokerResponse::Cancelled { .. }
            ));
            let query_response = query.join().expect("query thread").expect("query response");
            assert!(matches!(
                decode_response(decode_frame(&query_response).expect("query frame"))
                    .expect("query payload"),
                BrokerResponse::Failure {
                    failure: BrokerFailure {
                        code: BrokerFailureCode::Cancelled
                    },
                    ..
                }
            ));
        } else {
            let query_response = service
                .handle_frame(&frame, &authenticated)
                .expect("timed query response");
            assert!(matches!(
                decode_response(decode_frame(&query_response).expect("query frame"))
                    .expect("query payload"),
                BrokerResponse::Failure {
                    failure: BrokerFailure {
                        code: BrokerFailureCode::TimedOut
                    },
                    ..
                }
            ));
        }
        assert_eq!(service.active.active_count([14; 16]), 0);
    }
}

#[test]
fn empty_complete_and_partial_pages_report_only_proven_coverage() {
    let complete = service(
        ExistingJournalState::Supported(metadata()),
        ReadMode::Complete,
        vec![],
        true,
    );
    let response = round_trip(
        &complete,
        BrokerRequest::ReadRange {
            request_id: 50,
            request: read_request(),
        },
        &connection(1),
    );
    assert!(matches!(
        response,
        BrokerResponse::ReadRange {
            covered_until_usn: 20,
            is_complete: true,
            ref candidates,
            ..
        } if candidates.is_empty()
    ));

    let partial = service(
        ExistingJournalState::Supported(metadata()),
        ReadMode::Partial,
        vec![],
        true,
    );
    let response = round_trip(
        &partial,
        BrokerRequest::ReadRange {
            request_id: 51,
            request: read_request(),
        },
        &connection(1),
    );
    assert!(matches!(
        response,
        BrokerResponse::ReadRange {
            covered_until_usn: 15,
            is_complete: false,
            ..
        }
    ));
}

#[test]
fn reset_and_trimming_during_read_fail_closed() {
    for (request_id, mode) in [(61, ReadMode::Reset), (62, ReadMode::Trimmed)] {
        let service = service(
            ExistingJournalState::Supported(metadata()),
            mode,
            vec![],
            true,
        );
        let response = round_trip(
            &service,
            BrokerRequest::ReadRange {
                request_id,
                request: read_request(),
            },
            &connection(1),
        );
        assert!(matches!(
            response,
            BrokerResponse::Failure {
                failure: BrokerFailure {
                    code: BrokerFailureCode::JournalDiscontinuous
                },
                ..
            }
        ));
    }
}

#[test]
fn unauthorized_root_never_reaches_privileged_backend() {
    let pin_calls = Arc::new(AtomicUsize::new(0));
    let service = BrokerService::new(
        FakeBackend {
            pin_calls: Arc::clone(&pin_calls),
            state: ExistingJournalState::Supported(metadata()),
            mode: ReadMode::Complete,
            candidates: vec![],
            read_started: Arc::new(AtomicUsize::new(0)),
        },
        FakeAuthorizer {
            allow_root: false,
            name_semantics: vec![NameComparisonSemantics::OrdinalCaseSensitive],
        },
    );
    let response = round_trip(
        &service,
        BrokerRequest::QueryJournal {
            request_id: 70,
            request: query_request(),
        },
        &connection(1),
    );
    assert!(matches!(
        response,
        BrokerResponse::Failure {
            failure: BrokerFailure {
                code: BrokerFailureCode::RootUnauthorized
            },
            ..
        }
    ));
    assert_eq!(pin_calls.load(Ordering::SeqCst), 0);
}

#[test]
fn rename_edges_do_not_leak_external_paths_and_root_self_is_allowed() {
    let candidates = vec![
        candidate(
            "C:\\Pictures\\New.jpg",
            Some("D:\\Secret\\Old.jpg"),
            11,
            false,
        ),
        candidate(
            "D:\\Secret\\New.jpg",
            Some("C:\\Pictures\\Old.jpg"),
            12,
            false,
        ),
        candidate(
            "C:\\Pictures\\Renamed.jpg",
            Some("C:\\Pictures\\Before.jpg"),
            13,
            false,
        ),
        candidate("C:\\Pictures", None, 14, true),
    ];
    let service = service(
        ExistingJournalState::Supported(metadata()),
        ReadMode::Complete,
        candidates,
        true,
    );
    let response = round_trip(
        &service,
        BrokerRequest::ReadRange {
            request_id: 80,
            request: read_request(),
        },
        &connection(1),
    );
    let BrokerResponse::ReadRange { candidates, .. } = response else {
        panic!("expected read response");
    };
    assert_eq!(candidates.len(), 4);
    assert_eq!(
        candidates[0].scope,
        CandidateScope::RelativePath("New.jpg".to_owned())
    );
    assert_eq!(candidates[0].previous_scope, None);
    assert_eq!(candidates[0].kind, CandidateKind::Path);
    assert_eq!(
        candidates[1].scope,
        CandidateScope::RelativePath("Old.jpg".to_owned())
    );
    assert_eq!(candidates[1].previous_scope, None);
    assert_eq!(candidates[1].kind, CandidateKind::Path);
    assert_eq!(
        candidates[2].scope,
        CandidateScope::RelativePath("Renamed.jpg".to_owned())
    );
    assert_eq!(
        candidates[2].previous_scope,
        Some(CandidateScope::RelativePath("Before.jpg".to_owned()))
    );
    assert_eq!(candidates[2].kind, CandidateKind::Rename);
    assert_eq!(candidates[3].scope, CandidateScope::Root);
    assert_eq!(candidates[3].kind, CandidateKind::Subtree);
}

#[test]
fn caller_pinned_mixed_name_semantics_cannot_be_downgraded_by_backend() {
    let mut authorized_root = root();
    authorized_root.canonical_root_utf16 = "C:\\Parent\\Photos".encode_utf16().collect();
    let outside_case_sensitive_sibling = BackendJournalCandidate::copy_from_bounded(
        &authorized_root.volume_id,
        &"c:\\PARENT\\photos\\outside.jpg"
            .encode_utf16()
            .collect::<Vec<_>>(),
        None,
        &[31; 8],
        11,
        CandidateKind::Path,
        false,
    )
    .expect("case-sensitive sibling candidate");
    let inside_mixed_profile = BackendJournalCandidate::copy_from_bounded(
        &authorized_root.volume_id,
        &"c:\\PARENT\\Photos\\inside.jpg"
            .encode_utf16()
            .collect::<Vec<_>>(),
        None,
        &[32; 8],
        12,
        CandidateKind::Path,
        false,
    )
    .expect("mixed-profile candidate");
    let backend = ComparisonDowngradeBackend {
        inner: FakeBackend {
            pin_calls: Arc::new(AtomicUsize::new(0)),
            state: ExistingJournalState::Supported(metadata()),
            mode: ReadMode::Complete,
            candidates: vec![outside_case_sensitive_sibling, inside_mixed_profile],
            read_started: Arc::new(AtomicUsize::new(0)),
        },
        attempted_profile: NameComparisonSemantics::WindowsOrdinalIgnoreCase,
    };
    let service = BrokerService::new(
        backend,
        FakeAuthorizer {
            allow_root: true,
            name_semantics: vec![
                NameComparisonSemantics::WindowsOrdinalIgnoreCase,
                NameComparisonSemantics::OrdinalCaseSensitive,
            ],
        },
    );
    let mut request = read_request();
    request.root = authorized_root;
    let response = round_trip(
        &service,
        BrokerRequest::ReadRange {
            request_id: 330,
            request,
        },
        &connection(1),
    );
    assert!(matches!(
        response,
        BrokerResponse::ReadRange { candidates, .. }
            if candidates.len() == 1
                && candidates[0].scope
                    == CandidateScope::RelativePath("inside.jpg".to_owned())
    ));
}

#[test]
fn bounded_builder_rejects_duplicate_order_and_evidence_before_growth() {
    let range = JournalRange {
        start_usn: 10,
        end_usn: 20,
    };
    let mut builder = BoundedJournalPageBuilder::new(
        range,
        JournalReadLimits {
            max_records: 1,
            max_evidence_bytes: 128,
        },
    );
    builder
        .push(candidate("C:\\Pictures\\A.jpg", None, 11, false))
        .expect("first record should fit");
    assert_eq!(
        builder.push(candidate("C:\\Pictures\\B.jpg", None, 11, false)),
        Err(BackendError::EvidenceLimitExceeded)
    );

    let oversized = vec![b'a' as u16; crate::journal_broker::wire::MAX_PATH_UTF16_UNITS + 1];
    assert!(matches!(
        BackendJournalCandidate::copy_from_bounded(
            "volume-a",
            &oversized,
            None,
            &[1; 8],
            11,
            CandidateKind::Path,
            false,
        ),
        Err(BackendError::RecordUnsupported)
    ));

    let path: Vec<u16> = "C:\\Pictures\\A.jpg".encode_utf16().collect();
    let previous: Vec<u16> = "C:\\Pictures\\Old.jpg".encode_utf16().collect();
    for invalid in [
        BackendJournalCandidate::copy_from_bounded(
            "volume-a",
            &path,
            Some(&previous),
            &[1; 8],
            11,
            CandidateKind::Path,
            false,
        ),
        BackendJournalCandidate::copy_from_bounded(
            "volume-a",
            &path,
            None,
            &[1; 8],
            11,
            CandidateKind::Rename,
            false,
        ),
        BackendJournalCandidate::copy_from_bounded(
            "volume-a",
            &path,
            None,
            &[1; 8],
            11,
            CandidateKind::Subtree,
            false,
        ),
    ] {
        assert!(matches!(invalid, Err(BackendError::RecordUnsupported)));
    }

    let mut ordering = BoundedJournalPageBuilder::new(
        range,
        JournalReadLimits {
            max_records: 3,
            max_evidence_bytes: 1_024,
        },
    );
    ordering
        .push(candidate("C:\\Pictures\\A.jpg", None, 12, false))
        .expect("first ordered record");
    assert_eq!(
        ordering.push(candidate("C:\\Pictures\\B.jpg", None, 12, false)),
        Err(BackendError::JournalDiscontinuous)
    );
    assert_eq!(
        ordering.push(candidate("C:\\Pictures\\C.jpg", None, 11, false)),
        Err(BackendError::JournalDiscontinuous)
    );
}

#[test]
fn cancellation_is_connection_scoped_non_sticky_and_bounded() {
    let registry = ActiveRequestRegistry::default();
    let key = active_key(1, 1, 99);
    let deadline = Instant::now() + Duration::from_secs(1);
    assert_eq!(
        registry.cancel(key),
        Err(ActiveRequestError::ConnectionNotOpen)
    );
    registry
        .open_connection([1; 16], 1)
        .expect("open connection");
    registry
        .begin_query(key, deadline)
        .expect("request should register");
    assert_eq!(
        registry.cancel(active_key(2, 1, 99)),
        Err(ActiveRequestError::ConnectionNotOpen)
    );
    assert_eq!(registry.cancel(key), Ok(()));
    assert_eq!(
        registry.check(key, Instant::now()),
        Err(ActiveRequestError::Cancelled)
    );
    assert_eq!(
        registry.finish(key, Instant::now()),
        Ok(RequestTerminal::Cancelled)
    );
    assert_eq!(registry.cancel(key), Err(ActiveRequestError::NotActive));

    registry
        .open_connection([3; 16], 1)
        .expect("open connection");
    for offset in 0..MAX_ACTIVE_REQUESTS_PER_CONNECTION {
        registry
            .begin_query(active_key(3, 1, offset as u64 + 1), deadline)
            .expect("within connection limit");
    }
    assert!(matches!(
        registry.begin_query(active_key(3, 1, 100), deadline),
        Err(ActiveRequestError::LimitExceeded)
    ));
    registry.disconnect([3; 16], 1);
    assert_eq!(registry.active_count([3; 16]), 0);
}

#[test]
fn business_capacity_reserves_bounded_control_capacity() {
    let registry = ActiveRequestRegistry::default();
    registry
        .open_connection([6; 16], 1)
        .expect("open connection");
    let deadline = Instant::now() + Duration::from_secs(1);
    for offset in 0..MAX_ACTIVE_REQUESTS_PER_CONNECTION {
        registry
            .begin_query(active_key(6, 1, offset as u64 + 1), deadline)
            .expect("fill business capacity");
    }
    let first = registry
        .begin_control(active_key(6, 1, 100))
        .expect("first control slot");
    let second = registry
        .begin_control(active_key(6, 1, 101))
        .expect("second control slot");
    assert_eq!(registry.control_count([6; 16]), 2);
    assert!(matches!(
        registry.begin_control(active_key(6, 1, 102)),
        Err(ActiveRequestError::LimitExceeded)
    ));
    drop(first);
    let replacement = registry
        .begin_control(active_key(6, 1, 103))
        .expect("released control slot is reusable");
    assert_eq!(registry.active_count([6; 16]), 8);
    assert_eq!(registry.control_count([6; 16]), 2);
    drop((second, replacement));
    registry.disconnect([6; 16], 1);
}

#[test]
fn expired_cancelled_and_dispatch_failed_accepts_are_reaped_with_one_terminal() {
    let registry = ActiveRequestRegistry::default();
    registry
        .open_connection([7; 16], 1)
        .expect("open connection");
    let deadline = Instant::now() + Duration::from_millis(10);
    for offset in 0..MAX_ACTIVE_REQUESTS_PER_CONNECTION {
        registry
            .accept_read(
                active_key(7, 1, offset as u64 + 1),
                deadline,
                read_request(),
            )
            .expect("fill expiring accepts");
    }
    while Instant::now() < deadline {
        std::thread::yield_now();
    }
    registry
        .accept_read(
            active_key(7, 1, 100),
            Instant::now() + Duration::from_secs(1),
            read_request(),
        )
        .expect("registration reaps eight expired accepts");
    assert_eq!(registry.active_count([7; 16]), 1);
    registry.disconnect([7; 16], 1);

    registry
        .open_connection([8; 16], 1)
        .expect("open connection");
    for offset in 0..MAX_ACTIVE_REQUESTS_PER_CONNECTION {
        let key = active_key(8, 1, offset as u64 + 1);
        registry
            .accept_read(key, Instant::now() + Duration::from_secs(1), read_request())
            .expect("fill cancellable accepts");
        registry.cancel(key).expect("cancel accepted request");
    }
    let terminals = registry
        .reap_accepted([8; 16], 1, Instant::now())
        .expect("reap cancelled accepts");
    assert_eq!(
        terminals.terminals.len(),
        MAX_ACTIVE_REQUESTS_PER_CONNECTION
    );
    assert!(!terminals.must_close);
    assert!(
        terminals
            .terminals
            .iter()
            .all(|terminal| terminal.terminal == RequestTerminal::Cancelled)
    );
    let unique: HashSet<_> = terminals
        .terminals
        .iter()
        .map(|terminal| terminal.key.request_id)
        .collect();
    assert_eq!(unique.len(), MAX_ACTIVE_REQUESTS_PER_CONNECTION);
    assert_eq!(registry.active_count([8; 16]), 0);
}

#[test]
fn host_dispatch_failure_aborts_one_accepted_request_exactly_once() {
    let service = service(
        ExistingJournalState::Supported(metadata()),
        ReadMode::Complete,
        vec![],
        true,
    );
    let authenticated = connection(9);
    let request_id = 901;
    let response = round_trip_accept_only(
        &service,
        BrokerRequest::ReadRange {
            request_id,
            request: read_request(),
        },
        &authenticated,
    );
    assert!(matches!(response, BrokerResponse::ReadRangeAccepted { .. }));
    let terminal = service
        .abort_accepted_read(request_id, caller().client_instance, &authenticated)
        .expect("abort enqueue failure");
    assert_eq!(terminal.request_id, request_id);
    assert!(matches!(
        decode_response(decode_frame(&terminal.frame).expect("terminal frame"))
            .expect("terminal response"),
        BrokerResponse::Failure {
            request_id: returned,
            client_instance: Some(instance),
            failure,
        } if returned == request_id
            && instance == caller().client_instance
            && failure.code == BrokerFailureCode::RequestNotActive
    ));
    assert!(
        service
            .abort_accepted_read(request_id, caller().client_instance, &authenticated)
            .is_err()
    );
    assert!(
        service
            .reap_accepted_reads(&authenticated)
            .expect("reap after abort")
            .frames
            .is_empty()
    );
    assert_eq!(service.active.active_count([9; 16]), 0);
}

#[test]
fn abort_accepted_uses_cancel_then_deadline_then_abort_fact_priority() {
    let registry = ActiveRequestRegistry::default();
    registry
        .open_connection([10; 16], 1)
        .expect("open connection");

    let cancelled = active_key(10, 1, 1);
    registry
        .accept_read(cancelled, Instant::now(), read_request())
        .expect("accept cancellable read");
    registry.cancel(cancelled).expect("cancel accepted read");
    assert_eq!(
        registry
            .abort_accepted(cancelled, Instant::now())
            .expect("abort cancelled read")
            .terminal,
        RequestTerminal::Cancelled
    );

    let timed_out = active_key(10, 1, 2);
    let expired = Instant::now();
    registry
        .accept_read(timed_out, expired, read_request())
        .expect("accept expired read");
    assert_eq!(
        registry
            .abort_accepted(timed_out, expired)
            .expect("abort expired read")
            .terminal,
        RequestTerminal::TimedOut
    );

    let aborted = active_key(10, 1, 3);
    registry
        .accept_read(
            aborted,
            Instant::now() + Duration::from_secs(1),
            read_request(),
        )
        .expect("accept dispatch-failed read");
    assert_eq!(
        registry
            .abort_accepted(aborted, Instant::now())
            .expect("abort dispatch-failed read")
            .terminal,
        RequestTerminal::Aborted
    );
}

#[test]
fn terminal_outbox_applies_backpressure_without_overwriting_entries() {
    let registry = ActiveRequestRegistry::default();
    registry
        .open_connection([11; 16], 1)
        .expect("open connection");
    for wave in 0..2_u64 {
        for offset in 0..MAX_ACTIVE_REQUESTS_PER_CONNECTION as u64 {
            let key = active_key(11, 1, wave * 8 + offset + 1);
            registry
                .accept_read(key, Instant::now() + Duration::from_secs(1), read_request())
                .expect("accept terminal-producing read");
            registry.cancel(key).expect("cancel accepted read");
        }
    }
    let drain = registry
        .reap_accepted([11; 16], 1, Instant::now())
        .expect("drain full terminal outbox");
    assert!(drain.must_close);
    assert_eq!(drain.terminals.len(), 16);
    let unique: HashSet<_> = drain
        .terminals
        .iter()
        .map(|terminal| terminal.key.request_id)
        .collect();
    assert_eq!(unique.len(), 16);
    assert_eq!(
        registry.accept_read(
            active_key(11, 1, 99),
            Instant::now() + Duration::from_secs(1),
            read_request(),
        ),
        Err(ActiveRequestError::TerminalDeliveryBackpressure)
    );
}

#[test]
fn service_delivers_twenty_four_bound_terminal_frames_without_overwrite() {
    let service = service(
        ExistingJournalState::Supported(metadata()),
        ReadMode::Complete,
        vec![],
        true,
    );
    let mut delivered = HashSet::new();
    for connection_id in 12..15_u8 {
        let authenticated = connection(connection_id);
        service
            .open_connection(&authenticated)
            .expect("open connection");
        for offset in 0..MAX_ACTIVE_REQUESTS_PER_CONNECTION as u64 {
            let request_id = u64::from(connection_id) * 100 + offset;
            let response = round_trip_accept_only(
                &service,
                BrokerRequest::ReadRange {
                    request_id,
                    request: read_request(),
                },
                &authenticated,
            );
            assert!(matches!(response, BrokerResponse::ReadRangeAccepted { .. }));
            service
                .active
                .cancel(active_key(connection_id, 9, request_id))
                .expect("cancel accepted read");
        }
        let batch = service
            .reap_accepted_reads(&authenticated)
            .expect("encode terminal batch");
        assert!(!batch.must_close);
        assert_eq!(batch.frames.len(), MAX_ACTIVE_REQUESTS_PER_CONNECTION);
        for terminal in batch.frames {
            assert!(delivered.insert(terminal.request_id));
            assert!(matches!(
                decode_response(decode_frame(&terminal.frame).expect("terminal frame"))
                    .expect("terminal response"),
                BrokerResponse::Failure {
                    request_id,
                    client_instance: Some(instance),
                    failure,
                } if request_id == terminal.request_id
                    && instance == caller().client_instance
                    && failure.code == BrokerFailureCode::Cancelled
            ));
        }
    }
    assert_eq!(delivered.len(), 24);
}

#[test]
fn connection_generations_reject_late_work_and_do_not_leave_tombstones() {
    let registry = ActiveRequestRegistry::default();
    for generation in 1..=1_000_u64 {
        let mut connection_id = [0_u8; 16];
        connection_id[..8].copy_from_slice(&generation.to_le_bytes());
        registry
            .open_connection(connection_id, generation)
            .expect("open unique connection");
        registry.disconnect(connection_id, generation);
        assert_eq!(registry.connection_count(), 0);
    }

    let connection_id = [11; 16];
    registry
        .open_connection(connection_id, 1)
        .expect("open generation one");
    registry.disconnect(connection_id, 1);
    registry
        .open_connection(connection_id, 2)
        .expect("reopen generation two");
    let mut old = active_key(11, 1, 1);
    old.connection_generation = 1;
    assert_eq!(
        registry.begin_query(old, Instant::now() + Duration::from_secs(1)),
        Err(ActiveRequestError::ConnectionGenerationMismatch)
    );
    registry.disconnect(connection_id, 1);
    let mut current = active_key(11, 1, 2);
    current.connection_generation = 2;
    registry
        .begin_query(current, Instant::now() + Duration::from_secs(1))
        .expect("current generation remains open");
    registry.disconnect(connection_id, 2);
    assert_eq!(registry.connection_count(), 0);
}

#[test]
fn request_context_exposes_only_cancellable_bounded_steps() {
    let registry = ActiveRequestRegistry::default();
    let key = active_key(4, 2, 400);
    let deadline = Instant::now() + Duration::from_secs(1);
    registry
        .open_connection([4; 16], 1)
        .expect("open connection");
    registry
        .begin_query(key, deadline)
        .expect("register request");
    let mut context = RequestContext {
        deadline,
        registry: &registry,
        key,
        bounded_steps: 0,
    };
    assert!(context.bounded_step().expect("bounded step") <= std::time::Duration::from_millis(250));
    registry.cancel(key).expect("cancel active request");
    assert_eq!(context.check(), Err(BackendError::Cancelled));
}

#[test]
fn accept_and_disconnect_share_one_atomic_connection_lifecycle() {
    for iteration in 0..100_u64 {
        let registry = Arc::new(ActiveRequestRegistry::default());
        let connection_id = u8::try_from(iteration % 200 + 20).expect("connection byte");
        registry
            .open_connection([connection_id; 16], 1)
            .expect("open connection");
        let key = active_key(connection_id, 2, iteration + 1);
        let barrier = Arc::new(Barrier::new(3));
        let accepting_registry = Arc::clone(&registry);
        let accepting_barrier = Arc::clone(&barrier);
        let request = read_request();
        let accept = std::thread::spawn(move || {
            accepting_barrier.wait();
            accepting_registry.accept_read(key, Instant::now() + Duration::from_secs(1), request)
        });
        let closing_registry = Arc::clone(&registry);
        let closing_barrier = Arc::clone(&barrier);
        let close = std::thread::spawn(move || {
            closing_barrier.wait();
            closing_registry.disconnect([connection_id; 16], 1);
        });
        barrier.wait();
        let result = accept.join().expect("accept thread");
        close.join().expect("disconnect thread");
        assert!(result.is_ok() || result == Err(ActiveRequestError::ConnectionNotOpen));
        assert_eq!(registry.active_count([connection_id; 16]), 0);
        assert_eq!(registry.accepted_count([connection_id; 16]), 0);
    }
}

#[test]
fn delayed_execute_uses_deadline_captured_at_accept_and_skips_backend() {
    let backend_calls = Arc::new(AtomicUsize::new(0));
    let read_started = Arc::new(AtomicUsize::new(0));
    let service = BrokerService::new(
        FakeBackend {
            pin_calls: Arc::clone(&backend_calls),
            state: ExistingJournalState::Supported(metadata()),
            mode: ReadMode::Complete,
            candidates: vec![],
            read_started: Arc::clone(&read_started),
        },
        FakeAuthorizer {
            allow_root: true,
            name_semantics: vec![NameComparisonSemantics::OrdinalCaseSensitive],
        },
    );
    let mut request = read_request();
    request.timeout_ms = 1;
    let request_id = 451;
    let response = round_trip_accept_only(
        &service,
        BrokerRequest::ReadRange {
            request_id,
            request: request.clone(),
        },
        &connection(1),
    );
    assert!(matches!(response, BrokerResponse::ReadRangeAccepted { .. }));
    let wait_started = Instant::now();
    while wait_started.elapsed() < Duration::from_millis(3) {
        std::thread::yield_now();
    }
    let frame = service
        .execute_accepted_read_frame(request_id, request.caller.client_instance, &connection(1))
        .expect("timed-out response frame");
    let response = decode_response(decode_frame(&frame).expect("decode frame"))
        .expect("decode timed-out response");
    assert!(matches!(
        response,
        BrokerResponse::Failure {
            failure: BrokerFailure {
                code: BrokerFailureCode::TimedOut
            },
            ..
        }
    ));
    assert_eq!(backend_calls.load(Ordering::Acquire), 0);
    assert_eq!(read_started.load(Ordering::Acquire), 0);
    assert_eq!(service.active.active_count([1; 16]), 0);
}

#[test]
fn client_cancels_an_inflight_read_on_the_same_authenticated_connection() {
    let read_started = Arc::new(AtomicUsize::new(0));
    let service = Arc::new(BrokerService::new(
        FakeBackend {
            pin_calls: Arc::new(AtomicUsize::new(0)),
            state: ExistingJournalState::Supported(metadata()),
            mode: ReadMode::BlockingUntilCancelled,
            candidates: vec![],
            read_started: Arc::clone(&read_started),
        },
        FakeAuthorizer {
            allow_root: true,
            name_semantics: vec![NameComparisonSemantics::OrdinalCaseSensitive],
        },
    ));
    let transport = InProcessTransport::new(Arc::clone(&service), connection(1));
    let client = broker_client(transport.clone());
    let pending = client
        .begin_read_range(read_request())
        .expect("begin read range");
    let handle = pending.cancel_handle();
    let waiter = std::thread::spawn(move || pending.wait());
    wait_until_started(&read_started);
    client
        .cancel(&handle, caller())
        .expect("same connection cancel");
    assert!(matches!(
        waiter.join().expect("waiter join"),
        Err(BrokerClientError::Cancelled)
    ));

    read_started.store(0, Ordering::Release);
    let pending = client
        .begin_read_range(read_request())
        .expect("begin second read");
    let handle = pending.cancel_handle();
    let waiter = std::thread::spawn(move || pending.wait());
    wait_until_started(&read_started);
    let foreign = broker_client(InProcessTransport::new(service, connection(2)));
    assert!(matches!(
        foreign.cancel(&handle, caller()),
        Err(BrokerClientError::ForeignPendingHandle)
    ));
    client
        .cancel(&handle, caller())
        .expect("owner cleanup cancel");
    assert!(matches!(
        waiter.join().expect("second waiter join"),
        Err(BrokerClientError::Cancelled)
    ));
}

#[test]
fn accepted_ack_prevents_cancel_from_racing_delayed_read_dispatch() {
    let read_started = Arc::new(AtomicUsize::new(0));
    let service = Arc::new(BrokerService::new(
        FakeBackend {
            pin_calls: Arc::new(AtomicUsize::new(0)),
            state: ExistingJournalState::Supported(metadata()),
            mode: ReadMode::BlockingUntilCancelled,
            candidates: vec![],
            read_started: Arc::clone(&read_started),
        },
        FakeAuthorizer {
            allow_root: true,
            name_semantics: vec![NameComparisonSemantics::OrdinalCaseSensitive],
        },
    ));
    let transport = InProcessTransport::new(Arc::clone(&service), connection(1));
    transport.delay_read_execution();
    let client = broker_client(transport.clone());
    let pending = client
        .begin_read_range(read_request())
        .expect("accepted read range");
    assert_eq!(read_started.load(Ordering::Acquire), 0);
    client
        .cancel(&pending.cancel_handle(), caller())
        .expect("cancel accepted request");
    transport.release_read_execution();
    assert!(matches!(pending.wait(), Err(BrokerClientError::Cancelled)));
    assert_eq!(client.pending_count(), 0);
    assert_eq!(service.active.active_count([1; 16]), 0);
    assert_eq!(transport.response_count(), 0);
    assert_eq!(transport.expected_count(), 0);
}

#[test]
fn pending_registry_is_bounded_and_disconnect_cleans_all_demux_state() {
    let service = Arc::new(service(
        ExistingJournalState::Supported(metadata()),
        ReadMode::Complete,
        vec![],
        true,
    ));
    let transport = InProcessTransport::new(Arc::clone(&service), connection(1));
    transport.delay_read_execution();
    let client = broker_client(transport.clone());
    let mut pending: Vec<_> = (0..MAX_ACTIVE_REQUESTS_PER_CONNECTION)
        .map(|_| {
            client
                .begin_read_range(read_request())
                .expect("pending within limit")
        })
        .collect();
    assert_eq!(client.pending_count(), MAX_ACTIVE_REQUESTS_PER_CONNECTION);
    assert!(matches!(
        client.begin_read_range(read_request()),
        Err(BrokerClientError::PendingLimitExceeded)
    ));
    let first = pending.remove(0);
    client
        .cancel(&first.cancel_handle(), caller())
        .expect("reserved control capacity cancels with eight business requests active");
    assert_eq!(
        client.business_pending_count(),
        MAX_ACTIVE_REQUESTS_PER_CONNECTION
    );
    assert_eq!(client.control_pending_count(), 0);
    transport.release_read_execution();
    assert!(matches!(first.wait(), Err(BrokerClientError::Cancelled)));
    client.close().expect("disconnect client");
    drop(pending);
    assert_eq!(client.pending_count(), 0);
    assert_eq!(service.active.active_count([1; 16]), 0);
    assert_eq!(transport.response_count(), 0);
    assert_eq!(transport.expected_count(), 0);
}

#[test]
fn repeated_begin_and_drop_does_not_accumulate_pending_or_transport_responses() {
    let service = Arc::new(service(
        ExistingJournalState::Supported(metadata()),
        ReadMode::BlockingUntilCancelled,
        vec![],
        true,
    ));
    let transport = InProcessTransport::new(Arc::clone(&service), connection(1));
    let client = broker_client(transport.clone());
    for _ in 0..(MAX_ACTIVE_REQUESTS_PER_CONNECTION * 2) {
        let pending = client
            .begin_read_range(read_request())
            .expect("begin bounded read");
        drop(pending);
        wait_until_clean(&client, &service, &transport);
    }
}

#[test]
fn completed_pending_handle_cannot_send_a_late_cancel() {
    let service = Arc::new(service(
        ExistingJournalState::Supported(metadata()),
        ReadMode::Complete,
        vec![],
        true,
    ));
    let transport = InProcessTransport::new(service, connection(1));
    let client = broker_client(transport.clone());
    let pending = client
        .begin_read_range(read_request())
        .expect("begin completed read");
    let handle = pending.cancel_handle();
    assert!(pending.wait().is_ok());
    let sends_before = transport.sent_count();
    assert!(matches!(
        client.cancel(&handle, caller()),
        Err(BrokerClientError::RequestNotActive)
    ));
    assert_eq!(transport.sent_count(), sends_before);
}

#[derive(Clone, Copy)]
enum AckFault {
    Timeout,
    Malformed,
    WrongBinding,
}

#[derive(Clone)]
struct AckFaultTransport {
    inner: InProcessTransport,
    fault: AckFault,
    read_request_id: Arc<AtomicUsize>,
    injected: Arc<AtomicBool>,
}

impl AckFaultTransport {
    fn new(inner: InProcessTransport, fault: AckFault) -> Self {
        Self {
            inner,
            fault,
            read_request_id: Arc::new(AtomicUsize::new(0)),
            injected: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl sealed::Sealed for AckFaultTransport {}

impl BrokerClientTransport for AckFaultTransport {
    type Error = InProcessTransportError;

    fn try_send_frame(&mut self, request_frame: &[u8]) -> Result<AdapterPoll<()>, Self::Error> {
        let payload = decode_frame(request_frame).map_err(|_| InProcessTransportError)?;
        let request = decode_request(payload).map_err(|_| InProcessTransportError)?;
        if let BrokerRequest::ReadRange { request_id, .. } = request {
            let request_id = usize::try_from(request_id).map_err(|_| InProcessTransportError)?;
            self.read_request_id.store(request_id, Ordering::Release);
        }
        self.inner.try_send_frame(request_frame)
    }

    fn poll_response(&mut self, request_id: u64) -> Result<AdapterPoll<Vec<u8>>, Self::Error> {
        let AdapterPoll::Ready(response) = self.inner.poll_response(request_id)? else {
            return Ok(AdapterPoll::Pending);
        };
        if usize::try_from(request_id).ok() != Some(self.read_request_id.load(Ordering::Acquire))
            || self.injected.swap(true, Ordering::AcqRel)
        {
            return Ok(AdapterPoll::Ready(response));
        }
        match self.fault {
            AckFault::Timeout => Err(InProcessTransportError),
            AckFault::Malformed => encode_frame(&[0xff])
                .map(AdapterPoll::Ready)
                .map_err(|_| InProcessTransportError),
            AckFault::WrongBinding => {
                let payload = decode_frame(&response).map_err(|_| InProcessTransportError)?;
                let broker_response =
                    decode_response(payload).map_err(|_| InProcessTransportError)?;
                let BrokerResponse::ReadRangeAccepted {
                    request_id,
                    mut binding,
                    journal_id,
                    requested_start_usn,
                    requested_end_usn,
                    max_records,
                    max_evidence_bytes,
                } = broker_response
                else {
                    return Err(InProcessTransportError);
                };
                binding.root_generation = binding.root_generation.saturating_add(1);
                let payload = encode_response(&BrokerResponse::ReadRangeAccepted {
                    request_id,
                    binding,
                    journal_id,
                    requested_start_usn,
                    requested_end_usn,
                    max_records,
                    max_evidence_bytes,
                })
                .map_err(|_| InProcessTransportError)?;
                encode_frame(&payload)
                    .map(AdapterPoll::Ready)
                    .map_err(|_| InProcessTransportError)
            }
        }
    }

    fn try_discard(&mut self, request_id: u64) -> Result<AdapterPoll<()>, Self::Error> {
        self.inner.try_discard(request_id)
    }

    fn begin_abandon(
        &mut self,
        target_request_id: u64,
        cancel_request: Option<(u64, Vec<u8>)>,
        close_connection: bool,
    ) -> Result<AdapterPoll<()>, Self::Error> {
        self.inner
            .begin_abandon(target_request_id, cancel_request, close_connection)
    }

    fn poll_abandon(&mut self, target_request_id: u64) -> Result<AdapterPoll<()>, Self::Error> {
        self.inner.poll_abandon(target_request_id)
    }

    fn begin_close(&mut self) -> Result<AdapterPoll<()>, Self::Error> {
        self.inner.begin_close()
    }

    fn poll_close(&mut self) -> Result<AdapterPoll<()>, Self::Error> {
        self.inner.poll_close()
    }
}

#[test]
fn ack_faults_cancel_the_server_registration_and_clear_demux_state() {
    for (fault, expected) in [
        (AckFault::Timeout, "transport"),
        (AckFault::Malformed, "malformed"),
        (AckFault::WrongBinding, "binding"),
    ] {
        let service = Arc::new(service(
            ExistingJournalState::Supported(metadata()),
            ReadMode::BlockingUntilCancelled,
            vec![],
            true,
        ));
        let inner = InProcessTransport::new(Arc::clone(&service), connection(1));
        let transport = AckFaultTransport::new(inner.clone(), fault);
        let client = broker_client(transport);
        let error = match client.begin_read_range(read_request()) {
            Ok(_) => panic!("faulty acknowledgement must fail closed"),
            Err(error) => error,
        };
        match expected {
            "transport" => assert!(matches!(error, BrokerClientError::Transport(_))),
            "malformed" => assert!(matches!(error, BrokerClientError::MalformedResponse)),
            "binding" => assert!(matches!(error, BrokerClientError::ResponseBindingMismatch)),
            _ => unreachable!("fixed test case"),
        }
        let deadline = Instant::now() + Duration::from_secs(2);
        while client.pending_count() != 0
            || service.active.active_count([1; 16]) != 0
            || inner.response_count() != 0
            || inner.expected_count() != 0
        {
            assert!(Instant::now() < deadline, "fault cleanup did not finish");
            let _ = client.poll_maintenance(8, Instant::now() + Duration::from_millis(50));
            std::thread::yield_now();
        }
    }
}

#[derive(Clone)]
struct CancelResponseFaultTransport {
    inner: InProcessTransport,
    cancel_request_ids: Arc<Mutex<HashSet<u64>>>,
    injected: Arc<AtomicBool>,
}

impl sealed::Sealed for CancelResponseFaultTransport {}

impl BrokerClientTransport for CancelResponseFaultTransport {
    type Error = InProcessTransportError;

    fn try_send_frame(&mut self, request_frame: &[u8]) -> Result<AdapterPoll<()>, Self::Error> {
        let payload = decode_frame(request_frame).map_err(|_| InProcessTransportError)?;
        if let BrokerRequest::Cancel { request_id, .. } =
            decode_request(payload).map_err(|_| InProcessTransportError)?
        {
            self.cancel_request_ids
                .lock()
                .map_err(|_| InProcessTransportError)?
                .insert(request_id);
        }
        self.inner.try_send_frame(request_frame)
    }

    fn poll_response(&mut self, request_id: u64) -> Result<AdapterPoll<Vec<u8>>, Self::Error> {
        let AdapterPoll::Ready(response) = self.inner.poll_response(request_id)? else {
            return Ok(AdapterPoll::Pending);
        };
        let is_cancel = self
            .cancel_request_ids
            .lock()
            .map_err(|_| InProcessTransportError)?
            .remove(&request_id);
        if is_cancel && !self.injected.swap(true, Ordering::AcqRel) {
            Err(InProcessTransportError)
        } else {
            Ok(AdapterPoll::Ready(response))
        }
    }

    fn try_discard(&mut self, request_id: u64) -> Result<AdapterPoll<()>, Self::Error> {
        if let Ok(mut request_ids) = self.cancel_request_ids.lock() {
            request_ids.remove(&request_id);
        }
        self.inner.try_discard(request_id)
    }

    fn begin_abandon(
        &mut self,
        target_request_id: u64,
        cancel_request: Option<(u64, Vec<u8>)>,
        close_connection: bool,
    ) -> Result<AdapterPoll<()>, Self::Error> {
        self.inner
            .begin_abandon(target_request_id, cancel_request, close_connection)
    }

    fn poll_abandon(&mut self, target_request_id: u64) -> Result<AdapterPoll<()>, Self::Error> {
        self.inner.poll_abandon(target_request_id)
    }

    fn begin_close(&mut self) -> Result<AdapterPoll<()>, Self::Error> {
        self.inner.begin_close()
    }

    fn poll_close(&mut self) -> Result<AdapterPoll<()>, Self::Error> {
        self.inner.poll_close()
    }
}

#[test]
fn cancel_response_failure_unregisters_control_and_target_lifecycles() {
    let read_started = Arc::new(AtomicUsize::new(0));
    let service = Arc::new(BrokerService::new(
        FakeBackend {
            pin_calls: Arc::new(AtomicUsize::new(0)),
            state: ExistingJournalState::Supported(metadata()),
            mode: ReadMode::BlockingUntilCancelled,
            candidates: vec![],
            read_started: Arc::clone(&read_started),
        },
        FakeAuthorizer {
            allow_root: true,
            name_semantics: vec![NameComparisonSemantics::OrdinalCaseSensitive],
        },
    ));
    let inner = InProcessTransport::new(Arc::clone(&service), connection(1));
    let transport = CancelResponseFaultTransport {
        inner: inner.clone(),
        cancel_request_ids: Arc::new(Mutex::new(HashSet::new())),
        injected: Arc::new(AtomicBool::new(false)),
    };
    let client = broker_client(transport);
    let pending = client
        .begin_read_range(read_request())
        .expect("begin cancellable read");
    let handle = pending.cancel_handle();
    wait_until_started(&read_started);
    assert!(matches!(
        client.cancel(&handle, caller()),
        Err(BrokerClientError::Closed)
    ));
    assert!(matches!(pending.wait(), Err(BrokerClientError::Closed)));
    assert!(matches!(
        client.cancel(&handle, caller()),
        Err(BrokerClientError::Closed)
    ));
    wait_until_clean(&client, &service, &inner);
}

#[test]
fn cancel_and_final_race_has_one_terminal_state_and_never_revives_the_handle() {
    let service = Arc::new(service(
        ExistingJournalState::Supported(metadata()),
        ReadMode::Complete,
        vec![],
        true,
    ));
    let transport = InProcessTransport::new(Arc::clone(&service), connection(1));
    let client = broker_client(transport.clone());
    for _ in 0..100 {
        let pending = client
            .begin_read_range(read_request())
            .expect("begin racing read");
        let handle = pending.cancel_handle();
        let barrier = Arc::new(Barrier::new(3));
        let waiter_barrier = Arc::clone(&barrier);
        let waiter = std::thread::spawn(move || {
            waiter_barrier.wait();
            pending.wait()
        });
        let cancel_barrier = Arc::clone(&barrier);
        let cancelling_client = client.clone();
        let racing_handle = handle.clone();
        let canceller = std::thread::spawn(move || {
            cancel_barrier.wait();
            cancelling_client.cancel(&racing_handle, caller())
        });
        barrier.wait();
        let waited = waiter.join().expect("waiter join");
        let cancelled = canceller.join().expect("canceller join");
        match (waited, cancelled) {
            (Ok(BrokerResponse::ReadRange { .. }), Err(BrokerClientError::RequestNotActive))
            | (Err(BrokerClientError::Cancelled), Ok(())) => {}
            outcome => panic!("invalid concurrent terminal outcome: {outcome:?}"),
        }
        let sends_before = transport.sent_count();
        assert!(matches!(
            client.cancel(&handle, caller()),
            Err(BrokerClientError::RequestNotActive)
        ));
        assert_eq!(transport.sent_count(), sends_before);
        wait_until_clean(&client, &service, &transport);
    }
}

#[test]
fn protocol_mismatch_preserves_safe_request_id_and_backend_errors_are_sanitized() {
    let service = service(
        ExistingJournalState::Supported(metadata()),
        ReadMode::BackendUnavailable,
        vec![],
        true,
    );
    let request = BrokerRequest::QueryJournal {
        request_id: 91,
        request: query_request(),
    };
    let mut payload = encode_request(&request).expect("encode request");
    payload[8..10].copy_from_slice(&99_u16.to_le_bytes());
    let response = handle_payload(&service, &payload, &connection(1));
    assert!(matches!(
        response,
        BrokerResponse::Failure {
            request_id: 91,
            failure: BrokerFailure {
                code: BrokerFailureCode::ProtocolMismatch
            },
            ..
        }
    ));

    let mut legacy_payload = encode_request(&request).expect("encode legacy request");
    legacy_payload[..8].copy_from_slice(&crate::journal_broker::wire::LEGACY_PROTOCOL_MAGIC_V1);
    let response = handle_payload(&service, &legacy_payload, &connection(1));
    assert!(matches!(
        response,
        BrokerResponse::Failure {
            request_id: 91,
            failure: BrokerFailure {
                code: BrokerFailureCode::ProtocolMismatch
            },
            ..
        }
    ));

    let response = round_trip(
        &service,
        BrokerRequest::QueryJournal {
            request_id: 92,
            request: query_request(),
        },
        &connection(1),
    );
    assert!(matches!(
        response,
        BrokerResponse::Failure {
            failure: BrokerFailure {
                code: BrokerFailureCode::BackendUnavailable
            },
            ..
        }
    ));
    let encoded = crate::journal_broker::wire::encode_response(&response).expect("encode failure");
    assert!(!String::from_utf8_lossy(&encoded).contains("backend"));
}

#[test]
fn all_backend_failures_map_to_fixed_protocol_codes() {
    assert_eq!(
        map_backend_failure(BackendError::JournalUnavailable).code,
        BrokerFailureCode::JournalUnavailable
    );
    assert_eq!(
        map_backend_failure(BackendError::VolumeMismatch).code,
        BrokerFailureCode::VolumeMismatch
    );
}

#[derive(Clone)]
struct InProcessTransport {
    inner: Arc<InProcessTransportInner>,
}

struct InProcessTransportInner {
    service: Arc<BrokerService<FakeBackend, FakeAuthorizer>>,
    authenticated: AuthenticatedConnection,
    responses: Mutex<HashMap<u64, VecDeque<Vec<u8>>>>,
    expected: Mutex<HashSet<u64>>,
    abandon_cancel_ids: Mutex<HashMap<u64, u64>>,
    sent: AtomicUsize,
    is_closed: AtomicUsize,
    delay_read_execution: AtomicUsize,
}

impl InProcessTransport {
    fn new(
        service: Arc<BrokerService<FakeBackend, FakeAuthorizer>>,
        authenticated: AuthenticatedConnection,
    ) -> Self {
        service
            .open_connection(&authenticated)
            .expect("open in-process connection");
        Self {
            inner: Arc::new(InProcessTransportInner {
                service,
                authenticated,
                responses: Mutex::new(HashMap::new()),
                expected: Mutex::new(HashSet::new()),
                abandon_cancel_ids: Mutex::new(HashMap::new()),
                sent: AtomicUsize::new(0),
                is_closed: AtomicUsize::new(0),
                delay_read_execution: AtomicUsize::new(0),
            }),
        }
    }

    fn sent_count(&self) -> usize {
        self.inner.sent.load(Ordering::Acquire)
    }

    fn response_count(&self) -> usize {
        self.inner
            .responses
            .lock()
            .map_or(0, |responses| responses.values().map(VecDeque::len).sum())
    }

    fn expected_count(&self) -> usize {
        self.inner
            .expected
            .lock()
            .map_or(0, |expected| expected.len())
    }

    fn delay_read_execution(&self) {
        self.inner.delay_read_execution.store(1, Ordering::Release);
    }

    fn release_read_execution(&self) {
        self.inner.delay_read_execution.store(0, Ordering::Release);
    }

    fn discard_request(&self, request_id: u64) {
        if let Ok(mut expected) = self.inner.expected.lock() {
            expected.remove(&request_id);
        }
        if let Ok(mut responses) = self.inner.responses.lock() {
            responses.remove(&request_id);
        }
    }
}

#[derive(Debug)]
struct InProcessTransportError;

impl fmt::Display for InProcessTransportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("in-process transport failed")
    }
}

impl std::error::Error for InProcessTransportError {}

impl sealed::Sealed for InProcessTransport {}

impl BrokerClientTransport for InProcessTransport {
    type Error = InProcessTransportError;

    fn try_send_frame(&mut self, request_frame: &[u8]) -> Result<AdapterPoll<()>, Self::Error> {
        if self.inner.is_closed.load(Ordering::Acquire) != 0 {
            return Err(InProcessTransportError);
        }
        let payload = decode_frame(request_frame).map_err(|_| InProcessTransportError)?;
        let request = decode_request(payload).map_err(|_| InProcessTransportError)?;
        let request_id = inspect_request_header(payload)
            .map_err(|_| InProcessTransportError)?
            .request_id;
        self.inner
            .expected
            .lock()
            .map_err(|_| InProcessTransportError)?
            .insert(request_id);
        self.inner.sent.fetch_add(1, Ordering::AcqRel);
        let frame = request_frame.to_vec();
        let inner = Arc::clone(&self.inner);
        std::thread::spawn(move || {
            let Ok(response) = inner.service.handle_frame(&frame, &inner.authenticated) else {
                return;
            };
            push_in_process_response(&inner, request_id, response);
            if let BrokerRequest::ReadRange { request, .. } = request {
                while inner.delay_read_execution.load(Ordering::Acquire) != 0
                    && inner.is_closed.load(Ordering::Acquire) == 0
                {
                    std::thread::yield_now();
                }
                if let Ok(final_response) = inner.service.execute_accepted_read_frame(
                    request_id,
                    request.caller.client_instance,
                    &inner.authenticated,
                ) {
                    push_in_process_response(&inner, request_id, final_response);
                }
            }
        });
        Ok(AdapterPoll::Ready(()))
    }

    fn poll_response(&mut self, request_id: u64) -> Result<AdapterPoll<Vec<u8>>, Self::Error> {
        let mut responses = self
            .inner
            .responses
            .lock()
            .map_err(|_| InProcessTransportError)?;
        let Some(queue) = responses.get_mut(&request_id) else {
            return Ok(AdapterPoll::Pending);
        };
        let Some(response) = queue.pop_front() else {
            return Ok(AdapterPoll::Pending);
        };
        if queue.is_empty() {
            responses.remove(&request_id);
        }
        Ok(AdapterPoll::Ready(response))
    }

    fn try_discard(&mut self, request_id: u64) -> Result<AdapterPoll<()>, Self::Error> {
        self.discard_request(request_id);
        Ok(AdapterPoll::Ready(()))
    }

    fn begin_abandon(
        &mut self,
        target_request_id: u64,
        cancel_request: Option<(u64, Vec<u8>)>,
        close_connection: bool,
    ) -> Result<AdapterPoll<()>, Self::Error> {
        self.discard_request(target_request_id);
        if close_connection {
            return self.begin_close();
        }
        let Some((cancel_request_id, frame)) = cancel_request else {
            return Ok(AdapterPoll::Ready(()));
        };
        match self.try_send_frame(&frame)? {
            AdapterPoll::Ready(()) => {
                self.inner
                    .abandon_cancel_ids
                    .lock()
                    .map_err(|_| InProcessTransportError)?
                    .insert(target_request_id, cancel_request_id);
                Ok(AdapterPoll::Pending)
            }
            AdapterPoll::Pending => Ok(AdapterPoll::Pending),
        }
    }

    fn poll_abandon(&mut self, target_request_id: u64) -> Result<AdapterPoll<()>, Self::Error> {
        let Some(cancel_request_id) = self
            .inner
            .abandon_cancel_ids
            .lock()
            .map_err(|_| InProcessTransportError)?
            .get(&target_request_id)
            .copied()
        else {
            return Ok(AdapterPoll::Ready(()));
        };
        match self.poll_response(cancel_request_id)? {
            AdapterPoll::Ready(_) => {
                self.discard_request(cancel_request_id);
                self.inner
                    .abandon_cancel_ids
                    .lock()
                    .map_err(|_| InProcessTransportError)?
                    .remove(&target_request_id);
                Ok(AdapterPoll::Ready(()))
            }
            AdapterPoll::Pending => Ok(AdapterPoll::Pending),
        }
    }

    fn begin_close(&mut self) -> Result<AdapterPoll<()>, Self::Error> {
        self.inner.is_closed.store(1, Ordering::Release);
        self.inner.service.disconnect(&self.inner.authenticated);
        if let Ok(mut expected) = self.inner.expected.lock() {
            expected.clear();
        }
        if let Ok(mut responses) = self.inner.responses.lock() {
            responses.clear();
        }
        Ok(AdapterPoll::Ready(()))
    }

    fn poll_close(&mut self) -> Result<AdapterPoll<()>, Self::Error> {
        Ok(AdapterPoll::Ready(()))
    }
}

fn push_in_process_response(inner: &InProcessTransportInner, request_id: u64, response: Vec<u8>) {
    if !inner
        .expected
        .lock()
        .is_ok_and(|expected| expected.contains(&request_id))
    {
        return;
    }
    if let Ok(mut responses) = inner.responses.lock() {
        responses.entry(request_id).or_default().push_back(response);
    }
}

fn wait_until_started(read_started: &AtomicUsize) {
    let deadline = Instant::now() + Duration::from_secs(2);
    while read_started.load(Ordering::Acquire) == 0 {
        assert!(Instant::now() < deadline, "read backend did not start");
        std::thread::yield_now();
    }
}

fn wait_until_clean<Transport: BrokerClientTransport>(
    client: &JournalBrokerClient<Transport>,
    service: &BrokerService<FakeBackend, FakeAuthorizer>,
    transport: &InProcessTransport,
) {
    let deadline = Instant::now() + Duration::from_secs(2);
    while client.pending_count() != 0
        || service.active.active_count([1; 16]) != 0
        || transport.response_count() != 0
        || transport.expected_count() != 0
    {
        assert!(Instant::now() < deadline, "broker cleanup did not finish");
        let _ = client.poll_maintenance(8, Instant::now() + Duration::from_millis(50));
        std::thread::yield_now();
    }
}

fn service(
    state: ExistingJournalState,
    mode: ReadMode,
    candidates: Vec<BackendJournalCandidate>,
    allow_root: bool,
) -> BrokerService<FakeBackend, FakeAuthorizer> {
    BrokerService::new(
        FakeBackend {
            pin_calls: Arc::new(AtomicUsize::new(0)),
            state,
            mode,
            candidates,
            read_started: Arc::new(AtomicUsize::new(0)),
        },
        FakeAuthorizer {
            allow_root,
            name_semantics: vec![NameComparisonSemantics::OrdinalCaseSensitive],
        },
    )
}

fn round_trip<Backend, Authorizer>(
    service: &BrokerService<Backend, Authorizer>,
    request: BrokerRequest,
    authenticated: &AuthenticatedConnection,
) -> BrokerResponse
where
    Backend: ExistingJournalBackend,
    Authorizer: CallerAuthorizer,
{
    let accepted_read = match &request {
        BrokerRequest::ReadRange {
            request_id,
            request,
        } => Some((*request_id, request.caller.client_instance)),
        _ => None,
    };
    let payload = encode_request(&request).expect("encode request");
    let response = handle_payload(service, &payload, authenticated);
    if let Some((request_id, client_instance)) = accepted_read {
        assert!(matches!(response, BrokerResponse::ReadRangeAccepted { .. }));
        let frame = service
            .execute_accepted_read_frame(request_id, client_instance, authenticated)
            .expect("execute accepted read");
        return decode_response(decode_frame(&frame).expect("decode frame"))
            .expect("decode response");
    }
    response
}

fn round_trip_accept_only<Backend, Authorizer>(
    service: &BrokerService<Backend, Authorizer>,
    request: BrokerRequest,
    authenticated: &AuthenticatedConnection,
) -> BrokerResponse
where
    Backend: ExistingJournalBackend,
    Authorizer: CallerAuthorizer,
{
    let payload = encode_request(&request).expect("encode request");
    handle_payload(service, &payload, authenticated)
}

fn handle_payload<Backend, Authorizer>(
    service: &BrokerService<Backend, Authorizer>,
    payload: &[u8],
    authenticated: &AuthenticatedConnection,
) -> BrokerResponse
where
    Backend: ExistingJournalBackend,
    Authorizer: CallerAuthorizer,
{
    service
        .open_connection(authenticated)
        .expect("open test connection");
    let frame = encode_frame(payload).expect("encode frame");
    let response_frame = service
        .handle_frame(&frame, authenticated)
        .expect("service response");
    decode_response(decode_frame(&response_frame).expect("decode frame")).expect("decode response")
}

fn query_request() -> QueryJournalRequest {
    QueryJournalRequest {
        caller: caller(),
        root: root(),
        root_capability: RootCapability([7; 32]),
        timeout_ms: 1_000,
    }
}

fn read_request() -> ReadJournalRangeRequest {
    ReadJournalRangeRequest {
        caller: caller(),
        root: root(),
        root_capability: RootCapability([7; 32]),
        journal_id: 7,
        start_usn: 10,
        end_usn: 20,
        max_records: 16,
        max_evidence_bytes: 4_096,
        timeout_ms: 1_000,
    }
}

fn caller() -> CallerClaim {
    CallerClaim {
        process_id: 100,
        session_id: 2,
        client_instance: [9; 16],
    }
}

fn connection(id: u8) -> AuthenticatedConnection {
    AuthenticatedConnection::from_pipe_token(
        [id; 16],
        1,
        AuthenticatedPipeFacts {
            opaque_token_binding: [id; 32],
            client_binary_identity: [id.wrapping_add(1); 32],
            process_id: 100,
            session_id: 2,
            token_type: 2,
            impersonation_level: 2,
            is_elevated: false,
        },
    )
    .expect("valid fake authenticated connection")
}

fn root() -> RootAuthorization {
    RootAuthorization {
        root_id: "root-a".to_owned(),
        root_generation: 3,
        volume_id: "volume-a".to_owned(),
        root_identity: vec![1; 16],
        canonical_root_utf16: "C:\\Pictures".encode_utf16().collect(),
    }
}

fn metadata() -> ExistingJournalMetadata {
    ExistingJournalMetadata {
        journal_id: 7,
        first_usn: 0,
        next_usn: 20,
    }
}

#[derive(Clone)]
struct SharedCountingBackend {
    physical_reads: Arc<AtomicUsize>,
    post_admission_failure_index: Option<usize>,
    post_admission_failure_barrier: Option<i64>,
    handoff_coordinates: Vec<(i64, i64)>,
}

impl ExistingJournalBackend for SharedCountingBackend {
    fn query_existing_journal(
        &self,
        _root: &AuthorizedPinnedRoot,
        context: &mut RequestContext<'_>,
    ) -> Result<ExistingJournalState, BackendError> {
        let _ = context.bounded_step()?;
        Ok(ExistingJournalState::Supported(metadata()))
    }

    fn read_existing_journal_page(
        &self,
        _root: &AuthorizedPinnedRoot,
        _journal: ExistingJournalMetadata,
        _range: JournalRange,
        _page: &mut BoundedJournalPageBuilder,
        _context: &mut RequestContext<'_>,
    ) -> Result<JournalReadProof, BackendError> {
        panic!("shared requests must not fall back to a per-root read")
    }

    fn read_existing_journal_volume_page(
        &self,
        request: ExistingJournalVolumeRead<'_>,
        context: &mut RequestContext<'_>,
    ) -> Result<SharedBackendJournalPage, BackendError> {
        let _ = context.bounded_step()?;
        self.physical_reads.fetch_add(1, Ordering::AcqRel);
        let candidates_by_root = (0..request.roots.len())
            .map(|index| {
                if self.post_admission_failure_index == Some(index) {
                    Err(BackendError::RootUnavailable)
                } else {
                    Ok(Vec::new())
                }
            })
            .collect::<Vec<_>>();
        Ok(SharedBackendJournalPage {
            candidates_by_root,
            handoffs: if request.roots.len() == 2 && self.post_admission_failure_index.is_none() {
                self.handoff_coordinates
                    .iter()
                    .map(|(old_usn, new_usn)| BackendJournalHandoff {
                        previous_root_index: Some(0),
                        previous_pending_index: None,
                        current_root_index: 1,
                        previous_absolute_path_utf16: "C:\\Pictures\\old.jpg"
                            .encode_utf16()
                            .collect(),
                        current_absolute_path_utf16: "C:\\Other\\new.jpg".encode_utf16().collect(),
                        file_reference: vec![5; 16],
                        old_usn: *old_usn,
                        usn: *new_usn,
                        is_directory: false,
                    })
                    .collect()
            } else {
                Vec::new()
            },
            pending_renames: Vec::new(),
            rename_barriers_by_root: (0..request.roots.len())
                .map(|index| {
                    (self.post_admission_failure_index == Some(index))
                        .then_some(self.post_admission_failure_barrier)
                        .flatten()
                })
                .collect(),
            proof: Some(JournalReadProof::after_read(
                request.range.end_usn,
                true,
                request.journal,
            )?),
        })
    }
}

#[derive(Clone)]
struct CarryRevalidationAuthorizer {
    inner: FakeAuthorizer,
    calls_by_root: Arc<Mutex<HashMap<String, usize>>>,
}

impl CallerAuthorizer for CarryRevalidationAuthorizer {
    fn verify_claim(
        &self,
        authenticated: &AuthenticatedConnection,
        claim: &CallerClaim,
    ) -> Result<CallerAuthorization, AuthorizerError> {
        self.inner.verify_claim(authenticated, claim)
    }

    fn register_root(
        &self,
        caller: &CallerAuthorization,
        root: &RootAuthorization,
        client_root_handle: u64,
        context: &mut RequestContext<'_>,
    ) -> Result<RootCapability, AuthorizerError> {
        self.inner
            .register_root(caller, root, client_root_handle, context)
    }

    fn authorize_registered_root(
        &self,
        caller: &CallerAuthorization,
        root: &RootAuthorization,
        capability: RootCapability,
        context: &mut RequestContext<'_>,
    ) -> Result<AuthorizedPinnedRoot, AuthorizerError> {
        let mut calls = self
            .calls_by_root
            .lock()
            .map_err(|_| AuthorizerError::RootUnauthorized)?;
        let count = calls.entry(root.root_id.clone()).or_default();
        *count += 1;
        if root.root_id == "root-a" && *count > 1 {
            return Err(AuthorizerError::RootUnauthorized);
        }
        drop(calls);
        self.inner
            .authorize_registered_root(caller, root, capability, context)
    }

    fn filter_disclosable_candidates(
        &self,
        root: &AuthorizedPinnedRoot,
        candidates: Vec<BackendJournalCandidate>,
        context: &mut RequestContext<'_>,
    ) -> Result<Vec<BackendJournalCandidate>, AuthorizerError> {
        self.inner
            .filter_disclosable_candidates(root, candidates, context)
    }
}

#[derive(Clone)]
struct CarryBarrierBackend {
    saw_unavailable_carry: Arc<AtomicBool>,
    malformed_pending: bool,
}

impl ExistingJournalBackend for CarryBarrierBackend {
    fn query_existing_journal(
        &self,
        _root: &AuthorizedPinnedRoot,
        context: &mut RequestContext<'_>,
    ) -> Result<ExistingJournalState, BackendError> {
        let _ = context.bounded_step()?;
        Ok(ExistingJournalState::Supported(metadata()))
    }

    fn read_existing_journal_page(
        &self,
        _root: &AuthorizedPinnedRoot,
        _journal: ExistingJournalMetadata,
        _range: JournalRange,
        _page: &mut BoundedJournalPageBuilder,
        _context: &mut RequestContext<'_>,
    ) -> Result<JournalReadProof, BackendError> {
        panic!("shared requests must not fall back to a per-root read")
    }

    fn read_existing_journal_volume_page(
        &self,
        request: ExistingJournalVolumeRead<'_>,
        context: &mut RequestContext<'_>,
    ) -> Result<SharedBackendJournalPage, BackendError> {
        let _ = context.bounded_step()?;
        if self.malformed_pending {
            return Ok(SharedBackendJournalPage {
                candidates_by_root: vec![Ok(Vec::new()), Ok(Vec::new())],
                handoffs: Vec::new(),
                pending_renames: vec![BackendPendingRename {
                    root_index: 0,
                    previous_absolute_path_utf16: "C:\\Pictures\\..\\escape.jpg"
                        .encode_utf16()
                        .collect(),
                    file_reference: vec![5; 16],
                    old_usn: 15,
                    is_directory: false,
                }],
                rename_barriers_by_root: vec![None, None],
                proof: Some(JournalReadProof::after_read(20, true, request.journal)?),
            });
        }
        self.saw_unavailable_carry.store(
            request.pending_renames.len() == 1
                && request.pending_renames[0].read_root_index == Some(0)
                && request.pending_renames[0].root.is_none(),
            Ordering::Release,
        );
        Ok(SharedBackendJournalPage {
            candidates_by_root: vec![Ok(Vec::new()), Ok(Vec::new())],
            handoffs: Vec::new(),
            pending_renames: Vec::new(),
            rename_barriers_by_root: vec![None, None],
            proof: Some(JournalReadProof::after_read(15, false, request.journal)?),
        })
    }
}

#[test]
fn shared_volume_read_returns_cross_root_handoff_without_raw_paths() {
    let physical_reads = Arc::new(AtomicUsize::new(0));
    let service = BrokerService::new(
        SharedCountingBackend {
            physical_reads: Arc::clone(&physical_reads),
            post_admission_failure_index: None,
            post_admission_failure_barrier: None,
            handoff_coordinates: vec![(14, 15)],
        },
        FakeAuthorizer {
            allow_root: true,
            name_semantics: vec![NameComparisonSemantics::WindowsOrdinalIgnoreCase],
        },
    );
    let authenticated = connection(52);
    service
        .open_connection(&authenticated)
        .expect("open connection");
    let accepted = round_trip(
        &service,
        BrokerRequest::ReadVolume {
            request_id: 9_002,
            request: shared_read_request(false),
        },
        &authenticated,
    );
    assert!(matches!(
        accepted,
        BrokerResponse::ReadVolumeAccepted { .. }
    ));
    let response = service
        .execute_accepted_read_frame(9_002, caller().client_instance, &authenticated)
        .expect("execute shared read");
    let response = decode_response(decode_frame(&response).expect("frame")).expect("response");
    let BrokerResponse::ReadVolume { handoffs, .. } = response else {
        panic!("expected shared response")
    };
    assert_eq!(physical_reads.load(Ordering::Acquire), 1);
    assert_eq!(handoffs.len(), 1);
    assert_eq!(handoffs[0].previous_relative_path, "old.jpg");
    assert_eq!(handoffs[0].current_relative_path, "new.jpg");
    assert!(!handoffs[0].previous_relative_path.contains("C:"));
    assert!(!handoffs[0].current_relative_path.contains("C:"));
}

#[test]
fn shared_volume_read_calls_backend_once_and_isolates_one_root_failure() {
    let physical_reads = Arc::new(AtomicUsize::new(0));
    let service = BrokerService::new(
        SharedCountingBackend {
            physical_reads: Arc::clone(&physical_reads),
            post_admission_failure_index: None,
            post_admission_failure_barrier: None,
            handoff_coordinates: vec![(14, 15)],
        },
        FakeAuthorizer {
            allow_root: true,
            name_semantics: vec![NameComparisonSemantics::WindowsOrdinalIgnoreCase],
        },
    );
    let authenticated = connection(51);
    service
        .open_connection(&authenticated)
        .expect("open connection");
    let request = shared_read_request(true);
    let accepted = round_trip(
        &service,
        BrokerRequest::ReadVolume {
            request_id: 9_001,
            request,
        },
        &authenticated,
    );
    assert!(matches!(
        accepted,
        BrokerResponse::ReadVolumeAccepted { root_count: 2, .. }
    ));
    let response = service
        .execute_accepted_read_frame(9_001, caller().client_instance, &authenticated)
        .expect("execute shared read");
    let response = decode_response(decode_frame(&response).expect("frame")).expect("response");
    let BrokerResponse::ReadVolume { outcomes, .. } = response else {
        panic!("expected shared terminal response")
    };
    assert_eq!(physical_reads.load(Ordering::Acquire), 1);
    assert!(outcomes[0].failure.is_none());
    assert_eq!(
        outcomes[1].failure.map(|failure| failure.code),
        Some(BrokerFailureCode::RootUnauthorized)
    );
}

#[test]
fn post_admission_root_failure_keeps_a_healthy_volume_sibling_progressing() {
    let physical_reads = Arc::new(AtomicUsize::new(0));
    let service = BrokerService::new(
        SharedCountingBackend {
            physical_reads: Arc::clone(&physical_reads),
            post_admission_failure_index: Some(0),
            post_admission_failure_barrier: None,
            handoff_coordinates: vec![(14, 15)],
        },
        FakeAuthorizer {
            allow_root: true,
            name_semantics: vec![NameComparisonSemantics::WindowsOrdinalIgnoreCase],
        },
    );
    let authenticated = connection(53);
    service
        .open_connection(&authenticated)
        .expect("open connection");
    assert!(matches!(
        round_trip(
            &service,
            BrokerRequest::ReadVolume {
                request_id: 9_003,
                request: shared_read_request(false),
            },
            &authenticated,
        ),
        BrokerResponse::ReadVolumeAccepted { root_count: 2, .. }
    ));
    let response = service
        .execute_accepted_read_frame(9_003, caller().client_instance, &authenticated)
        .expect("execute post-admission failure");
    let response = decode_response(decode_frame(&response).expect("frame")).expect("response");
    let BrokerResponse::ReadVolume { outcomes, .. } = response else {
        panic!("expected shared terminal response")
    };

    assert_eq!(physical_reads.load(Ordering::Acquire), 1);
    assert_eq!(
        outcomes[0].failure.map(|failure| failure.code),
        Some(BrokerFailureCode::RootIdentityMismatch)
    );
    assert!(outcomes[1].failure.is_none());
    assert_eq!(outcomes[1].covered_until_usn, Some(20));
}

#[test]
fn post_admission_endpoint_uncertainty_stops_both_directions_at_rename_barrier() {
    for (case, failed_index) in [(54_u8, 0_usize), (55_u8, 1_usize)] {
        let physical_reads = Arc::new(AtomicUsize::new(0));
        let service = BrokerService::new(
            SharedCountingBackend {
                physical_reads: Arc::clone(&physical_reads),
                post_admission_failure_index: Some(failed_index),
                post_admission_failure_barrier: Some(15),
                handoff_coordinates: vec![(14, 15)],
            },
            FakeAuthorizer {
                allow_root: true,
                name_semantics: vec![NameComparisonSemantics::WindowsOrdinalIgnoreCase],
            },
        );
        let authenticated = connection(case);
        service
            .open_connection(&authenticated)
            .expect("open connection");
        let request_id = 9_100 + u64::from(case);
        assert!(matches!(
            round_trip(
                &service,
                BrokerRequest::ReadVolume {
                    request_id,
                    request: shared_read_request(false),
                },
                &authenticated,
            ),
            BrokerResponse::ReadVolumeAccepted { root_count: 2, .. }
        ));
        let response = service
            .execute_accepted_read_frame(request_id, caller().client_instance, &authenticated)
            .expect("execute endpoint uncertainty");
        let response = decode_response(decode_frame(&response).expect("frame")).expect("response");
        let BrokerResponse::ReadVolume {
            outcomes, handoffs, ..
        } = response
        else {
            panic!("expected shared terminal response")
        };
        assert_eq!(physical_reads.load(Ordering::Acquire), 1);
        assert!(outcomes[failed_index].failure.is_some());
        let healthy_index = 1 - failed_index;
        assert!(outcomes[healthy_index].failure.is_none());
        assert_eq!(outcomes[healthy_index].covered_until_usn, Some(15));
        assert!(!outcomes[healthy_index].is_complete);
        assert!(handoffs.is_empty());
    }
}

#[test]
fn target_translation_failure_preserves_verified_old_as_pending_carry() {
    let physical_reads = Arc::new(AtomicUsize::new(0));
    let service = BrokerService::new(
        SharedCountingBackend {
            physical_reads: Arc::clone(&physical_reads),
            post_admission_failure_index: None,
            post_admission_failure_barrier: None,
            handoff_coordinates: vec![(14, 15)],
        },
        EndpointTranslationAuthorizer {
            inner: FakeAuthorizer {
                allow_root: true,
                name_semantics: vec![NameComparisonSemantics::WindowsOrdinalIgnoreCase],
            },
            failing_root_id: "root-b",
        },
    );
    let authenticated = connection(58);
    service
        .open_connection(&authenticated)
        .expect("open connection");
    assert!(matches!(
        round_trip(
            &service,
            BrokerRequest::ReadVolume {
                request_id: 9_158,
                request: shared_read_request(false),
            },
            &authenticated,
        ),
        BrokerResponse::ReadVolumeAccepted { root_count: 2, .. }
    ));
    let response = service
        .execute_accepted_read_frame(9_158, caller().client_instance, &authenticated)
        .expect("execute target translation failure");
    let response = decode_response(decode_frame(&response).expect("frame")).expect("response");
    let BrokerResponse::ReadVolume {
        outcomes,
        handoffs,
        pending_renames,
        ..
    } = response
    else {
        panic!("expected shared terminal response")
    };

    assert_eq!(physical_reads.load(Ordering::Acquire), 1);
    assert!(outcomes[0].failure.is_none());
    assert_eq!(outcomes[0].covered_until_usn, Some(15));
    assert!(!outcomes[0].is_complete);
    assert_eq!(
        outcomes[1].failure.map(|failure| failure.code),
        Some(BrokerFailureCode::RootIdentityMismatch)
    );
    assert!(handoffs.is_empty());
    assert_eq!(pending_renames.len(), 1);
    assert_eq!(pending_renames[0].binding.root_id, "root-a");
    assert_eq!(pending_renames[0].old_usn, 14);
    assert_eq!(pending_renames[0].previous_relative_path, "old.jpg");
    assert_eq!(pending_renames[0].file_reference, vec![5; 16]);
}

#[test]
fn multiple_target_failures_use_the_earliest_safe_new_barrier() {
    let physical_reads = Arc::new(AtomicUsize::new(0));
    let service = BrokerService::new(
        SharedCountingBackend {
            physical_reads: Arc::clone(&physical_reads),
            post_admission_failure_index: None,
            post_admission_failure_barrier: None,
            handoff_coordinates: vec![(12, 13), (14, 15)],
        },
        EndpointTranslationAuthorizer {
            inner: FakeAuthorizer {
                allow_root: true,
                name_semantics: vec![NameComparisonSemantics::WindowsOrdinalIgnoreCase],
            },
            failing_root_id: "root-b",
        },
    );
    let authenticated = connection(60);
    service
        .open_connection(&authenticated)
        .expect("open connection");
    assert!(matches!(
        round_trip(
            &service,
            BrokerRequest::ReadVolume {
                request_id: 9_160,
                request: shared_read_request(false),
            },
            &authenticated,
        ),
        BrokerResponse::ReadVolumeAccepted { root_count: 2, .. }
    ));
    let response = service
        .execute_accepted_read_frame(9_160, caller().client_instance, &authenticated)
        .expect("execute multiple target failures");
    let response = decode_response(decode_frame(&response).expect("frame")).expect("response");
    let BrokerResponse::ReadVolume {
        outcomes,
        handoffs,
        pending_renames,
        ..
    } = response
    else {
        panic!("expected shared terminal response")
    };

    assert_eq!(physical_reads.load(Ordering::Acquire), 1);
    assert_eq!(outcomes[0].covered_until_usn, Some(13));
    assert!(!outcomes[0].is_complete);
    assert!(outcomes[1].failure.is_some());
    assert!(handoffs.is_empty());
    assert_eq!(pending_renames.len(), 1);
    assert_eq!(pending_renames[0].old_usn, 12);
}

#[test]
fn source_translation_failure_falls_back_before_old_without_synthesizing_carry() {
    let physical_reads = Arc::new(AtomicUsize::new(0));
    let service = BrokerService::new(
        SharedCountingBackend {
            physical_reads: Arc::clone(&physical_reads),
            post_admission_failure_index: None,
            post_admission_failure_barrier: None,
            handoff_coordinates: vec![(14, 15)],
        },
        EndpointTranslationAuthorizer {
            inner: FakeAuthorizer {
                allow_root: true,
                name_semantics: vec![NameComparisonSemantics::WindowsOrdinalIgnoreCase],
            },
            failing_root_id: "root-a",
        },
    );
    let authenticated = connection(59);
    service
        .open_connection(&authenticated)
        .expect("open connection");
    assert!(matches!(
        round_trip(
            &service,
            BrokerRequest::ReadVolume {
                request_id: 9_159,
                request: shared_read_request(false),
            },
            &authenticated,
        ),
        BrokerResponse::ReadVolumeAccepted { root_count: 2, .. }
    ));
    let response = service
        .execute_accepted_read_frame(9_159, caller().client_instance, &authenticated)
        .expect("execute source translation failure");
    let response = decode_response(decode_frame(&response).expect("frame")).expect("response");
    let BrokerResponse::ReadVolume {
        outcomes,
        handoffs,
        pending_renames,
        ..
    } = response
    else {
        panic!("expected shared terminal response")
    };

    assert_eq!(physical_reads.load(Ordering::Acquire), 1);
    assert_eq!(
        outcomes[0].failure.map(|failure| failure.code),
        Some(BrokerFailureCode::RootIdentityMismatch)
    );
    assert!(outcomes[1].failure.is_none());
    assert_eq!(outcomes[1].covered_until_usn, Some(14));
    assert!(!outcomes[1].is_complete);
    assert!(handoffs.is_empty());
    assert!(pending_renames.is_empty());
}

#[test]
fn pending_old_revalidation_failure_isolated_to_owner_and_barriers_healthy_sibling() {
    let saw_unavailable_carry = Arc::new(AtomicBool::new(false));
    let service = BrokerService::new(
        CarryBarrierBackend {
            saw_unavailable_carry: Arc::clone(&saw_unavailable_carry),
            malformed_pending: false,
        },
        CarryRevalidationAuthorizer {
            inner: FakeAuthorizer {
                allow_root: true,
                name_semantics: vec![NameComparisonSemantics::WindowsOrdinalIgnoreCase],
            },
            calls_by_root: Arc::new(Mutex::new(HashMap::new())),
        },
    );
    let authenticated = connection(56);
    service
        .open_connection(&authenticated)
        .expect("open connection");
    let mut request = shared_read_request(false);
    request.pending_renames.push(
        crate::journal_broker::wire::SharedJournalPendingRenameRequest {
            carry_id: "carry-a".to_owned(),
            source_range_id: "range-a".to_owned(),
            root: request.roots[0].root.clone(),
            root_capability: Some(RootCapability([7; 32])),
            file_reference: vec![5; 16],
            old_usn: 9,
            previous_relative_path: "old.jpg".to_owned(),
            is_directory: false,
        },
    );
    assert!(matches!(
        round_trip(
            &service,
            BrokerRequest::ReadVolume {
                request_id: 9_156,
                request,
            },
            &authenticated,
        ),
        BrokerResponse::ReadVolumeAccepted { root_count: 2, .. }
    ));
    let response = service
        .execute_accepted_read_frame(9_156, caller().client_instance, &authenticated)
        .expect("execute carry revalidation failure");
    let response = decode_response(decode_frame(&response).expect("frame")).expect("response");
    let BrokerResponse::ReadVolume { outcomes, .. } = response else {
        panic!("expected shared terminal response")
    };
    assert!(saw_unavailable_carry.load(Ordering::Acquire));
    assert_eq!(
        outcomes[0].failure.map(|failure| failure.code),
        Some(BrokerFailureCode::RootUnauthorized)
    );
    assert!(outcomes[1].failure.is_none());
    assert_eq!(outcomes[1].covered_until_usn, Some(15));
    assert!(!outcomes[1].is_complete);
}

#[test]
fn malformed_pending_old_is_a_root_failure_with_a_safe_sibling_prefix() {
    let service = BrokerService::new(
        CarryBarrierBackend {
            saw_unavailable_carry: Arc::new(AtomicBool::new(false)),
            malformed_pending: true,
        },
        FakeAuthorizer {
            allow_root: true,
            name_semantics: vec![NameComparisonSemantics::WindowsOrdinalIgnoreCase],
        },
    );
    let authenticated = connection(57);
    service
        .open_connection(&authenticated)
        .expect("open connection");
    assert!(matches!(
        round_trip(
            &service,
            BrokerRequest::ReadVolume {
                request_id: 9_157,
                request: shared_read_request(false),
            },
            &authenticated,
        ),
        BrokerResponse::ReadVolumeAccepted { root_count: 2, .. }
    ));
    let response = service
        .execute_accepted_read_frame(9_157, caller().client_instance, &authenticated)
        .expect("execute malformed carry page");
    let response = decode_response(decode_frame(&response).expect("frame")).expect("response");
    let BrokerResponse::ReadVolume { outcomes, .. } = response else {
        panic!("expected shared terminal response")
    };
    assert_eq!(
        outcomes[0].failure.map(|failure| failure.code),
        Some(BrokerFailureCode::RecordUnsupported)
    );
    assert!(outcomes[1].failure.is_none());
    assert_eq!(outcomes[1].covered_until_usn, Some(15));
    assert!(!outcomes[1].is_complete);
}

fn shared_read_request(fail_second_capability: bool) -> ReadJournalVolumeRequest {
    let mut second = root();
    second.root_id = "root-b".to_owned();
    second.root_generation = 4;
    second.root_identity = vec![2; 16];
    second.canonical_root_utf16 = "C:\\Other".encode_utf16().collect();
    ReadJournalVolumeRequest {
        caller: caller(),
        roots: vec![
            crate::journal_broker::SharedJournalRootRequest {
                root: root(),
                root_capability: RootCapability([7; 32]),
                start_usn: 10,
            },
            crate::journal_broker::SharedJournalRootRequest {
                root: second,
                root_capability: if fail_second_capability {
                    RootCapability([8; 32])
                } else {
                    RootCapability([7; 32])
                },
                start_usn: 12,
            },
        ],
        pending_renames: Vec::new(),
        journal_id: 7,
        end_usn: 20,
        max_records: 16,
        max_evidence_bytes: 4_096,
        timeout_ms: 1_000,
    }
}

fn candidate(
    path: &str,
    previous: Option<&str>,
    usn: i64,
    is_directory: bool,
) -> BackendJournalCandidate {
    let path_utf16: Vec<u16> = path.encode_utf16().collect();
    let previous_utf16 = previous.map(|value| value.encode_utf16().collect::<Vec<_>>());
    let kind = if previous.is_some() {
        CandidateKind::Rename
    } else if is_directory {
        CandidateKind::Subtree
    } else {
        CandidateKind::Path
    };
    BackendJournalCandidate::copy_from_bounded(
        "volume-a",
        &path_utf16,
        previous_utf16.as_deref(),
        &[1; 8],
        usn,
        kind,
        is_directory,
    )
    .expect("valid candidate")
}

fn active_key(connection: u8, client: u8, request_id: u64) -> ActiveRequestKey {
    ActiveRequestKey {
        connection_id: [connection; 16],
        connection_generation: 1,
        authentication_namespace: [connection; 32],
        client_instance: [client; 16],
        request_id,
    }
}
