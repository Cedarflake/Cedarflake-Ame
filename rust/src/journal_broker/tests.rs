use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use super::client::{
    AdapterPoll, BrokerClientError, BrokerClientTransport, JournalBrokerClient,
    OwnedBrokerConnection, sealed,
};
use super::framing::{MAX_FRAME_BYTES, decode_frame, decode_frame_length, encode_frame};
use super::wire::{
    BrokerCandidate, BrokerFailure, BrokerFailureCode, BrokerResponse, CallerClaim, CandidateKind,
    CandidateScope, JournalCapability, LEGACY_PROTOCOL_MAGIC_V1, MAX_EVIDENCE_BYTES, MAX_RECORDS,
    NameComparisonSemantics, PROTOCOL_VERSION, QueryJournalRequest, ReadJournalRangeRequest,
    ReadJournalVolumeRequest, RegisterRootRequest, ResponseBinding, RootAuthorization,
    RootCapability, SharedJournalHandoff, SharedJournalPendingRenameRequest,
    SharedJournalRootOutcome, SharedJournalRootRequest, decode_request, decode_response,
    encode_request, encode_response, scope_under_root, validate_root_path,
};
use super::{
    PersistentChangeJournalConnection, PersistentChangeJournalOperationError,
    PersistentChangeJournalSession,
};

#[test]
fn frame_header_is_rejected_before_payload_allocation() {
    assert!(matches!(decode_frame_length(1_u32.to_le_bytes()), Ok(1)));
    let oversized = u32::try_from(MAX_FRAME_BYTES + 1).expect("frame limit fits u32");
    assert!(decode_frame_length(oversized.to_le_bytes()).is_err());
    assert!(decode_frame_length(0_u32.to_le_bytes()).is_err());
}

#[test]
fn shared_volume_request_round_trips_and_rejects_cross_volume_or_root_replay() {
    let request = shared_volume_request();
    let encoded = encode_request(&super::wire::BrokerRequest::ReadVolume {
        request_id: 88,
        request: request.clone(),
    })
    .expect("encode shared request");
    assert_eq!(
        decode_request(&encoded).expect("decode shared request"),
        super::wire::BrokerRequest::ReadVolume {
            request_id: 88,
            request: request.clone(),
        }
    );
    let mut malformed_root_count = encoded.clone();
    malformed_root_count[44] = 9;
    assert!(decode_request(&malformed_root_count).is_err());

    let mut cross_volume = request.clone();
    cross_volume.roots[1].root.volume_id = "volume-b".to_owned();
    assert!(
        encode_request(&super::wire::BrokerRequest::ReadVolume {
            request_id: 89,
            request: cross_volume,
        })
        .is_err()
    );

    let mut replay = request;
    replay.roots[1].root.root_id = replay.roots[0].root.root_id.clone();
    replay.roots[1].root.root_generation = replay.roots[0].root.root_generation;
    assert!(
        encode_request(&super::wire::BrokerRequest::ReadVolume {
            request_id: 90,
            request: replay,
        })
        .is_err()
    );
}

fn shared_volume_request() -> ReadJournalVolumeRequest {
    let first = root();
    let mut second = first.clone();
    second.root_id = "root-b".to_owned();
    second.root_generation = 4;
    second.root_identity = vec![2; 16];
    second.canonical_root_utf16 = "C:\\Other".encode_utf16().collect();
    ReadJournalVolumeRequest {
        caller: caller(),
        roots: vec![
            SharedJournalRootRequest {
                root: first.clone(),
                root_capability: RootCapability([7; 32]),
                start_usn: 10,
            },
            SharedJournalRootRequest {
                root: second,
                root_capability: RootCapability([8; 32]),
                start_usn: 12,
            },
        ],
        pending_renames: vec![SharedJournalPendingRenameRequest {
            carry_id: "carry-a".to_owned(),
            source_range_id: "range-a".to_owned(),
            root: first,
            root_capability: Some(RootCapability([7; 32])),
            file_reference: vec![3; 16],
            old_usn: 9,
            previous_relative_path: "old.jpg".to_owned(),
            is_directory: false,
        }],
        journal_id: 7,
        end_usn: 20,
        max_records: 32,
        max_evidence_bytes: 4_096,
        timeout_ms: 1_000,
    }
}

#[test]
fn path_validation_rejects_all_unsupported_windows_namespaces_and_ads() {
    let rejected = [
        r"\\server\share\Pictures",
        r"\\?\UNC\server\share\Pictures",
        r"\\?\GLOBALROOT\Device\HarddiskVolumeShadowCopy1",
        r"\\.\C:\Pictures",
        r"\\?\C:\Pictures",
        r"C:\Pictures\file.jpg:stream",
        r"C:\Pictures\\file.jpg",
        r"C:\Pictures\.\file.jpg",
        r"C:\Pictures\..\file.jpg",
        "C:\\Pictures\\trailing ",
        "C:\\Pictures\\trailing.",
        r"C:\Pictures\NUL.jpg",
        r"C:\Pictures\COM1",
        "中:\\Pictures",
    ];
    for path in rejected {
        assert!(
            validate_root_path(&path.encode_utf16().collect::<Vec<_>>()).is_err(),
            "path should be rejected: {path:?}"
        );
    }
}

#[test]
fn case_sensitive_containment_keeps_photos_siblings_distinct() {
    #[cfg(windows)]
    super::windows::name::reset_call_count();
    let root =
        validate_root_path(&"C:\\Root\\Photos".encode_utf16().collect::<Vec<_>>()).expect("root");
    let sibling = validate_root_path(&"C:\\Root\\photos\\A.jpg".encode_utf16().collect::<Vec<_>>())
        .expect("sibling");
    assert!(matches!(
        scope_under_root(
            &sibling,
            &root,
            &[
                NameComparisonSemantics::OrdinalCaseSensitive,
                NameComparisonSemantics::OrdinalCaseSensitive,
            ]
        ),
        Ok(None)
    ));
    #[cfg(windows)]
    assert_eq!(super::windows::name::call_count(), 0);
}

#[cfg(windows)]
#[test]
fn windows_ordinal_ignore_case_allows_legal_case_difference_and_root_self() {
    let root =
        validate_root_path(&"C:\\Pictures".encode_utf16().collect::<Vec<_>>()).expect("root");
    let same =
        validate_root_path(&"c:\\PICTURES".encode_utf16().collect::<Vec<_>>()).expect("same root");
    let child = validate_root_path(
        &"c:\\pictures\\相册\\A.jpg"
            .encode_utf16()
            .collect::<Vec<_>>(),
    )
    .expect("child");
    assert!(matches!(
        scope_under_root(
            &same,
            &root,
            &[NameComparisonSemantics::WindowsOrdinalIgnoreCase]
        ),
        Ok(Some(super::wire::RootRelativeScope::Root))
    ));
    assert!(matches!(
        scope_under_root(
            &child,
            &root,
            &[NameComparisonSemantics::WindowsOrdinalIgnoreCase]
        ),
        Ok(Some(super::wire::RootRelativeScope::Relative(path))) if path == "相册/A.jpg"
    ));
}

#[cfg(windows)]
#[test]
fn windows_ordinal_ignore_case_does_not_apply_multichar_unicode_uppercase() {
    let root =
        validate_root_path(&"C:\\Straße".encode_utf16().collect::<Vec<_>>()).expect("unicode root");
    let sibling = validate_root_path(&"C:\\STRASSE\\A.jpg".encode_utf16().collect::<Vec<_>>())
        .expect("unicode sibling");
    assert!(matches!(
        scope_under_root(
            &sibling,
            &root,
            &[NameComparisonSemantics::WindowsOrdinalIgnoreCase]
        ),
        Ok(None)
    ));
}

#[test]
fn client_accepts_bound_query_and_read_responses() {
    let query_client = client_for(BrokerResponse::Journal {
        request_id: 1,
        binding: binding(),
        capability: JournalCapability::Supported,
        journal_id: Some(7),
        first_usn: Some(0),
        next_usn: Some(20),
    });
    assert!(matches!(
        query_client.query_journal(query_request()),
        Ok(BrokerResponse::Journal {
            journal_id: Some(7),
            ..
        })
    ));

    let read_client = client_for(read_response(binding(), 7, 10, 20, 20));
    assert!(matches!(
        read_client.read_range(read_request()),
        Ok(BrokerResponse::ReadRange {
            covered_until_usn: 20,
            ..
        })
    ));
}

#[test]
fn client_rejects_same_id_wrong_root_generation_volume_journal_and_boundaries() {
    let mut wrong_client = binding();
    wrong_client.client_instance = [8; 16];
    let mut wrong_root = binding();
    wrong_root.root_id = "root-b".to_owned();
    let mut wrong_generation = binding();
    wrong_generation.root_generation = 4;
    let mut wrong_volume = binding();
    wrong_volume.volume_id = "volume-b".to_owned();
    let responses = [
        read_response(wrong_client, 7, 10, 20, 20),
        read_response(wrong_root, 7, 10, 20, 20),
        read_response(wrong_generation, 7, 10, 20, 20),
        read_response(wrong_volume, 7, 10, 20, 20),
        read_response(binding(), 8, 10, 20, 20),
        read_response(binding(), 7, 11, 20, 20),
        read_response(binding(), 7, 10, 21, 21),
    ];
    for response in responses {
        let client = client_for(response);
        assert!(matches!(
            client.read_range(read_request()),
            Err(BrokerClientError::ResponseBindingMismatch)
                | Err(BrokerClientError::RangeBindingMismatch)
        ));
    }
}

