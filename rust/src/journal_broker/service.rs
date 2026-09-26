mod active;
mod contracts;
#[cfg(test)]
pub(super) mod integration_test_support;
#[cfg(windows)]
mod windows_security;

use std::time::{Duration, Instant};

#[cfg(test)]
pub(super) use active::MAX_ACTIVE_REQUESTS_PER_CONNECTION;
pub(super) use active::{
    AcceptedJournalRead, ActiveRequestError, ActiveRequestKey, ActiveRequestRegistry,
    RecordedTerminal, RequestTerminal,
};
pub(super) use contracts::{
    AuthenticatedConnection, AuthenticatedPipeFacts, AuthorizedPinnedRoot, AuthorizerError,
    BackendError, BackendJournalCandidate, BackendJournalHandoff, BackendPendingRename,
    BackendPendingRenameCarry, BoundedJournalPageBuilder, CallerAuthorization, CallerAuthorizer,
    DeclaredRootAuthorization, ExistingJournalBackend, ExistingJournalMetadata,
    ExistingJournalState, ExistingJournalVolumeRead, JournalRange, JournalReadLimits,
    JournalReadProof, PinnedBrokerRoot, SharedBackendJournalPage,
};
#[cfg(all(windows, feature = "broker-acceptance-client"))]
pub(in crate::journal_broker) use windows_security::describe_disposable_root_for_acceptance;
#[cfg(all(windows, feature = "broker-acceptance-client"))]
pub(in crate::journal_broker) use windows_security::open_directory_handle;
#[cfg(windows)]
pub(in crate::journal_broker) use windows_security::{
    ClientBinaryAdmission, ConnectedPipeAdmission, OwnedFileHandle, PinnedRootState,
    WindowsBrokerSecurity, WindowsPipeAuthorizer, WindowsRootRegistrationBudget,
    describe_client_root,
};

use super::framing::{decode_frame, encode_frame};
use super::wire::{
    BrokerCandidate, BrokerFailure, BrokerFailureCode, BrokerRequest, BrokerResponse, CallerClaim,
    CandidateKind, CandidateScope, JournalCapability, QueryJournalRequest, ReadJournalRangeRequest,
    ReadJournalVolumeRequest, RegisterRootRequest, ResponseBinding, RootAuthorization,
    RootRelativeScope, SharedJournalHandoff, SharedJournalPendingRename, SharedJournalRootOutcome,
    WireError, decode_request, encode_response, inspect_request_header, normalize_backend_path,
    scope_under_root, validate_root_path,
};

const MAX_BACKEND_STEP: Duration = Duration::from_millis(250);

pub(super) struct RequestContext<'a> {
    deadline: Instant,
    registry: &'a ActiveRequestRegistry,
    key: ActiveRequestKey,
    bounded_steps: usize,
}

impl RequestContext<'_> {
    pub(super) fn bounded_step(&mut self) -> Result<Duration, BackendError> {
        self.check()?;
        self.bounded_steps = self
            .bounded_steps
            .checked_add(1)
            .ok_or(BackendError::TimedOut)?;
        Ok(self.remaining().min(MAX_BACKEND_STEP))
    }

    pub(super) fn check(&self) -> Result<(), BackendError> {
        match self.registry.check(self.key, Instant::now()) {
            Ok(()) => Ok(()),
            Err(ActiveRequestError::TimedOut) => Err(BackendError::TimedOut),
            Err(_) => Err(BackendError::Cancelled),
        }
    }

    pub(super) fn remaining(&self) -> Duration {
        self.deadline.saturating_duration_since(Instant::now())
    }
}

pub(super) struct BrokerService<Backend, Authorizer> {
    backend: Backend,
    authorizer: Authorizer,
    active: ActiveRequestRegistry,
}

pub(super) struct TerminalFrame {
    pub(super) request_id: u64,
    pub(super) frame: Vec<u8>,
}

pub(super) struct TerminalFrameBatch {
    pub(super) frames: Vec<TerminalFrame>,
    pub(super) must_close: bool,
}

