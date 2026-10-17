use super::*;

#[test]
fn first_import_preflight_waits_until_the_runtime_is_ready_to_poll() {
    let empty_registry = SynchronizationRuntimeRegistry::default();
    assert_eq!(
        first_import_runtime_owner_for_poll(&empty_registry).expect("inspect an empty runtime"),
        None
    );

    let starting_registry = SynchronizationRuntimeRegistry::default();
    let starting_owner = reserve_runtime_start_ticket(&starting_registry)
        .expect("reserve the starting runtime owner");
    let SynchronizationRuntimeClaim::Construct { .. } =
        claim_runtime_for_start_with_ticket(&starting_registry, starting_owner)
            .expect("claim the starting runtime")
    else {
        panic!("an empty runtime must enter Starting")
    };
    assert_eq!(
        first_import_runtime_owner_for_poll(&starting_registry)
            .expect("inspect a starting runtime"),
        None
    );

    let close_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let ready_registry = ready_test_registry(runtime_with_close_probe(close_count));
    let ready_owner = active_owner_ticket(&ready_registry);
    assert_eq!(
        first_import_runtime_owner_for_poll(&ready_registry).expect("inspect a ready runtime"),
        Some(ready_owner)
    );

    let _poll_claim = claim_runtime_for_poll_with_ticket(&ready_registry, ready_owner)
        .expect("claim the ready runtime for polling");
    assert_eq!(
        first_import_runtime_owner_for_poll(&ready_registry).expect("inspect a polling runtime"),
        None
    );
}

#[test]
fn first_import_preflight_rejects_stopping_and_draining_runtimes() {
    let stopping_registry = SynchronizationRuntimeRegistry::default();
    let stopping_owner = reserve_runtime_start_ticket(&stopping_registry)
        .expect("reserve the stopping runtime owner");
    let SynchronizationRuntimeClaim::Construct { .. } =
        claim_runtime_for_start_with_ticket(&stopping_registry, stopping_owner)
            .expect("claim the runtime before stopping")
    else {
        panic!("an empty runtime must enter Starting")
    };
    reserve_runtime_stop_fence(&stopping_registry, Duration::from_secs(1))
        .expect("move the starting runtime to Stopping");
    let stopping_error = first_import_runtime_owner_for_poll(&stopping_registry)
        .expect_err("Stopping must terminate first-import preflight");
    assert_eq!(
        stopping_error.code,
        "library_synchronization_stop_in_progress"
    );

    let close_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let draining_registry = ready_test_registry(runtime_with_close_probe(close_count));
    reserve_runtime_stop_fence(&draining_registry, Duration::from_secs(1))
        .expect("move the ready runtime to Draining");
    let draining_error = first_import_runtime_owner_for_poll(&draining_registry)
        .expect_err("Draining must terminate first-import preflight");
    assert_eq!(
        draining_error.code,
        "library_synchronization_stop_in_progress"
    );
}

#[test]
fn stop_before_delayed_start_admission_prevents_a_ghost_runtime() {
    let registry = Arc::new(SynchronizationRuntimeRegistry::default());
    let owner_ticket =
        reserve_runtime_start_ticket(&registry).expect("reserve the pre-stop owner ticket");
    let release_start = Arc::new(std::sync::Barrier::new(2));
    let constructor_calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let start_registry = Arc::clone(&registry);
    let start_release = Arc::clone(&release_start);
    let calls = Arc::clone(&constructor_calls);

    let start = thread::spawn(move || {
        start_release.wait();
        start_runtime_with_ticket(
            &start_registry,
            owner_ticket,
            || -> ProductionSynchronization {
                calls.fetch_add(1, Ordering::AcqRel);
                panic!("a cancelled request must not reach construction")
            },
        )
    });

    stop_runtime(&registry).expect("an empty registry still accepts the stop boundary");
    release_start.wait();
    let error = start
        .join()
        .expect("delayed start thread")
        .expect_err("the pre-stop request must be rejected");

    assert_eq!(error.code, "library_synchronization_start_cancelled");
    assert_eq!(constructor_calls.load(Ordering::Acquire), 0);
    assert!(matches!(
        registry.state.lock().expect("local registry").entry,
        SynchronizationRuntimeEntry::Empty
    ));
}

