use super::{
    BrokerCandidate, BrokerFailure, BrokerFailureCode, BrokerRequest, BrokerResponse, CallerClaim,
    CandidateKind, CandidateScope, InspectedRequestHeader, JournalCapability,
    LEGACY_PROTOCOL_MAGIC_V1, LEGACY_PROTOCOL_MAGIC_V2, LEGACY_PROTOCOL_MAGIC_V3,
    LEGACY_PROTOCOL_MAGIC_V4, MAX_EVIDENCE_BYTES, MAX_PATH_UTF16_UNITS, MAX_RECORDS,
    MAX_SHARED_HANDOFFS, MAX_SHARED_PENDING_RENAMES, MAX_SHARED_ROOTS, PROTOCOL_MAGIC,
    PROTOCOL_VERSION, QueryJournalRequest, ReadJournalRangeRequest, ReadJournalVolumeRequest,
    RegisterRootRequest, ResponseBinding, RootAuthorization, RootCapability, SharedJournalHandoff,
    SharedJournalPendingRename, SharedJournalPendingRenameRequest, SharedJournalRootOutcome,
    SharedJournalRootRequest, WireError,
};

const REGISTER_ROOT_REQUEST: u16 = 4;
const READ_VOLUME_REQUEST: u16 = 5;
const QUERY_REQUEST: u16 = 1;
const READ_REQUEST: u16 = 2;
const CANCEL_REQUEST: u16 = 3;
const JOURNAL_RESPONSE: u16 = 0x8001;
const READ_RESPONSE: u16 = 0x8002;
const CANCEL_RESPONSE: u16 = 0x8003;
const READ_ACCEPTED_RESPONSE: u16 = 0x8004;
const ROOT_REGISTERED_RESPONSE: u16 = 0x8005;
const READ_VOLUME_RESPONSE: u16 = 0x8006;
const READ_VOLUME_ACCEPTED_RESPONSE: u16 = 0x8007;
const FAILURE_RESPONSE: u16 = 0x8fff;

pub(crate) fn inspect_request_header(payload: &[u8]) -> Result<InspectedRequestHeader, WireError> {
    let magic = payload.get(..8).ok_or(WireError::InvalidMagic)?;
    let is_current_magic = magic == PROTOCOL_MAGIC.as_slice();
    if !is_current_magic
        && magic != LEGACY_PROTOCOL_MAGIC_V1.as_slice()
        && magic != LEGACY_PROTOCOL_MAGIC_V2.as_slice()
        && magic != LEGACY_PROTOCOL_MAGIC_V3.as_slice()
        && magic != LEGACY_PROTOCOL_MAGIC_V4.as_slice()
    {
        return Err(WireError::InvalidMagic);
    }
    let version = read_u16_at(payload, 8)?;
    let request_id = read_u64_at(payload, 12)?;
    Ok(InspectedRequestHeader {
        request_id,
        version,
        is_current_magic,
    })
}

pub(crate) fn encode_request(request: &BrokerRequest) -> Result<Vec<u8>, WireError> {
    let request_id = request.request_id();
    if request_id == 0 {
        return Err(WireError::InvalidValue("request_id"));
    }
    let mut writer = Writer::new();
    match request {
        BrokerRequest::RegisterRoot { request, .. } => {
            request.validate()?;
            writer.header(REGISTER_ROOT_REQUEST, request_id);
            writer.caller(&request.caller);
            writer.root(&request.root)?;
            writer.u64(request.client_root_handle);
            writer.u32(request.timeout_ms);
        }
        BrokerRequest::QueryJournal { request, .. } => {
            request.validate()?;
            writer.header(QUERY_REQUEST, request_id);
            writer.caller(&request.caller);
            writer.root(&request.root)?;
            writer.root_capability(request.root_capability);
            writer.u32(request.timeout_ms);
        }
        BrokerRequest::ReadRange { request, .. } => {
            request.validate()?;
            writer.header(READ_REQUEST, request_id);
            writer.caller(&request.caller);
            writer.root(&request.root)?;
            writer.root_capability(request.root_capability);
            writer.u64(request.journal_id);
            writer.i64(request.start_usn);
            writer.i64(request.end_usn);
            writer.u32(request.max_records);
            writer.u32(request.max_evidence_bytes);
            writer.u32(request.timeout_ms);
        }
        BrokerRequest::ReadVolume { request, .. } => {
            request.validate()?;
            writer.header(READ_VOLUME_REQUEST, request_id);
            writer.caller(&request.caller);
            writer.u8(u8::try_from(request.roots.len()).map_err(|_| WireError::LengthOverflow)?);
            for root in &request.roots {
                writer.root(&root.root)?;
                writer.root_capability(root.root_capability);
                writer.i64(root.start_usn);
            }
            writer.u16(
                u16::try_from(request.pending_renames.len())
                    .map_err(|_| WireError::LengthOverflow)?,
            );
            for pending in &request.pending_renames {
                writer.string(&pending.carry_id)?;
                writer.string(&pending.source_range_id)?;
                writer.root(&pending.root)?;
                writer.boolean(pending.root_capability.is_some());
                if let Some(capability) = pending.root_capability {
                    writer.root_capability(capability);
                }
                writer.byte_vec(&pending.file_reference)?;
                writer.i64(pending.old_usn);
                writer.string(&pending.previous_relative_path)?;
                writer.boolean(pending.is_directory);
            }
            writer.u64(request.journal_id);
            writer.i64(request.end_usn);
            writer.u32(request.max_records);
            writer.u32(request.max_evidence_bytes);
            writer.u32(request.timeout_ms);
        }
        BrokerRequest::Cancel {
            caller,
            target_request_id,
            ..
        } => {
            caller.validate()?;
            if *target_request_id == 0 {
                return Err(WireError::InvalidValue("target_request_id"));
            }
            writer.header(CANCEL_REQUEST, request_id);
            writer.caller(caller);
            writer.u64(*target_request_id);
        }
    }
    Ok(writer.finish())
}