#[test]
fn shared_client_rejects_outcomes_not_bound_to_original_root_starts() {
    let request = shared_volume_request();
    let accepted = BrokerResponse::ReadVolumeAccepted {
        request_id: 1,
        client_instance: request.caller.client_instance,
        volume_id: request.roots[0].root.volume_id.clone(),
        journal_id: request.journal_id,
        requested_start_usn: request.minimum_start_usn(),
        requested_end_usn: request.end_usn,
        root_count: 2,
        max_records: request.max_records,
        max_evidence_bytes: request.max_evidence_bytes,
    };
    let bindings = request
        .roots
        .iter()
        .map(|root| ResponseBinding::from_request(&request.caller, &root.root))
        .collect::<Vec<_>>();
    let final_response = BrokerResponse::ReadVolume {
        request_id: 1,
        client_instance: request.caller.client_instance,
        volume_id: request.roots[0].root.volume_id.clone(),
        journal_id: request.journal_id,
        requested_end_usn: request.end_usn,
        max_records: request.max_records,
        max_evidence_bytes: request.max_evidence_bytes,
        outcomes: vec![
            SharedJournalRootOutcome {
                binding: bindings[0].clone(),
                requested_start_usn: 11,
                covered_until_usn: Some(20),
                is_complete: true,
                candidates: Vec::new(),
                failure: None,
            },
            SharedJournalRootOutcome {
                binding: bindings[1].clone(),
                requested_start_usn: 12,
                covered_until_usn: Some(20),
                is_complete: true,
                candidates: Vec::new(),
                failure: None,
            },
        ],
        handoffs: vec![SharedJournalHandoff {
            previous_binding: bindings[0].clone(),
            current_binding: bindings[1].clone(),
            file_reference: vec![7; 16],
            usn: 15,
            previous_relative_path: "source.jpg".to_owned(),
            current_relative_path: "moved.jpg".to_owned(),
            is_directory: false,
            previous_carry_id: None,
        }],
        pending_renames: Vec::new(),
    };
    let client = broker_client(QueueTransport::new(vec![
        encode_frame(&encode_response(&accepted).expect("accepted response"))
            .expect("accepted frame"),
        encode_frame(&encode_response(&final_response).expect("final response"))
            .expect("final frame"),
    ]));

    let pending = client
        .begin_read_volume(request)
        .expect("accepted shared read");
    assert!(matches!(
        pending.wait(),
        Err(BrokerClientError::RangeBindingMismatch)
    ));
}

#[test]
fn shared_client_rejects_carried_handoff_not_bound_to_original_pending_old() {
    let request = shared_volume_request();
    let accepted = BrokerResponse::ReadVolumeAccepted {
        request_id: 1,
        client_instance: request.caller.client_instance,
        volume_id: request.roots[0].root.volume_id.clone(),
        journal_id: request.journal_id,
        requested_start_usn: request.minimum_start_usn(),
        requested_end_usn: request.end_usn,
        root_count: 2,
        max_records: request.max_records,
        max_evidence_bytes: request.max_evidence_bytes,
    };
    let bindings = request
        .roots
        .iter()
        .map(|root| ResponseBinding::from_request(&request.caller, &root.root))
        .collect::<Vec<_>>();
    let outcomes = request
        .roots
        .iter()
        .zip(&bindings)
        .map(|(root, binding)| SharedJournalRootOutcome {
            binding: binding.clone(),
            requested_start_usn: root.start_usn,
            covered_until_usn: Some(request.end_usn),
            is_complete: true,
            candidates: Vec::new(),
            failure: None,
        })
        .collect::<Vec<_>>();
    let carry = &request.pending_renames[0];
    for malformed in [
        SharedJournalHandoff {
            previous_binding: ResponseBinding::from_request(&request.caller, &carry.root),
            current_binding: bindings[1].clone(),
            file_reference: carry.file_reference.clone(),
            usn: 15,
            previous_relative_path: "tampered.jpg".to_owned(),
            current_relative_path: "moved.jpg".to_owned(),
            is_directory: false,
            previous_carry_id: Some(carry.carry_id.clone()),
        },
        SharedJournalHandoff {
            previous_binding: ResponseBinding::from_request(&request.caller, &carry.root),
            current_binding: bindings[1].clone(),
            file_reference: vec![4; 16],
            usn: 15,
            previous_relative_path: carry.previous_relative_path.clone(),
            current_relative_path: "moved.jpg".to_owned(),
            is_directory: false,
            previous_carry_id: Some(carry.carry_id.clone()),
        },
        SharedJournalHandoff {
            previous_binding: ResponseBinding::from_request(&request.caller, &carry.root),
            current_binding: bindings[1].clone(),
            file_reference: carry.file_reference.clone(),
            usn: 15,
            previous_relative_path: carry.previous_relative_path.clone(),
            current_relative_path: "moved.jpg".to_owned(),
            is_directory: false,
            previous_carry_id: Some("unknown-carry".to_owned()),
        },
    ] {
        let response = BrokerResponse::ReadVolume {
            request_id: 1,
            client_instance: request.caller.client_instance,
            volume_id: request.roots[0].root.volume_id.clone(),
            journal_id: request.journal_id,
            requested_end_usn: request.end_usn,
            max_records: request.max_records,
            max_evidence_bytes: request.max_evidence_bytes,
            outcomes: outcomes.clone(),
            handoffs: vec![malformed],
            pending_renames: Vec::new(),
        };
        let client = broker_client(QueueTransport::new(vec![
            encode_frame(&encode_response(&accepted).expect("accepted response"))
                .expect("accepted frame"),
            encode_frame(&encode_response(&response).expect("carried response"))
                .expect("carried frame"),
        ]));
        let pending = client
            .begin_read_volume(request.clone())
            .expect("accepted shared read");
        assert!(matches!(
            pending.wait(),
            Err(BrokerClientError::RangeBindingMismatch)
        ));
    }
}

#[test]
fn client_rejects_cancelled_variant_and_cancelled_failure_for_read() {
    let variant = client_for(BrokerResponse::Cancelled {
        request_id: 1,
        client_instance: [9; 16],
        target_request_id: 77,
    });
    assert!(matches!(
        variant.read_range(read_request()),
        Err(BrokerClientError::UnexpectedResponse)
    ));

    let failure = client_for(BrokerResponse::Failure {
        request_id: 1,
        client_instance: Some([9; 16]),
        failure: BrokerFailure {
            code: BrokerFailureCode::Cancelled,
        },
    });
    assert!(matches!(
        failure.read_range(read_request()),
        Err(BrokerClientError::Cancelled)
    ));
}

#[test]
fn client_distinguishes_protocol_mismatch_and_malformed_response() {
    let response = BrokerResponse::Failure {
        request_id: 1,
        client_instance: None,
        failure: BrokerFailure {
            code: BrokerFailureCode::ProtocolMismatch,
        },
    };
    let mut payload = encode_response(&response).expect("encode response");
    payload[8..10].copy_from_slice(&(PROTOCOL_VERSION + 1).to_le_bytes());
    let mismatch = broker_client(QueueTransport::new(vec![
        encode_frame(&payload).expect("frame"),
    ]));
    assert!(matches!(
        mismatch.query_journal(query_request()),
        Err(BrokerClientError::ProtocolMismatch)
    ));

    let response = BrokerResponse::Failure {
        request_id: 1,
        client_instance: None,
        failure: BrokerFailure {
            code: BrokerFailureCode::ProtocolMismatch,
        },
    };
    let mut legacy_payload = encode_response(&response).expect("encode legacy response");
    legacy_payload[..8].copy_from_slice(&LEGACY_PROTOCOL_MAGIC_V1);
    let legacy = broker_client(QueueTransport::new(vec![
        encode_frame(&legacy_payload).expect("legacy frame"),
    ]));
    assert!(matches!(
        legacy.query_journal(query_request()),
        Err(BrokerClientError::ProtocolMismatch)
    ));

    let malformed = broker_client(QueueTransport::new(vec![
        encode_frame(b"not-a-protocol-payload").expect("frame"),
    ]));
    assert!(matches!(
        malformed.query_journal(query_request()),
        Err(BrokerClientError::MalformedResponse)
    ));
}

#[test]
fn failure_codes_have_fixed_wire_values() {
    let cases = [
        (BrokerFailureCode::ProtocolMismatch, 1_u16),
        (BrokerFailureCode::MalformedFrame, 2),
        (BrokerFailureCode::RequestTooLarge, 3),
        (BrokerFailureCode::CallerRejected, 4),
        (BrokerFailureCode::RootUnauthorized, 5),
        (BrokerFailureCode::RootIdentityMismatch, 6),
        (BrokerFailureCode::VolumeMismatch, 7),
        (BrokerFailureCode::JournalUnavailable, 8),
        (BrokerFailureCode::JournalDiscontinuous, 9),
        (BrokerFailureCode::RecordUnsupported, 10),
        (BrokerFailureCode::EvidenceLimitExceeded, 11),
        (BrokerFailureCode::TimedOut, 12),
        (BrokerFailureCode::Cancelled, 13),
        (BrokerFailureCode::RequestNotActive, 14),
        (BrokerFailureCode::ActiveLimitExceeded, 15),
        (BrokerFailureCode::BackendUnavailable, 16),
        (BrokerFailureCode::Internal, 17),
    ];
    for (code, expected) in cases {
        let response = BrokerResponse::Failure {
            request_id: 1,
            client_instance: Some([9; 16]),
            failure: BrokerFailure { code },
        };
        let payload = encode_response(&response).expect("encode failure");
        assert_eq!(&payload[payload.len() - 2..], &expected.to_le_bytes());
        assert!(matches!(
            decode_response(&payload),
            Ok(BrokerResponse::Failure {
                failure: BrokerFailure { code: decoded },
                ..
            }) if decoded == code
        ));
    }
}

