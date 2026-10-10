use std::ffi::c_void;
use std::io::{self, Write};
use std::mem::size_of;
use std::process::ExitCode;
use std::ptr::{null, null_mut};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::{Receiver, SyncSender, TryRecvError, sync_channel};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use windows_sys::Win32::Foundation::{
    ERROR_IO_PENDING, ERROR_PIPE_CONNECTED, LocalFree, WAIT_OBJECT_0,
};
use windows_sys::Win32::Security::Authorization::{
    ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1,
};
use windows_sys::Win32::Security::{PSECURITY_DESCRIPTOR, SECURITY_ATTRIBUTES};
use windows_sys::Win32::Storage::FileSystem::{
    FILE_FLAG_FIRST_PIPE_INSTANCE, FILE_FLAG_OVERLAPPED, PIPE_ACCESS_DUPLEX,
};
use windows_sys::Win32::System::IO::{CancelIoEx, GetOverlappedResult, OVERLAPPED};
use windows_sys::Win32::System::Pipes::{
    ConnectNamedPipe, CreateNamedPipeW, DisconnectNamedPipe, GetNamedPipeClientProcessId,
    GetNamedPipeClientSessionId, PIPE_READMODE_BYTE, PIPE_REJECT_REMOTE_CLIENTS, PIPE_TYPE_BYTE,
    PIPE_WAIT,
};
use windows_sys::Win32::System::Threading::{CreateEventW, WaitForSingleObject};

use super::connection::{
    CLIENT_PROOF_BYTES, ConnectionIdentitySource, ProcessConnectionIdentitySource,
    ServerConnectionIdentity, encode_handshake, encode_server_accept, validate_client_proof,
};
use super::transport::{
    OwnedHandle, PRODUCTION_PIPE_NAME, PendingFrameRead, WindowsTransportError, read_exact_bounded,
    write_exact_bounded,
};
use crate::journal_broker::MAX_FRAME_BYTES;
use crate::journal_broker::client::AdapterPoll;
use crate::journal_broker::framing::decode_frame;
use crate::journal_broker::service::{
    AuthenticatedConnection, BrokerService, ClientBinaryAdmission, ConnectedPipeAdmission,
    TerminalFrame, TerminalFrameBatch, WindowsPipeAuthorizer, WindowsRootRegistrationBudget,
};
use crate::journal_broker::windows::WindowsExistingJournalBackend;
use crate::journal_broker::wire::{BrokerRequest, BrokerResponse, decode_request, decode_response};

const CONSOLE_IDLE_TIMEOUT: Duration = Duration::from_secs(2);
const CONSOLE_MAX_LIFETIME: Duration = Duration::from_secs(30);
const PRODUCTION_IDLE_TIMEOUT: Duration = Duration::from_secs(30);
const PRODUCTION_MAX_LIFETIME: Duration = Duration::from_secs(5 * 60);
const HOST_POLL_INTERVAL: Duration = Duration::from_millis(25);
const WORKER_DRAIN_TIMEOUT: Duration = Duration::from_secs(2);
const MAX_HOST_READ_WORKERS: usize = 8;
const MAX_PRODUCTION_CONNECTIONS: u32 = 2;
const CONSOLE_PIPE_PREFIX: &str = r"\\.\pipe\CedarflakeAme.JournalBroker.Test.";
// The disposable host is still fail-closed at authorization. This DACL admits only local
// Any local disposable-test token may reach only this per-process pipe; remote clients are rejected
// and the production pipe retains its narrower interactive-user DACL. Root authorization remains
// fail-closed after the handshake.
const DISPOSABLE_PIPE_SDDL: &str = "D:P(A;;GA;;;SY)(A;;GA;;;BA)(A;;GRGW;;;WD)";
const PRODUCTION_PIPE_SDDL: &str = "D:P(A;;GA;;;SY)(A;;GA;;;BA)(A;;GRGW;;;IU)";

pub(super) fn run_disposable_console_host() -> ExitCode {
    match run_disposable_console_host_inner() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("journal_broker_console_host_failed:{error}");
            ExitCode::FAILURE
        }
    }
}

fn run_disposable_console_host_inner() -> Result<(), HostError> {
    let pipe_name = format!("{CONSOLE_PIPE_PREFIX}{}", std::process::id());
    let pipe = create_pipe(&pipe_name, DISPOSABLE_PIPE_SDDL, true, true, 1)?;
    println!("pipe={pipe_name}");
    io::stdout().flush()?;
    connect_bounded(&pipe, Instant::now() + CONSOLE_IDLE_TIMEOUT)?;

    let identity = ProcessConnectionIdentitySource::new().next_identity()?;
    serve_named_pipe(
        &pipe,
        identity,
        None,
        WindowsRootRegistrationBudget::production(),
        ReadWorkerBudget::new(MAX_HOST_READ_WORKERS),
    )
}

pub(super) fn run_production_service_with_ready(
    stop_requested: &'static AtomicBool,
    ready: SyncSender<()>,
) -> Result<(), ()> {
    run_production_service_inner(stop_requested, ready)
}