pub(crate) fn decode_request(payload: &[u8]) -> Result<BrokerRequest, WireError> {
    let header = inspect_request_header(payload)?;
    if !header.is_current_magic {
        return Err(WireError::UnsupportedVersion(header.version));
    }
    if header.request_id == 0 {
        return Err(WireError::InvalidValue("request_id"));
    }
    if header.version != PROTOCOL_VERSION {
        return Err(WireError::UnsupportedVersion(header.version));
    }
    let mut reader = Reader::after_header(payload)?;
    let kind = reader.kind;
    let request = match kind {
        REGISTER_ROOT_REQUEST => BrokerRequest::RegisterRoot {
            request_id: header.request_id,
            request: RegisterRootRequest {
                caller: reader.caller()?,
                root: reader.root()?,
                client_root_handle: reader.u64()?,
                timeout_ms: reader.u32()?,
            },
        },
        QUERY_REQUEST => BrokerRequest::QueryJournal {
            request_id: header.request_id,
            request: QueryJournalRequest {
                caller: reader.caller()?,
                root: reader.root()?,
                root_capability: reader.root_capability()?,
                timeout_ms: reader.u32()?,
            },
        },
        READ_REQUEST => BrokerRequest::ReadRange {
            request_id: header.request_id,
            request: ReadJournalRangeRequest {
                caller: reader.caller()?,
                root: reader.root()?,
                root_capability: reader.root_capability()?,
                journal_id: reader.u64()?,
                start_usn: reader.i64()?,
                end_usn: reader.i64()?,
                max_records: reader.u32()?,
                max_evidence_bytes: reader.u32()?,
                timeout_ms: reader.u32()?,
            },
        },
        READ_VOLUME_REQUEST => {
            let caller = reader.caller()?;
            let count = usize::from(reader.u8()?);
            if !(1..=MAX_SHARED_ROOTS).contains(&count) {
                return Err(WireError::LengthExceeded);
            }
            let mut roots = Vec::with_capacity(count);
            for _ in 0..count {
                roots.push(SharedJournalRootRequest {
                    root: reader.root()?,
                    root_capability: reader.root_capability()?,
                    start_usn: reader.i64()?,
                });
            }
            let pending_count = usize::from(reader.u16()?);
            if pending_count > MAX_SHARED_PENDING_RENAMES {
                return Err(WireError::LengthExceeded);
            }
            let mut pending_renames = Vec::with_capacity(pending_count);
            for _ in 0..pending_count {
                let carry_id = reader.string(512)?;
                let source_range_id = reader.string(512)?;
                let root = reader.root()?;
                let root_capability = reader
                    .boolean()?
                    .then(|| reader.root_capability())
                    .transpose()?;
                pending_renames.push(SharedJournalPendingRenameRequest {
                    carry_id,
                    source_range_id,
                    root,
                    root_capability,
                    file_reference: reader.byte_vec(16)?,
                    old_usn: reader.i64()?,
                    previous_relative_path: reader.string(MAX_PATH_UTF16_UNITS * 4)?,
                    is_directory: reader.boolean()?,
                });
            }
            BrokerRequest::ReadVolume {
                request_id: header.request_id,
                request: ReadJournalVolumeRequest {
                    caller,
                    roots,
                    pending_renames,
                    journal_id: reader.u64()?,
                    end_usn: reader.i64()?,
                    max_records: reader.u32()?,
                    max_evidence_bytes: reader.u32()?,
                    timeout_ms: reader.u32()?,
                },
            }
        }
        CANCEL_REQUEST => BrokerRequest::Cancel {
            request_id: header.request_id,
            caller: reader.caller()?,
            target_request_id: reader.u64()?,
        },
        _ => return Err(WireError::UnknownKind(kind)),
    };
    reader.finish()?;
    match &request {
        BrokerRequest::RegisterRoot { request, .. } => request.validate()?,
        BrokerRequest::QueryJournal { request, .. } => request.validate()?,
        BrokerRequest::ReadRange { request, .. } => request.validate()?,
        BrokerRequest::ReadVolume { request, .. } => request.validate()?,
        BrokerRequest::Cancel {
            caller,
            target_request_id,
            ..
        } => {
            caller.validate()?;
            if *target_request_id == 0 {
                return Err(WireError::InvalidValue("target_request_id"));
            }
        }
    }
    Ok(request)
}

