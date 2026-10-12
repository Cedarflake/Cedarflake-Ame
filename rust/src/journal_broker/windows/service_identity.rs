mod installed;

use installed::{InstalledBrokerIdentity, ServerProcessFacts};

use std::fs;
use std::io::{self, Read};
use std::mem::size_of;
use std::os::windows::io::AsRawHandle;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, ExitStatus, Stdio};
use std::ptr::{null, null_mut};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use windows_sys::Win32::Foundation::{
    CloseHandle, ERROR_INSUFFICIENT_BUFFER, ERROR_SERVICE_ALREADY_RUNNING, HANDLE,
};
use windows_sys::Win32::Security::Cryptography::{
    BCRYPT_SHA256_ALGORITHM, BCryptCloseAlgorithmProvider, BCryptHash, BCryptOpenAlgorithmProvider,
};
use windows_sys::Win32::System::Com::CoTaskMemFree;
use windows_sys::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation,
    SetInformationJobObject, TerminateJobObject,
};
use windows_sys::Win32::System::Pipes::GetNamedPipeServerProcessId;
use windows_sys::Win32::System::Services::{
    CloseServiceHandle, OpenSCManagerW, OpenServiceW, QueryServiceConfig2W, QueryServiceStatusEx,
    SC_HANDLE, SC_MANAGER_CONNECT, SC_STATUS_PROCESS_INFO, SERVICE_CONFIG_DESCRIPTION,
    SERVICE_DESCRIPTIONW, SERVICE_QUERY_CONFIG, SERVICE_QUERY_STATUS, SERVICE_RUNNING,
    SERVICE_START, SERVICE_START_PENDING, SERVICE_STATUS_PROCESS, StartServiceW,
};
use windows_sys::Win32::System::Threading::{
    CREATE_NO_WINDOW, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW,
};
use windows_sys::Win32::UI::Shell::{
    FOLDERID_ProgramFilesX64, KF_FLAG_DEFAULT, SHGetKnownFolderPath,
};

use super::process_token::ProcessToken;
use super::service_host::SERVICE_NAME;
use super::transport::{OwnedHandle, WindowsTransportError};
use crate::journal_broker::BROKER_EXECUTABLE_NAME;

const SERVICE_START_TIMEOUT: Duration = Duration::from_secs(5);
const MAX_MANIFEST_BYTES: usize = 16 * 1024;
const MAX_BINARY_BYTES: u64 = 128 * 1024 * 1024;
const IDENTITY_PROBE_TIMEOUT: Duration = Duration::from_secs(3);
const MAX_IDENTITY_PROBE_OUTPUT_BYTES: u64 = 512;
const IDENTITY_PROBE_PROTOCOL: &str = "AMEJBPROBE1";
static NEXT_IDENTITY_PROBE_NONCE: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct StartedService {
    process_id: u32,
    manifest: InstalledBrokerIdentity,
}

pub(super) fn demand_start_service() -> Result<StartedService, WindowsTransportError> {
    // SAFETY: null machine/database select the local active SCM database and no pointer escapes.
    let manager = ServiceHandle::new(unsafe { OpenSCManagerW(null(), null(), SC_MANAGER_CONNECT) })
        .map_err(map_service_open_error)?;
    let service_name = nul_terminated(SERVICE_NAME)?;
    // SAFETY: the service name is terminated and live; manager is a live SCM handle. The requested
    // rights permit only status queries and demand start of this fixed service.
    let service = ServiceHandle::new(unsafe {
        OpenServiceW(
            manager.raw(),
            service_name.as_ptr(),
            SERVICE_QUERY_CONFIG | SERVICE_QUERY_STATUS | SERVICE_START,
        )
    })
    .map_err(map_service_open_error)?;
    let mut status = query_status(&service)?;
    if status.dwCurrentState != SERVICE_RUNNING {
        // SAFETY: service is the fixed service handle and no caller-controlled arguments are used.
        if unsafe { StartServiceW(service.raw(), 0, null()) } == 0 {
            let error = io::Error::last_os_error();
            if error.raw_os_error() != Some(ERROR_SERVICE_ALREADY_RUNNING as i32) {
                return Err(WindowsTransportError::Io(error));
            }
        }
        let deadline = Instant::now() + SERVICE_START_TIMEOUT;
        loop {
            status = query_status(&service)?;
            if status.dwCurrentState == SERVICE_RUNNING && status.dwProcessId != 0 {
                break;
            }
            if status.dwCurrentState != SERVICE_START_PENDING || Instant::now() >= deadline {
                return Err(WindowsTransportError::TimedOut);
            }
            thread::sleep(Duration::from_millis(25));
        }
    }
    if status.dwProcessId == 0 {
        return Err(WindowsTransportError::ServerIdentityRejected);
    }
    Ok(StartedService {
        process_id: status.dwProcessId,
        manifest: query_identity_manifest(&service)?,
    })
}

