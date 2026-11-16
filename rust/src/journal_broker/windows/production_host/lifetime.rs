use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

use crate::journal_broker::service::AuthenticatedPrincipal;

#[derive(Clone)]
pub(in crate::journal_broker::windows) struct ServiceLifetime {
    shared: Arc<SharedState>,
    idle_timeout: Duration,
    max_connections: u32,
}

struct SharedState {
    state: Mutex<State>,
    principal_released: Condvar,
}

enum State {
    Accepting {
        connections: u32,
        principals: HashSet<AuthenticatedPrincipal>,
        idle_since: Instant,
    },
    Retiring,
}

#[derive(Debug, PartialEq, Eq)]
pub(in crate::journal_broker::windows) enum AdmissionError {
    Poisoned,
    Capacity,
    PrincipalBusy,
    Stopped,
    Retired,
}

pub(in crate::journal_broker::windows) struct ConnectionLease {
    shared: Arc<SharedState>,
}

pub(in crate::journal_broker::windows) struct PrincipalLease<'connection> {
    connection: &'connection ConnectionLease,
    principal: AuthenticatedPrincipal,
}

impl ServiceLifetime {
    pub(in crate::journal_broker::windows) fn new(
        now: Instant,
        idle_timeout: Duration,
        max_connections: u32,
    ) -> Self {
        Self {
            shared: Arc::new(SharedState {
                state: Mutex::new(State::Accepting {
                    connections: 0,
                    principals: HashSet::new(),
                    idle_since: now,
                }),
                principal_released: Condvar::new(),
            }),
            idle_timeout,
            max_connections,
        }
    }

    pub(in crate::journal_broker::windows) fn admit(
        &self,
    ) -> Result<Option<ConnectionLease>, AdmissionError> {
        let mut state = self
            .shared
            .state
            .lock()
            .map_err(|_| AdmissionError::Poisoned)?;
        match &mut *state {
            State::Retiring => Ok(None),
            State::Accepting { connections, .. } => {
                if *connections >= self.max_connections {
                    return Err(AdmissionError::Capacity);
                }
                *connections += 1;
                Ok(Some(ConnectionLease {
                    shared: Arc::clone(&self.shared),
                }))
            }
        }
    }

    pub(in crate::journal_broker::windows) fn retire_if_idle(
        &self,
        now: Instant,
    ) -> Result<bool, AdmissionError> {
        let mut state = self
            .shared
            .state
            .lock()
            .map_err(|_| AdmissionError::Poisoned)?;
        match &*state {
            State::Retiring => Ok(true),
            State::Accepting {
                connections,
                idle_since,
                ..
            } => {
                if *connections != 0
                    || now.saturating_duration_since(*idle_since) < self.idle_timeout
                {
                    return Ok(false);
                }
                *state = State::Retiring;
                Ok(true)
            }
        }
    }
}

impl ConnectionLease {
    pub(in crate::journal_broker::windows) fn bind_principal(
        &mut self,
        principal: AuthenticatedPrincipal,
        deadline: Instant,
        stop_requested: Option<&AtomicBool>,
    ) -> Result<PrincipalLease<'_>, AdmissionError> {
        let mut state = self
            .shared
            .state
            .lock()
            .map_err(|_| AdmissionError::Poisoned)?;
        loop {
            if stop_requested.is_some_and(|stop| stop.load(Ordering::Acquire)) {
                return Err(AdmissionError::Stopped);
            }
            match &mut *state {
                State::Retiring => return Err(AdmissionError::Retired),
                State::Accepting { principals, .. } => {
                    if principals.insert(principal) {
                        return Ok(PrincipalLease {
                            connection: self,
                            principal,
                        });
                    }
                }
            }
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err(AdmissionError::PrincipalBusy);
            }
            (state, _) = self
                .shared
                .principal_released
                .wait_timeout(state, remaining.min(Duration::from_millis(25)))
                .map_err(|_| AdmissionError::Poisoned)?;
        }
    }
}

impl Drop for PrincipalLease<'_> {
    fn drop(&mut self) {
        if let Ok(mut state) = self.connection.shared.state.lock()
            && let State::Accepting { principals, .. } = &mut *state
        {
            principals.remove(&self.principal);
            self.connection.shared.principal_released.notify_all();
        }
    }
}

impl Drop for ConnectionLease {
    fn drop(&mut self) {
        if let Ok(mut state) = self.shared.state.lock()
            && let State::Accepting {
                connections,
                idle_since,
                ..
            } = &mut *state
        {
            *connections -= 1;
            if *connections == 0 {
                *idle_since = Instant::now();
            }
        }
    }
}

#[cfg(test)]
mod tests;
