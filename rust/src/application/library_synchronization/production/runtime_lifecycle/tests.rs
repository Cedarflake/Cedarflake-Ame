use std::collections::VecDeque;
use std::sync::Condvar;

use super::super::runtime_test_support::*;
use super::super::*;
use super::admission::*;
use super::operation::*;
use super::retirement::*;
use super::state::*;

mod admission;
mod journal_sessions;
mod late_completion;
mod panics;
mod poll_catalog;
mod shutdown;

fn runtime_with_drain_panic(
    point: DrainPanicPoint,
    remaining: Option<usize>,
    close_count: Arc<std::sync::atomic::AtomicUsize>,
) -> ProductionSynchronization {
    let mut runtime = runtime_with_close_probe(close_count);
    match point {
        DrainPanicPoint::P0Result => {
            let (sender, receiver) = mpsc::sync_channel(1);
            let worker = thread::spawn(move || {
                let _ = sender.send(LiveWorkOutcome::default());
            });
            runtime.live = Some(LiveTask::from_test_parts(
                "panic-p0".to_owned(),
                Arc::new(AtomicBool::new(false)),
                receiver,
                worker,
            ));
        }
        DrainPanicPoint::P1Result => {
            let (sender, receiver) = mpsc::sync_channel(1);
            let worker = thread::spawn(move || {
                let _ = sender.send(Ok(JournalTaskOutcome::Page(
                    PersistentJournalEnrollmentReport::default().into(),
                )));
            });
            runtime.journal = Some(JournalTask::from_test_parts(
                Arc::new(AtomicBool::new(false)),
                receiver,
                worker,
            ));
        }
        DrainPanicPoint::P2Result => {
            let (sender, receiver) = mpsc::sync_channel(1);
            let worker = thread::spawn(move || {
                let _ = sender.send(Ok(RecoveryTaskOutcome::LegacyUnownedDrain(
                    IncrementalLibraryChangeReport::default(),
                )));
            });
            runtime.recovery = Some(RecoveryTask::from_test_parts(
                "panic-p2".to_owned(),
                RecoveryTaskKind::LegacyUnownedDrain {
                    continuity_revision: 1,
                },
                Arc::new(Mutex::new(LibrarySynchronizationPhase::Reconciliation)),
                Arc::new(AtomicBool::new(false)),
                receiver,
                Some(worker),
            ));
        }
        DrainPanicPoint::RequestStop
        | DrainPanicPoint::JournalCloseInstalled
        | DrainPanicPoint::JournalCloseResult => {}
    }
    runtime.inject_drain_panic(point, remaining);
    runtime
}

fn ready_test_registry(
    mut runtime: ProductionSynchronization,
) -> Arc<SynchronizationRuntimeRegistry> {
    let cancelled = Arc::new(AtomicBool::new(false));
    runtime.stop_requested = Arc::clone(&cancelled);
    Arc::new(SynchronizationRuntimeRegistry {
        state: Mutex::new(SynchronizationRuntimeState {
            next_epoch: 1,
            next_lifecycle_ordinal: LifecycleOrdinal(1),
            cancelled_through_ordinal: LifecycleOrdinal(0),
            admitted_stop_fences: VecDeque::new(),
            retired_through_epoch: 0,
            entry: SynchronizationRuntimeEntry::Ready {
                epoch: 1,
                owner_ticket: RuntimeStartTicket::from_ordinal(LifecycleOrdinal(1)),
                cancelled,
                runtime: Arc::new(Mutex::new(runtime)),
            },
        }),
        changed: Condvar::new(),
        panic_retention_barrier: Mutex::new(None),
    })
}

fn assert_start_is_blocked_while_stopping(registry: &SynchronizationRuntimeRegistry) {
    let error = match claim_runtime_for_start(registry) {
        Ok(_) => panic!("a stopping epoch must not admit another synchronization session"),
        Err(error) => error,
    };
    assert_eq!(error.code, "library_synchronization_stop_in_progress");
}

fn wait_for_draining_state(registry: &SynchronizationRuntimeRegistry, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    loop {
        if matches!(
            registry.state.lock().expect("local registry").entry,
            SynchronizationRuntimeEntry::Draining { .. }
        ) {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        thread::yield_now();
    }
}

fn wait_for_retained_journal_close_worker(
    registry: &SynchronizationRuntimeRegistry,
    timeout: Duration,
) {
    let deadline = Instant::now() + timeout;
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
            return;
        }
        assert!(
            Instant::now() < deadline,
            "the retained journal-close worker did not finish after release"
        );
        thread::yield_now();
    }
}

fn retained_stop_deadline(registry: &SynchronizationRuntimeRegistry) -> Option<Instant> {
    match registry.state.lock().expect("local registry").entry {
        SynchronizationRuntimeEntry::Stopping { deadline, .. }
        | SynchronizationRuntimeEntry::Draining { deadline, .. } => Some(deadline),
        _ => None,
    }
}

fn active_owner_ticket(registry: &SynchronizationRuntimeRegistry) -> RuntimeStartTicket {
    registry
        .state
        .lock()
        .expect("local registry")
        .entry
        .owner_ticket()
        .expect("test runtime owner ticket")
}
