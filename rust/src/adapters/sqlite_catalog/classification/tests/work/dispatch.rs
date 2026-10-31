use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;

use crate::application::classification_dispatch::{
    ClassificationDispatchClock, ClassificationDispatchSignals, ClassificationDispatchStep,
    ClassificationDispatcher, ClassificationForegroundState,
};
use crate::ports::classification_execution::{
    ClassificationExecutionError, ClassificationExecutionOutcome, ClassificationWorkExecutor,
};

use super::*;

#[derive(Clone, Default)]
struct Clock(Arc<AtomicU64>);

impl ClassificationDispatchClock for Clock {
    fn elapsed(&self) -> Duration {
        Duration::from_millis(self.unix_ms())
    }
    fn unix_ms(&self) -> u64 {
        self.0.load(Ordering::Acquire)
    }
}

impl Clock {
    fn set(&self, millis: u64) {
        self.0.store(millis, Ordering::Release);
    }
}

#[derive(Default)]
enum Action {
    #[default]
    Complete,
    Foreground,
    Transient,
    Blocked,
}

#[derive(Default)]
struct Observed {
    calls: usize,
    retired: usize,
    action: Action,
    failure_duration_ms: u64,
}

struct Executor {
    catalog: SqliteCatalog,
    run: ClassificationRun,
    signals: ClassificationDispatchSignals<Clock>,
    observed: Rc<RefCell<Observed>>,
    clock: Clock,
}

impl ClassificationWorkExecutor for Executor {
    fn execute(
        &mut self,
        lease: &ClassificationWorkLease,
        _: u64,
        cancelled: &AtomicBool,
    ) -> Result<ClassificationExecutionOutcome, ClassificationExecutionError> {
        let mut observed = self.observed.borrow_mut();
        observed.calls += 1;
        match observed.action {
            Action::Foreground => {
                self.signals
                    .foreground_changed(ClassificationForegroundState {
                        reading_sources: true,
                        ..Default::default()
                    })
                    .unwrap();
                assert!(cancelled.load(Ordering::Acquire));
                observed.action = Action::Complete;
                return Ok(ClassificationExecutionOutcome::Cancelled);
            }
            Action::Transient => {
                self.clock
                    .set(self.clock.unix_ms() + observed.failure_duration_ms);
                return Err(ClassificationExecutionError::Work {
                    kind: ClassificationWorkFailure::Transient,
                    error: ScanError::new("fixture_worker_exited", "Worker exited"),
                });
            }
            Action::Blocked => {
                return Err(ClassificationExecutionError::Blocked(ScanError::new(
                    "fixture_model_missing",
                    "Model is unavailable",
                )));
            }
            Action::Complete => {}
        }
        let committed = match lease.target {
            ClassificationWorkTarget::VerifySource(_) => self
                .catalog
                .finish_classification_verification(lease)
                .unwrap(),
            ClassificationWorkTarget::AnalyzeContent { .. } => {
                let evidence = evidence(&self.run, lease);
                self.catalog
                    .publish_classification_work(lease, &evidence)
                    .unwrap()
            }
        };
        assert!(committed);
        Ok(ClassificationExecutionOutcome::Completed)
    }

    fn retire(&mut self) -> Result<(), ScanError> {
        self.observed.borrow_mut().retired += 1;
        Ok(())
    }
    fn retire_if_idle(&mut self) -> Result<(), ScanError> {
        Ok(())
    }
    fn idle_delay(&self) -> Option<Duration> {
        None
    }
}

struct Harness {
    directory: tempfile::TempDir,
    catalog: SqliteCatalog,
    run: ClassificationRun,
    signals: ClassificationDispatchSignals<Clock>,
    clock: Clock,
    observed: Rc<RefCell<Observed>>,
    dispatcher: ClassificationDispatcher<SqliteCatalog, Executor, Clock>,
}

impl Harness {
    fn new() -> Self {
        let (directory, mut catalog) = fixture();
        let (run, session) = start(&mut catalog);
        for name in ["a", "b", "c"] {
            verify(&mut catalog, name, 'a');
        }
        let clock = Clock::default();
        let signals = ClassificationDispatchSignals::new(clock.clone());
        let observed = Rc::new(RefCell::new(Observed::default()));
        let executor = Executor {
            catalog: SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap(),
            run: run.clone(),
            signals: signals.clone(),
            observed: observed.clone(),
            clock: clock.clone(),
        };
        let observer = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
        let dispatcher =
            ClassificationDispatcher::open(catalog, executor, session, signals.clone()).unwrap();
        Self {
            directory,
            catalog: observer,
            run,
            signals,
            clock,
            observed,
            dispatcher,
        }
    }

    fn drain(&mut self) -> ClassificationDispatchStep {
        for _ in 0..16 {
            let step = self.dispatcher.step().unwrap();
            if step != ClassificationDispatchStep::Progress {
                return step;
            }
        }
        panic!("the bounded fixture did not reach a wait boundary")
    }
}

