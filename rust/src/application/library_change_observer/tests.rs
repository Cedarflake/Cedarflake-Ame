use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use crate::domain::{
    CatalogFreshnessState, LibraryChangeObservation, LibraryChangeObservationKind,
    LibraryChangeOrigin, LibraryChangePlanningLimits, LibraryChangeRestartPolicy,
    LibraryChangeScope, LibraryChangeSourceBatch, LibraryChangeSourceError,
    LibraryChangeSourceHealth, LibraryChangeSourceStopReport, LibraryRootGeneration,
};
use crate::ports::{LibraryChangeSource, LibraryChangeSourceFactory, LibraryChangeSourceRequest};

use super::LibraryChangeObserver;

#[derive(Clone)]
struct FakeFactory {
    state: Arc<Mutex<FakeFactoryState>>,
}

struct FakeFactoryState {
    starts: u32,
    outcomes: VecDeque<Result<FakeSource, LibraryChangeSourceError>>,
    start_delay: Duration,
}

struct FakeSource {
    health: LibraryChangeSourceHealth,
    batches: VecDeque<Result<LibraryChangeSourceBatch, LibraryChangeSourceError>>,
    stops: Arc<Mutex<u32>>,
    stop_delay: Duration,
    should_fail_stop: bool,
}

impl LibraryChangeSourceFactory for FakeFactory {
    type Source = FakeSource;

    fn start(
        &self,
        _request: &LibraryChangeSourceRequest,
    ) -> Result<Self::Source, LibraryChangeSourceError> {
        let mut state = self.state.lock().expect("factory state");
        state.starts = state.starts.saturating_add(1);
        let start_delay = state.start_delay;
        let outcome = state.outcomes.pop_front().unwrap_or_else(|| {
            Err(LibraryChangeSourceError::new(
                "fake_source_missing",
                "No fake source outcome was configured.",
            ))
        });
        drop(state);
        thread::sleep(start_delay);
        outcome
    }
}

impl LibraryChangeSource for FakeSource {
    fn health(&self) -> LibraryChangeSourceHealth {
        self.health
    }

    fn drain(
        &mut self,
        _max_observations: usize,
    ) -> Result<LibraryChangeSourceBatch, LibraryChangeSourceError> {
        self.batches.pop_front().unwrap_or_else(|| {
            Ok(LibraryChangeSourceBatch {
                observations: Vec::new(),
                health: self.health,
                dropped_observation_count: 0,
                ignored_callback_count: 0,
            })
        })
    }

    fn stop(&mut self) -> Result<LibraryChangeSourceStopReport, LibraryChangeSourceError> {
        let mut stops = self.stops.lock().expect("stop state");
        *stops = stops.saturating_add(1);
        drop(stops);
        thread::sleep(self.stop_delay);
        if self.should_fail_stop {
            return Err(LibraryChangeSourceError::retryable(
                "fake_stop_failed",
                "stop failed",
            ));
        }
        self.health = LibraryChangeSourceHealth::Stopped;
        Ok(LibraryChangeSourceStopReport {
            elapsed_millis: 3,
            ignored_callback_count: 0,
        })
    }
}

#[test]
fn start_failure_degrades_immediately_and_restarts_only_after_the_deadline() {
    let stops = Arc::new(Mutex::new(0));
    let state = Arc::new(Mutex::new(FakeFactoryState {
        starts: 0,
        start_delay: Duration::ZERO,
        outcomes: VecDeque::from([
            Err(LibraryChangeSourceError::retryable(
                "fake_start_failed",
                "start failed",
            )),
            Ok(source(LibraryChangeSourceHealth::Healthy, stops)),
        ]),
    }));
    let mut observer = LibraryChangeObserver::start(
        FakeFactory {
            state: Arc::clone(&state),
        },
        request(),
        limits(),
        restart_policy(),
        1_000,
    )
    .expect("observer");

    let degraded = observer.poll(1_249).expect("degraded poll");
    assert_eq!(degraded.source_health, LibraryChangeSourceHealth::Failed);
    assert_eq!(degraded.restart_attempt, 1);
    assert_eq!(degraded.next_restart_unix_ms, Some(1_250));
    assert_eq!(
        degraded.last_source_error_code.as_deref(),
        Some("fake_start_failed")
    );
    assert_eq!(
        degraded.planning.freshness,
        CatalogFreshnessState::NeedsReconciliation
    );
    assert_eq!(state.lock().expect("factory state").starts, 1);

    let starting = observer.poll(1_250).expect("restart poll");
    assert_eq!(starting.source_health, LibraryChangeSourceHealth::Starting);
    let recovered = wait_for_source_state(&mut observer, 1_250, |poll| {
        poll.source_health == LibraryChangeSourceHealth::Healthy
    });
    assert_eq!(recovered.source_health, LibraryChangeSourceHealth::Healthy);
    assert_eq!(recovered.restart_attempt, 1);
    assert_eq!(recovered.next_restart_unix_ms, None);
    assert_eq!(recovered.last_source_error_code, None);
    assert_eq!(state.lock().expect("factory state").starts, 2);
}

