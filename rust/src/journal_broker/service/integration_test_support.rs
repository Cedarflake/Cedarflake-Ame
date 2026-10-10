use std::collections::{HashMap, VecDeque};
use std::fmt;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use super::contracts::RootNameSemanticsProof;
use super::*;
use crate::journal_broker::client::{
    AdapterPoll, BrokerClientTransport, JournalBrokerClient, OwnedBrokerConnection, sealed,
};
use crate::journal_broker::framing::decode_frame;
use crate::journal_broker::wire::{
    BrokerRequest, NameComparisonSemantics, decode_request, inspect_request_header,
};
use crate::journal_broker::{
    PersistentChangeJournal, PersistentChangeJournalConnection, RootAuthorization, RootCapability,
};

const SOURCE_ROOT_ID: &str = "journal-source-root";
const TARGET_ROOT_ID: &str = "journal-destination-root";
const VOLUME_ID: &str = "journal-volume-guid";
const JOURNAL_ID: u64 = 77;
const OLD_USN: i64 = 15;
const NEW_USN: i64 = 18;

pub(crate) struct TargetTranslationRecoveryFixture {
    pub(crate) journal: Arc<dyn PersistentChangeJournal>,
    pub(crate) caller: CallerClaim,
    pub(crate) roots: Vec<RootAuthorization>,
    pub(crate) recovered: Arc<AtomicBool>,
    pub(crate) physical_reads: Arc<AtomicUsize>,
}

pub(crate) fn target_translation_recovery_fixture() -> TargetTranslationRecoveryFixture {
    let recovered = Arc::new(AtomicBool::new(false));
    let physical_reads = Arc::new(AtomicUsize::new(0));
    let service = Arc::new(BrokerService::new(
        TargetTranslationBackend {
            recovered: Arc::clone(&recovered),
            physical_reads: Arc::clone(&physical_reads),
        },
        TargetTranslationAuthorizer {
            recovered: Arc::clone(&recovered),
        },
    ));
    let caller = CallerClaim {
        process_id: 400,
        session_id: 9,
        client_instance: [41; 16],
    };
    let roots = vec![
        test_root(SOURCE_ROOT_ID, [11; 16], "C:\\JournalSource"),
        test_root(TARGET_ROOT_ID, [12; 16], "C:\\JournalDestination"),
    ];
    TargetTranslationRecoveryFixture {
        journal: Arc::new(InProcessJournal {
            service,
            next_connection: AtomicU64::new(1),
        }),
        caller,
        roots,
        recovered,
        physical_reads,
    }
}

fn test_root(root_id: &str, identity: [u8; 16], path: &str) -> RootAuthorization {
    RootAuthorization {
        root_id: root_id.to_owned(),
        root_generation: 1,
        volume_id: VOLUME_ID.to_owned(),
        root_identity: identity.to_vec(),
        canonical_root_utf16: path.encode_utf16().collect(),
    }
}

struct InProcessJournal {
    service: Arc<TestService>,
    next_connection: AtomicU64,
}

type TestService = BrokerService<TargetTranslationBackend, TargetTranslationAuthorizer>;

impl PersistentChangeJournal for InProcessJournal {
    fn connect(&self) -> PersistentChangeJournalConnection {
        let nonce = self.next_connection.fetch_add(1, Ordering::AcqRel);
        let mut connection_id = [0_u8; 16];
        connection_id[..8].copy_from_slice(&nonce.to_le_bytes());
        connection_id[8..].copy_from_slice(&(!nonce).to_le_bytes());
        let authenticated = AuthenticatedConnection::from_pipe_token(
            connection_id,
            1,
            AuthenticatedPipeFacts {
                opaque_token_binding: [51; 32],
                client_binary_identity: [52; 32],
                process_id: 400,
                session_id: 9,
                token_type: 2,
                impersonation_level: 2,
                is_elevated: false,
            },
        )
        .expect("valid in-process broker identity");
        let transport = InProcessVolumeTransport::new(Arc::clone(&self.service), authenticated);
        let connection = OwnedBrokerConnection::from_adapter(transport, connection_id, 1, nonce)
            .expect("unique in-process broker connection");
        PersistentChangeJournalConnection::Connected(std::sync::Arc::new(JournalBrokerClient::new(
            connection,
        )))
    }
}

#[derive(Clone)]
struct TargetTranslationBackend {
    recovered: Arc<AtomicBool>,
    physical_reads: Arc<AtomicUsize>,
}

impl ExistingJournalBackend for TargetTranslationBackend {
    fn query_existing_journal(
        &self,
        _root: &AuthorizedPinnedRoot,
        context: &mut RequestContext<'_>,
    ) -> Result<ExistingJournalState, BackendError> {
        let _ = context.bounded_step()?;
        Ok(ExistingJournalState::Supported(ExistingJournalMetadata {
            journal_id: JOURNAL_ID,
            first_usn: 1,
            next_usn: 20,
        }))
    }

