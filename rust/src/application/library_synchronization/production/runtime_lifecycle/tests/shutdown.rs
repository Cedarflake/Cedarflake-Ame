use super::*;

#[test]
fn production_stop_closes_the_retained_persistent_journal_session() {
    let close_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let mut production = new_production_synchronization_with_connection(
        crate::ports::erase_library_change_source_factory(HealthyFactory),
        PersistentChangeJournalConnection::Connected(Arc::new(CloseProbeSession {
            close_count: Arc::clone(&close_count),
            close_failures: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        })),
    );

    production.stop().expect("stop production synchronization");

    assert_eq!(close_count.load(Ordering::Acquire), 1);
}

#[test]
fn blocked_journal_close_uses_the_remaining_epoch_deadline_and_reaps_once() {
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
    let live_cancelled = Arc::new(AtomicBool::new(false));
    let worker_cancelled = Arc::clone(&live_cancelled);
    let (sender, receiver) = mpsc::sync_channel(1);
    let worker = thread::spawn(move || {
        while !worker_cancelled.load(Ordering::Acquire) {
            thread::yield_now();
        }
        thread::sleep(Duration::from_millis(20));
        let _ = sender.send(LiveWorkOutcome::default());
    });
    runtime.live = Some(LiveTask::from_test_parts(
        "deadline-root".to_owned(),
        live_cancelled,
        receiver,
        worker,
    ));
    let registry = ready_test_registry(runtime);

    let started = Instant::now();
    let error = stop_runtime_with_timeout(&registry, Duration::from_millis(100))
        .expect_err("a blocked journal close must exhaust the original epoch deadline");
    assert_eq!(error.code, "library_synchronization_stop_timeout");
    assert!(started.elapsed() < Duration::from_millis(500));
    assert!(close_entered.load(Ordering::Acquire));
    assert_eq!(close_count.load(Ordering::Acquire), 1);
    assert!(matches!(
        registry.state.lock().expect("local registry").entry,
        SynchronizationRuntimeEntry::Draining { .. }
    ));
    assert_start_is_blocked_while_stopping(&registry);

    close_release.store(true, Ordering::Release);
    wait_for_test_signal(&close_finished);
    let worker_finished_deadline = Instant::now() + Duration::from_secs(2);
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
            Instant::now() < worker_finished_deadline,
            "the retained journal-close worker did not finish after release"
        );
        thread::yield_now();
    }

    stop_runtime(&registry).expect("retry reaps the same completed journal-close worker");
    assert_eq!(close_count.load(Ordering::Acquire), 1);
    assert!(matches!(
        registry.state.lock().expect("local registry").entry,
        SynchronizationRuntimeEntry::Empty
    ));
    let SynchronizationRuntimeClaim::Construct { .. } =
        claim_runtime_for_start(&registry).expect("a reaped close admits the next epoch")
    else {
        panic!("an empty registry must admit construction");
    };
}

#[test]
fn failed_journal_close_is_cached_and_never_invoked_again() {
    let close_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let mut runtime = new_production_synchronization_with_connection(
        crate::ports::erase_library_change_source_factory(HealthyFactory),
        PersistentChangeJournalConnection::Connected(Arc::new(CloseProbeSession {
            close_count: Arc::clone(&close_count),
            close_failures: Arc::new(std::sync::atomic::AtomicUsize::new(1)),
        })),
    );
    let first = runtime
        .finish_stopping_until(Instant::now() + Duration::from_secs(1))
        .expect_err("the controlled close failure must remain visible");
    assert_eq!(first.code, "persistent_change_journal_close_failed");
    assert_eq!(close_count.load(Ordering::Acquire), 1);
    assert!(runtime.journal_close.is_some());

    let retry = runtime
        .finish_stopping_until(Instant::now() + Duration::from_secs(1))
        .expect_err("retry must project the retained close outcome");
    assert_eq!(retry.code, first.code);
    assert_eq!(retry.message, first.message);
    assert_eq!(close_count.load(Ordering::Acquire), 1);
    assert!(runtime.journal_close.is_some());
    assert!(!runtime.journal_closed);
}