#[test]
fn old_poll_ticket_cannot_attach_to_a_replacement_epoch() {
    let close_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let registry = ready_test_registry(runtime_with_close_probe(Arc::clone(&close_count)));
    let old_owner_ticket = active_owner_ticket(&registry);

    stop_runtime(&registry).expect("stop the original owner");
    let new_owner_ticket =
        reserve_runtime_start_ticket(&registry).expect("reserve replacement owner ticket");
    let SynchronizationRuntimeClaim::Construct { epoch, cancelled } =
        claim_runtime_for_start_with_ticket(&registry, new_owner_ticket)
            .expect("admit the replacement owner")
    else {
        panic!("a replacement owner must construct a new epoch")
    };
    let replacement = runtime_with_close_probe(Arc::clone(&close_count));
    let replacement = match promote_constructed_runtime(
        &registry,
        epoch,
        new_owner_ticket,
        Arc::clone(&cancelled),
        replacement,
    )
    .expect("promote the replacement owner")
    {
        ConstructedRuntimeDisposition::Poll(runtime) => runtime,
        ConstructedRuntimeDisposition::Drain { .. } => {
            panic!("a newly admitted owner must not inherit the old stop fence")
        }
    };
    execute_runtime_operation(
        &registry,
        epoch,
        new_owner_ticket,
        cancelled,
        Arc::clone(&replacement),
        |_| Ok(()),
    )
    .expect("publish the replacement owner as ready");

    let error = match claim_runtime_for_poll_with_ticket(&registry, old_owner_ticket) {
        Err(error) => error,
        Ok(_) => panic!("an old poll ticket must not claim the replacement runtime"),
    };

    assert_eq!(error.code, "library_synchronization_lifecycle_ticket_stale");
    let state = registry.state.lock().expect("local registry");
    let SynchronizationRuntimeEntry::Ready {
        epoch: active_epoch,
        owner_ticket,
        runtime,
        ..
    } = &state.entry
    else {
        panic!("the rejected old poll must leave the replacement ready")
    };
    assert_eq!(*active_epoch, epoch);
    assert_eq!(*owner_ticket, new_owner_ticket);
    assert!(Arc::ptr_eq(runtime, &replacement));
}

#[test]
fn same_start_ticket_can_retry_but_a_different_ticket_cannot_attach() {
    let registry = SynchronizationRuntimeRegistry::default();
    let owner_ticket =
        reserve_runtime_start_ticket(&registry).expect("reserve the original owner ticket");
    let SynchronizationRuntimeClaim::Construct { epoch, .. } =
        claim_runtime_for_start_with_ticket(&registry, owner_ticket)
            .expect("admit the original owner")
    else {
        panic!("the original owner must construct")
    };
    recover_starting_runtime_panic(&registry, epoch, owner_ticket)
        .expect("recover the failed construction");

    let SynchronizationRuntimeClaim::Construct {
        epoch: retry_epoch, ..
    } = claim_runtime_for_start_with_ticket(&registry, owner_ticket)
        .expect("the same bounded request may retry after recovery")
    else {
        panic!("the same owner ticket must construct a replacement epoch")
    };
    assert_eq!(retry_epoch, epoch + 1);

    let different_ticket =
        reserve_runtime_start_ticket(&registry).expect("reserve a different request ticket");
    let error = match claim_runtime_for_start_with_ticket(&registry, different_ticket) {
        Err(error) => error,
        Ok(_) => panic!("a different request must not attach to the starting owner"),
    };
    assert_eq!(error.code, "library_synchronization_start_in_progress");
    assert_eq!(active_owner_ticket(&registry), owner_ticket);
}

