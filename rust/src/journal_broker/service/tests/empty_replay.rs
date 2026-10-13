use super::*;

fn counted_service(
    physical_reads: &Arc<AtomicUsize>,
) -> BrokerService<SharedCountingBackend, FakeAuthorizer> {
    BrokerService::new(
        SharedCountingBackend {
            physical_reads: Arc::clone(physical_reads),
            post_admission_failure_index: None,
            post_admission_failure_barrier: None,
            handoff_coordinates: Vec::new(),
        },
        FakeAuthorizer {
            allow_root: true,
            name_semantics: vec![NameComparisonSemantics::WindowsOrdinalIgnoreCase],
        },
    )
}

#[test]
fn empty_closing_read_round_trips_through_authenticated_service() {
    let physical_reads = Arc::new(AtomicUsize::new(0));
    let service = counted_service(&physical_reads);
    let authenticated = connection(71);
    service
        .open_connection(&authenticated)
        .expect("open connection");
    let mut request = shared_read_request(false);
    request.roots.truncate(1);
    request.roots[0].start_usn = request.end_usn;
    let accepted = round_trip(
        &service,
        BrokerRequest::ReadVolume {
            request_id: 9_101,
            request,
        },
        &authenticated,
    );
    assert!(matches!(
        accepted,
        BrokerResponse::ReadVolumeAccepted {
            requested_start_usn: 20,
            requested_end_usn: 20,
            root_count: 1,
            ..
        }
    ));
    let response = service
        .execute_accepted_read_frame(9_101, caller().client_instance, &authenticated)
        .expect("execute empty closing read");
    let response =
        decode_response(decode_frame(&response).expect("frame")).expect("empty response");
    let BrokerResponse::ReadVolume {
        outcomes,
        handoffs,
        pending_renames,
        ..
    } = response
    else {
        panic!("expected successful empty replay")
    };
    assert_eq!(physical_reads.load(Ordering::Acquire), 1);
    assert_eq!(outcomes.len(), 1);
    let outcome = &outcomes[0];
    assert_eq!(outcome.requested_start_usn, 20);
    assert_eq!(outcome.covered_until_usn, Some(20));
    assert!(outcome.is_complete);
    assert!(outcome.failure.is_none());
    assert!(outcome.candidates.is_empty());
    assert!(handoffs.is_empty());
    assert!(pending_renames.is_empty());
}

#[test]
fn empty_read_still_requires_current_root_authority() {
    let physical_reads = Arc::new(AtomicUsize::new(0));
    let service = counted_service(&physical_reads);
    let authenticated = connection(72);
    service
        .open_connection(&authenticated)
        .expect("open connection");
    let mut request = shared_read_request(false);
    request.roots.truncate(1);
    request.roots[0].start_usn = request.end_usn;
    request.roots[0].root_capability = RootCapability([8; 32]);
    assert!(matches!(
        round_trip(
            &service,
            BrokerRequest::ReadVolume {
                request_id: 9_102,
                request
            },
            &authenticated,
        ),
        BrokerResponse::ReadVolumeAccepted { .. }
    ));
    let bytes = service
        .execute_accepted_read_frame(9_102, caller().client_instance, &authenticated)
        .expect("execute unauthorized read");
    let response = decode_response(decode_frame(&bytes).expect("frame")).expect("failure response");
    let BrokerResponse::ReadVolume { outcomes, .. } = response else {
        panic!("expected root-scoped failure")
    };
    assert_eq!(physical_reads.load(Ordering::Acquire), 0);
    assert_eq!(
        outcomes[0].failure.map(|failure| failure.code),
        Some(BrokerFailureCode::RootUnauthorized)
    );
    assert_eq!(outcomes[0].covered_until_usn, None);
    assert!(!outcomes[0].is_complete);
}
