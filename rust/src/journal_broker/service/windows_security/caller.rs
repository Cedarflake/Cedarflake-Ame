use std::mem::size_of;
use std::ptr::null_mut;

use windows_sys::Win32::Foundation::{
    CloseHandle, DuplicateHandle, FILETIME, HANDLE, INVALID_HANDLE_VALUE, LUID, WAIT_TIMEOUT,
};
use windows_sys::Win32::Security::{
    TOKEN_ELEVATION, TOKEN_QUERY, TOKEN_STATISTICS, TOKEN_TYPE, TokenElevation, TokenPrimary,
    TokenSessionId, TokenStatistics, TokenType,
};
use windows_sys::Win32::Storage::FileSystem::FILE_READ_ATTRIBUTES;
use windows_sys::Win32::System::Threading::{
    GetCurrentProcess, GetProcessTimes, OpenProcess, OpenProcessToken, PROCESS_DUP_HANDLE,
    PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SYNCHRONIZE, WaitForSingleObject,
};

use super::{
    OwnedFileHandle, OwnedToken, TokenEvidence, WindowsSecurityError, luid_bytes,
    query_token_information, query_token_sid_bytes, read_token_scalar,
};

pub(super) const ROOT_DUPLICATED_ACCESS: u32 = FILE_READ_ATTRIBUTES;

pub(super) struct PipeCallerProcess {
    process: OwnedProcessHandle,
    process_id: u32,
    creation_time: u64,
    primary: PrimaryTokenEvidence,
}

impl PipeCallerProcess {
    pub(super) fn open(process_id: u32) -> Result<Self, WindowsSecurityError> {
        if process_id == 0 {
            return Err(WindowsSecurityError::CallerRejected);
        }
        // SAFETY: the caller supplies the OS-observed pipe PID while impersonating that caller.
        // The non-inheritable handle pins this process for all subsequent token and handle use.
        let process = OwnedProcessHandle::new(unsafe {
            OpenProcess(
                PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_DUP_HANDLE | PROCESS_SYNCHRONIZE,
                0,
                process_id,
            )
        })?;
        let mut creation = FILETIME::default();
        let mut exit = FILETIME::default();
        let mut kernel = FILETIME::default();
        let mut user = FILETIME::default();
        // SAFETY: the query handle and four exactly sized output buffers remain live.
        if unsafe {
            GetProcessTimes(
                process.raw(),
                &mut creation,
                &mut exit,
                &mut kernel,
                &mut user,
            )
        } == 0
        {
            return Err(WindowsSecurityError::CallerRejected);
        }
        let creation_time =
            u64::from(creation.dwLowDateTime) | (u64::from(creation.dwHighDateTime) << 32);
        let primary = query_primary_token_evidence(&process)?;
        Ok(Self {
            process,
            process_id,
            creation_time,
            primary,
        })
    }

    pub(super) fn primary(&self) -> &PrimaryTokenEvidence {
        &self.primary
    }

    pub(super) fn opaque_identity(&self) -> [u8; 32] {
        process_identity(self.process_id, self.creation_time, &self.primary)
    }

    pub(super) fn ensure_alive(&self) -> Result<(), WindowsSecurityError> {
        // SAFETY: the owned process handle has SYNCHRONIZE access. A zero-duration wait checks
        // this exact process object without reopening or trusting its numeric PID.
        if unsafe { WaitForSingleObject(self.process.raw(), 0) } != WAIT_TIMEOUT {
            return Err(WindowsSecurityError::CallerRejected);
        }
        Ok(())
    }

    pub(super) fn duplicate_root_handle(
        &self,
        client_handle: u64,
    ) -> Result<OwnedFileHandle, WindowsSecurityError> {
        let source = client_handle as usize as HANDLE;
        if source.is_null() || source == INVALID_HANDLE_VALUE {
            return Err(WindowsSecurityError::RootRejected);
        }
        self.ensure_alive()?;
        let mut duplicate = null_mut();
        // SAFETY: the source process is pinned, source is only an opaque handle in that process,
        // and the non-inheritable duplicate grants attributes only, never media-content access.
        if unsafe {
            DuplicateHandle(
                self.process.raw(),
                source,
                GetCurrentProcess(),
                &mut duplicate,
                ROOT_DUPLICATED_ACCESS,
                0,
                0,
            )
        } == 0
        {
            return Err(WindowsSecurityError::RootRejected);
        }
        OwnedFileHandle::new(duplicate)
    }
}

