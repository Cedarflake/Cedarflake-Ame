use std::collections::{HashMap, HashSet};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU64, AtomicUsize, Ordering};
use std::sync::mpsc::{Receiver, SyncSender, TryRecvError, TrySendError, sync_channel};
use std::time::{Duration, Instant};

use thiserror::Error;

use super::framing::{BrokerFrameError, decode_frame, encode_frame};
use super::wire::{
    BrokerFailure, BrokerFailureCode, BrokerRequest, BrokerResponse, CallerClaim,
    QueryJournalRequest, ReadJournalRangeRequest, ReadJournalVolumeRequest, RegisterRootRequest,
    ResponseBinding, RootAuthorization, RootCapability, SharedJournalPendingRenameRequest,
    WireError, decode_response, encode_request,
};
use super::{
    PersistentChangeJournalOperationError, PersistentChangeJournalRead,
    PersistentChangeJournalSession,
};

const REGISTERED: u8 = 0;
const AWAITING_RESPONSE: u8 = 1;
const ACTIVE_READ: u8 = 2;
const CANCEL_IN_FLIGHT: u8 = 3;
const ABANDON_QUEUED: u8 = 4;
const TERMINAL: u8 = 5;
const CLOSE_UNCONFIRMED: u8 = 0;
const CLOSE_IN_PROGRESS: u8 = 1;
const CLOSE_CONFIRMED: u8 = 2;
const CLOSE_POISONED: u8 = 3;
const MAX_BUSINESS_REQUESTS_PER_CONNECTION: usize = 8;
const MAX_CONTROL_REQUESTS_PER_CONNECTION: usize = 2;
const TRANSPORT_POLL_TIMEOUT: Duration = Duration::from_millis(250);
const MAINTENANCE_STEPS_PER_OPERATION: usize = MAX_BUSINESS_REQUESTS_PER_CONNECTION;
const POLL_BACKOFF_MIN: Duration = Duration::from_micros(50);
const POLL_BACKOFF_MAX: Duration = Duration::from_millis(2);

pub(crate) mod sealed {
    pub trait Sealed {}
}

pub(crate) enum AdapterPoll<T> {
    Ready(T),
    Pending,
}

pub(crate) trait BrokerClientTransport: sealed::Sealed + Send + 'static {
    type Error: std::error::Error + Send + Sync + 'static;

    fn try_send_frame(&mut self, request_frame: &[u8]) -> Result<AdapterPoll<()>, Self::Error>;

    fn poll_response(&mut self, request_id: u64) -> Result<AdapterPoll<Vec<u8>>, Self::Error>;

    fn try_discard(&mut self, request_id: u64) -> Result<AdapterPoll<()>, Self::Error>;

    fn begin_abandon(
        &mut self,
        target_request_id: u64,
        cancel_request: Option<(u64, Vec<u8>)>,
        close_connection: bool,
    ) -> Result<AdapterPoll<()>, Self::Error>;

    fn poll_abandon(&mut self, target_request_id: u64) -> Result<AdapterPoll<()>, Self::Error>;

    fn begin_close(&mut self) -> Result<AdapterPoll<()>, Self::Error>;

    fn poll_close(&mut self) -> Result<AdapterPoll<()>, Self::Error>;
}

pub(crate) struct OwnedBrokerConnection<Transport> {
    transport: Transport,
    connection_id: [u8; 16],
    connection_generation: u64,
    connection_nonce: u64,
}

impl<Transport> OwnedBrokerConnection<Transport> {
    pub(in crate::journal_broker) fn from_adapter(
        transport: Transport,
        connection_id: [u8; 16],
        connection_generation: u64,
        connection_nonce: u64,
    ) -> Result<Self, BrokerClientError> {
        if connection_id == [0; 16] || connection_generation == 0 || connection_nonce == 0 {
            return Err(BrokerClientError::InvalidRequest);
        }
        Ok(Self {
            transport,
            connection_id,
            connection_generation,
            connection_nonce,
        })
    }
}

struct ClientDispatcher<Transport> {
    transport: Mutex<Transport>,
    connection_id: [u8; 16],
    connection_generation: u64,
    connection_nonce: u64,
    operation_epoch: AtomicU64,
    transport_poisoned: AtomicBool,
    poison_requested: AtomicBool,
    lifecycle_gate: Mutex<()>,
    next_request_id: AtomicU64,
    admission_closed: AtomicBool,
    transport_close_state: AtomicU8,
    business_count: AtomicUsize,
    control_count: AtomicUsize,
    requests: Mutex<HashMap<u64, Arc<RequestEntry>>>,
    cleanup_sender: SyncSender<CleanupTask>,
    cleanup_receiver: Mutex<Option<Receiver<CleanupTask>>>,
    #[cfg(test)]
    completion_pause: Mutex<Option<TestPause>>,
    #[cfg(test)]
    committed_pause: Mutex<Option<TestPause>>,
    #[cfg(test)]
    validation_pause: Mutex<Option<TestPause>>,
    #[cfg(test)]
    activation_pause: Mutex<Option<TestPause>>,
}

struct RequestEntry {
    lifecycle: AtomicU8,
    class: RequestClass,
    is_registered: AtomicBool,
    epoch: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RequestClass {
    Business,
    Control,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FinishOutcome {
    Committed,
    Superseded,
}

struct PollBackoff {
    delay: Duration,
}

impl PollBackoff {
    fn new() -> Self {
        Self {
            delay: POLL_BACKOFF_MIN,
        }
    }

    fn park(&mut self, deadline: Instant) {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return;
        }
        std::thread::park_timeout(self.delay.min(remaining));
        self.delay = self.delay.saturating_mul(2).min(POLL_BACKOFF_MAX);
    }
}

#[cfg(test)]
struct TestPause {
    entered: Arc<std::sync::Barrier>,
    release: Arc<std::sync::Barrier>,
}

#[cfg(test)]
pub(crate) struct ClientTestPause {
    entered: Arc<std::sync::Barrier>,
    release: Arc<std::sync::Barrier>,
}

#[cfg(test)]
impl ClientTestPause {
    pub(crate) fn wait_until_entered(&self) {
        self.entered.wait();
    }

    pub(crate) fn release(&self) {
        self.release.wait();
    }
}

struct CleanupTask {
    target_request_id: u64,
    caller: CallerClaim,
    entry: Arc<RequestEntry>,
    prior: u8,
    epoch: u64,
}

impl<Transport> ClientDispatcher<Transport>
where
    Transport: BrokerClientTransport,
{
    fn take_request_id(&self) -> Result<u64, BrokerClientError> {
        self.next_request_id
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |current| {
                current.checked_add(1)
            })
            .map_err(|_| BrokerClientError::RequestIdExhausted)
    }