#[test]
fn panicked_journal_close_is_joined_once_and_remains_fail_closed() {
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
            panic_after_release: true,
        })),
    );

    let first = runtime
        .finish_stopping_until(Instant::now() + Duration::from_millis(25))
        .expect_err("the blocked close must retain its worker");
    assert_eq!(first.code, "library_synchronization_stop_timeout");
    assert!(close_entered.load(Ordering::Acquire));
    close_release.store(true, Ordering::Release);
    wait_for_test_signal(&close_finished);
    let worker_finished_deadline = Instant::now() + Duration::from_secs(2);
    while !runtime
        .journal_close
        .as_ref()
        .and_then(|task| task.worker.as_ref())
        .is_some_and(JoinHandle::is_finished)
    {
        assert!(
            Instant::now() < worker_finished_deadline,
            "the panicked close worker did not terminate"
        );
        thread::yield_now();
    }

    let panic = runtime
        .finish_stopping_until(Instant::now() + Duration::from_secs(1))
        .expect_err("a panicked close must fail closed");
    assert_eq!(
        panic.code,
        "persistent_change_journal_close_worker_panicked"
    );
    assert_eq!(close_count.load(Ordering::Acquire), 1);
    let retry = runtime
        .finish_stopping_until(Instant::now() + Duration::from_secs(1))
        .expect_err("a retry must retain the panic outcome");
    assert_eq!(retry.code, panic.code);
    assert_eq!(retry.message, panic.message);
    assert_eq!(close_count.load(Ordering::Acquire), 1);
}

#[test]
fn one_stop_joins_delayed_p0_p1_p2_then_allows_immediate_restart() {
    let close_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let mut runtime = runtime_with_close_probe(Arc::clone(&close_count));
    let p0_cancelled = Arc::new(AtomicBool::new(false));
    let p1_cancelled = Arc::new(AtomicBool::new(false));
    let p2_cancelled = Arc::new(AtomicBool::new(false));
    let p0_finished = Arc::new(AtomicBool::new(false));
    let p1_finished = Arc::new(AtomicBool::new(false));
    let p2_finished = Arc::new(AtomicBool::new(false));

    let (p0_sender, p0_receiver) = mpsc::sync_channel(1);
    let p0_worker_cancelled = Arc::clone(&p0_cancelled);
    let p0_worker_finished = Arc::clone(&p0_finished);
    let p0_worker = thread::spawn(move || {
        while !p0_worker_cancelled.load(Ordering::Acquire) {
            thread::yield_now();
        }
        thread::sleep(Duration::from_millis(30));
        p0_worker_finished.store(true, Ordering::Release);
        let _ = p0_sender.send(LiveWorkOutcome::default());
    });
    runtime.live = Some(LiveTask::from_test_parts(
        "root-p0".to_owned(),
        Arc::clone(&p0_cancelled),
        p0_receiver,
        p0_worker,
    ));

    let (p1_sender, p1_receiver) = mpsc::sync_channel(1);
    let p1_worker_cancelled = Arc::clone(&p1_cancelled);
    let p1_worker_finished = Arc::clone(&p1_finished);
    let p1_worker = thread::spawn(move || {
        while !p1_worker_cancelled.load(Ordering::Acquire) {
            thread::yield_now();
        }
        thread::sleep(Duration::from_millis(30));
        p1_worker_finished.store(true, Ordering::Release);
        let _ = p1_sender.send(Ok(JournalTaskOutcome::Page(
            PersistentJournalEnrollmentReport::default().into(),
        )));
    });
    runtime.journal = Some(JournalTask {
        cancelled: Arc::clone(&p1_cancelled),
        receiver: p1_receiver,
        worker: Some(p1_worker),
    });

    let (p2_sender, p2_receiver) = mpsc::sync_channel(1);
    let p2_worker_cancelled = Arc::clone(&p2_cancelled);
    let p2_worker_finished = Arc::clone(&p2_finished);
    let p2_worker = thread::spawn(move || {
        while !p2_worker_cancelled.load(Ordering::Acquire) {
            thread::yield_now();
        }
        thread::sleep(Duration::from_millis(30));
        p2_worker_finished.store(true, Ordering::Release);
        let _ = p2_sender.send(Ok(RecoveryTaskOutcome::LegacyUnownedDrain(
            IncrementalLibraryChangeReport::default(),
        )));
    });
    runtime.recovery = Some(RecoveryTask {
        root_id: "root-p2".to_owned(),
        kind: RecoveryTaskKind::LegacyUnownedDrain {
            continuity_revision: 1,
        },
        phase: Arc::new(Mutex::new(LibrarySynchronizationPhase::Reconciliation)),
        cancelled: Arc::clone(&p2_cancelled),
        receiver: p2_receiver,
        worker: Some(p2_worker),
    });
    let registry = ready_test_registry(runtime);

    stop_runtime(&registry).expect("one stop joins every delayed worker");
    assert!(p0_finished.load(Ordering::Acquire));
    assert!(p1_finished.load(Ordering::Acquire));
    assert!(p2_finished.load(Ordering::Acquire));
    assert!(matches!(
        registry.state.lock().expect("local registry").entry,
        SynchronizationRuntimeEntry::Empty
    ));
    assert_eq!(close_count.load(Ordering::Acquire), 1);
    stop_runtime(&registry).expect("stop remains idempotent");
    assert_eq!(close_count.load(Ordering::Acquire), 1);

    let SynchronizationRuntimeClaim::Construct { epoch, cancelled } =
        claim_runtime_for_start(&registry).expect("restart immediately after successful stop")
    else {
        panic!("empty registry must admit a fresh construction epoch");
    };
    let owner_ticket = active_owner_ticket(&registry);
    let mut restarted = runtime_with_close_probe(Arc::clone(&close_count));
    restarted.stop_requested = Arc::clone(&cancelled);
    let restarted = match promote_constructed_runtime(
        &registry,
        epoch,
        owner_ticket,
        Arc::clone(&cancelled),
        restarted,
    )
    .expect("promote restarted runtime")
    {
        ConstructedRuntimeDisposition::Poll(runtime) => runtime,
        ConstructedRuntimeDisposition::Drain { .. } => {
            panic!("an active construction epoch must promote for polling")
        }
    };
    execute_runtime_operation(&registry, epoch, owner_ticket, cancelled, restarted, |_| {
        Ok(())
    })
    .expect("install restarted runtime");
    stop_runtime(&registry).expect("stop restarted runtime");
    assert_eq!(close_count.load(Ordering::Acquire), 2);
}