#[test]
fn invalid_planning_limits_are_rejected_before_source_creation() {
    let factory = factory([]);
    let state = Arc::clone(&factory.state);
    let invalid_limits = LibraryChangePlanningLimits {
        max_observations: 0,
        max_intents: 1,
    };

    let error =
        match LibraryChangeObserver::start(factory, request(), invalid_limits, restart_policy(), 0)
        {
            Ok(_) => panic!("invalid planning limits must be rejected"),
            Err(error) => error,
        };

    assert_eq!(error.code, "change_observer_planning_limits_invalid");
    assert_eq!(state.lock().expect("factory state").starts, 0);
}

#[test]
fn failed_callback_ingress_stops_the_source_and_schedules_bounded_backoff() {
    let stops = Arc::new(Mutex::new(0));
    let failing_source = FakeSource {
        health: LibraryChangeSourceHealth::Healthy,
        batches: VecDeque::from([Err(LibraryChangeSourceError::new(
            "fake_callback_failed",
            "callback failed",
        ))]),
        stops: Arc::clone(&stops),
        stop_delay: Duration::ZERO,
        should_fail_stop: false,
    };
    let factory = factory([Ok(failing_source)]);
    let mut observer =
        LibraryChangeObserver::start(factory, request(), limits(), restart_policy(), 10_000)
            .expect("observer");

    let poll = observer.poll(10_010).expect("failed poll");

    assert_eq!(poll.source_health, LibraryChangeSourceHealth::Failed);
    assert_eq!(poll.restart_attempt, 1);
    assert_eq!(poll.next_restart_unix_ms, Some(10_260));
    assert_eq!(
        poll.last_source_error_code.as_deref(),
        Some("fake_callback_failed")
    );
    let stopped = wait_for_stop(&stops);
    assert_eq!(stopped, 1);
    let waiting = observer.poll(10_020).expect("advance stop");
    assert_eq!(waiting.restart_attempt, 1);
    assert_eq!(waiting.next_restart_unix_ms, Some(10_260));
}

