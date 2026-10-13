use std::io;
use std::ptr::null;
use std::thread;
use std::time::{Duration, Instant};

use windows_sys::Win32::Foundation::{
    ERROR_SERVICE_ALREADY_RUNNING, ERROR_SERVICE_CANNOT_ACCEPT_CTRL,
};
use windows_sys::Win32::System::Services::{
    SERVICE_RUNNING, SERVICE_START_PENDING, SERVICE_STATUS_PROCESS, SERVICE_STOP_PENDING,
    SERVICE_STOPPED, StartServiceW,
};

use super::{ServiceHandle, WindowsTransportError, query_status};

const START_TIMEOUT: Duration = Duration::from_secs(5);
const RETIREMENT_OBSERVATION_TIMEOUT: Duration = Duration::from_millis(500);
const STATUS_POLL_INTERVAL: Duration = Duration::from_millis(25);

pub(super) fn activate(service: &ServiceHandle) -> Result<u32, WindowsTransportError> {
    let deadline = Instant::now() + START_TIMEOUT;
    let mut activation = Activation::default();
    loop {
        if Instant::now() >= deadline {
            return Err(WindowsTransportError::TimedOut);
        }
        match activation.observe(&query_status(service)?)? {
            NextStep::Ready(process_id) => return Ok(process_id),
            NextStep::Wait => {}
            NextStep::Start => {
                // SAFETY: this fixed SCM handle has start access and no arguments are supplied.
                if unsafe { StartServiceW(service.raw(), 0, null()) } != 0 {
                    activation.started = true;
                } else {
                    let error = io::Error::last_os_error();
                    match error.raw_os_error().map(|value| value as u32) {
                        Some(ERROR_SERVICE_ALREADY_RUNNING | ERROR_SERVICE_CANNOT_ACCEPT_CTRL) => {}
                        _ => return Err(WindowsTransportError::Io(error)),
                    }
                }
            }
        }
        thread::sleep(STATUS_POLL_INTERVAL);
    }
}

#[derive(Default)]
struct Activation {
    started: bool,
}

#[derive(Debug, PartialEq, Eq)]
enum NextStep {
    Ready(u32),
    Start,
    Wait,
}

impl Activation {
    fn observe(
        &mut self,
        status: &SERVICE_STATUS_PROCESS,
    ) -> Result<NextStep, WindowsTransportError> {
        match status.dwCurrentState {
            SERVICE_RUNNING if status.dwProcessId != 0 => Ok(NextStep::Ready(status.dwProcessId)),
            SERVICE_STOP_PENDING => Ok(NextStep::Wait),
            SERVICE_START_PENDING => {
                self.started = true;
                Ok(NextStep::Wait)
            }
            SERVICE_STOPPED if !self.started => Ok(NextStep::Start),
            SERVICE_STOPPED if status.dwWin32ExitCode != 0 => Err(WindowsTransportError::Io(
                io::Error::from_raw_os_error(status.dwWin32ExitCode as i32),
            )),
            _ => Err(WindowsTransportError::ServerIdentityRejected),
        }
    }
}

pub(super) fn wait_for_retirement(
    service: &ServiceHandle,
    process_id: u32,
) -> Result<bool, WindowsTransportError> {
    observe_retirement(
        process_id,
        || query_status(service),
        Instant::now() + RETIREMENT_OBSERVATION_TIMEOUT,
    )
}

fn observe_retirement(
    process_id: u32,
    mut query: impl FnMut() -> Result<SERVICE_STATUS_PROCESS, WindowsTransportError>,
    deadline: Instant,
) -> Result<bool, WindowsTransportError> {
    loop {
        let status = query()?;
        if process_has_retired(&status, process_id) {
            return Ok(true);
        }
        if status.dwCurrentState != SERVICE_RUNNING || status.dwProcessId != process_id {
            return Ok(false);
        }
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Ok(false);
        }
        // Pipe retirement can precede the SCM status update. Observation alone never authorizes
        // another connection; an actual process transition must still be reported before expiry.
        thread::sleep(remaining.min(STATUS_POLL_INTERVAL));
    }
}

fn process_has_retired(status: &SERVICE_STATUS_PROCESS, process_id: u32) -> bool {
    match status.dwCurrentState {
        SERVICE_RUNNING => status.dwProcessId != 0 && status.dwProcessId != process_id,
        SERVICE_STOPPED | SERVICE_STOP_PENDING | SERVICE_START_PENDING => true,
        _ => false,
    }
}

#[cfg(test)]
mod tests;
