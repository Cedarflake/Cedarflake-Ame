use super::*;

#[test]
fn panicked_constructor_is_contained_without_clearing_a_later_epoch() {
    let registry = SynchronizationRuntimeRegistry::default();

    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        start_runtime_with(&registry, || -> ProductionSynchronization {
            panic!("secret constructor panic payload")
        })
    }));

    let error = outcome
        .expect("the registry boundary must contain the constructor panic")
        .expect_err("a contained constructor panic remains a structured failure");
    assert_eq!(error.code, "library_synchronization_owner_panicked");
    assert_eq!(
        error.message,
        "The library synchronization owner panicked and entered bounded recovery"
    );
    assert!(!error.message.contains("secret constructor panic payload"));
    assert!(matches!(
        registry.state.lock().expect("local registry").entry,
        SynchronizationRuntimeEntry::Empty
    ));

    let SynchronizationRuntimeClaim::Construct {
        epoch: next_epoch, ..
    } = claim_runtime_for_start(&registry).expect("constructor recovery admits a later epoch")
    else {
        panic!("constructor recovery must leave an empty registry")
    };
    assert_eq!(next_epoch, 2);
    recover_starting_runtime_panic(
        &registry,
        1,
        RuntimeStartTicket::from_ordinal(LifecycleOrdinal(1)),
    )
    .expect("ignore stale constructor recovery");
    assert!(matches!(
        registry.state.lock().expect("local registry").entry,
        SynchronizationRuntimeEntry::Starting { epoch: 2, .. }
    ));
}

#[test]
fn panicked_constructor_releases_a_concurrent_stopper_with_the_same_deadline() {
    let registry = Arc::new(SynchronizationRuntimeRegistry::default());
    let constructor_entered = Arc::new(AtomicBool::new(false));
    let constructor_release = Arc::new(AtomicBool::new(false));
    let owner_registry = Arc::clone(&registry);
    let owner_entered = Arc::clone(&constructor_entered);
    let owner_release = Arc::clone(&constructor_release);
    let owner = thread::spawn(move || {
        start_runtime_with(&owner_registry, move || -> ProductionSynchronization {
            owner_entered.store(true, Ordering::Release);
            while !owner_release.load(Ordering::Acquire) {
                thread::yield_now();
            }
            panic!("secret concurrent constructor panic payload")
        })
    });
    wait_for_test_signal(&constructor_entered);

    let stop_deadline = Instant::now() + Duration::from_millis(500);
    let stop_registry = Arc::clone(&registry);
    let stopper = thread::spawn(move || stop_runtime_until(&stop_registry, stop_deadline));
    let transition_deadline = Instant::now() + Duration::from_millis(100);
    loop {
        if matches!(
            registry.state.lock().expect("local registry").entry,
            SynchronizationRuntimeEntry::Stopping { .. }
        ) {
            break;
        }
        assert!(
            Instant::now() < transition_deadline,
            "concurrent stop did not claim the constructor epoch"
        );
        thread::yield_now();
    }
    assert_eq!(retained_stop_deadline(&registry), Some(stop_deadline));

    constructor_release.store(true, Ordering::Release);
    let owner_error = owner
        .join()
        .expect("join panicked-constructor owner boundary")
        .expect_err("constructor panic remains a structured failure");
    assert_eq!(owner_error.code, "library_synchronization_owner_panicked");
    assert!(!owner_error.message.contains("secret concurrent"));
    stopper
        .join()
        .expect("join concurrent stopper")
        .expect("constructor recovery notifies the concurrent stopper");
    assert!(matches!(
        registry.state.lock().expect("local registry").entry,
        SynchronizationRuntimeEntry::Empty
    ));

    let SynchronizationRuntimeClaim::Construct { epoch, .. } =
        claim_runtime_for_start(&registry).expect("constructor recovery admits a new epoch")
    else {
        panic!("constructor recovery must leave the registry empty")
    };
    assert_eq!(epoch, 2);
}