pub(super) fn authenticate_pipe_server(
    pipe: &OwnedHandle,
    started_service: &StartedService,
) -> Result<(), WindowsTransportError> {
    let mut pipe_process_id = 0_u32;
    // SAFETY: pipe is a live named-pipe client handle and the output slot is live.
    if unsafe { GetNamedPipeServerProcessId(pipe.raw(), &mut pipe_process_id) } == 0 {
        return Err(WindowsTransportError::Io(io::Error::last_os_error()));
    }
    run_identity_probe(&[
        "server".to_owned(),
        started_service.process_id.to_string(),
        pipe_process_id.to_string(),
        identity_digest_hex(&started_service.manifest),
    ])?;
    let expected_image_path = expected_broker_path()?;
    // SAFETY: the PID came from the fixed service's SCM status and no handle is inherited.
    let process = OwnedKernelHandle::new(unsafe {
        OpenProcess(
            PROCESS_QUERY_LIMITED_INFORMATION,
            0,
            started_service.process_id,
        )
    })?;
    let image_path = process_image_path(&process)?;
    let token_identity = ProcessToken::open(process.raw())?.identity()?;
    ServerProcessFacts {
        scm_process_id: started_service.process_id,
        pipe_process_id,
        image_path,
        expected_image_path,
        token_user_sid: token_identity.user_sid,
        token_type: token_identity.token_type,
        is_elevated: token_identity.is_elevated,
        is_restricted: token_identity.is_restricted,
        restricted_sids: token_identity.restricted_sids,
        manifest: started_service.manifest.clone(),
        actual_binary_sha256: started_service.manifest.binary_sha256,
    }
    .validate()
}

fn identity_digest_hex(identity: &InstalledBrokerIdentity) -> String {
    identity
        .binary_sha256
        .iter()
        .map(|byte| format!("{byte:02X}"))
        .collect()
}

pub(super) fn run_identity_probe_process(arguments: &[String]) -> ExitCode {
    let result = match arguments {
        [
            mode,
            scm_process_id,
            pipe_process_id,
            expected_digest,
            nonce,
        ] if mode == "server" => {
            parse_probe_process_id(scm_process_id).and_then(|scm_process_id| {
                let pipe_process_id = parse_probe_process_id(pipe_process_id)?;
                validate_probe_nonce(nonce)?;
                verify_server_identity(scm_process_id, pipe_process_id, expected_digest)?;
                Ok(nonce)
            })
        }
        _ => Err(WindowsTransportError::ServerIdentityRejected),
    };
    match result {
        Ok(nonce) => {
            println!("{IDENTITY_PROBE_PROTOCOL} nonce={nonce}");
            ExitCode::SUCCESS
        }
        Err(_) => ExitCode::FAILURE,
    }
}

fn parse_probe_process_id(value: &str) -> Result<u32, WindowsTransportError> {
    value
        .parse::<u32>()
        .ok()
        .filter(|process_id| *process_id != 0)
        .ok_or(WindowsTransportError::ServerIdentityRejected)
}

fn validate_probe_nonce(nonce: &str) -> Result<(), WindowsTransportError> {
    if nonce.len() != 64
        || !nonce
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'A'..=b'F').contains(&byte))
    {
        return Err(WindowsTransportError::ServerIdentityRejected);
    }
    Ok(())
}

fn verify_server_identity(
    scm_process_id: u32,
    pipe_process_id: u32,
    expected_digest: &str,
) -> Result<(), WindowsTransportError> {
    let manager = ServiceHandle::new(unsafe { OpenSCManagerW(null(), null(), SC_MANAGER_CONNECT) })
        .map_err(map_service_open_error)?;
    let service_name = nul_terminated(SERVICE_NAME)?;
    let service = ServiceHandle::new(unsafe {
        OpenServiceW(
            manager.raw(),
            service_name.as_ptr(),
            SERVICE_QUERY_CONFIG | SERVICE_QUERY_STATUS,
        )
    })
    .map_err(map_service_open_error)?;
    let status = query_status(&service)?;
    if status.dwCurrentState != SERVICE_RUNNING
        || status.dwProcessId != scm_process_id
        || pipe_process_id != scm_process_id
    {
        return Err(WindowsTransportError::ServerIdentityRejected);
    }
    let manifest = query_identity_manifest(&service)?;
    if identity_digest_hex(&manifest) != expected_digest {
        return Err(WindowsTransportError::ServerIdentityRejected);
    }
    let process = OwnedKernelHandle::new(unsafe {
        OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, scm_process_id)
    })?;
    let image_path = process_image_path(&process)?;
    let token_identity = ProcessToken::open(process.raw())?.identity()?;
    let expected_image_path = expected_broker_path()?;
    ServerProcessFacts {
        scm_process_id,
        pipe_process_id,
        image_path: image_path.clone(),
        expected_image_path,
        token_user_sid: token_identity.user_sid,
        token_type: token_identity.token_type,
        is_elevated: token_identity.is_elevated,
        is_restricted: token_identity.is_restricted,
        restricted_sids: token_identity.restricted_sids,
        actual_binary_sha256: binary_sha256(&image_path)?,
        manifest,
    }
    .validate()
}

