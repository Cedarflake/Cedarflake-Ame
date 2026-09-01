use std::collections::{HashMap, HashSet, VecDeque};
use std::io;
use std::ptr::{null, null_mut};
use std::time::{Duration, Instant};

use thiserror::Error;
use windows_sys::Win32::Foundation::{
    CloseHandle, ERROR_BROKEN_PIPE, ERROR_IO_INCOMPLETE, ERROR_IO_PENDING, ERROR_NO_DATA,
    GENERIC_READ, GENERIC_WRITE, HANDLE, INVALID_HANDLE_VALUE, WAIT_OBJECT_0,
};
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, FILE_FLAG_OVERLAPPED, OPEN_EXISTING, ReadFile, SECURITY_IMPERSONATION,
    SECURITY_SQOS_PRESENT, WriteFile,
};
use windows_sys::Win32::System::IO::{CancelIoEx, GetOverlappedResult, OVERLAPPED};
use windows_sys::Win32::System::Pipes::WaitNamedPipeW;
use windows_sys::Win32::System::Threading::{CreateEventW, ResetEvent, WaitForSingleObject};

use super::connection::{
    HANDSHAKE_BYTES, SERVER_ACCEPT_BYTES, ServerConnectionIdentity, decode_handshake,
    encode_client_proof, validate_server_accept,
};
use super::service_identity::{authenticate_pipe_server, demand_start_service};
use crate::journal_broker::client::{
    AdapterPoll, BrokerClientTransport, JournalBrokerClient, OwnedBrokerConnection, sealed,
};
use crate::journal_broker::framing::{decode_frame, decode_frame_length};
use crate::journal_broker::wire::decode_response;
use crate::journal_broker::{
    PersistentChangeJournal, PersistentChangeJournalConnection,
    PersistentChangeJournalLiveOnlyReason,
};

pub(super) const PRODUCTION_PIPE_NAME: &str = r"\\.\pipe\CedarflakeAme.JournalBroker.v3";
const CONNECT_TIMEOUT_MS: u32 = 250;
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(2);
const DROP_CANCEL_TIMEOUT_MS: u32 = 1_000;
const MAX_BUFFERED_RESPONSE_REQUESTS: usize = 12;
const MAX_BUFFERED_RESPONSE_FRAMES: usize = 20;
const MAX_DISCARDED_REQUESTS: usize = 16;
const MAX_ABANDON_CANCELS: usize = 2;

pub(crate) struct WindowsPersistentChangeJournal;

impl PersistentChangeJournal for WindowsPersistentChangeJournal {
    fn connect(&self) -> PersistentChangeJournalConnection {
        if !super::service_identity::current_process_has_installed_client_identity() {
            return PersistentChangeJournalConnection::LiveOnly(
                PersistentChangeJournalLiveOnlyReason::PortableDistribution,
            );
        }
        connection_from_result(WindowsNamedPipeTransport::connect(PRODUCTION_PIPE_NAME))
    }
}

fn connection_from_result(
    result: Result<(WindowsNamedPipeTransport, ServerConnectionIdentity), WindowsTransportError>,
) -> PersistentChangeJournalConnection {
    match result {
        Ok((transport, identity)) => {
            match OwnedBrokerConnection::from_adapter(
                transport,
                identity.connection_id,
                identity.connection_generation,
                identity.connection_nonce,
            ) {
                Ok(connection) => PersistentChangeJournalConnection::Connected(
                    std::sync::Arc::new(JournalBrokerClient::new(connection)),
                ),
                Err(_) => PersistentChangeJournalConnection::LiveOnly(
                    PersistentChangeJournalLiveOnlyReason::ProtocolMismatch,
                ),
            }
        }
        Err(WindowsTransportError::PortableDistribution) => {
            PersistentChangeJournalConnection::LiveOnly(
                PersistentChangeJournalLiveOnlyReason::PortableDistribution,
            )
        }
        Err(WindowsTransportError::BrokerAbsent) => PersistentChangeJournalConnection::LiveOnly(
            PersistentChangeJournalLiveOnlyReason::BrokerAbsent,
        ),
        Err(WindowsTransportError::ProtocolMismatch) => {
            PersistentChangeJournalConnection::LiveOnly(
                PersistentChangeJournalLiveOnlyReason::ProtocolMismatch,
            )
        }
        Err(_) => PersistentChangeJournalConnection::LiveOnly(
            PersistentChangeJournalLiveOnlyReason::TransportUnavailable,
        ),
    }
}