#[test]
fn panicked_poll_owner_is_contained_and_releases_the_registry_epoch() {
    let close_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let registry = ready_test_registry(runtime_with_close_probe(Arc::clone(&close_count)));
    let (epoch, cancelled, runtime) =
        claim_runtime_for_poll(&registry).expect("claim panic poll owner");
    let owner_ticket = active_owner_ticket(&registry);

    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        execute_runtime_operation(
            &registry,
            epoch,
            owner_ticket,
            cancelled,
            runtime,
            |_| -> Result<(), ScanError> { panic!("controlled poll owner panic") },
        )
    }));

    let error = outcome
        .expect("the registry boundary must contain the poll owner panic")
        .expect_err("a contained owner panic remains a structured failure");
    assert_eq!(error.code, "library_synchronization_owner_panicked");
    assert!(matches!(
        registry.state.lock().expect("local registry").entry,
        SynchronizationRuntimeEntry::Empty
    ));
    assert_eq!(close_count.load(Ordering::Acquire), 1);

    let SynchronizationRuntimeClaim::Construct { .. } =
        claim_runtime_for_start(&registry).expect("panic cleanup permits the next epoch")
    else {
        panic!("panic cleanup must leave an empty registry")
    };
}

#[test]
fn panicked_poll_racing_stop_before_deadline_drains_the_same_owner_once() {
    let close_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let registry = ready_test_registry(runtime_with_close_probe(Arc::clone(&close_count)));
    let runtime_identity = {
        let state = registry.state.lock().expect("local registry");
        let SynchronizationRuntimeEntry::Ready { runtime, .. } = &state.entry else {
            panic!("test runtime must begin ready")
        };
        Arc::clone(runtime)
    };
    let (epoch, cancelled, runtime) =
        claim_runtime_for_poll(&registry).expect("claim racing panic owner");
    let owner_ticket = active_owner_ticket(&registry);
    let operation_entered = Arc::new(AtomicBool::new(false));
    let operation_release = Arc::new(AtomicBool::new(false));
    let owner_registry = Arc::clone(&registry);
    let owner_entered = Arc::clone(&operation_entered);
    let owner_release = Arc::clone(&operation_release);
    let owner = thread::spawn(move || {
        execute_runtime_operation(
            &owner_registry,
            epoch,
            owner_ticket,
            cancelled,
            runtime,
            move |_| -> Result<(), ScanError> {
                owner_entered.store(true, Ordering::Release);
                while !owner_release.load(Ordering::Acquire) {
                    thread::yield_now();
                }
                panic!("controlled racing poll panic")
            },
        )
    });
    wait_for_test_signal(&operation_entered);

    let stop_deadline = Instant::now() + Duration::from_millis(500);
    let stop_registry = Arc::clone(&registry);
    let stopper = thread::spawn(move || stop_runtime_until(&stop_registry, stop_deadline));
    assert!(wait_for_draining_state(
        &registry,
        Duration::from_millis(100)
    ));
    {
        let state = registry.state.lock().expect("local registry");
        let SynchronizationRuntimeEntry::Draining {
            deadline, runtime, ..
        } = &state.entry
        else {
            panic!("concurrent stop must publish Draining before cleanup")
        };
        assert_eq!(*deadline, stop_deadline);
        assert!(Arc::ptr_eq(runtime, &runtime_identity));
    }

    operation_release.store(true, Ordering::Release);
    let owner_error = owner
        .join()
        .expect("join racing poll owner boundary")
        .expect_err("the racing poll panic remains structured");
    assert_eq!(owner_error.code, "library_synchronization_owner_panicked");
    stopper
        .join()
        .expect("join racing stopper")
        .expect("one drainer completes and the stale drainer exits cleanly");
    assert_eq!(close_count.load(Ordering::Acquire), 1);
    assert!(!runtime_identity.is_poisoned());
    assert!(matches!(
        registry.state.lock().expect("local registry").entry,
        SynchronizationRuntimeEntry::Empty
    ));
}