fn run_identity_probe(arguments: &[String]) -> Result<(), WindowsTransportError> {
    let executable = expected_broker_path()?;
    let nonce = next_identity_probe_nonce();
    let mut command = Command::new(executable);
    command
        .arg("--identity-probe")
        .args(arguments)
        .arg(&nonce)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .creation_flags(CREATE_NO_WINDOW);
    let mut child = command.spawn().map_err(WindowsTransportError::Io)?;
    let job = match identity_probe_job(child.as_raw_handle().cast()) {
        Ok(job) => job,
        Err(error) => {
            let _ = child.kill();
            let _ = child.wait();
            return Err(error);
        }
    };
    let stdout = child
        .stdout
        .take()
        .ok_or(WindowsTransportError::ServerIdentityRejected)?;
    let output_reader = thread::spawn(move || {
        let mut output = Vec::new();
        stdout
            .take(MAX_IDENTITY_PROBE_OUTPUT_BYTES + 1)
            .read_to_end(&mut output)
            .map(|_| output)
    });
    let status = wait_for_identity_probe(
        Instant::now() + IDENTITY_PROBE_TIMEOUT,
        || child.try_wait(),
        || {
            if unsafe { TerminateJobObject(job.raw(), 1) } == 0 {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        },
    );
    if status.is_err() {
        let _ = unsafe { TerminateJobObject(job.raw(), 1) };
        let _ = child.wait();
    }
    let output = output_reader
        .join()
        .map_err(|_| WindowsTransportError::ServerIdentityRejected)?
        .map_err(WindowsTransportError::Io)?;
    let status = status?;
    if !status.success()
        || output.len() > MAX_IDENTITY_PROBE_OUTPUT_BYTES as usize
        || output != format!("{IDENTITY_PROBE_PROTOCOL} nonce={nonce}\n").as_bytes()
    {
        return Err(WindowsTransportError::ServerIdentityRejected);
    }
    Ok(())
}

fn wait_for_identity_probe(
    deadline: Instant,
    mut try_wait: impl FnMut() -> io::Result<Option<ExitStatus>>,
    mut terminate_and_wait: impl FnMut() -> io::Result<()>,
) -> Result<ExitStatus, WindowsTransportError> {
    loop {
        if let Some(status) = try_wait().map_err(WindowsTransportError::Io)? {
            return Ok(status);
        }
        if Instant::now() >= deadline {
            terminate_and_wait().map_err(WindowsTransportError::Io)?;
            return Err(WindowsTransportError::TimedOut);
        }
        thread::sleep(Duration::from_millis(10));
    }
}

fn identity_probe_job(process: HANDLE) -> Result<OwnedKernelHandle, WindowsTransportError> {
    let job = OwnedKernelHandle::new(unsafe { CreateJobObjectW(null(), null()) })?;
    let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
    limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
    if unsafe {
        SetInformationJobObject(
            job.raw(),
            JobObjectExtendedLimitInformation,
            (&raw const limits).cast(),
            u32::try_from(size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>())
                .map_err(|_| WindowsTransportError::ServerIdentityRejected)?,
        )
    } == 0
        || unsafe { AssignProcessToJobObject(job.raw(), process) } == 0
    {
        return Err(WindowsTransportError::Io(io::Error::last_os_error()));
    }
    Ok(job)
}

fn next_identity_probe_nonce() -> String {
    let sequence = NEXT_IDENTITY_PROBE_NONCE.fetch_add(1, Ordering::AcqRel);
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"cedarflake-ame-identity-probe-v1\0");
    hasher.update(&std::process::id().to_le_bytes());
    hasher.update(&sequence.to_le_bytes());
    hasher.update(
        &SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
            .to_le_bytes(),
    );
    hasher
        .finalize()
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02X}"))
        .collect()
}

