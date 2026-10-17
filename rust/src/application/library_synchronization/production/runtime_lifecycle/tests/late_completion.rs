use super::*;

#[test]
fn late_poll_after_public_stop_deadline_retains_original_epoch_drain() {
    let close_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let close_entered = Arc::new(AtomicBool::new(false));
    let close_release = Arc::new(AtomicBool::new(false));
    let close_finished = Arc::new(AtomicBool::new(false));
    let runtime = runtime_with_blocking_close(
        Arc::clone(&close_count),
        Arc::clone(&close_entered),
        Arc::clone(&close_release),
        Arc::clone(&close_finished),
    );
    let registry = ready_test_registry(runtime);
    let (epoch, cancelled, runtime) =
        claim_runtime_for_poll(&registry).expect("claim late poll owner");
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
            move |_| {
                owner_entered.store(true, Ordering::Release);
                while !owner_release.load(Ordering::Acquire) {
                    thread::yield_now();
                }
                Ok(())
            },
        )
    });
    wait_for_test_signal(&operation_entered);

    let public_deadline = Instant::now() + Duration::from_millis(25);
    let stopped = stop_runtime_until(&registry, public_deadline)
        .expect_err("the public stop deadline must expire while poll owns the runtime");
    assert_eq!(stopped.code, "library_synchronization_stop_timeout");
    assert_eq!(retained_stop_deadline(&registry), Some(public_deadline));
    let repeated = stop_runtime_until(&registry, public_deadline + Duration::from_secs(1))
        .expect_err("a repeated stop must not refresh the first public deadline");
    assert_eq!(repeated.code, "library_synchronization_stop_timeout");
    assert_eq!(retained_stop_deadline(&registry), Some(public_deadline));
    assert_start_is_blocked_while_stopping(&registry);

    operation_release.store(true, Ordering::Release);
    wait_for_test_signal(&close_entered);
    let retained_before_release = wait_for_draining_state(&registry, Duration::from_millis(50));
    assert_eq!(retained_stop_deadline(&registry), Some(public_deadline));
    close_release.store(true, Ordering::Release);
    wait_for_test_signal(&close_finished);
    let late_result = owner.join().expect("join late poll owner");

    assert!(
        retained_before_release,
        "a late poll runtime must enter Draining under the already exhausted public deadline"
    );
    assert_eq!(
        late_result
            .expect_err("the stopped late poll cannot publish")
            .code,
        "library_synchronization_stop_timeout"
    );
    assert_eq!(close_count.load(Ordering::Acquire), 1);
    assert_start_is_blocked_while_stopping(&registry);
    wait_for_retained_journal_close_worker(&registry, Duration::from_secs(2));

    stop_runtime(&registry).expect("retry reaps the original completed close task");
    assert_eq!(close_count.load(Ordering::Acquire), 1);
    assert!(matches!(
        registry.state.lock().expect("local registry").entry,
        SynchronizationRuntimeEntry::Empty
    ));
    let SynchronizationRuntimeClaim::Construct { .. } =
        claim_runtime_for_start(&registry).expect("completed drain permits restart")
    else {
        panic!("an empty registry must admit the next epoch");
    };
}

