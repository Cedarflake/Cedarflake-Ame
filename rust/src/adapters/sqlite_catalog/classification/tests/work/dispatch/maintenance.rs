use crate::application::classification_dispatch::{
    ClassificationDispatchEvent, ClassificationMaintenance, ClassificationMaintenanceStep,
};

use super::*;

#[derive(Default)]
enum Outcome {
    #[default]
    Prepared,
    Cancel,
    Pause,
    IdleAfterDelay,
    Failure,
}

#[derive(Default)]
struct Maintenance {
    calls: usize,
    completed_work_seen: usize,
    releases: usize,
    outcome: Outcome,
}

impl ClassificationMaintenance<SqliteCatalog, Executor> for Maintenance {
    type Prepared = u64;

    fn prepare(
        &mut self,
        repository: &SqliteCatalog,
        executor: &mut Executor,
        _: u64,
        cancelled: &Arc<AtomicBool>,
    ) -> Result<ClassificationMaintenanceStep<u64>, ScanError> {
        self.calls += 1;
        self.completed_work_seen = executor.observed.borrow().calls;
        assert!(!cancelled.load(Ordering::Acquire));
        assert_eq!(
            scalar(
                repository,
                "SELECT count(*) FROM classification_work_items WHERE phase = 'running'"
            ),
            0
        );
        match self.outcome {
            Outcome::Prepared => {}
            Outcome::Cancel => {
                executor
                    .signals
                    .foreground_changed(ClassificationForegroundState {
                        memory_pressure: true,
                        ..Default::default()
                    })?;
                assert!(cancelled.load(Ordering::Acquire));
            }
            Outcome::Pause => {
                let state = executor.catalog.classification_dispatch_state()?;
                executor.catalog.change_classification_dispatch(
                    state.revision,
                    &ClassificationDispatchCommand::Pause,
                )?;
            }
            Outcome::IdleAfterDelay => {
                executor.clock.set(executor.clock.unix_ms() + 5_000);
                return Ok(ClassificationMaintenanceStep::Idle);
            }
            Outcome::Failure => {
                return Err(ScanError::new(
                    "fixture_maintenance_failed",
                    "Preparation failed",
                ));
            }
        }
        Ok(ClassificationMaintenanceStep::Prepared(7))
    }

    fn release_resources(&mut self) {
        self.releases += 1;
    }
}

fn drain(harness: &mut Harness, maintenance: &mut Maintenance) -> ClassificationDispatchEvent<u64> {
    for _ in 0..16 {
        let event = harness
            .dispatcher
            .step_with_maintenance(maintenance)
            .unwrap();
        if !matches!(
            event,
            ClassificationDispatchEvent::Step(ClassificationDispatchStep::Progress)
        ) {
            return event;
        }
        if maintenance.calls != 0 {
            return event;
        }
    }
    panic!("the bounded fixture did not reach maintenance");
}

#[test]
fn ready_content_finishes_before_maintenance_and_workers_retire_before_handoff() {
    let mut harness = Harness::new();
    harness.clock.set(3_000);
    let mut maintenance = Maintenance::default();
    assert_eq!(
        harness
            .dispatcher
            .run_with_maintenance(&mut maintenance)
            .unwrap(),
        Some(7)
    );
    assert_eq!(maintenance.calls, 1);
    assert_eq!(maintenance.completed_work_seen, 4);
    assert!(harness.observed.borrow().retired > 0);
    assert!(maintenance.releases > 0);
}

