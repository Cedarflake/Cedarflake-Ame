use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Clone)]
pub(super) struct ServiceLifetime {
    state: Arc<Mutex<State>>,
    idle_timeout: Duration,
    max_connections: u32,
}

enum State {
    Accepting {
        connections: u32,
        idle_since: Instant,
    },
    Retiring,
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum AdmissionError {
    Poisoned,
    Capacity,
}

pub(super) struct ConnectionLease {
    state: Arc<Mutex<State>>,
}

impl ServiceLifetime {
    pub(super) fn new(now: Instant, idle_timeout: Duration, max_connections: u32) -> Self {
        Self {
            state: Arc::new(Mutex::new(State::Accepting {
                connections: 0,
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

impl Drop for ConnectionLease {
    fn drop(&mut self) {
        if let Ok(mut state) = self.state.lock()
            && let State::Accepting {
                connections,
                idle_since,
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