pub(crate) struct WindowsNamedPipeTransport {
    pipe: OwnedHandle,
    pending_write: Option<PendingFrameWrite>,
    pending_read: Option<PendingFrameRead>,
    responses: HashMap<u64, VecDeque<Vec<u8>>>,
    discarded: HashSet<u64>,
    discarded_order: VecDeque<u64>,
    abandon_cancels: HashMap<u64, AbandonCancel>,
    is_closing: bool,
}

impl WindowsNamedPipeTransport {
    pub(super) fn connect(
        pipe_name: &str,
    ) -> Result<(Self, ServerConnectionIdentity), WindowsTransportError> {
        if !super::service_identity::current_process_has_installed_client_identity() {
            return Err(WindowsTransportError::PortableDistribution);
        }
        let started_service = demand_start_service()?;
        let pipe_name = nul_terminated(pipe_name)?;
        // SAFETY: pipe_name is bounded, NUL-terminated, and live for the call. This wait does not
        // create or mutate a journal and is bounded by CONNECT_TIMEOUT_MS.
        if unsafe { WaitNamedPipeW(pipe_name.as_ptr(), CONNECT_TIMEOUT_MS) } == 0 {
            return Err(map_connect_error(io::Error::last_os_error()));
        }
        // SAFETY: all pointers are either null or point to the live, terminated pipe name. The
        // returned handle is checked before becoming the unique OwnedHandle owner.
        let raw = unsafe {
            CreateFileW(
                pipe_name.as_ptr(),
                GENERIC_READ | GENERIC_WRITE,
                0,
                null(),
                OPEN_EXISTING,
                FILE_FLAG_OVERLAPPED | SECURITY_SQOS_PRESENT | SECURITY_IMPERSONATION,
                null_mut(),
            )
        };
        let pipe = OwnedHandle::new(raw).map_err(map_connect_error)?;
        authenticate_pipe_server(&pipe, &started_service)?;
        let mut handshake = PendingIo::start_read(&pipe, vec![0; HANDSHAKE_BYTES])?;
        let deadline = Instant::now() + HANDSHAKE_TIMEOUT;
        let bytes = loop {
            match handshake.poll(&pipe)? {
                AdapterPoll::Ready(bytes) => break bytes,
                AdapterPoll::Pending if Instant::now() < deadline => handshake.wait_slice(10),
                AdapterPoll::Pending => {
                    handshake.cancel_and_drain(&pipe);
                    return Err(WindowsTransportError::TimedOut);
                }
            }
        };
        let identity =
            decode_handshake(&bytes).map_err(|_| WindowsTransportError::ProtocolMismatch)?;
        write_exact_bounded(
            &pipe,
            encode_client_proof(identity).to_vec(),
            Instant::now() + HANDSHAKE_TIMEOUT,
        )?;
        let accepted = read_exact_bounded(
            &pipe,
            SERVER_ACCEPT_BYTES,
            Instant::now() + HANDSHAKE_TIMEOUT,
        )?;
        validate_server_accept(identity, &accepted)
            .map_err(|_| WindowsTransportError::ProtocolMismatch)?;
        Ok((
            Self {
                pipe,
                pending_write: None,
                pending_read: None,
                responses: HashMap::new(),
                discarded: HashSet::new(),
                discarded_order: VecDeque::new(),
                abandon_cancels: HashMap::new(),
                is_closing: false,
            },
            identity,
        ))
    }

    fn poll_write(
        &mut self,
        request_frame: &[u8],
    ) -> Result<AdapterPoll<()>, WindowsTransportError> {
        let Some(write) = self.pending_write.as_mut() else {
            return Ok(AdapterPoll::Ready(()));
        };
        if !write.belongs_to(request_frame) {
            return Ok(AdapterPoll::Pending);
        }
        match write.operation.poll(&self.pipe)? {
            AdapterPoll::Ready(_) => {
                self.pending_write = None;
                Ok(AdapterPoll::Ready(()))
            }
            AdapterPoll::Pending => Ok(AdapterPoll::Pending),
        }
    }