#[test]
fn lifecycle_boundary_rejects_zero_and_unissued_start_tickets() {
    for owner_ticket in [
        0,
        RuntimeStartTicket::from_ordinal(LifecycleOrdinal(1)).raw(),
    ] {
        let registry = SynchronizationRuntimeRegistry::default();
        let constructor_calls = std::sync::atomic::AtomicUsize::new(0);

        let error = RuntimeStartTicket::decode(owner_ticket)
            .and_then(|owner_ticket| {
                start_runtime_with_ticket(
                    &registry,
                    owner_ticket,
                    || -> ProductionSynchronization {
                        constructor_calls.fetch_add(1, Ordering::AcqRel);
                        panic!("an invalid ticket must not reach construction")
                    },
                )
            })
            .expect_err("a zero or unissued start ticket must fail closed");

        assert_eq!(
            error.code,
            "library_synchronization_lifecycle_ticket_invalid"
        );
        assert_eq!(constructor_calls.load(Ordering::Acquire), 0);
        let state = registry.state.lock().expect("local registry");
        assert_eq!(state.next_epoch, 0);
        assert_eq!(state.next_lifecycle_ordinal, LifecycleOrdinal(0));
        assert_eq!(state.cancelled_through_ordinal, LifecycleOrdinal(0));
        assert!(matches!(state.entry, SynchronizationRuntimeEntry::Empty));
    }
}

#[test]
fn lifecycle_boundary_rejects_zero_and_unadmitted_stop_fences() {
    let registry = SynchronizationRuntimeRegistry::default();

    for raw in [0, RuntimeStopFence::KIND] {
        let zero_error = RuntimeStopFence::decode(raw)
            .and_then(|fence| stop_runtime_with_fence(&registry, fence))
            .expect_err("a zero-ordinal stop fence must fail closed");
        assert_eq!(
            zero_error.code,
            "library_synchronization_lifecycle_fence_invalid"
        );
    }

    let unadmitted_start =
        reserve_runtime_start_ticket(&registry).expect("reserve a non-fence ticket");
    let unadmitted_raw = RuntimeStopFence::from_ordinal(unadmitted_start.ordinal()).raw();
    let unadmitted_error = RuntimeStopFence::decode(unadmitted_raw)
        .and_then(|fence| stop_runtime_with_fence(&registry, fence))
        .expect_err("a same-kind but unissued stop fence must fail closed");
    assert_eq!(
        unadmitted_error.code,
        "library_synchronization_lifecycle_fence_invalid"
    );

    let state = registry.state.lock().expect("local registry");
    assert_eq!(state.next_epoch, 0);
    assert_eq!(state.next_lifecycle_ordinal, unadmitted_start.ordinal());
    assert_eq!(state.cancelled_through_ordinal, LifecycleOrdinal(0));
    assert!(matches!(state.entry, SynchronizationRuntimeEntry::Empty));
}

#[test]
fn stop_fence_capacity_failure_preserves_all_unconsumed_admission_state() {
    let close_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let registry = ready_test_registry(runtime_with_close_probe(Arc::clone(&close_count)));
    let fences = (0..MAX_ADMITTED_STOP_FENCES)
        .map(|_| {
            reserve_runtime_stop_fence(&registry, Duration::from_secs(1))
                .expect("reserve one bounded unconsumed stop fence")
        })
        .collect::<Vec<_>>();
    let first_fence = fences[0];
    let (next_ordinal, cancelled_through, ledger, owner, deadline, runtime) = {
        let state = registry.state.lock().expect("local registry");
        let SynchronizationRuntimeEntry::Draining {
            cancellation_fence,
            deadline,
            runtime,
            ..
        } = &state.entry
        else {
            panic!("the first fence must linearize the owner into draining")
        };
        assert_eq!(*cancellation_fence, Some(first_fence));
        (
            state.next_lifecycle_ordinal,
            state.cancelled_through_ordinal,
            state.admitted_stop_fences.clone(),
            state.entry.owner_identity(),
            *deadline,
            Arc::clone(runtime),
        )
    };

    let error = reserve_runtime_stop_fence(&registry, Duration::from_secs(1))
        .expect_err("a full unconsumed ledger must reject another stop fence");

    assert_eq!(
        error.code,
        "library_synchronization_lifecycle_fence_capacity_exhausted"
    );
    let state = registry.state.lock().expect("local registry");
    assert_eq!(state.next_lifecycle_ordinal, next_ordinal);
    assert_eq!(state.cancelled_through_ordinal, cancelled_through);
    assert_eq!(state.admitted_stop_fences, ledger);
    assert_eq!(state.entry.owner_identity(), owner);
    let SynchronizationRuntimeEntry::Draining {
        cancellation_fence,
        deadline: active_deadline,
        runtime: active_runtime,
        ..
    } = &state.entry
    else {
        panic!("capacity rejection must preserve the draining owner")
    };
    assert_eq!(*cancellation_fence, Some(first_fence));
    assert_eq!(*active_deadline, deadline);
    assert!(Arc::ptr_eq(active_runtime, &runtime));
    assert!(
        state
            .admitted_stop_fences
            .iter()
            .all(|entry| entry.use_state == AdmittedStopFenceUse::Pending)
    );
    drop(state);

    stop_runtime_with_fence(&registry, first_fence)
        .expect("an existing admitted fence remains usable after capacity rejection");
    assert_eq!(close_count.load(Ordering::Acquire), 1);
}