#[test]
fn quiet_foreground_pause_and_close_all_prevent_maintenance() {
    let mut harness = Harness::new();
    let mut maintenance = Maintenance::default();
    assert!(matches!(
        drain(&mut harness, &mut maintenance),
        ClassificationDispatchEvent::Step(ClassificationDispatchStep::Waiting { .. })
    ));
    harness.clock.set(3_000);
    harness
        .signals
        .foreground_changed(ClassificationForegroundState {
            reading_sources: true,
            ..Default::default()
        })
        .unwrap();
    assert!(matches!(
        drain(&mut harness, &mut maintenance),
        ClassificationDispatchEvent::Step(ClassificationDispatchStep::Waiting { .. })
    ));
    harness
        .signals
        .foreground_changed(ClassificationForegroundState::default())
        .unwrap();
    harness.clock.set(6_000);
    let state = harness.catalog.classification_dispatch_state().unwrap();
    harness
        .catalog
        .change_classification_dispatch(state.revision, &ClassificationDispatchCommand::Pause)
        .unwrap();
    assert!(matches!(
        drain(&mut harness, &mut maintenance),
        ClassificationDispatchEvent::Step(ClassificationDispatchStep::Waiting { .. })
    ));
    harness.signals.close().unwrap();
    assert!(matches!(
        drain(&mut harness, &mut maintenance),
        ClassificationDispatchEvent::Step(ClassificationDispatchStep::Closed)
    ));
    assert_eq!(maintenance.calls, 0);
    assert!(maintenance.releases > 0);
}

#[test]
fn cancellation_discards_a_ready_plan_and_releases_resources_on_the_next_turn() {
    let mut harness = Harness::new();
    harness.clock.set(3_000);
    let mut maintenance = Maintenance {
        outcome: Outcome::Cancel,
        ..Default::default()
    };
    assert!(matches!(
        drain(&mut harness, &mut maintenance),
        ClassificationDispatchEvent::Step(ClassificationDispatchStep::Progress)
    ));
    assert!(matches!(
        drain(&mut harness, &mut maintenance),
        ClassificationDispatchEvent::Step(ClassificationDispatchStep::Waiting { .. })
    ));
    assert_eq!(maintenance.calls, 1);
    assert!(maintenance.releases > 0);
    assert!(harness.observed.borrow().retired > 0);
}

#[test]
fn a_pause_committed_without_a_wakeup_still_rejects_the_prepared_plan() {
    let mut harness = Harness::new();
    harness.clock.set(3_000);
    let mut maintenance = Maintenance {
        outcome: Outcome::Pause,
        ..Default::default()
    };
    assert!(matches!(
        drain(&mut harness, &mut maintenance),
        ClassificationDispatchEvent::Step(ClassificationDispatchStep::Progress)
    ));
    assert_eq!(
        harness
            .catalog
            .classification_dispatch_state()
            .unwrap()
            .mode,
        ClassificationDispatchMode::Paused
    );
    assert_eq!(maintenance.calls, 1);
}

#[test]
fn the_wait_deadline_is_recomputed_after_maintenance() {
    let mut harness = Harness::new();
    harness.clock.set(3_000);
    harness.observed.borrow_mut().action = Action::Transient;
    let mut maintenance = Maintenance {
        outcome: Outcome::IdleAfterDelay,
        ..Default::default()
    };
    assert!(matches!(drain(&mut harness, &mut maintenance),
        ClassificationDispatchEvent::Step(ClassificationDispatchStep::Waiting { wake_after: Some(delay) }) if delay.is_zero()));
    assert_eq!(maintenance.calls, 1);
}

#[test]
fn preparation_and_retirement_failures_never_return_a_handoff() {
    for (outcome, retirement_error, expected) in [
        (Outcome::Failure, false, "fixture_maintenance_failed"),
        (Outcome::Prepared, true, "fixture_retirement_failed"),
    ] {
        let mut harness = Harness::new();
        harness.clock.set(3_000);
        harness.observed.borrow_mut().retirement_error = retirement_error;
        let mut maintenance = Maintenance {
            outcome,
            ..Default::default()
        };
        assert_eq!(
            harness
                .dispatcher
                .run_with_maintenance(&mut maintenance)
                .unwrap_err()
                .code,
            expected
        );
        assert_eq!(maintenance.calls, 1);
        assert!(maintenance.releases > 0);
    }
}
