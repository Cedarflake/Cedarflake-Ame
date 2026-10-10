use std::collections::{HashMap, VecDeque};
use std::sync::Mutex;
use std::time::Instant;

use crate::journal_broker::wire::{ReadJournalRangeRequest, ReadJournalVolumeRequest};

pub(crate) const MAX_ACTIVE_REQUESTS_PER_CONNECTION: usize = 8;
pub(crate) const MAX_CONTROL_REQUESTS_PER_CONNECTION: usize = 2;
const MAX_RECORDED_TERMINALS_PER_CONNECTION: usize = 16;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct ActiveRequestKey {
    pub(super) connection_id: [u8; 16],
    pub(super) connection_generation: u64,
    pub(super) authentication_namespace: [u8; 32],
    pub(super) client_instance: [u8; 16],
    pub(super) request_id: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct RequestNamespace {
    authentication_namespace: [u8; 32],
    client_instance: [u8; 16],
    request_id: u64,
}

impl From<ActiveRequestKey> for RequestNamespace {
    fn from(key: ActiveRequestKey) -> Self {
        Self {
            authentication_namespace: key.authentication_namespace,
            client_instance: key.client_instance,
            request_id: key.request_id,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ActiveRequestError {
    AlreadyActive,
    LimitExceeded,
    NotActive,
    ConnectionNotOpen,
    ConnectionGenerationMismatch,
    TerminalDeliveryBackpressure,
    Cancelled,
    TimedOut,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RequestTerminal {
    Completed,
    Cancelled,
    TimedOut,
    Aborted,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct RecordedTerminal {
    pub(super) key: ActiveRequestKey,
    pub(super) terminal: RequestTerminal,
}

pub(super) struct TerminalDrain {
    pub(super) terminals: Vec<RecordedTerminal>,
    pub(super) must_close: bool,
}

pub(super) struct AcceptedRead {
    pub(super) request: AcceptedJournalRead,
    pub(super) deadline: Instant,
}

#[derive(Clone)]
pub(crate) enum AcceptedJournalRead {
    Root(ReadJournalRangeRequest),
    Volume(ReadJournalVolumeRequest),
}

struct ActiveRequest {
    deadline: Instant,
    is_cancelled: bool,
    accepted_read: Option<AcceptedJournalRead>,
}

struct ConnectionLifecycle {
    generation: u64,
    requests: HashMap<RequestNamespace, ActiveRequest>,
    active_controls: usize,
    recorded_terminals: VecDeque<RecordedTerminal>,
    must_close: bool,
}

impl ConnectionLifecycle {
    fn new(generation: u64) -> Self {
        Self {
            generation,
            requests: HashMap::new(),
            active_controls: 0,
            recorded_terminals: VecDeque::new(),
            must_close: false,
        }
    }

    fn record(
        &mut self,
        key: ActiveRequestKey,
        terminal: RequestTerminal,
    ) -> Result<(), ActiveRequestError> {
        if self.recorded_terminals.len() == MAX_RECORDED_TERMINALS_PER_CONNECTION {
            self.must_close = true;
            return Err(ActiveRequestError::TerminalDeliveryBackpressure);
        }
        self.recorded_terminals
            .push_back(RecordedTerminal { key, terminal });
        if self.recorded_terminals.len() == MAX_RECORDED_TERMINALS_PER_CONNECTION {
            self.must_close = true;
        }
        Ok(())
    }
}

#[derive(Default)]
struct RegistryState {
    connections: HashMap<[u8; 16], ConnectionLifecycle>,
}

#[derive(Default)]
pub(crate) struct ActiveRequestRegistry {
    state: Mutex<RegistryState>,
}

impl ActiveRequestRegistry {
    pub(super) fn open_connection(
        &self,
        connection_id: [u8; 16],
        generation: u64,
    ) -> Result<(), ActiveRequestError> {
        if connection_id == [0; 16] || generation == 0 {
            return Err(ActiveRequestError::ConnectionNotOpen);
        }
        let mut state = self
            .state
            .lock()
            .map_err(|_| ActiveRequestError::ConnectionNotOpen)?;
        match state.connections.get(&connection_id) {
            Some(connection) if connection.generation == generation => Ok(()),
            Some(_) => Err(ActiveRequestError::ConnectionGenerationMismatch),
            None => {
                state
                    .connections
                    .insert(connection_id, ConnectionLifecycle::new(generation));
                Ok(())
            }
        }
    }

    pub(super) fn begin_query(
        &self,
        key: ActiveRequestKey,
        deadline: Instant,
    ) -> Result<(), ActiveRequestError> {
        self.register(key, deadline, None)
    }

    pub(super) fn accept_read(
        &self,
        key: ActiveRequestKey,
        deadline: Instant,
        request: ReadJournalRangeRequest,
    ) -> Result<(), ActiveRequestError> {
        self.register(key, deadline, Some(AcceptedJournalRead::Root(request)))
    }

    pub(super) fn accept_volume_read(
        &self,
        key: ActiveRequestKey,
        deadline: Instant,
        request: ReadJournalVolumeRequest,
    ) -> Result<(), ActiveRequestError> {
        self.register(key, deadline, Some(AcceptedJournalRead::Volume(request)))
    }

    fn register(
        &self,
        key: ActiveRequestKey,
        deadline: Instant,
        accepted_read: Option<AcceptedJournalRead>,
    ) -> Result<(), ActiveRequestError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| ActiveRequestError::LimitExceeded)?;
        let connection = connection_mut(&mut state, key)?;
        reap_accepted_locked(
            connection,
            key.connection_id,
            key.connection_generation,
            Instant::now(),
        );
        if connection.must_close {
            return Err(ActiveRequestError::TerminalDeliveryBackpressure);
        }
        let namespace = RequestNamespace::from(key);
        if connection.requests.contains_key(&namespace) {
            return Err(ActiveRequestError::AlreadyActive);
        }
        if connection.requests.len() >= MAX_ACTIVE_REQUESTS_PER_CONNECTION {
            return Err(ActiveRequestError::LimitExceeded);
        }
        connection.requests.insert(
            namespace,
            ActiveRequest {
                deadline,
                is_cancelled: false,
                accepted_read,
            },
        );
        Ok(())
    }

    pub(super) fn begin_control(
        &self,
        key: ActiveRequestKey,
    ) -> Result<ActiveControlGuard<'_>, ActiveRequestError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| ActiveRequestError::LimitExceeded)?;
        let connection = connection_mut(&mut state, key)?;
        if connection.active_controls >= MAX_CONTROL_REQUESTS_PER_CONNECTION {
            return Err(ActiveRequestError::LimitExceeded);
        }
        connection.active_controls += 1;
        Ok(ActiveControlGuard {
            registry: self,
            connection_id: key.connection_id,
            connection_generation: key.connection_generation,
            is_armed: true,
        })
    }

    pub(super) fn begin_execution(
        &self,
        key: ActiveRequestKey,
        now: Instant,
    ) -> Result<AcceptedRead, ActiveRequestError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| ActiveRequestError::NotActive)?;
        let connection = connection_mut(&mut state, key)?;
        let namespace = RequestNamespace::from(key);
        let Some(request) = connection.requests.get(&namespace) else {
            return Err(ActiveRequestError::NotActive);
        };
        if request.is_cancelled {
            connection.requests.remove(&namespace);
            return Err(ActiveRequestError::Cancelled);
        }
        if now >= request.deadline {
            connection.requests.remove(&namespace);
            return Err(ActiveRequestError::TimedOut);
        }
        let request = connection
            .requests
            .get_mut(&namespace)
            .ok_or(ActiveRequestError::NotActive)?;
        let deadline = request.deadline;
        let accepted = request
            .accepted_read
            .take()
            .ok_or(ActiveRequestError::NotActive)?;
        Ok(AcceptedRead {
            request: accepted,
            deadline,
        })
    }

    pub(super) fn abort_accepted(
        &self,
        key: ActiveRequestKey,
        now: Instant,
    ) -> Result<RecordedTerminal, ActiveRequestError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| ActiveRequestError::NotActive)?;
        let connection = connection_mut(&mut state, key)?;
        let namespace = RequestNamespace::from(key);
        let Some(request) = connection.requests.get(&namespace) else {
            return Err(ActiveRequestError::NotActive);
        };
        if request.accepted_read.is_none() {
            return Err(ActiveRequestError::NotActive);
        }
        let terminal = if request.is_cancelled {
            RequestTerminal::Cancelled
        } else if now >= request.deadline {
            RequestTerminal::TimedOut
        } else {
            RequestTerminal::Aborted
        };
        connection.requests.remove(&namespace);
        Ok(RecordedTerminal { key, terminal })
    }

    pub(super) fn reap_accepted(
        &self,
        connection_id: [u8; 16],
        connection_generation: u64,
        now: Instant,
    ) -> Result<TerminalDrain, ActiveRequestError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| ActiveRequestError::NotActive)?;
        let key = ActiveRequestKey {
            connection_id,
            connection_generation,
            authentication_namespace: [0; 32],
            client_instance: [0; 16],
            request_id: 0,
        };
        let connection = connection_mut(&mut state, key)?;
        reap_accepted_locked(connection, connection_id, connection_generation, now);
        let terminals = connection.recorded_terminals.drain(..).collect();
        Ok(TerminalDrain {
            terminals,
            must_close: connection.must_close,
        })
    }

    pub(super) fn cancel(&self, key: ActiveRequestKey) -> Result<(), ActiveRequestError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| ActiveRequestError::NotActive)?;
        let connection = connection_mut(&mut state, key)?;
        let request = connection
            .requests
            .get_mut(&RequestNamespace::from(key))
            .ok_or(ActiveRequestError::NotActive)?;
        request.is_cancelled = true;
        Ok(())
    }

    pub(super) fn check(
        &self,
        key: ActiveRequestKey,
        now: Instant,
    ) -> Result<(), ActiveRequestError> {
        let state = self
            .state
            .lock()
            .map_err(|_| ActiveRequestError::NotActive)?;
        let connection = connection_ref(&state, key)?;
        let request = connection
            .requests
            .get(&RequestNamespace::from(key))
            .ok_or(ActiveRequestError::NotActive)?;
        if request.is_cancelled {
            return Err(ActiveRequestError::Cancelled);
        }
        if now >= request.deadline {
            return Err(ActiveRequestError::TimedOut);
        }
        Ok(())
    }

    pub(super) fn finish(
        &self,
        key: ActiveRequestKey,
        now: Instant,
    ) -> Result<RequestTerminal, ActiveRequestError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| ActiveRequestError::NotActive)?;
        let connection = connection_mut(&mut state, key)?;
        let request = connection
            .requests
            .remove(&RequestNamespace::from(key))
            .ok_or(ActiveRequestError::NotActive)?;
        if request.is_cancelled {
            Ok(RequestTerminal::Cancelled)
        } else if now >= request.deadline {
            Ok(RequestTerminal::TimedOut)
        } else {
            Ok(RequestTerminal::Completed)
        }
    }

    pub(super) fn disconnect(&self, connection_id: [u8; 16], generation: u64) {
        if let Ok(mut state) = self.state.lock()
            && state
                .connections
                .get(&connection_id)
                .is_some_and(|connection| connection.generation == generation)
        {
            state.connections.remove(&connection_id);
        }
    }

    fn finish_control(&self, connection_id: [u8; 16], generation: u64) {
        if let Ok(mut state) = self.state.lock()
            && let Some(connection) = state.connections.get_mut(&connection_id)
            && connection.generation == generation
        {
            connection.active_controls = connection.active_controls.saturating_sub(1);
        }
    }

    #[cfg(test)]
    pub(super) fn active_count(&self, connection_id: [u8; 16]) -> usize {
        self.state.lock().map_or(0, |state| {
            state
                .connections
                .get(&connection_id)
                .map_or(0, |connection| connection.requests.len())
        })
    }

    #[cfg(test)]
    pub(super) fn accepted_count(&self, connection_id: [u8; 16]) -> usize {
        self.state.lock().map_or(0, |state| {
            state
                .connections
                .get(&connection_id)
                .map_or(0, |connection| {
                    connection
                        .requests
                        .values()
                        .filter(|request| request.accepted_read.is_some())
                        .count()
                })
        })
    }

    #[cfg(test)]
    pub(super) fn control_count(&self, connection_id: [u8; 16]) -> usize {
        self.state.lock().map_or(0, |state| {
            state
                .connections
                .get(&connection_id)
                .map_or(0, |connection| connection.active_controls)
        })
    }

    #[cfg(test)]
    pub(super) fn connection_count(&self) -> usize {
        self.state.lock().map_or(0, |state| state.connections.len())
    }
}

