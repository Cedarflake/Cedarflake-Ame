use std::sync::mpsc::{Receiver, TryRecvError, sync_channel};
use std::thread;
use std::time::Duration;

use crate::domain::{
    LibraryChangeObserverPoll, LibraryChangePlanningContext, LibraryChangePlanningLimits,
    LibraryChangeRestartPolicy, LibraryChangeSourceError, LibraryChangeSourceHealth,
    LibraryChangeSourceStopReport, LibraryRootAvailability,
};
use crate::ports::{LibraryChangeSource, LibraryChangeSourceFactory, LibraryChangeSourceRequest};

use super::plan_library_changes;

const STOP_TASK_TIMEOUT: Duration = Duration::from_secs(2);
const HEALTHY_POLLS_BEFORE_RESTART_RESET: u32 = 2;

type StartResult<Source> = Result<Source, LibraryChangeSourceError>;

pub(crate) struct LibraryChangeObserver<Factory>
where
    Factory: LibraryChangeSourceFactory,
{
    factory: Factory,
    request: LibraryChangeSourceRequest,
    limits: LibraryChangePlanningLimits,
    restart_policy: LibraryChangeRestartPolicy,
    source: Option<Factory::Source>,
    start_receiver: Option<Receiver<StartResult<Factory::Source>>>,
    stop_receiver:
        Option<Receiver<Result<LibraryChangeSourceStopReport, LibraryChangeSourceError>>>,
    source_health: LibraryChangeSourceHealth,
    restart_attempt: u32,
    next_restart_unix_ms: Option<i64>,
    last_source_error_code: Option<String>,
    healthy_poll_streak: u32,
    is_stopped: bool,
}

impl<Factory> LibraryChangeObserver<Factory>
where
    Factory: LibraryChangeSourceFactory,
{
    pub(crate) fn start(
        factory: Factory,
        request: LibraryChangeSourceRequest,
        limits: LibraryChangePlanningLimits,
        restart_policy: LibraryChangeRestartPolicy,
        now_unix_ms: i64,
    ) -> Result<Self, LibraryChangeSourceError> {
        validate_restart_policy(restart_policy)?;
        validate_planning_limits(limits)?;
        let mut observer = Self {
            factory,
            request,
            limits,
            restart_policy,
            source: None,
            start_receiver: None,
            stop_receiver: None,
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
            && self.start_receiver.is_none()
            && self.stop_receiver.is_none()
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
                    observations = batch.observations;
                    if dropped_observation_count > 0
                        && matches!(
                            self.source_health,
                            LibraryChangeSourceHealth::Healthy
                                | LibraryChangeSourceHealth::Starting
                        )
                    {
                        self.source_health = LibraryChangeSourceHealth::Degraded;
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
        self.is_stopped = true;
        self.next_restart_unix_ms = None;
        self.source_health = LibraryChangeSourceHealth::Stopped;
        if let Some(receiver) = self.start_receiver.take() {
            return match receiver.recv_timeout(STOP_TASK_TIMEOUT).map_err(|_| {
                LibraryChangeSourceError::new(
                    "change_observer_start_timeout",
                    "The library observer start task did not finish within the bounded interval.",
                )
            })? {
                Ok(mut source) => source.stop(),
                Err(_) => Ok(LibraryChangeSourceStopReport::default()),
            };
        }
        if let Some(source) = self.source.as_mut() {
            let report = source.stop()?;
            self.source = None;
            return Ok(report);
        }
        if let Some(receiver) = self.stop_receiver.take() {
            return receiver.recv_timeout(STOP_TASK_TIMEOUT).map_err(|_| {
                LibraryChangeSourceError::new(
                    "change_observer_stop_timeout",
                    "The library observer stop task did not finish within the bounded interval.",
                )
            })?;
        }
        Ok(LibraryChangeSourceStopReport::default())
    }

    fn try_start_initial(&mut self, now_unix_ms: i64) -> Result<(), LibraryChangeSourceError> {
        self.source_health = LibraryChangeSourceHealth::Starting;
        match self.factory.start(&self.request) {
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
        let factory = self.factory.clone();
        let request = self.request.clone();
        let (sender, receiver) = sync_channel(1);
        thread::Builder::new()
            .name("ame-change-source-start".to_owned())
            .spawn(move || {
                let _ = sender.send(factory.start(&request));
            })
            .map_err(|_| {
                LibraryChangeSourceError::retryable(
                    "change_observer_start_thread_failed",
                    "The library observer could not start its non-blocking restart task.",
                )
            })?;
        self.source_health = LibraryChangeSourceHealth::Starting;
        self.next_restart_unix_ms = None;
        self.start_receiver = Some(receiver);
        Ok(())
    }

    fn advance_start(&mut self, now_unix_ms: i64) {
        let Some(receiver) = self.start_receiver.as_ref() else {
            return;
        };
        let completion = match receiver.try_recv() {
            Ok(result) => Some(result),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => Some(Err(LibraryChangeSourceError::retryable(
                "change_observer_start_disconnected",
                "The library observer start task disconnected unexpectedly.",
            ))),
        };
        let Some(completion) = completion else {
            return;
        };
        self.start_receiver = None;
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
        thread::Builder::new()
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
        self.stop_receiver = Some(receiver);
        Ok(())
    }

    fn advance_stop(&mut self) {
        let Some(receiver) = self.stop_receiver.as_ref() else {
            return;
        };
        let completion = match receiver.try_recv() {
            Ok(result) => Some(result),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => Some(Err(LibraryChangeSourceError::retryable(
                "change_observer_stop_disconnected",
                "The library observer stop task disconnected unexpectedly.",
            ))),
        };
        let Some(completion) = completion else {
            return;
        };
        self.stop_receiver = None;
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