#[test]
fn delivered_degraded_gap_restarts_the_source_and_can_recover_health() {
    let stops = Arc::new(Mutex::new(0));
    let degraded_batch = LibraryChangeSourceBatch {
        observations: vec![LibraryChangeObservation {
            root_id: "root-a".to_owned(),
            root_generation: LibraryRootGeneration::new(7).expect("generation"),
            sequence: 1,
            observed_unix_ms: 1_000,
            kind: LibraryChangeObservationKind::EvidenceGap,
            scope: LibraryChangeScope::Root,
            relative_path: String::new(),
            previous_relative_path: None,
            origin: LibraryChangeOrigin::LiveNotification,
        }],
        health: LibraryChangeSourceHealth::Degraded,
        dropped_observation_count: 1,
        ignored_callback_count: 0,
    };
    let degraded_source = FakeSource {
        health: LibraryChangeSourceHealth::Healthy,
        batches: VecDeque::from([Ok(degraded_batch)]),
        stops: Arc::clone(&stops),
        stop_delay: Duration::ZERO,
        should_fail_stop: false,
    };
    let recovered_source = source(LibraryChangeSourceHealth::Healthy, Arc::clone(&stops));
    let factory = factory([Ok(degraded_source), Ok(recovered_source)]);
    let state = Arc::clone(&factory.state);
    let mut observer =
        LibraryChangeObserver::start(factory, request(), limits(), restart_policy(), 0)
            .expect("observer");

    let degraded = observer.poll(1).expect("degraded poll");

    assert_eq!(degraded.source_health, LibraryChangeSourceHealth::Degraded);
    assert_eq!(degraded.dropped_observation_count, 1);
    assert_eq!(
        degraded.planning.freshness,
        CatalogFreshnessState::NeedsReconciliation
    );
    assert!(
        degraded
            .planning
            .issues
            .contains(&crate::domain::LibraryChangePlanningIssue::ChangeEvidenceGap)
    );
    wait_for_stop(&stops);

    let deadline = Instant::now() + Duration::from_secs(1);
    let waiting = loop {
        let poll = observer.poll(2).expect("advance degraded stop");
        if poll.next_restart_unix_ms.is_some() {
            break poll;
        }
        assert!(
            Instant::now() < deadline,
            "timed out waiting for degraded source stop"
        );
        thread::yield_now();
    };
    assert_eq!(waiting.restart_attempt, 1);
    assert_eq!(waiting.next_restart_unix_ms, Some(251));

    let starting = observer.poll(251).expect("restart poll");
    assert_eq!(starting.source_health, LibraryChangeSourceHealth::Starting);
    let recovered = wait_for_source_state(&mut observer, 251, |poll| {
        poll.source_health == LibraryChangeSourceHealth::Healthy
    });

    assert_eq!(recovered.source_health, LibraryChangeSourceHealth::Healthy);
    assert_eq!(
        recovered.planning.freshness,
        CatalogFreshnessState::Synchronized
    );
    assert_eq!(recovered.restart_attempt, 1);
    assert_eq!(state.lock().expect("factory state").starts, 2);
}

#[test]
fn slow_runtime_stop_does_not_block_poll_and_window_close_remains_bounded() {
    let stops = Arc::new(Mutex::new(0));
    let failing_source = FakeSource {
        health: LibraryChangeSourceHealth::Healthy,
        batches: VecDeque::from([Err(LibraryChangeSourceError::retryable(
            "fake_callback_failed",
            "callback failed",
        ))]),
        stops: Arc::clone(&stops),
        stop_delay: Duration::from_millis(500),
        should_fail_stop: false,
    };
    let mut observer = LibraryChangeObserver::start(
        factory([Ok(failing_source)]),
        request(),
        limits(),
        restart_policy(),
        0,
    )
    .expect("observer");

    let poll_started = Instant::now();
    let poll = observer.poll(1).expect("non-blocking failure poll");

    assert!(poll_started.elapsed() < Duration::from_millis(200));
    assert_eq!(poll.source_health, LibraryChangeSourceHealth::Failed);
    let close_started = Instant::now();
    observer.stop().expect("bounded window-close stop");
    assert!(close_started.elapsed() < Duration::from_secs(2));
    assert_eq!(*stops.lock().expect("stop state"), 1);
}

#[test]
fn stale_generation_callbacks_cannot_enter_the_current_plan() {
    let stops = Arc::new(Mutex::new(0));
    let mut stale_observation = observation("late.jpg");
    stale_observation.root_generation = LibraryRootGeneration::new(6).expect("generation");
    let batch = LibraryChangeSourceBatch {
        observations: vec![stale_observation],
        health: LibraryChangeSourceHealth::Healthy,
        dropped_observation_count: 0,
        ignored_callback_count: 0,
    };
    let source = FakeSource {
        health: LibraryChangeSourceHealth::Healthy,
        batches: VecDeque::from([Ok(batch)]),
        stops,
        stop_delay: Duration::ZERO,
        should_fail_stop: false,
    };
    let mut observer = LibraryChangeObserver::start(
        factory([Ok(source)]),
        request(),
        limits(),
        restart_policy(),
        0,
    )
    .expect("observer");

    let poll = observer.poll(1).expect("poll");

    assert_eq!(poll.planning.superseded_observation_count, 1);
    assert!(poll.planning.intents.is_empty());
    assert_eq!(poll.planning.freshness, CatalogFreshnessState::Synchronized);
}

