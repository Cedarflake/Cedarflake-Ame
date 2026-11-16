use std::sync::Barrier;
use std::sync::mpsc::sync_channel;

use super::*;

fn principal(value: u8) -> AuthenticatedPrincipal {
    AuthenticatedPrincipal::for_test(value)
}

#[test]
fn service_retires_at_the_idle_boundary_and_rejects_late_connections() {
    let now = Instant::now();
    let lifetime = ServiceLifetime::new(now, Duration::from_secs(10), 2);
    assert!(
        !lifetime
            .retire_if_idle(now + Duration::from_secs(9))
            .unwrap()
    );
    assert!(
        lifetime
            .retire_if_idle(now + Duration::from_secs(10))
            .unwrap()
    );
    assert!(lifetime.admit().unwrap().is_none());
}

#[test]
fn active_peer_and_worker_drain_lease_prevent_idle_retirement() {
    let now = Instant::now();
    let lifetime = ServiceLifetime::new(now, Duration::from_secs(10), 2);
    let mut first = lifetime.admit().unwrap().unwrap();
    let mut draining = lifetime.admit().unwrap().unwrap();
    let first_principal = first.bind_principal(principal(1), now, None).unwrap();
    let draining_principal = draining.bind_principal(principal(2), now, None).unwrap();
    drop(first_principal);
    drop(first);
    assert!(
        !lifetime
            .retire_if_idle(now + Duration::from_secs(60))
            .unwrap()
    );
    drop(draining_principal);
    assert!(
        !lifetime
            .retire_if_idle(now + Duration::from_secs(60))
            .unwrap()
    );
    drop(draining);
    assert!(!lifetime.retire_if_idle(Instant::now()).unwrap());
    assert!(
        lifetime
            .retire_if_idle(Instant::now() + Duration::from_secs(10))
            .unwrap()
    );
}

#[test]
fn connection_capacity_is_reclaimed_only_by_the_owned_lease() {
    let lifetime = ServiceLifetime::new(Instant::now(), Duration::from_secs(10), 2);
    let first = lifetime.admit().unwrap().unwrap();
    let second = lifetime.admit().unwrap().unwrap();
    assert!(matches!(lifetime.admit(), Err(AdmissionError::Capacity)));
    drop(first);
    let replacement = lifetime.admit().unwrap().unwrap();
    assert!(matches!(lifetime.admit(), Err(AdmissionError::Capacity)));
    drop((second, replacement));
}

#[test]
fn one_authenticated_principal_cannot_open_two_request_sessions() {
    let lifetime = ServiceLifetime::new(Instant::now(), Duration::from_secs(10), 2);
    let mut first = lifetime.admit().unwrap().unwrap();
    let _first_principal = first
        .bind_principal(principal(1), Instant::now(), None)
        .unwrap();
    let mut duplicate = lifetime.admit().unwrap().unwrap();
    assert!(matches!(
        duplicate.bind_principal(principal(1), Instant::now(), None),
        Err(AdmissionError::PrincipalBusy)
    ));
    drop(duplicate);
    let mut peer = lifetime.admit().unwrap().unwrap();
    let _peer_principal = peer
        .bind_principal(principal(2), Instant::now(), None)
        .unwrap();
    assert!(matches!(lifetime.admit(), Err(AdmissionError::Capacity)));
}

#[test]
fn rejected_duplicate_lease_cannot_release_the_active_principal() {
    let lifetime = ServiceLifetime::new(Instant::now(), Duration::from_secs(10), 2);
    let mut first = lifetime.admit().unwrap().unwrap();
    let first_principal = first
        .bind_principal(principal(1), Instant::now(), None)
        .unwrap();
    let mut duplicate = lifetime.admit().unwrap().unwrap();
    assert!(matches!(
        duplicate.bind_principal(principal(1), Instant::now(), None),
        Err(AdmissionError::PrincipalBusy)
    ));
    drop(duplicate);
    let mut replacement = lifetime.admit().unwrap().unwrap();
    assert!(matches!(
        replacement.bind_principal(principal(1), Instant::now(), None),
        Err(AdmissionError::PrincipalBusy)
    ));
    drop(first_principal);
    let replacement_principal = replacement
        .bind_principal(principal(1), Instant::now(), None)
        .unwrap();
    assert!(matches!(lifetime.admit(), Err(AdmissionError::Capacity)));
    drop(first);
    let mut next = lifetime.admit().unwrap().unwrap();
    assert!(matches!(
        next.bind_principal(principal(1), Instant::now(), None),
        Err(AdmissionError::PrincipalBusy)
    ));
    drop(replacement_principal);
    let _next_principal = next
        .bind_principal(principal(1), Instant::now(), None)
        .unwrap();
}