impl<Backend, Authorizer> BrokerService<Backend, Authorizer>
where
    Backend: ExistingJournalBackend,
    Authorizer: CallerAuthorizer,
{
    pub(super) fn new(backend: Backend, authorizer: Authorizer) -> Self {
        Self {
            backend,
            authorizer,
            active: ActiveRequestRegistry::default(),
        }
    }

    pub(super) fn handle_frame(
        &self,
        frame: &[u8],
        authenticated: &AuthenticatedConnection,
    ) -> Result<Vec<u8>, BrokerFailure> {
        let response = match decode_frame(frame) {
            Ok(payload) => match decode_request(payload) {
                Ok(request) => self.handle_request(request, authenticated),
                Err(error) => malformed_response(payload, error),
            },
            Err(_) => failure_response(0, None, BrokerFailureCode::MalformedFrame),
        };
        let response_payload = encode_response(&response)
            .map_err(|_| BrokerFailure::new(BrokerFailureCode::Internal))?;
        encode_frame(&response_payload).map_err(|_| BrokerFailure::new(BrokerFailureCode::Internal))
    }

    pub(super) fn disconnect(&self, authenticated: &AuthenticatedConnection) {
        self.active.disconnect(
            authenticated.connection_id,
            authenticated.connection_generation,
        );
    }

    pub(super) fn open_connection(
        &self,
        authenticated: &AuthenticatedConnection,
    ) -> Result<(), BrokerFailure> {
        self.active
            .open_connection(
                authenticated.connection_id,
                authenticated.connection_generation,
            )
            .map_err(map_active_failure)
    }

    pub(super) fn reap_authorizer(&self, now: Instant) {
        self.authorizer.reap_expired(now);
    }

    fn handle_request(
        &self,
        request: BrokerRequest,
        authenticated: &AuthenticatedConnection,
    ) -> BrokerResponse {
        match request {
            BrokerRequest::RegisterRoot {
                request_id,
                request,
            } => self
                .register_root(request_id, &request, authenticated)
                .unwrap_or_else(|failure| {
                    failure_response(
                        request_id,
                        Some(request.caller.client_instance),
                        failure.code,
                    )
                }),
            BrokerRequest::QueryJournal {
                request_id,
                request,
            } => self
                .query_journal(request_id, &request, authenticated)
                .unwrap_or_else(|failure| {
                    failure_response(
                        request_id,
                        Some(request.caller.client_instance),
                        failure.code,
                    )
                }),
            BrokerRequest::ReadRange {
                request_id,
                request,
            } => self
                .accept_read_range(request_id, &request, authenticated)
                .unwrap_or_else(|failure| {
                    failure_response(
                        request_id,
                        Some(request.caller.client_instance),
                        failure.code,
                    )
                }),
            BrokerRequest::ReadVolume {
                request_id,
                request,
            } => self
                .accept_read_volume(request_id, &request, authenticated)
                .unwrap_or_else(|failure| {
                    failure_response(
                        request_id,
                        Some(request.caller.client_instance),
                        failure.code,
                    )
                }),
            BrokerRequest::Cancel {
                request_id,
                caller,
                target_request_id,
            } => self
                .cancel(request_id, &caller, target_request_id, authenticated)
                .unwrap_or_else(|failure| {
                    failure_response(request_id, Some(caller.client_instance), failure.code)
                }),
        }
    }

    pub(super) fn execute_accepted_read_frame(
        &self,
        request_id: u64,
        client_instance: [u8; 16],
        authenticated: &AuthenticatedConnection,
    ) -> Result<Vec<u8>, BrokerFailure> {
        let key = ActiveRequestKey {
            connection_id: authenticated.connection_id,
            connection_generation: authenticated.connection_generation,
            authentication_namespace: authenticated.opaque_token_binding,
            client_instance,
            request_id,
        };
        let response = match self.active.begin_execution(key, Instant::now()) {
            Ok(accepted) => {
                let result = match &accepted.request {
                    AcceptedJournalRead::Root(request) => self.execute_read_range(
                        request_id,
                        key,
                        request,
                        accepted.deadline,
                        authenticated,
                    ),
                    AcceptedJournalRead::Volume(request) => self.execute_read_volume(
                        request_id,
                        key,
                        request,
                        accepted.deadline,
                        authenticated,
                    ),
                };
                match self.active.finish(key, Instant::now()) {
                    Ok(RequestTerminal::Cancelled) => failure_response(
                        request_id,
                        Some(client_instance),
                        BrokerFailureCode::Cancelled,
                    ),
                    Ok(RequestTerminal::TimedOut) => failure_response(
                        request_id,
                        Some(client_instance),
                        BrokerFailureCode::TimedOut,
                    ),
                    Ok(RequestTerminal::Completed) => result.unwrap_or_else(|failure| {
                        failure_response(request_id, Some(client_instance), failure.code)
                    }),
                    Ok(RequestTerminal::Aborted) => failure_response(
                        request_id,
                        Some(client_instance),
                        BrokerFailureCode::RequestNotActive,
                    ),
                    Err(_) => failure_response(
                        request_id,
                        Some(client_instance),
                        BrokerFailureCode::RequestNotActive,
                    ),
                }
            }
            Err(error) => failure_response(
                request_id,
                Some(client_instance),
                map_active_failure(error).code,
            ),
        };
        let payload = encode_response(&response)
            .map_err(|_| BrokerFailure::new(BrokerFailureCode::Internal))?;
        encode_frame(&payload).map_err(|_| BrokerFailure::new(BrokerFailureCode::Internal))
    }

    pub(super) fn abort_accepted_read(
        &self,
        request_id: u64,
        client_instance: [u8; 16],
        authenticated: &AuthenticatedConnection,
    ) -> Result<TerminalFrame, BrokerFailure> {
        let terminal = self
            .active
            .abort_accepted(
                ActiveRequestKey {
                    connection_id: authenticated.connection_id,
                    connection_generation: authenticated.connection_generation,
                    authentication_namespace: authenticated.opaque_token_binding,
                    client_instance,
                    request_id,
                },
                Instant::now(),
            )
            .map_err(map_active_failure)?;
        encode_recorded_terminal(terminal)
    }

    pub(super) fn reap_accepted_reads(
        &self,
        authenticated: &AuthenticatedConnection,
    ) -> Result<TerminalFrameBatch, BrokerFailure> {
        let drain = self
            .active
            .reap_accepted(
                authenticated.connection_id,
                authenticated.connection_generation,
                Instant::now(),
            )
            .map_err(map_active_failure)?;
        let frames = drain
            .terminals
            .into_iter()
            .map(encode_recorded_terminal)
            .collect::<Result<_, _>>()?;
        Ok(TerminalFrameBatch {
            frames,
            must_close: drain.must_close,
        })
    }

    fn query_journal(
        &self,
        request_id: u64,
        request: &QueryJournalRequest,
        authenticated: &AuthenticatedConnection,
    ) -> Result<BrokerResponse, BrokerFailure> {
        let caller = self
            .authorizer
            .verify_claim(authenticated, &request.caller)
            .map_err(map_authorizer_failure)?;
        let (key, deadline) = self.begin_request(request_id, &caller, request.timeout_ms)?;
        let result = (|| {
            let mut context = self.context(key, deadline);
            let root = self
                .authorizer
                .authorize_registered_root(
                    &caller,
                    &request.root,
                    request.root_capability,
                    &mut context,
                )
                .map_err(map_authorizer_failure)?;
            validate_authorized_root_facts(&caller, &request.root, &root)?;
            context.bounded_step().map_err(map_backend_failure)?;
            let journal = self
                .backend
                .query_existing_journal(&root, &mut context)
                .map_err(map_backend_failure)?;
            context.check().map_err(map_backend_failure)?;
            let binding = ResponseBinding::from_request(&request.caller, &request.root);
            match journal {
                ExistingJournalState::Supported(metadata) => {
                    let metadata = metadata.validate().map_err(map_backend_failure)?;
                    Ok(BrokerResponse::Journal {
                        request_id,
                        binding,
                        capability: JournalCapability::Supported,
                        journal_id: Some(metadata.journal_id),
                        first_usn: Some(metadata.first_usn),
                        next_usn: Some(metadata.next_usn),
                    })
                }
                ExistingJournalState::LiveOnly => Ok(BrokerResponse::Journal {
                    request_id,
                    binding,
                    capability: JournalCapability::LiveOnly,
                    journal_id: None,
                    first_usn: None,
                    next_usn: None,
                }),
            }
        })();
        match self.active.finish(key, Instant::now()) {
            Ok(RequestTerminal::Completed) => result,
            Ok(RequestTerminal::Cancelled) => Err(BrokerFailure::new(BrokerFailureCode::Cancelled)),
            Ok(RequestTerminal::TimedOut) => Err(BrokerFailure::new(BrokerFailureCode::TimedOut)),
            Ok(RequestTerminal::Aborted) | Err(_) => {
                Err(BrokerFailure::new(BrokerFailureCode::RequestNotActive))
            }
        }
    }

    fn register_root(
        &self,
        request_id: u64,
        request: &RegisterRootRequest,
        authenticated: &AuthenticatedConnection,
    ) -> Result<BrokerResponse, BrokerFailure> {
        let caller = self
            .authorizer
            .verify_claim(authenticated, &request.caller)
            .map_err(map_authorizer_failure)?;
        let (key, deadline) = self.begin_request(request_id, &caller, request.timeout_ms)?;
        let result = (|| {
            let mut context = self.context(key, deadline);
            let root_capability = self
                .authorizer
                .register_root(
                    &caller,
                    &request.root,
                    request.client_root_handle,
                    &mut context,
                )
                .map_err(map_authorizer_failure)?;
            root_capability
                .validate()
                .map_err(|_| BrokerFailure::new(BrokerFailureCode::RootUnauthorized))?;
            context.check().map_err(map_backend_failure)?;
            Ok(BrokerResponse::RootRegistered {
                request_id,
                binding: ResponseBinding::from_request(&request.caller, &request.root),
                root_capability,
            })
        })();
        match self.active.finish(key, Instant::now()) {
            Ok(RequestTerminal::Completed) => result,
            Ok(RequestTerminal::Cancelled) => Err(BrokerFailure::new(BrokerFailureCode::Cancelled)),
            Ok(RequestTerminal::TimedOut) => Err(BrokerFailure::new(BrokerFailureCode::TimedOut)),
            Ok(RequestTerminal::Aborted) | Err(_) => {
                Err(BrokerFailure::new(BrokerFailureCode::RequestNotActive))
            }
        }
    }

    fn accept_read_range(
        &self,
        request_id: u64,
        request: &ReadJournalRangeRequest,
        authenticated: &AuthenticatedConnection,
    ) -> Result<BrokerResponse, BrokerFailure> {
        let caller = self
            .authorizer
            .verify_claim(authenticated, &request.caller)
            .map_err(map_authorizer_failure)?;
        let key = self.active_key(request_id, &caller);
        let deadline = deadline_from_timeout(request.timeout_ms)?;
        self.active
            .accept_read(key, deadline, request.clone())
            .map_err(map_active_failure)?;
        Ok(BrokerResponse::ReadRangeAccepted {
            request_id,
            binding: ResponseBinding::from_request(&request.caller, &request.root),
            journal_id: request.journal_id,
            requested_start_usn: request.start_usn,
            requested_end_usn: request.end_usn,
            max_records: request.max_records,
            max_evidence_bytes: request.max_evidence_bytes,
        })
    }

    fn accept_read_volume(
        &self,
        request_id: u64,
        request: &ReadJournalVolumeRequest,
        authenticated: &AuthenticatedConnection,
    ) -> Result<BrokerResponse, BrokerFailure> {
        let caller = self
            .authorizer
            .verify_claim(authenticated, &request.caller)
            .map_err(map_authorizer_failure)?;
        let key = self.active_key(request_id, &caller);
        let deadline = deadline_from_timeout(request.timeout_ms)?;
        self.active
            .accept_volume_read(key, deadline, request.clone())
            .map_err(map_active_failure)?;
        Ok(BrokerResponse::ReadVolumeAccepted {
            request_id,
            client_instance: request.caller.client_instance,
            volume_id: request.roots[0].root.volume_id.clone(),
            journal_id: request.journal_id,
            requested_start_usn: request.minimum_start_usn(),
            requested_end_usn: request.end_usn,
            root_count: request.roots.len() as u8,
            max_records: request.max_records,
            max_evidence_bytes: request.max_evidence_bytes,
        })
    }

    fn execute_read_volume(
        &self,
        request_id: u64,
        key: ActiveRequestKey,
        request: &ReadJournalVolumeRequest,
        deadline: Instant,
        authenticated: &AuthenticatedConnection,
    ) -> Result<BrokerResponse, BrokerFailure> {
        let caller = self
            .authorizer
            .verify_claim(authenticated, &request.caller)
            .map_err(map_authorizer_failure)?;
        if key != self.active_key(request_id, &caller) {
            return Err(BrokerFailure::new(BrokerFailureCode::CallerRejected));
        }
        let mut context = self.context(key, deadline);
        let mut outcomes = request
            .roots
            .iter()
            .map(|root| {
                Some(shared_failure_outcome(
                    &request.caller,
                    root,
                    BrokerFailureCode::Internal,
                ))
            })
            .collect::<Vec<_>>();
        let mut admitted = Vec::new();
        let mut before = None;
        let mut handoffs = Vec::new();
        let mut durable_pending_renames = Vec::new();
        for (index, requested) in request.roots.iter().enumerate() {
            context.check().map_err(map_backend_failure)?;
            let root = match self.authorizer.authorize_registered_root(
                &caller,
                &requested.root,
                requested.root_capability,
                &mut context,
            ) {
                Ok(root) => root,
                Err(error) => {
                    outcomes[index] = Some(shared_failure_outcome(
                        &request.caller,
                        requested,
                        map_authorizer_failure(error).code,
                    ));
                    continue;
                }
            };
            if let Err(failure) = validate_authorized_root_facts(&caller, &requested.root, &root) {
                outcomes[index] = Some(shared_failure_outcome(
                    &request.caller,
                    requested,
                    failure.code,
                ));
                continue;
            }
            context.bounded_step().map_err(map_backend_failure)?;
            let metadata = match self.backend.query_existing_journal(&root, &mut context) {
                Ok(ExistingJournalState::Supported(metadata)) => match metadata.validate() {
                    Ok(metadata) => metadata,
                    Err(error) => {
                        outcomes[index] = Some(shared_failure_outcome(
                            &request.caller,
                            requested,
                            map_backend_failure(error).code,
                        ));
                        continue;
                    }
                },
                Ok(ExistingJournalState::LiveOnly) => {
                    outcomes[index] = Some(shared_failure_outcome(
                        &request.caller,
                        requested,
                        BrokerFailureCode::JournalUnavailable,
                    ));
                    continue;
                }
                Err(error) => {
                    outcomes[index] = Some(shared_failure_outcome(
                        &request.caller,
                        requested,
                        map_backend_failure(error).code,
                    ));
                    continue;
                }
            };
            if metadata.journal_id != request.journal_id
                || requested.start_usn < metadata.first_usn
                || request.end_usn > metadata.next_usn
            {
                outcomes[index] = Some(shared_failure_outcome(
                    &request.caller,
                    requested,
                    BrokerFailureCode::JournalDiscontinuous,
                ));
                continue;
            }
            before.get_or_insert(metadata);
            outcomes[index] = None;
            admitted.push((index, root));
        }

        if !admitted.is_empty() {
            let metadata = before.ok_or_else(|| BrokerFailure::new(BrokerFailureCode::Internal))?;
            let range = JournalRange {
                start_usn: admitted
                    .iter()
                    .map(|(index, _)| request.roots[*index].start_usn)
                    .min()
                    .ok_or_else(|| BrokerFailure::new(BrokerFailureCode::Internal))?,
                end_usn: request.end_usn,
            };
            let roots = admitted
                .iter()
                .map(|(_, root)| root.clone())
                .collect::<Vec<_>>();
            let root_start_usns = admitted
                .iter()
                .map(|(index, _)| request.roots[*index].start_usn)
                .collect::<Vec<_>>();
            let mut pending_renames = Vec::with_capacity(request.pending_renames.len());
            let mut pending_authorization_failures = Vec::new();
            for pending in &request.pending_renames {
                let declared_admitted_index = admitted.iter().position(|(request_index, _)| {
                    request.roots[*request_index].root == pending.root
                });
                let root = if let Some(capability) = pending.root_capability {
                    match self.authorizer.authorize_registered_root(
                        &caller,
                        &pending.root,
                        capability,
                        &mut context,
                    ) {
                        Ok(root) => {
                            match validate_authorized_root_facts(&caller, &pending.root, &root) {
                                Ok(()) => Some(root),
                                Err(failure) => {
                                    if let Some(index) = declared_admitted_index {
                                        pending_authorization_failures.push((index, failure.code));
                                    }
                                    None
                                }
                            }
                        }
                        Err(error) => {
                            if let Some(index) = declared_admitted_index {
                                pending_authorization_failures
                                    .push((index, map_authorizer_failure(error).code));
                            }
                            None
                        }
                    }
                } else {
                    None
                };
                pending_renames.push(BackendPendingRenameCarry {
                    read_root_index: declared_admitted_index,
                    root,
                    file_reference: pending.file_reference.clone(),
                    old_usn: pending.old_usn,
                    previous_relative_path: pending.previous_relative_path.clone(),
                    is_directory: pending.is_directory,
                });
            }
            context.bounded_step().map_err(map_backend_failure)?;
            let shared = self
                .backend
                .read_existing_journal_volume_page(
                    ExistingJournalVolumeRead {
                        roots: &roots,
                        root_start_usns: &root_start_usns,
                        pending_renames: &pending_renames,
                        journal: metadata,
                        range,
                        limits: JournalReadLimits {
                            max_records: request.max_records as usize,
                            max_evidence_bytes: request.max_evidence_bytes as usize,
                        },
                    },
                    &mut context,
                )
                .map_err(map_backend_failure)?;
            validate_shared_backend_page(metadata, range, &shared, admitted.len())?;
            let mut failed_admitted = std::collections::BTreeSet::new();
            for (admitted_index, code) in pending_authorization_failures {
                failed_admitted.insert(admitted_index);
                let request_index = admitted[admitted_index].0;
                outcomes[request_index] = Some(shared_failure_outcome(
                    &request.caller,
                    &request.roots[request_index],
                    code,
                ));
            }
            for (admitted_index, ((index, root), candidates)) in
                admitted.iter().zip(shared.candidates_by_root).enumerate()
            {
                if failed_admitted.contains(&admitted_index) {
                    continue;
                }
                let requested = &request.roots[*index];
                let candidates = match candidates {
                    Ok(candidates) => candidates,
                    Err(error) => {
                        failed_admitted.insert(admitted_index);
                        outcomes[*index] = Some(shared_failure_outcome(
                            &request.caller,
                            requested,
                            map_backend_failure(error).code,
                        ));
                        continue;
                    }
                };
                let Some(proof) = shared.proof else {
                    return Err(BrokerFailure::new(BrokerFailureCode::Internal));
                };
                if proof.covered_until_usn <= requested.start_usn {
                    failed_admitted.insert(admitted_index);
                    outcomes[*index] = Some(shared_failure_outcome(
                        &request.caller,
                        requested,
                        BrokerFailureCode::EvidenceLimitExceeded,
                    ));
                    continue;
                }
                let candidates = candidates
                    .into_iter()
                    .filter(|candidate| candidate.usn >= requested.start_usn)
                    .collect();
                let disclosable = match self.authorizer.filter_disclosable_candidates(
                    root,
                    candidates,
                    &mut context,
                ) {
                    Ok(candidates) => candidates,
                    Err(error) => {
                        failed_admitted.insert(admitted_index);
                        outcomes[*index] = Some(shared_failure_outcome(
                            &request.caller,
                            requested,
                            map_authorizer_failure(error).code,
                        ));
                        continue;
                    }
                };
                let candidates = match constrain_candidates(root, disclosable) {
                    Ok(candidates) => candidates,
                    Err(failure) => {
                        failed_admitted.insert(admitted_index);
                        outcomes[*index] = Some(shared_failure_outcome(
                            &request.caller,
                            requested,
                            failure.code,
                        ));
                        continue;
                    }
                };
                outcomes[*index] = Some(SharedJournalRootOutcome {
                    binding: ResponseBinding::from_request(&request.caller, &requested.root),
                    requested_start_usn: requested.start_usn,
                    covered_until_usn: Some(proof.covered_until_usn),
                    is_complete: proof.is_complete,
                    candidates,
                    failure: None,
                });
            }
            let mut translation_barrier = None;
            if let Some(proof) = shared.proof {
                let translated_handoffs = translate_shared_handoffs(
                    &self.authorizer,
                    &request.caller,
                    request,
                    &admitted,
                    &pending_renames,
                    &shared.handoffs,
                    proof.covered_until_usn,
                    &mut context,
                )?;
                handoffs = translated_handoffs.handoffs;
                durable_pending_renames = translated_handoffs.pending_renames;
                translation_barrier = translated_handoffs.barrier;
                for (admitted_index, code) in translated_handoffs.failures {
                    failed_admitted.insert(admitted_index);
                    let request_index = admitted[admitted_index].0;
                    outcomes[request_index] = Some(shared_failure_outcome(
                        &request.caller,
                        &request.roots[request_index],
                        code,
                    ));
                }
                let (translated, failures, barrier) = translate_shared_pending_renames(
                    &self.authorizer,
                    &request.caller,
                    request,
                    &admitted,
                    &shared.pending_renames,
                    proof.covered_until_usn,
                    &mut context,
                )?;
                durable_pending_renames.extend(translated);
                translation_barrier = match (translation_barrier, barrier) {
                    (Some(left), Some(right)) => Some(left.min(right)),
                    (left, right) => left.or(right),
                };
                for (admitted_index, code) in failures {
                    failed_admitted.insert(admitted_index);
                    let request_index = admitted[admitted_index].0;
                    outcomes[request_index] = Some(shared_failure_outcome(
                        &request.caller,
                        &request.roots[request_index],
                        code,
                    ));
                }
            }
            let failed_root_barrier = failed_admitted
                .iter()
                .filter_map(|index| shared.rename_barriers_by_root[*index])
                .min();
            let barrier = match (failed_root_barrier, translation_barrier) {
                (Some(left), Some(right)) => Some(left.min(right)),
                (left, right) => left.or(right),
            };
            if let Some(barrier) = barrier {
                for (admitted_index, (request_index, _)) in admitted.iter().enumerate() {
                    if failed_admitted.contains(&admitted_index) {
                        continue;
                    }
                    let requested = &request.roots[*request_index];
                    let Some(outcome) = outcomes[*request_index].as_mut() else {
                        return Err(BrokerFailure::new(BrokerFailureCode::Internal));
                    };
                    if barrier <= requested.start_usn {
                        *outcome = shared_failure_outcome(
                            &request.caller,
                            requested,
                            BrokerFailureCode::EvidenceLimitExceeded,
                        );
                        continue;
                    }
                    outcome.covered_until_usn = Some(
                        outcome
                            .covered_until_usn
                            .map_or(barrier, |covered| covered.min(barrier)),
                    );
                    outcome.is_complete = false;
                    outcome
                        .candidates
                        .retain(|candidate| candidate.usn < barrier);
                }
                handoffs.retain(|handoff| handoff.usn < barrier);
                durable_pending_renames.retain(|pending| pending.old_usn < barrier);
            }
        }
        let outcomes = outcomes
            .into_iter()
            .collect::<Option<Vec<_>>>()
            .ok_or_else(|| BrokerFailure::new(BrokerFailureCode::Internal))?;
        validate_shared_response_limits(
            &outcomes,
            &handoffs,
            &durable_pending_renames,
            request.max_records as usize,
            request.max_evidence_bytes as usize,
        )?;
        context.check().map_err(map_backend_failure)?;
        Ok(BrokerResponse::ReadVolume {
            request_id,
            client_instance: request.caller.client_instance,
            volume_id: request.roots[0].root.volume_id.clone(),
            journal_id: request.journal_id,
            requested_end_usn: request.end_usn,
            max_records: request.max_records,
            max_evidence_bytes: request.max_evidence_bytes,
            outcomes,
            handoffs,
            pending_renames: durable_pending_renames,
        })
    }

    fn execute_read_range(
        &self,
        request_id: u64,
        key: ActiveRequestKey,
        request: &ReadJournalRangeRequest,
        deadline: Instant,
        authenticated: &AuthenticatedConnection,
    ) -> Result<BrokerResponse, BrokerFailure> {
        let caller = self
            .authorizer
            .verify_claim(authenticated, &request.caller)
            .map_err(map_authorizer_failure)?;
        if key != self.active_key(request_id, &caller) {
            return Err(BrokerFailure::new(BrokerFailureCode::CallerRejected));
        }
        let mut context = self.context(key, deadline);
        let root = self
            .authorizer
            .authorize_registered_root(
                &caller,
                &request.root,
                request.root_capability,
                &mut context,
            )
            .map_err(map_authorizer_failure)?;
        validate_authorized_root_facts(&caller, &request.root, &root)?;
        context.bounded_step().map_err(map_backend_failure)?;
        let before = match self
            .backend
            .query_existing_journal(&root, &mut context)
            .map_err(map_backend_failure)?
        {
            ExistingJournalState::Supported(metadata) => {
                metadata.validate().map_err(map_backend_failure)?
            }
            ExistingJournalState::LiveOnly => {
                return Err(BrokerFailure::new(BrokerFailureCode::JournalUnavailable));
            }
        };
        validate_requested_range(request, before)?;
        let range = JournalRange {
            start_usn: request.start_usn,
            end_usn: request.end_usn,
        };
        let mut builder = BoundedJournalPageBuilder::new(
            range,
            JournalReadLimits {
                max_records: request.max_records as usize,
                max_evidence_bytes: request.max_evidence_bytes as usize,
            },
        );
        context.bounded_step().map_err(map_backend_failure)?;
        let proof = self
            .backend
            .read_existing_journal_page(&root, before, range, &mut builder, &mut context)
            .map_err(map_backend_failure)?;
        context.check().map_err(map_backend_failure)?;
        let page = builder.finish(before, proof).map_err(map_backend_failure)?;
        let disclosable = self
            .authorizer
            .filter_disclosable_candidates(&root, page.candidates, &mut context)
            .map_err(map_authorizer_failure)?;
        context.check().map_err(map_backend_failure)?;
        let candidates = constrain_candidates(&root, disclosable)?;
        Ok(BrokerResponse::ReadRange {
            request_id,
            binding: ResponseBinding::from_request(&request.caller, &request.root),
            journal_id: before.journal_id,
            requested_start_usn: request.start_usn,
            requested_end_usn: request.end_usn,
            max_records: request.max_records,
            max_evidence_bytes: request.max_evidence_bytes,
            covered_until_usn: page.proof.covered_until_usn,
            is_complete: page.proof.is_complete,
            candidates,
        })
    }

    fn cancel(
        &self,
        request_id: u64,
        caller: &CallerClaim,
        target_request_id: u64,
        authenticated: &AuthenticatedConnection,
    ) -> Result<BrokerResponse, BrokerFailure> {
        let caller = self
            .authorizer
            .verify_claim(authenticated, caller)
            .map_err(map_authorizer_failure)?;
        let _control = self
            .active
            .begin_control(self.active_key(request_id, &caller))
            .map_err(map_active_failure)?;
        self.active
            .cancel(ActiveRequestKey {
                connection_id: caller.connection_id,
                connection_generation: caller.connection_generation,
                authentication_namespace: caller.opaque_namespace,
                client_instance: caller.client_instance,
                request_id: target_request_id,
            })
            .map_err(map_active_failure)?;
        Ok(BrokerResponse::Cancelled {
            request_id,
            client_instance: caller.client_instance,
            target_request_id,
        })
    }

    fn begin_request(
        &self,
        request_id: u64,
        caller: &CallerAuthorization,
        timeout_ms: u32,
    ) -> Result<(ActiveRequestKey, Instant), BrokerFailure> {
        let deadline = deadline_from_timeout(timeout_ms)?;
        let key = self.active_key(request_id, caller);
        self.active
            .begin_query(key, deadline)
            .map_err(map_active_failure)?;
        Ok((key, deadline))
    }

    fn active_key(&self, request_id: u64, caller: &CallerAuthorization) -> ActiveRequestKey {
        ActiveRequestKey {
            connection_id: caller.connection_id,
            connection_generation: caller.connection_generation,
            authentication_namespace: caller.opaque_namespace,
            client_instance: caller.client_instance,
            request_id,
        }
    }

    fn context<'a>(&'a self, key: ActiveRequestKey, deadline: Instant) -> RequestContext<'a> {
        RequestContext {
            deadline,
            registry: &self.active,
            key,
            bounded_steps: 0,
        }
    }
}