#[test]
fn dropped_observations_force_root_reconciliation_even_if_a_source_claims_healthy() {
    let stops = Arc::new(Mutex::new(0));
    let batch = LibraryChangeSourceBatch {
        observations: Vec::new(),
        health: LibraryChangeSourceHealth::Healthy,
        dropped_observation_count: 1,
        ignored_callback_count: 0,
    };
    let source = FakeSource {
        health: LibraryChangeSourceHealth::Healthy,
        batches: VecDeque::from([Ok(batch)]),
        stops,
        stop_delay: Duration::ZERO,
        should_fail_stop: false,
    };
    let mut observer = LibraryChangeObserver::start(
        factory([Ok(source)]),
        request(),
        limits(),
        restart_policy(),
        0,
    )
    .expect("observer");

    let poll = observer.poll(1).expect("poll");

    assert_eq!(poll.source_health, LibraryChangeSourceHealth::Degraded);
    assert_eq!(poll.dropped_observation_count, 1);
    assert_eq!(
        poll.planning.freshness,
        CatalogFreshnessState::NeedsReconciliation
    );
    assert!(poll.planning.intents.iter().any(|intent| {
        intent.kind == crate::domain::LibraryChangeIntentKind::FreshnessUnknown
            && intent.scope == LibraryChangeScope::Root
    }));
}

#[test]
fn dropped_observations_never_downgrade_failed_source_health() {
    let stops = Arc::new(Mutex::new(0));
    let batch = LibraryChangeSourceBatch {
        observations: Vec::new(),
        health: LibraryChangeSourceHealth::Failed,
        dropped_observation_count: 1,
        ignored_callback_count: 0,
    };
    let source = FakeSource {
        health: LibraryChangeSourceHealth::Healthy,
        batches: VecDeque::from([Ok(batch)]),
        stops,
        stop_delay: Duration::ZERO,
        should_fail_stop: false,
    };
    let mut observer = LibraryChangeObserver::start(
        factory([Ok(source)]),
        request(),
        limits(),
        restart_policy(),
        0,
    )
    .expect("observer");

    let poll = observer.poll(1).expect("poll");

    assert_eq!(poll.source_health, LibraryChangeSourceHealth::Failed);
    assert_eq!(
        poll.planning.freshness,
        CatalogFreshnessState::NeedsReconciliation
    );
}

#[test]
fn repeated_runtime_failures_increase_backoff_until_health_is_stable() {
    let stops = Arc::new(Mutex::new(0));
    let failed_source = || FakeSource {
        health: LibraryChangeSourceHealth::Healthy,
        batches: VecDeque::from([Err(LibraryChangeSourceError::retryable(
            "fake_callback_failed",
            "callback failed",
        ))]),
        stops: Arc::clone(&stops),
        stop_delay: Duration::ZERO,
        should_fail_stop: false,
    };
    let mut observer = LibraryChangeObserver::start(
        factory([
            Ok(failed_source()),
            Ok(failed_source()),
            Ok(failed_source()),
        ]),
        request(),
        limits(),
        restart_policy(),
        0,
    )
    .expect("observer");

    let first = observer.poll(1).expect("first failure");
    assert_eq!(first.restart_attempt, 1);
    assert_eq!(first.next_restart_unix_ms, Some(251));
    wait_for_stop(&stops);
    observer.poll(251).expect("first restart");
    wait_for_source_state(&mut observer, 251, |poll| {
        poll.source_health == LibraryChangeSourceHealth::Failed && poll.restart_attempt == 2
    });
    let second = observer.poll(252).expect("second failure state");
    assert_eq!(second.restart_attempt, 2);
    assert_eq!(second.next_restart_unix_ms, Some(751));

    wait_for_stop_count(&stops, 2);
    observer.poll(751).expect("second restart");
    wait_for_source_state(&mut observer, 751, |poll| {
        poll.source_health == LibraryChangeSourceHealth::Failed && poll.restart_attempt == 3
    });
    let third = observer.poll(752).expect("third failure state");
    assert_eq!(third.restart_attempt, 3);
    assert_eq!(third.next_restart_unix_ms, Some(1_751));
}