#[test]
fn panicked_poll_reports_stable_error_after_stop_drains_and_restarts_epoch() {
    let old_close_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let registry = ready_test_registry(runtime_with_close_probe(Arc::clone(&old_close_count)));
    let reached = Arc::new(std::sync::Barrier::new(2));
    let resume = Arc::new(std::sync::Barrier::new(2));
    *registry
        .panic_retention_barrier
        .lock()
        .expect("install panic retention barrier") = Some(PanicRetentionBarrier {
        reached: Arc::clone(&reached),
        resume: Arc::clone(&resume),
    });
    let (epoch, cancelled, runtime) =
        claim_runtime_for_poll(&registry).expect("claim panicked poll owner");
    let owner_ticket = active_owner_ticket(&registry);
    let owner_registry = Arc::clone(&registry);
    let owner = thread::spawn(move || {
        execute_runtime_operation(
            &owner_registry,
            epoch,
            owner_ticket,
            cancelled,
            runtime,
            |_| -> Result<(), ScanError> { panic!("secret raced panic payload") },
        )
    });

    reached.wait();
    stop_runtime_until(&registry, Instant::now() + Duration::from_millis(500))
        .expect("concurrent stop drains the old owner before panic reconciliation");
    assert_eq!(old_close_count.load(Ordering::Acquire), 1);
    assert!(matches!(
        registry.state.lock().expect("local registry").entry,
        SynchronizationRuntimeEntry::Empty
    ));

    let SynchronizationRuntimeClaim::Construct {
        epoch: new_epoch,
        cancelled: new_cancelled,
    } = claim_runtime_for_start(&registry).expect("admit replacement epoch")
    else {
        panic!("a drained owner must admit replacement construction")
    };
    let new_owner_ticket = active_owner_ticket(&registry);
    assert_eq!(new_epoch, epoch + 1);
    let new_close_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let mut replacement = runtime_with_close_probe(Arc::clone(&new_close_count));
    replacement.stop_requested = Arc::clone(&new_cancelled);
    let ConstructedRuntimeDisposition::Poll(new_runtime) = promote_constructed_runtime(
        &registry,
        new_epoch,
        new_owner_ticket,
        Arc::clone(&new_cancelled),
        replacement,
    )
    .expect("install replacement runtime") else {
        panic!("a fresh replacement epoch must enter polling")
    };

    resume.wait();
    let owner_error = owner
        .join()
        .expect("join panicked old owner")
        .expect_err("the old owner panic remains a structured failure");
    assert_eq!(owner_error.code, "library_synchronization_owner_panicked");
    assert_eq!(
        owner_error.message,
        "The library synchronization owner panicked and entered bounded recovery"
    );
    assert!(!owner_error.message.contains("secret raced panic payload"));
    {
        let state = registry.state.lock().expect("replacement registry");
        let SynchronizationRuntimeEntry::Polling {
            epoch: active_epoch,
            cancelled,
            runtime,
            ..
        } = &state.entry
        else {
            panic!("old panic reconciliation must preserve the replacement epoch")
        };
        assert_eq!(*active_epoch, new_epoch);
        assert!(!cancelled.load(Ordering::Acquire));
        assert!(Arc::ptr_eq(runtime, &new_runtime));
    }
    assert_eq!(new_close_count.load(Ordering::Acquire), 0);

    stop_runtime_until(&registry, Instant::now() + Duration::from_millis(500))
        .expect("clean replacement epoch");
    assert_eq!(new_close_count.load(Ordering::Acquire), 1);
}