    fn read_existing_journal_page(
        &self,
        _root: &AuthorizedPinnedRoot,
        _journal: ExistingJournalMetadata,
        _range: JournalRange,
        _page: &mut BoundedJournalPageBuilder,
        _context: &mut RequestContext<'_>,
    ) -> Result<JournalReadProof, BackendError> {
        Err(BackendError::Unavailable)
    }

    fn read_existing_journal_volume_page(
        &self,
        request: ExistingJournalVolumeRead<'_>,
        context: &mut RequestContext<'_>,
    ) -> Result<SharedBackendJournalPage, BackendError> {
        let _ = context.bounded_step()?;
        self.physical_reads.fetch_add(1, Ordering::AcqRel);
        let source = request
            .roots
            .iter()
            .position(|root| root.declared_root.root_id == SOURCE_ROOT_ID)
            .ok_or(BackendError::RootUnavailable)?;
        let target = request
            .roots
            .iter()
            .position(|root| root.declared_root.root_id == TARGET_ROOT_ID)
            .ok_or(BackendError::RootUnavailable)?;
        let recovered = self.recovered.load(Ordering::Acquire);
        let mut candidates_by_root = (0..request.roots.len())
            .map(|_| Ok(Vec::new()))
            .collect::<Vec<_>>();
        if recovered {
            candidates_by_root[source]
                .as_mut()
                .map_err(|error| *error)?
                .push(test_candidate("C:\\JournalSource\\old.png", NEW_USN)?);
            candidates_by_root[target]
                .as_mut()
                .map_err(|error| *error)?
                .push(test_candidate("C:\\JournalDestination\\new.png", NEW_USN)?);
        }
        let (previous_root_index, previous_pending_index, old_usn) = if recovered {
            let pending = request
                .pending_renames
                .first()
                .ok_or(BackendError::RecordUnsupported)?;
            if pending.file_reference != vec![5; 16]
                || pending.previous_relative_path != "old.png"
                || pending.old_usn != OLD_USN
            {
                return Err(BackendError::RecordUnsupported);
            }
            (None, Some(0), pending.old_usn)
        } else {
            (Some(source), None, OLD_USN)
        };
        Ok(SharedBackendJournalPage {
            candidates_by_root,
            handoffs: vec![BackendJournalHandoff {
                previous_root_index,
                previous_pending_index,
                current_root_index: target,
                previous_absolute_path_utf16: "C:\\JournalSource\\old.png".encode_utf16().collect(),
                current_absolute_path_utf16: "C:\\JournalDestination\\new.png"
                    .encode_utf16()
                    .collect(),
                file_reference: vec![5; 16],
                old_usn,
                usn: NEW_USN,
                is_directory: false,
            }],
            pending_renames: Vec::new(),
            rename_barriers_by_root: vec![None; request.roots.len()],
            proof: Some(JournalReadProof::after_read(
                request.range.end_usn,
                true,
                request.journal,
            )?),
        })
    }
}

fn test_candidate(path: &str, usn: i64) -> Result<BackendJournalCandidate, BackendError> {
    BackendJournalCandidate::copy_from_bounded(
        VOLUME_ID,
        &path.encode_utf16().collect::<Vec<_>>(),
        None,
        &[5; 16],
        usn,
        CandidateKind::Path,
        false,
    )
}

#[derive(Clone)]
struct TargetTranslationAuthorizer {
    recovered: Arc<AtomicBool>,
}

impl CallerAuthorizer for TargetTranslationAuthorizer {
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
        let _ = self.authorize(caller, root, context)?;
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
        self.authorize(caller, root, context)
    }

    fn filter_disclosable_candidates(
        &self,
        root: &AuthorizedPinnedRoot,
        candidates: Vec<BackendJournalCandidate>,
        context: &mut RequestContext<'_>,
    ) -> Result<Vec<BackendJournalCandidate>, AuthorizerError> {
        let _ = context
            .bounded_step()
            .map_err(|_| AuthorizerError::RootUnauthorized)?;
        if !self.recovered.load(Ordering::Acquire)
            && root.declared_root.root_id == TARGET_ROOT_ID
            && !candidates.is_empty()
        {
            return Err(AuthorizerError::RootIdentityMismatch);
        }
        Ok(candidates)
    }
}

impl TargetTranslationAuthorizer {
    fn authorize(
        &self,
        caller: &CallerAuthorization,
        root: &RootAuthorization,
        context: &mut RequestContext<'_>,
    ) -> Result<AuthorizedPinnedRoot, AuthorizerError> {
        if !matches!(root.root_id.as_str(), SOURCE_ROOT_ID | TARGET_ROOT_ID) {
            return Err(AuthorizerError::RootUnauthorized);
        }
        let _ = context
            .bounded_step()
            .map_err(|_| AuthorizerError::RootIdentityMismatch)?;
        let declared = DeclaredRootAuthorization::after_access_check(caller, root)?;
        let semantics = RootNameSemanticsProof::from_pinned_directory_handles(&[
            NameComparisonSemantics::WindowsOrdinalIgnoreCase,
        ])?;
        let pinned = PinnedBrokerRoot::from_caller_pinned_handle(
            &root.volume_id,
            &root.root_identity,
            &root.canonical_root_utf16,
            semantics,
        )
        .map_err(|_| AuthorizerError::RootIdentityMismatch)?;
        AuthorizedPinnedRoot::after_identity_check(caller, &declared, pinned)
    }
}

