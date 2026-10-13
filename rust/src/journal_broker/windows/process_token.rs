use std::ffi::c_void;
use std::io;
use std::mem::{offset_of, size_of};
use std::ptr::{null, null_mut};

use windows_sys::Win32::Foundation::{HANDLE, HLOCAL, LUID, LocalFree};
use windows_sys::Win32::Security::Authorization::ConvertSidToStringSidW;
use windows_sys::Win32::Security::{
    GetTokenInformation, IsTokenRestricted, LUID_AND_ATTRIBUTES, LookupPrivilegeNameW,
    TOKEN_ELEVATION, TOKEN_GROUPS, TOKEN_PRIVILEGES, TOKEN_QUERY, TOKEN_TYPE, TOKEN_USER,
    TokenElevation, TokenPrivileges, TokenRestrictedSids, TokenType, TokenUser,
};
use windows_sys::Win32::System::Threading::OpenProcessToken;

use super::transport::{OwnedHandle, WindowsTransportError};

#[derive(Clone, Debug)]
pub(super) struct ProcessTokenIdentity {
    pub(super) user_sid: String,
    pub(super) token_type: TOKEN_TYPE,
    pub(super) is_elevated: bool,
    pub(super) is_restricted: bool,
    pub(super) restricted_sids: Vec<String>,
}

pub(super) struct ProcessToken {
    handle: OwnedHandle,
}

impl ProcessToken {
    pub(super) fn open(process: HANDLE) -> Result<Self, WindowsTransportError> {
        let mut raw = null_mut();
        // SAFETY: callers retain the process handle and raw is a live output slot.
        if unsafe { OpenProcessToken(process, TOKEN_QUERY, &mut raw) } == 0 {
            return Err(WindowsTransportError::Io(io::Error::last_os_error()));
        }
        Ok(Self {
            handle: OwnedHandle::new(raw)?,
        })
    }

    pub(super) fn identity(&self) -> Result<ProcessTokenIdentity, WindowsTransportError> {
        Ok(ProcessTokenIdentity {
            user_sid: token_sid(self)?,
            token_type: token_fixed::<TOKEN_TYPE>(self, TokenType)?,
            is_elevated: token_fixed::<TOKEN_ELEVATION>(self, TokenElevation)?.TokenIsElevated != 0,
            // SAFETY: this owner retains a live process token with TOKEN_QUERY access.
            is_restricted: unsafe { IsTokenRestricted(self.handle.raw()) } != 0,
            restricted_sids: token_group_sids(self)?,
        })
    }

    pub(super) fn privilege_names(&self) -> Result<Vec<String>, WindowsTransportError> {
        let bytes = token_information(self, TokenPrivileges)?;
        token_privilege_ids(&bytes)?
            .into_iter()
            .map(privilege_name)
            .collect()
    }
}

fn token_privilege_ids(bytes: &[u8]) -> Result<Vec<LUID>, WindowsTransportError> {
    let count_bytes: [u8; 4] = bytes
        .get(..4)
        .and_then(|value| value.try_into().ok())
        .ok_or(WindowsTransportError::ServerIdentityRejected)?;
    let count = u32::from_ne_bytes(count_bytes) as usize;
    let offset = offset_of!(TOKEN_PRIVILEGES, Privileges);
    let entry_size = size_of::<LUID_AND_ATTRIBUTES>();
    let required = count
        .checked_mul(entry_size)
        .and_then(|size| size.checked_add(offset))
        .ok_or(WindowsTransportError::ServerIdentityRejected)?;
    if count > 1024 || required != bytes.len() {
        return Err(WindowsTransportError::ServerIdentityRejected);
    }
    let mut privileges = Vec::with_capacity(count);
    for index in 0..count {
        // SAFETY: checked length and count cover every complete entry. Unaligned reads do not
        // require Vec<u8> to satisfy LUID_AND_ATTRIBUTES alignment.
        let entry = unsafe {
            bytes
                .as_ptr()
                .add(offset + index * entry_size)
                .cast::<LUID_AND_ATTRIBUTES>()
                .read_unaligned()
        };
        privileges.push(entry.Luid);
    }
    Ok(privileges)
}