    fn pump_read(&mut self) -> Result<(), WindowsTransportError> {
        if self.pending_read.is_none() {
            self.pending_read = Some(PendingFrameRead::start(&self.pipe)?);
        }
        let Some(read) = self.pending_read.as_mut() else {
            return Ok(());
        };
        if let AdapterPoll::Ready(frame) = read.poll(&self.pipe)? {
            self.pending_read = None;
            let response = decode_response(
                decode_frame(&frame).map_err(|_| WindowsTransportError::ProtocolMismatch)?,
            )
            .map_err(|_| WindowsTransportError::ProtocolMismatch)?;
            let request_id = response.request_id();
            self.buffer_response(request_id, frame)?;
        }
        Ok(())
    }

    fn take_response(&mut self, request_id: u64) -> Option<Vec<u8>> {
        let queue = self.responses.get_mut(&request_id)?;
        let frame = queue.pop_front();
        if queue.is_empty() {
            self.responses.remove(&request_id);
        }
        frame
    }

    fn discard(&mut self, request_id: u64) {
        if self.responses.remove(&request_id).is_some() {
            self.remove_discard(request_id);
            return;
        }
        if self.discarded.insert(request_id) {
            self.discarded_order.push_back(request_id);
        }
        while self.discarded_order.len() > MAX_DISCARDED_REQUESTS {
            if let Some(expired) = self.discarded_order.pop_front() {
                self.discarded.remove(&expired);
            }
        }
    }

    fn remove_discard(&mut self, request_id: u64) {
        if self.discarded.remove(&request_id) {
            self.discarded_order.retain(|id| *id != request_id);
        }
    }

    fn buffer_response(
        &mut self,
        request_id: u64,
        frame: Vec<u8>,
    ) -> Result<(), WindowsTransportError> {
        if self.discarded.contains(&request_id) {
            self.remove_discard(request_id);
            return Ok(());
        }
        let buffered_frames: usize = self.responses.values().map(VecDeque::len).sum();
        if buffered_frames >= MAX_BUFFERED_RESPONSE_FRAMES
            || (!self.responses.contains_key(&request_id)
                && self.responses.len() >= MAX_BUFFERED_RESPONSE_REQUESTS)
        {
            return Err(WindowsTransportError::Backpressure);
        }
        self.responses
            .entry(request_id)
            .or_default()
            .push_back(frame);
        Ok(())
    }

    fn cancel_pending(&mut self) -> bool {
        let mut drained = true;
        if let Some(write) = self.pending_write.as_mut() {
            drained &= write.operation.cancel_and_drain(&self.pipe);
        }
        if let Some(read) = self.pending_read.as_mut() {
            drained &= read.cancel_and_drain(&self.pipe);
        }
        if !drained {
            if let Some(write) = self.pending_write.take() {
                std::mem::forget(write);
            }
            if let Some(read) = self.pending_read.take() {
                std::mem::forget(read);
            }
        } else {
            self.pending_write = None;
            self.pending_read = None;
        }
        self.responses.clear();
        self.discarded.clear();
        self.discarded_order.clear();
        self.abandon_cancels.clear();
        drained
    }
}

impl sealed::Sealed for WindowsNamedPipeTransport {}

impl BrokerClientTransport for WindowsNamedPipeTransport {
    type Error = WindowsTransportError;

    fn try_send_frame(&mut self, request_frame: &[u8]) -> Result<AdapterPoll<()>, Self::Error> {
        if self.is_closing {
            return Err(WindowsTransportError::Closed);
        }
        if self.pending_write.is_none() {
            self.pending_write = Some(PendingFrameWrite {
                frame: request_frame.to_vec(),
                operation: PendingIo::start_write(&self.pipe, request_frame.to_vec())?,
            });
        }
        self.poll_write(request_frame)
    }

    fn poll_response(&mut self, request_id: u64) -> Result<AdapterPoll<Vec<u8>>, Self::Error> {
        if let Some(frame) = self.take_response(request_id) {
            return Ok(AdapterPoll::Ready(frame));
        }
        if self.is_closing {
            return Err(WindowsTransportError::Closed);
        }
        self.pump_read()?;
        Ok(self
            .take_response(request_id)
            .map_or(AdapterPoll::Pending, AdapterPoll::Ready))
    }

    fn try_discard(&mut self, request_id: u64) -> Result<AdapterPoll<()>, Self::Error> {
        self.discard(request_id);
        Ok(AdapterPoll::Ready(()))
    }