fn run_production_service_inner(
    stop_requested: &'static AtomicBool,
    ready: SyncSender<()>,
) -> Result<(), ()> {
    let first = create_pipe(
        PRODUCTION_PIPE_NAME,
        PRODUCTION_PIPE_SDDL,
        false,
        true,
        MAX_PRODUCTION_CONNECTIONS,
    )
    .map_err(|_| ())?;
    let second = create_pipe(
        PRODUCTION_PIPE_NAME,
        PRODUCTION_PIPE_SDDL,
        false,
        false,
        MAX_PRODUCTION_CONNECTIONS,
    )
    .map_err(|_| ())?;
    let identities = Arc::new(ProcessConnectionIdentitySource::new());
    let root_budget = WindowsRootRegistrationBudget::production();
    let worker_budget = ReadWorkerBudget::new(MAX_HOST_READ_WORKERS);
    let mut listeners = Vec::with_capacity(MAX_PRODUCTION_CONNECTIONS as usize);
    for initial in [first, second] {
        let identities = Arc::clone(&identities);
        let root_budget = root_budget.clone();
        let worker_budget = worker_budget.clone();
        listeners.push(std::thread::spawn(move || {
            run_production_listener(
                initial,
                stop_requested,
                identities,
                root_budget,
                worker_budget,
            )
        }));
    }
    ready.send(()).map_err(|_| ())?;
    let mut listener_failed = false;
    while !stop_requested.load(Ordering::Acquire) {
        if listeners.iter().any(JoinHandle::is_finished) {
            listener_failed = true;
            stop_requested.store(true, Ordering::Release);
            break;
        }
        std::thread::sleep(HOST_POLL_INTERVAL);
    }
    for listener in listeners {
        match listener.join() {
            Ok(Ok(())) => {}
            Ok(Err(_)) | Err(_) => listener_failed = true,
        }
    }
    if listener_failed { Err(()) } else { Ok(()) }
}

fn run_production_listener(
    initial: OwnedHandle,
    stop_requested: &'static AtomicBool,
    identities: Arc<ProcessConnectionIdentitySource>,
    root_budget: WindowsRootRegistrationBudget,
    worker_budget: ReadWorkerBudget,
) -> Result<(), HostError> {
    let mut pipe = Some(initial);
    while !stop_requested.load(Ordering::Acquire) {
        let current = match pipe.take() {
            Some(initial) => initial,
            None => create_pipe(
                PRODUCTION_PIPE_NAME,
                PRODUCTION_PIPE_SDDL,
                false,
                false,
                MAX_PRODUCTION_CONNECTIONS,
            )?,
        };
        match connect_bounded(&current, Instant::now() + Duration::from_millis(250)) {
            Ok(()) => {
                let identity = identities.next_identity()?;
                let _ = serve_named_pipe(
                    &current,
                    identity,
                    Some(stop_requested),
                    root_budget.clone(),
                    worker_budget.clone(),
                );
                // SAFETY: current is a live server-side pipe and serve_named_pipe drained or
                // retained every operation-owned OVERLAPPED before returning.
                let _ = unsafe { DisconnectNamedPipe(current.raw()) };
            }
            Err(HostError::TimedOut) => {}
            Err(error) => return Err(error),
        }
    }
    Ok(())
}

fn serve_named_pipe(
    pipe: &OwnedHandle,
    identity: ServerConnectionIdentity,
    stop_requested: Option<&AtomicBool>,
    root_budget: WindowsRootRegistrationBudget,
    worker_budget: ReadWorkerBudget,
) -> Result<(), HostError> {
    let handshake = encode_handshake(identity);
    write_exact_bounded(
        pipe,
        handshake.to_vec(),
        Instant::now() + CONSOLE_IDLE_TIMEOUT,
    )?;
    let client_proof = read_exact_bounded(
        pipe,
        CLIENT_PROOF_BYTES,
        Instant::now() + CONSOLE_IDLE_TIMEOUT,
    )?;
    validate_client_proof(identity, &client_proof).map_err(|_| HostError::ClientProofRejected)?;
    let (process_id, session_id) = observed_client(pipe)?;
    let client_admission = if stop_requested.is_some() {
        ClientBinaryAdmission::Production(
            super::service_identity::authenticate_pipe_client(process_id)
                .map_err(|_| HostError::AdmissionRejected)?,
        )
    } else {
        ClientBinaryAdmission::DisposableTest
    };
    let (authenticated, authorizer) = WindowsPipeAuthorizer::from_connected_pipe(
        pipe.raw(),
        ConnectedPipeAdmission::new(
            identity.connection_id,
            identity.connection_generation,
            identity.connection_nonce,
            process_id,
            session_id,
            client_admission,
        ),
        root_budget,
    )
    .map_err(|error| HostError::ClientTokenRejected(error.to_string()))?;
    let backend_security = authorizer
        .backend_security()
        .map_err(|error| HostError::ClientTokenRejected(error.to_string()))?;
    let service = Arc::new(BrokerService::new(
        WindowsExistingJournalBackend::new(backend_security),
        authorizer,
    ));
    write_exact_bounded(
        pipe,
        encode_server_accept(identity).to_vec(),
        Instant::now() + CONSOLE_IDLE_TIMEOUT,
    )?;
    let mut io = NamedPipeHostIo::new(pipe, stop_requested);
    let mut dispatch = ServiceHostDispatch::new(service, authenticated, worker_budget);
    serve_connection(&mut io, &mut dispatch)
}

