use super::*;

fn admit(registry: &CatalogReclamationRegistry, operation_id: &str) -> ReclamationAdmission {
    registry
        .admit(PathBuf::from("catalog"), operation_id.to_owned(), true)
        .expect("admit request")
}

fn snapshot(registry: &CatalogReclamationRegistry) -> CatalogReclamationSnapshot {
    registry
        .snapshot(Path::new("catalog"))
        .expect("read registry")
        .expect("registered catalog")
}

#[test]
fn a_request_before_worker_retirement_is_handed_to_the_existing_worker() {
    let registry = CatalogReclamationRegistry::default();
    let first = admit(&registry, "first").worker.expect("first worker");
    first.update(|snapshot| snapshot.phase = CatalogReclamationPhase::Reclaiming);
    let queued = admit(&registry, "second");
    assert!(queued.worker.is_none());
    assert_eq!(queued.snapshot.operation_id.as_deref(), Some("first"));

    let second = first
        .finish(Ok(ReclamationCompletion::Completed))
        .expect("finish first")
        .expect("pending successor");
    let current = snapshot(&registry);
    assert_eq!(current.operation_id.as_deref(), Some("second"));
    assert_eq!(current.phase, CatalogReclamationPhase::Queued);
    assert!(Arc::ptr_eq(&first.entry, &second.entry));
    assert!(!Arc::ptr_eq(&first.token, &second.token));
    assert!(
        second
            .finish(Ok(ReclamationCompletion::Completed))
            .expect("finish second")
            .is_none()
    );
    assert_eq!(
        snapshot(&registry).phase,
        CatalogReclamationPhase::Completed
    );
}

#[test]
fn a_request_after_worker_retirement_claims_a_new_worker_in_the_same_entry() {
    let registry = CatalogReclamationRegistry::default();
    let first = admit(&registry, "first").worker.expect("first worker");
    assert!(
        first
            .finish(Ok(ReclamationCompletion::Completed))
            .expect("retire first")
            .is_none()
    );
    let second = admit(&registry, "second")
        .worker
        .expect("new worker after retirement");
    assert!(Arc::ptr_eq(&first.entry, &second.entry));
    assert_eq!(snapshot(&registry).operation_id.as_deref(), Some("second"));
    assert!(admit(&registry, "third").worker.is_none());
    assert_eq!(
        second
            .finish(Ok(ReclamationCompletion::Completed))
            .expect("handoff third")
            .expect("third worker")
            .snapshot()
            .expect("third snapshot")
            .operation_id
            .as_deref(),
        Some("third")
    );
}

#[test]
fn late_preemption_cannot_revive_a_terminal_snapshot_or_interrupt_a_new_run() {
    let registry = CatalogReclamationRegistry::default();
    let first = admit(&registry, "first").worker.expect("first worker");
    let old_control = first.begin_attempt().expect("first attempt");
    assert!(
        first
            .finish(Ok(ReclamationCompletion::Completed))
            .expect("retire first")
            .is_none()
    );
    old_control.preempt();
    registry.preempt(Path::new("catalog"));
    first.update(|snapshot| snapshot.phase = CatalogReclamationPhase::WaitingForIdle);
    assert_eq!(
        snapshot(&registry).phase,
        CatalogReclamationPhase::Completed
    );

    let second = admit(&registry, "second").worker.expect("second worker");
    let new_control = second.begin_attempt().expect("second attempt");
    old_control.cancel();
    old_control.preempt();
    assert!(!new_control.is_interrupted());
    assert!(!second.is_cancelled());
    first.finish_attempt();
    assert_eq!(
        first
            .begin_attempt()
            .err()
            .expect("retired attempt rejected")
            .code,
        "catalog_reclamation_run_superseded"
    );
    assert_eq!(
        first
            .finish(Ok(ReclamationCompletion::Completed))
            .err()
            .expect("retired completion rejected")
            .code,
        "catalog_reclamation_run_superseded"
    );
    registry.preempt(Path::new("catalog"));
    assert!(new_control.is_interrupted());
    assert!(!new_control.is_user_cancelled());
    assert_eq!(snapshot(&registry).phase, CatalogReclamationPhase::Queued);
}