    fn begin_abandon(
        &mut self,
        target_request_id: u64,
        cancel_request: Option<(u64, Vec<u8>)>,
        close_connection: bool,
    ) -> Result<AdapterPoll<()>, Self::Error> {
        self.discard(target_request_id);
        if close_connection {
            return self.begin_close();
        }
        let Some((cancel_request_id, frame)) = cancel_request else {
            return Ok(AdapterPoll::Ready(()));
        };
        if !self.abandon_cancels.contains_key(&target_request_id) {
            if self.abandon_cancels.len() >= MAX_ABANDON_CANCELS {
                return Err(WindowsTransportError::Backpressure);
            }
            self.abandon_cancels.insert(
                target_request_id,
                AbandonCancel {
                    request_id: cancel_request_id,
                    frame,
                    is_sent: false,
                },
            );
        }
        self.poll_abandon_send(target_request_id)
    }

    fn poll_abandon(&mut self, target_request_id: u64) -> Result<AdapterPoll<()>, Self::Error> {
        match self.poll_abandon_send(target_request_id)? {
            AdapterPoll::Pending => return Ok(AdapterPoll::Pending),
            AdapterPoll::Ready(()) => {}
        }
        let cancel_request_id = self
            .abandon_cancels
            .get(&target_request_id)
            .map(|cancel| cancel.request_id)
            .ok_or(WindowsTransportError::ProtocolMismatch)?;
        match self.poll_response(cancel_request_id)? {
            AdapterPoll::Ready(_) => {
                self.discard(cancel_request_id);
                self.abandon_cancels.remove(&target_request_id);
                Ok(AdapterPoll::Ready(()))
            }
            AdapterPoll::Pending => Ok(AdapterPoll::Pending),
        }
    }

    fn begin_close(&mut self) -> Result<AdapterPoll<()>, Self::Error> {
        self.is_closing = true;
        if self.cancel_pending() {
            Ok(AdapterPoll::Ready(()))
        } else {
            Err(WindowsTransportError::TimedOut)
        }
    }

    fn poll_close(&mut self) -> Result<AdapterPoll<()>, Self::Error> {
        if self.cancel_pending() {
            Ok(AdapterPoll::Ready(()))
        } else {
            Err(WindowsTransportError::TimedOut)
        }
    }
}

impl WindowsNamedPipeTransport {
    fn poll_abandon_send(
        &mut self,
        target_request_id: u64,
    ) -> Result<AdapterPoll<()>, WindowsTransportError> {
        let Some(cancel) = self.abandon_cancels.get(&target_request_id) else {
            return Ok(AdapterPoll::Ready(()));
        };
        if cancel.is_sent {
            return Ok(AdapterPoll::Ready(()));
        }
        let frame = cancel.frame.clone();
        match self.try_send_frame(&frame)? {
            AdapterPoll::Ready(()) => {
                self.abandon_cancels
                    .get_mut(&target_request_id)
                    .ok_or(WindowsTransportError::ProtocolMismatch)?
                    .is_sent = true;
                Ok(AdapterPoll::Ready(()))
            }
            AdapterPoll::Pending => Ok(AdapterPoll::Pending),
        }
    }
}

impl Drop for WindowsNamedPipeTransport {
    fn drop(&mut self) {
        self.is_closing = true;
        let _ = self.cancel_pending();
    }
}

struct PendingFrameWrite {
    frame: Vec<u8>,
    operation: PendingIo,
}

struct AbandonCancel {
    request_id: u64,
    frame: Vec<u8>,
    is_sent: bool,
}

impl PendingFrameWrite {
    fn belongs_to(&self, request_frame: &[u8]) -> bool {
        frame_identity_matches(&self.frame, request_frame)
    }
}

fn frame_identity_matches(active_frame: &[u8], request_frame: &[u8]) -> bool {
    active_frame == request_frame
}

pub(super) struct PendingFrameRead {
    state: FrameReadState,
}

enum FrameReadState {
    Header(PendingIo),
    Payload { header: [u8; 4], payload: PendingIo },
}

impl PendingFrameRead {
    pub(super) fn start(pipe: &OwnedHandle) -> Result<Self, WindowsTransportError> {
        Ok(Self {
            state: FrameReadState::Header(PendingIo::start_read(pipe, vec![0; 4])?),
        })
    }