trait HostIo {
    fn read_frame(&mut self) -> Result<HostRead, HostError>;
    fn write_frame(&mut self, frame: Vec<u8>) -> Result<(), HostError>;
}

trait HostDispatch {
    fn open(&mut self) -> Result<(), HostError>;
    fn reap(&mut self) -> Result<HostBatch, HostError>;
    fn dispatch(&mut self, frame: &[u8]) -> Result<HostReply, HostError>;
    fn execute(&mut self, accepted: AcceptedRead) -> Result<(), HostError>;
    fn abort(&mut self, accepted: AcceptedRead);
    fn disconnect(&mut self);
}

enum HostRead {
    Frame(Vec<u8>),
    Idle,
    Closed,
}

struct HostReply {
    immediate: Vec<u8>,
    accepted: Option<AcceptedRead>,
}

#[derive(Clone, Copy)]
struct AcceptedRead {
    request_id: u64,
    client_instance: [u8; 16],
}

struct HostBatch {
    frames: Vec<Vec<u8>>,
    must_close: bool,
}

fn serve_connection(
    io: &mut impl HostIo,
    dispatch: &mut impl HostDispatch,
) -> Result<(), HostError> {
    let result = (|| {
        dispatch.open()?;
        loop {
            let batch = dispatch.reap()?;
            for frame in batch.frames {
                io.write_frame(frame)?;
            }
            if batch.must_close {
                break;
            }
            let frame = match io.read_frame()? {
                HostRead::Frame(frame) => frame,
                HostRead::Idle => continue,
                HostRead::Closed => break,
            };
            let reply = dispatch.dispatch(&frame)?;
            if let Err(error) = io.write_frame(reply.immediate) {
                if let Some(accepted) = reply.accepted {
                    dispatch.abort(accepted);
                }
                return Err(error);
            }
            if let Some(accepted) = reply.accepted {
                dispatch.execute(accepted)?;
            }
        }
        Ok(())
    })();
    dispatch.disconnect();
    result
}

struct NamedPipeHostIo<'a> {
    pipe: &'a OwnedHandle,
    stop_requested: Option<&'a AtomicBool>,
    pending_read: Option<PendingFrameRead>,
    connected_at: Instant,
    last_activity: Instant,
    idle_timeout: Duration,
    max_lifetime: Duration,
}

impl<'a> NamedPipeHostIo<'a> {
    fn new(pipe: &'a OwnedHandle, stop_requested: Option<&'a AtomicBool>) -> Self {
        let now = Instant::now();
        let (idle_timeout, max_lifetime) = if stop_requested.is_some() {
            (PRODUCTION_IDLE_TIMEOUT, PRODUCTION_MAX_LIFETIME)
        } else {
            (CONSOLE_IDLE_TIMEOUT, CONSOLE_MAX_LIFETIME)
        };
        Self {
            pipe,
            stop_requested,
            pending_read: None,
            connected_at: now,
            last_activity: now,
            idle_timeout,
            max_lifetime,
        }
    }

    fn deadline_reached(&self, now: Instant) -> bool {
        connection_deadline_reached(
            now,
            self.connected_at,
            self.last_activity,
            self.idle_timeout,
            self.max_lifetime,
        )
    }
}

fn connection_deadline_reached(
    now: Instant,
    connected_at: Instant,
    last_activity: Instant,
    idle_timeout: Duration,
    max_lifetime: Duration,
) -> bool {
    now.saturating_duration_since(last_activity) >= idle_timeout
        || now.saturating_duration_since(connected_at) >= max_lifetime
}

