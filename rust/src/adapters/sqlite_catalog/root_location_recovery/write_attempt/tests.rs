use std::sync::atomic::AtomicUsize;

use super::*;

#[test]
fn retired_callback_cannot_interrupt_a_later_catalog_operation() {
    let attempt = LocationWriteAttempt::new();
    let interrupted = Arc::new(AtomicUsize::new(0));
    let observed = Arc::clone(&interrupted);
    let callback = attempt.callback(move || {
        observed.fetch_add(1, Ordering::Relaxed);
    });
    drop(attempt.retirement_guard());
    callback();
    assert_eq!(interrupted.load(Ordering::Relaxed), 0);
    assert!(!attempt.is_interrupted());
    attempt.ensure_running().unwrap();
}

#[test]
fn preemption_retains_its_cause_but_disables_interruption_before_rollback() {
    let attempt = LocationWriteAttempt::new();
    attempt.callback(|| {})();
    assert!(attempt.is_interrupted());
    drop(attempt.retirement_guard());
    assert!(!attempt.is_interrupted());
    assert_eq!(
        attempt
            .classify(ScanError::new(
                "catalog_database_interrupted",
                "interrupted"
            ))
            .code,
        "root_location_recovery_preempted"
    );
}