pub(crate) fn encode_response(response: &BrokerResponse) -> Result<Vec<u8>, WireError> {
    validate_response(response)?;
    let mut writer = Writer::new();
    match response {
        BrokerResponse::RootRegistered {
            request_id,
            binding,
            root_capability,
        } => {
            writer.header(ROOT_REGISTERED_RESPONSE, *request_id);
            writer.binding(binding)?;
            writer.root_capability(*root_capability);
        }
        BrokerResponse::Journal {
            request_id,
            binding,
            capability,
            journal_id,
            first_usn,
            next_usn,
        } => {
            writer.header(JOURNAL_RESPONSE, *request_id);
            writer.binding(binding)?;
            writer.u8(capability_code(*capability));
            writer.optional_u64(*journal_id);
            writer.optional_i64(*first_usn);
            writer.optional_i64(*next_usn);
        }
        BrokerResponse::ReadRange {
            request_id,
            binding,
            journal_id,
            requested_start_usn,
            requested_end_usn,
            max_records,
            max_evidence_bytes,
            covered_until_usn,
            is_complete,
            candidates,
        } => {
            writer.header(READ_RESPONSE, *request_id);
            writer.binding(binding)?;
            writer.u64(*journal_id);
            writer.i64(*requested_start_usn);
            writer.i64(*requested_end_usn);
            writer.u32(*max_records);
            writer.u32(*max_evidence_bytes);
            writer.i64(*covered_until_usn);
            writer.boolean(*is_complete);
            writer.u32(u32::try_from(candidates.len()).map_err(|_| WireError::LengthOverflow)?);
            for candidate in candidates {
                writer.candidate(candidate)?;
            }
        }
        BrokerResponse::ReadRangeAccepted {
            request_id,
            binding,
            journal_id,
            requested_start_usn,
            requested_end_usn,
            max_records,
            max_evidence_bytes,
        } => {
            writer.header(READ_ACCEPTED_RESPONSE, *request_id);
            writer.binding(binding)?;
            writer.u64(*journal_id);
            writer.i64(*requested_start_usn);
            writer.i64(*requested_end_usn);
            writer.u32(*max_records);
            writer.u32(*max_evidence_bytes);
        }
        BrokerResponse::ReadVolumeAccepted {
            request_id,
            client_instance,
            volume_id,
            journal_id,
            requested_start_usn,
            requested_end_usn,
            root_count,
            max_records,
            max_evidence_bytes,
        } => {
            writer.header(READ_VOLUME_ACCEPTED_RESPONSE, *request_id);
            writer.bytes(client_instance);
            writer.string(volume_id)?;
            writer.u64(*journal_id);
            writer.i64(*requested_start_usn);
            writer.i64(*requested_end_usn);
            writer.u8(*root_count);
            writer.u32(*max_records);
            writer.u32(*max_evidence_bytes);
        }
        BrokerResponse::ReadVolume {
            request_id,
            client_instance,
            volume_id,
            journal_id,
            requested_end_usn,
            max_records,
            max_evidence_bytes,
            outcomes,
            handoffs,
            pending_renames,
        } => {
            writer.header(READ_VOLUME_RESPONSE, *request_id);
            writer.bytes(client_instance);
            writer.string(volume_id)?;
            writer.u64(*journal_id);
            writer.i64(*requested_end_usn);
            writer.u32(*max_records);
            writer.u32(*max_evidence_bytes);
            writer.u8(u8::try_from(outcomes.len()).map_err(|_| WireError::LengthOverflow)?);
            for outcome in outcomes {
                writer.binding(&outcome.binding)?;
                writer.i64(outcome.requested_start_usn);
                writer.boolean(outcome.failure.is_some());
                if let Some(failure) = outcome.failure {
                    writer.u16(failure_code(failure.code));
                } else {
                    writer.i64(
                        outcome
                            .covered_until_usn
                            .ok_or(WireError::InvalidValue("shared_root_proof"))?,
                    );
                    writer.boolean(outcome.is_complete);
                    writer.u32(
                        u32::try_from(outcome.candidates.len())
                            .map_err(|_| WireError::LengthOverflow)?,
                    );
                    for candidate in &outcome.candidates {
                        writer.candidate(candidate)?;
                    }
                }
            }
            writer.u32(u32::try_from(handoffs.len()).map_err(|_| WireError::LengthOverflow)?);
            for handoff in handoffs {
                writer.binding(&handoff.previous_binding)?;
                writer.binding(&handoff.current_binding)?;
                writer.byte_vec(&handoff.file_reference)?;
                writer.i64(handoff.usn);
                writer.string(&handoff.previous_relative_path)?;
                writer.string(&handoff.current_relative_path)?;
                writer.boolean(handoff.is_directory);
                writer.boolean(handoff.previous_carry_id.is_some());
                if let Some(carry_id) = &handoff.previous_carry_id {
                    writer.string(carry_id)?;
                }
            }
            writer
                .u32(u32::try_from(pending_renames.len()).map_err(|_| WireError::LengthOverflow)?);
            for pending in pending_renames {
                writer.binding(&pending.binding)?;
                writer.byte_vec(&pending.file_reference)?;
                writer.i64(pending.old_usn);
                writer.string(&pending.previous_relative_path)?;
                writer.boolean(pending.is_directory);
            }
        }
        BrokerResponse::Cancelled {
            request_id,
            client_instance,
            target_request_id,
        } => {
            writer.header(CANCEL_RESPONSE, *request_id);
            writer.bytes(client_instance);
            writer.u64(*target_request_id);
        }
        BrokerResponse::Failure {
            request_id,
            client_instance,
            failure,
        } => {
            writer.header(FAILURE_RESPONSE, *request_id);
            writer.boolean(client_instance.is_some());
            if let Some(client_instance) = client_instance {
                writer.bytes(client_instance);
            }
            writer.u16(failure_code(failure.code));
        }
    }
    Ok(writer.finish())
}

fn validate_identifier_for_codec(value: &str, maximum: usize) -> Result<(), WireError> {
    if value.is_empty() || value.len() > maximum || value.contains('\0') {
        return Err(WireError::InvalidValue("identifier"));
    }
    Ok(())
}