impl HostIo for NamedPipeHostIo<'_> {
    fn read_frame(&mut self) -> Result<HostRead, HostError> {
        let now = Instant::now();
        if self
            .stop_requested
            .is_some_and(|stop| stop.load(Ordering::Acquire))
            || self.deadline_reached(now)
        {
            self.cancel_pending_read();
            return Ok(HostRead::Closed);
        }
        if self.pending_read.is_none() {
            match PendingFrameRead::start(self.pipe) {
                Ok(read) => self.pending_read = Some(read),
                Err(WindowsTransportError::Closed) => return Ok(HostRead::Closed),
                Err(error) => return Err(error.into()),
            }
        }
        let read = self
            .pending_read
            .as_mut()
            .ok_or(HostError::DispatchFailed)?;
        match read.poll(self.pipe) {
            Ok(AdapterPoll::Ready(frame)) => {
                self.pending_read = None;
                self.last_activity = Instant::now();
                Ok(HostRead::Frame(frame))
            }
            Ok(AdapterPoll::Pending) => {
                read.wait_slice(HOST_POLL_INTERVAL.as_millis() as u32);
                Ok(HostRead::Idle)
            }
            Err(WindowsTransportError::Closed) => {
                self.cancel_pending_read();
                Ok(HostRead::Closed)
            }
            Err(error) => {
                self.cancel_pending_read();
                Err(error.into())
            }
        }
    }

    fn write_frame(&mut self, frame: Vec<u8>) -> Result<(), HostError> {
        if self.deadline_reached(Instant::now()) {
            return Err(HostError::TimedOut);
        }
        write_exact_bounded(self.pipe, frame, Instant::now() + CONSOLE_IDLE_TIMEOUT)?;
        self.last_activity = Instant::now();
        Ok(())
    }
}

impl NamedPipeHostIo<'_> {
    fn cancel_pending_read(&mut self) {
        if let Some(mut read) = self.pending_read.take()
            && !read.cancel_and_drain(self.pipe)
        {
            std::mem::forget(read);
        }
    }
}

impl Drop for NamedPipeHostIo<'_> {
    fn drop(&mut self) {
        self.cancel_pending_read();
    }
}

type WindowsBrokerService = BrokerService<WindowsExistingJournalBackend, WindowsPipeAuthorizer>;

#[derive(Clone)]
struct ReadWorkerBudget {
    state: Arc<ReadWorkerBudgetState>,
}

struct ReadWorkerBudgetState {
    active: AtomicUsize,
    limit: usize,
}

struct ReadWorkerPermit {
    state: Arc<ReadWorkerBudgetState>,
}

impl ReadWorkerBudget {
    fn new(limit: usize) -> Self {
        Self {
            state: Arc::new(ReadWorkerBudgetState {
                active: AtomicUsize::new(0),
                limit,
            }),
        }
    }

    fn try_acquire(&self) -> Result<ReadWorkerPermit, HostError> {
        self.state
            .active
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |active| {
                (active < self.state.limit).then_some(active + 1)
            })
            .map_err(|_| HostError::DispatchBackpressure)?;
        Ok(ReadWorkerPermit {
            state: Arc::clone(&self.state),
        })
    }
}

impl Drop for ReadWorkerPermit {
    fn drop(&mut self) {
        self.state.active.fetch_sub(1, Ordering::AcqRel);
    }
}

struct ServiceHostDispatch {
    service: Arc<WindowsBrokerService>,
    authenticated: AuthenticatedConnection,
    completed_sender: SyncSender<Result<Vec<u8>, ()>>,
    completed_receiver: Receiver<Result<Vec<u8>, ()>>,
    workers: Mutex<Vec<JoinHandle<()>>>,
    worker_budget: ReadWorkerBudget,
}

impl ServiceHostDispatch {
    fn new(
        service: Arc<WindowsBrokerService>,
        authenticated: AuthenticatedConnection,
        worker_budget: ReadWorkerBudget,
    ) -> Self {
        let (completed_sender, completed_receiver) = sync_channel(MAX_HOST_READ_WORKERS);
        Self {
            service,
            authenticated,
            completed_sender,
            completed_receiver,
            workers: Mutex::new(Vec::with_capacity(MAX_HOST_READ_WORKERS)),
            worker_budget,
        }
    }

    fn reap_workers(&self) -> Result<(), HostError> {
        let mut workers = self.workers.lock().map_err(|_| HostError::DispatchFailed)?;
        let mut index = 0;
        while index < workers.len() {
            if workers[index].is_finished() {
                workers
                    .swap_remove(index)
                    .join()
                    .map_err(|_| HostError::DispatchFailed)?;
            } else {
                index += 1;
            }
        }
        Ok(())
    }
}

impl HostDispatch for ServiceHostDispatch {
    fn open(&mut self) -> Result<(), HostError> {
        self.service
            .open_connection(&self.authenticated)
            .map_err(|_| HostError::AdmissionRejected)
    }