fn validate_authorized_root_facts(
    caller: &CallerAuthorization,
    root: &RootAuthorization,
    authorized: &AuthorizedPinnedRoot,
) -> Result<(), BrokerFailure> {
    if authorized.caller.connection_id != caller.connection_id
        || authorized.caller.connection_generation != caller.connection_generation
        || authorized.caller.client_instance != caller.client_instance
        || authorized.caller.opaque_namespace != caller.opaque_namespace
        || authorized.declared_root != *root
    {
        return Err(BrokerFailure::new(BrokerFailureCode::RootIdentityMismatch));
    }
    validate_pinned_root(root, &authorized.pinned_root)
}

fn validate_pinned_root(
    declared: &RootAuthorization,
    pinned: &PinnedBrokerRoot,
) -> Result<(), BrokerFailure> {
    if pinned.volume_id != declared.volume_id {
        return Err(BrokerFailure::new(BrokerFailureCode::VolumeMismatch));
    }
    if pinned.root_identity != declared.root_identity {
        return Err(BrokerFailure::new(BrokerFailureCode::RootIdentityMismatch));
    }
    let declared_path = validate_root_path(&declared.canonical_root_utf16)
        .map_err(|_| BrokerFailure::new(BrokerFailureCode::RootIdentityMismatch))?;
    let pinned_path = validate_root_path(&pinned.canonical_root_utf16)
        .map_err(|_| BrokerFailure::new(BrokerFailureCode::RootIdentityMismatch))?;
    let comparisons = pinned.name_semantics.components();
    if scope_under_root(&declared_path, &pinned_path, comparisons)
        .map_err(|_| BrokerFailure::new(BrokerFailureCode::RootIdentityMismatch))?
        != Some(RootRelativeScope::Root)
        || scope_under_root(&pinned_path, &declared_path, comparisons)
            .map_err(|_| BrokerFailure::new(BrokerFailureCode::RootIdentityMismatch))?
            != Some(RootRelativeScope::Root)
    {
        return Err(BrokerFailure::new(BrokerFailureCode::RootIdentityMismatch));
    }
    Ok(())
}

