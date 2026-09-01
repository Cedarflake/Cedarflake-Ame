use std::sync::mpsc::{Receiver, RecvTimeoutError, TryRecvError, sync_channel};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use crate::domain::{
    LibraryChangeObservation, LibraryChangeObservationKind, LibraryChangeObserverPoll,
    LibraryChangeOrigin, LibraryChangePlanningContext, LibraryChangePlanningLimits,
    LibraryChangeRestartPolicy, LibraryChangeScope, LibraryChangeSourceError,
    LibraryChangeSourceHealth, LibraryChangeSourceStopReport, LibraryRootAvailability,
};
use crate::ports::{
    BoxedLibraryChangeSource, LibraryChangeSourceRequest, LibraryChangeSourceStarter,
};
#[cfg(test)]
use crate::ports::{LibraryChangeSourceFactory, erase_library_change_source_factory};

use super::plan_library_changes;

const STOP_TASK_TIMEOUT: Duration = Duration::from_secs(2);
const HEALTHY_POLLS_BEFORE_RESTART_RESET: u32 = 2;

type StartResult = Result<BoxedLibraryChangeSource, LibraryChangeSourceError>;
type StopResult = Result<LibraryChangeSourceStopReport, LibraryChangeSourceError>;

struct ObserverTask<T> {
    receiver: Receiver<T>,
    worker: Option<JoinHandle<()>>,
    completion: Option<T>,
}

pub(crate) struct LibraryChangeObserver {
    start_source: LibraryChangeSourceStarter,
    request: LibraryChangeSourceRequest,
    limits: LibraryChangePlanningLimits,
    restart_policy: LibraryChangeRestartPolicy,
    source: Option<BoxedLibraryChangeSource>,
    start_task: Option<ObserverTask<StartResult>>,
    stop_task: Option<ObserverTask<StopResult>>,
    source_health: LibraryChangeSourceHealth,
    restart_attempt: u32,
    next_restart_unix_ms: Option<i64>,
    last_source_error_code: Option<String>,
    healthy_poll_streak: u32,
    is_stopped: bool,
}

impl LibraryChangeObserver {
    #[cfg(test)]
    pub(crate) fn start<Factory>(
        factory: Factory,
        request: LibraryChangeSourceRequest,
        limits: LibraryChangePlanningLimits,
        restart_policy: LibraryChangeRestartPolicy,
        now_unix_ms: i64,
    ) -> Result<Self, LibraryChangeSourceError>
    where
        Factory: LibraryChangeSourceFactory,
    {
        Self::start_erased(
            erase_library_change_source_factory(factory),
            request,
            limits,
            restart_policy,
            now_unix_ms,
        )
    }

    pub(crate) fn start_erased(
        start_source: LibraryChangeSourceStarter,
        request: LibraryChangeSourceRequest,
        limits: LibraryChangePlanningLimits,
        restart_policy: LibraryChangeRestartPolicy,
        now_unix_ms: i64,
    ) -> Result<Self, LibraryChangeSourceError> {
        validate_restart_policy(restart_policy)?;
        validate_planning_limits(limits)?;
        let mut observer = Self {
            start_source,
            request,
            limits,
            restart_policy,
            source: None,
            start_task: None,
            stop_task: None,
            source_health: LibraryChangeSourceHealth::Starting,
            restart_attempt: 0,
            next_restart_unix_ms: None,
            last_source_error_code: None,
            healthy_poll_streak: 0,
            is_stopped: false,
        };
        observer.try_start_initial(now_unix_ms)?;
        Ok(observer)
    }