    fn reap(&mut self) -> Result<HostBatch, HostError> {
        self.reap_workers()?;
        self.service.reap_authorizer(Instant::now());
        let TerminalFrameBatch { frames, must_close } = self
            .service
            .reap_accepted_reads(&self.authenticated)
            .map_err(|_| HostError::DispatchFailed)?;
        let mut output = frames
            .into_iter()
            .map(|TerminalFrame { request_id, frame }| {
                if request_id == 0 || frame.is_empty() {
                    Err(HostError::DispatchFailed)
                } else {
                    Ok(frame)
                }
            })
            .collect::<Result<Vec<_>, _>>()?;
        loop {
            match self.completed_receiver.try_recv() {
                Ok(Ok(frame)) if !frame.is_empty() => output.push(frame),
                Ok(Ok(_) | Err(())) => return Err(HostError::DispatchFailed),
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected)
                    if self.workers.lock().is_ok_and(|w| w.is_empty()) =>
                {
                    break;
                }
                Err(TryRecvError::Disconnected) => return Err(HostError::DispatchFailed),
            }
        }
        Ok(HostBatch {
            frames: output,
            must_close,
        })
    }

    fn dispatch(&mut self, frame: &[u8]) -> Result<HostReply, HostError> {
        let request =
            decode_request(decode_frame(frame).map_err(|_| HostError::MalformedFrame)?).ok();
        let immediate = self
            .service
            .handle_frame(frame, &self.authenticated)
            .map_err(|_| HostError::DispatchFailed)?;
        let response =
            decode_response(decode_frame(&immediate).map_err(|_| HostError::DispatchFailed)?)
                .map_err(|_| HostError::DispatchFailed)?;
        let accepted = match (request, response) {
            (
                Some(BrokerRequest::ReadRange {
                    request_id,
                    request,
                }),
                BrokerResponse::ReadRangeAccepted {
                    request_id: accepted_id,
                    ..
                },
            ) if request_id == accepted_id => Some(AcceptedRead {
                request_id,
                client_instance: request.caller.client_instance,
            }),
            (
                Some(BrokerRequest::ReadVolume {
                    request_id,
                    request,
                }),
                BrokerResponse::ReadVolumeAccepted {
                    request_id: accepted_id,
                    ..
                },
            ) if request_id == accepted_id => Some(AcceptedRead {
                request_id,
                client_instance: request.caller.client_instance,
            }),
            _ => None,
        };
        Ok(HostReply {
            immediate,
            accepted,
        })
    }

    fn execute(&mut self, accepted: AcceptedRead) -> Result<(), HostError> {
        self.reap_workers()?;
        let mut workers = self.workers.lock().map_err(|_| HostError::DispatchFailed)?;
        if workers.len() >= MAX_HOST_READ_WORKERS {
            return Err(HostError::DispatchBackpressure);
        }
        let permit = self.worker_budget.try_acquire()?;
        let service = Arc::clone(&self.service);
        let authenticated = self.authenticated.clone();
        let completed = self.completed_sender.clone();
        workers.push(std::thread::spawn(move || {
            let _permit = permit;
            let result = service
                .execute_accepted_read_frame(
                    accepted.request_id,
                    accepted.client_instance,
                    &authenticated,
                )
                .map_err(|_| ());
            let _ = completed.try_send(result);
        }));
        Ok(())
    }

    fn abort(&mut self, accepted: AcceptedRead) {
        let _ = self.service.abort_accepted_read(
            accepted.request_id,
            accepted.client_instance,
            &self.authenticated,
        );
    }

    fn disconnect(&mut self) {
        self.service.disconnect(&self.authenticated);
        if let Ok(mut workers) = self.workers.lock() {
            let deadline = Instant::now() + WORKER_DRAIN_TIMEOUT;
            if !drain_worker_handles(&mut workers, deadline) {
                std::process::abort();
            }
        }
    }
}