#[test]
fn client_close_is_fail_closed_and_idempotent() {
    let client = broker_client(QueueTransport::new(vec![]));
    client.close().expect("first close");
    client.close().expect("second close");
    assert!(matches!(
        client.query_journal(query_request()),
        Err(BrokerClientError::Closed)
    ));
}

#[test]
fn candidate_kind_scope_and_directory_combinations_fail_closed() {
    let relative = CandidateScope::RelativePath("A.jpg".to_owned());
    let previous = CandidateScope::RelativePath("Old.jpg".to_owned());
    let invalid = [
        candidate(
            relative.clone(),
            Some(previous.clone()),
            CandidateKind::Path,
            false,
        ),
        candidate(relative.clone(), None, CandidateKind::Rename, false),
        candidate(relative.clone(), None, CandidateKind::Subtree, false),
        candidate(CandidateScope::Root, None, CandidateKind::Path, true),
        candidate(CandidateScope::Root, None, CandidateKind::Subtree, false),
        candidate(
            relative.clone(),
            Some(CandidateScope::Root),
            CandidateKind::Rename,
            true,
        ),
    ];
    for candidate in invalid {
        assert!(encode_response(&response_with_candidate(candidate)).is_err());
    }

    let rename = candidate(relative, Some(previous), CandidateKind::Rename, false);
    assert!(encode_response(&response_with_candidate(rename)).is_ok());
    let subtree = candidate(
        CandidateScope::RelativePath("Album".to_owned()),
        None,
        CandidateKind::Subtree,
        true,
    );
    assert!(encode_response(&response_with_candidate(subtree)).is_ok());
}

#[test]
fn client_rejects_wire_rename_with_previous_root_scope() {
    let previous = "Old.jpg";
    let response = response_with_candidate(candidate(
        CandidateScope::RelativePath("New.jpg".to_owned()),
        Some(CandidateScope::RelativePath(previous.to_owned())),
        CandidateKind::Rename,
        false,
    ));
    let mut payload = encode_response(&response).expect("valid rename response");
    let bytes_after_previous = 4 + 8 + 8 + 1 + 1;
    let previous_tag = payload.len() - bytes_after_previous - previous.len() - 4 - 1;
    payload[previous_tag] = 0;
    payload.drain(previous_tag + 1..previous_tag + 1 + 4 + previous.len());
    let accepted = encode_response(&accepted_response()).expect("accepted response");
    let client = broker_client(QueueTransport::new(vec![
        encode_frame(&accepted).expect("accepted frame"),
        encode_frame(&payload).expect("malformed candidate frame"),
    ]));
    assert!(matches!(
        client.read_range(read_request()),
        Err(BrokerClientError::MalformedResponse)
    ));
}

#[test]
fn client_rejects_broker_evidence_above_original_request_limits() {
    let mut request = read_request();
    request.max_records = 1;
    let accepted = BrokerResponse::ReadRangeAccepted {
        request_id: 1,
        binding: binding(),
        journal_id: 7,
        requested_start_usn: 10,
        requested_end_usn: 20,
        max_records: 1,
        max_evidence_bytes: 4_096,
    };
    let final_response = BrokerResponse::ReadRange {
        request_id: 1,
        binding: binding(),
        journal_id: 7,
        requested_start_usn: 10,
        requested_end_usn: 20,
        max_records: 1,
        max_evidence_bytes: 4_096,
        covered_until_usn: 20,
        is_complete: true,
        candidates: vec![
            candidate(
                CandidateScope::RelativePath("A.jpg".to_owned()),
                None,
                CandidateKind::Path,
                false,
            ),
            BrokerCandidate {
                usn: 12,
                scope: CandidateScope::RelativePath("B.jpg".to_owned()),
                previous_scope: None,
                file_reference: vec![2; 8],
                kind: CandidateKind::Path,
                is_directory: false,
            },
        ],
    };
    let accepted = encode_response(&accepted).expect("accepted response");
    let final_response = encode_response(&final_response).expect("oversized broker evidence");
    let client = broker_client(QueueTransport::new(vec![
        encode_frame(&accepted).expect("accepted frame"),
        encode_frame(&final_response).expect("final frame"),
    ]));
    assert!(matches!(
        client.read_range(request),
        Err(BrokerClientError::EvidenceBindingMismatch)
    ));
}

fn client_for(response: BrokerResponse) -> JournalBrokerClient<QueueTransport> {
    let mut responses = Vec::new();
    if !matches!(response, BrokerResponse::Journal { .. }) {
        let accepted = encode_response(&accepted_response()).expect("encode accepted response");
        responses.push(encode_frame(&accepted).expect("encode accepted frame"));
    }
    let payload = encode_response(&response).expect("encode response");
    responses.push(encode_frame(&payload).expect("encode frame"));
    broker_client(QueueTransport::new(responses))
}

fn accepted_response() -> BrokerResponse {
    BrokerResponse::ReadRangeAccepted {
        request_id: 1,
        binding: binding(),
        journal_id: 7,
        requested_start_usn: 10,
        requested_end_usn: 20,
        max_records: 32,
        max_evidence_bytes: 4_096,
    }
}

fn read_response(
    binding: ResponseBinding,
    journal_id: u64,
    start: i64,
    end: i64,
    covered: i64,
) -> BrokerResponse {
    BrokerResponse::ReadRange {
        request_id: 1,
        binding,
        journal_id,
        requested_start_usn: start,
        requested_end_usn: end,
        max_records: 32,
        max_evidence_bytes: 4_096,
        covered_until_usn: covered,
        is_complete: covered == end,
        candidates: vec![],
    }
}

fn response_with_candidate(candidate: BrokerCandidate) -> BrokerResponse {
    BrokerResponse::ReadRange {
        request_id: 1,
        binding: binding(),
        journal_id: 7,
        requested_start_usn: 10,
        requested_end_usn: 20,
        max_records: 32,
        max_evidence_bytes: 4_096,
        covered_until_usn: 20,
        is_complete: true,
        candidates: vec![candidate],
    }
}

fn candidate(
    scope: CandidateScope,
    previous_scope: Option<CandidateScope>,
    kind: CandidateKind,
    is_directory: bool,
) -> BrokerCandidate {
    BrokerCandidate {
        scope,
        previous_scope,
        file_reference: vec![1; 8],
        usn: 11,
        kind,
        is_directory,
    }
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
        max_records: 32,
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

fn root() -> RootAuthorization {
    RootAuthorization {
        root_id: "root-a".to_owned(),
        root_generation: 3,
        volume_id: "volume-a".to_owned(),
        root_identity: vec![1; 16],
        canonical_root_utf16: "C:\\Pictures".encode_utf16().collect(),
    }
}

fn binding() -> ResponseBinding {
    ResponseBinding {
        client_instance: [9; 16],
        root_id: "root-a".to_owned(),
        root_generation: 3,
        volume_id: "volume-a".to_owned(),
    }
}

struct QueueTransport {
    responses: Mutex<VecDeque<Vec<u8>>>,
    is_closed: AtomicBool,
}

impl QueueTransport {
    fn new(responses: Vec<Vec<u8>>) -> Self {
        Self {
            responses: Mutex::new(responses.into()),
            is_closed: AtomicBool::new(false),
        }
    }
}

#[derive(Debug)]
struct QueueTransportError;

impl fmt::Display for QueueTransportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("queue transport failed")
    }
}

impl std::error::Error for QueueTransportError {}

fn owned_connection<Transport>(transport: Transport) -> OwnedBrokerConnection<Transport> {
    static NEXT_CONNECTION_NONCE: AtomicUsize = AtomicUsize::new(1);
    let nonce = NEXT_CONNECTION_NONCE.fetch_add(1, Ordering::AcqRel) as u64;
    let mut connection_id = [0_u8; 16];
    connection_id[..8].copy_from_slice(&nonce.to_le_bytes());
    connection_id[8..].copy_from_slice(&(!nonce).to_le_bytes());
    OwnedBrokerConnection::from_adapter(transport, connection_id, 1, nonce)
        .expect("unique test connection")
}

fn broker_client<Transport>(transport: Transport) -> JournalBrokerClient<Transport>
where
    Transport: BrokerClientTransport,
{
    JournalBrokerClient::new(owned_connection(transport))
}

impl sealed::Sealed for QueueTransport {}

impl BrokerClientTransport for QueueTransport {
    type Error = QueueTransportError;

    fn try_send_frame(&mut self, _request_frame: &[u8]) -> Result<AdapterPoll<()>, Self::Error> {
        Ok(AdapterPoll::Ready(()))
    }

