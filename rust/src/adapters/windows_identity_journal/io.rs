use std::fs::File;
use std::io;
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
use std::ptr::null_mut;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use windows_sys::Win32::Foundation::{DUPLICATE_SAME_ACCESS, DuplicateHandle, HANDLE};
use windows_sys::Win32::System::IO::{CancelSynchronousIo, DeviceIoControl};
use windows_sys::Win32::System::Threading::{GetCurrentProcess, GetCurrentThread};

use crate::domain::ScanError;

pub(super) fn control(
    file: &File,
    code: u32,
    input: &[u8],
    output: &mut [u8],
    cancelled: &AtomicBool,
    deadline: Instant,
) -> Result<usize, ScanError> {
    check_active(cancelled, deadline)?;
    let input_len = u32::try_from(input.len()).map_err(|_| invalid())?;
    let output_len = u32::try_from(output.len()).map_err(|_| invalid())?;
    let mut thread_handle = null_mut();
    // SAFETY: both pseudo handles name this process/thread; the duplicate is uniquely owned.
    if unsafe {
        DuplicateHandle(
            GetCurrentProcess(),
            GetCurrentThread(),
            GetCurrentProcess(),
            &mut thread_handle,
            0,
            0,
            DUPLICATE_SAME_ACCESS,
        )
    } == 0
    {
        return Err(native_error(
            "identity_journal_thread_handle",
            io::Error::last_os_error(),
        ));
    }
    // SAFETY: DuplicateHandle succeeded and transferred one valid thread-handle owner.
    let thread_handle = unsafe { OwnedHandle::from_raw_handle(thread_handle) };
    let finished = AtomicBool::new(false);
    let (succeeded, returned, error) = std::thread::scope(|scope| -> Result<_, ScanError> {
        let monitor = std::thread::Builder::new()
            .name("ame-journal-io-cancel".to_owned())
            .spawn_scoped(scope, || {
                while !finished.load(Ordering::Acquire) {
                    if cancelled.load(Ordering::Acquire) || Instant::now() >= deadline {
                        // SAFETY: only this issuing thread is targeted. It cannot start other I/O
                        // until this scoped monitor retires. The issuing call owns all buffers.
                        let _ =
                            unsafe { CancelSynchronousIo(thread_handle.as_raw_handle() as HANDLE) };
                    }
                    std::thread::sleep(Duration::from_millis(10));
                }
            })
            .map_err(|error| native_error("identity_journal_cancel_monitor", error))?;
        let mut returned = 0_u32;
        // SAFETY: only existing-journal QUERY/READ codes are admitted by the callers. This is
        // synchronous I/O: input, output and file stay live until terminal completion. A cancel
        // request never releases kernel-owned storage or terminates the desktop process.
        let succeeded = unsafe {
            DeviceIoControl(
                file.as_raw_handle() as HANDLE,
                code,
                input.as_ptr().cast(),
                input_len,
                output.as_mut_ptr().cast(),
                output_len,
                &mut returned,
                null_mut(),
            )
        };
        let error = (succeeded == 0).then(io::Error::last_os_error);
        finished.store(true, Ordering::Release);
        let _ = monitor.join();
        Ok((succeeded, returned, error))
    })?;
    check_active(cancelled, deadline)?;
    if succeeded == 0 {
        let error = error.unwrap_or_else(io::Error::last_os_error);
        if matches!(error.raw_os_error(), Some(1 | 5 | 50 | 1179)) {
            return Err(native_error("identity_journal_not_supported", error));
        }
        return Err(native_error("identity_journal_native_read", error));
    }
    let returned = returned as usize;
    if returned > output.len() {
        return Err(invalid());
    }
    Ok(returned)
}

pub(super) fn check_active(cancelled: &AtomicBool, deadline: Instant) -> Result<(), ScanError> {
    if cancelled.load(Ordering::Acquire) {
        return Err(ScanError::new(
            "identity_journal_cancelled",
            "The identity journal read was cancelled",
        ));
    }
    if Instant::now() >= deadline {
        return Err(ScanError::new(
            "identity_journal_timeout",
            "The bounded identity journal read expired",
        ));
    }
    Ok(())
}

pub(super) fn native_error(code: &str, error: io::Error) -> ScanError {
    ScanError::new(
        code,
        format!(
            "Windows metadata operation failed: {}",
            error.raw_os_error().unwrap_or(0)
        ),
    )
}

fn invalid() -> ScanError {
    ScanError::new(
        "identity_journal_buffer_invalid",
        "The identity journal buffer is invalid",
    )
}