#[test]
fn panicked_poll_cannot_install_an_external_runtime_into_stopping() {
    let registered_close_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let registry = ready_test_registry(runtime_with_close_probe(Arc::clone(
        &registered_close_count,
    )));
    let deadline = Instant::now() + Duration::from_millis(500);
    let owner_ticket = RuntimeStartTicket::from_ordinal(LifecycleOrdinal(1));
    let cancellation_fence = RuntimeStopFence::from_ordinal(LifecycleOrdinal(2));
    {
        let mut state = registry.state.lock().expect("local registry");
        state.next_lifecycle_ordinal = cancellation_fence.ordinal();
        state.cancelled_through_ordinal = cancellation_fence.ordinal();
        state.entry = SynchronizationRuntimeEntry::Stopping {
            epoch: 1,
            owner_ticket,
            cancellation_fence,
            deadline,
        };
    }
    let external_close_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let external_runtime = Arc::new(Mutex::new(runtime_with_close_probe(Arc::clone(
        &external_close_count,
    ))));
    let cancelled = Arc::new(AtomicBool::new(false));

    let error =
        match retain_panicked_runtime(&registry, 1, owner_ticket, &cancelled, &external_runtime) {
            Ok(_) => {
                panic!("runtime-less Stopping cannot adopt a poll owner's external runtime")
            }
            Err(error) => error,
        };

    assert_eq!(error.code, "library_synchronization_owner_state_invalid");
    assert!(cancelled.load(Ordering::Acquire));
    {
        let state = registry.state.lock().expect("local registry");
        let SynchronizationRuntimeEntry::Stopping {
            epoch,
            deadline: active_deadline,
            ..
        } = state.entry
        else {
            panic!("failed reconciliation must preserve runtime-less Stopping")
        };
        assert_eq!(epoch, 1);
        assert_eq!(active_deadline, deadline);
    }
    assert_eq!(registered_close_count.load(Ordering::Acquire), 0);
    assert_eq!(external_close_count.load(Ordering::Acquire), 0);
}