    pub(super) fn poll(
        &mut self,
        pipe: &OwnedHandle,
    ) -> Result<AdapterPoll<Vec<u8>>, WindowsTransportError> {
        match &mut self.state {
            FrameReadState::Header(header) => match header.poll(pipe)? {
                AdapterPoll::Pending => Ok(AdapterPoll::Pending),
                AdapterPoll::Ready(bytes) => {
                    let header_bytes: [u8; 4] = bytes
                        .try_into()
                        .map_err(|_| WindowsTransportError::ProtocolMismatch)?;
                    let length = decode_frame_length(header_bytes)
                        .map_err(|_| WindowsTransportError::ProtocolMismatch)?;
                    self.state = FrameReadState::Payload {
                        header: header_bytes,
                        payload: PendingIo::start_read(pipe, vec![0; length])?,
                    };
                    Ok(AdapterPoll::Pending)
                }
            },
            FrameReadState::Payload { header, payload } => match payload.poll(pipe)? {
                AdapterPoll::Pending => Ok(AdapterPoll::Pending),
                AdapterPoll::Ready(payload) => {
                    let mut frame = Vec::with_capacity(4 + payload.len());
                    frame.extend_from_slice(header);
                    frame.extend_from_slice(&payload);
                    Ok(AdapterPoll::Ready(frame))
                }
            },
        }
    }

    pub(super) fn wait_slice(&self, milliseconds: u32) {
        match &self.state {
            FrameReadState::Header(operation)
            | FrameReadState::Payload {
                payload: operation, ..
            } => operation.wait_slice(milliseconds),
        }
    }

    pub(super) fn cancel_and_drain(&mut self, pipe: &OwnedHandle) -> bool {
        match &mut self.state {
            FrameReadState::Header(operation)
            | FrameReadState::Payload {
                payload: operation, ..
            } => operation.cancel_and_drain(pipe),
        }
    }
}

struct PendingIo {
    overlapped: Box<OVERLAPPED>,
    event: OwnedHandle,
    buffer: Vec<u8>,
    completed: usize,
    direction: IoDirection,
}

// SAFETY: PendingIo uniquely owns its stable OVERLAPPED allocation, event, and buffer. It is only
// accessed through the connection dispatcher's transport mutex; moving ownership between threads
// does not move the boxed OVERLAPPED or resize the buffer while a kernel operation is pending.
unsafe impl Send for PendingIo {}

impl PendingIo {
    fn start_read(pipe: &OwnedHandle, buffer: Vec<u8>) -> Result<Self, WindowsTransportError> {
        Self::start(pipe, buffer, IoDirection::Read)
    }

    fn start_write(pipe: &OwnedHandle, buffer: Vec<u8>) -> Result<Self, WindowsTransportError> {
        Self::start(pipe, buffer, IoDirection::Write)
    }

    fn start(
        pipe: &OwnedHandle,
        buffer: Vec<u8>,
        direction: IoDirection,
    ) -> Result<Self, WindowsTransportError> {
        if buffer.is_empty() || u32::try_from(buffer.len()).is_err() {
            return Err(WindowsTransportError::ProtocolMismatch);
        }
        // SAFETY: null security attributes and name request a private manual-reset event. The
        // returned handle is validated before ownership.
        let event = OwnedHandle::new(unsafe { CreateEventW(null(), 1, 0, null()) })?;
        let mut overlapped = Box::<OVERLAPPED>::default();
        overlapped.hEvent = event.raw();
        let mut operation = Self {
            overlapped,
            event,
            buffer,
            completed: 0,
            direction,
        };
        operation.issue(pipe)?;
        Ok(operation)
    }

    fn issue(&mut self, pipe: &OwnedHandle) -> Result<(), WindowsTransportError> {
        let remaining = self
            .buffer
            .len()
            .checked_sub(self.completed)
            .ok_or(WindowsTransportError::ProtocolMismatch)?;
        let count =
            u32::try_from(remaining).map_err(|_| WindowsTransportError::ProtocolMismatch)?;
        // SAFETY: no I/O is pending for this object here. The manual-reset event remains owned,
        // the OVERLAPPED allocation is stable, and the buffer is not resized or moved until the
        // operation reports completion.
        unsafe {
            let _ = ResetEvent(self.event.raw());
            *self.overlapped = OVERLAPPED::default();
            self.overlapped.hEvent = self.event.raw();
        }
        let buffer = self.buffer.as_mut_ptr();
        // SAFETY: buffer plus completed points to count live bytes, the stable OVERLAPPED remains
        // alive until completion, and pipe is the unique live owner of a valid overlapped handle.
        let succeeded = unsafe {
            match self.direction {
                IoDirection::Read => ReadFile(
                    pipe.raw(),
                    buffer.add(self.completed),
                    count,
                    null_mut(),
                    self.overlapped.as_mut(),
                ),
                IoDirection::Write => WriteFile(
                    pipe.raw(),
                    buffer.add(self.completed),
                    count,
                    null_mut(),
                    self.overlapped.as_mut(),
                ),
            }
        };
        if succeeded == 0 {
            let error = io::Error::last_os_error();
            if error.raw_os_error() != Some(ERROR_IO_PENDING as i32) {
                return Err(map_pipe_io_error(error));
            }
        }
        Ok(())
    }