fn drain_worker_handles(workers: &mut Vec<JoinHandle<()>>, deadline: Instant) -> bool {
    while !workers.is_empty() {
        let mut index = 0;
        while index < workers.len() {
            if workers[index].is_finished() {
                let _ = workers.swap_remove(index).join();
            } else {
                index += 1;
            }
        }
        if workers.is_empty() {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        std::thread::sleep(HOST_POLL_INTERVAL);
    }
    true
}

fn create_pipe(
    name: &str,
    sddl: &str,
    is_disposable: bool,
    is_first_instance: bool,
    max_instances: u32,
) -> Result<OwnedHandle, HostError> {
    let disposable_policy = is_disposable
        && (is_first_instance && max_instances == 1
            || cfg!(test) && max_instances == MAX_PRODUCTION_CONNECTIONS);
    if (is_disposable && !name.starts_with(CONSOLE_PIPE_PREFIX))
        || (!is_disposable && name != PRODUCTION_PIPE_NAME)
        || (is_disposable && !disposable_policy)
        || (!is_disposable && max_instances != MAX_PRODUCTION_CONNECTIONS)
        || name.encode_utf16().any(|unit| unit == 0)
    {
        return Err(HostError::InvalidPipeName);
    }
    let mut name_utf16: Vec<u16> = name.encode_utf16().collect();
    name_utf16.push(0);
    let security = SecurityDescriptor::from_sddl(sddl)?;
    let attributes = SECURITY_ATTRIBUTES {
        nLength: u32::try_from(size_of::<SECURITY_ATTRIBUTES>())
            .map_err(|_| HostError::SecurityDescriptor)?,
        lpSecurityDescriptor: security.raw,
        bInheritHandle: 0,
    };
    let buffer_size = u32::try_from(MAX_FRAME_BYTES + 4).map_err(|_| HostError::MalformedFrame)?;
    // SAFETY: name is bounded, NUL-terminated, and live for the call; attributes references the
    // live self-relative descriptor; the checked handle becomes the sole RAII owner.
    let access = PIPE_ACCESS_DUPLEX
        | FILE_FLAG_OVERLAPPED
        | if is_first_instance {
            FILE_FLAG_FIRST_PIPE_INSTANCE
        } else {
            0
        };
    let raw = unsafe {
        CreateNamedPipeW(
            name_utf16.as_ptr(),
            access,
            PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT | PIPE_REJECT_REMOTE_CLIENTS,
            max_instances,
            buffer_size,
            buffer_size,
            0,
            &attributes,
        )
    };
    OwnedHandle::new(raw).map_err(HostError::Io)
}

fn connect_bounded(pipe: &OwnedHandle, deadline: Instant) -> Result<(), HostError> {
    // SAFETY: null attributes and name create a private manual-reset event owned below.
    let event = OwnedHandle::new(unsafe { CreateEventW(null(), 1, 0, null()) })?;
    let mut overlapped = Box::<OVERLAPPED>::default();
    overlapped.hEvent = event.raw();
    // SAFETY: pipe and stable OVERLAPPED/event remain live until completion or safe retention.
    let connected = unsafe { ConnectNamedPipe(pipe.raw(), overlapped.as_mut()) };
    if connected != 0 {
        return Ok(());
    }
    let error = io::Error::last_os_error();
    if error.raw_os_error() == Some(ERROR_PIPE_CONNECTED as i32) {
        return Ok(());
    }
    if error.raw_os_error() != Some(ERROR_IO_PENDING as i32) {
        return Err(HostError::Io(error));
    }
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            // SAFETY: the OVERLAPPED belongs to pipe and stays live for the completion wait.
            let _ = unsafe { CancelIoEx(pipe.raw(), overlapped.as_ref()) };
            // SAFETY: event is a live waitable handle and the drain wait is bounded.
            if unsafe { WaitForSingleObject(event.raw(), 1_000) } != WAIT_OBJECT_0 {
                std::mem::forget(overlapped);
                std::mem::forget(event);
            }
            return Err(HostError::TimedOut);
        }
        // SAFETY: event is a live waitable handle and each wait slice is bounded.
        if unsafe { WaitForSingleObject(event.raw(), 10) } == WAIT_OBJECT_0 {
            let mut transferred = 0;
            // SAFETY: stable OVERLAPPED and pipe are live, and the event reported completion.
            if unsafe { GetOverlappedResult(pipe.raw(), overlapped.as_ref(), &mut transferred, 0) }
                == 0
            {
                return Err(HostError::Io(io::Error::last_os_error()));
            }
            return Ok(());
        }
    }
}

fn observed_client(pipe: &OwnedHandle) -> Result<(u32, u32), HostError> {
    let mut process_id = 0;
    let mut session_id = 0;
    // SAFETY: outputs point to live u32 values and pipe is a connected server pipe.
    if unsafe { GetNamedPipeClientProcessId(pipe.raw(), &mut process_id) } == 0
        // SAFETY: same invariant as the process-id query above.
        || unsafe { GetNamedPipeClientSessionId(pipe.raw(), &mut session_id) } == 0
        || process_id == 0
    {
        return Err(HostError::AdmissionRejected);
    }
    Ok((process_id, session_id))
}

struct SecurityDescriptor {
    raw: *mut c_void,
}

impl SecurityDescriptor {
    fn from_sddl(sddl: &str) -> Result<Self, HostError> {
        let mut sddl_utf16: Vec<u16> = sddl.encode_utf16().collect();
        sddl_utf16.push(0);
        let mut raw: PSECURITY_DESCRIPTOR = null_mut();
        // SAFETY: sddl is NUL-terminated and live, and raw points to an output slot. LocalAlloc
        // ownership transfers to SecurityDescriptor only after a non-null success result.
        if unsafe {
            ConvertStringSecurityDescriptorToSecurityDescriptorW(
                sddl_utf16.as_ptr(),
                SDDL_REVISION_1,
                &mut raw,
                null_mut(),
            )
        } == 0
            || raw.is_null()
        {
            return Err(HostError::SecurityDescriptor);
        }
        Ok(Self { raw })
    }
}

impl Drop for SecurityDescriptor {
    fn drop(&mut self) {
        // SAFETY: raw is the unique non-null allocation returned by the SDDL conversion API.
        let _ = unsafe { LocalFree(self.raw) };
    }
}

#[derive(Debug, thiserror::Error)]
enum HostError {
    #[error("invalid disposable pipe name")]
    InvalidPipeName,
    #[error("security descriptor creation failed")]
    SecurityDescriptor,
    #[error("client admission rejected")]
    AdmissionRejected,
    #[error("connection-bound client proof rejected")]
    ClientProofRejected,
    #[error("client token admission rejected: {0}")]
    ClientTokenRejected(String),
    #[error("frame is malformed")]
    MalformedFrame,
    #[error("broker dispatch failed")]
    DispatchFailed,
    #[error("broker dispatch resource limit reached")]
    DispatchBackpressure,
    #[error("bounded host operation timed out")]
    TimedOut,
    #[error(transparent)]
    Transport(#[from] WindowsTransportError),
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error("connection identity generation failed")]
    Identity,
}