#[test]
fn late_start_after_public_stop_deadline_retains_original_epoch_drain() {
    let close_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let close_entered = Arc::new(AtomicBool::new(false));
    let close_release = Arc::new(AtomicBool::new(false));
    let close_finished = Arc::new(AtomicBool::new(false));
    let registry = Arc::new(SynchronizationRuntimeRegistry::default());
    let SynchronizationRuntimeClaim::Construct { epoch, cancelled } =
        claim_runtime_for_start(&registry).expect("claim construction epoch")
    else {
        panic!("an empty registry must admit construction");
    };
    let owner_ticket = active_owner_ticket(&registry);

    let public_deadline = Instant::now() + Duration::from_millis(25);
    let stopped = stop_runtime_until(&registry, public_deadline)
        .expect_err("the public stop deadline must expire while construction is outside");
    assert_eq!(stopped.code, "library_synchronization_stop_timeout");
    assert_eq!(retained_stop_deadline(&registry), Some(public_deadline));
    let repeated = stop_runtime_until(&registry, public_deadline + Duration::from_secs(1))
        .expect_err("a repeated stop must not refresh the first public deadline");
    assert_eq!(repeated.code, "library_synchronization_stop_timeout");
    assert_eq!(retained_stop_deadline(&registry), Some(public_deadline));
    assert!(cancelled.load(Ordering::Acquire));
    let runtime = runtime_with_blocking_close(
        Arc::clone(&close_count),
        Arc::clone(&close_entered),
        Arc::clone(&close_release),
        Arc::clone(&close_finished),
    );
    let (retained_deadline, runtime) = match promote_constructed_runtime(
        &registry,
        epoch,
        owner_ticket,
        Arc::clone(&cancelled),
        runtime,
    )
    .expect("late construction retains its stopping owner")
    {
        ConstructedRuntimeDisposition::Drain { deadline, runtime } => (deadline, runtime),
        ConstructedRuntimeDisposition::Poll(_) => {
            panic!("a stopped construction epoch must drain rather than poll")
        }
    };
    assert_eq!(retained_deadline, public_deadline);
    let owner_registry = Arc::clone(&registry);
    let owner = thread::spawn(move || {
        finish_draining_runtime(
            &owner_registry,
            epoch,
            owner_ticket,
            retained_deadline,
            &runtime,
        )
    });
    wait_for_test_signal(&close_entered);
    let retained_before_release = wait_for_draining_state(&registry, Duration::from_millis(50));
    assert_eq!(retained_stop_deadline(&registry), Some(public_deadline));
    close_release.store(true, Ordering::Release);
    wait_for_test_signal(&close_finished);
    let late_result = owner.join().expect("join late construction owner");

    assert!(
        retained_before_release,
        "a late constructed runtime must enter Draining under the exhausted public deadline"
    );
    assert_eq!(
        late_result
            .expect_err("late construction drain must retain the timeout")
            .code,
        "library_synchronization_stop_timeout"
    );
    assert_eq!(close_count.load(Ordering::Acquire), 1);
    assert_start_is_blocked_while_stopping(&registry);
    wait_for_retained_journal_close_worker(&registry, Duration::from_secs(2));

    stop_runtime(&registry).expect("retry reaps the original late-construction close task");
    assert_eq!(close_count.load(Ordering::Acquire), 1);
    assert!(matches!(
        registry.state.lock().expect("local registry").entry,
        SynchronizationRuntimeEntry::Empty
    ));
    let SynchronizationRuntimeClaim::Construct { .. } =
        claim_runtime_for_start(&registry).expect("completed drain permits restart")
    else {
        panic!("an empty registry must admit the next epoch");
    };
}

#[test]
fn blocked_broker_connect_cannot_hold_the_registry_or_install_a_late_session() {
    let connect_entered = Arc::new(AtomicBool::new(false));
    let connect_release = Arc::new(AtomicBool::new(false));
    let connect_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let close_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let runtime = new_production_synchronization_with_factory(
        crate::ports::erase_library_change_source_factory(HealthyFactory),
        Arc::new(BlockingJournalFactory {
            connect_entered: Arc::clone(&connect_entered),
            connect_release: Arc::clone(&connect_release),
            connect_count: Arc::clone(&connect_count),
            close_count: Arc::clone(&close_count),
        }),
    );
    let registry = ready_test_registry(runtime);
    let (epoch, cancelled, runtime) =
        claim_runtime_for_poll(&registry).expect("claim connect poll owner");
    let owner_ticket = active_owner_ticket(&registry);
    let owner_registry = Arc::clone(&registry);
    let owner = thread::spawn(move || {
        execute_runtime_operation(
            &owner_registry,
            epoch,
            owner_ticket,
            cancelled,
            runtime,
            |runtime| {
                runtime.open_persistent_change_journal_after_watcher();
                Ok(())
            },
        )
    });
    wait_for_test_signal(&connect_entered);

    let stop_registry = Arc::clone(&registry);
    let (stop_sender, stop_receiver) = mpsc::sync_channel(1);
    let stopper = thread::spawn(move || {
        let _ = stop_sender.send(stop_runtime(&stop_registry));
    });
    assert!(matches!(
        stop_receiver.recv_timeout(Duration::from_millis(50)),
        Err(RecvTimeoutError::Timeout)
    ));
    assert_start_is_blocked_while_stopping(&registry);
    assert_eq!(connect_count.load(Ordering::Acquire), 1);

    connect_release.store(true, Ordering::Release);
    stop_receiver
        .recv_timeout(Duration::from_millis(500))
        .expect("single stop must finish after broker connect releases")
        .expect("single stop rejects and drains the late session");
    stopper.join().expect("join connect stop caller");
    let error = owner
        .join()
        .expect("join blocked connect owner")
        .expect_err("late broker connection must not install");
    assert_eq!(error.code, "library_synchronization_poll_cancelled");
    assert!(matches!(
        registry.state.lock().expect("local registry").entry,
        SynchronizationRuntimeEntry::Empty
    ));
    assert_eq!(connect_count.load(Ordering::Acquire), 1);
    assert_eq!(close_count.load(Ordering::Acquire), 1);
}

