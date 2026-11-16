use super::*;
use crate::journal_broker::windows::connection::{
    HANDSHAKE_BYTES, SERVER_ACCEPT_BYTES, decode_handshake, encode_client_proof,
    validate_server_accept,
};
use crate::journal_broker::windows::production_host::lifetime::ServiceLifetime;

#[test]
fn native_same_user_reconnect_waits_for_the_previous_pipe_to_close() {
    let lifetime = ServiceLifetime::new(Instant::now(), Duration::from_secs(10), 2);
    let identities = ProcessConnectionIdentitySource::new();
    let root_budget = WindowsRootRegistrationBudget::production();
    let worker_budget = ReadWorkerBudget::new(MAX_HOST_READ_WORKERS);
    let start = || {
        let (server, client) = connected_test_pipe();
        let mut connection = lifetime.admit().unwrap().unwrap();
        let identity = identities.next_identity().unwrap();
        let root_budget = root_budget.clone();
        let worker_budget = worker_budget.clone();
        let worker = std::thread::spawn(move || {
            let stop = AtomicBool::new(false);
            serve_named_pipe(
                &server,
                identity,
                Some(&stop),
                root_budget,
                worker_budget,
                Some(&mut connection),
            )
        });
        (worker, client)
    };

    let (first, first_client) = start();
    let identity = send_client_proof(&first_client);
    receive_server_accept(&first_client, identity);

    let (duplicate, duplicate_client) = start();
    send_client_proof(&duplicate_client);
    assert!(
        read_exact_bounded(
            &duplicate_client,
            SERVER_ACCEPT_BYTES,
            Instant::now() + Duration::from_secs(2),
        )
        .is_err()
    );
    drop(duplicate_client);
    assert!(matches!(
        duplicate.join().unwrap(),
        Err(HostError::AdmissionRejected)
    ));

    let (replacement, replacement_client) = start();
    let identity = send_client_proof(&replacement_client);
    drop(first_client);
    receive_server_accept(&replacement_client, identity);
    assert!(first.join().unwrap().is_ok());
    drop(replacement_client);
    assert!(replacement.join().unwrap().is_ok());
    assert!(
        lifetime
            .retire_if_idle(Instant::now() + Duration::from_secs(10))
            .unwrap()
    );
}

fn send_client_proof(client: &OwnedHandle) -> ServerConnectionIdentity {
    let handshake = read_exact_bounded(
        client,
        HANDSHAKE_BYTES,
        Instant::now() + Duration::from_secs(2),
    )
    .expect("server handshake");
    let identity = decode_handshake(&handshake).expect("bound server identity");
    write_exact_bounded(
        client,
        encode_client_proof(identity).to_vec(),
        Instant::now() + Duration::from_secs(2),
    )
    .expect("connection proof");
    identity
}

fn receive_server_accept(client: &OwnedHandle, identity: ServerConnectionIdentity) {
    let accepted = read_exact_bounded(
        client,
        SERVER_ACCEPT_BYTES,
        Instant::now() + Duration::from_secs(2),
    )
    .expect("authenticated server acceptance");
    validate_server_accept(identity, &accepted).expect("bound acceptance identity");
}