fn connection_ref(
    state: &RegistryState,
    key: ActiveRequestKey,
) -> Result<&ConnectionLifecycle, ActiveRequestError> {
    let connection = state
        .connections
        .get(&key.connection_id)
        .ok_or(ActiveRequestError::ConnectionNotOpen)?;
    if connection.generation != key.connection_generation {
        return Err(ActiveRequestError::ConnectionGenerationMismatch);
    }
    Ok(connection)
}

fn connection_mut(
    state: &mut RegistryState,
    key: ActiveRequestKey,
) -> Result<&mut ConnectionLifecycle, ActiveRequestError> {
    let connection = state
        .connections
        .get_mut(&key.connection_id)
        .ok_or(ActiveRequestError::ConnectionNotOpen)?;
    if connection.generation != key.connection_generation {
        return Err(ActiveRequestError::ConnectionGenerationMismatch);
    }
    Ok(connection)
}

fn reap_accepted_locked(
    connection: &mut ConnectionLifecycle,
    connection_id: [u8; 16],
    connection_generation: u64,
    now: Instant,
) -> Vec<RecordedTerminal> {
    let stale: Vec<_> = connection
        .requests
        .iter()
        .filter_map(|(namespace, request)| {
            let terminal = if request.accepted_read.is_none() {
                return None;
            } else if request.is_cancelled {
                RequestTerminal::Cancelled
            } else if now >= request.deadline {
                RequestTerminal::TimedOut
            } else {
                return None;
            };
            Some((*namespace, terminal))
        })
        .collect();
    let mut recorded = Vec::with_capacity(stale.len());
    for (namespace, terminal) in stale {
        let key = ActiveRequestKey {
            connection_id,
            connection_generation,
            authentication_namespace: namespace.authentication_namespace,
            client_instance: namespace.client_instance,
            request_id: namespace.request_id,
        };
        if connection.record(key, terminal).is_err() {
            break;
        }
        connection.requests.remove(&namespace);
        recorded.push(RecordedTerminal { key, terminal });
    }
    recorded
}

pub(super) struct ActiveControlGuard<'a> {
    registry: &'a ActiveRequestRegistry,
    connection_id: [u8; 16],
    connection_generation: u64,
    is_armed: bool,
}

impl Drop for ActiveControlGuard<'_> {
    fn drop(&mut self) {
        if self.is_armed {
            self.registry
                .finish_control(self.connection_id, self.connection_generation);
            self.is_armed = false;
        }
    }
}