    pub(crate) fn poll(
        &mut self,
        now_unix_ms: i64,
    ) -> Result<LibraryChangeObserverPoll, LibraryChangeSourceError> {
        if self.is_stopped {
            return Err(LibraryChangeSourceError::new(
                "change_observer_stopped",
                "The library observer has already stopped.",
            ));
        }
        self.advance_start(now_unix_ms);
        self.advance_stop();
        self.advance_start(now_unix_ms);
        if self.source.is_none()
            && self.start_task.is_none()
            && self.stop_task.is_none()
            && self
                .next_restart_unix_ms
                .is_some_and(|deadline| now_unix_ms >= deadline)
            && let Err(error) = self.begin_start_source()
        {
            self.source_health = LibraryChangeSourceHealth::Failed;
            self.last_source_error_code = Some(error.code);
            if error.is_retryable {
                self.schedule_restart(now_unix_ms);
            } else {
                self.next_restart_unix_ms = None;
            }
        }

        let mut observations = Vec::new();
        let mut dropped_observation_count = 0;
        let mut ignored_callback_count = 0;
        let mut should_restart = false;
        if let Some(source) = self.source.as_mut() {
            match source.drain(self.limits.max_observations) {
                Ok(batch) => {
                    dropped_observation_count = batch.dropped_observation_count;
                    ignored_callback_count = batch.ignored_callback_count;
                    self.source_health = batch.health;
                    if let Some(code) = batch.last_issue_code {
                        self.last_source_error_code = Some(code);
                    }
                    observations = batch.observations;
                    if dropped_observation_count > 0
                        && !observations.iter().any(|observation| {
                            observation.kind == LibraryChangeObservationKind::EvidenceGap
                        })
                    {
                        let sequence = observations
                            .iter()
                            .map(|observation| observation.sequence)
                            .max()
                            .unwrap_or(0)
                            .saturating_add(1);
                        observations.push(LibraryChangeObservation {
                            root_id: self.request.root_id.clone(),
                            root_generation: self.request.root_generation,
                            sequence,
                            observed_unix_ms: now_unix_ms,
                            kind: LibraryChangeObservationKind::EvidenceGap,
                            scope: LibraryChangeScope::Root,
                            relative_path: String::new(),
                            previous_relative_path: None,
                            origin: LibraryChangeOrigin::LiveNotification,
                        });
                    }
                    should_restart = matches!(
                        self.source_health,
                        LibraryChangeSourceHealth::Degraded | LibraryChangeSourceHealth::Failed
                    );
                    if matches!(self.source_health, LibraryChangeSourceHealth::Healthy) {
                        self.healthy_poll_streak = self.healthy_poll_streak.saturating_add(1);
                        if self.healthy_poll_streak >= HEALTHY_POLLS_BEFORE_RESTART_RESET {
                            self.restart_attempt = 0;
                        }
                    } else {
                        self.healthy_poll_streak = 0;
                    }
                }
                Err(error) => {
                    self.healthy_poll_streak = 0;
                    self.source_health = LibraryChangeSourceHealth::Failed;
                    self.last_source_error_code = Some(error.code);
                    should_restart = true;
                }
            }
        }
        if should_restart {
            self.schedule_restart(now_unix_ms);
            self.begin_stop_source()?;
        }

        let planning = plan_library_changes(
            &LibraryChangePlanningContext {
                root_id: self.request.root_id.clone(),
                root_generation: self.request.root_generation,
                availability: LibraryRootAvailability::Available,
                source_health: self.source_health,
            },
            observations,
            self.limits,
        )
        .map_err(|error| LibraryChangeSourceError::new(error.code, error.message))?;

        Ok(LibraryChangeObserverPoll {
            planning,
            source_health: self.source_health,
            restart_attempt: self.restart_attempt,
            next_restart_unix_ms: self.next_restart_unix_ms,
            dropped_observation_count,
            ignored_callback_count,
            last_source_error_code: self.last_source_error_code.clone(),
        })
    }

    pub(crate) fn stop(
        &mut self,
    ) -> Result<LibraryChangeSourceStopReport, LibraryChangeSourceError> {
        self.request_stop()?;
        self.finish_stop_until(Instant::now() + STOP_TASK_TIMEOUT)
    }

