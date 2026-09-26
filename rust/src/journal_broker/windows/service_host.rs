use std::ffi::c_void;
use std::io;
use std::process::ExitCode;
use std::ptr::{null, null_mut};
use std::sync::atomic::{AtomicBool, AtomicPtr, Ordering};
use std::sync::mpsc::{RecvTimeoutError, sync_channel};
use std::time::{Duration, Instant};

use windows_sys::Win32::Foundation::{
    ERROR_FAILED_SERVICE_CONTROLLER_CONNECT, ERROR_SERVICE_SPECIFIC_ERROR, NO_ERROR,
};
use windows_sys::Win32::System::Services::{
    RegisterServiceCtrlHandlerExW, SERVICE_ACCEPT_STOP, SERVICE_CONTROL_INTERROGATE,
    SERVICE_CONTROL_STOP, SERVICE_RUNNING, SERVICE_START_PENDING, SERVICE_STATUS,
    SERVICE_STOP_PENDING, SERVICE_STOPPED, SERVICE_TABLE_ENTRYW, SERVICE_WIN32_OWN_PROCESS,
    SetServiceStatus, StartServiceCtrlDispatcherW,
};

pub(super) const SERVICE_NAME: &str = "CedarflakeAmeJournalBroker";

static STOP_REQUESTED: AtomicBool = AtomicBool::new(false);
static STATUS_HANDLE: AtomicPtr<c_void> = AtomicPtr::new(null_mut());
const STOP_STATUS_INTERVAL: Duration = Duration::from_millis(500);
const STOP_WAIT_HINT_MS: u32 = 5_000;
const START_READY_TIMEOUT: Duration = Duration::from_secs(5);

pub(super) fn run_service_dispatcher() -> ExitCode {
    let mut service_name: Vec<u16> = SERVICE_NAME.encode_utf16().collect();
    service_name.push(0);
    let entries = [
        SERVICE_TABLE_ENTRYW {
            lpServiceName: service_name.as_mut_ptr(),
            lpServiceProc: Some(service_main),
        },
        SERVICE_TABLE_ENTRYW::default(),
    ];
    // SAFETY: the table and terminated service name remain live until the dispatcher returns. The
    // final zero entry terminates the table and service_main uses the required system ABI.
    if unsafe { StartServiceCtrlDispatcherW(entries.as_ptr()) } != 0 {
        return ExitCode::SUCCESS;
    }
    let error = io::Error::last_os_error();
    if error.raw_os_error() == Some(ERROR_FAILED_SERVICE_CONTROLLER_CONNECT as i32) {
        eprintln!("journal_broker_not_started_by_scm");
    } else {
        eprintln!("journal_broker_scm_dispatch_failed:{error}");
    }
    ExitCode::FAILURE
}