    fn poll(&mut self, pipe: &OwnedHandle) -> Result<AdapterPoll<Vec<u8>>, WindowsTransportError> {
        let mut transferred = 0_u32;
        // SAFETY: the pipe, stable OVERLAPPED, event, and buffer remain live. FALSE makes this a
        // non-blocking completion query.
        let completed = unsafe {
            GetOverlappedResult(pipe.raw(), self.overlapped.as_ref(), &mut transferred, 0)
        };
        if completed == 0 {
            let error = io::Error::last_os_error();
            if error.raw_os_error() == Some(ERROR_IO_INCOMPLETE as i32) {
                return Ok(AdapterPoll::Pending);
            }
            return Err(map_pipe_io_error(error));
        }
        if transferred == 0 {
            return Err(WindowsTransportError::Closed);
        }
        self.completed = self
            .completed
            .checked_add(transferred as usize)
            .ok_or(WindowsTransportError::ProtocolMismatch)?;
        if self.completed > self.buffer.len() {
            return Err(WindowsTransportError::ProtocolMismatch);
        }
        if self.completed == self.buffer.len() {
            return Ok(AdapterPoll::Ready(std::mem::take(&mut self.buffer)));
        }
        self.issue(pipe)?;
        Ok(AdapterPoll::Pending)
    }

    fn wait_slice(&self, milliseconds: u32) {
        // SAFETY: the event is a live waitable handle and the wait is explicitly bounded.
        let _ = unsafe { WaitForSingleObject(self.event.raw(), milliseconds) };
    }

    fn cancel_and_drain(&mut self, pipe: &OwnedHandle) -> bool {
        // SAFETY: the OVERLAPPED belongs to this pipe and remains live through the bounded wait.
        let _ = unsafe { CancelIoEx(pipe.raw(), self.overlapped.as_ref()) };
        // SAFETY: the event is a live waitable handle and the wait is bounded. If completion is not
        // observed, the caller leaks the operation rather than freeing live kernel pointers.
        unsafe { WaitForSingleObject(self.event.raw(), DROP_CANCEL_TIMEOUT_MS) == WAIT_OBJECT_0 }
    }
}

#[derive(Clone, Copy)]
enum IoDirection {
    Read,
    Write,
}

pub(super) struct OwnedHandle(HANDLE);

impl OwnedHandle {
    pub(super) fn new(raw: HANDLE) -> Result<Self, io::Error> {
        if raw.is_null() || raw == INVALID_HANDLE_VALUE {
            Err(io::Error::last_os_error())
        } else {
            Ok(Self(raw))
        }
    }

    pub(super) fn raw(&self) -> HANDLE {
        self.0
    }
}

// SAFETY: HANDLE is an opaque kernel object reference. OwnedHandle has exactly one owner, is not
// Clone, closes once in Drop, and exposes no API that transfers ownership or aliases mutable Rust
// memory. Operation buffers remain inside the transport mutex while Win32 uses their pointers.
unsafe impl Send for OwnedHandle {}

impl Drop for OwnedHandle {
    fn drop(&mut self) {
        // SAFETY: construction rejected null and INVALID_HANDLE_VALUE and this unique owner closes
        // the handle exactly once.
        let _ = unsafe { CloseHandle(self.0) };
    }
}

