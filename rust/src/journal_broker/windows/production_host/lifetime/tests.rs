use std::sync::Barrier;

use super::*;

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
    let first = lifetime.admit().unwrap().unwrap();
    let draining = lifetime.admit().unwrap().unwrap();
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
    let first = lifetime.admit().unwrap().unwrap();
    let second = lifetime.admit().unwrap().unwrap();
    assert!(matches!(lifetime.admit(), Err(AdmissionError::Capacity)));
    drop(first);
    let replacement = lifetime.admit().unwrap().unwrap();
    assert!(matches!(lifetime.admit(), Err(AdmissionError::Capacity)));
    drop((second, replacement));
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