fn query_status(service: &ServiceHandle) -> Result<SERVICE_STATUS_PROCESS, WindowsTransportError> {
    let mut status = SERVICE_STATUS_PROCESS::default();
    let mut required = 0_u32;
    // SAFETY: status is live and exactly sized for SC_STATUS_PROCESS_INFO.
    if unsafe {
        QueryServiceStatusEx(
            service.raw(),
            SC_STATUS_PROCESS_INFO,
            (&raw mut status).cast(),
            u32::try_from(size_of::<SERVICE_STATUS_PROCESS>())
                .map_err(|_| WindowsTransportError::ServerIdentityRejected)?,
            &mut required,
        )
    } == 0
    {
        return Err(WindowsTransportError::Io(io::Error::last_os_error()));
    }
    Ok(status)
}

fn query_identity_manifest(
    service: &ServiceHandle,
) -> Result<InstalledBrokerIdentity, WindowsTransportError> {
    let mut required = 0_u32;
    // SAFETY: this is the documented bounded size query for the fixed service description.
    let first = unsafe {
        QueryServiceConfig2W(
            service.raw(),
            SERVICE_CONFIG_DESCRIPTION,
            null_mut(),
            0,
            &mut required,
        )
    };
    let first_error = io::Error::last_os_error();
    if first != 0
        || first_error.raw_os_error() != Some(ERROR_INSUFFICIENT_BUFFER as i32)
        || required as usize > MAX_MANIFEST_BYTES
        || (required as usize) < size_of::<SERVICE_DESCRIPTIONW>()
    {
        return Err(WindowsTransportError::ServerIdentityRejected);
    }
    let mut bytes = vec![0_u8; required as usize];
    // SAFETY: bytes is exactly the bounded size returned above and all pointers remain live.
    if unsafe {
        QueryServiceConfig2W(
            service.raw(),
            SERVICE_CONFIG_DESCRIPTION,
            bytes.as_mut_ptr(),
            required,
            &mut required,
        )
    } == 0
    {
        return Err(WindowsTransportError::ServerIdentityRejected);
    }
    // SAFETY: QueryServiceConfig2W initialized a SERVICE_DESCRIPTIONW header in bytes.
    let description = unsafe {
        bytes
            .as_ptr()
            .cast::<SERVICE_DESCRIPTIONW>()
            .read_unaligned()
    };
    if description.lpDescription.is_null() {
        return Err(WindowsTransportError::ServerIdentityRejected);
    }
    InstalledBrokerIdentity::parse(&copy_bounded_utf16(description.lpDescription)?)
}

fn binary_sha256(path: &Path) -> Result<[u8; 32], WindowsTransportError> {
    let metadata = fs::metadata(path).map_err(WindowsTransportError::Io)?;
    if !metadata.is_file() || metadata.len() == 0 || metadata.len() > MAX_BINARY_BYTES {
        return Err(WindowsTransportError::ServerIdentityRejected);
    }
    let bytes = fs::read(path).map_err(WindowsTransportError::Io)?;
    if bytes.len() as u64 != metadata.len() {
        return Err(WindowsTransportError::ServerIdentityRejected);
    }
    let mut algorithm = null_mut();
    // SAFETY: algorithm is a live output slot and the algorithm identifier is a fixed terminated
    // Windows constant. No provider name or flags are requested.
    if unsafe { BCryptOpenAlgorithmProvider(&mut algorithm, BCRYPT_SHA256_ALGORITHM, null(), 0) }
        < 0
    {
        return Err(WindowsTransportError::ServerIdentityRejected);
    }
    let mut digest = [0_u8; 32];
    // SAFETY: algorithm is live, all input/output buffers are bounded and initialized, and SHA-256
    // produces exactly the supplied 32-byte output.
    let status = unsafe {
        BCryptHash(
            algorithm,
            null(),
            0,
            bytes.as_ptr(),
            u32::try_from(bytes.len())
                .map_err(|_| WindowsTransportError::ServerIdentityRejected)?,
            digest.as_mut_ptr(),
            u32::try_from(digest.len())
                .map_err(|_| WindowsTransportError::ServerIdentityRejected)?,
        )
    };
    // SAFETY: algorithm is the unique provider handle opened above.
    let closed = unsafe { BCryptCloseAlgorithmProvider(algorithm, 0) };
    if status < 0 || closed < 0 {
        return Err(WindowsTransportError::ServerIdentityRejected);
    }
    Ok(digest)
}