#[test]
fn panicked_poll_retains_all_workers_and_the_expired_public_stop_deadline() {
    let close_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let close_entered = Arc::new(AtomicBool::new(false));
    let close_release = Arc::new(AtomicBool::new(false));
    let close_finished = Arc::new(AtomicBool::new(false));
    let mut production = runtime_with_blocking_close(
        Arc::clone(&close_count),
        Arc::clone(&close_entered),
        Arc::clone(&close_release),
        Arc::clone(&close_finished),
    );

    let p0_cancelled = Arc::new(AtomicBool::new(false));
    let p0_release = Arc::new(AtomicBool::new(false));
    let p0_finished = Arc::new(AtomicBool::new(false));
    let (p0_sender, p0_receiver) = mpsc::sync_channel(1);
    let p0_worker_cancelled = Arc::clone(&p0_cancelled);
    let p0_worker_release = Arc::clone(&p0_release);
    let p0_worker_finished = Arc::clone(&p0_finished);
    let p0_worker = thread::spawn(move || {
        while !p0_worker_cancelled.load(Ordering::Acquire)
            || !p0_worker_release.load(Ordering::Acquire)
        {
            thread::yield_now();
        }
        p0_worker_finished.store(true, Ordering::Release);
        let _ = p0_sender.send(LiveWorkOutcome::default());
    });
    production.live = Some(LiveTask::from_test_parts(
        "panic-p0".to_owned(),
        p0_cancelled,
        p0_receiver,
        p0_worker,
    ));

    let p1_cancelled = Arc::new(AtomicBool::new(false));
    let p1_release = Arc::new(AtomicBool::new(false));
    let p1_finished = Arc::new(AtomicBool::new(false));
    let (p1_sender, p1_receiver) = mpsc::sync_channel(1);
    let p1_worker_cancelled = Arc::clone(&p1_cancelled);
    let p1_worker_release = Arc::clone(&p1_release);
    let p1_worker_finished = Arc::clone(&p1_finished);
    let p1_worker = thread::spawn(move || {
        while !p1_worker_cancelled.load(Ordering::Acquire)
            || !p1_worker_release.load(Ordering::Acquire)
        {
            thread::yield_now();
        }
        p1_worker_finished.store(true, Ordering::Release);
        let _ = p1_sender.send(Ok(JournalTaskOutcome::Page(
            PersistentJournalEnrollmentReport::default().into(),
        )));
    });
    production.journal = Some(JournalTask::from_test_parts(
        p1_cancelled,
        p1_receiver,
        p1_worker,
    ));

    let p2_cancelled = Arc::new(AtomicBool::new(false));
    let p2_release = Arc::new(AtomicBool::new(false));
    let p2_finished = Arc::new(AtomicBool::new(false));
    let (p2_sender, p2_receiver) = mpsc::sync_channel(1);
    let p2_worker_cancelled = Arc::clone(&p2_cancelled);
    let p2_worker_release = Arc::clone(&p2_release);
    let p2_worker_finished = Arc::clone(&p2_finished);
    let p2_worker = thread::spawn(move || {
        while !p2_worker_cancelled.load(Ordering::Acquire)
            || !p2_worker_release.load(Ordering::Acquire)
        {
            thread::yield_now();
        }
        p2_worker_finished.store(true, Ordering::Release);
        let _ = p2_sender.send(Ok(RecoveryTaskOutcome::LegacyUnownedDrain(
            IncrementalLibraryChangeReport::default(),
        )));
    });
    production.recovery = Some(RecoveryTask {
        root_id: "panic-p2".to_owned(),
        kind: RecoveryTaskKind::LegacyUnownedDrain {
            continuity_revision: 1,
        },
        phase: Arc::new(Mutex::new(LibrarySynchronizationPhase::Reconciliation)),
        cancelled: p2_cancelled,
        receiver: p2_receiver,
        worker: Some(p2_worker),
    });

    let registry = ready_test_registry(production);
    let runtime_identity = {
        let state = registry.state.lock().expect("local registry");
        let SynchronizationRuntimeEntry::Ready { runtime, .. } = &state.entry else {
            panic!("test runtime must begin ready")
        };
        Arc::clone(runtime)
    };
    let (epoch, cancelled, runtime) =
        claim_runtime_for_poll(&registry).expect("claim panicked poll owner");
    let owner_ticket = active_owner_ticket(&registry);
    assert!(Arc::ptr_eq(&runtime_identity, &runtime));
    let operation_entered = Arc::new(AtomicBool::new(false));
    let operation_release = Arc::new(AtomicBool::new(false));
    let owner_registry = Arc::clone(&registry);
    let owner_entered = Arc::clone(&operation_entered);
    let owner_release = Arc::clone(&operation_release);
    let owner = thread::spawn(move || {
        execute_runtime_operation(
            &owner_registry,
            epoch,
            owner_ticket,
            cancelled,
            runtime,
            move |_| -> Result<(), ScanError> {
                owner_entered.store(true, Ordering::Release);
                while !owner_release.load(Ordering::Acquire) {
                    thread::yield_now();
                }
                panic!("secret poll panic after worker creation")
            },
        )
    });
    wait_for_test_signal(&operation_entered);

    let public_deadline = Instant::now() + Duration::from_millis(15);
    let stop_error = stop_runtime_until(&registry, public_deadline)
        .expect_err("the public deadline expires while the poll guard owns the runtime");
    assert_eq!(stop_error.code, "library_synchronization_stop_timeout");
    {
        let state = registry.state.lock().expect("local registry");
        let SynchronizationRuntimeEntry::Draining {
            epoch: draining_epoch,
            deadline,
            runtime,
            ..
        } = &state.entry
        else {
            panic!("stop must retain the complete polling runtime in Draining")
        };
        assert_eq!(*draining_epoch, epoch);
        assert_eq!(*deadline, public_deadline);
        assert!(Arc::ptr_eq(runtime, &runtime_identity));
    }

    operation_release.store(true, Ordering::Release);
    let owner_error = owner
        .join()
        .expect("join panicked poll owner boundary")
        .expect_err("a panicked poll owner remains a structured failure");
    assert_eq!(owner_error.code, "library_synchronization_owner_panicked");
    assert!(!owner_error.message.contains("secret poll panic"));
    assert!(!runtime_identity.is_poisoned());
    {
        let runtime = runtime_identity.lock().expect("retained runtime");
        assert!(
            runtime
                .live
                .as_ref()
                .and_then(|task| task.progress().1)
                .is_some()
        );
        assert!(
            runtime
                .journal
                .as_ref()
                .and_then(|task| task.progress().1)
                .is_some()
        );
        assert!(
            runtime
                .recovery
                .as_ref()
                .and_then(|task| task.worker.as_ref())
                .is_some()
        );
    }
    assert_eq!(retained_stop_deadline(&registry), Some(public_deadline));
    assert_start_is_blocked_while_stopping(&registry);

    p0_release.store(true, Ordering::Release);
    p1_release.store(true, Ordering::Release);
    p2_release.store(true, Ordering::Release);
    wait_for_test_signal(&p0_finished);
    wait_for_test_signal(&p1_finished);
    wait_for_test_signal(&p2_finished);
    let lane_worker_deadline = Instant::now() + Duration::from_secs(1);
    loop {
        let all_finished = {
            let runtime = runtime_identity.lock().expect("retained lane runtime");
            runtime
                .live
                .as_ref()
                .is_some_and(|task| task.progress().1 == Some(true))
                && runtime
                    .journal
                    .as_ref()
                    .is_some_and(|task| task.progress().1 == Some(true))
                && runtime
                    .recovery
                    .as_ref()
                    .and_then(|task| task.worker.as_ref())
                    .is_some_and(JoinHandle::is_finished)
        };
        if all_finished {
            break;
        }
        assert!(
            Instant::now() < lane_worker_deadline,
            "retained lane workers did not finish"
        );
        thread::yield_now();
    }
    let close_error = stop_runtime_until(&registry, public_deadline + Duration::from_secs(1))
        .expect_err("the expired first deadline must remain authoritative at journal close");
    assert_eq!(close_error.code, "library_synchronization_stop_timeout");
    wait_for_test_signal(&close_entered);
    assert_eq!(retained_stop_deadline(&registry), Some(public_deadline));
    assert_eq!(close_count.load(Ordering::Acquire), 1);
    {
        let runtime = runtime_identity.lock().expect("retained close runtime");
        assert!(runtime.live.is_none());
        assert!(runtime.journal.is_none());
        assert!(runtime.recovery.is_none());
        assert!(runtime.journal_close.is_some());
    }

    close_release.store(true, Ordering::Release);
    wait_for_test_signal(&close_finished);
    let close_worker_deadline = Instant::now() + Duration::from_secs(1);
    loop {
        let finished = runtime_identity
            .lock()
            .expect("retained close runtime")
            .journal_close
            .as_ref()
            .and_then(|task| task.worker.as_ref())
            .is_some_and(JoinHandle::is_finished);
        if finished {
            break;
        }
        assert!(
            Instant::now() < close_worker_deadline,
            "retained close worker did not finish"
        );
        thread::yield_now();
    }
    stop_runtime_until(&registry, public_deadline + Duration::from_secs(2))
        .expect("retry reaps every retained owner under the original deadline");
    assert_eq!(close_count.load(Ordering::Acquire), 1);
    assert!(matches!(
        registry.state.lock().expect("local registry").entry,
        SynchronizationRuntimeEntry::Empty
    ));
    let SynchronizationRuntimeClaim::Construct {
        epoch: next_epoch, ..
    } = claim_runtime_for_start(&registry).expect("full panic drain admits the next epoch")
    else {
        panic!("the next epoch must wait for the full drain")
    };
    assert_eq!(next_epoch, 2);
}