#[test]
fn stable_recovery_resets_backoff_after_two_healthy_polls() {
    let stops = Arc::new(Mutex::new(0));
    let failing_source = FakeSource {
        health: LibraryChangeSourceHealth::Healthy,
        batches: VecDeque::from([Err(LibraryChangeSourceError::retryable(
            "fake_callback_failed",
            "callback failed",
        ))]),
        stops: Arc::clone(&stops),
        stop_delay: Duration::ZERO,
        should_fail_stop: false,
    };
    let recovered_source = source(LibraryChangeSourceHealth::Healthy, Arc::clone(&stops));
    let mut observer = LibraryChangeObserver::start(
        factory([Ok(failing_source), Ok(recovered_source)]),
        request(),
        limits(),
        restart_policy(),
        0,
    )
    .expect("observer");

    let failed = observer.poll(1).expect("runtime failure");
    assert_eq!(failed.restart_attempt, 1);
    wait_for_stop(&stops);
    observer.poll(251).expect("restart poll");
    let first_healthy = wait_for_source_state(&mut observer, 251, |poll| {
        poll.source_health == LibraryChangeSourceHealth::Healthy
    });
    assert_eq!(first_healthy.restart_attempt, 1);

    let stable = observer.poll(252).expect("stable healthy poll");
    assert_eq!(stable.source_health, LibraryChangeSourceHealth::Healthy);
    assert_eq!(stable.restart_attempt, 0);
}

#[test]
fn slow_restart_start_does_not_block_poll() {
    let stops = Arc::new(Mutex::new(0));
    let failing_source = FakeSource {
        health: LibraryChangeSourceHealth::Healthy,
        batches: VecDeque::from([Err(LibraryChangeSourceError::retryable(
            "fake_callback_failed",
            "callback failed",
        ))]),
        stops: Arc::clone(&stops),
        stop_delay: Duration::ZERO,
        should_fail_stop: false,
    };
    let factory = factory([
        Ok(failing_source),
        Ok(source(
            LibraryChangeSourceHealth::Healthy,
            Arc::clone(&stops),
        )),
    ]);
    let mut observer =
        LibraryChangeObserver::start(factory.clone(), request(), limits(), restart_policy(), 0)
            .expect("observer");
    factory.state.lock().expect("factory state").start_delay = Duration::from_millis(500);

    observer.poll(1).expect("runtime failure");
    wait_for_stop(&stops);
    let started = Instant::now();
    let poll = observer.poll(251).expect("non-blocking restart poll");

    assert!(started.elapsed() < Duration::from_millis(200));
    assert_eq!(poll.source_health, LibraryChangeSourceHealth::Starting);
}

#[test]
fn stop_is_idempotent_and_prevents_future_polling_or_restart() {
    let stops = Arc::new(Mutex::new(0));
    let source = source(LibraryChangeSourceHealth::Healthy, Arc::clone(&stops));
    let mut observer = LibraryChangeObserver::start(
        factory([Ok(source)]),
        request(),
        limits(),
        restart_policy(),
        0,
    )
    .expect("observer");

    let first = observer.stop().expect("first stop");
    let second = observer.stop().expect("second stop");

    assert_eq!(first.elapsed_millis, 3);
    assert_eq!(second, LibraryChangeSourceStopReport::default());
    assert_eq!(*stops.lock().expect("stop state"), 1);
    assert_eq!(
        observer.poll(1).expect_err("stopped observer").code,
        "change_observer_stopped"
    );
}