fn validate_requested_range(
    request: &ReadJournalRangeRequest,
    metadata: ExistingJournalMetadata,
) -> Result<(), BrokerFailure> {
    if metadata.journal_id != request.journal_id
        || request.start_usn < metadata.first_usn
        || request.end_usn > metadata.next_usn
    {
        return Err(BrokerFailure::new(BrokerFailureCode::JournalDiscontinuous));
    }
    Ok(())
}

fn deadline_from_timeout(timeout_ms: u32) -> Result<Instant, BrokerFailure> {
    Instant::now()
        .checked_add(Duration::from_millis(u64::from(timeout_ms)))
        .ok_or_else(|| BrokerFailure::new(BrokerFailureCode::TimedOut))
}

fn constrain_candidates(
    root: &AuthorizedPinnedRoot,
    backend_candidates: Vec<BackendJournalCandidate>,
) -> Result<Vec<BrokerCandidate>, BrokerFailure> {
    let canonical_root =
        contracts::normalized_root(&root.pinned_root).map_err(map_backend_failure)?;
    let mut candidates = Vec::with_capacity(backend_candidates.len());
    for candidate in backend_candidates {
        if candidate.volume_id != root.pinned_root.volume_id {
            return Err(BrokerFailure::new(BrokerFailureCode::VolumeMismatch));
        }
        let current = normalize_backend_path(&candidate.absolute_path_utf16)
            .map_err(|_| BrokerFailure::new(BrokerFailureCode::RecordUnsupported))?;
        let current_scope = scope_under_root(
            &current,
            &canonical_root,
            root.pinned_root.name_semantics.components(),
        )
        .map_err(|_| BrokerFailure::new(BrokerFailureCode::RootIdentityMismatch))?;
        let previous_scope = candidate
            .previous_absolute_path_utf16
            .as_ref()
            .map(|path| {
                normalize_backend_path(path)
                    .and_then(|path| {
                        scope_under_root(
                            &path,
                            &canonical_root,
                            root.pinned_root.name_semantics.components(),
                        )
                    })
                    .map_err(|_| BrokerFailure::new(BrokerFailureCode::RecordUnsupported))
            })
            .transpose()?
            .flatten();
        let (scope, previous_scope, kind) = match (current_scope, previous_scope) {
            (None, None) => continue,
            (Some(current), Some(previous)) => (
                to_candidate_scope(current),
                Some(to_candidate_scope(previous)),
                candidate.kind,
            ),
            (Some(current), None) => (
                to_candidate_scope(current),
                None,
                isolated_candidate_kind(candidate.is_directory),
            ),
            (None, Some(previous)) => (
                to_candidate_scope(previous),
                None,
                isolated_candidate_kind(candidate.is_directory),
            ),
        };
        let (kind, previous_scope) = if matches!(scope, CandidateScope::Root)
            || previous_scope
                .as_ref()
                .is_some_and(|previous| matches!(previous, CandidateScope::Root))
        {
            (CandidateKind::Subtree, None)
        } else {
            (kind, previous_scope)
        };
        let normalized = BrokerCandidate {
            scope,
            previous_scope,
            file_reference: candidate.file_reference,
            usn: candidate.usn,
            kind,
            is_directory: candidate.is_directory,
        };
        normalized
            .validate()
            .map_err(|_| BrokerFailure::new(BrokerFailureCode::RecordUnsupported))?;
        candidates.push(normalized);
    }
    Ok(candidates)
}