    pub(crate) fn request_stop(&mut self) -> Result<(), LibraryChangeSourceError> {
        self.is_stopped = true;
        self.next_restart_unix_ms = None;
        self.source_health = LibraryChangeSourceHealth::Stopped;
        if self.start_task.is_none() && self.stop_task.is_none() && self.source.is_some() {
            self.begin_stop_source()?;
        }
        Ok(())
    }

    pub(crate) fn finish_stop_until(
        &mut self,
        deadline: Instant,
    ) -> Result<LibraryChangeSourceStopReport, LibraryChangeSourceError> {
        self.request_stop()?;
        if let Some(task) = self.start_task.as_mut() {
            finish_observer_task_until(
                task,
                deadline,
                "change_observer_start_timeout",
                "The library observer start task did not finish within the bounded interval.",
                || {
                    Err(LibraryChangeSourceError::retryable(
                        "change_observer_start_disconnected",
                        "The library observer start task disconnected unexpectedly.",
                    ))
                },
            )?;
        }
        if let Some(mut task) = self.start_task.take() {
            match task
                .completion
                .take()
                .expect("joined start task completion")
            {
                Ok(source) => self.source = Some(source),
                Err(error) => self.last_source_error_code = Some(error.code),
            }
        }
        if self.stop_task.is_none() && self.source.is_some() {
            self.begin_stop_source()?;
        }
        if let Some(task) = self.stop_task.as_mut() {
            finish_observer_task_until(
                task,
                deadline,
                "change_observer_stop_timeout",
                "The library observer stop task did not finish within the bounded interval.",
                || {
                    Err(LibraryChangeSourceError::retryable(
                        "change_observer_stop_disconnected",
                        "The library observer stop task disconnected unexpectedly.",
                    ))
                },
            )?;
        }
        let Some(mut task) = self.stop_task.take() else {
            return Ok(LibraryChangeSourceStopReport::default());
        };
        task.completion.take().expect("joined stop task completion")
    }

    pub(crate) fn try_finish_stop(
        &mut self,
    ) -> Result<Option<LibraryChangeSourceStopReport>, LibraryChangeSourceError> {
        self.request_stop()?;
        if let Some(task) = self.start_task.as_mut()
            && !try_finish_observer_task(task, || {
                Err(LibraryChangeSourceError::retryable(
                    "change_observer_start_disconnected",
                    "The library observer start task disconnected unexpectedly.",
                ))
            })?
        {
            return Ok(None);
        }
        if let Some(mut task) = self.start_task.take() {
            match task
                .completion
                .take()
                .expect("joined start task completion")
            {
                Ok(source) => self.source = Some(source),
                Err(error) => self.last_source_error_code = Some(error.code),
            }
        }
        if self.stop_task.is_none() && self.source.is_some() {
            self.begin_stop_source()?;
        }
        let Some(task) = self.stop_task.as_mut() else {
            return Ok(Some(LibraryChangeSourceStopReport::default()));
        };
        if !try_finish_observer_task(task, || {
            Err(LibraryChangeSourceError::retryable(
                "change_observer_stop_disconnected",
                "The library observer stop task disconnected unexpectedly.",
            ))
        })? {
            return Ok(None);
        }
        let mut task = self.stop_task.take().expect("joined stop task");
        task.completion
            .take()
            .expect("joined stop task completion")
            .map(Some)
    }

    fn try_start_initial(&mut self, now_unix_ms: i64) -> Result<(), LibraryChangeSourceError> {
        self.source_health = LibraryChangeSourceHealth::Starting;
        match (self.start_source)(&self.request) {
            Ok(source) => {
                self.source_health = source.health();
                self.source = Some(source);
                self.next_restart_unix_ms = None;
                self.last_source_error_code = None;
                self.healthy_poll_streak = 0;
                Ok(())
            }
            Err(error) => {
                self.source_health = LibraryChangeSourceHealth::Failed;
                self.last_source_error_code = Some(error.code.clone());
                if error.is_retryable {
                    self.schedule_restart(now_unix_ms);
                    Ok(())
                } else {
                    Err(error)
                }
            }
        }
    }

