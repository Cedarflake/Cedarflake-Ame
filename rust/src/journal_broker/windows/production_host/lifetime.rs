use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::journal_broker::service::AuthenticatedPrincipal;

#[derive(Clone)]
pub(super) struct ServiceLifetime {
    state: Arc<Mutex<State>>,
    idle_timeout: Duration,
    max_connections: u32,
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
pub(super) enum AdmissionError {
    Poisoned,
    Capacity,
    AlreadyBound,
    Retired,
}

pub(super) struct ConnectionLease {
    state: Arc<Mutex<State>>,
    principal: Option<AuthenticatedPrincipal>,
}

impl ServiceLifetime {
    pub(super) fn new(now: Instant, idle_timeout: Duration, max_connections: u32) -> Self {
        Self {
            state: Arc::new(Mutex::new(State::Accepting {
                connections: 0,
                principals: HashSet::new(),
                idle_since: now,
            })),
            idle_timeout,
            max_connections,
        }
    }

    pub(super) fn admit(&self) -> Result<Option<ConnectionLease>, AdmissionError> {
        let mut state = self.state.lock().map_err(|_| AdmissionError::Poisoned)?;
        match &mut *state {
            State::Retiring => Ok(None),
            State::Accepting { connections, .. } => {
                if *connections >= self.max_connections {
                    return Err(AdmissionError::Capacity);
                }
                *connections += 1;
                Ok(Some(ConnectionLease {
                    state: Arc::clone(&self.state),
                    principal: None,
                }))
            }
        }
    }

    pub(super) fn retire_if_idle(&self, now: Instant) -> Result<bool, AdmissionError> {
        let mut state = self.state.lock().map_err(|_| AdmissionError::Poisoned)?;
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
    pub(super) fn bind_principal(
        &mut self,
        principal: AuthenticatedPrincipal,
    ) -> Result<bool, AdmissionError> {
        if self.principal.is_some() {
            return Err(AdmissionError::AlreadyBound);
        }
        let mut state = self.state.lock().map_err(|_| AdmissionError::Poisoned)?;
        match &mut *state {
            State::Retiring => Err(AdmissionError::Retired),
            State::Accepting { principals, .. } => {
                if principals.contains(&principal) {
                    return Ok(false);
                }
                principals.insert(principal);
                self.principal = Some(principal);
                Ok(true)
            }
        }
    }
}

impl Drop for ConnectionLease {
    fn drop(&mut self) {
        if let Ok(mut state) = self.state.lock()
            && let State::Accepting {
                connections,
                principals,
                idle_since,
            } = &mut *state
        {
            *connections -= 1;
            if let Some(principal) = self.principal {
                principals.remove(&principal);
            }
            if *connections == 0 {
                *idle_since = Instant::now();
            }
        }
    }
}

#[cfg(test)]
mod tests;