#[derive(Debug, Error)]
pub(crate) enum WindowsTransportError {
    #[error("portable distributions cannot activate the installed journal broker")]
    PortableDistribution,
    #[error("journal broker is not installed or running")]
    BrokerAbsent,
    #[error("journal broker protocol is incompatible")]
    ProtocolMismatch,
    #[error("journal broker I/O exceeded its bounded deadline")]
    TimedOut,
    #[error("journal broker connection is closed")]
    Closed,
    #[error("journal broker transport buffer limit was exceeded")]
    Backpressure,
    #[error("journal broker server identity was rejected")]
    ServerIdentityRejected,
    #[error("journal broker named-pipe I/O failed: {0}")]
    Io(#[from] io::Error),
}

fn nul_terminated(value: &str) -> Result<Vec<u16>, WindowsTransportError> {
    if value.is_empty() || value.encode_utf16().any(|unit| unit == 0) {
        return Err(WindowsTransportError::ProtocolMismatch);
    }
    let mut units: Vec<u16> = value.encode_utf16().collect();
    units.push(0);
    Ok(units)
}

fn map_connect_error(error: io::Error) -> WindowsTransportError {
    match error.raw_os_error() {
        Some(2 | 3 | 53 | 67 | 121 | 231) => WindowsTransportError::BrokerAbsent,
        _ => WindowsTransportError::Io(error),
    }
}

fn map_pipe_io_error(error: io::Error) -> WindowsTransportError {
    match error.raw_os_error().map(|value| value as u32) {
        Some(ERROR_BROKEN_PIPE | ERROR_NO_DATA) => WindowsTransportError::Closed,
        _ => WindowsTransportError::Io(error),
    }
}

pub(super) fn write_exact_bounded(
    pipe: &OwnedHandle,
    bytes: Vec<u8>,
    deadline: Instant,
) -> Result<(), WindowsTransportError> {
    let operation = PendingIo::start_write(pipe, bytes)?;
    poll_exact_bounded(pipe, operation, deadline).map(|_| ())
}

pub(super) fn read_exact_bounded(
    pipe: &OwnedHandle,
    byte_count: usize,
    deadline: Instant,
) -> Result<Vec<u8>, WindowsTransportError> {
    let operation = PendingIo::start_read(pipe, vec![0; byte_count])?;
    poll_exact_bounded(pipe, operation, deadline)
}

fn poll_exact_bounded(
    pipe: &OwnedHandle,
    mut operation: PendingIo,
    deadline: Instant,
) -> Result<Vec<u8>, WindowsTransportError> {
    loop {
        match operation.poll(pipe)? {
            AdapterPoll::Ready(bytes) => return Ok(bytes),
            AdapterPoll::Pending if Instant::now() < deadline => operation.wait_slice(10),
            AdapterPoll::Pending => {
                if !operation.cancel_and_drain(pipe) {
                    std::mem::forget(operation);
                }
                return Err(WindowsTransportError::TimedOut);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::journal_broker::windows::connection::{
        ConnectionIdentitySource, ProcessConnectionIdentitySource, encode_handshake,
    };

    #[test]
    fn production_connection_identity_can_only_arrive_in_server_handshake() {
        let identity = ProcessConnectionIdentitySource::new()
            .next_identity()
            .expect("server identity");
        assert_eq!(decode_handshake(&encode_handshake(identity)), Ok(identity));
    }

    #[test]
    fn malformed_pipe_names_fail_before_win32() {
        assert!(matches!(
            nul_terminated("bad\0name"),
            Err(WindowsTransportError::ProtocolMismatch)
        ));
    }

    #[test]
    fn frame_identity_comparison_rejects_a_different_request() {
        let first = vec![4, 0, 0, 0, 1, 2, 3, 4];
        let same = first.clone();
        let mut different_request = first.clone();
        different_request[7] = 5;

        assert!(frame_identity_matches(&first, &same));
        assert!(!frame_identity_matches(&first, &different_request));
    }

    #[test]
    fn foreign_clone_cannot_claim_another_frames_pending_write() {
        let active_frame = vec![4, 0, 0, 0, 1, 2, 3, 4];
        let mut foreign_frame = active_frame.clone();
        foreign_frame[7] = 5;
        // SAFETY: these private manual-reset events are test-only RAII handles. No I/O is issued;
        // the foreign-frame branch must return before consulting the pending operation or pipe.
        let pipe = OwnedHandle::new(unsafe { CreateEventW(null(), 1, 0, null()) })
            .expect("test pipe sentinel");
        // SAFETY: same private-event invariant as the sentinel above.
        let event = OwnedHandle::new(unsafe { CreateEventW(null(), 1, 1, null()) })
            .expect("test operation event");
        let mut overlapped = Box::<OVERLAPPED>::default();
        overlapped.hEvent = event.raw();
        let mut transport = WindowsNamedPipeTransport {
            pipe,
            pending_write: Some(PendingFrameWrite {
                frame: active_frame.clone(),
                operation: PendingIo {
                    overlapped,
                    event,
                    buffer: active_frame.clone(),
                    completed: 0,
                    direction: IoDirection::Write,
                },
            }),
            pending_read: None,
            responses: HashMap::new(),
            discarded: HashSet::new(),
            discarded_order: VecDeque::new(),
            abandon_cancels: HashMap::new(),
            is_closing: false,
        };

        assert!(matches!(
            transport.try_send_frame(&foreign_frame),
            Ok(AdapterPoll::Pending)
        ));
        assert_eq!(
            transport
                .pending_write
                .as_ref()
                .map(|write| write.frame.as_slice()),
            Some(active_frame.as_slice())
        );
    }

    #[test]
    fn demultiplexing_tombstones_and_responses_are_hard_bounded() {
        let mut transport = sentinel_transport();
        for request_id in 1..=100 {
            transport.discard(request_id);
        }
        assert_eq!(transport.discarded.len(), MAX_DISCARDED_REQUESTS);
        assert_eq!(transport.discarded_order.len(), MAX_DISCARDED_REQUESTS);
        assert!(!transport.discarded.contains(&1));
        assert!(transport.discarded.contains(&100));

        transport.discarded.clear();
        transport.discarded_order.clear();
        for index in 0..MAX_BUFFERED_RESPONSE_FRAMES {
            transport
                .buffer_response(7, vec![u8::try_from(index).expect("bounded index")])
                .expect("within response bound");
        }
        assert!(matches!(
            transport.buffer_response(7, vec![255]),
            Err(WindowsTransportError::Backpressure)
        ));
    }

    #[test]
    fn abandon_waits_until_its_cancel_frame_is_actually_sent() {
        let active_frame = vec![4, 0, 0, 0, 1, 2, 3, 4];
        let cancel_frame = vec![4, 0, 0, 0, 4, 3, 2, 1];
        let mut transport = sentinel_transport();
        // SAFETY: the signaled private event is not submitted to Win32. It lets Drop drain the
        // synthetic operation without waiting after the ownership branch has been exercised.
        let event = OwnedHandle::new(unsafe { CreateEventW(null(), 1, 1, null()) })
            .expect("test operation event");
        let mut overlapped = Box::<OVERLAPPED>::default();
        overlapped.hEvent = event.raw();
        transport.pending_write = Some(PendingFrameWrite {
            frame: active_frame.clone(),
            operation: PendingIo {
                overlapped,
                event,
                buffer: active_frame,
                completed: 0,
                direction: IoDirection::Write,
            },
        });

        assert!(matches!(
            transport.begin_abandon(9, Some((10, cancel_frame)), false),
            Ok(AdapterPoll::Pending)
        ));
        assert!(matches!(
            transport.poll_abandon(9),
            Ok(AdapterPoll::Pending)
        ));
        assert_eq!(transport.abandon_cancels.len(), 1);
        assert!(!transport.abandon_cancels[&9].is_sent);
    }

    fn sentinel_transport() -> WindowsNamedPipeTransport {
        // SAFETY: the private event is used only as a non-null RAII handle. Tests that construct
        // this transport do not issue pipe I/O against it.
        let pipe = OwnedHandle::new(unsafe { CreateEventW(null(), 1, 1, null()) })
            .expect("test transport sentinel");
        WindowsNamedPipeTransport {
            pipe,
            pending_write: None,
            pending_read: None,
            responses: HashMap::new(),
            discarded: HashSet::new(),
            discarded_order: VecDeque::new(),
            abandon_cancels: HashMap::new(),
            is_closing: false,
        }
    }

    #[test]
    fn broker_absence_and_protocol_mismatch_are_explicit_live_only_states() {
        assert!(matches!(
            connection_from_result(Err(WindowsTransportError::BrokerAbsent)),
            PersistentChangeJournalConnection::LiveOnly(
                PersistentChangeJournalLiveOnlyReason::BrokerAbsent
            )
        ));
        assert!(matches!(
            connection_from_result(Err(WindowsTransportError::ProtocolMismatch)),
            PersistentChangeJournalConnection::LiveOnly(
                PersistentChangeJournalLiveOnlyReason::ProtocolMismatch
            )
        ));
        assert!(matches!(
            connection_from_result(Err(WindowsTransportError::TimedOut)),
            PersistentChangeJournalConnection::LiveOnly(
                PersistentChangeJournalLiveOnlyReason::TransportUnavailable
            )
        ));
    }
}