#[test]
fn failed_stop_does_not_start_an_overlapping_watcher() {
    let stops = Arc::new(Mutex::new(0));
    let failed_source = FakeSource {
        health: LibraryChangeSourceHealth::Healthy,
        batches: VecDeque::from([Err(LibraryChangeSourceError::retryable(
            "fake_callback_failed",
            "callback failed",
        ))]),
        stops: Arc::clone(&stops),
        stop_delay: Duration::ZERO,
        should_fail_stop: true,
    };
    let replacement = source(LibraryChangeSourceHealth::Healthy, Arc::clone(&stops));
    let factory = factory([Ok(failed_source), Ok(replacement)]);
    let state = Arc::clone(&factory.state);
    let mut observer =
        LibraryChangeObserver::start(factory, request(), limits(), restart_policy(), 0)
            .expect("observer");

    observer.poll(1).expect("failure poll");
    wait_for_stop(&stops);
    let deadline = Instant::now() + Duration::from_secs(1);
    let stopped = loop {
        let poll = observer.poll(10_000).expect("advance failed stop");
        if poll.last_source_error_code.as_deref() == Some("fake_stop_failed") {
            break poll;
        }
        assert!(
            Instant::now() < deadline,
            "timed out waiting for stop failure"
        );
        thread::yield_now();
    };

    assert_eq!(stopped.source_health, LibraryChangeSourceHealth::Failed);
    assert_eq!(stopped.restart_attempt, 0);
    assert_eq!(stopped.next_restart_unix_ms, None);
    observer.poll(i64::MAX).expect("no restart poll");
    assert_eq!(state.lock().expect("factory state").starts, 1);
}

fn factory(
    outcomes: impl IntoIterator<Item = Result<FakeSource, LibraryChangeSourceError>>,
) -> FakeFactory {
    FakeFactory {
        state: Arc::new(Mutex::new(FakeFactoryState {
            starts: 0,
            start_delay: Duration::ZERO,
            outcomes: outcomes.into_iter().collect(),
        })),
    }
}

fn source(health: LibraryChangeSourceHealth, stops: Arc<Mutex<u32>>) -> FakeSource {
    FakeSource {
        health,
        batches: VecDeque::new(),
        stops,
        stop_delay: Duration::ZERO,
        should_fail_stop: false,
    }
}

fn request() -> LibraryChangeSourceRequest {
    LibraryChangeSourceRequest {
        root_id: "root-a".to_owned(),
        root_generation: LibraryRootGeneration::new(7).expect("generation"),
        root_path: PathBuf::from(r"C:\library"),
        ingress_capacity: 16,
    }
}

fn limits() -> LibraryChangePlanningLimits {
    LibraryChangePlanningLimits {
        max_observations: 16,
        max_intents: 16,
    }
}

fn restart_policy() -> LibraryChangeRestartPolicy {
    LibraryChangeRestartPolicy {
        initial_delay_millis: 250,
        maximum_delay_millis: 1_000,
    }
}

fn observation(relative_path: &str) -> LibraryChangeObservation {
    LibraryChangeObservation {
        root_id: "root-a".to_owned(),
        root_generation: LibraryRootGeneration::new(7).expect("generation"),
        sequence: 1,
        observed_unix_ms: 1_000,
        kind: LibraryChangeObservationKind::Modified,
        scope: LibraryChangeScope::Path,
        relative_path: relative_path.to_owned(),
        previous_relative_path: None,
        origin: LibraryChangeOrigin::LiveNotification,
    }
}

fn wait_for_stop(stops: &Arc<Mutex<u32>>) -> u32 {
    let deadline = Instant::now() + Duration::from_secs(1);
    while Instant::now() < deadline {
        let count = *stops.lock().expect("stop state");
        if count > 0 {
            thread::sleep(Duration::from_millis(10));
            return count;
        }
        thread::yield_now();
    }
    panic!("timed out waiting for fake source stop");
}

fn wait_for_stop_count(stops: &Arc<Mutex<u32>>, expected: u32) {
    let deadline = Instant::now() + Duration::from_secs(1);
    while Instant::now() < deadline {
        if *stops.lock().expect("stop state") >= expected {
            return;
        }
        thread::yield_now();
    }
    panic!("timed out waiting for {expected} fake source stops");
}

fn wait_for_source_state(
    observer: &mut LibraryChangeObserver<FakeFactory>,
    now_unix_ms: i64,
    predicate: impl Fn(&crate::domain::LibraryChangeObserverPoll) -> bool,
) -> crate::domain::LibraryChangeObserverPoll {
    let deadline = Instant::now() + Duration::from_secs(1);
    loop {
        let poll = observer.poll(now_unix_ms).expect("advance observer state");
        if predicate(&poll) {
            return poll;
        }
        assert!(
            Instant::now() < deadline,
            "timed out waiting for source state"
        );
        thread::yield_now();
    }
}