#[test]
fn timed_out_p1_then_journal_close_share_one_epoch_deadline_and_reap_once() {
    let close_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let close_entered = Arc::new(AtomicBool::new(false));
    let close_release = Arc::new(AtomicBool::new(false));
    let close_finished = Arc::new(AtomicBool::new(false));
    let mut runtime = new_production_synchronization_with_connection(
        crate::ports::erase_library_change_source_factory(HealthyFactory),
        PersistentChangeJournalConnection::Connected(Arc::new(BlockingCloseSession {
            close_count: Arc::clone(&close_count),
            close_entered: Arc::clone(&close_entered),
            close_release: Arc::clone(&close_release),
            close_finished: Arc::clone(&close_finished),
            panic_after_release: false,
        })),
    );
    let worker_cancelled = Arc::new(AtomicBool::new(false));
    let worker_release = Arc::new(AtomicBool::new(false));
    let completed_sibling = Arc::new(AtomicBool::new(false));
    let release = Arc::clone(&worker_release);
    let sibling = Arc::clone(&completed_sibling);
    let (sender, receiver) = mpsc::sync_channel(1);
    let worker = thread::spawn(move || {
        while !release.load(Ordering::Acquire) {
            std::thread::yield_now();
        }
        sibling.store(true, Ordering::Release);
        let _ = sender.send(Ok(JournalTaskOutcome::Page(
            PersistentJournalEnrollmentReport {
                enrolled_root_count: 1,
                ..PersistentJournalEnrollmentReport::default()
            }
            .into(),
        )));
    });
    runtime.journal = Some(JournalTask::from_test_parts(
        Arc::clone(&worker_cancelled),
        receiver,
        worker,
    ));
    let registry = ready_test_registry(runtime);

    let error = stop_runtime_with_timeout(&registry, Duration::from_millis(25))
        .expect_err("uncooperative worker must produce a structured timeout");
    assert_eq!(error.code, "persistent_journal_stop_timeout");
    assert!(worker_cancelled.load(Ordering::Acquire));
    assert!(matches!(
        registry.state.lock().expect("local registry").entry,
        SynchronizationRuntimeEntry::Draining { .. }
    ));
    assert_start_is_blocked_while_stopping(&registry);
    assert_eq!(close_count.load(Ordering::Acquire), 0);

    worker_release.store(true, Ordering::Release);
    let worker_finished_deadline = Instant::now() + Duration::from_secs(2);
    loop {
        let is_finished = {
            let state = registry.state.lock().expect("local registry");
            match &state.entry {
                SynchronizationRuntimeEntry::Draining { runtime, .. } => runtime
                    .lock()
                    .expect("draining runtime")
                    .journal
                    .as_ref()
                    .is_some_and(|task| task.progress().1 == Some(true)),
                _ => false,
            }
        };
        if is_finished {
            break;
        }
        assert!(
            Instant::now() < worker_finished_deadline,
            "the retained P1 worker did not finish after release"
        );
        thread::yield_now();
    }

    let exhausted = stop_runtime(&registry)
        .expect_err("journal close must not receive a fresh deadline after P1 exhaustion");
    assert_eq!(exhausted.code, "library_synchronization_stop_timeout");
    wait_for_test_signal(&close_entered);
    assert!(close_entered.load(Ordering::Acquire));
    assert_eq!(close_count.load(Ordering::Acquire), 1);
    assert!(matches!(
        registry.state.lock().expect("local registry").entry,
        SynchronizationRuntimeEntry::Draining { .. }
    ));

    close_release.store(true, Ordering::Release);
    wait_for_test_signal(&close_finished);
    let close_worker_finished_deadline = Instant::now() + Duration::from_secs(2);
    loop {
        let is_finished = {
            let state = registry.state.lock().expect("local registry");
            match &state.entry {
                SynchronizationRuntimeEntry::Draining { runtime, .. } => runtime
                    .lock()
                    .expect("draining runtime")
                    .journal_close
                    .as_ref()
                    .and_then(|task| task.worker.as_ref())
                    .is_some_and(JoinHandle::is_finished),
                _ => false,
            }
        };
        if is_finished {
            break;
        }
        assert!(
            Instant::now() < close_worker_finished_deadline,
            "the retained journal-close worker did not finish after release"
        );
        thread::yield_now();
    }

    stop_runtime(&registry).expect("retry joins the same completed close task");
    assert!(matches!(
        registry.state.lock().expect("local registry").entry,
        SynchronizationRuntimeEntry::Empty
    ));
    assert_eq!(close_count.load(Ordering::Acquire), 1);
    assert!(completed_sibling.load(Ordering::Acquire));
}