fn privilege_name(luid: LUID) -> Result<String, WindowsTransportError> {
    let mut name = [0_u16; 128];
    let mut length = name.len() as u32;
    // SAFETY: null selects the local system; luid, name and length remain live for this query.
    if unsafe { LookupPrivilegeNameW(null(), &luid, name.as_mut_ptr(), &mut length) } == 0 {
        return Err(WindowsTransportError::Io(io::Error::last_os_error()));
    }
    if length == 0 || length as usize >= name.len() {
        return Err(WindowsTransportError::ServerIdentityRejected);
    }
    String::from_utf16(&name[..length as usize])
        .map_err(|_| WindowsTransportError::ServerIdentityRejected)
}

fn token_sid(token: &ProcessToken) -> Result<String, WindowsTransportError> {
    let bytes = token_information(token, TokenUser)?;
    let sid = token_user_sid_slice(&bytes)?;
    sid_string(sid.as_ptr().cast_mut().cast())
}

fn token_user_sid_slice(bytes: &[u8]) -> Result<&[u8], WindowsTransportError> {
    if bytes.len() < size_of::<TOKEN_USER>() {
        return Err(WindowsTransportError::ServerIdentityRejected);
    }
    // SAFETY: the fixed header fits; read_unaligned does not form a reference with Vec<u8>'s
    // alignment. No SID memory is accessed until its complete structural length is in `bytes`.
    let user = unsafe { bytes.as_ptr().cast::<TOKEN_USER>().read_unaligned() };
    sid_slice(bytes, user.User.Sid)
}

fn sid_slice(bytes: &[u8], raw_sid: *mut c_void) -> Result<&[u8], WindowsTransportError> {
    let sid = raw_sid.cast::<u8>();
    if sid.is_null() {
        return Err(WindowsTransportError::ServerIdentityRejected);
    }
    let start = bytes.as_ptr() as usize;
    let end = start
        .checked_add(bytes.len())
        .ok_or(WindowsTransportError::ServerIdentityRejected)?;
    let sid_start = sid as usize;
    if sid_start < start || sid_start >= end {
        return Err(WindowsTransportError::ServerIdentityRejected);
    }
    let sid_offset = sid_start - start;
    let sid_header_end = sid_offset
        .checked_add(8)
        .ok_or(WindowsTransportError::ServerIdentityRejected)?;
    if sid_header_end > bytes.len() {
        return Err(WindowsTransportError::ServerIdentityRejected);
    }
    let sub_authority_count = usize::from(bytes[sid_offset + 1]);
    let length = sub_authority_count
        .checked_mul(size_of::<u32>())
        .and_then(|value| value.checked_add(8))
        .ok_or(WindowsTransportError::ServerIdentityRejected)?;
    let sid_end = sid_offset
        .checked_add(length)
        .ok_or(WindowsTransportError::ServerIdentityRejected)?;
    if sid_end > bytes.len() {
        return Err(WindowsTransportError::ServerIdentityRejected);
    }
    let sid_slice = &bytes[sid_offset..sid_end];
    // SAFETY: the complete length implied by the SID header is inside bytes, so IsValidSid and
    // GetLengthSid cannot walk beyond the live allocation. The pointer remains live for both calls.
    if unsafe { windows_sys::Win32::Security::IsValidSid(sid.cast()) } == 0
        || unsafe { windows_sys::Win32::Security::GetLengthSid(sid.cast()) } as usize != length
    {
        return Err(WindowsTransportError::ServerIdentityRejected);
    }
    Ok(sid_slice)
}

fn token_fixed<T: Copy>(
    token: &ProcessToken,
    information_class: i32,
) -> Result<T, WindowsTransportError> {
    let bytes = token_information(token, information_class)?;
    if bytes.len() != size_of::<T>() {
        return Err(WindowsTransportError::ServerIdentityRejected);
    }
    // SAFETY: GetTokenInformation initialized exactly size_of::<T>() bytes for this fixed class;
    // read_unaligned avoids imposing Vec<u8> alignment.
    Ok(unsafe { bytes.as_ptr().cast::<T>().read_unaligned() })
}