#[test]
fn one_shot_drain_panics_retain_the_same_owner_and_retry_every_cleanup_boundary() {
    for point in [
        DrainPanicPoint::RequestStop,
        DrainPanicPoint::P0Result,
        DrainPanicPoint::P1Result,
        DrainPanicPoint::P2Result,
        DrainPanicPoint::JournalCloseInstalled,
        DrainPanicPoint::JournalCloseResult,
    ] {
        let close_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let registry = ready_test_registry(runtime_with_drain_panic(
            point,
            Some(1),
            Arc::clone(&close_count),
        ));
        let runtime_identity = {
            let state = registry.state.lock().expect("local registry");
            let SynchronizationRuntimeEntry::Ready { runtime, .. } = &state.entry else {
                panic!("test runtime must begin ready")
            };
            Arc::clone(runtime)
        };
        let first_deadline = Instant::now() + Duration::from_millis(500);

        let error = stop_runtime_until(&registry, first_deadline)
            .expect_err("the injected drain panic must remain structured");
        assert_eq!(error.code, "library_synchronization_owner_panicked");
        assert_eq!(
            error.message,
            "The library synchronization owner panicked and entered bounded recovery"
        );
        assert!(!runtime_identity.is_poisoned());
        {
            let state = registry.state.lock().expect("local registry");
            let SynchronizationRuntimeEntry::Draining {
                epoch,
                deadline,
                runtime,
                ..
            } = &state.entry
            else {
                panic!("a drain panic must fail closed in Draining")
            };
            assert_eq!(*epoch, 1, "unexpected epoch for {point:?}");
            assert_eq!(
                *deadline, first_deadline,
                "refreshed deadline for {point:?}"
            );
            assert!(Arc::ptr_eq(runtime, &runtime_identity));
        }
        assert_start_is_blocked_while_stopping(&registry);

        stop_runtime_until(&registry, first_deadline + Duration::from_secs(1))
            .unwrap_or_else(|error| panic!("one-shot {point:?} did not retry: {error:?}"));
        assert_eq!(close_count.load(Ordering::Acquire), 1, "{point:?}");
        assert!(matches!(
            registry.state.lock().expect("local registry").entry,
            SynchronizationRuntimeEntry::Empty
        ));
        let SynchronizationRuntimeClaim::Construct { epoch, .. } =
            claim_runtime_for_start(&registry)
                .unwrap_or_else(|error| panic!("{point:?} did not permit restart: {error:?}"))
        else {
            panic!("one-shot {point:?} must admit a fresh construction epoch")
        };
        assert_eq!(epoch, 2);
    }
}

