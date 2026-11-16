use std::sync::Barrier;

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
    assert!(first.bind_principal(principal(1)).unwrap());
    assert!(draining.bind_principal(principal(2)).unwrap());
    drop(first);
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
    let mut first = lifetime.admit().unwrap().unwrap();
    let mut second = lifetime.admit().unwrap().unwrap();
    assert!(first.bind_principal(principal(1)).unwrap());
    assert!(second.bind_principal(principal(2)).unwrap());
    assert!(matches!(lifetime.admit(), Err(AdmissionError::Capacity)));
    drop(first);
    let mut replacement = lifetime.admit().unwrap().unwrap();
    assert!(replacement.bind_principal(principal(3)).unwrap());
    assert!(matches!(lifetime.admit(), Err(AdmissionError::Capacity)));
    drop((second, replacement));
}

#[test]
fn one_authenticated_principal_cannot_retain_both_connection_slots() {
    let lifetime = ServiceLifetime::new(Instant::now(), Duration::from_secs(10), 2);
    let mut first = lifetime.admit().unwrap().unwrap();
    assert!(first.bind_principal(principal(1)).unwrap());

    let mut duplicate = lifetime.admit().unwrap().unwrap();
    assert!(!duplicate.bind_principal(principal(1)).unwrap());
    drop(duplicate);

    let mut peer = lifetime.admit().unwrap().unwrap();
    assert!(peer.bind_principal(principal(2)).unwrap());
    assert!(matches!(lifetime.admit(), Err(AdmissionError::Capacity)));
    drop((first, peer));
}

#[test]
fn rejected_duplicate_lease_cannot_release_the_active_principal() {
    let lifetime = ServiceLifetime::new(Instant::now(), Duration::from_secs(10), 2);
    let mut first = lifetime.admit().unwrap().unwrap();
    assert!(first.bind_principal(principal(1)).unwrap());

    let mut duplicate = lifetime.admit().unwrap().unwrap();
    assert!(!duplicate.bind_principal(principal(1)).unwrap());
    drop(duplicate);

    let mut replacement = lifetime.admit().unwrap().unwrap();
    assert!(!replacement.bind_principal(principal(1)).unwrap());
    drop(first);
    assert!(replacement.bind_principal(principal(1)).unwrap());
}

#[test]
fn racing_same_principal_bindings_have_one_winner() {
    let lifetime = ServiceLifetime::new(Instant::now(), Duration::from_secs(10), 2);
    let first = lifetime.admit().unwrap().unwrap();
    let second = lifetime.admit().unwrap().unwrap();
    let barrier = Arc::new(Barrier::new(2));

    let first_barrier = Arc::clone(&barrier);
    let first_thread = std::thread::spawn(move || {
        let mut lease = first;
        first_barrier.wait();
        let admitted = lease.bind_principal(principal(1)).unwrap();
        (admitted, lease)
    });
    let second_thread = std::thread::spawn(move || {
        let mut lease = second;
        barrier.wait();
        let admitted = lease.bind_principal(principal(1)).unwrap();
        (admitted, lease)
    });

    let (first_admitted, first) = first_thread.join().unwrap();
    let (second_admitted, second) = second_thread.join().unwrap();
    assert_ne!(first_admitted, second_admitted);
    let (winner, loser) = if first_admitted {
        (first, second)
    } else {
        (second, first)
    };
    drop(loser);

    let mut replacement = lifetime.admit().unwrap().unwrap();
    assert!(!replacement.bind_principal(principal(1)).unwrap());
    drop(winner);
    assert!(replacement.bind_principal(principal(1)).unwrap());
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