    fn poll_response(&mut self, _request_id: u64) -> Result<AdapterPoll<Vec<u8>>, Self::Error> {
        Ok(self
            .responses
            .lock()
            .map_err(|_| QueueTransportError)?
            .pop_front()
            .map_or(AdapterPoll::Pending, AdapterPoll::Ready))
    }

    fn try_discard(&mut self, _request_id: u64) -> Result<AdapterPoll<()>, Self::Error> {
        Ok(AdapterPoll::Ready(()))
    }

    fn begin_abandon(
        &mut self,
        _target_request_id: u64,
        _cancel_request: Option<(u64, Vec<u8>)>,
        _close_connection: bool,
    ) -> Result<AdapterPoll<()>, Self::Error> {
        Ok(AdapterPoll::Ready(()))
    }

    fn poll_abandon(&mut self, _target_request_id: u64) -> Result<AdapterPoll<()>, Self::Error> {
        Ok(AdapterPoll::Ready(()))
    }

    fn begin_close(&mut self) -> Result<AdapterPoll<()>, Self::Error> {
        self.is_closed.store(true, Ordering::Release);
        Ok(AdapterPoll::Ready(()))
    }

    fn poll_close(&mut self) -> Result<AdapterPoll<()>, Self::Error> {
        Ok(AdapterPoll::Ready(()))
    }
}

#[derive(Clone)]
struct NeverReceiveTransport {
    accepted: std::sync::Arc<Mutex<Option<Vec<u8>>>>,
    receive_calls: std::sync::Arc<AtomicUsize>,
    abandon_calls: std::sync::Arc<AtomicUsize>,
}

impl NeverReceiveTransport {
    fn new(accepted: Vec<u8>) -> Self {
        Self {
            accepted: std::sync::Arc::new(Mutex::new(Some(accepted))),
            receive_calls: std::sync::Arc::new(AtomicUsize::new(0)),
            abandon_calls: std::sync::Arc::new(AtomicUsize::new(0)),
        }
    }
}

impl sealed::Sealed for NeverReceiveTransport {}

impl BrokerClientTransport for NeverReceiveTransport {
    type Error = QueueTransportError;

    fn try_send_frame(&mut self, _request_frame: &[u8]) -> Result<AdapterPoll<()>, Self::Error> {
        Ok(AdapterPoll::Ready(()))
    }

    fn poll_response(&mut self, _request_id: u64) -> Result<AdapterPoll<Vec<u8>>, Self::Error> {
        self.receive_calls.fetch_add(1, Ordering::AcqRel);
        Ok(self
            .accepted
            .lock()
            .map_err(|_| QueueTransportError)?
            .take()
            .map_or(AdapterPoll::Pending, AdapterPoll::Ready))
    }

    fn try_discard(&mut self, _request_id: u64) -> Result<AdapterPoll<()>, Self::Error> {
        Ok(AdapterPoll::Ready(()))
    }

    fn begin_abandon(
        &mut self,
        _target_request_id: u64,
        _cancel_request: Option<(u64, Vec<u8>)>,
        _close_connection: bool,
    ) -> Result<AdapterPoll<()>, Self::Error> {
        self.abandon_calls.fetch_add(1, Ordering::AcqRel);
        Ok(AdapterPoll::Ready(()))
    }

    fn poll_abandon(&mut self, _target_request_id: u64) -> Result<AdapterPoll<()>, Self::Error> {
        Ok(AdapterPoll::Ready(()))
    }

    fn begin_close(&mut self) -> Result<AdapterPoll<()>, Self::Error> {
        Ok(AdapterPoll::Ready(()))
    }

    fn poll_close(&mut self) -> Result<AdapterPoll<()>, Self::Error> {
        Ok(AdapterPoll::Ready(()))
    }
}

#[test]
fn pending_drop_never_waits_for_transport_receive() {
    let accepted = encode_response(&accepted_response()).expect("accepted response");
    let transport = NeverReceiveTransport::new(encode_frame(&accepted).expect("accepted frame"));
    let client = broker_client(transport.clone());
    let pending = client
        .begin_read_range(read_request())
        .expect("pending read");
    let started = Instant::now();
    drop(pending);
    assert!(started.elapsed() < Duration::from_millis(250));
    client
        .poll_maintenance(8, Instant::now() + Duration::from_secs(1))
        .expect("queued abandon finishes within deadline");
    assert_eq!(transport.receive_calls.load(Ordering::Acquire), 1);
    assert_eq!(transport.abandon_calls.load(Ordering::Acquire), 1);
    assert_eq!(client.pending_count(), 0);
}

#[test]
fn owned_connection_identity_is_unique_and_clones_share_one_dispatcher() {
    let first = broker_client(QueueTransport::new(vec![]));
    let first_clone = first.clone();
    let second = broker_client(QueueTransport::new(vec![]));
    assert_eq!(
        first.connection_identity(),
        first_clone.connection_identity()
    );
    assert_ne!(first.connection_identity(), second.connection_identity());
    assert_eq!(first.internal_thread_count(), 0);
    assert_eq!(second.internal_thread_count(), 0);
}

#[test]
fn pending_read_close_advances_epoch_and_stops_all_future_adapter_polls() {
    let accepted = encode_response(&accepted_response()).expect("accepted response");
    let transport = NeverReceiveTransport::new(encode_frame(&accepted).expect("accepted frame"));
    let client = broker_client(transport.clone());
    let pending = client
        .begin_read_range(read_request())
        .expect("pending read");
    let waiter = std::thread::spawn(move || pending.wait());
    let deadline = Instant::now() + Duration::from_secs(2);
    while transport.receive_calls.load(Ordering::Acquire) < 2 {
        assert!(Instant::now() < deadline, "read did not enter pending poll");
        std::thread::yield_now();
    }
    client.close().expect("close owns epoch transition");
    assert!(matches!(
        waiter.join().expect("read waiter"),
        Err(BrokerClientError::Closed)
    ));
    let calls_after_close = transport.receive_calls.load(Ordering::Acquire);
    for _ in 0..1_000 {
        std::thread::yield_now();
    }
    assert_eq!(
        transport.receive_calls.load(Ordering::Acquire),
        calls_after_close
    );
}

#[test]
fn accepted_read_does_not_activate_when_close_wins_after_ack_validation() {
    let accepted = encode_response(&accepted_response()).expect("accepted response");
    let client = broker_client(QueueTransport::new(vec![
        encode_frame(&accepted).expect("accepted frame"),
    ]));
    let pause = client.pause_next_read_activation();
    let begin_client = client.clone();
    let begin = std::thread::spawn(move || begin_client.begin_read_range(read_request()));
    pause.wait_until_entered();
    client
        .close()
        .expect("close wins before read activation CAS");
    pause.release();
    assert!(matches!(
        begin.join().expect("begin read thread"),
        Err(BrokerClientError::Closed)
    ));
    assert_eq!(client.registry_entry_count(), 0);
    assert_eq!(client.business_pending_count(), 0);
    assert!(!client.transport_is_poisoned());
}

#[test]
fn query_ready_payload_is_superseded_when_close_wins_before_finish() {
    let response = BrokerResponse::Journal {
        request_id: 1,
        binding: binding(),
        capability: JournalCapability::Supported,
        journal_id: Some(7),
        first_usn: Some(0),
        next_usn: Some(20),
    };
    let payload = encode_response(&response).expect("query response");
    let client = broker_client(QueueTransport::new(vec![
        encode_frame(&payload).expect("query frame"),
    ]));
    let pause = client.pause_next_completion();
    let query_client = client.clone();
    let query = std::thread::spawn(move || query_client.query_journal(query_request()));
    pause.wait_until_entered();
    client.close().expect("close wins before completion CAS");
    pause.release();
    assert!(matches!(
        query.join().expect("query thread"),
        Err(BrokerClientError::Closed)
    ));
    assert_eq!(client.registry_entry_count(), 0);
}

#[test]
fn committed_query_survives_close_invalidating_best_effort_discard() {
    let response = BrokerResponse::Journal {
        request_id: 1,
        binding: binding(),
        capability: JournalCapability::Supported,
        journal_id: Some(7),
        first_usn: Some(0),
        next_usn: Some(20),
    };
    let payload = encode_response(&response).expect("query response");
    let client = broker_client(QueueTransport::new(vec![
        encode_frame(&payload).expect("query frame"),
    ]));
    let pause = client.pause_next_committed_discard();
    let query_client = client.clone();
    let query = std::thread::spawn(move || query_client.query_journal(query_request()));
    pause.wait_until_entered();
    client.close().expect("close follows committed finish");
    pause.release();
    assert!(matches!(
        query.join().expect("query thread"),
        Ok(BrokerResponse::Journal { .. })
    ));
    assert!(!client.transport_is_poisoned());
    assert_eq!(client.registry_entry_count(), 0);
}