fn token_group_sids(token: &ProcessToken) -> Result<Vec<String>, WindowsTransportError> {
    let bytes = token_information(token, TokenRestrictedSids)?;
    token_group_sids_from_bytes(&bytes)
}

fn token_group_sids_from_bytes(bytes: &[u8]) -> Result<Vec<String>, WindowsTransportError> {
    if bytes.len() < size_of::<u32>() {
        return Err(WindowsTransportError::ServerIdentityRejected);
    }
    // SAFETY: count is the first initialized u32 in TOKEN_GROUPS.
    let count = unsafe { bytes.as_ptr().cast::<u32>().read_unaligned() } as usize;
    let groups_offset = std::mem::offset_of!(TOKEN_GROUPS, Groups);
    let entry_size = size_of::<windows_sys::Win32::Security::SID_AND_ATTRIBUTES>();
    let groups_bytes = count
        .checked_mul(entry_size)
        .and_then(|value| value.checked_add(groups_offset))
        .ok_or(WindowsTransportError::ServerIdentityRejected)?;
    if groups_bytes > bytes.len() || count > 1024 {
        return Err(WindowsTransportError::ServerIdentityRejected);
    }
    let mut result = Vec::with_capacity(count);
    for index in 0..count {
        let offset = groups_offset + index * entry_size;
        // SAFETY: the checked total above proves this unaligned entry is within bytes.
        let group = unsafe {
            bytes
                .as_ptr()
                .add(offset)
                .cast::<windows_sys::Win32::Security::SID_AND_ATTRIBUTES>()
                .read_unaligned()
        };
        let sid = sid_slice(bytes, group.Sid)?;
        result.push(sid_string(sid.as_ptr().cast_mut().cast())?);
    }
    Ok(result)
}

fn token_information(
    token: &ProcessToken,
    information_class: i32,
) -> Result<Vec<u8>, WindowsTransportError> {
    let mut required = 0_u32;
    // SAFETY: this sizing query intentionally supplies no buffer.
    let _ = unsafe {
        GetTokenInformation(
            token.handle.raw(),
            information_class,
            null_mut(),
            0,
            &mut required,
        )
    };
    if required == 0 || required > 1024 * 1024 {
        return Err(WindowsTransportError::ServerIdentityRejected);
    }
    let mut bytes = vec![0_u8; required as usize];
    // SAFETY: bytes is exactly the bounded requested size and all output pointers are live.
    if unsafe {
        GetTokenInformation(
            token.handle.raw(),
            information_class,
            bytes.as_mut_ptr().cast(),
            required,
            &mut required,
        )
    } == 0
    {
        return Err(WindowsTransportError::Io(io::Error::last_os_error()));
    }
    if required as usize > bytes.len() {
        return Err(WindowsTransportError::ServerIdentityRejected);
    }
    bytes.truncate(required as usize);
    Ok(bytes)
}

fn sid_string(sid: *mut c_void) -> Result<String, WindowsTransportError> {
    let mut raw = null_mut();
    // SAFETY: sid originates from a validated token information buffer and raw is a live output.
    if unsafe { ConvertSidToStringSidW(sid, &mut raw) } == 0 || raw.is_null() {
        return Err(WindowsTransportError::ServerIdentityRejected);
    }
    let value = copy_bounded_utf16(raw);
    // SAFETY: raw is the unique LocalAlloc allocation returned above.
    let _ = unsafe { LocalFree(raw as HLOCAL) };
    value
}

fn copy_bounded_utf16(raw: *const u16) -> Result<String, WindowsTransportError> {
    let mut length = 0_usize;
    // SAFETY: callers pass a terminated Windows-owned string. The length cap prevents an
    // unbounded walk if an OS contract is violated.
    while unsafe { *raw.add(length) } != 0 {
        length = length
            .checked_add(1)
            .ok_or(WindowsTransportError::ServerIdentityRejected)?;
        if length > 32_768 {
            return Err(WindowsTransportError::ServerIdentityRejected);
        }
    }
    // SAFETY: the bounded walk found a terminator and the allocation remains live.
    String::from_utf16(unsafe { std::slice::from_raw_parts(raw, length) })
        .map_err(|_| WindowsTransportError::ServerIdentityRejected)
}

#[cfg(test)]
mod tests;