#[test]
fn consumed_stop_fence_history_is_reclaimed_only_under_capacity_pressure() {
    let registry = SynchronizationRuntimeRegistry::default();
    let fences = (0..MAX_ADMITTED_STOP_FENCES)
        .map(|_| {
            reserve_runtime_stop_fence(&registry, Duration::from_secs(1))
                .expect("fill the bounded stop-fence ledger")
        })
        .collect::<Vec<_>>();
    let consumed = fences[0];
    stop_runtime_with_fence(&registry, consumed).expect("consume the oldest empty fence");
    {
        let state = registry.state.lock().expect("local registry");
        assert_eq!(state.admitted_stop_fences.len(), MAX_ADMITTED_STOP_FENCES);
        assert_eq!(
            state
                .admitted_stop_fences
                .front()
                .map(|entry| entry.use_state),
            Some(AdmittedStopFenceUse::Consumed)
        );
    }

    let replacement = reserve_runtime_stop_fence(&registry, Duration::from_secs(1))
        .expect("capacity pressure may reclaim consumed empty history");
    let state = registry.state.lock().expect("local registry");
    assert_eq!(state.admitted_stop_fences.len(), MAX_ADMITTED_STOP_FENCES);
    assert!(
        !state
            .admitted_stop_fences
            .iter()
            .any(|entry| entry.fence == consumed)
    );
    assert_eq!(
        state.admitted_stop_fences.back().map(|entry| entry.fence),
        Some(replacement)
    );
    drop(state);

    let error = stop_runtime_with_fence(&registry, consumed)
        .expect_err("reclaimed history no longer proves exact issuance");
    assert_eq!(
        error.code,
        "library_synchronization_lifecycle_fence_invalid"
    );
}

#[test]
fn consumed_fence_covering_a_draining_owner_survives_capacity_pressure() {
    let close_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let registry = ready_test_registry(runtime_with_close_probe(Arc::clone(&close_count)));
    let runtime = {
        let state = registry.state.lock().expect("local registry");
        let SynchronizationRuntimeEntry::Ready { runtime, .. } = &state.entry else {
            panic!("test runtime must begin ready")
        };
        Arc::clone(runtime)
    };
    let runtime_guard = runtime.lock().expect("hold the native owner");
    let deadline = Instant::now() + Duration::from_millis(20);
    let first_fence = reserve_runtime_stop_fence_until(&registry, deadline)
        .expect("reserve the first owner-covering fence");
    thread::sleep(Duration::from_millis(30));
    let timeout = stop_runtime_with_fence(&registry, first_fence)
        .expect_err("the held owner must exhaust the first deadline");
    assert_eq!(timeout.code, "library_synchronization_stop_timeout");

    for _ in 1..MAX_ADMITTED_STOP_FENCES {
        reserve_runtime_stop_fence(&registry, Duration::from_secs(1))
            .expect("fill remaining ledger slots without dropping the active fence");
    }
    let capacity_error = reserve_runtime_stop_fence(&registry, Duration::from_secs(1))
        .expect_err("the consumed active-owner fence is not reclaimable");
    assert_eq!(
        capacity_error.code,
        "library_synchronization_lifecycle_fence_capacity_exhausted"
    );
    assert!(
        registry
            .state
            .lock()
            .expect("local registry")
            .admitted_stop_fences
            .iter()
            .any(|entry| {
                entry.fence == first_fence && entry.use_state == AdmittedStopFenceUse::Consumed
            })
    );

    drop(runtime_guard);
    let reap_deadline = Instant::now() + Duration::from_secs(1);
    loop {
        match stop_runtime_with_fence(&registry, first_fence) {
            Ok(()) => break,
            Err(error)
                if error.code == "library_synchronization_stop_timeout"
                    && Instant::now() < reap_deadline =>
            {
                thread::yield_now();
            }
            Err(error) => panic!("the retained fence did not reap its owner: {error:?}"),
        }
    }
    assert_eq!(close_count.load(Ordering::Acquire), 1);
}