pub(super) struct PrimaryTokenEvidence {
    user_sid: Vec<u8>,
    pub(super) session_id: u32,
    authentication_id: LUID,
    pub(super) is_elevated: bool,
}

impl PrimaryTokenEvidence {
    pub(super) fn matches_impersonation(&self, evidence: &TokenEvidence) -> bool {
        self.user_sid == evidence.user_sid
            && self.session_id == evidence.session_id
            && self.authentication_id.LowPart == evidence.authentication_id.LowPart
            && self.authentication_id.HighPart == evidence.authentication_id.HighPart
            && self.is_elevated == evidence.is_elevated
    }
}

fn query_primary_token_evidence(
    process: &OwnedProcessHandle,
) -> Result<PrimaryTokenEvidence, WindowsSecurityError> {
    let mut raw_token = null_mut();
    // SAFETY: process is the retained query handle and raw_token is a live output slot.
    if unsafe { OpenProcessToken(process.raw(), TOKEN_QUERY, &mut raw_token) } == 0 {
        return Err(WindowsSecurityError::TokenUnavailable);
    }
    let token = OwnedToken::new(raw_token)?;
    let token_type = read_token_scalar::<TOKEN_TYPE>(&token, TokenType)
        .map_err(|_| WindowsSecurityError::TokenEvidenceRejected("primary token type"))?;
    if token_type != TokenPrimary {
        return Err(WindowsSecurityError::TokenEvidenceRejected(
            "primary token type value",
        ));
    }
    let session_id = read_token_scalar::<u32>(&token, TokenSessionId)
        .map_err(|_| WindowsSecurityError::TokenEvidenceRejected("primary session"))?;
    let statistics = query_token_information(&token, TokenStatistics)
        .map_err(|_| WindowsSecurityError::TokenEvidenceRejected("primary statistics"))?;
    if statistics.len() < size_of::<TOKEN_STATISTICS>() {
        return Err(WindowsSecurityError::TokenEvidenceRejected(
            "primary statistics size",
        ));
    }
    // SAFETY: size is checked and an unaligned read does not assume Vec<u8> alignment.
    let statistics =
        unsafe { std::ptr::read_unaligned(statistics.as_ptr().cast::<TOKEN_STATISTICS>()) };
    let user_sid = query_token_sid_bytes(&token, "primary user")?;
    let elevation = read_token_scalar::<TOKEN_ELEVATION>(&token, TokenElevation)
        .map_err(|_| WindowsSecurityError::TokenEvidenceRejected("primary elevation"))?;
    Ok(PrimaryTokenEvidence {
        user_sid,
        session_id,
        authentication_id: statistics.AuthenticationId,
        is_elevated: elevation.TokenIsElevated != 0,
    })
}

fn process_identity(
    process_id: u32,
    creation_time: u64,
    primary: &PrimaryTokenEvidence,
) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"cedarflake-ame-pipe-caller-process-v1\0");
    hasher.update(&process_id.to_le_bytes());
    hasher.update(&creation_time.to_le_bytes());
    hasher.update(&primary.user_sid);
    hasher.update(&primary.session_id.to_le_bytes());
    hasher.update(&luid_bytes(primary.authentication_id));
    hasher.update(&[u8::from(primary.is_elevated)]);
    *hasher.finalize().as_bytes()
}

struct OwnedProcessHandle(HANDLE);

impl OwnedProcessHandle {
    fn new(raw: HANDLE) -> Result<Self, WindowsSecurityError> {
        if raw.is_null() || raw == INVALID_HANDLE_VALUE {
            Err(WindowsSecurityError::TokenUnavailable)
        } else {
            Ok(Self(raw))
        }
    }

    fn raw(&self) -> HANDLE {
        self.0
    }
}

// SAFETY: this unique kernel process handle may move between threads. It exposes only bounded
// metadata queries and one-way, reduced-access handle duplication into this process.
unsafe impl Send for OwnedProcessHandle {}
// SAFETY: the Win32 operations use independent caller-owned buffers; the handle is immutable and
// remains live until all connection-security owners have dropped it.
unsafe impl Sync for OwnedProcessHandle {}

impl Drop for OwnedProcessHandle {
    fn drop(&mut self) {
        // SAFETY: this unique owner was constructed from a valid process handle and closes once.
        let _ = unsafe { CloseHandle(self.0) };
    }
}

#[cfg(test)]
mod tests;