impl From<super::connection::ConnectionIdentityError> for HostError {
    fn from(_: super::connection::ConnectionIdentityError) -> Self {
        Self::Identity
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use std::sync::atomic::{AtomicU64, Ordering};
    use windows_sys::Win32::Foundation::{GENERIC_READ, GENERIC_WRITE};
    use windows_sys::Win32::Storage::FileSystem::{CreateFileW, OPEN_EXISTING};
    use windows_sys::Win32::System::Pipes::WaitNamedPipeW;

    #[derive(Default)]
    struct FakeIo {
        incoming: VecDeque<Vec<u8>>,
        outgoing: Vec<Vec<u8>>,
        fail_write_at: Option<usize>,
    }

    impl HostIo for FakeIo {
        fn read_frame(&mut self) -> Result<HostRead, HostError> {
            Ok(match self.incoming.pop_front() {
                Some(frame) => HostRead::Frame(frame),
                None => HostRead::Closed,
            })
        }

        fn write_frame(&mut self, frame: Vec<u8>) -> Result<(), HostError> {
            if self.fail_write_at == Some(self.outgoing.len()) {
                return Err(HostError::DispatchFailed);
            }
            self.outgoing.push(frame);
            Ok(())
        }
    }

    #[derive(Default)]
    struct FakeDispatch {
        opened: bool,
        executed: Vec<u64>,
        aborted: Vec<u64>,
        disconnected: bool,
        accept_next: bool,
        completed: Vec<Vec<u8>>,
    }

    impl HostDispatch for FakeDispatch {
        fn open(&mut self) -> Result<(), HostError> {
            self.opened = true;
            Ok(())
        }

        fn reap(&mut self) -> Result<HostBatch, HostError> {
            Ok(HostBatch {
                frames: std::mem::take(&mut self.completed),
                must_close: false,
            })
        }

        fn dispatch(&mut self, _frame: &[u8]) -> Result<HostReply, HostError> {
            Ok(HostReply {
                immediate: vec![1],
                accepted: self.accept_next.then_some(AcceptedRead {
                    request_id: 7,
                    client_instance: [3; 16],
                }),
            })
        }

        fn execute(&mut self, accepted: AcceptedRead) -> Result<(), HostError> {
            self.executed.push(accepted.request_id);
            self.completed.push(vec![2]);
            Ok(())
        }

        fn abort(&mut self, accepted: AcceptedRead) {
            self.aborted.push(accepted.request_id);
        }

        fn disconnect(&mut self) {
            self.disconnected = true;
        }
    }

    #[test]
    fn accepted_read_is_dispatched_after_ack_and_disconnect_is_guaranteed() {
        let mut io = FakeIo {
            incoming: VecDeque::from([vec![9]]),
            ..FakeIo::default()
        };
        let mut dispatch = FakeDispatch {
            accept_next: true,
            ..FakeDispatch::default()
        };

        serve_connection(&mut io, &mut dispatch).expect("host loop");

        assert!(dispatch.opened);
        assert_eq!(dispatch.executed, vec![7]);
        assert!(dispatch.aborted.is_empty());
        assert!(dispatch.disconnected);
        assert_eq!(io.outgoing, vec![vec![1], vec![2]]);
    }

    #[test]
    fn failed_ack_aborts_accepted_read_and_disconnects() {
        let mut io = FakeIo {
            incoming: VecDeque::from([vec![9]]),
            fail_write_at: Some(0),
            ..FakeIo::default()
        };
        let mut dispatch = FakeDispatch {
            accept_next: true,
            ..FakeDispatch::default()
        };

        assert!(serve_connection(&mut io, &mut dispatch).is_err());
        assert!(dispatch.executed.is_empty());
        assert_eq!(dispatch.aborted, vec![7]);
        assert!(dispatch.disconnected);
    }

    #[test]
    fn partial_header_and_body_survive_multiple_host_polls() {
        let (server, client) = connected_test_pipe();
        let expected =
            crate::journal_broker::framing::encode_frame(&[9, 8, 7, 6, 5]).expect("test frame");
        write_exact_bounded(
            &client,
            expected[..2].to_vec(),
            Instant::now() + Duration::from_secs(1),
        )
        .expect("partial header");
        let mut io = NamedPipeHostIo::new(&server, None);

        assert!(matches!(io.read_frame(), Ok(HostRead::Idle)));
        write_exact_bounded(
            &client,
            expected[2..6].to_vec(),
            Instant::now() + Duration::from_secs(1),
        )
        .expect("rest of header and partial body");
        assert!(matches!(io.read_frame(), Ok(HostRead::Idle)));
        write_exact_bounded(
            &client,
            expected[6..].to_vec(),
            Instant::now() + Duration::from_secs(1),
        )
        .expect("rest of body");

        let deadline = Instant::now() + Duration::from_secs(1);
        loop {
            match io.read_frame().expect("poll frame") {
                HostRead::Frame(actual) => {
                    assert_eq!(actual, expected);
                    break;
                }
                HostRead::Idle if Instant::now() < deadline => {}
                HostRead::Idle => panic!("partial frame did not complete"),
                HostRead::Closed => panic!("pipe closed before frame completion"),
            }
        }
    }

    #[test]
    fn connection_deadlines_are_idle_and_absolute() {
        let now = Instant::now();
        assert!(!connection_deadline_reached(
            now + Duration::from_secs(9),
            now,
            now + Duration::from_secs(5),
            Duration::from_secs(5),
            Duration::from_secs(10),
        ));
        assert!(connection_deadline_reached(
            now + Duration::from_secs(10),
            now,
            now + Duration::from_secs(9),
            Duration::from_secs(5),
            Duration::from_secs(10),
        ));
        assert!(connection_deadline_reached(
            now + Duration::from_secs(6),
            now,
            now,
            Duration::from_secs(5),
            Duration::from_secs(10),
        ));
    }

    #[test]
    fn one_connected_client_does_not_consume_the_second_listener() {
        let pipe_name = unique_test_pipe_name("listeners");
        let first = create_pipe(
            &pipe_name,
            DISPOSABLE_PIPE_SDDL,
            true,
            true,
            MAX_PRODUCTION_CONNECTIONS,
        )
        .expect("first listener");
        let second = create_pipe(
            &pipe_name,
            DISPOSABLE_PIPE_SDDL,
            true,
            false,
            MAX_PRODUCTION_CONNECTIONS,
        )
        .expect("second listener");
        let first_client = spawn_test_client(pipe_name.clone());
        connect_bounded(&first, Instant::now() + Duration::from_secs(1)).expect("first connection");
        let first_client = first_client.join().expect("first client thread");

        let second_client = spawn_test_client(pipe_name);
        connect_bounded(&second, Instant::now() + Duration::from_secs(1))
            .expect("second connection while first remains open");
        let second_client = second_client.join().expect("second client thread");
        drop((first_client, second_client));
    }

    #[test]
    fn global_worker_budget_is_shared_and_reclaimed() {
        let budget = ReadWorkerBudget::new(2);
        let first = budget.try_acquire().expect("first permit");
        let second = budget.try_acquire().expect("second permit");
        assert!(matches!(
            budget.try_acquire(),
            Err(HostError::DispatchBackpressure)
        ));
        drop(first);
        assert!(budget.try_acquire().is_ok());
        drop(second);
    }

    #[test]
    fn stop_signal_drains_eight_active_workers_without_detaching() {
        let cancelled = Arc::new(AtomicBool::new(false));
        let mut workers = Vec::new();
        for _ in 0..MAX_HOST_READ_WORKERS {
            let cancelled = Arc::clone(&cancelled);
            workers.push(std::thread::spawn(move || {
                while !cancelled.load(Ordering::Acquire) {
                    std::thread::yield_now();
                }
            }));
        }
        cancelled.store(true, Ordering::Release);

        assert!(drain_worker_handles(
            &mut workers,
            Instant::now() + Duration::from_secs(1),
        ));
        assert!(workers.is_empty());
    }

    fn connected_test_pipe() -> (OwnedHandle, OwnedHandle) {
        let pipe_name = unique_test_pipe_name("partial");
        let server =
            create_pipe(&pipe_name, DISPOSABLE_PIPE_SDDL, true, true, 1).expect("server pipe");
        let client = spawn_test_client(pipe_name);
        connect_bounded(&server, Instant::now() + Duration::from_secs(1)).expect("connect");
        (server, client.join().expect("client thread"))
    }

    fn unique_test_pipe_name(label: &str) -> String {
        static NEXT_PIPE: AtomicU64 = AtomicU64::new(1);
        format!(
            "{CONSOLE_PIPE_PREFIX}{}.{label}.{}",
            std::process::id(),
            NEXT_PIPE.fetch_add(1, Ordering::AcqRel),
        )
    }

    fn spawn_test_client(pipe_name: String) -> JoinHandle<OwnedHandle> {
        std::thread::spawn(move || {
            let mut name: Vec<u16> = pipe_name.encode_utf16().collect();
            name.push(0);
            // SAFETY: the unique test pipe name is terminated and live for both bounded calls.
            assert_ne!(unsafe { WaitNamedPipeW(name.as_ptr(), 1_000) }, 0);
            // SAFETY: the terminated name is live, the test requests only pipe read/write access,
            // and the checked handle is transferred into its unique RAII owner.
            let raw = unsafe {
                CreateFileW(
                    name.as_ptr(),
                    GENERIC_READ | GENERIC_WRITE,
                    0,
                    null(),
                    OPEN_EXISTING,
                    FILE_FLAG_OVERLAPPED,
                    null_mut(),
                )
            };
            OwnedHandle::new(raw).expect("client pipe")
        })
    }
}