#[test]
fn repeated_and_concurrent_calls_share_one_admitted_stop_fence() {
    let close_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let registry = ready_test_registry(runtime_with_close_probe(Arc::clone(&close_count)));
    let fence = reserve_runtime_stop_fence(&registry, Duration::from_secs(1))
        .expect("reserve one shared stop fence");
    let first_registry = Arc::clone(&registry);
    let second_registry = Arc::clone(&registry);

    let first = thread::spawn(move || stop_runtime_with_fence(&first_registry, fence));
    let second = thread::spawn(move || stop_runtime_with_fence(&second_registry, fence));
    first
        .join()
        .expect("join first stop caller")
        .expect("first stop caller succeeds");
    second
        .join()
        .expect("join second stop caller")
        .expect("second stop caller converges");
    stop_runtime_with_fence(&registry, fence).expect("a repeated stop remains idempotent");

    assert_eq!(close_count.load(Ordering::Acquire), 1);
    let state = registry.state.lock().expect("local registry");
    assert!(matches!(state.entry, SynchronizationRuntimeEntry::Empty));
    assert_eq!(
        state
            .admitted_stop_fences
            .iter()
            .find(|entry| entry.fence == fence)
            .map(|entry| entry.use_state),
        Some(AdmittedStopFenceUse::Consumed)
    );
}

#[test]
fn empty_stop_fence_is_a_delayed_noop_for_a_newer_owner() {
    let registry = SynchronizationRuntimeRegistry::default();
    let old_fence = reserve_runtime_stop_fence(&registry, Duration::from_secs(1))
        .expect("reserve an empty-registry stop fence");
    let close_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let owner_ticket =
        reserve_runtime_start_ticket(&registry).expect("reserve a newer start owner");
    let SynchronizationRuntimeClaim::Construct { epoch, cancelled } =
        claim_runtime_for_start_with_ticket(&registry, owner_ticket)
            .expect("admit the newer owner")
    else {
        panic!("the newer owner must construct")
    };
    let runtime = runtime_with_close_probe(Arc::clone(&close_count));
    let runtime = match promote_constructed_runtime(
        &registry,
        epoch,
        owner_ticket,
        Arc::clone(&cancelled),
        runtime,
    )
    .expect("promote the newer owner")
    {
        ConstructedRuntimeDisposition::Poll(runtime) => runtime,
        ConstructedRuntimeDisposition::Drain { .. } => {
            panic!("an old empty fence must not attach to a newer owner")
        }
    };
    execute_runtime_operation(
        &registry,
        epoch,
        owner_ticket,
        cancelled,
        Arc::clone(&runtime),
        |_| Ok(()),
    )
    .expect("publish the newer owner as ready");

    stop_runtime_with_fence(&registry, old_fence)
        .expect("the old empty fence is an idempotent no-op");
    assert_eq!(close_count.load(Ordering::Acquire), 0);
    assert!(matches!(
        registry.state.lock().expect("local registry").entry,
        SynchronizationRuntimeEntry::Ready { .. }
    ));

    stop_runtime(&registry).expect("a fresh fence drains the newer owner");
    assert_eq!(close_count.load(Ordering::Acquire), 1);
}

