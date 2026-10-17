use super::*;

fn opened_runtime(factory: &Arc<CountingJournalFactory>) -> ProductionSynchronization {
    let mut runtime = new_production_synchronization_with_factory(
        crate::ports::erase_library_change_source_factory(HealthyFactory),
        factory.clone(),
    );
    runtime.open_persistent_change_journal_after_watcher();
    runtime
}

#[test]
fn journal_factory_reopens_only_after_the_previous_runtime_retires() {
    let connect_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let close_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let factory = Arc::new(CountingJournalFactory {
        connect_count: Arc::clone(&connect_count),
        close_count: Arc::clone(&close_count),
        close_failures: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
    });
    let registry = ready_test_registry(opened_runtime(&factory));

    stop_runtime(&registry).expect("retire the first runtime and journal session");
    assert_eq!(connect_count.load(Ordering::Acquire), 1);
    assert_eq!(close_count.load(Ordering::Acquire), 1);

    let SynchronizationRuntimeClaim::Construct { epoch, cancelled } =
        claim_runtime_for_start(&registry).expect("admit replacement after retirement")
    else {
        panic!("the retired runtime must admit a new construction epoch");
    };
    let owner_ticket = active_owner_ticket(&registry);
    let mut replacement = opened_runtime(&factory);
    replacement.stop_requested = Arc::clone(&cancelled);
    let replacement = match promote_constructed_runtime(
        &registry,
        epoch,
        owner_ticket,
        Arc::clone(&cancelled),
        replacement,
    )
    .expect("promote the replacement runtime")
    {
        ConstructedRuntimeDisposition::Poll(runtime) => runtime,
        ConstructedRuntimeDisposition::Drain { .. } => {
            panic!("the current replacement must remain eligible to poll")
        }
    };
    execute_runtime_operation(
        &registry,
        epoch,
        owner_ticket,
        cancelled,
        replacement,
        |_| Ok(()),
    )
    .expect("publish the replacement runtime");
    assert_eq!(connect_count.load(Ordering::Acquire), 2);
    assert_eq!(close_count.load(Ordering::Acquire), 1);

    stop_runtime(&registry).expect("retire the replacement runtime");
    assert_eq!(close_count.load(Ordering::Acquire), 2);
}

#[test]
fn failed_journal_close_prevents_a_replacement_session() {
    let connect_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let close_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let factory = Arc::new(CountingJournalFactory {
        connect_count: Arc::clone(&connect_count),
        close_count: Arc::clone(&close_count),
        close_failures: Arc::new(std::sync::atomic::AtomicUsize::new(1)),
    });
    let registry = ready_test_registry(opened_runtime(&factory));

    for _ in 0..2 {
        let error = stop_runtime(&registry).expect_err("retain the failed close outcome");
        assert_eq!(error.code, "persistent_change_journal_close_failed");
        let error = start_runtime_with(&registry, || opened_runtime(&factory))
            .expect_err("an unretired session must block replacement construction");
        assert_eq!(error.code, "library_synchronization_stop_in_progress");
        assert_eq!(connect_count.load(Ordering::Acquire), 1);
        assert_eq!(close_count.load(Ordering::Acquire), 1);
    }
    assert!(matches!(
        registry.state.lock().expect("local registry").entry,
        SynchronizationRuntimeEntry::Draining { .. }
    ));
}
