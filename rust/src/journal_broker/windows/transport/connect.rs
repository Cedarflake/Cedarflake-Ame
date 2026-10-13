use std::io;
use std::ptr::{null, null_mut};
use std::time::{Duration, Instant};

use windows_sys::Win32::Foundation::{GENERIC_READ, GENERIC_WRITE};
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, FILE_FLAG_OVERLAPPED, OPEN_EXISTING, SECURITY_IMPERSONATION, SECURITY_SQOS_PRESENT,
};
use windows_sys::Win32::System::Pipes::WaitNamedPipeW;

use super::super::connection::{
    HANDSHAKE_BYTES, SERVER_ACCEPT_BYTES, ServerConnectionIdentity, decode_handshake,
    encode_client_proof, validate_server_accept,
};
use super::super::service_identity::{
    StartedService, authenticate_pipe_server, demand_start_service,
};
use super::{
    OwnedHandle, PendingIo, WindowsTransportError, read_exact_bounded, write_exact_bounded,
};
use crate::journal_broker::client::AdapterPoll;

const CONNECT_TIMEOUT_MS: u32 = 250;
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(2);

pub(super) fn authenticate(
    pipe_name: &str,
) -> Result<(OwnedHandle, ServerConnectionIdentity), WindowsTransportError> {
    connect_with_handover(
        demand_start_service,
        |service| connect_to_service(pipe_name, service),
        StartedService::wait_for_retirement,
    )
}

fn connect_with_handover<Service, Connection>(
    mut start: impl FnMut() -> Result<Service, WindowsTransportError>,
    mut connect: impl FnMut(&Service) -> Result<Connection, WindowsTransportError>,
    retired: impl FnOnce(&Service) -> Result<bool, WindowsTransportError>,
) -> Result<Connection, WindowsTransportError> {
    let service = start()?;
    match connect(&service) {
        Ok(connection) => Ok(connection),
        Err(error) => {
            if !matches!(retired(&service), Ok(true)) {
                return Err(error);
            }
            // The old SCM object must retire before another fixed-service activation. Every
            // check is repeated on the new process, and a second failure is terminal.
            drop(service);
            let replacement = start()?;
            connect(&replacement)
        }
    }
}

fn connect_to_service(
    pipe_name: &str,
    service: &StartedService,
) -> Result<(OwnedHandle, ServerConnectionIdentity), WindowsTransportError> {
    let pipe_name = nul_terminated(pipe_name)?;
    // SAFETY: the terminated pipe name remains live for this bounded, non-mutating wait.
    if unsafe { WaitNamedPipeW(pipe_name.as_ptr(), CONNECT_TIMEOUT_MS) } == 0 {
        return Err(map_connect_error(io::Error::last_os_error()));
    }
    // SAFETY: all pointer arguments are null or refer to the terminated name; the returned
    // handle has a unique checked owner, with overlapped I/O and caller impersonation support.
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
    authenticate_pipe_server(&pipe, service)?;
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
    let identity = decode_handshake(&bytes).map_err(|_| WindowsTransportError::ProtocolMismatch)?;
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
    Ok((pipe, identity))
}

fn nul_terminated(value: &str) -> Result<Vec<u16>, WindowsTransportError> {
    if value.is_empty() || value.encode_utf16().any(|unit| unit == 0) {
        return Err(WindowsTransportError::ProtocolMismatch);
    }
    Ok(value.encode_utf16().chain(Some(0)).collect())
}

fn map_connect_error(error: io::Error) -> WindowsTransportError {
    match error.raw_os_error() {
        Some(2 | 3 | 53 | 67 | 121 | 231) => WindowsTransportError::BrokerAbsent,
        _ => WindowsTransportError::Io(error),
    }
}

#[cfg(test)]
mod tests;