#[test]
fn lifecycle_boundary_rejects_a_start_ticket_after_a_later_stop_fence_is_admitted() {
    let close_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let registry = ready_test_registry(runtime_with_close_probe(Arc::clone(&close_count)));
    let start_ticket = active_owner_ticket(&registry);
    let stop_fence = reserve_runtime_stop_fence(&registry, Duration::from_secs(1))
        .expect("reserve the matching stop fence");
    let forged_fence = start_ticket.raw() | RuntimeStopFence::KIND;

    let error = RuntimeStopFence::decode(forged_fence)
        .and_then(|fence| stop_runtime_with_fence(&registry, fence))
        .expect_err("changing a start ticket kind bit must not forge an admitted stop fence");

    assert_eq!(
        error.code,
        "library_synchronization_lifecycle_fence_invalid"
    );
    assert_eq!(close_count.load(Ordering::Acquire), 0);
    assert!(matches!(
        registry.state.lock().expect("local registry").entry,
        SynchronizationRuntimeEntry::Draining { .. }
    ));

    stop_runtime_with_fence(&registry, stop_fence)
        .expect("the admitted stop fence must drain its matching owner");
    assert_eq!(close_count.load(Ordering::Acquire), 1);
}

#[test]
fn lifecycle_boundary_epoch_exhaustion_restores_empty_state() {
    let registry = SynchronizationRuntimeRegistry::default();
    let owner_ticket =
        reserve_runtime_start_ticket(&registry).expect("reserve the final start owner");
    {
        let mut state = registry.state.lock().expect("local registry");
        state.next_epoch = u64::MAX;
    }
    let constructor_calls = std::sync::atomic::AtomicUsize::new(0);

    let error =
        start_runtime_with_ticket(&registry, owner_ticket, || -> ProductionSynchronization {
            constructor_calls.fetch_add(1, Ordering::AcqRel);
            panic!("an exhausted epoch must not reach construction")
        })
        .expect_err("an exhausted epoch allocator must fail closed");

    assert_eq!(error.code, "library_synchronization_epoch_exhausted");
    assert_eq!(constructor_calls.load(Ordering::Acquire), 0);
    let state = registry.state.lock().expect("local registry");
    assert_eq!(state.next_epoch, u64::MAX);
    assert_eq!(state.next_lifecycle_ordinal, owner_ticket.ordinal());
    assert_eq!(state.cancelled_through_ordinal, LifecycleOrdinal(0));
    assert!(matches!(state.entry, SynchronizationRuntimeEntry::Empty));
}

#[test]
fn lifecycle_boundary_old_stop_fence_cannot_drain_a_newer_epoch() {
    let close_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let registry = ready_test_registry(runtime_with_close_probe(Arc::clone(&close_count)));
    let old_fence = reserve_runtime_stop_fence(&registry, Duration::from_secs(1))
        .expect("reserve the original owner fence");
    stop_runtime_with_fence(&registry, old_fence).expect("drain the original owner");
    assert_eq!(close_count.load(Ordering::Acquire), 1);

    let new_owner_ticket =
        reserve_runtime_start_ticket(&registry).expect("reserve a newer owner ticket");
    let SynchronizationRuntimeClaim::Construct { epoch, cancelled } =
        claim_runtime_for_start_with_ticket(&registry, new_owner_ticket)
            .expect("admit the newer owner")
    else {
        panic!("the newer owner must construct a new epoch")
    };
    let replacement = runtime_with_close_probe(Arc::clone(&close_count));
    let replacement = match promote_constructed_runtime(
        &registry,
        epoch,
        new_owner_ticket,
        Arc::clone(&cancelled),
        replacement,
    )
    .expect("promote the newer owner")
    {
        ConstructedRuntimeDisposition::Poll(runtime) => runtime,
        ConstructedRuntimeDisposition::Drain { .. } => {
            panic!("the newer owner must not inherit the old fence")
        }
    };
    execute_runtime_operation(
        &registry,
        epoch,
        new_owner_ticket,
        cancelled,
        Arc::clone(&replacement),
        |_| Ok(()),
    )
    .expect("publish the newer owner as ready");

    stop_runtime_with_fence(&registry, old_fence)
        .expect("the stale fence is an idempotent no-op for a newer owner");

    let state = registry.state.lock().expect("local registry");
    let SynchronizationRuntimeEntry::Ready {
        epoch: active_epoch,
        owner_ticket: active_owner_ticket,
        runtime,
        ..
    } = &state.entry
    else {
        panic!("the stale fence must leave the newer owner ready")
    };
    assert_eq!(*active_epoch, epoch);
    assert_eq!(*active_owner_ticket, new_owner_ticket);
    assert!(Arc::ptr_eq(runtime, &replacement));
    assert_eq!(close_count.load(Ordering::Acquire), 1);
    drop(state);

    stop_runtime(&registry).expect("drain the newer owner with a fresh fence");
    assert_eq!(close_count.load(Ordering::Acquire), 2);
}