pub(crate) fn decode_response(payload: &[u8]) -> Result<BrokerResponse, WireError> {
    let header = inspect_request_header(payload)?;
    if !header.is_current_magic {
        return Err(WireError::UnsupportedVersion(header.version));
    }
    if header.version != PROTOCOL_VERSION {
        return Err(WireError::UnsupportedVersion(header.version));
    }
    let mut reader = Reader::after_header(payload)?;
    let response = match reader.kind {
        ROOT_REGISTERED_RESPONSE => BrokerResponse::RootRegistered {
            request_id: header.request_id,
            binding: reader.binding()?,
            root_capability: reader.root_capability()?,
        },
        JOURNAL_RESPONSE => BrokerResponse::Journal {
            request_id: header.request_id,
            binding: reader.binding()?,
            capability: decode_capability(reader.u8()?)?,
            journal_id: reader.optional_u64()?,
            first_usn: reader.optional_i64()?,
            next_usn: reader.optional_i64()?,
        },
        READ_RESPONSE => {
            let binding = reader.binding()?;
            let journal_id = reader.u64()?;
            let requested_start_usn = reader.i64()?;
            let requested_end_usn = reader.i64()?;
            let max_records = reader.u32()?;
            let max_evidence_bytes = reader.u32()?;
            let covered_until_usn = reader.i64()?;
            let is_complete = reader.boolean()?;
            let count = usize::try_from(reader.u32()?).map_err(|_| WireError::LengthOverflow)?;
            if count > MAX_RECORDS as usize {
                return Err(WireError::LengthExceeded);
            }
            let mut candidates = Vec::with_capacity(count);
            let mut evidence = 0_usize;
            for _ in 0..count {
                let candidate = reader.candidate()?;
                evidence = evidence
                    .checked_add(
                        candidate
                            .evidence_bytes()
                            .ok_or(WireError::LengthOverflow)?,
                    )
                    .ok_or(WireError::LengthOverflow)?;
                if evidence > MAX_EVIDENCE_BYTES as usize {
                    return Err(WireError::LengthExceeded);
                }
                candidates.push(candidate);
            }
            BrokerResponse::ReadRange {
                request_id: header.request_id,
                binding,
                journal_id,
                requested_start_usn,
                requested_end_usn,
                max_records,
                max_evidence_bytes,
                covered_until_usn,
                is_complete,
                candidates,
            }
        }
        READ_ACCEPTED_RESPONSE => BrokerResponse::ReadRangeAccepted {
            request_id: header.request_id,
            binding: reader.binding()?,
            journal_id: reader.u64()?,
            requested_start_usn: reader.i64()?,
            requested_end_usn: reader.i64()?,
            max_records: reader.u32()?,
            max_evidence_bytes: reader.u32()?,
        },
        READ_VOLUME_ACCEPTED_RESPONSE => BrokerResponse::ReadVolumeAccepted {
            request_id: header.request_id,
            client_instance: reader.array_16()?,
            volume_id: reader.string(super::MAX_VOLUME_ID_BYTES)?,
            journal_id: reader.u64()?,
            requested_start_usn: reader.i64()?,
            requested_end_usn: reader.i64()?,
            root_count: reader.u8()?,
            max_records: reader.u32()?,
            max_evidence_bytes: reader.u32()?,
        },
        READ_VOLUME_RESPONSE => {
            let client_instance = reader.array_16()?;
            let volume_id = reader.string(super::MAX_VOLUME_ID_BYTES)?;
            let journal_id = reader.u64()?;
            let requested_end_usn = reader.i64()?;
            let max_records = reader.u32()?;
            let max_evidence_bytes = reader.u32()?;
            let count = usize::from(reader.u8()?);
            if !(1..=MAX_SHARED_ROOTS).contains(&count) {
                return Err(WireError::LengthExceeded);
            }
            let mut outcomes = Vec::with_capacity(count);
            let mut total_records = 0_usize;
            let mut total_evidence = 0_usize;
            for _ in 0..count {
                let binding = reader.binding()?;
                let requested_start_usn = reader.i64()?;
                let has_failure = reader.boolean()?;
                let (covered_until_usn, is_complete, candidates, failure) = if has_failure {
                    (
                        None,
                        false,
                        Vec::new(),
                        Some(BrokerFailure::new(decode_failure_code(reader.u16()?)?)),
                    )
                } else {
                    let covered = reader.i64()?;
                    let is_complete = reader.boolean()?;
                    let candidate_count =
                        usize::try_from(reader.u32()?).map_err(|_| WireError::LengthOverflow)?;
                    total_records = total_records
                        .checked_add(candidate_count)
                        .ok_or(WireError::LengthOverflow)?;
                    if total_records > MAX_RECORDS as usize {
                        return Err(WireError::LengthExceeded);
                    }
                    let mut candidates = Vec::with_capacity(candidate_count);
                    for _ in 0..candidate_count {
                        let candidate = reader.candidate()?;
                        total_evidence = total_evidence
                            .checked_add(
                                candidate
                                    .evidence_bytes()
                                    .ok_or(WireError::LengthOverflow)?,
                            )
                            .ok_or(WireError::LengthOverflow)?;
                        if total_evidence > MAX_EVIDENCE_BYTES as usize {
                            return Err(WireError::LengthExceeded);
                        }
                        candidates.push(candidate);
                    }
                    (Some(covered), is_complete, candidates, None)
                };
                outcomes.push(SharedJournalRootOutcome {
                    binding,
                    requested_start_usn,
                    covered_until_usn,
                    is_complete,
                    candidates,
                    failure,
                });
            }
            let handoff_count =
                usize::try_from(reader.u32()?).map_err(|_| WireError::LengthOverflow)?;
            if handoff_count > MAX_SHARED_HANDOFFS {
                return Err(WireError::LengthExceeded);
            }
            let mut handoffs = Vec::with_capacity(handoff_count);
            for _ in 0..handoff_count {
                let handoff = SharedJournalHandoff {
                    previous_binding: reader.binding()?,
                    current_binding: reader.binding()?,
                    file_reference: reader.byte_vec(16)?,
                    usn: reader.i64()?,
                    previous_relative_path: reader.string(MAX_PATH_UTF16_UNITS * 4)?,
                    current_relative_path: reader.string(MAX_PATH_UTF16_UNITS * 4)?,
                    is_directory: reader.boolean()?,
                    previous_carry_id: reader.boolean()?.then(|| reader.string(512)).transpose()?,
                };
                total_evidence = total_evidence
                    .checked_add(handoff.evidence_bytes().ok_or(WireError::LengthOverflow)?)
                    .ok_or(WireError::LengthOverflow)?;
                if total_evidence > MAX_EVIDENCE_BYTES as usize {
                    return Err(WireError::LengthExceeded);
                }
                total_records = total_records
                    .checked_add(1)
                    .ok_or(WireError::LengthOverflow)?;
                if total_records > MAX_RECORDS as usize {
                    return Err(WireError::LengthExceeded);
                }
                handoffs.push(handoff);
            }
            let pending_count =
                usize::try_from(reader.u32()?).map_err(|_| WireError::LengthOverflow)?;
            if pending_count > MAX_SHARED_PENDING_RENAMES {
                return Err(WireError::LengthExceeded);
            }
            let mut pending_renames = Vec::with_capacity(pending_count);
            for _ in 0..pending_count {
                let pending = SharedJournalPendingRename {
                    binding: reader.binding()?,
                    file_reference: reader.byte_vec(16)?,
                    old_usn: reader.i64()?,
                    previous_relative_path: reader.string(MAX_PATH_UTF16_UNITS * 4)?,
                    is_directory: reader.boolean()?,
                };
                total_evidence = total_evidence
                    .checked_add(pending.evidence_bytes().ok_or(WireError::LengthOverflow)?)
                    .ok_or(WireError::LengthOverflow)?;
                if total_evidence > MAX_EVIDENCE_BYTES as usize {
                    return Err(WireError::LengthExceeded);
                }
                total_records = total_records
                    .checked_add(1)
                    .ok_or(WireError::LengthOverflow)?;
                if total_records > MAX_RECORDS as usize {
                    return Err(WireError::LengthExceeded);
                }
                pending_renames.push(pending);
            }
            BrokerResponse::ReadVolume {
                request_id: header.request_id,
                client_instance,
                volume_id,
                journal_id,
                requested_end_usn,
                max_records,
                max_evidence_bytes,
                outcomes,
                handoffs,
                pending_renames,
            }
        }
        CANCEL_RESPONSE => BrokerResponse::Cancelled {
            request_id: header.request_id,
            client_instance: reader.array_16()?,
            target_request_id: reader.u64()?,
        },
        FAILURE_RESPONSE => {
            let has_instance = reader.boolean()?;
            BrokerResponse::Failure {
                request_id: header.request_id,
                client_instance: has_instance.then(|| reader.array_16()).transpose()?,
                failure: BrokerFailure::new(decode_failure_code(reader.u16()?)?),
            }
        }
        kind => return Err(WireError::UnknownKind(kind)),
    };
    reader.finish()?;
    validate_response(&response)?;
    Ok(response)
}