#[test]
fn maximum_read_response_is_not_published_when_close_wins_during_validation() {
    let mut request = read_request();
    request.end_usn = i64::from(MAX_RECORDS) + 11;
    request.max_records = MAX_RECORDS;
    request.max_evidence_bytes = MAX_EVIDENCE_BYTES;
    let accepted = BrokerResponse::ReadRangeAccepted {
        request_id: 1,
        binding: binding(),
        journal_id: request.journal_id,
        requested_start_usn: request.start_usn,
        requested_end_usn: request.end_usn,
        max_records: request.max_records,
        max_evidence_bytes: request.max_evidence_bytes,
    };
    let candidates = (0..MAX_RECORDS)
        .map(|index| BrokerCandidate {
            scope: CandidateScope::RelativePath(format!("{index}.jpg")),
            previous_scope: None,
            file_reference: vec![1; 8],
            usn: 11 + i64::from(index),
            kind: CandidateKind::Path,
            is_directory: false,
        })
        .collect();
    let response = BrokerResponse::ReadRange {
        request_id: 1,
        binding: binding(),
        journal_id: request.journal_id,
        requested_start_usn: request.start_usn,
        requested_end_usn: request.end_usn,
        max_records: request.max_records,
        max_evidence_bytes: request.max_evidence_bytes,
        covered_until_usn: request.end_usn,
        is_complete: true,
        candidates,
    };
    let accepted = encode_response(&accepted).expect("maximum accepted response");
    let response = encode_response(&response).expect("maximum final response");
    let client = broker_client(QueueTransport::new(vec![
        encode_frame(&accepted).expect("accepted frame"),
        encode_frame(&response).expect("maximum final frame"),
    ]));
    let pending = client
        .begin_read_range(request)
        .expect("maximum response pending read");
    let pause = client.pause_next_candidate_validation();
    let waiter = std::thread::spawn(move || pending.wait());
    pause.wait_until_entered();
    client.close().expect("close wins during validation");
    pause.release();
    assert!(matches!(
        waiter.join().expect("read waiter"),
        Err(BrokerClientError::Closed)
    ));
    assert_eq!(client.registry_entry_count(), 0);
}

#[derive(Clone)]
struct TimeoutLifecycleTransport {
    inner: std::sync::Arc<TimeoutLifecycleInner>,
}

struct TimeoutLifecycleInner {
    expected: Mutex<HashSet<u64>>,
    responses: Mutex<HashMap<u64, Vec<u8>>>,
    sent_ids: Mutex<Vec<u64>>,
    query_ids: Mutex<Vec<u64>>,
    max_expected: AtomicUsize,
    late_dropped: AtomicUsize,
    poll_calls: AtomicUsize,
}

impl TimeoutLifecycleTransport {
    fn new() -> Self {
        Self {
            inner: std::sync::Arc::new(TimeoutLifecycleInner {
                expected: Mutex::new(HashSet::new()),
                responses: Mutex::new(HashMap::new()),
                sent_ids: Mutex::new(Vec::new()),
                query_ids: Mutex::new(Vec::new()),
                max_expected: AtomicUsize::new(0),
                late_dropped: AtomicUsize::new(0),
                poll_calls: AtomicUsize::new(0),
            }),
        }
    }

    fn push_if_expected(&self, request_id: u64, response: BrokerResponse) {
        let is_expected = self
            .inner
            .expected
            .lock()
            .is_ok_and(|expected| expected.contains(&request_id));
        if !is_expected {
            self.inner.late_dropped.fetch_add(1, Ordering::AcqRel);
            return;
        }
        let payload = encode_response(&response).expect("encode lifecycle response");
        let frame = encode_frame(&payload).expect("encode lifecycle frame");
        self.inner
            .responses
            .lock()
            .expect("response map")
            .insert(request_id, frame);
    }

    fn inject_late_query_responses(&self) {
        let query_ids = self.inner.query_ids.lock().expect("query ids").clone();
        for request_id in query_ids {
            self.push_if_expected(
                request_id,
                BrokerResponse::Journal {
                    request_id,
                    binding: binding(),
                    capability: JournalCapability::Supported,
                    journal_id: Some(7),
                    first_usn: Some(0),
                    next_usn: Some(20),
                },
            );
        }
    }

    fn expected_count(&self) -> usize {
        self.inner
            .expected
            .lock()
            .map_or(0, |expected| expected.len())
    }