#[test]
fn actual_queue_drains_one_shared_prediction_and_does_not_repeat_it_after_restart() {
    let mut harness = Harness::new();
    assert_eq!(
        harness.dispatcher.step().unwrap(),
        ClassificationDispatchStep::Waiting {
            wake_after: Some(Duration::from_secs(3)),
        }
    );
    assert_eq!(harness.observed.borrow().calls, 0);
    harness.clock.set(3_000);
    assert_eq!(
        harness.drain(),
        ClassificationDispatchStep::Waiting { wake_after: None }
    );
    assert_eq!(harness.observed.borrow().calls, 4);
    assert_eq!(
        scalar(
            &harness.catalog,
            "SELECT count(*) FROM classification_predictions"
        ),
        1
    );
    let executor = Executor {
        catalog: SqliteCatalog::open(harness.directory.path().join("catalog.sqlite3")).unwrap(),
        run: harness.run.clone(),
        signals: harness.signals.clone(),
        observed: harness.observed.clone(),
        clock: harness.clock.clone(),
    };
    harness.dispatcher = ClassificationDispatcher::open(
        SqliteCatalog::open(harness.directory.path().join("catalog.sqlite3")).unwrap(),
        executor,
        id("next-process"),
        harness.signals.clone(),
    )
    .unwrap();
    assert_eq!(
        harness.drain(),
        ClassificationDispatchStep::Waiting { wake_after: None }
    );
    assert_eq!(harness.observed.borrow().calls, 4);
}

#[test]
fn pause_after_a_checkpoint_preserves_it_until_explicit_continue() {
    let mut harness = Harness::new();
    harness.clock.set(3_000);
    assert_eq!(
        harness.dispatcher.step().unwrap(),
        ClassificationDispatchStep::Progress
    );
    change(&mut harness.catalog, ClassificationDispatchCommand::Pause);
    harness.signals.dispatch_changed().unwrap();
    harness.clock.set(60_000);
    assert_eq!(
        harness.drain(),
        ClassificationDispatchStep::Waiting { wake_after: None }
    );
    assert_eq!(harness.observed.borrow().calls, 1);
    assert_eq!(
        scalar(
            &harness.catalog,
            "SELECT count(*) FROM classification_work_items WHERE phase = 'done'"
        ),
        1
    );
    change(
        &mut harness.catalog,
        ClassificationDispatchCommand::Continue,
    );
    harness.signals.dispatch_changed().unwrap();
    assert_eq!(
        harness.drain(),
        ClassificationDispatchStep::Waiting {
            wake_after: Some(Duration::from_secs(3))
        }
    );
    harness.clock.set(63_000);
    harness.drain();
    assert_eq!(harness.observed.borrow().calls, 4);
}

#[test]
fn foreground_yield_requeues_without_failure_and_retires_before_waiting() {
    let mut harness = Harness::new();
    harness.clock.set(3_000);
    harness.observed.borrow_mut().action = Action::Foreground;
    harness.dispatcher.step().unwrap();
    assert!(harness.observed.borrow().retired > 0);
    assert_eq!(
        scalar(
            &harness.catalog,
            "SELECT count(*) FROM classification_work_items WHERE phase = 'running'"
        ),
        0
    );
    assert_eq!(
        scalar(
            &harness.catalog,
            "SELECT sum(failure_count) FROM classification_work_items"
        ),
        0
    );
    assert_eq!(
        harness.drain(),
        ClassificationDispatchStep::Waiting { wake_after: None }
    );
    harness
        .signals
        .foreground_changed(ClassificationForegroundState::default())
        .unwrap();
    harness.clock.set(6_000);
    harness.drain();
    assert_eq!(
        scalar(
            &harness.catalog,
            "SELECT count(*) FROM classification_predictions"
        ),
        1
    );
}

#[test]
fn transient_failures_sleep_until_the_next_retry_then_exhaust_the_durable_budget() {
    let mut harness = Harness::new();
    harness.observed.borrow_mut().action = Action::Transient;
    for now in [3_000, 8_000] {
        harness.clock.set(now);
        assert_eq!(
            harness.drain(),
            ClassificationDispatchStep::Waiting {
                wake_after: Some(Duration::from_secs(5))
            }
        );
    }
    harness.clock.set(13_000);
    assert_eq!(
        harness.drain(),
        ClassificationDispatchStep::Waiting { wake_after: None }
    );
    assert_eq!(harness.observed.borrow().calls, 9);
    assert_eq!(
        scalar(
            &harness.catalog,
            "SELECT count(*) FROM classification_work_items WHERE phase = 'failed' AND failure_count = 3"
        ),
        3
    );
}

#[test]
fn setup_failure_stops_the_loop_without_marking_library_images_as_failures() {
    let mut harness = Harness::new();
    harness.clock.set(3_000);
    harness.observed.borrow_mut().action = Action::Blocked;
    assert_eq!(
        harness.dispatcher.run().unwrap_err().code,
        "fixture_model_missing"
    );
    assert_eq!(harness.observed.borrow().calls, 1);
    assert_eq!(
        scalar(
            &harness.catalog,
            "SELECT sum(failure_count) FROM classification_work_items"
        ),
        0
    );
    assert_eq!(
        scalar(
            &harness.catalog,
            "SELECT count(*) FROM classification_work_items WHERE phase = 'running'"
        ),
        0
    );
    assert!(harness.observed.borrow().retired > 0);
}

#[test]
fn failure_processing_time_does_not_consume_the_retry_interval() {
    let mut harness = Harness::new();
    harness.clock.set(3_000);
    harness.observed.borrow_mut().action = Action::Transient;
    harness.observed.borrow_mut().failure_duration_ms = 4_000;
    harness.dispatcher.step().unwrap();
    let ready: i64 = harness.catalog.connection.query_row(
        "SELECT ready_unix_ms FROM classification_work_items WHERE issue_code = 'fixture_worker_exited'",
        [], |row| row.get(0),
    ).unwrap();
    assert_eq!(ready, 12_000);
}

#[test]
fn an_empty_queue_closes_without_a_polling_timer() {
    let mut harness = Harness::new();
    harness.clock.set(3_000);
    harness.drain();
    harness.signals.close().unwrap();
    harness.dispatcher.run().unwrap();
    assert_eq!(harness.observed.borrow().calls, 4);
    assert!(harness.observed.borrow().retired > 0);
}