#[test]
fn persistent_drain_panic_fails_closed_without_reclosing_or_admitting_aba() {
    let close_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let registry = ready_test_registry(runtime_with_drain_panic(
        DrainPanicPoint::JournalCloseResult,
        None,
        Arc::clone(&close_count),
    ));
    let runtime_identity = {
        let state = registry.state.lock().expect("local registry");
        let SynchronizationRuntimeEntry::Ready { runtime, .. } = &state.entry else {
            panic!("test runtime must begin ready")
        };
        Arc::clone(runtime)
    };
    let first_deadline = Instant::now() + Duration::from_millis(500);

    for requested_deadline in [first_deadline, first_deadline + Duration::from_secs(1)] {
        let error = stop_runtime_until(&registry, requested_deadline)
            .expect_err("a persistent drain panic must remain fail closed");
        assert_eq!(error.code, "library_synchronization_owner_panicked");
        assert!(!runtime_identity.is_poisoned());
        assert_eq!(retained_stop_deadline(&registry), Some(first_deadline));
        let state = registry.state.lock().expect("local registry");
        let SynchronizationRuntimeEntry::Draining { runtime, .. } = &state.entry else {
            panic!("persistent panic must never publish Empty")
        };
        assert!(Arc::ptr_eq(runtime, &runtime_identity));
        assert_eq!(close_count.load(Ordering::Acquire), 1);
    }
    assert_start_is_blocked_while_stopping(&registry);
    {
        let mut runtime = runtime_identity
            .lock()
            .expect("unpoisoned persistent owner");
        assert!(runtime.core_stopped);
        assert!(!runtime.journal_closed);
        let close = runtime
            .journal_close
            .as_ref()
            .expect("the close result remains owned for retry");
        assert!(close.worker.is_none());
        assert!(close.outcome.is_some());
        runtime.clear_drain_panic();
    }

    stop_runtime_until(&registry, first_deadline + Duration::from_secs(2))
        .expect("clearing the persistent injection allows the same owner to finish");
    assert_eq!(close_count.load(Ordering::Acquire), 1);
    assert!(matches!(
        registry.state.lock().expect("local registry").entry,
        SynchronizationRuntimeEntry::Empty
    ));
    let SynchronizationRuntimeClaim::Construct { epoch, .. } =
        claim_runtime_for_start(&registry).expect("complete persistent recovery admits restart")
    else {
        panic!("persistent recovery must not admit an ABA epoch before cleanup")
    };
    assert_eq!(epoch, 2);
}