fn validate_response(response: &BrokerResponse) -> Result<(), WireError> {
    if response.request_id() == 0 && !matches!(response, BrokerResponse::Failure { .. }) {
        return Err(WireError::InvalidValue("request_id"));
    }
    match response {
        BrokerResponse::RootRegistered {
            binding,
            root_capability,
            ..
        } => {
            binding.validate()?;
            root_capability.validate()?;
        }
        BrokerResponse::Journal {
            binding,
            capability,
            journal_id,
            first_usn,
            next_usn,
            ..
        } => {
            binding.validate()?;
            match capability {
                JournalCapability::Supported => {
                    if journal_id.is_none_or(|value| value == 0)
                        || first_usn.is_none_or(|value| value < 0)
                        || next_usn
                            .zip(*first_usn)
                            .is_none_or(|(next, first)| next < first)
                    {
                        return Err(WireError::InvalidValue("journal_metadata"));
                    }
                }
                JournalCapability::LiveOnly => {
                    if journal_id.is_some() || first_usn.is_some() || next_usn.is_some() {
                        return Err(WireError::InvalidValue("live_only_metadata"));
                    }
                }
            }
        }
        BrokerResponse::ReadRange {
            binding,
            journal_id,
            requested_start_usn,
            requested_end_usn,
            max_records,
            max_evidence_bytes,
            covered_until_usn,
            is_complete,
            candidates,
            ..
        } => {
            binding.validate()?;
            if *journal_id == 0
                || *requested_start_usn < 0
                || *requested_end_usn <= *requested_start_usn
                || !(1..=MAX_RECORDS).contains(max_records)
                || !(1..=MAX_EVIDENCE_BYTES).contains(max_evidence_bytes)
                || *covered_until_usn <= *requested_start_usn
                || *covered_until_usn > *requested_end_usn
                || *is_complete != (*covered_until_usn == *requested_end_usn)
                || candidates.len() > MAX_RECORDS as usize
            {
                return Err(WireError::InvalidValue("covered_range"));
            }
            let mut last_usn = None;
            for candidate in candidates {
                candidate.validate()?;
                if candidate.usn < *requested_start_usn
                    || candidate.usn >= *covered_until_usn
                    || last_usn.is_some_and(|previous| candidate.usn <= previous)
                {
                    return Err(WireError::InvalidValue("candidate_usn"));
                }
                last_usn = Some(candidate.usn);
            }
        }
        BrokerResponse::ReadRangeAccepted {
            binding,
            journal_id,
            requested_start_usn,
            requested_end_usn,
            max_records,
            max_evidence_bytes,
            ..
        } => {
            binding.validate()?;
            if *journal_id == 0
                || *requested_start_usn < 0
                || *requested_end_usn <= *requested_start_usn
                || !(1..=MAX_RECORDS).contains(max_records)
                || !(1..=MAX_EVIDENCE_BYTES).contains(max_evidence_bytes)
            {
                return Err(WireError::InvalidValue("accepted_range"));
            }
        }
        BrokerResponse::ReadVolumeAccepted {
            client_instance,
            volume_id,
            journal_id,
            requested_start_usn,
            requested_end_usn,
            root_count,
            max_records,
            max_evidence_bytes,
            ..
        } => {
            if client_instance.iter().all(|byte| *byte == 0)
                || validate_identifier_for_codec(volume_id, super::MAX_VOLUME_ID_BYTES).is_err()
                || *journal_id == 0
                || *requested_start_usn < 0
                || *requested_end_usn <= *requested_start_usn
                || !(1..=MAX_SHARED_ROOTS as u8).contains(root_count)
                || !(1..=MAX_RECORDS).contains(max_records)
                || !(1..=MAX_EVIDENCE_BYTES).contains(max_evidence_bytes)
            {
                return Err(WireError::InvalidValue("accepted_shared_range"));
            }
        }
        BrokerResponse::ReadVolume {
            client_instance,
            volume_id,
            journal_id,
            requested_end_usn,
            max_records,
            max_evidence_bytes,
            outcomes,
            handoffs,
            pending_renames,
            ..
        } => {
            if client_instance.iter().all(|byte| *byte == 0)
                || validate_identifier_for_codec(volume_id, super::MAX_VOLUME_ID_BYTES).is_err()
                || *journal_id == 0
                || *requested_end_usn <= 0
                || !(1..=MAX_RECORDS).contains(max_records)
                || !(1..=MAX_EVIDENCE_BYTES).contains(max_evidence_bytes)
                || !(1..=MAX_SHARED_ROOTS).contains(&outcomes.len())
                || handoffs.len() > MAX_SHARED_HANDOFFS
                || pending_renames.len() > MAX_SHARED_PENDING_RENAMES
            {
                return Err(WireError::InvalidValue("shared_range"));
            }
            let mut bindings = std::collections::BTreeSet::new();
            let mut records = 0_usize;
            let mut evidence = 0_usize;
            for outcome in outcomes {
                outcome.validate(*requested_end_usn, *max_records as usize)?;
                if outcome.binding.client_instance != *client_instance
                    || outcome.binding.volume_id != *volume_id
                    || !bindings.insert((
                        outcome.binding.root_id.as_str(),
                        outcome.binding.root_generation,
                    ))
                {
                    return Err(WireError::InvalidValue("shared_response_binding"));
                }
                records = records
                    .checked_add(outcome.candidates.len())
                    .ok_or(WireError::LengthOverflow)?;
                for candidate in &outcome.candidates {
                    evidence = evidence
                        .checked_add(
                            candidate
                                .evidence_bytes()
                                .ok_or(WireError::LengthOverflow)?,
                        )
                        .ok_or(WireError::LengthOverflow)?;
                }
            }
            for handoff in handoffs {
                records = records.checked_add(1).ok_or(WireError::LengthOverflow)?;
                handoff.validate(*client_instance, volume_id, *requested_end_usn)?;
                let endpoints: &[&ResponseBinding] = if handoff.previous_carry_id.is_some() {
                    &[&handoff.current_binding]
                } else {
                    &[&handoff.previous_binding, &handoff.current_binding]
                };
                for endpoint in endpoints {
                    let outcome = outcomes
                        .iter()
                        .find(|outcome| {
                            outcome.binding.root_id == endpoint.root_id
                                && outcome.binding.root_generation == endpoint.root_generation
                                && outcome.binding.client_instance == endpoint.client_instance
                                && outcome.binding.volume_id == endpoint.volume_id
                        })
                        .ok_or(WireError::InvalidValue("shared_handoff_endpoint"))?;
                    let covered = outcome
                        .covered_until_usn
                        .filter(|_| outcome.failure.is_none())
                        .ok_or(WireError::InvalidValue("shared_handoff_endpoint"))?;
                    if handoff.usn < outcome.requested_start_usn || handoff.usn >= covered {
                        return Err(WireError::InvalidValue("shared_handoff_range"));
                    }
                }
                evidence = evidence
                    .checked_add(handoff.evidence_bytes().ok_or(WireError::LengthOverflow)?)
                    .ok_or(WireError::LengthOverflow)?;
            }
            for pending in pending_renames {
                pending.validate(*client_instance, volume_id, *requested_end_usn)?;
                let outcome = outcomes
                    .iter()
                    .find(|outcome| {
                        outcome.binding.root_id == pending.binding.root_id
                            && outcome.binding.root_generation == pending.binding.root_generation
                            && outcome.binding.client_instance == pending.binding.client_instance
                            && outcome.binding.volume_id == pending.binding.volume_id
                    })
                    .ok_or(WireError::InvalidValue("shared_pending_rename_endpoint"))?;
                let covered = outcome
                    .covered_until_usn
                    .filter(|_| outcome.failure.is_none())
                    .ok_or(WireError::InvalidValue("shared_pending_rename_endpoint"))?;
                if pending.old_usn < outcome.requested_start_usn || pending.old_usn >= covered {
                    return Err(WireError::InvalidValue("shared_pending_rename_range"));
                }
                records = records.checked_add(1).ok_or(WireError::LengthOverflow)?;
                evidence = evidence
                    .checked_add(pending.evidence_bytes().ok_or(WireError::LengthOverflow)?)
                    .ok_or(WireError::LengthOverflow)?;
            }
            if records > *max_records as usize || evidence > *max_evidence_bytes as usize {
                return Err(WireError::LengthExceeded);
            }
        }
        BrokerResponse::Cancelled {
            client_instance,
            target_request_id,
            ..
        } => {
            if client_instance.iter().all(|byte| *byte == 0) || *target_request_id == 0 {
                return Err(WireError::InvalidValue("cancelled_response"));
            }
        }
        BrokerResponse::Failure {
            client_instance, ..
        } => {
            if client_instance.is_some_and(|value| value.iter().all(|byte| *byte == 0)) {
                return Err(WireError::InvalidValue("failure_binding"));
            }
        }
    }
    Ok(())
}