    fn begin_start_source(&mut self) -> Result<(), LibraryChangeSourceError> {
        let start_source = self.start_source.clone();
        let request = self.request.clone();
        let (sender, receiver) = sync_channel(1);
        let worker = thread::Builder::new()
            .name("ame-change-source-start".to_owned())
            .spawn(move || {
                let _ = sender.send(start_source(&request));
            })
            .map_err(|_| {
                LibraryChangeSourceError::retryable(
                    "change_observer_start_thread_failed",
                    "The library observer could not start its non-blocking restart task.",
                )
            })?;
        self.source_health = LibraryChangeSourceHealth::Starting;
        self.next_restart_unix_ms = None;
        self.start_task = Some(ObserverTask {
            receiver,
            worker: Some(worker),
            completion: None,
        });
        Ok(())
    }

    fn advance_start(&mut self, now_unix_ms: i64) {
        let Some(task) = self.start_task.as_mut() else {
            return;
        };
        if task.completion.is_none() {
            task.completion = match task.receiver.try_recv() {
                Ok(result) => Some(result),
                Err(TryRecvError::Empty) => None,
                Err(TryRecvError::Disconnected) => Some(Err(LibraryChangeSourceError::retryable(
                    "change_observer_start_disconnected",
                    "The library observer start task disconnected unexpectedly.",
                ))),
            };
        }
        if task.completion.is_none()
            || task
                .worker
                .as_ref()
                .is_some_and(|worker| !worker.is_finished())
        {
            return;
        }
        let mut task = self.start_task.take().expect("completed start task");
        if let Some(worker) = task.worker.take() {
            let _ = worker.join();
        }
        let completion = task.completion.take().expect("start task completion");
        match completion {
            Ok(source) => {
                self.source_health = source.health();
                self.source = Some(source);
                self.next_restart_unix_ms = None;
                self.last_source_error_code = None;
                self.healthy_poll_streak = 0;
            }
            Err(error) => {
                self.source_health = LibraryChangeSourceHealth::Failed;
                self.last_source_error_code = Some(error.code);
                self.healthy_poll_streak = 0;
                if error.is_retryable {
                    self.schedule_restart(now_unix_ms);
                } else {
                    self.next_restart_unix_ms = None;
                }
            }
        }
    }

    fn begin_stop_source(&mut self) -> Result<(), LibraryChangeSourceError> {
        let Some(mut source) = self.source.take() else {
            return Ok(());
        };
        let (sender, receiver) = sync_channel(1);
        let worker = thread::Builder::new()
            .name("ame-change-source-stop".to_owned())
            .spawn(move || {
                let _ = sender.send(source.stop());
            })
            .map_err(|_| {
                LibraryChangeSourceError::new(
                    "change_observer_stop_thread_failed",
                    "The library observer could not start its bounded stop task.",
                )
            })?;
        self.stop_task = Some(ObserverTask {
            receiver,
            worker: Some(worker),
            completion: None,
        });
        Ok(())
    }

    fn advance_stop(&mut self) {
        let Some(task) = self.stop_task.as_mut() else {
            return;
        };
        if task.completion.is_none() {
            task.completion = match task.receiver.try_recv() {
                Ok(result) => Some(result),
                Err(TryRecvError::Empty) => None,
                Err(TryRecvError::Disconnected) => Some(Err(LibraryChangeSourceError::retryable(
                    "change_observer_stop_disconnected",
                    "The library observer stop task disconnected unexpectedly.",
                ))),
            };
        }
        if task.completion.is_none()
            || task
                .worker
                .as_ref()
                .is_some_and(|worker| !worker.is_finished())
        {
            return;
        }
        let mut task = self.stop_task.take().expect("completed stop task");
        if let Some(worker) = task.worker.take() {
            let _ = worker.join();
        }
        let completion = task.completion.take().expect("stop task completion");
        if let Err(error) = completion {
            self.last_source_error_code = Some(error.code);
            self.source_health = LibraryChangeSourceHealth::Failed;
            self.restart_attempt = 0;
            self.next_restart_unix_ms = None;
        }
    }