fn assert_stop_is_responsive_during_blocked_poll_operation(operation_name: &'static str) {
    let close_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let registry = ready_test_registry(runtime_with_close_probe(Arc::clone(&close_count)));
    let (epoch, cancelled, runtime) =
        claim_runtime_for_poll(&registry).expect("claim local poll owner");
    let owner_ticket = active_owner_ticket(&registry);
    let operation_entered = Arc::new(AtomicBool::new(false));
    let operation_release = Arc::new(AtomicBool::new(false));
    let owner_registry = Arc::clone(&registry);
    let owner_entered = Arc::clone(&operation_entered);
    let owner_release = Arc::clone(&operation_release);
    let owner = thread::Builder::new()
        .name(format!("ame-test-blocked-{operation_name}"))
        .spawn(move || {
            execute_runtime_operation(
                &owner_registry,
                epoch,
                owner_ticket,
                cancelled,
                runtime,
                move |_| {
                    owner_entered.store(true, Ordering::Release);
                    while !owner_release.load(Ordering::Acquire) {
                        std::thread::yield_now();
                    }
                    Ok(())
                },
            )
        })
        .expect("start blocked poll owner");
    wait_for_test_signal(&operation_entered);

    let stop_registry = Arc::clone(&registry);
    let (stop_sender, stop_receiver) = mpsc::sync_channel(1);
    let stopper = thread::spawn(move || {
        let _ = stop_sender.send(stop_runtime(&stop_registry));
    });
    assert!(
        matches!(
            stop_receiver.recv_timeout(Duration::from_millis(50)),
            Err(RecvTimeoutError::Timeout)
        ),
        "stop must not report success while {operation_name} still owns the runtime"
    );
    assert_start_is_blocked_while_stopping(&registry);

    operation_release.store(true, Ordering::Release);
    stop_receiver
        .recv_timeout(Duration::from_millis(500))
        .expect("single stop must finish after the poll owner releases")
        .expect("single stop completes the late runtime drain");
    stopper.join().expect("join bounded stop caller");
    let error = owner
        .join()
        .expect("join late poll owner")
        .expect_err("a stopped epoch must discard the late operation result");
    assert_eq!(error.code, "library_synchronization_poll_cancelled");
    assert!(matches!(
        registry.state.lock().expect("local registry").entry,
        SynchronizationRuntimeEntry::Empty
    ));
    stop_runtime(&registry).expect("idempotent stop after completed drain");
    assert_eq!(close_count.load(Ordering::Acquire), 1);
}

#[test]
fn stop_and_restart_remain_responsive_during_blocked_filesystem_and_writer_work() {
    assert_stop_is_responsive_during_blocked_poll_operation("filesystem");
    assert_stop_is_responsive_during_blocked_poll_operation("sqlite-writer");
}