struct Writer {
    bytes: Vec<u8>,
}

impl Writer {
    fn new() -> Self {
        Self { bytes: Vec::new() }
    }

    fn header(&mut self, kind: u16, request_id: u64) {
        self.bytes(&PROTOCOL_MAGIC);
        self.u16(PROTOCOL_VERSION);
        self.u16(kind);
        self.u64(request_id);
    }

    fn caller(&mut self, caller: &CallerClaim) {
        self.u32(caller.process_id);
        self.u32(caller.session_id);
        self.bytes(&caller.client_instance);
    }

    fn root(&mut self, root: &RootAuthorization) -> Result<(), WireError> {
        self.string(&root.root_id)?;
        self.u64(root.root_generation);
        self.string(&root.volume_id)?;
        self.byte_vec(&root.root_identity)?;
        self.utf16(&root.canonical_root_utf16)?;
        Ok(())
    }

    fn binding(&mut self, binding: &ResponseBinding) -> Result<(), WireError> {
        self.bytes(&binding.client_instance);
        self.string(&binding.root_id)?;
        self.u64(binding.root_generation);
        self.string(&binding.volume_id)
    }

    fn root_capability(&mut self, capability: RootCapability) {
        self.bytes(&capability.0);
    }

    fn candidate(&mut self, candidate: &BrokerCandidate) -> Result<(), WireError> {
        self.scope(&candidate.scope)?;
        self.boolean(candidate.previous_scope.is_some());
        if let Some(scope) = &candidate.previous_scope {
            self.scope(scope)?;
        }
        self.byte_vec(&candidate.file_reference)?;
        self.i64(candidate.usn);
        self.u8(candidate_kind_code(candidate.kind));
        self.boolean(candidate.is_directory);
        Ok(())
    }

    fn scope(&mut self, scope: &CandidateScope) -> Result<(), WireError> {
        match scope {
            CandidateScope::Root => self.u8(0),
            CandidateScope::RelativePath(path) => {
                self.u8(1);
                self.string(path)?;
            }
        }
        Ok(())
    }

    fn optional_u64(&mut self, value: Option<u64>) {
        self.boolean(value.is_some());
        if let Some(value) = value {
            self.u64(value);
        }
    }

    fn optional_i64(&mut self, value: Option<i64>) {
        self.boolean(value.is_some());
        if let Some(value) = value {
            self.i64(value);
        }
    }

    fn utf16(&mut self, value: &[u16]) -> Result<(), WireError> {
        self.u32(u32::try_from(value.len()).map_err(|_| WireError::LengthOverflow)?);
        for unit in value {
            self.u16(*unit);
        }
        Ok(())
    }

    fn string(&mut self, value: &str) -> Result<(), WireError> {
        self.byte_vec(value.as_bytes())
    }

    fn byte_vec(&mut self, value: &[u8]) -> Result<(), WireError> {
        self.u32(u32::try_from(value.len()).map_err(|_| WireError::LengthOverflow)?);
        self.bytes(value);
        Ok(())
    }

    fn boolean(&mut self, value: bool) {
        self.u8(u8::from(value));
    }

    fn bytes(&mut self, value: &[u8]) {
        self.bytes.extend_from_slice(value);
    }

    fn u8(&mut self, value: u8) {
        self.bytes.push(value);
    }

    fn u16(&mut self, value: u16) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn u32(&mut self, value: u32) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn u64(&mut self, value: u64) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn i64(&mut self, value: i64) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn finish(self) -> Vec<u8> {
        self.bytes
    }
}

struct Reader<'a> {
    payload: &'a [u8],
    cursor: usize,
    kind: u16,
}

impl<'a> Reader<'a> {
    fn after_header(payload: &'a [u8]) -> Result<Self, WireError> {
        Ok(Self {
            payload,
            cursor: 20,
            kind: read_u16_at(payload, 10)?,
        })
    }

    fn caller(&mut self) -> Result<CallerClaim, WireError> {
        Ok(CallerClaim {
            process_id: self.u32()?,
            session_id: self.u32()?,
            client_instance: self.array_16()?,
        })
    }

    fn root(&mut self) -> Result<RootAuthorization, WireError> {
        Ok(RootAuthorization {
            root_id: self.string(super::MAX_ROOT_ID_BYTES)?,
            root_generation: self.u64()?,
            volume_id: self.string(super::MAX_VOLUME_ID_BYTES)?,
            root_identity: self.byte_vec(16)?,
            canonical_root_utf16: self.utf16(MAX_PATH_UTF16_UNITS)?,
        })
    }

    fn binding(&mut self) -> Result<ResponseBinding, WireError> {
        Ok(ResponseBinding {
            client_instance: self.array_16()?,
            root_id: self.string(super::MAX_ROOT_ID_BYTES)?,
            root_generation: self.u64()?,
            volume_id: self.string(super::MAX_VOLUME_ID_BYTES)?,
        })
    }

    fn root_capability(&mut self) -> Result<RootCapability, WireError> {
        Ok(RootCapability(self.array_32()?))
    }

