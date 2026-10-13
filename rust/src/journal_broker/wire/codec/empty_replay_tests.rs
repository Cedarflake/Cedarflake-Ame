use super::*;

fn request() -> ReadJournalVolumeRequest {
    ReadJournalVolumeRequest {
        caller: CallerClaim {
            process_id: 41,
            session_id: 2,
            client_instance: [3; 16],
        },
        roots: vec![SharedJournalRootRequest {
            root: RootAuthorization {
                root_id: "root-a".to_owned(),
                root_generation: 1,
                volume_id: "volume-a".to_owned(),
                root_identity: vec![4; 16],
                canonical_root_utf16: "C:\\Photos".encode_utf16().collect(),
            },
            root_capability: RootCapability([7; 32]),
            start_usn: 20,
        }],
        pending_renames: Vec::new(),
        journal_id: 7,
        end_usn: 20,
        max_records: 16,
        max_evidence_bytes: 4_096,
        timeout_ms: 1_000,
    }
}

fn response(start_usn: i64, covered: i64, complete: bool) -> BrokerResponse {
    BrokerResponse::ReadVolume {
        request_id: 77,
        client_instance: [3; 16],
        volume_id: "volume-a".to_owned(),
        journal_id: 7,
        requested_end_usn: 20,
        max_records: 16,
        max_evidence_bytes: 4_096,
        outcomes: vec![SharedJournalRootOutcome {
            binding: ResponseBinding {
                client_instance: [3; 16],
                root_id: "root-a".to_owned(),
                root_generation: 1,
                volume_id: "volume-a".to_owned(),
            },
            requested_start_usn: start_usn,
            covered_until_usn: Some(covered),
            is_complete: complete,
            candidates: Vec::new(),
            failure: None,
        }],
        handoffs: Vec::new(),
        pending_renames: Vec::new(),
    }
}

#[test]
fn empty_read_request_and_terminal_receipt_survive_wire_round_trip() {
    let request = BrokerRequest::ReadVolume {
        request_id: 77,
        request: request(),
    };
    assert_eq!(
        decode_request(&encode_request(&request).expect("request")),
        Ok(request)
    );
    let response = response(20, 20, true);
    assert_eq!(
        decode_response(&encode_response(&response).expect("receipt")),
        Ok(response)
    );
}

#[test]
fn empty_receipt_cannot_forge_progress_or_partial_completion() {
    for (start, covered, complete) in [
        (20, 20, false),
        (20, 21, true),
        (20, 19, true),
        (10, 10, false),
    ] {
        assert!(encode_response(&response(start, covered, complete)).is_err());
    }
    let mut reversed = request();
    reversed.roots[0].start_usn = 21;
    assert!(
        encode_request(&BrokerRequest::ReadVolume {
            request_id: 78,
            request: reversed
        })
        .is_err()
    );
}
