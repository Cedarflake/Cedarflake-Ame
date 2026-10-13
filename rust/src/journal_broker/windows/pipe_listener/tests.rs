use std::ptr::{null, null_mut};
use std::sync::atomic::{AtomicU64, Ordering};

use windows_sys::Win32::Foundation::{ERROR_ACCESS_DENIED, LocalFree};
use windows_sys::Win32::Security::Authorization::ConvertStringSidToSidW;
use windows_sys::Win32::Security::{
    CreateRestrictedToken, ImpersonateLoggedOnUser, RevertToSelf, SID_AND_ATTRIBUTES,
    TOKEN_DUPLICATE, TOKEN_QUERY, WRITE_RESTRICTED,
};
use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

use super::*;
use crate::journal_broker::windows::service_profile::SERVICE_SID;

#[test]
fn restricted_service_can_create_both_production_listener_instances() {
    let name = test_pipe_name("restricted-service");
    let restricted = RestrictedCaller::enter(SERVICE_SID);
    let first = create_pipe(
        &name,
        &production_pipe_sddl(),
        true,
        true,
        MAX_PRODUCTION_CONNECTIONS,
    )
    .expect("first listener");
    let second = create_pipe(
        &name,
        &production_pipe_sddl(),
        true,
        false,
        MAX_PRODUCTION_CONNECTIONS,
    );
    assert!(
        second.is_ok(),
        "restricted second listener rejected: {:?}",
        second.as_ref().err()
    );
    drop(first);
    let replacement = create_pipe(
        &name,
        &production_pipe_sddl(),
        true,
        false,
        MAX_PRODUCTION_CONNECTIONS,
    );
    drop(restricted);
    assert!(
        replacement.is_ok(),
        "replacement listener rejected: {:?}",
        replacement.as_ref().err()
    );
    drop((second, replacement));
}

#[test]
fn restricted_service_rejects_listener_without_its_service_sid() {
    let name = test_pipe_name("missing-service-sid");
    let legacy_sddl = "D:P(A;;GA;;;SY)(A;;GA;;;BA)(A;;GRGW;;;IU)";
    let first = create_pipe(&name, legacy_sddl, true, true, MAX_PRODUCTION_CONNECTIONS)
        .expect("first listener");
    let restricted = RestrictedCaller::enter(SERVICE_SID);
    let second = create_pipe(&name, legacy_sddl, true, false, MAX_PRODUCTION_CONNECTIONS);
    drop(restricted);
    assert!(matches!(second, Err(HostError::Io(ref error))
        if error.raw_os_error() == Some(ERROR_ACCESS_DENIED as i32)));
    drop(first);
}

#[test]
fn another_restricted_service_cannot_create_a_production_listener() {
    let name = test_pipe_name("other-service");
    let first = create_pipe(
        &name,
        &production_pipe_sddl(),
        true,
        true,
        MAX_PRODUCTION_CONNECTIONS,
    )
    .expect("first listener");
    let restricted = RestrictedCaller::enter("S-1-5-80-1-2-3-4-5");
    let second = create_pipe(
        &name,
        &production_pipe_sddl(),
        true,
        false,
        MAX_PRODUCTION_CONNECTIONS,
    );
    drop(restricted);
    assert!(matches!(second, Err(HostError::Io(ref error))
        if error.raw_os_error() == Some(ERROR_ACCESS_DENIED as i32)));
    drop(first);
}

fn test_pipe_name(label: &str) -> String {
    static NEXT_PIPE: AtomicU64 = AtomicU64::new(1);
    format!(
        "{CONSOLE_PIPE_PREFIX}{}.{label}.{}",
        std::process::id(),
        NEXT_PIPE.fetch_add(1, Ordering::Relaxed),
    )
}

struct RestrictedCaller {
    _token: OwnedHandle,
}

impl RestrictedCaller {
    fn enter(service_sid: &str) -> Self {
        let mut current = null_mut();
        // SAFETY: the pseudo-handle is borrowed and current is a live output slot.
        assert_ne!(
            unsafe {
                OpenProcessToken(
                    GetCurrentProcess(),
                    TOKEN_QUERY | TOKEN_DUPLICATE,
                    &mut current,
                )
            },
            0
        );
        let current = OwnedHandle::new(current).expect("current token");
        let sids = [
            TestSid::new(service_sid),
            TestSid::new("S-1-1-0"),
            TestSid::new("S-1-5-33"),
        ];
        let groups = sids.each_ref().map(|sid| SID_AND_ATTRIBUTES {
            Sid: sid.0,
            Attributes: 0,
        });
        let mut restricted = null_mut();
        // SAFETY: the token and all three SID allocations remain live. This creates a less
        // privileged test token; it neither changes the process token nor requires elevation.
        assert_ne!(
            unsafe {
                CreateRestrictedToken(
                    current.raw(),
                    WRITE_RESTRICTED,
                    0,
                    null(),
                    0,
                    null(),
                    groups.len() as u32,
                    groups.as_ptr(),
                    &mut restricted,
                )
            },
            0
        );
        let token = OwnedHandle::new(restricted).expect("restricted token");
        // SAFETY: token belongs to this caller's authentication session; Drop always reverts
        // this test thread before any assertion on the operation under test.
        assert_ne!(unsafe { ImpersonateLoggedOnUser(token.raw()) }, 0);
        Self { _token: token }
    }
}

impl Drop for RestrictedCaller {
    fn drop(&mut self) {
        // SAFETY: this guard owns the current test thread's impersonation.
        assert_ne!(unsafe { RevertToSelf() }, 0);
    }
}

struct TestSid(*mut c_void);

impl TestSid {
    fn new(value: &str) -> Self {
        let wide: Vec<u16> = value.encode_utf16().chain([0]).collect();
        let mut sid = null_mut();
        // SAFETY: wide is terminated, sid is a live output slot, and TestSid owns the result.
        assert_ne!(
            unsafe { ConvertStringSidToSidW(wide.as_ptr(), &mut sid) },
            0
        );
        Self(sid)
    }
}

impl Drop for TestSid {
    fn drop(&mut self) {
        // SAFETY: this is the unique allocation returned by ConvertStringSidToSidW.
        let _ = unsafe { LocalFree(self.0) };
    }
}