#[test]
fn racing_same_principal_bindings_have_one_winner() {
    let lifetime = ServiceLifetime::new(Instant::now(), Duration::from_secs(10), 2);
    let barrier = Arc::new(Barrier::new(2));
    let (result_sender, result_receiver) = sync_channel(2);
    let mut threads = Vec::new();
    let mut releases = Vec::new();
    for _ in 0..2 {
        let mut connection = lifetime.admit().unwrap().unwrap();
        let barrier = Arc::clone(&barrier);
        let result_sender = result_sender.clone();
        let (release, released) = sync_channel(1);
        releases.push(release);
        threads.push(std::thread::spawn(move || {
            barrier.wait();
            let binding = connection.bind_principal(principal(1), Instant::now(), None);
            result_sender.send(binding.is_ok()).unwrap();
            released.recv_timeout(Duration::from_secs(5)).unwrap();
            drop(binding);
        }));
    }
    let first = result_receiver
        .recv_timeout(Duration::from_secs(5))
        .unwrap();
    let second = result_receiver
        .recv_timeout(Duration::from_secs(5))
        .unwrap();
    for release in releases {
        release.send(()).unwrap();
    }
    for thread in threads {
        thread.join().unwrap();
    }
    assert_ne!(first, second);
}

#[test]
fn waiting_principal_handover_releases_no_global_capacity() {
    let lifetime = ServiceLifetime::new(Instant::now(), Duration::from_secs(10), 2);
    let mut first = lifetime.admit().unwrap().unwrap();
    let first_principal = first
        .bind_principal(principal(1), Instant::now(), None)
        .unwrap();
    let mut replacement = lifetime.admit().unwrap().unwrap();
    let (started, starting) = sync_channel(1);
    let (admitted, admission) = sync_channel(1);
    let (release, released) = sync_channel(1);
    let worker = std::thread::spawn(move || {
        started.send(()).unwrap();
        let binding = replacement
            .bind_principal(principal(1), Instant::now() + Duration::from_secs(2), None)
            .unwrap();
        admitted.send(()).unwrap();
        released.recv_timeout(Duration::from_secs(5)).unwrap();
        drop(binding);
    });
    starting.recv_timeout(Duration::from_secs(5)).unwrap();
    assert!(admission.recv_timeout(Duration::from_millis(30)).is_err());
    drop(first_principal);
    admission.recv_timeout(Duration::from_secs(2)).unwrap();
    assert!(matches!(lifetime.admit(), Err(AdmissionError::Capacity)));
    release.send(()).unwrap();
    worker.join().unwrap();
    assert!(
        !lifetime
            .retire_if_idle(Instant::now() + Duration::from_secs(60))
            .unwrap()
    );
    drop(first);
}

#[test]
fn occupied_principal_wait_observes_deadline_and_service_stop() {
    let lifetime = ServiceLifetime::new(Instant::now(), Duration::from_secs(10), 2);
    let mut first = lifetime.admit().unwrap().unwrap();
    let _first_principal = first
        .bind_principal(principal(1), Instant::now(), None)
        .unwrap();
    let mut waiting = lifetime.admit().unwrap().unwrap();
    assert!(matches!(
        waiting.bind_principal(
            principal(1),
            Instant::now() + Duration::from_millis(30),
            None
        ),
        Err(AdmissionError::PrincipalBusy)
    ));
    let stop = Arc::new(AtomicBool::new(false));
    let worker_stop = Arc::clone(&stop);
    let (started, starting) = sync_channel(1);
    let (finished, finishing) = sync_channel(1);
    let worker = std::thread::spawn(move || {
        started.send(()).unwrap();
        let stopped = matches!(
            waiting.bind_principal(
                principal(1),
                Instant::now() + Duration::from_secs(5),
                Some(worker_stop.as_ref()),
            ),
            Err(AdmissionError::Stopped)
        );
        finished.send(stopped).unwrap();
    });
    starting.recv_timeout(Duration::from_secs(5)).unwrap();
    stop.store(true, Ordering::Release);
    assert!(finishing.recv_timeout(Duration::from_secs(1)).unwrap());
    worker.join().unwrap();
}

#[test]
fn racing_connection_and_idle_retirement_have_one_admission_winner() {
    let now = Instant::now();
    let lifetime = ServiceLifetime::new(now, Duration::from_secs(10), 2);
    let barrier = Arc::new(Barrier::new(2));
    let admitting = lifetime.clone();
    let admission_barrier = Arc::clone(&barrier);
    let thread = std::thread::spawn(move || {
        admission_barrier.wait();
        admitting.admit().unwrap()
    });
    barrier.wait();
    let retired = lifetime
        .retire_if_idle(now + Duration::from_secs(10))
        .unwrap();
    let admitted = thread.join().unwrap();
    assert_ne!(retired, admitted.is_some());
}