fn to_candidate_scope(scope: RootRelativeScope) -> CandidateScope {
    match scope {
        RootRelativeScope::Root => CandidateScope::Root,
        RootRelativeScope::Relative(path) => CandidateScope::RelativePath(path),
    }
}

fn isolated_candidate_kind(is_directory: bool) -> CandidateKind {
    if is_directory {
        CandidateKind::Subtree
    } else {
        CandidateKind::Path
    }
}

fn shared_failure_outcome(
    caller: &CallerClaim,
    root: &super::wire::SharedJournalRootRequest,
    code: BrokerFailureCode,
) -> SharedJournalRootOutcome {
    SharedJournalRootOutcome {
        binding: ResponseBinding::from_request(caller, &root.root),
        requested_start_usn: root.start_usn,
        covered_until_usn: None,
        is_complete: false,
        candidates: Vec::new(),
        failure: Some(BrokerFailure::new(code)),
    }
}

fn validate_shared_backend_page(
    before: ExistingJournalMetadata,
    range: JournalRange,
    page: &SharedBackendJournalPage,
    expected_roots: usize,
) -> Result<(), BrokerFailure> {
    if page.candidates_by_root.len() != expected_roots
        || page.rename_barriers_by_root.len() != expected_roots
        || page
            .rename_barriers_by_root
            .iter()
            .any(|barrier| barrier.is_some_and(|usn| usn < range.start_usn || usn >= range.end_usn))
        || page.handoffs.len() > super::wire::MAX_SHARED_HANDOFFS
        || (page.candidates_by_root.iter().any(Result::is_ok) != page.proof.is_some())
        || (page.proof.is_none() && !page.handoffs.is_empty())
        || page.handoffs.iter().any(|handoff| {
            (handoff.previous_root_index.is_some() == handoff.previous_pending_index.is_some())
                || handoff
                    .previous_root_index
                    .is_some_and(|index| index >= expected_roots)
                || handoff.current_root_index >= expected_roots
                || handoff.previous_root_index == Some(handoff.current_root_index)
                || handoff
                    .previous_root_index
                    .is_some_and(|index| page.candidates_by_root[index].is_err())
                || page.candidates_by_root[handoff.current_root_index].is_err()
                || !matches!(handoff.file_reference.len(), 8 | 16)
                || handoff.usn < range.start_usn
                || page
                    .proof
                    .is_none_or(|proof| handoff.usn >= proof.covered_until_usn)
        })
        || page.pending_renames.iter().any(|pending| {
            pending.root_index >= expected_roots
                || page.candidates_by_root[pending.root_index].is_err()
        })
        || page
            .candidates_by_root
            .iter()
            .filter_map(|result| result.as_ref().ok())
            .any(|candidates| {
                let mut last = None;
                candidates.iter().any(|candidate| {
                    let invalid = candidate.usn < range.start_usn
                        || page
                            .proof
                            .is_none_or(|proof| candidate.usn >= proof.covered_until_usn)
                        || last.is_some_and(|previous| candidate.usn <= previous);
                    last = Some(candidate.usn);
                    invalid
                })
            })
    {
        return Err(BrokerFailure::new(BrokerFailureCode::JournalDiscontinuous));
    }
    if let Some(proof) = page.proof {
        let after = proof.after.validate().map_err(map_backend_failure)?;
        if after.journal_id != before.journal_id
            || after.first_usn > range.start_usn
            || after.next_usn < range.end_usn
            || proof.covered_until_usn <= range.start_usn
            || proof.covered_until_usn > range.end_usn
            || proof.is_complete != (proof.covered_until_usn == range.end_usn)
        {
            return Err(BrokerFailure::new(BrokerFailureCode::JournalDiscontinuous));
        }
    }
    Ok(())
}

