use std::ffi::c_void;
use std::mem::size_of;
use std::ptr::null_mut;

use windows_sys::Win32::Foundation::LocalFree;
use windows_sys::Win32::Security::Authorization::{
    ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1,
};
use windows_sys::Win32::Security::{PSECURITY_DESCRIPTOR, SECURITY_ATTRIBUTES};
use windows_sys::Win32::Storage::FileSystem::{
    FILE_FLAG_FIRST_PIPE_INSTANCE, FILE_FLAG_OVERLAPPED, PIPE_ACCESS_DUPLEX,
};
use windows_sys::Win32::System::Pipes::{
    CreateNamedPipeW, PIPE_READMODE_BYTE, PIPE_REJECT_REMOTE_CLIENTS, PIPE_TYPE_BYTE, PIPE_WAIT,
};

use super::host::HostError;
use super::service_profile::SERVICE_SID;
use super::transport::{OwnedHandle, PRODUCTION_PIPE_NAME};
use crate::journal_broker::MAX_FRAME_BYTES;

pub(super) const MAX_PRODUCTION_CONNECTIONS: u32 = 2;
pub(super) const CONSOLE_PIPE_PREFIX: &str = r"\\.\pipe\CedarflakeAme.JournalBroker.Test.";
pub(super) const DISPOSABLE_PIPE_SDDL: &str = "D:P(A;;GA;;;SY)(A;;GA;;;BA)(A;;GRGW;;;WD)";
pub(super) fn production_pipe_sddl() -> String {
    // Restricted tokens must pass both the normal and restricted SID access checks when opening
    // another server instance. SYSTEM alone does not authorize the service's restricted token.
    format!("D:P(A;;GA;;;SY)(A;;GA;;;BA)(A;;GA;;;{SERVICE_SID})(A;;GRGW;;;IU)")
}

pub(super) fn create_pipe(
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

#[cfg(test)]
mod tests;