    fn register_request(
        &self,
        class: RequestClass,
    ) -> Result<(u64, Arc<RequestEntry>), BrokerClientError> {
        if self.admission_closed.load(Ordering::Acquire) {
            return Err(BrokerClientError::Closed);
        }
        let (counter, limit) = self.class_capacity(class);
        counter
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |current| {
                (current < limit).then_some(current + 1)
            })
            .map_err(|_| BrokerClientError::PendingLimitExceeded)?;
        let mut requests = self.requests.lock().map_err(|_| {
            counter.fetch_sub(1, Ordering::AcqRel);
            BrokerClientError::PendingLimitExceeded
        })?;
        if self.admission_closed.load(Ordering::Acquire) {
            counter.fetch_sub(1, Ordering::AcqRel);
            return Err(BrokerClientError::Closed);
        }
        let request_id = match self.take_request_id() {
            Ok(request_id) => request_id,
            Err(error) => {
                counter.fetch_sub(1, Ordering::AcqRel);
                return Err(error);
            }
        };
        let entry = Arc::new(RequestEntry {
            lifecycle: AtomicU8::new(REGISTERED),
            class,
            is_registered: AtomicBool::new(true),
            epoch: self.operation_epoch.load(Ordering::Acquire),
        });
        requests.insert(request_id, Arc::clone(&entry));
        Ok((request_id, entry))
    }

    fn class_capacity(&self, class: RequestClass) -> (&AtomicUsize, usize) {
        match class {
            RequestClass::Business => (&self.business_count, MAX_BUSINESS_REQUESTS_PER_CONNECTION),
            RequestClass::Control => (&self.control_count, MAX_CONTROL_REQUESTS_PER_CONNECTION),
        }
    }

    fn retire_capacity(&self, entry: &RequestEntry) {
        if entry
            .is_registered
            .compare_exchange(true, false, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
        {
            self.class_capacity(entry.class)
                .0
                .fetch_sub(1, Ordering::AcqRel);
        }
    }

    fn remove_request_entry(&self, request_id: u64) {
        let mut requests = match self.requests.lock() {
            Ok(requests) => requests,
            Err(poisoned) => poisoned.into_inner(),
        };
        requests.remove(&request_id);
    }

    fn transition(lifecycle: &AtomicU8, from: u8, to: u8) -> Result<(), BrokerClientError> {
        lifecycle
            .compare_exchange(from, to, Ordering::AcqRel, Ordering::Acquire)
            .map(|_| ())
            .map_err(|_| BrokerClientError::RequestNotActive)
    }

    fn register_awaiting(
        &self,
        class: RequestClass,
    ) -> Result<(u64, Arc<RequestEntry>), BrokerClientError> {
        let (request_id, entry) = self.register_request(class)?;
        if let Err(error) = Self::transition(&entry.lifecycle, REGISTERED, AWAITING_RESPONSE) {
            self.finish_request(request_id, &entry);
            return Err(error);
        }
        Ok((request_id, entry))
    }

    fn activate_read(&self, entry: &RequestEntry) -> Result<(), BrokerClientError> {
        let _lifecycle = match self.lifecycle_gate.lock() {
            Ok(gate) => gate,
            Err(poisoned) => poisoned.into_inner(),
        };
        if self.poison_requested.load(Ordering::Acquire) {
            self.apply_poison_locked();
            self.clear_requests_locked();
            return Err(BrokerClientError::Closed);
        }
        if self.transport_poisoned.load(Ordering::Acquire)
            || self.operation_epoch.load(Ordering::Acquire) != entry.epoch
        {
            return Err(BrokerClientError::Closed);
        }
        entry
            .lifecycle
            .compare_exchange(
                AWAITING_RESPONSE,
                ACTIVE_READ,
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .map(|_| ())
            .map_err(|_| BrokerClientError::Closed)
    }

    fn finish_request(&self, request_id: u64, entry: &RequestEntry) -> FinishOutcome {
        #[cfg(test)]
        self.pause_before_completion_for_test();
        let _lifecycle = match self.lifecycle_gate.lock() {
            Ok(gate) => gate,
            Err(poisoned) => poisoned.into_inner(),
        };
        if self.poison_requested.load(Ordering::Acquire) {
            self.apply_poison_locked();
            self.clear_requests_locked();
        }
        if self.transport_poisoned.load(Ordering::Acquire)
            || self.operation_epoch.load(Ordering::Acquire) != entry.epoch
        {
            self.retire_capacity(entry);
            self.remove_request_entry(request_id);
            return FinishOutcome::Superseded;
        }
        let mut state = entry.lifecycle.load(Ordering::Acquire);
        loop {
            if state == TERMINAL {
                self.retire_capacity(entry);
                self.remove_request_entry(request_id);
                return FinishOutcome::Superseded;
            }
            match entry.lifecycle.compare_exchange(
                state,
                TERMINAL,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => break,
                Err(current) => state = current,
            }
        }
        self.retire_capacity(entry);
        self.remove_request_entry(request_id);
        drop(_lifecycle);
        #[cfg(test)]
        Self::run_test_pause(&self.committed_pause);
        match self.transport_step(entry.epoch, |transport| transport.try_discard(request_id)) {
            Ok(AdapterPoll::Ready(())) | Err(BrokerClientError::Closed) => {}
            Ok(AdapterPoll::Pending) | Err(_) => self.poison_transport(),
        }
        FinishOutcome::Committed
    }

    fn finish_result<ResultValue>(
        &self,
        request_id: u64,
        entry: &RequestEntry,
        result: Result<ResultValue, BrokerClientError>,
    ) -> Result<ResultValue, BrokerClientError> {
        match self.finish_request(request_id, entry) {
            FinishOutcome::Committed => result,
            FinishOutcome::Superseded => Err(BrokerClientError::Closed),
        }
    }

    fn encode_cancel(
        &self,
        caller: &CallerClaim,
        target_request_id: u64,
    ) -> Result<(u64, Arc<RequestEntry>, Vec<u8>), BrokerClientError> {
        let (request_id, entry) = self.register_awaiting(RequestClass::Control)?;
        let payload = match encode_request(&BrokerRequest::Cancel {
            request_id,
            caller: caller.clone(),
            target_request_id,
        })
        .map_err(map_local_wire)
        {
            Ok(payload) => payload,
            Err(error) => {
                self.finish_request(request_id, &entry);
                return Err(error);
            }
        };
        let frame = match encode_frame(&payload).map_err(BrokerClientError::Frame) {
            Ok(frame) => frame,
            Err(error) => {
                self.finish_request(request_id, &entry);
                return Err(error);
            }
        };
        Ok((request_id, entry, frame))
    }

    fn queue_abandon(
        &self,
        target_request_id: u64,
        caller: &CallerClaim,
        entry: &Arc<RequestEntry>,
    ) {
        let prior = if entry
            .lifecycle
            .compare_exchange(
                ACTIVE_READ,
                ABANDON_QUEUED,
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .is_ok()
        {
            ACTIVE_READ
        } else if entry
            .lifecycle
            .compare_exchange(
                CANCEL_IN_FLIGHT,
                ABANDON_QUEUED,
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .is_ok()
        {
            CANCEL_IN_FLIGHT
        } else {
            return;
        };
        let task = CleanupTask {
            target_request_id,
            caller: caller.clone(),
            entry: Arc::clone(entry),
            prior,
            epoch: entry.epoch,
        };
        match self.cleanup_sender.try_send(task) {
            Ok(()) => {}
            Err(TrySendError::Full(task)) | Err(TrySendError::Disconnected(task)) => {
                self.fail_cleanup_nonblocking(task.target_request_id, &task.entry);
            }
        }
    }

    fn poll_maintenance(
        &self,
        max_steps: usize,
        requested_deadline: Instant,
    ) -> Result<usize, BrokerClientError> {
        let hard_deadline = Instant::now()
            .checked_add(TRANSPORT_POLL_TIMEOUT)
            .ok_or(BrokerClientError::TransportPoisoned)?;
        let deadline = requested_deadline.min(hard_deadline);
        self.apply_deferred_poison_and_sweep();
        let mut processed = 0;
        while processed < max_steps && Instant::now() < deadline {
            let task = {
                let receiver = self
                    .cleanup_receiver
                    .lock()
                    .map_err(|_| BrokerClientError::TransportPoisoned)?;
                let Some(receiver) = receiver.as_ref() else {
                    self.sweep_terminal_entries();
                    return Ok(processed);
                };
                match receiver.try_recv() {
                    Ok(task) => task,
                    Err(TryRecvError::Empty) | Err(TryRecvError::Disconnected) => {
                        self.sweep_terminal_entries();
                        return Ok(processed);
                    }
                }
            };
            let target_request_id = task.target_request_id;
            let entry = Arc::clone(&task.entry);
            match catch_unwind(AssertUnwindSafe(|| {
                self.process_queued_abandon(task, deadline)
            })) {
                Ok(Ok(())) => {}
                Ok(Err(error)) => {
                    self.fail_cleanup_nonblocking(target_request_id, &entry);
                    self.apply_deferred_poison_and_sweep();
                    return Err(error);
                }
                Err(_) => {
                    self.fail_cleanup_nonblocking(target_request_id, &entry);
                    self.apply_deferred_poison_and_sweep();
                    return Err(BrokerClientError::TransportPoisoned);
                }
            }
            processed += 1;
        }
        self.apply_deferred_poison_and_sweep();
        Ok(processed)
    }

    fn maintain_before_operation(&self) -> Result<(), BrokerClientError> {
        let deadline = Instant::now()
            .checked_add(TRANSPORT_POLL_TIMEOUT)
            .ok_or(BrokerClientError::TransportPoisoned)?;
        self.poll_maintenance(MAINTENANCE_STEPS_PER_OPERATION, deadline)
            .map(|_| ())
    }

    #[cfg(test)]
    fn install_pause(slot: &Mutex<Option<TestPause>>) -> ClientTestPause {
        let entered = Arc::new(std::sync::Barrier::new(2));
        let release = Arc::new(std::sync::Barrier::new(2));
        let mut pause = match slot.lock() {
            Ok(pause) => pause,
            Err(poisoned) => poisoned.into_inner(),
        };
        assert!(pause.is_none(), "only one test pause may be installed");
        *pause = Some(TestPause {
            entered: Arc::clone(&entered),
            release: Arc::clone(&release),
        });
        ClientTestPause { entered, release }
    }

    #[cfg(test)]
    fn run_test_pause(slot: &Mutex<Option<TestPause>>) {
        let pause = match slot.lock() {
            Ok(mut pause) => pause.take(),
            Err(poisoned) => poisoned.into_inner().take(),
        };
        if let Some(pause) = pause {
            pause.entered.wait();
            pause.release.wait();
        }
    }

    #[cfg(test)]
    fn pause_before_completion_for_test(&self) {
        Self::run_test_pause(&self.completion_pause);
    }

    #[cfg(test)]
    fn pause_during_validation_for_test(&self) {
        Self::run_test_pause(&self.validation_pause);
    }

    #[cfg(test)]
    fn pause_before_activation_for_test(&self) {
        Self::run_test_pause(&self.activation_pause);
    }

    fn process_queued_abandon(
        &self,
        task: CleanupTask,
        deadline: Instant,
    ) -> Result<(), BrokerClientError> {
        if task.entry.lifecycle.load(Ordering::Acquire) != ABANDON_QUEUED {
            return Ok(());
        }
        let target_request_id = task.target_request_id;
        let entry = task.entry;
        self.retire_registry_only(target_request_id, &entry);
        if self.transport_poisoned.load(Ordering::Acquire)
            || self.operation_epoch.load(Ordering::Acquire) != task.epoch
        {
            entry
                .lifecycle
                .compare_exchange(
                    ABANDON_QUEUED,
                    TERMINAL,
                    Ordering::AcqRel,
                    Ordering::Acquire,
                )
                .ok();
            return Ok(());
        }
        self.poll_transport_until(task.epoch, deadline, |transport| {
            transport.try_discard(target_request_id)
        })?;

        let cancel = if task.prior == ACTIVE_READ {
            self.encode_cancel(&task.caller, target_request_id).ok()
        } else {
            None
        };
        let cancel_request = cancel
            .as_ref()
            .map(|(request_id, _, frame)| (*request_id, frame.clone()));
        let close_connection = task.prior == ACTIVE_READ && cancel.is_none();
        if close_connection {
            self.poison_transport();
        }
        let cancel_entry = cancel
            .as_ref()
            .map(|(request_id, entry, _)| (*request_id, Arc::clone(entry)));
        let outcome = if self.transport_poisoned.load(Ordering::Acquire) {
            Err(BrokerClientError::TransportPoisoned)
        } else {
            let begin = self.transport_step(task.epoch, |transport| {
                transport.begin_abandon(target_request_id, cancel_request.clone(), close_connection)
            });
            match begin {
                Ok(AdapterPoll::Ready(())) => Ok(()),
                Ok(AdapterPoll::Pending) => {
                    self.poll_transport_until(task.epoch, deadline, |transport| {
                        transport.poll_abandon(target_request_id)
                    })
                }
                Err(error) => Err(error),
            }
        };
        entry
            .lifecycle
            .compare_exchange(
                ABANDON_QUEUED,
                TERMINAL,
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .ok();
        if let Some((request_id, cancel_entry)) = cancel_entry.as_ref() {
            self.finish_request(*request_id, cancel_entry);
        }
        if outcome.is_err() {
            self.poison_transport();
        }
        outcome
    }

    fn transport_step<ResultValue>(
        &self,
        epoch: u64,
        step: impl FnOnce(&mut Transport) -> Result<AdapterPoll<ResultValue>, Transport::Error>,
    ) -> Result<AdapterPoll<ResultValue>, BrokerClientError> {
        if self.poison_requested.load(Ordering::Acquire)
            || self.transport_poisoned.load(Ordering::Acquire)
            || self.operation_epoch.load(Ordering::Acquire) != epoch
        {
            return Err(BrokerClientError::Closed);
        }
        let result = catch_unwind(AssertUnwindSafe(|| {
            let mut transport = match self.transport.lock() {
                Ok(transport) => transport,
                Err(poisoned) => poisoned.into_inner(),
            };
            if self.poison_requested.load(Ordering::Acquire)
                || self.transport_poisoned.load(Ordering::Acquire)
                || self.operation_epoch.load(Ordering::Acquire) != epoch
            {
                return Err(BrokerClientError::Closed);
            }
            step(&mut transport).map_err(|error| BrokerClientError::Transport(error.to_string()))
        }))
        .map_err(|_| BrokerClientError::Transport("transport poll panicked".to_owned()))??;
        if self.poison_requested.load(Ordering::Acquire)
            || self.transport_poisoned.load(Ordering::Acquire)
            || self.operation_epoch.load(Ordering::Acquire) != epoch
        {
            return Err(BrokerClientError::Closed);
        }
        Ok(result)
    }

    fn poll_transport_until(
        &self,
        epoch: u64,
        deadline: Instant,
        mut step: impl FnMut(&mut Transport) -> Result<AdapterPoll<()>, Transport::Error>,
    ) -> Result<(), BrokerClientError> {
        let mut backoff = PollBackoff::new();
        loop {
            match self.transport_step(epoch, |transport| step(transport))? {
                AdapterPoll::Ready(()) => return Ok(()),
                AdapterPoll::Pending if Instant::now() < deadline => backoff.park(deadline),
                AdapterPoll::Pending => {
                    self.poison_transport();
                    return Err(BrokerClientError::TransportPoisoned);
                }
            }
        }
    }

    fn poison_transport(&self) {
        self.admission_closed.store(true, Ordering::Release);
        self.poison_requested.store(true, Ordering::Release);
        self.apply_deferred_poison_and_sweep();
    }

    fn poison_transport_nonblocking(&self) {
        self.admission_closed.store(true, Ordering::Release);
        self.poison_requested.store(true, Ordering::Release);
        if let Ok(_lifecycle) = self.lifecycle_gate.try_lock() {
            self.apply_poison_locked();
        }
        if let Ok(mut requests) = self.requests.try_lock() {
            for (_, entry) in requests.drain() {
                let mut state = entry.lifecycle.load(Ordering::Acquire);
                while state != TERMINAL {
                    match entry.lifecycle.compare_exchange(
                        state,
                        TERMINAL,
                        Ordering::AcqRel,
                        Ordering::Acquire,
                    ) {
                        Ok(_) => break,
                        Err(current) => state = current,
                    }
                }
                self.retire_capacity(&entry);
            }
        }
    }

    fn apply_poison_locked(&self) {
        let is_new = !self.transport_poisoned.swap(true, Ordering::AcqRel);
        if is_new {
            self.admission_closed.store(true, Ordering::Release);
            self.operation_epoch.fetch_add(1, Ordering::AcqRel);
        }
        let mut state = self.transport_close_state.load(Ordering::Acquire);
        while state != CLOSE_CONFIRMED && state != CLOSE_POISONED {
            match self.transport_close_state.compare_exchange(
                state,
                CLOSE_POISONED,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => break,
                Err(current) => state = current,
            }
        }
    }

    fn apply_deferred_poison_and_sweep(&self) {
        {
            let _lifecycle = match self.lifecycle_gate.lock() {
                Ok(gate) => gate,
                Err(poisoned) => poisoned.into_inner(),
            };
            if self.poison_requested.load(Ordering::Acquire) {
                self.apply_poison_locked();
                self.clear_requests_locked();
            }
        }
        self.sweep_terminal_entries();
    }

    fn fail_close_before_poison(&self, epoch: u64) {
        let close_epoch = {
            let _lifecycle = match self.lifecycle_gate.lock() {
                Ok(gate) => gate,
                Err(poisoned) => poisoned.into_inner(),
            };
            if self.poison_requested.load(Ordering::Acquire)
                || self.transport_poisoned.load(Ordering::Acquire)
                || self.operation_epoch.load(Ordering::Acquire) != epoch
            {
                return;
            }
            self.admission_closed.store(true, Ordering::Release);
            let close_epoch = self.operation_epoch.fetch_add(1, Ordering::AcqRel) + 1;
            self.clear_requests_locked();
            close_epoch
        };
        let Some(deadline) = Instant::now().checked_add(TRANSPORT_POLL_TIMEOUT) else {
            self.poison_transport();
            return;
        };
        let begin = self.transport_step(close_epoch, Transport::begin_close);
        if matches!(begin, Ok(AdapterPoll::Pending)) {
            let _ = self.poll_transport_until(close_epoch, deadline, Transport::poll_close);
        }
        self.poison_transport();
    }

    fn close_transport(&self) -> Result<(), BrokerClientError> {
        self.apply_deferred_poison_and_sweep();
        if self.poison_requested.load(Ordering::Acquire)
            || self.transport_poisoned.load(Ordering::Acquire)
        {
            return Err(BrokerClientError::TransportPoisoned);
        }
        match self.transport_close_state.compare_exchange(
            CLOSE_UNCONFIRMED,
            CLOSE_IN_PROGRESS,
            Ordering::AcqRel,
            Ordering::Acquire,
        ) {
            Ok(_) => {}
            Err(CLOSE_CONFIRMED) => return Ok(()),
            Err(CLOSE_POISONED) => return Err(BrokerClientError::TransportPoisoned),
            Err(CLOSE_IN_PROGRESS) => return Err(BrokerClientError::CloseInProgress),
            Err(_) => {
                return Err(BrokerClientError::Transport(
                    "invalid close state".to_owned(),
                ));
            }
        }
        let epoch = {
            let _lifecycle = match self.lifecycle_gate.lock() {
                Ok(gate) => gate,
                Err(poisoned) => poisoned.into_inner(),
            };
            if self.poison_requested.load(Ordering::Acquire)
                || self.transport_poisoned.load(Ordering::Acquire)
            {
                self.transport_close_state
                    .store(CLOSE_POISONED, Ordering::Release);
                return Err(BrokerClientError::TransportPoisoned);
            }
            self.admission_closed.store(true, Ordering::Release);
            let epoch = self.operation_epoch.fetch_add(1, Ordering::AcqRel) + 1;
            self.clear_requests_locked();
            epoch
        };
        let Some(deadline) = Instant::now().checked_add(TRANSPORT_POLL_TIMEOUT) else {
            self.poison_transport();
            return Err(BrokerClientError::TransportPoisoned);
        };
        let result = match self.transport_step(epoch, Transport::begin_close) {
            Ok(AdapterPoll::Ready(())) => Ok(()),
            Ok(AdapterPoll::Pending) => {
                self.poll_transport_until(epoch, deadline, Transport::poll_close)
            }
            Err(error) => Err(error),
        };
        match result {
            Ok(()) => {
                self.transport_close_state
                    .compare_exchange(
                        CLOSE_IN_PROGRESS,
                        CLOSE_CONFIRMED,
                        Ordering::AcqRel,
                        Ordering::Acquire,
                    )
                    .map_err(|_| BrokerClientError::CloseInProgress)?;
                Ok(())
            }
            Err(BrokerClientError::Transport(error)) => {
                self.transport_close_state
                    .compare_exchange(
                        CLOSE_IN_PROGRESS,
                        CLOSE_UNCONFIRMED,
                        Ordering::AcqRel,
                        Ordering::Acquire,
                    )
                    .ok();
                Err(BrokerClientError::Transport(error.to_string()))
            }
            Err(BrokerClientError::TransportPoisoned) => Err(BrokerClientError::TransportPoisoned),
            Err(error) => {
                self.transport_close_state
                    .compare_exchange(
                        CLOSE_IN_PROGRESS,
                        CLOSE_UNCONFIRMED,
                        Ordering::AcqRel,
                        Ordering::Acquire,
                    )
                    .ok();
                Err(error)
            }
        }
    }

    fn retire_registry_only(&self, request_id: u64, entry: &RequestEntry) {
        self.retire_capacity(entry);
        self.remove_request_entry(request_id);
    }

    fn fail_cleanup_nonblocking(&self, request_id: u64, entry: &RequestEntry) {
        entry
            .lifecycle
            .compare_exchange(
                ABANDON_QUEUED,
                TERMINAL,
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .ok();
        self.retire_capacity(entry);
        if let Ok(mut requests) = self.requests.try_lock() {
            requests.remove(&request_id);
        }
        self.poison_transport_nonblocking();
    }

    fn abandon_bounded(
        &self,
        target_request_id: u64,
        caller: &CallerClaim,
        entry: &RequestEntry,
    ) -> Result<(), BrokerClientError> {
        let mut state = entry.lifecycle.load(Ordering::Acquire);
        let needs_cancel = loop {
            match state {
                AWAITING_RESPONSE | ACTIVE_READ => {
                    match entry.lifecycle.compare_exchange(
                        state,
                        CANCEL_IN_FLIGHT,
                        Ordering::AcqRel,
                        Ordering::Acquire,
                    ) {
                        Ok(_) => break true,
                        Err(current) => state = current,
                    }
                }
                CANCEL_IN_FLIGHT => break false,
                TERMINAL | ABANDON_QUEUED => return Err(BrokerClientError::RequestNotActive),
                _ => return Err(BrokerClientError::RequestNotActive),
            }
        };
        let cancel_result = if needs_cancel {
            self.send_cancel_control(target_request_id, caller)
        } else {
            Ok(())
        };
        let final_result = receive_response(
            self,
            target_request_id,
            Duration::from_secs(1),
            caller.client_instance,
            entry.epoch,
        )
        .map(|_| ())
        .or_else(|error| {
            if matches!(error, BrokerClientError::Cancelled) {
                Ok(())
            } else {
                Err(error)
            }
        });
        let final_result = self.finish_result(target_request_id, entry, final_result);
        if cancel_result.is_err() || final_result.is_err() {
            let _ = self.close_transport();
        }
        cancel_result.and(final_result)
    }

    fn send_cancel_control(
        &self,
        target_request_id: u64,
        caller: &CallerClaim,
    ) -> Result<(), BrokerClientError> {
        let (request_id, entry) = self.register_awaiting(RequestClass::Control)?;
        let result = send_request(
            self,
            &BrokerRequest::Cancel {
                request_id,
                caller: caller.clone(),
                target_request_id,
            },
            entry.epoch,
        )
        .and_then(|()| {
            receive_response(
                self,
                request_id,
                Duration::from_secs(1),
                caller.client_instance,
                entry.epoch,
            )
            .and_then(|response| match response {
                BrokerResponse::Cancelled {
                    client_instance,
                    target_request_id: returned_target,
                    ..
                } if client_instance == caller.client_instance
                    && returned_target == target_request_id =>
                {
                    Ok(())
                }
                _ => Err(BrokerClientError::UnexpectedResponse),
            })
        });
        self.finish_result(request_id, &entry, result)
    }

    fn clear_requests_locked(&self) {
        let mut requests = match self.requests.lock() {
            Ok(requests) => requests,
            Err(poisoned) => poisoned.into_inner(),
        };
        for (_, entry) in requests.drain() {
            let mut state = entry.lifecycle.load(Ordering::Acquire);
            while state != TERMINAL {
                match entry.lifecycle.compare_exchange(
                    state,
                    TERMINAL,
                    Ordering::AcqRel,
                    Ordering::Acquire,
                ) {
                    Ok(_) => break,
                    Err(current) => state = current,
                }
            }
            self.retire_capacity(&entry);
        }
    }

    fn sweep_terminal_entries(&self) {
        let mut requests = match self.requests.lock() {
            Ok(requests) => requests,
            Err(poisoned) => poisoned.into_inner(),
        };
        requests.retain(|_, entry| {
            let keep = entry.lifecycle.load(Ordering::Acquire) != TERMINAL;
            if !keep {
                self.retire_capacity(entry);
            }
            keep
        });
    }

    #[cfg(test)]
    fn request_count(&self) -> usize {
        self.business_count.load(Ordering::Acquire) + self.control_count.load(Ordering::Acquire)
    }
}

pub(crate) struct JournalBrokerClient<Transport> {
    dispatcher: Arc<ClientDispatcher<Transport>>,
}

impl<Transport> Clone for JournalBrokerClient<Transport> {
    fn clone(&self) -> Self {
        Self {
            dispatcher: Arc::clone(&self.dispatcher),
        }
    }
}

pub(crate) struct PendingCancelHandle<Transport: BrokerClientTransport> {
    dispatcher: Arc<ClientDispatcher<Transport>>,
    target_request_id: u64,
    client_instance: [u8; 16],
    entry: Arc<RequestEntry>,
}

impl<Transport: BrokerClientTransport> Clone for PendingCancelHandle<Transport> {
    fn clone(&self) -> Self {
        Self {
            dispatcher: Arc::clone(&self.dispatcher),
            target_request_id: self.target_request_id,
            client_instance: self.client_instance,
            entry: Arc::clone(&self.entry),
        }
    }
}

pub(crate) struct PendingReadRange<Transport: BrokerClientTransport> {
    cancel_handle: PendingCancelHandle<Transport>,
    expected_root: RootAuthorization,
    expected_journal: u64,
    expected_start: i64,
    expected_end: i64,
    expected_max_records: u32,
    expected_max_evidence_bytes: u32,
    timeout: Duration,
    caller: CallerClaim,
    is_armed: bool,
}

impl<Transport> PendingReadRange<Transport>
where
    Transport: BrokerClientTransport,
{
    pub(crate) fn cancel_handle(&self) -> PendingCancelHandle<Transport> {
        self.cancel_handle.clone()
    }

    pub(crate) fn wait(mut self) -> Result<BrokerResponse, BrokerClientError> {
        let response = receive_response(
            &self.cancel_handle.dispatcher,
            self.cancel_handle.target_request_id,
            self.timeout,
            self.cancel_handle.client_instance,
            self.cancel_handle.entry.epoch,
        )
        .and_then(|response| match &response {
            BrokerResponse::ReadRange {
                binding,
                journal_id,
                requested_start_usn,
                requested_end_usn,
                max_records,
                max_evidence_bytes,
                covered_until_usn,
                is_complete,
                candidates,
                ..
            } => {
                validate_binding(
                    binding,
                    self.cancel_handle.client_instance,
                    &self.expected_root,
                )?;
                if *journal_id != self.expected_journal
                    || *requested_start_usn != self.expected_start
                    || *requested_end_usn != self.expected_end
                    || *max_records != self.expected_max_records
                    || *max_evidence_bytes != self.expected_max_evidence_bytes
                    || *covered_until_usn <= self.expected_start
                    || *covered_until_usn > self.expected_end
                    || *is_complete != (*covered_until_usn == self.expected_end)
                {
                    return Err(BrokerClientError::RangeBindingMismatch);
                }
                if candidates.len() > self.expected_max_records as usize {
                    return Err(BrokerClientError::EvidenceBindingMismatch);
                }
                let mut previous_usn = None;
                let mut evidence_bytes = 0_usize;
                for candidate in candidates {
                    #[cfg(test)]
                    self.cancel_handle
                        .dispatcher
                        .pause_during_validation_for_test();
                    if candidate.usn < self.expected_start
                        || candidate.usn >= *covered_until_usn
                        || previous_usn.is_some_and(|previous| candidate.usn <= previous)
                    {
                        return Err(BrokerClientError::RangeBindingMismatch);
                    }
                    evidence_bytes = evidence_bytes
                        .checked_add(
                            candidate
                                .evidence_bytes()
                                .ok_or(BrokerClientError::EvidenceBindingMismatch)?,
                        )
                        .ok_or(BrokerClientError::EvidenceBindingMismatch)?;
                    if evidence_bytes > self.expected_max_evidence_bytes as usize {
                        return Err(BrokerClientError::EvidenceBindingMismatch);
                    }
                    previous_usn = Some(candidate.usn);
                }
                Ok(response)
            }
            _ => Err(BrokerClientError::UnexpectedResponse),
        });
        self.is_armed = false;
        match &response {
            Ok(_) | Err(BrokerClientError::Cancelled) | Err(BrokerClientError::Remote(_)) => {
                return self.cancel_handle.dispatcher.finish_result(
                    self.cancel_handle.target_request_id,
                    &self.cancel_handle.entry,
                    response,
                );
            }
            Err(_) => {
                let _ = self.cancel_handle.dispatcher.abandon_bounded(
                    self.cancel_handle.target_request_id,
                    &self.caller,
                    &self.cancel_handle.entry,
                );
            }
        }
        response
    }

    pub(crate) fn abandon(mut self) -> Result<(), BrokerClientError> {
        self.is_armed = false;
        self.cancel_handle.dispatcher.abandon_bounded(
            self.cancel_handle.target_request_id,
            &self.caller,
            &self.cancel_handle.entry,
        )
    }
}

impl<Transport: BrokerClientTransport> Drop for PendingReadRange<Transport> {
    fn drop(&mut self) {
        if self.is_armed {
            self.cancel_handle.dispatcher.queue_abandon(
                self.cancel_handle.target_request_id,
                &self.caller,
                &self.cancel_handle.entry,
            );
            self.is_armed = false;
        }
    }
}

pub(crate) struct PendingReadVolume<Transport: BrokerClientTransport> {
    cancel_handle: PendingCancelHandle<Transport>,
    expected_roots: Vec<(RootAuthorization, i64)>,
    expected_pending_renames: Vec<SharedJournalPendingRenameRequest>,
    expected_journal: u64,
    expected_end: i64,
    expected_max_records: u32,
    expected_max_evidence_bytes: u32,
    timeout: Duration,
    caller: CallerClaim,
    is_armed: bool,
}

impl<Transport> PendingReadVolume<Transport>
where
    Transport: BrokerClientTransport,
{
    pub(crate) fn cancel_handle(&self) -> PendingCancelHandle<Transport> {
        self.cancel_handle.clone()
    }

    pub(crate) fn wait(mut self) -> Result<BrokerResponse, BrokerClientError> {
        let response = receive_response(
            &self.cancel_handle.dispatcher,
            self.cancel_handle.target_request_id,
            self.timeout,
            self.cancel_handle.client_instance,
            self.cancel_handle.entry.epoch,
        )
        .and_then(|response| match &response {
            BrokerResponse::ReadVolume {
                client_instance,
                volume_id,
                journal_id,
                requested_end_usn,
                max_records,
                max_evidence_bytes,
                outcomes,
                handoffs,
                pending_renames,
                ..
            } => {
                if *client_instance != self.cancel_handle.client_instance
                    || *journal_id != self.expected_journal
                    || *requested_end_usn != self.expected_end
                    || *max_records != self.expected_max_records
                    || *max_evidence_bytes != self.expected_max_evidence_bytes
                    || outcomes.len() != self.expected_roots.len()
                    || self
                        .expected_roots
                        .first()
                        .is_none_or(|(root, _)| root.volume_id != *volume_id)
                {
                    return Err(BrokerClientError::RangeBindingMismatch);
                }
                for (outcome, (root, requested_start)) in outcomes.iter().zip(&self.expected_roots)
                {
                    validate_binding(&outcome.binding, *client_instance, root)?;
                    if outcome.requested_start_usn != *requested_start {
                        return Err(BrokerClientError::RangeBindingMismatch);
                    }
                }
                for pending in pending_renames {
                    let Some((expected_root, requested_start)) =
                        self.expected_roots.iter().find(|(root, _)| {
                            root.root_id == pending.binding.root_id
                                && root.root_generation == pending.binding.root_generation
                                && root.volume_id == pending.binding.volume_id
                        })
                    else {
                        return Err(BrokerClientError::RangeBindingMismatch);
                    };
                    validate_binding(&pending.binding, *client_instance, expected_root)?;
                    let Some(outcome) = outcomes.iter().find(|outcome| {
                        outcome.binding.root_id == pending.binding.root_id
                            && outcome.binding.root_generation == pending.binding.root_generation
                    }) else {
                        return Err(BrokerClientError::RangeBindingMismatch);
                    };
                    let Some(covered) = outcome
                        .covered_until_usn
                        .filter(|_| outcome.failure.is_none())
                    else {
                        return Err(BrokerClientError::RangeBindingMismatch);
                    };
                    if outcome.requested_start_usn != *requested_start
                        || pending.old_usn < *requested_start
                        || pending.old_usn >= covered
                    {
                        return Err(BrokerClientError::RangeBindingMismatch);
                    }
                }
                let mut consumed_carries = HashSet::new();
                for handoff in handoffs {
                    if let Some(carry_id) = &handoff.previous_carry_id {
                        if !consumed_carries.insert(carry_id.as_str()) {
                            return Err(BrokerClientError::RangeBindingMismatch);
                        }
                        let Some(pending) = self
                            .expected_pending_renames
                            .iter()
                            .find(|pending| pending.carry_id == *carry_id)
                        else {
                            return Err(BrokerClientError::RangeBindingMismatch);
                        };
                        validate_binding(
                            &handoff.previous_binding,
                            *client_instance,
                            &pending.root,
                        )?;
                        if handoff.file_reference != pending.file_reference
                            || handoff.previous_relative_path != pending.previous_relative_path
                            || handoff.is_directory != pending.is_directory
                            || handoff.usn <= pending.old_usn
                        {
                            return Err(BrokerClientError::RangeBindingMismatch);
                        }
                    }
                    let endpoints: &[&ResponseBinding] = if handoff.previous_carry_id.is_some() {
                        &[&handoff.current_binding]
                    } else {
                        &[&handoff.previous_binding, &handoff.current_binding]
                    };
                    for endpoint in endpoints {
                        let Some((expected_root, requested_start)) =
                            self.expected_roots.iter().find(|(root, _)| {
                                root.root_id == endpoint.root_id
                                    && root.root_generation == endpoint.root_generation
                                    && root.volume_id == endpoint.volume_id
                            })
                        else {
                            return Err(BrokerClientError::RangeBindingMismatch);
                        };
                        validate_binding(endpoint, *client_instance, expected_root)?;
                        let Some(outcome) = outcomes.iter().find(|outcome| {
                            outcome.binding.root_id == endpoint.root_id
                                && outcome.binding.root_generation == endpoint.root_generation
                        }) else {
                            return Err(BrokerClientError::RangeBindingMismatch);
                        };
                        let Some(covered) = outcome
                            .covered_until_usn
                            .filter(|_| outcome.failure.is_none())
                        else {
                            return Err(BrokerClientError::RangeBindingMismatch);
                        };
                        if outcome.requested_start_usn != *requested_start
                            || handoff.usn < *requested_start
                            || handoff.usn >= covered
                        {
                            return Err(BrokerClientError::RangeBindingMismatch);
                        }
                    }
                }
                Ok(response)
            }
            _ => Err(BrokerClientError::UnexpectedResponse),
        });
        self.is_armed = false;
        match &response {
            Ok(_) | Err(BrokerClientError::Cancelled) | Err(BrokerClientError::Remote(_)) => {
                return self.cancel_handle.dispatcher.finish_result(
                    self.cancel_handle.target_request_id,
                    &self.cancel_handle.entry,
                    response,
                );
            }
            Err(_) => {
                let _ = self.cancel_handle.dispatcher.abandon_bounded(
                    self.cancel_handle.target_request_id,
                    &self.caller,
                    &self.cancel_handle.entry,
                );
            }
        }
        response
    }
}

impl<Transport: BrokerClientTransport> Drop for PendingReadVolume<Transport> {
    fn drop(&mut self) {
        if self.is_armed {
            self.cancel_handle.dispatcher.queue_abandon(
                self.cancel_handle.target_request_id,
                &self.caller,
                &self.cancel_handle.entry,
            );
            self.is_armed = false;
        }
    }
}

impl<Transport> JournalBrokerClient<Transport>
where
    Transport: BrokerClientTransport,
{
    pub(crate) fn new(connection: OwnedBrokerConnection<Transport>) -> Self {
        Self::with_cleanup_configuration(connection, MAX_BUSINESS_REQUESTS_PER_CONNECTION, true)
    }

    #[cfg(test)]
    fn with_cleanup_capacity(
        connection: OwnedBrokerConnection<Transport>,
        cleanup_capacity: usize,
    ) -> Self {
        Self::with_cleanup_configuration(connection, cleanup_capacity, true)
    }

    fn with_cleanup_configuration(
        connection: OwnedBrokerConnection<Transport>,
        cleanup_capacity: usize,
        retain_cleanup_receiver: bool,
    ) -> Self {
        let (cleanup_sender, cleanup_receiver) = sync_channel(cleanup_capacity);
        let cleanup_receiver = retain_cleanup_receiver.then_some(cleanup_receiver);
        let dispatcher = Arc::new(ClientDispatcher {
            transport: Mutex::new(connection.transport),
            connection_id: connection.connection_id,
            connection_generation: connection.connection_generation,
            connection_nonce: connection.connection_nonce,
            operation_epoch: AtomicU64::new(1),
            transport_poisoned: AtomicBool::new(false),
            poison_requested: AtomicBool::new(false),
            lifecycle_gate: Mutex::new(()),
            next_request_id: AtomicU64::new(1),
            admission_closed: AtomicBool::new(false),
            transport_close_state: AtomicU8::new(CLOSE_UNCONFIRMED),
            business_count: AtomicUsize::new(0),
            control_count: AtomicUsize::new(0),
            requests: Mutex::new(HashMap::new()),
            cleanup_sender,
            cleanup_receiver: Mutex::new(cleanup_receiver),
            #[cfg(test)]
            completion_pause: Mutex::new(None),
            #[cfg(test)]
            committed_pause: Mutex::new(None),
            #[cfg(test)]
            validation_pause: Mutex::new(None),
            #[cfg(test)]
            activation_pause: Mutex::new(None),
        });
        Self { dispatcher }
    }

    #[cfg(test)]
    pub(crate) fn new_with_cleanup_capacity(
        connection: OwnedBrokerConnection<Transport>,
        cleanup_capacity: usize,
    ) -> Self {
        Self::with_cleanup_capacity(connection, cleanup_capacity)
    }

    #[cfg(test)]
    pub(crate) fn new_with_disconnected_cleanup(
        connection: OwnedBrokerConnection<Transport>,
    ) -> Self {
        Self::with_cleanup_configuration(connection, 1, false)
    }

    pub(crate) fn query_journal(
        &self,
        request: QueryJournalRequest,
    ) -> Result<BrokerResponse, BrokerClientError> {
        self.dispatcher.maintain_before_operation()?;
        self.ensure_open()?;
        request.validate().map_err(map_local_wire)?;
        let timeout = Duration::from_millis(u64::from(request.timeout_ms));
        let expected_root = request.root.clone();
        let caller = request.caller.clone();
        let client_instance = caller.client_instance;
        let (request_id, entry) = self.dispatcher.register_awaiting(RequestClass::Business)?;
        if let Err(error) = send_request(
            &self.dispatcher,
            &BrokerRequest::QueryJournal {
                request_id,
                request,
            },
            entry.epoch,
        ) {
            self.dispatcher.finish_request(request_id, &entry);
            return Err(error);
        }
        let response = receive_response(
            &self.dispatcher,
            request_id,
            timeout,
            client_instance,
            entry.epoch,
        )
        .and_then(|response| match &response {
            BrokerResponse::Journal { binding, .. } => {
                validate_binding(binding, client_instance, &expected_root)?;
                Ok(response)
            }
            _ => Err(BrokerClientError::UnexpectedResponse),
        });
        match &response {
            Ok(_) | Err(BrokerClientError::Cancelled) | Err(BrokerClientError::Remote(_)) => {
                return self.dispatcher.finish_result(request_id, &entry, response);
            }
            Err(_) => {
                let _ = self.dispatcher.abandon_bounded(request_id, &caller, &entry);
            }
        }
        response
    }

    #[allow(
        dead_code,
        reason = "R2c-O seals root registration before R2c-P product checkpoint wiring"
    )]
    pub(crate) fn register_root(
        &self,
        request: RegisterRootRequest,
    ) -> Result<RootCapability, BrokerClientError> {
        self.dispatcher.maintain_before_operation()?;
        self.ensure_open()?;
        request.validate().map_err(map_local_wire)?;
        let timeout = Duration::from_millis(u64::from(request.timeout_ms));
        let expected_root = request.root.clone();
        let caller = request.caller.clone();
        let client_instance = caller.client_instance;
        let (request_id, entry) = self.dispatcher.register_awaiting(RequestClass::Business)?;
        if let Err(error) = send_request(
            &self.dispatcher,
            &BrokerRequest::RegisterRoot {
                request_id,
                request,
            },
            entry.epoch,
        ) {
            self.dispatcher.finish_request(request_id, &entry);
            return Err(error);
        }
        let response = receive_response(
            &self.dispatcher,
            request_id,
            timeout,
            client_instance,
            entry.epoch,
        )
        .and_then(|response| match response {
            BrokerResponse::RootRegistered {
                binding,
                root_capability,
                ..
            } => {
                validate_binding(&binding, client_instance, &expected_root)?;
                root_capability.validate().map_err(map_local_wire)?;
                Ok(root_capability)
            }
            _ => Err(BrokerClientError::UnexpectedResponse),
        });
        match &response {
            Ok(_) | Err(BrokerClientError::Cancelled) | Err(BrokerClientError::Remote(_)) => {
                return self.dispatcher.finish_result(request_id, &entry, response);
            }
            Err(_) => {
                let _ = self.dispatcher.abandon_bounded(request_id, &caller, &entry);
            }
        }
        response
    }

    pub(crate) fn begin_read_range(
        &self,
        request: ReadJournalRangeRequest,
    ) -> Result<PendingReadRange<Transport>, BrokerClientError> {
        self.dispatcher.maintain_before_operation()?;
        self.ensure_open()?;
        request.validate().map_err(map_local_wire)?;
        let timeout = Duration::from_millis(u64::from(request.timeout_ms));
        let expected_root = request.root.clone();
        let expected_journal = request.journal_id;
        let expected_start = request.start_usn;
        let expected_end = request.end_usn;
        let expected_max_records = request.max_records;
        let expected_max_evidence_bytes = request.max_evidence_bytes;
        let caller = request.caller.clone();
        let client_instance = caller.client_instance;
        let (request_id, entry) = self.dispatcher.register_awaiting(RequestClass::Business)?;
        if let Err(error) = send_request(
            &self.dispatcher,
            &BrokerRequest::ReadRange {
                request_id,
                request,
            },
            entry.epoch,
        ) {
            self.dispatcher.finish_request(request_id, &entry);
            return Err(error);
        }
        let accepted = receive_response(
            &self.dispatcher,
            request_id,
            Duration::from_secs(1).min(timeout),
            client_instance,
            entry.epoch,
        );
        let accepted = match accepted {
            Ok(accepted) => accepted,
            Err(error) => {
                let _ = self.dispatcher.abandon_bounded(request_id, &caller, &entry);
                return Err(error);
            }
        };
        match accepted {
            BrokerResponse::ReadRangeAccepted {
                binding,
                journal_id,
                requested_start_usn,
                requested_end_usn,
                max_records,
                max_evidence_bytes,
                ..
            } => {
                if let Err(error) = validate_binding(&binding, client_instance, &expected_root) {
                    let _ = self.dispatcher.abandon_bounded(request_id, &caller, &entry);
                    return Err(error);
                }
                if journal_id != expected_journal
                    || requested_start_usn != expected_start
                    || requested_end_usn != expected_end
                    || max_records != expected_max_records
                    || max_evidence_bytes != expected_max_evidence_bytes
                {
                    let _ = self.dispatcher.abandon_bounded(request_id, &caller, &entry);
                    return Err(BrokerClientError::RangeBindingMismatch);
                }
            }
            _ => {
                let _ = self.dispatcher.abandon_bounded(request_id, &caller, &entry);
                return Err(BrokerClientError::UnexpectedResponse);
            }
        }
        #[cfg(test)]
        self.dispatcher.pause_before_activation_for_test();
        if let Err(error) = self.dispatcher.activate_read(&entry) {
            if entry.lifecycle.load(Ordering::Acquire) != TERMINAL {
                self.dispatcher.finish_request(request_id, &entry);
            }
            return Err(error);
        }
        Ok(PendingReadRange {
            cancel_handle: PendingCancelHandle {
                dispatcher: Arc::clone(&self.dispatcher),
                target_request_id: request_id,
                client_instance,
                entry,
            },
            expected_root,
            expected_journal,
            expected_start,
            expected_end,
            expected_max_records,
            expected_max_evidence_bytes,
            timeout,
            caller,
            is_armed: true,
        })
    }

    pub(crate) fn read_range(
        &self,
        request: ReadJournalRangeRequest,
    ) -> Result<BrokerResponse, BrokerClientError> {
        self.begin_read_range(request)?.wait()
    }

    pub(crate) fn begin_read_volume(
        &self,
        request: ReadJournalVolumeRequest,
    ) -> Result<PendingReadVolume<Transport>, BrokerClientError> {
        self.dispatcher.maintain_before_operation()?;
        self.ensure_open()?;
        request.validate().map_err(map_local_wire)?;
        let timeout = Duration::from_millis(u64::from(request.timeout_ms));
        let expected_roots = request
            .roots
            .iter()
            .map(|root| (root.root.clone(), root.start_usn))
            .collect::<Vec<_>>();
        let expected_pending_renames = request.pending_renames.clone();
        let expected_journal = request.journal_id;
        let expected_start = request.minimum_start_usn();
        let expected_end = request.end_usn;
        let expected_max_records = request.max_records;
        let expected_max_evidence_bytes = request.max_evidence_bytes;
        let expected_volume = request.roots[0].root.volume_id.clone();
        let caller = request.caller.clone();
        let client_instance = caller.client_instance;
        let (request_id, entry) = self.dispatcher.register_awaiting(RequestClass::Business)?;
        if let Err(error) = send_request(
            &self.dispatcher,
            &BrokerRequest::ReadVolume {
                request_id,
                request,
            },
            entry.epoch,
        ) {
            self.dispatcher.finish_request(request_id, &entry);
            return Err(error);
        }
        let accepted = receive_response(
            &self.dispatcher,
            request_id,
            Duration::from_secs(1).min(timeout),
            client_instance,
            entry.epoch,
        );
        match accepted {
            Ok(BrokerResponse::ReadVolumeAccepted {
                client_instance: returned_instance,
                volume_id,
                journal_id,
                requested_start_usn,
                requested_end_usn,
                root_count,
                max_records,
                max_evidence_bytes,
                ..
            }) if returned_instance == client_instance
                && volume_id == expected_volume
                && journal_id == expected_journal
                && requested_start_usn == expected_start
                && requested_end_usn == expected_end
                && usize::from(root_count) == expected_roots.len()
                && max_records == expected_max_records
                && max_evidence_bytes == expected_max_evidence_bytes => {}
            Ok(_) => {
                let _ = self.dispatcher.abandon_bounded(request_id, &caller, &entry);
                return Err(BrokerClientError::RangeBindingMismatch);
            }
            Err(error) => {
                let _ = self.dispatcher.abandon_bounded(request_id, &caller, &entry);
                return Err(error);
            }
        }
        #[cfg(test)]
        self.dispatcher.pause_before_activation_for_test();
        if let Err(error) = self.dispatcher.activate_read(&entry) {
            if entry.lifecycle.load(Ordering::Acquire) != TERMINAL {
                self.dispatcher.finish_request(request_id, &entry);
            }
            return Err(error);
        }
        Ok(PendingReadVolume {
            cancel_handle: PendingCancelHandle {
                dispatcher: Arc::clone(&self.dispatcher),
                target_request_id: request_id,
                client_instance,
                entry,
            },
            expected_roots,
            expected_pending_renames,
            expected_journal,
            expected_end,
            expected_max_records,
            expected_max_evidence_bytes,
            timeout,
            caller,
            is_armed: true,
        })
    }

    pub(crate) fn cancel(
        &self,
        pending: &PendingCancelHandle<Transport>,
        caller: CallerClaim,
    ) -> Result<(), BrokerClientError> {
        self.dispatcher.maintain_before_operation()?;
        self.ensure_open()?;
        if !Arc::ptr_eq(&self.dispatcher, &pending.dispatcher) {
            return Err(BrokerClientError::ForeignPendingHandle);
        }
        caller.validate().map_err(map_local_wire)?;
        if caller.client_instance != pending.client_instance {
            return Err(BrokerClientError::ResponseBindingMismatch);
        }
        pending
            .entry
            .lifecycle
            .compare_exchange(
                ACTIVE_READ,
                CANCEL_IN_FLIGHT,
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .map_err(|_| BrokerClientError::RequestNotActive)?;
        let result = self
            .dispatcher
            .send_cancel_control(pending.target_request_id, &caller);
        if result.is_err() {
            pending
                .entry
                .lifecycle
                .compare_exchange(
                    CANCEL_IN_FLIGHT,
                    ACTIVE_READ,
                    Ordering::AcqRel,
                    Ordering::Acquire,
                )
                .ok();
        }
        result
    }

    pub(crate) fn close(&self) -> Result<(), BrokerClientError> {
        let _ = self.dispatcher.maintain_before_operation();
        self.dispatcher.close_transport()
    }

    fn ensure_open(&self) -> Result<(), BrokerClientError> {
        if self.dispatcher.admission_closed.load(Ordering::Acquire) {
            Err(BrokerClientError::Closed)
        } else {
            Ok(())
        }
    }

    #[cfg(test)]
    pub(crate) fn pending_count(&self) -> usize {
        self.dispatcher.request_count()
    }

    #[cfg(test)]
    pub(crate) fn business_pending_count(&self) -> usize {
        self.dispatcher.business_count.load(Ordering::Acquire)
    }

    #[cfg(test)]
    pub(crate) fn control_pending_count(&self) -> usize {
        self.dispatcher.control_count.load(Ordering::Acquire)
    }

    pub(crate) fn poll_maintenance(
        &self,
        max_steps: usize,
        deadline: Instant,
    ) -> Result<usize, BrokerClientError> {
        self.dispatcher.poll_maintenance(max_steps, deadline)
    }

    #[cfg(test)]
    pub(crate) fn pause_next_completion(&self) -> ClientTestPause {
        ClientDispatcher::<Transport>::install_pause(&self.dispatcher.completion_pause)
    }

    #[cfg(test)]
    pub(crate) fn pause_next_candidate_validation(&self) -> ClientTestPause {
        ClientDispatcher::<Transport>::install_pause(&self.dispatcher.validation_pause)
    }

    #[cfg(test)]
    pub(crate) fn pause_next_committed_discard(&self) -> ClientTestPause {
        ClientDispatcher::<Transport>::install_pause(&self.dispatcher.committed_pause)
    }

    #[cfg(test)]
    pub(crate) fn pause_next_read_activation(&self) -> ClientTestPause {
        ClientDispatcher::<Transport>::install_pause(&self.dispatcher.activation_pause)
    }

    #[cfg(test)]
    pub(crate) fn hold_registry_lock_for_test(
        &self,
        entered: &std::sync::Barrier,
        release: &std::sync::Barrier,
    ) {
        let _requests = match self.dispatcher.requests.lock() {
            Ok(requests) => requests,
            Err(poisoned) => poisoned.into_inner(),
        };
        entered.wait();
        release.wait();
    }

    #[cfg(test)]
    pub(crate) fn internal_thread_count(&self) -> usize {
        0
    }

    #[cfg(test)]
    pub(crate) fn connection_identity(&self) -> ([u8; 16], u64, u64) {
        (
            self.dispatcher.connection_id,
            self.dispatcher.connection_generation,
            self.dispatcher.connection_nonce,
        )
    }

    #[cfg(test)]
    pub(crate) fn transport_close_confirmed(&self) -> bool {
        self.dispatcher
            .transport_close_state
            .load(Ordering::Acquire)
            == CLOSE_CONFIRMED
    }

    #[cfg(test)]
    pub(crate) fn transport_is_poisoned(&self) -> bool {
        self.dispatcher.poison_requested.load(Ordering::Acquire)
            || self.dispatcher.transport_poisoned.load(Ordering::Acquire)
    }

    #[cfg(test)]
    pub(crate) fn registry_entry_count(&self) -> usize {
        self.dispatcher
            .requests
            .lock()
            .map_or(0, |requests| requests.len())
    }
}

fn send_request<Transport>(
    dispatcher: &ClientDispatcher<Transport>,
    request: &BrokerRequest,
    epoch: u64,
) -> Result<(), BrokerClientError>
where
    Transport: BrokerClientTransport,
{
    let payload = encode_request(request).map_err(map_local_wire)?;
    let frame = encode_frame(&payload).map_err(BrokerClientError::Frame)?;
    let deadline = Instant::now()
        .checked_add(TRANSPORT_POLL_TIMEOUT)
        .ok_or(BrokerClientError::TransportPoisoned)?;
    let mut backoff = PollBackoff::new();
    let result = loop {
        match dispatcher.transport_step(epoch, |transport| transport.try_send_frame(&frame)) {
            Ok(AdapterPoll::Ready(())) => break Ok(()),
            Ok(AdapterPoll::Pending) if Instant::now() < deadline => backoff.park(deadline),
            Ok(AdapterPoll::Pending) => break Err(BrokerClientError::TransportPoisoned),
            Err(error) => break Err(error),
        }
    };
    if matches!(
        result,
        Err(BrokerClientError::Transport(_)) | Err(BrokerClientError::TransportPoisoned)
    ) {
        dispatcher.fail_close_before_poison(epoch);
    }
    result
}

fn receive_response<Transport>(
    dispatcher: &ClientDispatcher<Transport>,
    request_id: u64,
    timeout: Duration,
    expected_client_instance: [u8; 16],
    epoch: u64,
) -> Result<BrokerResponse, BrokerClientError>
where
    Transport: BrokerClientTransport,
{
    let deadline = Instant::now()
        .checked_add(timeout)
        .ok_or(BrokerClientError::TransportPoisoned)?;
    let mut backoff = PollBackoff::new();
    let response_frame = loop {
        match dispatcher.transport_step(epoch, |transport| transport.poll_response(request_id)) {
            Ok(AdapterPoll::Ready(frame)) => break frame,
            Ok(AdapterPoll::Pending) if Instant::now() < deadline => backoff.park(deadline),
            Ok(AdapterPoll::Pending) => {
                let _ =
                    dispatcher.transport_step(epoch, |transport| transport.try_discard(request_id));
                dispatcher.fail_close_before_poison(epoch);
                return Err(BrokerClientError::TransportPoisoned);
            }
            Err(error) => {
                if matches!(error, BrokerClientError::Transport(_)) {
                    dispatcher.fail_close_before_poison(epoch);
                }
                return Err(error);
            }
        }
    };
    let response_payload = decode_frame(&response_frame).map_err(|error| match error {
        BrokerFrameError::LengthRejected | BrokerFrameError::LengthOverflow => {
            BrokerClientError::ProtocolMismatch
        }
        BrokerFrameError::Truncated | BrokerFrameError::TrailingBytes => {
            BrokerClientError::MalformedResponse
        }
    })?;
    let response = decode_response(response_payload).map_err(|error| match error {
        WireError::UnsupportedVersion(_) => BrokerClientError::ProtocolMismatch,
        _ => BrokerClientError::MalformedResponse,
    })?;
    if let BrokerResponse::Failure {
        request_id: returned_id,
        client_instance,
        failure,
    } = response
    {
        if returned_id != 0 && returned_id != request_id {
            return Err(BrokerClientError::RequestMismatch);
        }
        if client_instance.is_some_and(|instance| instance != expected_client_instance) {
            return Err(BrokerClientError::ResponseBindingMismatch);
        }
        return match failure.code {
            BrokerFailureCode::ProtocolMismatch => Err(BrokerClientError::ProtocolMismatch),
            BrokerFailureCode::MalformedFrame => Err(BrokerClientError::MalformedResponse),
            BrokerFailureCode::Cancelled => Err(BrokerClientError::Cancelled),
            BrokerFailureCode::RequestNotActive => Err(BrokerClientError::RequestNotActive),
            _ if returned_id == 0 || client_instance.is_none() => {
                Err(BrokerClientError::ResponseBindingMismatch)
            }
            _ => Err(BrokerClientError::Remote(failure)),
        };
    }
    if response.request_id() != request_id {
        return Err(BrokerClientError::RequestMismatch);
    }
    Ok(response)
}

fn validate_binding(
    binding: &ResponseBinding,
    client_instance: [u8; 16],
    root: &RootAuthorization,
) -> Result<(), BrokerClientError> {
    if binding.client_instance != client_instance
        || binding.root_id != root.root_id
        || binding.root_generation != root.root_generation
        || binding.volume_id != root.volume_id
    {
        return Err(BrokerClientError::ResponseBindingMismatch);
    }
    Ok(())
}

fn map_local_wire(_: WireError) -> BrokerClientError {
    BrokerClientError::InvalidRequest
}

#[derive(Debug, Error)]
pub enum BrokerClientError {
    #[error("journal broker client is closed")]
    Closed,
    #[error("journal broker transport close is already in progress")]
    CloseInProgress,
    #[error("journal broker transport is poisoned after a bounded poll deadline")]
    TransportPoisoned,
    #[error("journal broker request is invalid")]
    InvalidRequest,
    #[error("journal broker request identifiers are exhausted")]
    RequestIdExhausted,
    #[error("journal broker returned a response for another request")]
    RequestMismatch,
    #[error("journal broker response binding does not match the request")]
    ResponseBindingMismatch,
    #[error("journal broker range boundary does not match the request")]
    RangeBindingMismatch,
    #[error("journal broker evidence exceeds the requested limits")]
    EvidenceBindingMismatch,
    #[error("journal broker protocol version is incompatible")]
    ProtocolMismatch,
    #[error("journal broker response is malformed")]
    MalformedResponse,
    #[error("journal broker request was cancelled")]
    Cancelled,
    #[error("pending request belongs to another authenticated dispatcher")]
    ForeignPendingHandle,
    #[error("pending request is no longer active")]
    RequestNotActive,
    #[error("journal broker client pending request limit was reached")]
    PendingLimitExceeded,
    #[error("journal broker returned an unexpected response")]
    UnexpectedResponse,
    #[error("journal broker rejected the request: {0:?}")]
    Remote(BrokerFailure),
    #[error("journal broker transport failed: {0}")]
    Transport(String),
    #[error("journal broker frame failed: {0}")]
    Frame(BrokerFrameError),
}

struct BrokerPendingRead<Transport: BrokerClientTransport> {
    client: JournalBrokerClient<Transport>,
    pending: Option<BrokerPendingReadKind<Transport>>,
    cancel_handle: PendingCancelHandle<Transport>,
}

enum BrokerPendingReadKind<Transport: BrokerClientTransport> {
    Root(PendingReadRange<Transport>),
    Volume(PendingReadVolume<Transport>),
}

impl<Transport> PersistentChangeJournalRead for BrokerPendingRead<Transport>
where
    Transport: BrokerClientTransport,
{
    fn cancel(&self, caller: CallerClaim) -> Result<(), PersistentChangeJournalOperationError> {
        self.client
            .cancel(&self.cancel_handle, caller)
            .map_err(Into::into)
    }

    fn wait(mut self: Box<Self>) -> Result<BrokerResponse, PersistentChangeJournalOperationError> {
        match self
            .pending
            .take()
            .ok_or(PersistentChangeJournalOperationError::RequestNotActive)?
        {
            BrokerPendingReadKind::Root(pending) => pending.wait().map_err(Into::into),
            BrokerPendingReadKind::Volume(pending) => pending.wait().map_err(Into::into),
        }
    }
}

impl<Transport> PersistentChangeJournalSession for JournalBrokerClient<Transport>
where
    Transport: BrokerClientTransport,
{
    fn register_root(
        &self,
        request: RegisterRootRequest,
    ) -> Result<RootCapability, PersistentChangeJournalOperationError> {
        JournalBrokerClient::register_root(self, request).map_err(Into::into)
    }

    fn query_journal(
        &self,
        request: QueryJournalRequest,
    ) -> Result<BrokerResponse, PersistentChangeJournalOperationError> {
        JournalBrokerClient::query_journal(self, request).map_err(Into::into)
    }

    fn begin_read_range(
        &self,
        request: ReadJournalRangeRequest,
    ) -> Result<Box<dyn PersistentChangeJournalRead>, PersistentChangeJournalOperationError> {
        let pending = JournalBrokerClient::begin_read_range(self, request)?;
        let cancel_handle = pending.cancel_handle();
        Ok(Box::new(BrokerPendingRead {
            client: self.clone(),
            pending: Some(BrokerPendingReadKind::Root(pending)),
            cancel_handle,
        }))
    }

    fn begin_read_volume(
        &self,
        request: ReadJournalVolumeRequest,
    ) -> Result<Box<dyn PersistentChangeJournalRead>, PersistentChangeJournalOperationError> {
        let pending = JournalBrokerClient::begin_read_volume(self, request)?;
        let cancel_handle = pending.cancel_handle();
        Ok(Box::new(BrokerPendingRead {
            client: self.clone(),
            pending: Some(BrokerPendingReadKind::Volume(pending)),
            cancel_handle,
        }))
    }

    fn close(&self) -> Result<(), PersistentChangeJournalOperationError> {
        JournalBrokerClient::close(self).map_err(Into::into)
    }
}

impl From<BrokerClientError> for PersistentChangeJournalOperationError {
    fn from(error: BrokerClientError) -> Self {
        match error {
            BrokerClientError::Closed => Self::Closed,
            BrokerClientError::CloseInProgress => Self::CloseInProgress,
            BrokerClientError::InvalidRequest | BrokerClientError::Frame(_) => Self::InvalidRequest,
            BrokerClientError::RequestIdExhausted | BrokerClientError::PendingLimitExceeded => {
                Self::CapacityExceeded
            }
            BrokerClientError::ProtocolMismatch => Self::ProtocolMismatch,
            BrokerClientError::Cancelled => Self::Cancelled,
            BrokerClientError::RequestNotActive => Self::RequestNotActive,
            BrokerClientError::RequestMismatch
            | BrokerClientError::ResponseBindingMismatch
            | BrokerClientError::RangeBindingMismatch
            | BrokerClientError::EvidenceBindingMismatch
            | BrokerClientError::MalformedResponse
            | BrokerClientError::ForeignPendingHandle
            | BrokerClientError::UnexpectedResponse => Self::ResponseIntegrity,
            BrokerClientError::TransportPoisoned | BrokerClientError::Transport(_) => {
                Self::TransportUnavailable
            }
            BrokerClientError::Remote(failure) => Self::Remote(failure.code),
        }
    }
}