type RootScopedTranslation<T> = (Vec<T>, Vec<(usize, BrokerFailureCode)>, Option<i64>);

struct HandoffTranslation {
    handoffs: Vec<SharedJournalHandoff>,
    pending_renames: Vec<SharedJournalPendingRename>,
    failures: Vec<(usize, BrokerFailureCode)>,
    barrier: Option<i64>,
}

fn lower_translation_barrier(barrier: &mut Option<i64>, candidate: i64) {
    *barrier = Some(barrier.map_or(candidate, |current| current.min(candidate)));
}

fn preserve_verified_handoff_source(
    pending_renames: &mut Vec<SharedJournalPendingRename>,
    barrier: &mut Option<i64>,
    previous_binding: &ResponseBinding,
    previous_carry_id: Option<&str>,
    handoff: &BackendJournalHandoff,
    previous_relative_path: &str,
) {
    if previous_carry_id.is_none() {
        pending_renames.push(SharedJournalPendingRename {
            binding: previous_binding.clone(),
            file_reference: handoff.file_reference.clone(),
            old_usn: handoff.old_usn,
            previous_relative_path: previous_relative_path.to_owned(),
            is_directory: handoff.is_directory,
        });
    }
    lower_translation_barrier(barrier, handoff.usn);
}

#[allow(clippy::too_many_arguments)]
fn translate_shared_pending_renames(
    authorizer: &impl CallerAuthorizer,
    caller: &CallerClaim,
    request: &ReadJournalVolumeRequest,
    admitted: &[(usize, AuthorizedPinnedRoot)],
    backend_pending: &[BackendPendingRename],
    covered_until_usn: i64,
    context: &mut RequestContext<'_>,
) -> Result<RootScopedTranslation<SharedJournalPendingRename>, BrokerFailure> {
    let mut pending_renames = Vec::with_capacity(backend_pending.len());
    let mut failures = Vec::new();
    let mut barrier = None;
    for pending in backend_pending {
        let Some((request_index, root)) = admitted.get(pending.root_index) else {
            return Err(BrokerFailure::new(BrokerFailureCode::RecordUnsupported));
        };
        let failure_barrier = if pending.old_usn >= request.minimum_start_usn()
            && pending.old_usn < covered_until_usn
        {
            pending.old_usn
        } else {
            request.minimum_start_usn()
        };
        if !matches!(pending.file_reference.len(), 8 | 16)
            || pending.old_usn < request.roots[*request_index].start_usn
            || pending.old_usn >= covered_until_usn
        {
            failures.push((pending.root_index, BrokerFailureCode::RecordUnsupported));
            barrier = Some(barrier.map_or(failure_barrier, |old: i64| old.min(failure_barrier)));
            continue;
        }
        let candidate = match BackendJournalCandidate::copy_from_bounded(
            &request.roots[*request_index].root.volume_id,
            &pending.previous_absolute_path_utf16,
            None,
            &pending.file_reference,
            pending.old_usn,
            if pending.is_directory {
                CandidateKind::Subtree
            } else {
                CandidateKind::Path
            },
            pending.is_directory,
        ) {
            Ok(candidate) => candidate,
            Err(error) => {
                failures.push((pending.root_index, map_backend_failure(error).code));
                barrier =
                    Some(barrier.map_or(failure_barrier, |old: i64| old.min(failure_barrier)));
                continue;
            }
        };
        let candidate =
            match authorizer.filter_disclosable_candidates(root, vec![candidate], context) {
                Ok(candidate) => candidate,
                Err(error) => {
                    failures.push((pending.root_index, map_authorizer_failure(error).code));
                    barrier =
                        Some(barrier.map_or(pending.old_usn, |old: i64| old.min(pending.old_usn)));
                    continue;
                }
            };
        let candidate = match constrain_candidates(root, candidate) {
            Ok(candidate) => candidate,
            Err(failure) => {
                failures.push((pending.root_index, failure.code));
                barrier =
                    Some(barrier.map_or(pending.old_usn, |old: i64| old.min(pending.old_usn)));
                continue;
            }
        };
        let [
            BrokerCandidate {
                scope: CandidateScope::RelativePath(previous_relative_path),
                ..
            },
        ] = candidate.as_slice()
        else {
            failures.push((pending.root_index, BrokerFailureCode::RecordUnsupported));
            barrier = Some(barrier.map_or(pending.old_usn, |old: i64| old.min(pending.old_usn)));
            continue;
        };
        pending_renames.push(SharedJournalPendingRename {
            binding: ResponseBinding::from_request(caller, &request.roots[*request_index].root),
            file_reference: pending.file_reference.clone(),
            old_usn: pending.old_usn,
            previous_relative_path: previous_relative_path.clone(),
            is_directory: pending.is_directory,
        });
    }
    Ok((pending_renames, failures, barrier))
}