    fn candidate(&mut self) -> Result<BrokerCandidate, WireError> {
        let scope = self.scope()?;
        let previous_scope = self.boolean()?.then(|| self.scope()).transpose()?;
        Ok(BrokerCandidate {
            scope,
            previous_scope,
            file_reference: self.byte_vec(16)?,
            usn: self.i64()?,
            kind: decode_candidate_kind(self.u8()?)?,
            is_directory: self.boolean()?,
        })
    }

    fn scope(&mut self) -> Result<CandidateScope, WireError> {
        match self.u8()? {
            0 => Ok(CandidateScope::Root),
            1 => Ok(CandidateScope::RelativePath(
                self.string(MAX_PATH_UTF16_UNITS * 3)?,
            )),
            _ => Err(WireError::InvalidValue("candidate_scope")),
        }
    }

    fn optional_u64(&mut self) -> Result<Option<u64>, WireError> {
        self.boolean()?.then(|| self.u64()).transpose()
    }

    fn optional_i64(&mut self) -> Result<Option<i64>, WireError> {
        self.boolean()?.then(|| self.i64()).transpose()
    }

    fn utf16(&mut self, maximum: usize) -> Result<Vec<u16>, WireError> {
        let length = usize::try_from(self.u32()?).map_err(|_| WireError::LengthOverflow)?;
        if length > maximum {
            return Err(WireError::LengthExceeded);
        }
        let byte_length = length.checked_mul(2).ok_or(WireError::LengthOverflow)?;
        let bytes = self.take(byte_length)?;
        let mut value = Vec::with_capacity(length);
        for chunk in bytes.chunks_exact(2) {
            value.push(u16::from_le_bytes([chunk[0], chunk[1]]));
        }
        Ok(value)
    }

    fn string(&mut self, maximum: usize) -> Result<String, WireError> {
        String::from_utf8(self.byte_vec(maximum)?).map_err(|_| WireError::InvalidUtf8)
    }

    fn byte_vec(&mut self, maximum: usize) -> Result<Vec<u8>, WireError> {
        let length = usize::try_from(self.u32()?).map_err(|_| WireError::LengthOverflow)?;
        if length > maximum {
            return Err(WireError::LengthExceeded);
        }
        Ok(self.take(length)?.to_vec())
    }

    fn array_16(&mut self) -> Result<[u8; 16], WireError> {
        self.take(16)?
            .try_into()
            .map_err(|_| WireError::UnexpectedEnd)
    }

    fn array_32(&mut self) -> Result<[u8; 32], WireError> {
        self.take(32)?
            .try_into()
            .map_err(|_| WireError::UnexpectedEnd)
    }

    fn boolean(&mut self) -> Result<bool, WireError> {
        match self.u8()? {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(WireError::InvalidValue("boolean")),
        }
    }

    fn u8(&mut self) -> Result<u8, WireError> {
        Ok(self.take(1)?[0])
    }

    fn u16(&mut self) -> Result<u16, WireError> {
        self.take(2)?
            .try_into()
            .map(u16::from_le_bytes)
            .map_err(|_| WireError::UnexpectedEnd)
    }

    fn u32(&mut self) -> Result<u32, WireError> {
        self.take(4)?
            .try_into()
            .map(u32::from_le_bytes)
            .map_err(|_| WireError::UnexpectedEnd)
    }

    fn u64(&mut self) -> Result<u64, WireError> {
        self.take(8)?
            .try_into()
            .map(u64::from_le_bytes)
            .map_err(|_| WireError::UnexpectedEnd)
    }

    fn i64(&mut self) -> Result<i64, WireError> {
        self.take(8)?
            .try_into()
            .map(i64::from_le_bytes)
            .map_err(|_| WireError::UnexpectedEnd)
    }

    fn take(&mut self, length: usize) -> Result<&'a [u8], WireError> {
        let end = self
            .cursor
            .checked_add(length)
            .ok_or(WireError::LengthOverflow)?;
        let value = self
            .payload
            .get(self.cursor..end)
            .ok_or(WireError::UnexpectedEnd)?;
        self.cursor = end;
        Ok(value)
    }

    fn finish(self) -> Result<(), WireError> {
        if self.cursor == self.payload.len() {
            Ok(())
        } else {
            Err(WireError::TrailingBytes)
        }
    }
}

fn read_u16_at(payload: &[u8], offset: usize) -> Result<u16, WireError> {
    payload
        .get(offset..offset + 2)
        .ok_or(WireError::UnexpectedEnd)?
        .try_into()
        .map(u16::from_le_bytes)
        .map_err(|_| WireError::UnexpectedEnd)
}

fn read_u64_at(payload: &[u8], offset: usize) -> Result<u64, WireError> {
    payload
        .get(offset..offset + 8)
        .ok_or(WireError::UnexpectedEnd)?
        .try_into()
        .map(u64::from_le_bytes)
        .map_err(|_| WireError::UnexpectedEnd)
}

fn capability_code(value: JournalCapability) -> u8 {
    match value {
        JournalCapability::Supported => 1,
        JournalCapability::LiveOnly => 2,
    }
}

fn decode_capability(value: u8) -> Result<JournalCapability, WireError> {
    match value {
        1 => Ok(JournalCapability::Supported),
        2 => Ok(JournalCapability::LiveOnly),
        _ => Err(WireError::InvalidValue("journal_capability")),
    }
}

fn candidate_kind_code(value: CandidateKind) -> u8 {
    match value {
        CandidateKind::Path => 1,
        CandidateKind::Subtree => 2,
        CandidateKind::Rename => 3,
    }
}

fn decode_candidate_kind(value: u8) -> Result<CandidateKind, WireError> {
    match value {
        1 => Ok(CandidateKind::Path),
        2 => Ok(CandidateKind::Subtree),
        3 => Ok(CandidateKind::Rename),
        _ => Err(WireError::InvalidValue("candidate_kind")),
    }
}

fn failure_code(value: BrokerFailureCode) -> u16 {
    use BrokerFailureCode as Code;
    match value {
        Code::ProtocolMismatch => 1,
        Code::MalformedFrame => 2,
        Code::RequestTooLarge => 3,
        Code::CallerRejected => 4,
        Code::RootUnauthorized => 5,
        Code::RootIdentityMismatch => 6,
        Code::VolumeMismatch => 7,
        Code::JournalUnavailable => 8,
        Code::JournalDiscontinuous => 9,
        Code::RecordUnsupported => 10,
        Code::EvidenceLimitExceeded => 11,
        Code::TimedOut => 12,
        Code::Cancelled => 13,
        Code::RequestNotActive => 14,
        Code::ActiveLimitExceeded => 15,
        Code::BackendUnavailable => 16,
        Code::Internal => 17,
        Code::TerminalDeliveryBackpressure => 18,
    }
}