fn expected_broker_path() -> Result<PathBuf, WindowsTransportError> {
    let mut raw = null_mut();
    // SAFETY: the known-folder identifier is fixed and raw is a live output slot. The returned
    // CoTaskMem allocation is copied and freed exactly once below.
    let result = unsafe {
        SHGetKnownFolderPath(
            &FOLDERID_ProgramFilesX64,
            KF_FLAG_DEFAULT as u32,
            null_mut(),
            &mut raw,
        )
    };
    if result < 0 || raw.is_null() {
        return Err(WindowsTransportError::ServerIdentityRejected);
    }
    let path = copy_bounded_utf16(raw).map(PathBuf::from);
    // SAFETY: raw is the unique CoTaskMem allocation returned above and is no longer used.
    unsafe { CoTaskMemFree(raw.cast()) };
    Ok(path?
        .join("Cedarflake Ame")
        .join("Journal Broker")
        .join(BROKER_EXECUTABLE_NAME))
}

fn process_image_path(process: &OwnedKernelHandle) -> Result<PathBuf, WindowsTransportError> {
    let mut buffer = vec![0_u16; 32_768];
    let mut length =
        u32::try_from(buffer.len()).map_err(|_| WindowsTransportError::ServerIdentityRejected)?;
    // SAFETY: process has query rights and the output buffer and length are live and bounded.
    if unsafe { QueryFullProcessImageNameW(process.raw(), 0, buffer.as_mut_ptr(), &mut length) }
        == 0
    {
        return Err(WindowsTransportError::Io(io::Error::last_os_error()));
    }
    buffer.truncate(length as usize);
    Ok(PathBuf::from(String::from_utf16(&buffer).map_err(
        |_| WindowsTransportError::ServerIdentityRejected,
    )?))
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

fn nul_terminated(value: &str) -> Result<Vec<u16>, WindowsTransportError> {
    if value.is_empty() || value.encode_utf16().any(|unit| unit == 0) {
        return Err(WindowsTransportError::ServerIdentityRejected);
    }
    let mut units: Vec<u16> = value.encode_utf16().collect();
    units.push(0);
    Ok(units)
}

fn map_service_open_error(error: io::Error) -> WindowsTransportError {
    match error.raw_os_error() {
        Some(2 | 3 | 1060) => WindowsTransportError::BrokerAbsent,
        _ => WindowsTransportError::Io(error),
    }
}

struct OwnedKernelHandle(HANDLE);

impl OwnedKernelHandle {
    fn new(raw: HANDLE) -> Result<Self, WindowsTransportError> {
        if raw.is_null() {
            Err(WindowsTransportError::Io(io::Error::last_os_error()))
        } else {
            Ok(Self(raw))
        }
    }

    fn raw(&self) -> HANDLE {
        self.0
    }
}

impl Drop for OwnedKernelHandle {
    fn drop(&mut self) {
        // SAFETY: the unique owner closes the validated kernel handle exactly once.
        let _ = unsafe { CloseHandle(self.0) };
    }
}

struct ServiceHandle(SC_HANDLE);

impl ServiceHandle {
    fn new(raw: SC_HANDLE) -> Result<Self, io::Error> {
        if raw.is_null() {
            Err(io::Error::last_os_error())
        } else {
            Ok(Self(raw))
        }
    }

    fn raw(&self) -> SC_HANDLE {
        self.0
    }
}

impl Drop for ServiceHandle {
    fn drop(&mut self) {
        // SAFETY: the unique owner closes the validated SCM handle exactly once.
        let _ = unsafe { CloseServiceHandle(self.0) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expected_broker_executable_name_includes_windows_extension() {
        assert_eq!(BROKER_EXECUTABLE_NAME, "cedarflake_ame_journal_broker.exe");
        assert!(BROKER_EXECUTABLE_NAME.ends_with(".exe"));
    }

    #[test]
    fn hung_identity_probe_is_terminated_at_the_absolute_deadline() {
        let mut terminated = false;
        let result = wait_for_identity_probe(
            Instant::now(),
            || Ok(None),
            || {
                terminated = true;
                Ok(())
            },
        );

        assert!(matches!(result, Err(WindowsTransportError::TimedOut)));
        assert!(terminated);
    }

    #[test]
    fn identity_probe_nonce_is_exact_uppercase_256_bit_hex() {
        let nonce = next_identity_probe_nonce();
        assert_eq!(nonce.len(), 64);
        assert!(validate_probe_nonce(&nonce).is_ok());
        assert!(validate_probe_nonce(&nonce.to_ascii_lowercase()).is_err());
        assert!(validate_probe_nonce(&nonce[..62]).is_err());
    }
}