#[allow(clippy::too_many_arguments)]
fn translate_shared_handoffs(
    authorizer: &impl CallerAuthorizer,
    caller: &CallerClaim,
    request: &ReadJournalVolumeRequest,
    admitted: &[(usize, AuthorizedPinnedRoot)],
    pending_renames: &[BackendPendingRenameCarry],
    backend_handoffs: &[BackendJournalHandoff],
    covered_until_usn: i64,
    context: &mut RequestContext<'_>,
) -> Result<HandoffTranslation, BrokerFailure> {
    let mut handoffs = Vec::with_capacity(backend_handoffs.len());
    let mut translated_pending = Vec::new();
    let mut failures = Vec::new();
    let mut barrier = None;
    for handoff in backend_handoffs {
        let Some((current_request_index, current_root)) = admitted.get(handoff.current_root_index)
        else {
            return Err(BrokerFailure::new(BrokerFailureCode::RecordUnsupported));
        };
        let previous = match (handoff.previous_root_index, handoff.previous_pending_index) {
            (Some(previous_root_index), None) => {
                let Some((previous_request_index, previous_root)) =
                    admitted.get(previous_root_index)
                else {
                    return Err(BrokerFailure::new(BrokerFailureCode::RecordUnsupported));
                };
                (
                    previous_root,
                    ResponseBinding::from_request(
                        caller,
                        &request.roots[*previous_request_index].root,
                    ),
                    None,
                    Some(previous_root_index),
                )
            }
            (None, Some(previous_pending_index)) => {
                let Some(pending) = pending_renames.get(previous_pending_index) else {
                    return Err(BrokerFailure::new(BrokerFailureCode::RecordUnsupported));
                };
                let Some(previous_root) = pending.root.as_ref() else {
                    return Err(BrokerFailure::new(BrokerFailureCode::RecordUnsupported));
                };
                let Some(requested) = request.pending_renames.get(previous_pending_index) else {
                    return Err(BrokerFailure::new(BrokerFailureCode::RecordUnsupported));
                };
                if requested.file_reference != handoff.file_reference
                    || requested.is_directory != handoff.is_directory
                    || requested.old_usn != handoff.old_usn
                {
                    return Err(BrokerFailure::new(BrokerFailureCode::RecordUnsupported));
                }
                (
                    previous_root,
                    ResponseBinding::from_request(caller, &requested.root),
                    Some(requested.carry_id.clone()),
                    pending.read_root_index,
                )
            }
            _ => return Err(BrokerFailure::new(BrokerFailureCode::RecordUnsupported)),
        };
        let (previous_root, previous_binding, previous_carry_id, previous_root_index) = previous;
        let safe_old_barrier = if handoff.old_usn >= request.minimum_start_usn()
            && handoff.old_usn < handoff.usn
            && handoff.old_usn < covered_until_usn
        {
            handoff.old_usn
        } else {
            request.minimum_start_usn()
        };
        let previous_request_start = handoff
            .previous_root_index
            .map(|index| request.roots[admitted[index].0].start_usn)
            .unwrap_or(handoff.old_usn);
        if !matches!(handoff.file_reference.len(), 8 | 16)
            || handoff.old_usn < previous_request_start
            || handoff.old_usn < 0
            || handoff.old_usn >= handoff.usn
            || handoff.usn < request.roots[*current_request_index].start_usn
            || handoff.usn >= covered_until_usn
        {
            if let Some(index) = previous_root_index {
                failures.push((index, BrokerFailureCode::RecordUnsupported));
            }
            failures.push((
                handoff.current_root_index,
                BrokerFailureCode::RecordUnsupported,
            ));
            lower_translation_barrier(&mut barrier, safe_old_barrier);
            continue;
        }
        let kind = if handoff.is_directory {
            CandidateKind::Subtree
        } else {
            CandidateKind::Path
        };
        let previous = match BackendJournalCandidate::copy_from_bounded(
            &previous_root.pinned_root.volume_id,
            &handoff.previous_absolute_path_utf16,
            None,
            &handoff.file_reference,
            handoff.old_usn,
            kind,
            handoff.is_directory,
        ) {
            Ok(previous) => previous,
            Err(error) => {
                if let Some(index) = previous_root_index {
                    failures.push((index, map_backend_failure(error).code));
                }
                lower_translation_barrier(&mut barrier, safe_old_barrier);
                continue;
            }
        };
        let previous = match authorizer.filter_disclosable_candidates(
            previous_root,
            vec![previous],
            context,
        ) {
            Ok(previous) => previous,
            Err(error) => {
                if let Some(index) = previous_root_index {
                    failures.push((index, map_authorizer_failure(error).code));
                }
                lower_translation_barrier(&mut barrier, safe_old_barrier);
                continue;
            }
        };
        let previous = match constrain_candidates(previous_root, previous) {
            Ok(previous) => previous,
            Err(failure) => {
                if let Some(index) = previous_root_index {
                    failures.push((index, failure.code));
                }
                lower_translation_barrier(&mut barrier, safe_old_barrier);
                continue;
            }
        };
        let [
            BrokerCandidate {
                scope: CandidateScope::RelativePath(previous_relative_path),
                ..
            },
        ] = previous.as_slice()
        else {
            if let Some(index) = previous_root_index {
                failures.push((index, BrokerFailureCode::RecordUnsupported));
            }
            lower_translation_barrier(&mut barrier, safe_old_barrier);
            continue;
        };
        if let Some(previous_pending_index) = handoff.previous_pending_index {
            let requested = &request.pending_renames[previous_pending_index];
            if requested.previous_relative_path != *previous_relative_path {
                if let Some(index) = previous_root_index {
                    failures.push((index, BrokerFailureCode::RecordUnsupported));
                }
                lower_translation_barrier(&mut barrier, safe_old_barrier);
                continue;
            }
        }
        let current = match BackendJournalCandidate::copy_from_bounded(
            &request.roots[*current_request_index].root.volume_id,
            &handoff.current_absolute_path_utf16,
            None,
            &handoff.file_reference,
            handoff.usn,
            kind,
            handoff.is_directory,
        ) {
            Ok(current) => current,
            Err(error) => {
                failures.push((handoff.current_root_index, map_backend_failure(error).code));
                preserve_verified_handoff_source(
                    &mut translated_pending,
                    &mut barrier,
                    &previous_binding,
                    previous_carry_id.as_deref(),
                    handoff,
                    previous_relative_path,
                );
                continue;
            }
        };
        let current =
            match authorizer.filter_disclosable_candidates(current_root, vec![current], context) {
                Ok(current) => current,
                Err(error) => {
                    failures.push((
                        handoff.current_root_index,
                        map_authorizer_failure(error).code,
                    ));
                    preserve_verified_handoff_source(
                        &mut translated_pending,
                        &mut barrier,
                        &previous_binding,
                        previous_carry_id.as_deref(),
                        handoff,
                        previous_relative_path,
                    );
                    continue;
                }
            };
        let current = match constrain_candidates(current_root, current) {
            Ok(current) => current,
            Err(failure) => {
                failures.push((handoff.current_root_index, failure.code));
                preserve_verified_handoff_source(
                    &mut translated_pending,
                    &mut barrier,
                    &previous_binding,
                    previous_carry_id.as_deref(),
                    handoff,
                    previous_relative_path,
                );
                continue;
            }
        };
        let [
            BrokerCandidate {
                scope: CandidateScope::RelativePath(current_relative_path),
                ..
            },
        ] = current.as_slice()
        else {
            failures.push((
                handoff.current_root_index,
                BrokerFailureCode::RecordUnsupported,
            ));
            preserve_verified_handoff_source(
                &mut translated_pending,
                &mut barrier,
                &previous_binding,
                previous_carry_id.as_deref(),
                handoff,
                previous_relative_path,
            );
            continue;
        };
        handoffs.push(SharedJournalHandoff {
            previous_binding,
            current_binding: ResponseBinding::from_request(
                caller,
                &request.roots[*current_request_index].root,
            ),
            file_reference: handoff.file_reference.clone(),
            usn: handoff.usn,
            previous_relative_path: previous_relative_path.clone(),
            current_relative_path: current_relative_path.clone(),
            is_directory: handoff.is_directory,
            previous_carry_id,
        });
    }
    Ok(HandoffTranslation {
        handoffs,
        pending_renames: translated_pending,
        failures,
        barrier,
    })
}