    fn schedule_restart(&mut self, now_unix_ms: i64) {
        self.restart_attempt = self.restart_attempt.saturating_add(1);
        let exponent = self.restart_attempt.saturating_sub(1).min(63);
        let multiplier = 1_u64.checked_shl(exponent).unwrap_or(u64::MAX);
        let delay = self
            .restart_policy
            .initial_delay_millis
            .saturating_mul(multiplier)
            .min(self.restart_policy.maximum_delay_millis);
        self.next_restart_unix_ms =
            Some(now_unix_ms.saturating_add(i64::try_from(delay).unwrap_or(i64::MAX)));
    }
}

fn finish_observer_task_until<T>(
    task: &mut ObserverTask<T>,
    deadline: Instant,
    timeout_code: &'static str,
    timeout_message: &'static str,
    disconnected_completion: impl FnOnce() -> T,
) -> Result<(), LibraryChangeSourceError> {
    if task.completion.is_none() {
        let remaining = deadline
            .checked_duration_since(Instant::now())
            .ok_or_else(|| LibraryChangeSourceError::new(timeout_code, timeout_message))?;
        task.completion = match task.receiver.recv_timeout(remaining) {
            Ok(completion) => Some(completion),
            Err(RecvTimeoutError::Disconnected) => Some(disconnected_completion()),
            Err(RecvTimeoutError::Timeout) => {
                return Err(LibraryChangeSourceError::new(timeout_code, timeout_message));
            }
        };
    }
    if let Some(worker) = task.worker.as_ref() {
        while !worker.is_finished() {
            let remaining = deadline
                .checked_duration_since(Instant::now())
                .ok_or_else(|| LibraryChangeSourceError::new(timeout_code, timeout_message))?;
            thread::sleep(remaining.min(Duration::from_millis(2)));
        }
    }
    if let Some(worker) = task.worker.take() {
        worker.join().map_err(|_| {
            LibraryChangeSourceError::new(
                "change_observer_task_panicked",
                "The library observer task panicked during shutdown.",
            )
        })?;
    }
    Ok(())
}

fn try_finish_observer_task<T>(
    task: &mut ObserverTask<T>,
    disconnected_completion: impl FnOnce() -> T,
) -> Result<bool, LibraryChangeSourceError> {
    if task.completion.is_none() {
        task.completion = match task.receiver.try_recv() {
            Ok(completion) => Some(completion),
            Err(TryRecvError::Empty) => return Ok(false),
            Err(TryRecvError::Disconnected) => Some(disconnected_completion()),
        };
    }
    if task
        .worker
        .as_ref()
        .is_some_and(|worker| !worker.is_finished())
    {
        return Ok(false);
    }
    if let Some(worker) = task.worker.take() {
        worker.join().map_err(|_| {
            LibraryChangeSourceError::new(
                "change_observer_task_panicked",
                "The library observer task panicked during shutdown.",
            )
        })?;
    }
    Ok(true)
}

fn validate_restart_policy(
    policy: LibraryChangeRestartPolicy,
) -> Result<(), LibraryChangeSourceError> {
    if policy.initial_delay_millis == 0
        || policy.maximum_delay_millis == 0
        || policy.initial_delay_millis > policy.maximum_delay_millis
    {
        return Err(LibraryChangeSourceError::new(
            "change_observer_restart_policy_invalid",
            "The library observer restart policy must have positive ordered bounds.",
        ));
    }
    Ok(())
}

fn validate_planning_limits(
    limits: LibraryChangePlanningLimits,
) -> Result<(), LibraryChangeSourceError> {
    if limits.max_observations == 0
        || limits.max_observations > LibraryChangePlanningLimits::MAX_OBSERVATIONS
        || limits.max_intents == 0
        || limits.max_intents > LibraryChangePlanningLimits::MAX_INTENTS
    {
        return Err(LibraryChangeSourceError::new(
            "change_observer_planning_limits_invalid",
            "The library observer planning limits must stay within the supported bounds.",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests;