unsafe extern "system" fn service_main(_argument_count: u32, _arguments: *mut *mut u16) {
    STOP_REQUESTED.store(false, Ordering::Release);
    let mut service_name: Vec<u16> = SERVICE_NAME.encode_utf16().collect();
    service_name.push(0);
    // SAFETY: the terminated name remains live for the call, handler has the system ABI, and the
    // null context is never dereferenced.
    let status_handle = unsafe {
        RegisterServiceCtrlHandlerExW(service_name.as_ptr(), Some(service_control_handler), null())
    };
    if status_handle.is_null() {
        return;
    }
    STATUS_HANDLE.store(status_handle, Ordering::Release);
    report_status(SERVICE_START_PENDING, 0, NO_ERROR, 0, 1, 5_000);
    let (ready_tx, ready_rx) = sync_channel(1);
    let worker = std::thread::spawn(move || {
        super::host::run_production_service_with_ready(&STOP_REQUESTED, ready_tx)
    });
    if startup_state(ready_rx.recv_timeout(START_READY_TIMEOUT)) != SERVICE_RUNNING {
        STOP_REQUESTED.store(true, Ordering::Release);
        let _ = worker.join();
        report_status(SERVICE_STOPPED, 0, ERROR_SERVICE_SPECIFIC_ERROR, 1, 0, 0);
        return;
    }
    report_status(SERVICE_RUNNING, SERVICE_ACCEPT_STOP, NO_ERROR, 0, 0, 0);
    let mut checkpoint = 1_u32;
    let mut last_report = Instant::now();
    while !worker.is_finished() {
        if STOP_REQUESTED.load(Ordering::Acquire) && last_report.elapsed() >= STOP_STATUS_INTERVAL {
            checkpoint = next_checkpoint(checkpoint);
            report_status(
                SERVICE_STOP_PENDING,
                0,
                NO_ERROR,
                0,
                checkpoint,
                STOP_WAIT_HINT_MS,
            );
            last_report = Instant::now();
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    let result = worker.join().unwrap_or(Err(()));
    match result {
        Ok(()) => report_status(SERVICE_STOPPED, 0, NO_ERROR, 0, 0, 0),
        Err(()) => report_status(SERVICE_STOPPED, 0, ERROR_SERVICE_SPECIFIC_ERROR, 1, 0, 0),
    }
}

unsafe extern "system" fn service_control_handler(
    control: u32,
    _event_type: u32,
    _event_data: *mut c_void,
    _context: *mut c_void,
) -> u32 {
    match control {
        SERVICE_CONTROL_STOP => {
            STOP_REQUESTED.store(true, Ordering::Release);
            report_status(SERVICE_STOP_PENDING, 0, NO_ERROR, 0, 1, STOP_WAIT_HINT_MS);
        }
        SERVICE_CONTROL_INTERROGATE => {
            let state = if STOP_REQUESTED.load(Ordering::Acquire) {
                SERVICE_STOP_PENDING
            } else {
                SERVICE_RUNNING
            };
            let accepted = if state == SERVICE_RUNNING {
                SERVICE_ACCEPT_STOP
            } else {
                0
            };
            report_status(state, accepted, NO_ERROR, 0, 0, 0);
        }
        _ => {}
    }
    NO_ERROR
}

fn report_status(
    state: u32,
    accepted: u32,
    win32_exit_code: u32,
    service_exit_code: u32,
    checkpoint: u32,
    wait_hint: u32,
) {
    let status = SERVICE_STATUS {
        dwServiceType: SERVICE_WIN32_OWN_PROCESS,
        dwCurrentState: state,
        dwControlsAccepted: accepted,
        dwWin32ExitCode: win32_exit_code,
        dwServiceSpecificExitCode: service_exit_code,
        dwCheckPoint: checkpoint,
        dwWaitHint: wait_hint,
    };
    let handle = STATUS_HANDLE.load(Ordering::Acquire);
    if !handle.is_null() {
        // SAFETY: handle is owned by SCM and status is live for the synchronous call.
        let _ = unsafe { SetServiceStatus(handle, &status) };
    }
}

fn next_checkpoint(current: u32) -> u32 {
    current.saturating_add(1).max(1)
}

fn startup_state(result: Result<(), RecvTimeoutError>) -> u32 {
    if result.is_ok() {
        SERVICE_RUNNING
    } else {
        SERVICE_START_PENDING
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn service_identity_is_stable_and_not_a_display_name() {
        assert_eq!(SERVICE_NAME, "CedarflakeAmeJournalBroker");
        assert!(!SERVICE_NAME.contains(' '));
    }

    #[test]
    fn stop_pending_checkpoint_is_monotonic_and_saturating() {
        assert_eq!(next_checkpoint(1), 2);
        assert_eq!(next_checkpoint(41), 42);
        assert_eq!(next_checkpoint(u32::MAX), u32::MAX);
    }

    #[test]
    fn service_remains_start_pending_until_all_listener_handles_are_ready() {
        assert_eq!(
            startup_state(Err(RecvTimeoutError::Timeout)),
            SERVICE_START_PENDING
        );
        assert_eq!(
            startup_state(Err(RecvTimeoutError::Disconnected)),
            SERVICE_START_PENDING
        );
        assert_eq!(startup_state(Ok(())), SERVICE_RUNNING);
    }
}