fn validate_shared_response_limits(
    outcomes: &[SharedJournalRootOutcome],
    handoffs: &[SharedJournalHandoff],
    pending_renames: &[SharedJournalPendingRename],
    maximum_records: usize,
    maximum_evidence_bytes: usize,
) -> Result<(), BrokerFailure> {
    let mut records = 0_usize;
    let mut evidence = 0_usize;
    for candidate in outcomes.iter().flat_map(|outcome| &outcome.candidates) {
        records = records
            .checked_add(1)
            .ok_or_else(|| BrokerFailure::new(BrokerFailureCode::EvidenceLimitExceeded))?;
        evidence = evidence
            .checked_add(
                candidate
                    .evidence_bytes()
                    .ok_or_else(|| BrokerFailure::new(BrokerFailureCode::EvidenceLimitExceeded))?,
            )
            .ok_or_else(|| BrokerFailure::new(BrokerFailureCode::EvidenceLimitExceeded))?;
    }
    for handoff in handoffs {
        records = records
            .checked_add(1)
            .ok_or_else(|| BrokerFailure::new(BrokerFailureCode::EvidenceLimitExceeded))?;
        evidence = evidence
            .checked_add(
                handoff
                    .evidence_bytes()
                    .ok_or_else(|| BrokerFailure::new(BrokerFailureCode::EvidenceLimitExceeded))?,
            )
            .ok_or_else(|| BrokerFailure::new(BrokerFailureCode::EvidenceLimitExceeded))?;
    }
    for pending in pending_renames {
        records = records
            .checked_add(1)
            .ok_or_else(|| BrokerFailure::new(BrokerFailureCode::EvidenceLimitExceeded))?;
        evidence = evidence
            .checked_add(
                pending
                    .evidence_bytes()
                    .ok_or_else(|| BrokerFailure::new(BrokerFailureCode::EvidenceLimitExceeded))?,
            )
            .ok_or_else(|| BrokerFailure::new(BrokerFailureCode::EvidenceLimitExceeded))?;
    }
    if records > maximum_records || evidence > maximum_evidence_bytes {
        return Err(BrokerFailure::new(BrokerFailureCode::EvidenceLimitExceeded));
    }
    Ok(())
}

fn malformed_response(payload: &[u8], error: WireError) -> BrokerResponse {
    let inspected = inspect_request_header(payload).ok();
    let code = match error {
        WireError::UnsupportedVersion(_) => BrokerFailureCode::ProtocolMismatch,
        WireError::LengthExceeded | WireError::LengthOverflow => BrokerFailureCode::RequestTooLarge,
        _ => BrokerFailureCode::MalformedFrame,
    };
    failure_response(inspected.map_or(0, |header| header.request_id), None, code)
}

fn encode_recorded_terminal(terminal: RecordedTerminal) -> Result<TerminalFrame, BrokerFailure> {
    let code = match terminal.terminal {
        RequestTerminal::Cancelled => BrokerFailureCode::Cancelled,
        RequestTerminal::TimedOut => BrokerFailureCode::TimedOut,
        RequestTerminal::Aborted => BrokerFailureCode::RequestNotActive,
        RequestTerminal::Completed => BrokerFailureCode::Internal,
    };
    let response = failure_response(
        terminal.key.request_id,
        Some(terminal.key.client_instance),
        code,
    );
    let payload =
        encode_response(&response).map_err(|_| BrokerFailure::new(BrokerFailureCode::Internal))?;
    let frame =
        encode_frame(&payload).map_err(|_| BrokerFailure::new(BrokerFailureCode::Internal))?;
    Ok(TerminalFrame {
        request_id: terminal.key.request_id,
        frame,
    })
}

fn failure_response(
    request_id: u64,
    client_instance: Option<[u8; 16]>,
    code: BrokerFailureCode,
) -> BrokerResponse {
    BrokerResponse::Failure {
        request_id,
        client_instance,
        failure: BrokerFailure::new(code),
    }
}

fn map_authorizer_failure(error: AuthorizerError) -> BrokerFailure {
    let code = match error {
        AuthorizerError::CallerRejected => BrokerFailureCode::CallerRejected,
        AuthorizerError::RootUnauthorized => BrokerFailureCode::RootUnauthorized,
        AuthorizerError::RootIdentityMismatch => BrokerFailureCode::RootIdentityMismatch,
        AuthorizerError::UnsupportedFilesystem => BrokerFailureCode::JournalUnavailable,
    };
    BrokerFailure::new(code)
}

fn map_backend_failure(error: BackendError) -> BrokerFailure {
    let code = match error {
        BackendError::RootUnavailable => BrokerFailureCode::RootIdentityMismatch,
        BackendError::JournalUnavailable => BrokerFailureCode::JournalUnavailable,
        BackendError::JournalDiscontinuous => BrokerFailureCode::JournalDiscontinuous,
        BackendError::VolumeMismatch => BrokerFailureCode::VolumeMismatch,
        BackendError::RecordUnsupported => BrokerFailureCode::RecordUnsupported,
        BackendError::EvidenceLimitExceeded => BrokerFailureCode::EvidenceLimitExceeded,
        BackendError::TimedOut => BrokerFailureCode::TimedOut,
        BackendError::Cancelled => BrokerFailureCode::Cancelled,
        BackendError::Unavailable => BrokerFailureCode::BackendUnavailable,
    };
    BrokerFailure::new(code)
}

fn map_active_failure(error: ActiveRequestError) -> BrokerFailure {
    let code = match error {
        ActiveRequestError::AlreadyActive | ActiveRequestError::LimitExceeded => {
            BrokerFailureCode::ActiveLimitExceeded
        }
        ActiveRequestError::TerminalDeliveryBackpressure => {
            BrokerFailureCode::TerminalDeliveryBackpressure
        }
        ActiveRequestError::NotActive
        | ActiveRequestError::ConnectionNotOpen
        | ActiveRequestError::ConnectionGenerationMismatch => BrokerFailureCode::RequestNotActive,
        ActiveRequestError::Cancelled => BrokerFailureCode::Cancelled,
        ActiveRequestError::TimedOut => BrokerFailureCode::TimedOut,
    };
    BrokerFailure::new(code)
}

#[cfg(test)]
mod tests;