#[derive(Clone)]
struct InProcessVolumeTransport {
    inner: Arc<InProcessVolumeTransportInner>,
}

struct InProcessVolumeTransportInner {
    service: Arc<TestService>,
    authenticated: AuthenticatedConnection,
    responses: Mutex<HashMap<u64, VecDeque<Vec<u8>>>>,
    is_closed: AtomicBool,
}

impl InProcessVolumeTransport {
    fn new(service: Arc<TestService>, authenticated: AuthenticatedConnection) -> Self {
        service
            .open_connection(&authenticated)
            .expect("open in-process volume connection");
        Self {
            inner: Arc::new(InProcessVolumeTransportInner {
                service,
                authenticated,
                responses: Mutex::new(HashMap::new()),
                is_closed: AtomicBool::new(false),
            }),
        }
    }
}

#[derive(Debug)]
struct InProcessVolumeTransportError;

impl fmt::Display for InProcessVolumeTransportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("in-process volume transport failed")
    }
}

impl std::error::Error for InProcessVolumeTransportError {}

impl sealed::Sealed for InProcessVolumeTransport {}

impl BrokerClientTransport for InProcessVolumeTransport {
    type Error = InProcessVolumeTransportError;

    fn try_send_frame(&mut self, request_frame: &[u8]) -> Result<AdapterPoll<()>, Self::Error> {
        if self.inner.is_closed.load(Ordering::Acquire) {
            return Err(InProcessVolumeTransportError);
        }
        let payload = decode_frame(request_frame).map_err(|_| InProcessVolumeTransportError)?;
        let request = decode_request(payload).map_err(|_| InProcessVolumeTransportError)?;
        let request_id = inspect_request_header(payload)
            .map_err(|_| InProcessVolumeTransportError)?
            .request_id;
        let accepted = self
            .inner
            .service
            .handle_frame(request_frame, &self.inner.authenticated)
            .map_err(|_| InProcessVolumeTransportError)?;
        let mut frames = VecDeque::from([accepted]);
        let accepted_read = match request {
            BrokerRequest::ReadRange { request, .. } => {
                Some((request.caller.client_instance, request_id))
            }
            BrokerRequest::ReadVolume { request, .. } => {
                Some((request.caller.client_instance, request_id))
            }
            _ => None,
        };
        if let Some((client_instance, request_id)) = accepted_read {
            frames.push_back(
                self.inner
                    .service
                    .execute_accepted_read_frame(
                        request_id,
                        client_instance,
                        &self.inner.authenticated,
                    )
                    .map_err(|_| InProcessVolumeTransportError)?,
            );
        }
        self.inner
            .responses
            .lock()
            .map_err(|_| InProcessVolumeTransportError)?
            .insert(request_id, frames);
        Ok(AdapterPoll::Ready(()))
    }

    fn poll_response(&mut self, request_id: u64) -> Result<AdapterPoll<Vec<u8>>, Self::Error> {
        let mut responses = self
            .inner
            .responses
            .lock()
            .map_err(|_| InProcessVolumeTransportError)?;
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
        self.inner
            .responses
            .lock()
            .map_err(|_| InProcessVolumeTransportError)?
            .remove(&request_id);
        Ok(AdapterPoll::Ready(()))
    }

    fn begin_abandon(
        &mut self,
        target_request_id: u64,
        cancel_request: Option<(u64, Vec<u8>)>,
        close_connection: bool,
    ) -> Result<AdapterPoll<()>, Self::Error> {
        let _ = self.try_discard(target_request_id)?;
        if close_connection {
            return self.begin_close();
        }
        if let Some((cancel_request_id, frame)) = cancel_request {
            let _ = self.try_send_frame(&frame)?;
            let _ = self.try_discard(cancel_request_id)?;
        }
        Ok(AdapterPoll::Ready(()))
    }

    fn poll_abandon(&mut self, _target_request_id: u64) -> Result<AdapterPoll<()>, Self::Error> {
        Ok(AdapterPoll::Ready(()))
    }

    fn begin_close(&mut self) -> Result<AdapterPoll<()>, Self::Error> {
        self.inner.is_closed.store(true, Ordering::Release);
        self.inner.service.disconnect(&self.inner.authenticated);
        self.inner
            .responses
            .lock()
            .map_err(|_| InProcessVolumeTransportError)?
            .clear();
        Ok(AdapterPoll::Ready(()))
    }

    fn poll_close(&mut self) -> Result<AdapterPoll<()>, Self::Error> {
        Ok(AdapterPoll::Ready(()))
    }
}