fn decode_failure_code(value: u16) -> Result<BrokerFailureCode, WireError> {
    use BrokerFailureCode as Code;
    match value {
        1 => Ok(Code::ProtocolMismatch),
        2 => Ok(Code::MalformedFrame),
        3 => Ok(Code::RequestTooLarge),
        4 => Ok(Code::CallerRejected),
        5 => Ok(Code::RootUnauthorized),
        6 => Ok(Code::RootIdentityMismatch),
        7 => Ok(Code::VolumeMismatch),
        8 => Ok(Code::JournalUnavailable),
        9 => Ok(Code::JournalDiscontinuous),
        10 => Ok(Code::RecordUnsupported),
        11 => Ok(Code::EvidenceLimitExceeded),
        12 => Ok(Code::TimedOut),
        13 => Ok(Code::Cancelled),
        14 => Ok(Code::RequestNotActive),
        15 => Ok(Code::ActiveLimitExceeded),
        16 => Ok(Code::BackendUnavailable),
        17 => Ok(Code::Internal),
        18 => Ok(Code::TerminalDeliveryBackpressure),
        _ => Err(WireError::InvalidValue("failure_code")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn registration() -> RegisterRootRequest {
        RegisterRootRequest {
            caller: CallerClaim {
                process_id: 41,
                session_id: 2,
                client_instance: [3; 16],
            },
            root: RootAuthorization {
                root_id: "root-a".to_owned(),
                root_generation: 7,
                volume_id: r"\\?\Volume{00000000-0000-0000-0000-000000000001}\".to_owned(),
                root_identity: vec![4; 16],
                canonical_root_utf16: r"C:\Photos".encode_utf16().collect(),
            },
            client_root_handle: 55,
            timeout_ms: 1_000,
        }
    }

    #[test]
    fn v5_registration_and_capability_round_trip_and_v2_v3_v4_are_rejected() {
        let request = BrokerRequest::RegisterRoot {
            request_id: 9,
            request: registration(),
        };
        assert_eq!(
            decode_request(&encode_request(&request).expect("encode")),
            Ok(request)
        );

        let response = BrokerResponse::RootRegistered {
            request_id: 9,
            binding: ResponseBinding::from_request(&registration().caller, &registration().root),
            root_capability: RootCapability([8; 32]),
        };
        assert_eq!(
            decode_response(&encode_response(&response).expect("encode response")),
            Ok(response)
        );

        let mut old = encode_request(&BrokerRequest::RegisterRoot {
            request_id: 10,
            request: registration(),
        })
        .expect("encode old fixture");
        old[..8].copy_from_slice(&LEGACY_PROTOCOL_MAGIC_V2);
        old[8..10].copy_from_slice(&2_u16.to_le_bytes());
        assert!(matches!(
            decode_request(&old),
            Err(WireError::UnsupportedVersion(2))
        ));
        old[..8].copy_from_slice(&LEGACY_PROTOCOL_MAGIC_V3);
        old[8..10].copy_from_slice(&3_u16.to_le_bytes());
        assert!(matches!(
            decode_request(&old),
            Err(WireError::UnsupportedVersion(3))
        ));
        old[..8].copy_from_slice(&LEGACY_PROTOCOL_MAGIC_V4);
        old[8..10].copy_from_slice(&4_u16.to_le_bytes());
        assert!(matches!(
            decode_request(&old),
            Err(WireError::UnsupportedVersion(4))
        ));
    }

    #[test]
    fn terminal_delivery_backpressure_has_fixed_wire_code() {
        assert_eq!(
            failure_code(BrokerFailureCode::TerminalDeliveryBackpressure),
            18
        );
        assert_eq!(
            decode_failure_code(18),
            Ok(BrokerFailureCode::TerminalDeliveryBackpressure)
        );
    }

    #[test]
    fn decode_rejects_rename_with_previous_root_scope() {
        let mut writer = Writer::new();
        writer.header(READ_RESPONSE, 44);
        writer
            .binding(&ResponseBinding {
                client_instance: [9; 16],
                root_id: "root-a".to_owned(),
                root_generation: 3,
                volume_id: "volume-a".to_owned(),
            })
            .expect("binding");
        writer.u64(7);
        writer.i64(10);
        writer.i64(20);
        writer.u32(1);
        writer.u32(512);
        writer.i64(20);
        writer.boolean(true);
        writer.u32(1);
        writer
            .scope(&CandidateScope::RelativePath("inside.jpg".to_owned()))
            .expect("current scope");
        writer.boolean(true);
        writer.scope(&CandidateScope::Root).expect("previous root");
        writer.byte_vec(&[1; 8]).expect("file reference");
        writer.i64(11);
        writer.u8(candidate_kind_code(CandidateKind::Rename));
        writer.boolean(true);

        assert!(matches!(
            decode_response(&writer.finish()),
            Err(WireError::InvalidValue("previous_root_candidate"))
        ));
    }

    #[test]
    fn decode_rejects_handoff_bound_to_a_failed_endpoint() {
        let client_instance = [9; 16];
        let previous = ResponseBinding {
            client_instance,
            root_id: "root-a".to_owned(),
            root_generation: 1,
            volume_id: "volume-a".to_owned(),
        };
        let current = ResponseBinding {
            client_instance,
            root_id: "root-b".to_owned(),
            root_generation: 2,
            volume_id: "volume-a".to_owned(),
        };
        let mut writer = Writer::new();
        writer.header(READ_VOLUME_RESPONSE, 44);
        writer.bytes(&client_instance);
        writer.string("volume-a").expect("volume");
        writer.u64(7);
        writer.i64(20);
        writer.u32(32);
        writer.u32(4_096);
        writer.u8(2);
        writer.binding(&previous).expect("previous binding");
        writer.i64(10);
        writer.boolean(false);
        writer.i64(20);
        writer.boolean(true);
        writer.u32(0);
        writer.binding(&current).expect("current binding");
        writer.i64(12);
        writer.boolean(true);
        writer.u16(failure_code(BrokerFailureCode::RootUnauthorized));
        writer.u32(1);
        writer.binding(&previous).expect("handoff previous");
        writer.binding(&current).expect("handoff current");
        writer.byte_vec(&[7; 16]).expect("file reference");
        writer.i64(15);
        writer.string("source.jpg").expect("previous path");
        writer.string("moved.jpg").expect("current path");
        writer.boolean(false);
        writer.boolean(false);
        writer.u32(0);

        assert!(matches!(
            decode_response(&writer.finish()),
            Err(WireError::InvalidValue("shared_handoff_endpoint"))
        ));
    }
}