#[test]
fn lifecycle_ticket_and_stop_fence_overflow_fail_closed() {
    let registry = SynchronizationRuntimeRegistry::default();
    {
        let mut state = registry.state.lock().expect("local registry");
        state.next_lifecycle_ordinal = LifecycleOrdinal(LifecycleOrdinal::MAX);
        state.cancelled_through_ordinal = LifecycleOrdinal(LifecycleOrdinal::MAX - 1);
    }

    let start_error = reserve_runtime_start_ticket(&registry)
        .expect_err("an exhausted ticket allocator must reject start");
    let stop_error = reserve_runtime_stop_fence(&registry, Duration::from_secs(2))
        .expect_err("an exhausted ticket allocator must reject stop");

    assert_eq!(
        start_error.code,
        "library_synchronization_lifecycle_ticket_exhausted"
    );
    assert_eq!(
        stop_error.code,
        "library_synchronization_lifecycle_ticket_exhausted"
    );
    let state = registry.state.lock().expect("local registry");
    assert_eq!(
        state.next_lifecycle_ordinal,
        LifecycleOrdinal(LifecycleOrdinal::MAX)
    );
    assert_eq!(
        state.cancelled_through_ordinal,
        LifecycleOrdinal(LifecycleOrdinal::MAX - 1)
    );
    assert!(matches!(state.entry, SynchronizationRuntimeEntry::Empty));
}

#[test]
fn delayed_stop_task_uses_the_deadline_reserved_by_the_sync_fence() {
    let close_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let registry = ready_test_registry(runtime_with_close_probe(Arc::clone(&close_count)));
    let runtime = {
        let state = registry.state.lock().expect("local registry");
        let SynchronizationRuntimeEntry::Ready { runtime, .. } = &state.entry else {
            panic!("test runtime must begin ready")
        };
        Arc::clone(runtime)
    };
    let runtime_guard = runtime.lock().expect("hold the native owner");
    let deadline = Instant::now() + Duration::from_millis(20);
    let cancellation_fence = reserve_runtime_stop_fence_until(&registry, deadline)
        .expect("the public stop call reserves its fence and deadline");
    thread::sleep(Duration::from_millis(30));

    let started = Instant::now();
    let error = stop_runtime_with_fence(&registry, cancellation_fence)
        .expect_err("a delayed async stop task must not receive a fresh deadline");

    assert_eq!(error.code, "library_synchronization_stop_timeout");
    assert!(started.elapsed() < Duration::from_millis(20));
    assert_eq!(retained_stop_deadline(&registry), Some(deadline));
    drop(runtime_guard);
    let reap_deadline = Instant::now() + Duration::from_secs(1);
    loop {
        match stop_runtime_with_fence(&registry, cancellation_fence) {
            Ok(()) => break,
            Err(error)
                if error.code == "library_synchronization_stop_timeout"
                    && Instant::now() < reap_deadline =>
            {
                thread::yield_now();
            }
            Err(error) => panic!("the retained fence did not reap its owner: {error:?}"),
        }
    }
    assert_eq!(close_count.load(Ordering::Acquire), 1);
}