#[test]
fn stale_drainer_rechecks_registry_after_runtime_lock_before_cleanup() {
    let close_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let old_runtime = Arc::new(Mutex::new(runtime_with_close_probe(Arc::clone(
        &close_count,
    ))));
    let deadline = Instant::now() + Duration::from_millis(500);
    let old_owner_ticket = RuntimeStartTicket::from_ordinal(LifecycleOrdinal(1));
    let old_stop_fence = RuntimeStopFence::from_ordinal(LifecycleOrdinal(2));
    let registry = Arc::new(SynchronizationRuntimeRegistry {
        state: Mutex::new(SynchronizationRuntimeState {
            next_epoch: 1,
            next_lifecycle_ordinal: old_stop_fence.ordinal(),
            cancelled_through_ordinal: old_stop_fence.ordinal(),
            admitted_stop_fences: VecDeque::new(),
            retired_through_epoch: 0,
            entry: SynchronizationRuntimeEntry::Draining {
                epoch: 1,
                owner_ticket: old_owner_ticket,
                cancellation_fence: Some(old_stop_fence),
                deadline,
                runtime: Arc::clone(&old_runtime),
            },
        }),
        changed: Condvar::new(),
        panic_retention_barrier: Mutex::new(None),
    });
    let old_guard = old_runtime.lock().expect("hold old runtime owner");
    let stale_registry = Arc::clone(&registry);
    let stale_runtime = Arc::clone(&old_runtime);
    let stale = thread::spawn(move || {
        finish_draining_runtime_after_precheck(
            &stale_registry,
            1,
            old_owner_ticket,
            deadline,
            &stale_runtime,
        )
    });

    let mut replacement =
        runtime_with_close_probe(Arc::new(std::sync::atomic::AtomicUsize::new(0)));
    let replacement_cancelled = Arc::new(AtomicBool::new(false));
    replacement.stop_requested = Arc::clone(&replacement_cancelled);
    let replacement_runtime = Arc::new(Mutex::new(replacement));
    let replacement_owner_ticket = RuntimeStartTicket::from_ordinal(LifecycleOrdinal(3));
    {
        let mut state = registry.state.lock().expect("local registry");
        state.next_epoch = 2;
        state.next_lifecycle_ordinal = replacement_owner_ticket.ordinal();
        state.entry = SynchronizationRuntimeEntry::Ready {
            epoch: 2,
            owner_ticket: replacement_owner_ticket,
            cancelled: replacement_cancelled,
            runtime: Arc::clone(&replacement_runtime),
        };
    }
    drop(old_guard);
    stale
        .join()
        .expect("join stale drainer")
        .expect("a stale drainer exits without touching another epoch");

    assert_eq!(close_count.load(Ordering::Acquire), 0);
    {
        let state = registry.state.lock().expect("local registry");
        let SynchronizationRuntimeEntry::Ready { epoch, runtime, .. } = &state.entry else {
            panic!("stale drain must preserve the replacement epoch")
        };
        assert_eq!(*epoch, 2);
        assert!(Arc::ptr_eq(runtime, &replacement_runtime));
    }
    old_runtime
        .lock()
        .expect("old runtime remains unpoisoned")
        .stop()
        .expect("clean up the isolated old runtime");
    assert_eq!(close_count.load(Ordering::Acquire), 1);
}