    fn response_count(&self) -> usize {
        self.inner
            .responses
            .lock()
            .map_or(0, |responses| responses.len())
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

impl sealed::Sealed for TimeoutLifecycleTransport {}

impl BrokerClientTransport for TimeoutLifecycleTransport {
    type Error = QueueTransportError;

    fn try_send_frame(&mut self, request_frame: &[u8]) -> Result<AdapterPoll<()>, Self::Error> {
        let payload = decode_frame(request_frame).map_err(|_| QueueTransportError)?;
        let request = decode_request(payload).map_err(|_| QueueTransportError)?;
        let request_id = request.request_id();
        let expected_len = {
            let mut expected = self
                .inner
                .expected
                .lock()
                .map_err(|_| QueueTransportError)?;
            expected.insert(request_id);
            expected.len()
        };
        self.inner
            .max_expected
            .fetch_max(expected_len, Ordering::AcqRel);
        self.inner
            .sent_ids
            .lock()
            .map_err(|_| QueueTransportError)?
            .push(request_id);
        match request {
            super::wire::BrokerRequest::QueryJournal { .. } => {
                self.inner
                    .query_ids
                    .lock()
                    .map_err(|_| QueueTransportError)?
                    .push(request_id);
            }
            super::wire::BrokerRequest::Cancel {
                caller,
                target_request_id,
                ..
            } => {
                self.push_if_expected(
                    request_id,
                    BrokerResponse::Cancelled {
                        request_id,
                        client_instance: caller.client_instance,
                        target_request_id,
                    },
                );
                self.push_if_expected(
                    target_request_id,
                    BrokerResponse::Failure {
                        request_id: target_request_id,
                        client_instance: Some(caller.client_instance),
                        failure: BrokerFailure {
                            code: BrokerFailureCode::Cancelled,
                        },
                    },
                );
            }
            super::wire::BrokerRequest::ReadRange { .. }
            | super::wire::BrokerRequest::ReadVolume { .. }
            | super::wire::BrokerRequest::RegisterRoot { .. } => {
                return Err(QueueTransportError);
            }
        }
        Ok(AdapterPoll::Ready(()))
    }

    fn poll_response(&mut self, request_id: u64) -> Result<AdapterPoll<Vec<u8>>, Self::Error> {
        self.inner.poll_calls.fetch_add(1, Ordering::AcqRel);
        Ok(self
            .inner
            .responses
            .lock()
            .map_err(|_| QueueTransportError)?
            .remove(&request_id)
            .map_or(AdapterPoll::Pending, AdapterPoll::Ready))
    }

    fn try_discard(&mut self, request_id: u64) -> Result<AdapterPoll<()>, Self::Error> {
        self.discard_request(request_id);
        Ok(AdapterPoll::Ready(()))
    }

    fn begin_abandon(
        &mut self,
        target_request_id: u64,
        _cancel_request: Option<(u64, Vec<u8>)>,
        _close_connection: bool,
    ) -> Result<AdapterPoll<()>, Self::Error> {
        self.discard_request(target_request_id);
        Ok(AdapterPoll::Ready(()))
    }

    fn poll_abandon(&mut self, _target_request_id: u64) -> Result<AdapterPoll<()>, Self::Error> {
        Ok(AdapterPoll::Ready(()))
    }

    fn begin_close(&mut self) -> Result<AdapterPoll<()>, Self::Error> {
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

#[test]
fn query_and_cancel_timeout_lifecycle_stays_bounded_for_late_responses() {
    let transport = TimeoutLifecycleTransport::new();
    let client = broker_client(transport.clone());
    let mut request = query_request();
    request.timeout_ms = 10;
    assert!(matches!(
        client.query_journal(request),
        Err(BrokerClientError::TransportPoisoned)
    ));
    for _ in 0..100 {
        assert!(matches!(
            client.query_journal(query_request()),
            Err(BrokerClientError::Closed)
        ));
    }
    assert_eq!(client.pending_count(), 0);
    assert_eq!(transport.expected_count(), 0);
    assert_eq!(transport.response_count(), 0);
    transport.inject_late_query_responses();
    assert_eq!(transport.expected_count(), 0);
    assert_eq!(transport.response_count(), 0);
    assert_eq!(transport.inner.late_dropped.load(Ordering::Acquire), 1);
    assert!(transport.inner.max_expected.load(Ordering::Acquire) <= 1);
    let sent = transport.inner.sent_ids.lock().expect("sent ids");
    assert_eq!(sent.len(), 1);
}

#[test]
fn pending_query_close_rejects_late_ready_and_never_reenters_transport() {
    let transport = TimeoutLifecycleTransport::new();
    let client = broker_client(transport.clone());
    let query_client = client.clone();
    let query = std::thread::spawn(move || query_client.query_journal(query_request()));
    let deadline = Instant::now() + Duration::from_secs(2);
    while transport.inner.poll_calls.load(Ordering::Acquire) == 0 {
        assert!(
            Instant::now() < deadline,
            "query did not enter pending poll"
        );
        std::thread::yield_now();
    }
    client.close().expect("close advances query epoch");
    transport.inject_late_query_responses();
    assert!(matches!(
        query.join().expect("query waiter"),
        Err(BrokerClientError::Closed)
    ));
    let calls_after_close = transport.inner.poll_calls.load(Ordering::Acquire);
    for _ in 0..1_000 {
        std::thread::yield_now();
    }
    assert_eq!(
        transport.inner.poll_calls.load(Ordering::Acquire),
        calls_after_close
    );
    assert_eq!(client.registry_entry_count(), 0);
}

#[derive(Clone, Copy)]
enum AbandonBehavior {
    Complete,
    Error,
    Timeout,
    Panic,
    Block,
    NeverReturn,
}

#[derive(Clone)]
struct CleanupAdversaryTransport {
    inner: Arc<CleanupAdversaryInner>,
}

struct CleanupAdversaryInner {
    responses: Mutex<HashMap<u64, VecDeque<Vec<u8>>>>,
    behaviors: Mutex<VecDeque<AbandonBehavior>>,
    active_abandon: Mutex<Option<AbandonBehavior>>,
    gate: Mutex<bool>,
    cancel_request_ids: Mutex<HashSet<u64>>,
    block_cancel_receives: AtomicBool,
    cancel_gate: Mutex<bool>,
    abandon_calls: AtomicUsize,
    abandon_poll_calls: AtomicUsize,
    discard_calls: AtomicUsize,
    panic_try_discard: AtomicBool,
    close_attempts: AtomicUsize,
    close_failures_remaining: AtomicUsize,
    close_panics_remaining: AtomicUsize,
    block_close: AtomicBool,
    close_started: AtomicUsize,
    close_gate: Mutex<bool>,
}

impl CleanupAdversaryTransport {
    fn new(behaviors: Vec<AbandonBehavior>, close_failures: usize) -> Self {
        Self {
            inner: Arc::new(CleanupAdversaryInner {
                responses: Mutex::new(HashMap::new()),
                behaviors: Mutex::new(behaviors.into()),
                active_abandon: Mutex::new(None),
                gate: Mutex::new(false),
                cancel_request_ids: Mutex::new(HashSet::new()),
                block_cancel_receives: AtomicBool::new(false),
                cancel_gate: Mutex::new(false),
                abandon_calls: AtomicUsize::new(0),
                abandon_poll_calls: AtomicUsize::new(0),
                discard_calls: AtomicUsize::new(0),
                panic_try_discard: AtomicBool::new(false),
                close_attempts: AtomicUsize::new(0),
                close_failures_remaining: AtomicUsize::new(close_failures),
                close_panics_remaining: AtomicUsize::new(0),
                block_close: AtomicBool::new(false),
                close_started: AtomicUsize::new(0),
                close_gate: Mutex::new(false),
            }),
        }
    }

    fn block_cancel_receives(&self) {
        self.inner
            .block_cancel_receives
            .store(true, Ordering::Release);
    }

    fn release_cancel_receives(&self) {
        if let Ok(mut released) = self.inner.cancel_gate.lock() {
            *released = true;
        }
    }

    fn block_close(&self) {
        self.inner.block_close.store(true, Ordering::Release);
    }

    fn panic_next_close(&self) {
        self.inner
            .close_panics_remaining
            .store(1, Ordering::Release);
    }

    fn wait_for_close(&self) {
        let deadline = Instant::now() + Duration::from_secs(2);
        while self.inner.close_started.load(Ordering::Acquire) == 0 {
            assert!(Instant::now() < deadline, "close did not start");
            std::thread::yield_now();
        }
    }

    fn release_close(&self) {
        if let Ok(mut released) = self.inner.close_gate.lock() {
            *released = true;
        }
    }

    fn push_response(
        &self,
        request_id: u64,
        response: BrokerResponse,
    ) -> Result<(), QueueTransportError> {
        let payload = encode_response(&response).map_err(|_| QueueTransportError)?;
        let frame = encode_frame(&payload).map_err(|_| QueueTransportError)?;
        self.inner
            .responses
            .lock()
            .map_err(|_| QueueTransportError)?
            .entry(request_id)
            .or_default()
            .push_back(frame);
        Ok(())
    }
}

impl sealed::Sealed for CleanupAdversaryTransport {}

impl BrokerClientTransport for CleanupAdversaryTransport {
    type Error = QueueTransportError;

    fn try_send_frame(&mut self, request_frame: &[u8]) -> Result<AdapterPoll<()>, Self::Error> {
        let payload = decode_frame(request_frame).map_err(|_| QueueTransportError)?;
        let request = decode_request(payload).map_err(|_| QueueTransportError)?;
        match request {
            super::wire::BrokerRequest::ReadRange {
                request_id,
                request,
            } => {
                let response = BrokerResponse::ReadRangeAccepted {
                    request_id,
                    binding: ResponseBinding::from_request(&request.caller, &request.root),
                    journal_id: request.journal_id,
                    requested_start_usn: request.start_usn,
                    requested_end_usn: request.end_usn,
                    max_records: request.max_records,
                    max_evidence_bytes: request.max_evidence_bytes,
                };
                self.push_response(request_id, response)?;
            }
            super::wire::BrokerRequest::Cancel {
                request_id,
                caller,
                target_request_id,
            } => {
                self.inner
                    .cancel_request_ids
                    .lock()
                    .map_err(|_| QueueTransportError)?
                    .insert(request_id);
                self.push_response(
                    request_id,
                    BrokerResponse::Cancelled {
                        request_id,
                        client_instance: caller.client_instance,
                        target_request_id,
                    },
                )?;
                self.push_response(
                    target_request_id,
                    BrokerResponse::Failure {
                        request_id: target_request_id,
                        client_instance: Some(caller.client_instance),
                        failure: BrokerFailure {
                            code: BrokerFailureCode::Cancelled,
                        },
                    },
                )?;
            }
            super::wire::BrokerRequest::QueryJournal { .. }
            | super::wire::BrokerRequest::ReadVolume { .. }
            | super::wire::BrokerRequest::RegisterRoot { .. } => {
                return Err(QueueTransportError);
            }
        }
        Ok(AdapterPoll::Ready(()))
    }

    fn poll_response(&mut self, request_id: u64) -> Result<AdapterPoll<Vec<u8>>, Self::Error> {
        let is_cancel = self
            .inner
            .cancel_request_ids
            .lock()
            .map_err(|_| QueueTransportError)?
            .contains(&request_id);
        if is_cancel && self.inner.block_cancel_receives.load(Ordering::Acquire) {
            let released = self
                .inner
                .cancel_gate
                .lock()
                .map_err(|_| QueueTransportError)?;
            if !*released {
                return Ok(AdapterPoll::Pending);
            }
        }
        let mut responses = self
            .inner
            .responses
            .lock()
            .map_err(|_| QueueTransportError)?;
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
        self.inner.discard_calls.fetch_add(1, Ordering::AcqRel);
        assert!(
            !self.inner.panic_try_discard.load(Ordering::Acquire),
            "adversarial try_discard panic"
        );
        if let Ok(mut cancel_ids) = self.inner.cancel_request_ids.lock() {
            cancel_ids.remove(&request_id);
        }
        if let Ok(mut responses) = self.inner.responses.lock() {
            responses.remove(&request_id);
        }
        Ok(AdapterPoll::Ready(()))
    }

    fn begin_abandon(
        &mut self,
        _target_request_id: u64,
        _cancel_request: Option<(u64, Vec<u8>)>,
        _close_connection: bool,
    ) -> Result<AdapterPoll<()>, Self::Error> {
        self.inner.abandon_calls.fetch_add(1, Ordering::AcqRel);
        let behavior = self
            .inner
            .behaviors
            .lock()
            .map_err(|_| QueueTransportError)?
            .pop_front()
            .unwrap_or(AbandonBehavior::Complete);
        match behavior {
            AbandonBehavior::Complete => Ok(AdapterPoll::Ready(())),
            AbandonBehavior::Error => Err(QueueTransportError),
            AbandonBehavior::Timeout | AbandonBehavior::NeverReturn => {
                *self
                    .inner
                    .active_abandon
                    .lock()
                    .map_err(|_| QueueTransportError)? = Some(behavior);
                Ok(AdapterPoll::Pending)
            }
            AbandonBehavior::Panic => panic!("adversarial transport abandon panic"),
            AbandonBehavior::Block => {
                *self
                    .inner
                    .active_abandon
                    .lock()
                    .map_err(|_| QueueTransportError)? = Some(behavior);
                Ok(AdapterPoll::Pending)
            }
        }
    }

    fn poll_abandon(&mut self, _target_request_id: u64) -> Result<AdapterPoll<()>, Self::Error> {
        self.inner.abandon_poll_calls.fetch_add(1, Ordering::AcqRel);
        let behavior = *self
            .inner
            .active_abandon
            .lock()
            .map_err(|_| QueueTransportError)?;
        match behavior {
            Some(AbandonBehavior::Block)
                if self.inner.gate.lock().is_ok_and(|released| *released) =>
            {
                *self
                    .inner
                    .active_abandon
                    .lock()
                    .map_err(|_| QueueTransportError)? = None;
                Ok(AdapterPoll::Ready(()))
            }
            Some(_) => Ok(AdapterPoll::Pending),
            None => Ok(AdapterPoll::Ready(())),
        }
    }

    fn begin_close(&mut self) -> Result<AdapterPoll<()>, Self::Error> {
        self.inner.close_attempts.fetch_add(1, Ordering::AcqRel);
        if self
            .inner
            .close_panics_remaining
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |remaining| {
                (remaining > 0).then_some(remaining.saturating_sub(1))
            })
            .is_ok()
        {
            panic!("adversarial transport close panic");
        }
        if self.inner.block_close.load(Ordering::Acquire) {
            self.inner.close_started.fetch_add(1, Ordering::AcqRel);
            if !self
                .inner
                .close_gate
                .lock()
                .map_err(|_| QueueTransportError)?
                .to_owned()
            {
                return Ok(AdapterPoll::Pending);
            }
        }
        if self
            .inner
            .close_failures_remaining
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |remaining| {
                if remaining > 0 {
                    Some(remaining - 1)
                } else {
                    None
                }
            })
            .is_ok()
        {
            Err(QueueTransportError)
        } else {
            Ok(AdapterPoll::Ready(()))
        }
    }

    fn poll_close(&mut self) -> Result<AdapterPoll<()>, Self::Error> {
        if self.inner.close_gate.lock().is_ok_and(|released| *released) {
            Ok(AdapterPoll::Ready(()))
        } else {
            Ok(AdapterPoll::Pending)
        }
    }
}

#[test]
fn persistent_journal_object_contract_owns_register_query_read_cancel_and_close() {
    let registered = BrokerResponse::RootRegistered {
        request_id: 1,
        binding: binding(),
        root_capability: RootCapability([5; 32]),
    };
    let registered = encode_response(&registered).expect("registered response");
    let register_connection =
        PersistentChangeJournalConnection::Connected(std::sync::Arc::new(broker_client(
            QueueTransport::new(vec![encode_frame(&registered).expect("registered frame")]),
        )));
    let register_session = match register_connection {
        PersistentChangeJournalConnection::Connected(session) => session,
        PersistentChangeJournalConnection::LiveOnly(reason) => {
            panic!("object contract unexpectedly LiveOnly: {reason:?}")
        }
    };
    assert_eq!(
        register_session
            .register_root(RegisterRootRequest {
                caller: caller(),
                root: root(),
                client_root_handle: 1,
                timeout_ms: 100,
            })
            .expect("register through object contract"),
        RootCapability([5; 32])
    );
    register_session.close().expect("close register session");

    let query_session: Box<dyn PersistentChangeJournalSession> =
        Box::new(client_for(BrokerResponse::Journal {
            request_id: 1,
            binding: binding(),
            capability: JournalCapability::Supported,
            journal_id: Some(7),
            first_usn: Some(0),
            next_usn: Some(20),
        }));
    assert!(matches!(
        query_session.query_journal(query_request()),
        Ok(BrokerResponse::Journal {
            journal_id: Some(7),
            ..
        })
    ));
    query_session.close().expect("close query session");

    let completed_read_session: Box<dyn PersistentChangeJournalSession> =
        Box::new(client_for(read_response(binding(), 7, 10, 20, 20)));
    assert!(matches!(
        completed_read_session.read_range(read_request()),
        Ok(BrokerResponse::ReadRange {
            covered_until_usn: 20,
            ..
        })
    ));
    completed_read_session
        .close()
        .expect("close completed read session");

    let read_session: Box<dyn PersistentChangeJournalSession> =
        Box::new(broker_client(CleanupAdversaryTransport::new(vec![], 0)));
    let pending = read_session
        .begin_read_range(read_request())
        .expect("begin read through object contract");
    pending
        .cancel(caller())
        .expect("cancel through pending object contract");
    assert!(matches!(
        pending.wait(),
        Err(PersistentChangeJournalOperationError::Cancelled)
    ));
    read_session.close().expect("close read session");
}

fn wait_for_cleanup_state(
    client: &JournalBrokerClient<CleanupAdversaryTransport>,
    transport: &CleanupAdversaryTransport,
    expected_abandons: usize,
) {
    let deadline = Instant::now() + Duration::from_secs(2);
    while client.pending_count() != 0
        || transport.inner.abandon_calls.load(Ordering::Acquire) < expected_abandons
    {
        assert!(Instant::now() < deadline, "cleanup did not settle");
        let _ = client.poll_maintenance(8, Instant::now() + Duration::from_millis(50));
        std::thread::yield_now();
    }
}

#[test]
fn cancel_ack_is_superseded_when_close_wins_before_finish() {
    let transport = CleanupAdversaryTransport::new(vec![], 0);
    let client = broker_client(transport);
    let pending = client
        .begin_read_range(read_request())
        .expect("pending read");
    let handle = pending.cancel_handle();
    let pause = client.pause_next_completion();
    let cancel_client = client.clone();
    let cancel = std::thread::spawn(move || cancel_client.cancel(&handle, caller()));
    pause.wait_until_entered();
    client
        .close()
        .expect("close wins before cancel completion CAS");
    pause.release();
    assert!(matches!(
        cancel.join().expect("cancel thread"),
        Err(BrokerClientError::Closed)
    ));
    drop(pending);
    assert_eq!(client.registry_entry_count(), 0);
    assert_eq!(client.control_pending_count(), 0);
    assert_eq!(client.business_pending_count(), 0);
}

#[test]
fn automatic_maintenance_reclaims_eight_drops_before_ninth_business_request() {
    let transport = CleanupAdversaryTransport::new(vec![AbandonBehavior::Complete; 9], 0);
    let client = broker_client(transport.clone());
    let mut pending = Vec::new();
    for _ in 0..8 {
        pending.push(
            client
                .begin_read_range(read_request())
                .expect("business capacity available"),
        );
    }
    for request in pending {
        drop(request);
    }
    assert_eq!(client.business_pending_count(), 8);
    let ninth = client
        .begin_read_range(read_request())
        .expect("entrypoint maintenance reclaims dropped requests");
    assert_eq!(client.business_pending_count(), 1);
    assert_eq!(client.registry_entry_count(), 1);
    drop(ninth);
    client
        .poll_maintenance(8, Instant::now() + Duration::from_secs(1))
        .expect("idle owner pump drains final cleanup");
    assert_eq!(client.pending_count(), 0);
    assert_eq!(client.registry_entry_count(), 0);
    assert_eq!(transport.inner.abandon_calls.load(Ordering::Acquire), 9);
}

#[test]
fn maintenance_sweeps_terminal_registry_entry_after_drop_lock_contention() {
    let transport = CleanupAdversaryTransport::new(vec![], 0);
    let client =
        JournalBrokerClient::new_with_disconnected_cleanup(owned_connection(transport.clone()));
    let pending = client
        .begin_read_range(read_request())
        .expect("pending read");
    let entered = Arc::new(std::sync::Barrier::new(2));
    let release = Arc::new(std::sync::Barrier::new(2));
    let lock_client = client.clone();
    let lock_entered = Arc::clone(&entered);
    let lock_release = Arc::clone(&release);
    let holder = std::thread::spawn(move || {
        lock_client.hold_registry_lock_for_test(&lock_entered, &lock_release);
    });
    entered.wait();
    let started = Instant::now();
    drop(pending);
    assert!(started.elapsed() < Duration::from_millis(50));
    assert_eq!(client.business_pending_count(), 0);
    release.wait();
    holder.join().expect("registry lock holder");
    client
        .poll_maintenance(0, Instant::now() + Duration::from_secs(1))
        .expect("maintenance sweeps deferred terminal entry");
    assert_eq!(client.registry_entry_count(), 0);
    assert_eq!(client.pending_count(), 0);
    assert!(client.transport_is_poisoned());
    assert_eq!(transport.inner.discard_calls.load(Ordering::Acquire), 0);
}

#[test]
fn client_control_storm_is_bounded_without_consuming_business_capacity() {
    let transport = CleanupAdversaryTransport::new(vec![AbandonBehavior::Complete], 0);
    transport.block_cancel_receives();
    let client = broker_client(transport.clone());
    let first = client
        .begin_read_range(read_request())
        .expect("first pending read");
    let second = client
        .begin_read_range(read_request())
        .expect("second pending read");
    let third = client
        .begin_read_range(read_request())
        .expect("third pending read");
    let first_handle = first.cancel_handle();
    let second_handle = second.cancel_handle();
    let first_client = client.clone();
    let first_cancel = std::thread::spawn(move || first_client.cancel(&first_handle, caller()));
    let second_client = client.clone();
    let second_cancel = std::thread::spawn(move || second_client.cancel(&second_handle, caller()));
    let control_deadline = Instant::now() + Duration::from_secs(2);
    while client.control_pending_count() != 2 {
        assert!(
            Instant::now() < control_deadline,
            "two control requests were not admitted"
        );
        std::thread::yield_now();
    }
    assert_eq!(client.business_pending_count(), 3);
    assert_eq!(client.control_pending_count(), 2);
    assert!(matches!(
        client.cancel(&third.cancel_handle(), caller()),
        Err(BrokerClientError::PendingLimitExceeded)
    ));
    assert_eq!(client.business_pending_count(), 3);
    transport.release_cancel_receives();
    first_cancel
        .join()
        .expect("first cancel thread")
        .expect("first cancel");
    second_cancel
        .join()
        .expect("second cancel thread")
        .expect("second cancel");
    assert!(matches!(first.wait(), Err(BrokerClientError::Cancelled)));
    assert!(matches!(second.wait(), Err(BrokerClientError::Cancelled)));
    drop(third);
    wait_for_cleanup_state(&client, &transport, 1);
    assert_eq!(client.business_pending_count(), 0);
    assert_eq!(client.control_pending_count(), 0);
}

#[test]
fn cleanup_poll_failures_poison_without_spawning_or_reusing_transport() {
    for behavior in [
        AbandonBehavior::Panic,
        AbandonBehavior::Error,
        AbandonBehavior::Timeout,
    ] {
        let transport =
            CleanupAdversaryTransport::new(vec![behavior, AbandonBehavior::Complete], 0);
        let client = broker_client(transport.clone());
        let first = client
            .begin_read_range(read_request())
            .expect("first pending read");
        let second = client
            .begin_read_range(read_request())
            .expect("second pending read");
        drop(first);
        drop(second);
        assert!(
            client
                .poll_maintenance(8, Instant::now() + Duration::from_secs(1))
                .is_err()
        );
        assert_eq!(client.internal_thread_count(), 0);
        assert!(client.transport_is_poisoned());
        assert_eq!(client.business_pending_count(), 0);
        assert_eq!(client.control_pending_count(), 0);
        assert_eq!(client.registry_entry_count(), 0);
        assert_eq!(transport.inner.abandon_calls.load(Ordering::Acquire), 1);
    }
}

#[test]
fn explicit_abandon_completes_on_the_same_owned_connection() {
    let transport = CleanupAdversaryTransport::new(vec![AbandonBehavior::Complete], 0);
    let client = broker_client(transport.clone());
    let pending = client
        .begin_read_range(read_request())
        .expect("pending read");
    pending.abandon().expect("bounded explicit abandon");
    assert_eq!(client.registry_entry_count(), 0);
    assert_eq!(client.business_pending_count(), 0);
    assert_eq!(client.control_pending_count(), 0);
    assert_eq!(transport.inner.abandon_calls.load(Ordering::Acquire), 0);
}

#[test]
fn try_discard_panic_fails_closed_without_calling_abandon() {
    let transport = CleanupAdversaryTransport::new(vec![AbandonBehavior::Complete], 0);
    transport
        .inner
        .panic_try_discard
        .store(true, Ordering::Release);
    let client = broker_client(transport.clone());
    let pending = client
        .begin_read_range(read_request())
        .expect("pending read");
    drop(pending);
    assert!(
        client
            .poll_maintenance(8, Instant::now() + Duration::from_secs(1))
            .is_err()
    );
    assert!(client.transport_is_poisoned());
    assert_eq!(client.registry_entry_count(), 0);
    assert_eq!(transport.inner.abandon_calls.load(Ordering::Acquire), 0);
    assert_eq!(client.control_pending_count(), 0);
}

#[test]
fn cleanup_queue_full_fallback_retires_locally_and_queues_teardown() {
    let transport =
        CleanupAdversaryTransport::new(vec![AbandonBehavior::Block, AbandonBehavior::Complete], 0);
    let client =
        JournalBrokerClient::new_with_cleanup_capacity(owned_connection(transport.clone()), 1);
    let first = client
        .begin_read_range(read_request())
        .expect("first pending read");
    let second = client
        .begin_read_range(read_request())
        .expect("second pending read");
    drop(first);
    let started = Instant::now();
    drop(second);
    assert!(started.elapsed() < Duration::from_millis(50));
    assert!(client.transport_is_poisoned());
    assert_eq!(transport.inner.abandon_calls.load(Ordering::Acquire), 0);
    assert_eq!(client.business_pending_count(), 0);
    assert_eq!(client.control_pending_count(), 0);
    assert_eq!(client.registry_entry_count(), 0);
}

#[test]
fn disconnected_cleanup_fallback_removes_the_registry_entry() {
    let transport = CleanupAdversaryTransport::new(vec![], 0);
    let client =
        JournalBrokerClient::new_with_disconnected_cleanup(owned_connection(transport.clone()));
    let pending = client
        .begin_read_range(read_request())
        .expect("pending read");
    drop(pending);
    assert_eq!(client.pending_count(), 0);
    assert_eq!(client.registry_entry_count(), 0);
    assert!(client.transport_is_poisoned());
    assert_eq!(transport.inner.discard_calls.load(Ordering::Acquire), 0);
}

#[test]
fn never_returning_abandon_poison_is_bounded_and_stops_future_transport_calls() {
    let transport = CleanupAdversaryTransport::new(vec![AbandonBehavior::NeverReturn], 0);
    let client = broker_client(transport.clone());
    let first = client
        .begin_read_range(read_request())
        .expect("first pending read");
    let second = client
        .begin_read_range(read_request())
        .expect("second pending read");
    let drop_started = Instant::now();
    drop(first);
    assert!(drop_started.elapsed() < Duration::from_millis(50));
    let cleanup_started = Instant::now();
    assert!(
        client
            .poll_maintenance(8, Instant::now() + Duration::from_secs(1))
            .is_err()
    );
    assert!(cleanup_started.elapsed() < Duration::from_secs(1));
    assert!(client.transport_is_poisoned());
    assert_eq!(client.internal_thread_count(), 0);
    assert_eq!(transport.inner.abandon_calls.load(Ordering::Acquire), 1);
    let abandon_poll_calls = transport.inner.abandon_poll_calls.load(Ordering::Acquire);
    assert!(abandon_poll_calls > 1);
    assert!(
        abandon_poll_calls <= 256,
        "deadline-aware backoff polled {abandon_poll_calls} times"
    );
    let discard_calls = transport.inner.discard_calls.load(Ordering::Acquire);
    assert!(matches!(
        client.begin_read_range(read_request()),
        Err(BrokerClientError::Closed)
    ));

    drop(second);
    assert_eq!(client.registry_entry_count(), 0);
    assert_eq!(transport.inner.abandon_calls.load(Ordering::Acquire), 1);
    assert_eq!(
        transport.inner.discard_calls.load(Ordering::Acquire),
        discard_calls
    );
    let close_started = Instant::now();
    assert!(matches!(
        client.close(),
        Err(BrokerClientError::TransportPoisoned)
    ));
    assert!(close_started.elapsed() < Duration::from_millis(50));
    assert_eq!(transport.inner.close_attempts.load(Ordering::Acquire), 0);
}

#[test]
fn close_retries_until_transport_confirms_shutdown() {
    let transport = CleanupAdversaryTransport::new(vec![], 1);
    let client = broker_client(transport.clone());
    assert!(matches!(
        client.close(),
        Err(BrokerClientError::Transport(_))
    ));
    assert!(!client.transport_close_confirmed());
    client.close().expect("second close retries and succeeds");
    assert!(client.transport_close_confirmed());
    client.close().expect("confirmed close is idempotent");
    assert_eq!(transport.inner.close_attempts.load(Ordering::Acquire), 2);
}

#[test]
fn concurrent_close_has_one_transport_owner_and_retry_after_failure() {
    let transport = CleanupAdversaryTransport::new(vec![], 0);
    transport.block_close();
    let client = broker_client(transport.clone());
    let owner = client.clone();
    let close_thread = std::thread::spawn(move || owner.close());
    transport.wait_for_close();
    assert!(matches!(
        client.close(),
        Err(BrokerClientError::CloseInProgress)
    ));
    assert_eq!(transport.inner.close_attempts.load(Ordering::Acquire), 1);
    transport.release_close();
    close_thread
        .join()
        .expect("close owner thread")
        .expect("close owner succeeds");
    assert!(client.transport_close_confirmed());
    client.close().expect("confirmed close stays idempotent");
    assert_eq!(transport.inner.close_attempts.load(Ordering::Acquire), 1);

    let retry_transport = CleanupAdversaryTransport::new(vec![], 1);
    let retry_client = broker_client(retry_transport.clone());
    assert!(matches!(
        retry_client.close(),
        Err(BrokerClientError::Transport(_))
    ));
    retry_client.close().expect("failed owner can retry");
    assert_eq!(
        retry_transport.inner.close_attempts.load(Ordering::Acquire),
        2
    );
}

#[test]
fn close_panic_returns_to_unconfirmed_and_retries_once() {
    let transport = CleanupAdversaryTransport::new(vec![], 0);
    transport.panic_next_close();
    let client = broker_client(transport.clone());
    assert!(matches!(
        client.close(),
        Err(BrokerClientError::Transport(_))
    ));
    assert!(!client.transport_is_poisoned());
    assert!(!client.transport_close_confirmed());
    client
        .close()
        .expect("close retries after helper catches panic");
    assert!(client.transport_close_confirmed());
    assert_eq!(transport.inner.close_attempts.load(Ordering::Acquire), 2);
}

#[test]
fn never_returning_close_poison_is_bounded_and_never_retries_transport() {
    let transport = CleanupAdversaryTransport::new(vec![], 0);
    transport.block_close();
    let client = broker_client(transport.clone());
    let started = Instant::now();
    assert!(matches!(
        client.close(),
        Err(BrokerClientError::TransportPoisoned)
    ));
    assert!(started.elapsed() < Duration::from_secs(1));
    assert!(client.transport_is_poisoned());
    assert_eq!(transport.inner.close_attempts.load(Ordering::Acquire), 1);
    let retry_started = Instant::now();
    assert!(matches!(
        client.close(),
        Err(BrokerClientError::TransportPoisoned)
    ));
    assert!(retry_started.elapsed() < Duration::from_millis(50));
    assert_eq!(transport.inner.close_attempts.load(Ordering::Acquire), 1);
}