#[test]
fn coalesced_requests_survive_cancellation_with_a_fresh_generation() {
    let registry = CatalogReclamationRegistry::default();
    let first = admit(&registry, "first").worker.expect("first worker");
    let old_control = first.begin_attempt().expect("old control");
    assert!(registry.cancel("first"));
    assert!(old_control.is_user_cancelled());
    assert!(admit(&registry, "superseded-pending").worker.is_none());
    assert!(admit(&registry, "latest").worker.is_none());
    let latest = first
        .finish(Ok(ReclamationCompletion::Cancelled))
        .expect("finish cancellation")
        .expect("latest request retained");
    let new_control = latest.begin_attempt().expect("new control");
    assert_eq!(snapshot(&registry).operation_id.as_deref(), Some("latest"));
    assert!(!registry.cancel("first"));
    old_control.cancel();
    assert!(!latest.is_cancelled());
    assert!(!new_control.is_interrupted());
    assert!(registry.cancel("latest"));
    assert!(new_control.is_user_cancelled());
    assert!(
        latest
            .finish(Ok(ReclamationCompletion::Cancelled))
            .expect("finish latest")
            .is_none()
    );
    assert!(!registry.cancel("latest"));
    assert_eq!(
        snapshot(&registry).phase,
        CatalogReclamationPhase::Cancelled
    );
}

#[test]
fn late_attempt_preemption_does_not_poison_the_next_attempt_of_the_same_run() {
    let registry = CatalogReclamationRegistry::default();
    let worker = admit(&registry, "first").worker.expect("worker");
    let old = worker.begin_attempt().expect("first attempt");
    worker.finish_attempt();
    let current = worker.begin_attempt().expect("next attempt");
    old.preempt();
    assert!(!current.is_interrupted());
    registry.preempt(Path::new("catalog"));
    assert!(current.is_interrupted());
    assert!(!worker.is_cancelled());
}

#[test]
fn startup_recovery_does_not_enqueue_or_restart_an_observed_operation() {
    let registry = CatalogReclamationRegistry::default();
    let key = PathBuf::from("catalog");
    let worker = registry
        .admit(key.clone(), "startup".to_owned(), false)
        .expect("initial recovery")
        .worker
        .expect("startup worker");
    assert!(
        registry
            .admit(key.clone(), "poll".to_owned(), false)
            .expect("active recovery poll")
            .worker
            .is_none()
    );
    assert!(
        worker
            .finish(Ok(ReclamationCompletion::Completed))
            .expect("no recovery rescan")
            .is_none()
    );
    let terminal = registry
        .admit(key, "poll".to_owned(), false)
        .expect("terminal recovery poll");
    assert!(terminal.worker.is_none());
    assert_eq!(terminal.snapshot.operation_id.as_deref(), Some("startup"));
    assert_eq!(terminal.snapshot.phase, CatalogReclamationPhase::Completed);
}

#[test]
fn worker_start_failure_settles_the_latest_coalesced_request_and_allows_retry() {
    let registry = CatalogReclamationRegistry::default();
    let first = admit(&registry, "first").worker.expect("first worker");
    assert!(admit(&registry, "latest").worker.is_none());
    let failure = ScanError::new(
        "catalog_reclamation_worker_unavailable",
        "Injected worker start failure",
    );
    first.worker_unavailable(failure.clone());
    let failed = snapshot(&registry);
    assert_eq!(failed.operation_id.as_deref(), Some("latest"));
    assert_eq!(failed.phase, CatalogReclamationPhase::Failed);
    assert_eq!(failed.error_code.as_deref(), Some(failure.code.as_str()));
    assert!(!registry.cancel("latest"));

    let retry = admit(&registry, "retry").worker.expect("retry worker");
    first.worker_unavailable(failure);
    assert_eq!(snapshot(&registry).operation_id.as_deref(), Some("retry"));
    assert_eq!(snapshot(&registry).phase, CatalogReclamationPhase::Queued);
    assert!(
        retry
            .finish(Ok(ReclamationCompletion::Completed))
            .expect("no lost pending request")
            .is_none()
    );
}

#[test]
fn permanent_failure_retires_the_worker_without_losing_its_error() {
    let registry = CatalogReclamationRegistry::default();
    let worker = admit(&registry, "first").worker.expect("worker");
    assert!(
        worker
            .finish(Err(ScanError::new("injected_failure", "Injected failure")))
            .expect("retire failed worker")
            .is_none()
    );
    let failed = snapshot(&registry);
    assert_eq!(failed.phase, CatalogReclamationPhase::Failed);
    assert_eq!(failed.error_code.as_deref(), Some("injected_failure"));
    assert!(admit(&registry, "retry").worker.is_some());
}
