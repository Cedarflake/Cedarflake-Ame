use std::ffi::c_void;
use std::io;
use std::ptr::null_mut;

use windows_sys::Win32::Foundation::{HANDLE, LocalFree};
use windows_sys::Win32::Security::Authorization::{
    ConvertStringSidToSidW, EXPLICIT_ACCESS_W, GRANT_ACCESS, GetSecurityInfo, NO_MULTIPLE_TRUSTEE,
    SE_KERNEL_OBJECT, SetEntriesInAclW, SetSecurityInfo, TRUSTEE_IS_SID,
    TRUSTEE_IS_WELL_KNOWN_GROUP, TRUSTEE_W,
};
use windows_sys::Win32::Security::{ACL, DACL_SECURITY_INFORMATION, NO_INHERITANCE, TOKEN_QUERY};
use windows_sys::Win32::Storage::FileSystem::{READ_CONTROL, WRITE_DAC};
use windows_sys::Win32::System::Threading::{
    GetCurrentProcess, OpenProcessToken, PROCESS_QUERY_LIMITED_INFORMATION,
};

use super::service_profile::verify_running_service_profile;
use super::transport::{OwnedHandle, WindowsTransportError};

const INTERACTIVE_SID: &str = "S-1-5-4";

#[derive(Clone, Copy)]
enum IdentityObject {
    Process,
    Token,
}

impl IdentityObject {
    fn query_access(self) -> u32 {
        match self {
            Self::Process => PROCESS_QUERY_LIMITED_INFORMATION,
            Self::Token => TOKEN_QUERY,
        }
    }
}

pub(super) fn prepare_current_service() -> Result<(), WindowsTransportError> {
    verify_running_service_profile()?;
    // SAFETY: the pseudo-handle borrows this process and is never closed or transferred.
    let process = unsafe { GetCurrentProcess() };
    let mut raw_token = null_mut();
    // SAFETY: process is live and raw_token is a live output slot. These rights only permit
    // querying and extending this verified service token's object DACL, not its privileges.
    if unsafe {
        OpenProcessToken(
            process,
            TOKEN_QUERY | READ_CONTROL | WRITE_DAC,
            &mut raw_token,
        )
    } == 0
    {
        return Err(WindowsTransportError::Io(io::Error::last_os_error()));
    }
    let token = OwnedHandle::new(raw_token)?;
    grant_identity_queries(process, IdentityObject::Process)?;
    grant_identity_queries(token.raw(), IdentityObject::Token)
}

fn grant_identity_queries(
    object: HANDLE,
    identity: IdentityObject,
) -> Result<(), WindowsTransportError> {
    let mut original = LocalAllocation::empty();
    let mut original_dacl = null_mut();
    // SAFETY: the caller owns object; both output slots remain live. The returned descriptor
    // retains original_dacl until the replacement ACL has been applied.
    check_status(unsafe {
        GetSecurityInfo(
            object,
            SE_KERNEL_OBJECT,
            DACL_SECURITY_INFORMATION,
            null_mut(),
            null_mut(),
            &mut original_dacl,
            null_mut(),
            &mut original.0,
        )
    })?;
    let replacement = identity_query_acl(original_dacl, identity)?;
    // SAFETY: object is retained and replacement owns the complete ACL for this call. Owner,
    // group, SACL and token privileges are unchanged; existing DACL entries are preserved.
    check_status(unsafe {
        SetSecurityInfo(
            object,
            SE_KERNEL_OBJECT,
            DACL_SECURITY_INFORMATION,
            null_mut(),
            null_mut(),
            replacement.0.cast(),
            null_mut(),
        )
    })
}

fn identity_query_acl(
    original: *const ACL,
    identity: IdentityObject,
) -> Result<LocalAllocation, WindowsTransportError> {
    if original.is_null() {
        return Err(WindowsTransportError::ServerIdentityRejected);
    }
    let mut sid = LocalAllocation::empty();
    let sid_text: Vec<u16> = INTERACTIVE_SID.encode_utf16().chain(Some(0)).collect();
    // SAFETY: sid_text is a fixed terminated SID and sid owns the returned LocalAlloc storage.
    if unsafe { ConvertStringSidToSidW(sid_text.as_ptr(), &mut sid.0) } == 0 {
        return Err(WindowsTransportError::Io(io::Error::last_os_error()));
    }
    let entry = EXPLICIT_ACCESS_W {
        grfAccessPermissions: identity.query_access(),
        grfAccessMode: GRANT_ACCESS,
        grfInheritance: NO_INHERITANCE,
        Trustee: TRUSTEE_W {
            pMultipleTrustee: null_mut(),
            MultipleTrusteeOperation: NO_MULTIPLE_TRUSTEE,
            TrusteeForm: TRUSTEE_IS_SID,
            TrusteeType: TRUSTEE_IS_WELL_KNOWN_GROUP,
            ptstrName: sid.0.cast(),
        },
    };
    let mut replacement = LocalAllocation::empty();
    // SAFETY: entry, its SID and the original Windows ACL remain live during the merge. The
    // returned ACL is a separate LocalAlloc allocation with its own lifetime.
    check_status(unsafe {
        SetEntriesInAclW(1, &entry, original, (&raw mut replacement.0).cast())
    })?;
    Ok(replacement)
}

fn check_status(status: u32) -> Result<(), WindowsTransportError> {
    if status == 0 {
        Ok(())
    } else {
        Err(WindowsTransportError::Io(io::Error::from_raw_os_error(
            status as i32,
        )))
    }
}

struct LocalAllocation(*mut c_void);

impl LocalAllocation {
    fn empty() -> Self {
        Self(null_mut())
    }
}

impl Drop for LocalAllocation {
    fn drop(&mut self) {
        if !self.0.is_null() {
            // SAFETY: this is the unique allocation returned by a Windows security API.
            let _ = unsafe { LocalFree(self.0) };
        }
    }
}

#[cfg(test)]
mod tests;
