use std::ffi::c_void;
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
    CloseHandle, ERROR_INSUFFICIENT_BUFFER, ERROR_SERVICE_ALREADY_RUNNING, HANDLE, HLOCAL,
    LocalFree,
};
use windows_sys::Win32::Security::Authorization::ConvertSidToStringSidW;
use windows_sys::Win32::Security::Cryptography::{
    BCRYPT_SHA256_ALGORITHM, BCryptCloseAlgorithmProvider, BCryptHash, BCryptOpenAlgorithmProvider,
    CryptHashCertificate2,
};
use windows_sys::Win32::Security::WinTrust::{
    WINTRUST_ACTION_GENERIC_VERIFY_V2, WINTRUST_DATA, WINTRUST_DATA_0, WINTRUST_FILE_INFO,
    WTD_CACHE_ONLY_URL_RETRIEVAL, WTD_CHOICE_FILE, WTD_REVOCATION_CHECK_CHAIN_EXCLUDE_ROOT,
    WTD_REVOKE_WHOLECHAIN, WTD_STATEACTION_CLOSE, WTD_STATEACTION_VERIFY, WTD_UI_NONE,
    WTHelperGetProvCertFromChain, WTHelperGetProvSignerFromChain, WTHelperProvDataFromStateData,
    WinVerifyTrust,
};
use windows_sys::Win32::Security::{
    GetTokenInformation, IsTokenRestricted, TOKEN_ELEVATION, TOKEN_GROUPS, TOKEN_QUERY,
    TOKEN_STATISTICS, TOKEN_TYPE, TOKEN_USER, TokenElevation, TokenPrimary, TokenRestrictedSids,
    TokenSessionId, TokenStatistics, TokenType, TokenUser,
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
    CREATE_NO_WINDOW, OpenProcess, OpenProcessToken, PROCESS_QUERY_LIMITED_INFORMATION,
    QueryFullProcessImageNameW,
};
use windows_sys::Win32::UI::Shell::{
    FOLDERID_ProgramFilesX64, KF_FLAG_DEFAULT, SHGetKnownFolderPath,
};

use super::service_host::SERVICE_NAME;
use super::transport::{OwnedHandle, WindowsTransportError};
use crate::journal_broker::{BROKER_EXECUTABLE_NAME, MAX_FRAME_BYTES, PROTOCOL_VERSION};

const SERVICE_SID: &str = "S-1-5-80-2098581772-366539847-2170527201-1913264099-3006267874";
const LOCAL_SYSTEM_SID: &str = "S-1-5-18";
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
    manifest: ServiceIdentityManifest,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ServiceIdentityManifest {
    protocol: u16,
    max_frame: usize,
    binary_sha256: [u8; 32],
    signer_sha256: [u8; 32],
    publisher_utf16le: Vec<u8>,
    client_path_utf16le: Vec<u8>,
    client_sha256: [u8; 32],
    client_signer_sha256: [u8; 32],
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::journal_broker) struct VerifiedClientProcessIdentity {
    process_id: u32,
    session_id: u32,
    user_sid: Vec<u8>,
    authentication_id: [u8; 8],
    is_elevated: bool,
    opaque_binary_identity: [u8; 32],
}

impl VerifiedClientProcessIdentity {
    pub(in crate::journal_broker) fn matches_pipe_token(
        &self,
        process_id: u32,
        session_id: u32,
        user_sid: &[u8],
        authentication_id: [u8; 8],
        is_elevated: bool,
    ) -> bool {
        self.process_id == process_id
            && self.session_id == session_id
            && self.user_sid == user_sid
            && self.authentication_id == authentication_id
            && self.is_elevated == is_elevated
            && self.opaque_binary_identity != [0; 32]
    }

    pub(in crate::journal_broker) fn opaque_binary_identity(&self) -> [u8; 32] {
        self.opaque_binary_identity
    }
}

#[derive(Debug, PartialEq, Eq)]
struct ServerProcessFacts {
    scm_process_id: u32,
    pipe_process_id: u32,
    image_path: PathBuf,
    expected_image_path: PathBuf,
    token_user_sid: String,
    token_type: TOKEN_TYPE,
    is_elevated: bool,
    is_restricted: bool,
    restricted_sids: Vec<String>,
    manifest: ServiceIdentityManifest,
    actual_binary_sha256: [u8; 32],
    actual_signer_sha256: [u8; 32],
    signature_valid: bool,
}

#[derive(Debug, PartialEq, Eq)]
struct ClientProcessFacts {
    process_id: u32,
    image_path: PathBuf,
    expected_image_path: PathBuf,
    manifest: ServiceIdentityManifest,
    actual_binary_sha256: [u8; 32],
    actual_signer_sha256: [u8; 32],
    signature_valid: bool,
}

impl ClientProcessFacts {
    fn validate(&self) -> Result<(), WindowsTransportError> {
        if self.process_id == 0
            || !paths_equal(&self.image_path, &self.expected_image_path)
            || self.manifest.protocol != PROTOCOL_VERSION
            || self.manifest.max_frame != MAX_FRAME_BYTES
            || self.manifest.publisher_utf16le.is_empty()
            || self.manifest.client_path_utf16le != path_utf16le(&self.expected_image_path)?
            || self.manifest.client_sha256 != self.actual_binary_sha256
            || self.manifest.client_signer_sha256 != self.actual_signer_sha256
            || !self.signature_valid
        {
            return Err(WindowsTransportError::ServerIdentityRejected);
        }
        Ok(())
    }
}

impl ServerProcessFacts {
    fn validate(&self) -> Result<(), WindowsTransportError> {
        if self.scm_process_id == 0
            || self.pipe_process_id != self.scm_process_id
            || !paths_equal(&self.image_path, &self.expected_image_path)
            || self.token_user_sid != LOCAL_SYSTEM_SID
            || self.token_type != TokenPrimary
            || !self.is_elevated
            || !self.is_restricted
            || !self.restricted_sids.iter().any(|sid| sid == SERVICE_SID)
            || self.manifest.protocol != PROTOCOL_VERSION
            || self.manifest.max_frame != MAX_FRAME_BYTES
            || self.manifest.publisher_utf16le.is_empty()
            || self.manifest.binary_sha256 != self.actual_binary_sha256
            || self.manifest.signer_sha256 != self.actual_signer_sha256
            || !self.signature_valid
        {
            return Err(WindowsTransportError::ServerIdentityRejected);
        }
        Ok(())
    }
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

pub(in crate::journal_broker) fn current_process_has_installed_client_identity() -> bool {
    current_process_image_path()
        .and_then(|image_path| {
            expected_client_path().map(|expected_path| paths_equal(&image_path, &expected_path))
        })
        .unwrap_or(false)
}

fn current_process_image_path() -> Result<PathBuf, WindowsTransportError> {
    // SAFETY: the PID is the current process, the handle is non-inheritable, and only bounded
    // process-image metadata is queried before the handle's unique owner closes it.
    let process = OwnedKernelHandle::new(unsafe {
        OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, std::process::id())
    })?;
    process_image_path(&process)
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
        std::process::id().to_string(),
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
    let token = process_token(&process)?;
    ServerProcessFacts {
        scm_process_id: started_service.process_id,
        pipe_process_id,
        image_path,
        expected_image_path,
        token_user_sid: token_sid(&token)?,
        token_type: token_fixed::<TOKEN_TYPE>(&token, TokenType)?,
        is_elevated: token_fixed::<TOKEN_ELEVATION>(&token, TokenElevation)?.TokenIsElevated != 0,
        // SAFETY: token is a live process token handle with TOKEN_QUERY access.
        is_restricted: unsafe { IsTokenRestricted(token.raw()) } != 0,
        restricted_sids: token_group_sids(&token)?,
        manifest: started_service.manifest.clone(),
        actual_binary_sha256: started_service.manifest.binary_sha256,
        actual_signer_sha256: started_service.manifest.signer_sha256,
        signature_valid: true,
    }
    .validate()?;
    let client_path = std::env::current_exe().map_err(WindowsTransportError::Io)?;
    let expected_client_path = expected_client_path()?;
    ClientProcessFacts {
        process_id: std::process::id(),
        image_path: client_path,
        expected_image_path: expected_client_path,
        manifest: started_service.manifest.clone(),
        actual_binary_sha256: started_service.manifest.client_sha256,
        actual_signer_sha256: started_service.manifest.client_signer_sha256,
        signature_valid: true,
    }
    .validate()
}

pub(in crate::journal_broker) fn authenticate_pipe_client(
    process_id: u32,
) -> Result<VerifiedClientProcessIdentity, WindowsTransportError> {
    if process_id == 0 {
        return Err(WindowsTransportError::ServerIdentityRejected);
    }
    let manager = ServiceHandle::new(unsafe { OpenSCManagerW(null(), null(), SC_MANAGER_CONNECT) })
        .map_err(map_service_open_error)?;
    let service_name = nul_terminated(SERVICE_NAME)?;
    // SAFETY: the service name is fixed and terminated; this handle can only read configuration.
    let service = ServiceHandle::new(unsafe {
        OpenServiceW(manager.raw(), service_name.as_ptr(), SERVICE_QUERY_CONFIG)
    })
    .map_err(map_service_open_error)?;
    let manifest = query_identity_manifest(&service)?;
    let expected_image_path = expected_client_path()?;
    // SAFETY: the PID was observed by GetNamedPipeClientProcessId for the connected pipe and the
    // returned handle is non-inheritable. Its primary token is bound to the impersonated pipe token
    // before any request is admitted.
    let process = OwnedKernelHandle::new(unsafe {
        OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, process_id)
    })?;
    let image_path = process_image_path(&process)?;
    run_identity_probe(&["client".to_owned(), process_id.to_string()])?;
    ClientProcessFacts {
        process_id,
        image_path: image_path.clone(),
        expected_image_path,
        manifest: manifest.clone(),
        actual_binary_sha256: manifest.client_sha256,
        actual_signer_sha256: manifest.client_signer_sha256,
        signature_valid: true,
    }
    .validate()?;

    let token = process_token(&process)?;
    if token_fixed::<TOKEN_TYPE>(&token, TokenType)? != TokenPrimary {
        return Err(WindowsTransportError::ServerIdentityRejected);
    }
    let session_id = token_fixed::<u32>(&token, TokenSessionId)?;
    let statistics = token_fixed::<TOKEN_STATISTICS>(&token, TokenStatistics)?;
    let user_sid = token_sid_bytes(&token)?;
    let is_elevated = token_fixed::<TOKEN_ELEVATION>(&token, TokenElevation)?.TokenIsElevated != 0;

    let mut hasher = blake3::Hasher::new();
    hasher.update(b"cedarflake-ame-client-process-identity-v1\0");
    hasher.update(&manifest.client_path_utf16le);
    hasher.update(&manifest.client_sha256);
    hasher.update(&manifest.client_signer_sha256);
    let opaque_binary_identity = *hasher.finalize().as_bytes();
    if opaque_binary_identity == [0; 32] {
        return Err(WindowsTransportError::ServerIdentityRejected);
    }
    Ok(VerifiedClientProcessIdentity {
        process_id,
        session_id,
        user_sid,
        authentication_id: luid_bytes(statistics.AuthenticationId),
        is_elevated,
        opaque_binary_identity,
    })
}

pub(super) fn run_identity_probe_process(arguments: &[String]) -> ExitCode {
    let result = match arguments {
        [
            mode,
            scm_process_id,
            pipe_process_id,
            client_process_id,
            nonce,
        ] if mode == "server" => parse_probe_process_id(scm_process_id)
            .and_then(|scm_process_id| {
                parse_probe_process_id(pipe_process_id)
                    .map(|pipe_process_id| (scm_process_id, pipe_process_id))
            })
            .and_then(|(scm_process_id, pipe_process_id)| {
                parse_probe_process_id(client_process_id).and_then(|client_process_id| {
                    validate_probe_nonce(nonce)?;
                    verify_server_and_client_identity(
                        scm_process_id,
                        pipe_process_id,
                        client_process_id,
                    )?;
                    Ok(nonce)
                })
            }),
        [mode, client_process_id, nonce] if mode == "client" => {
            parse_probe_process_id(client_process_id).and_then(|client_process_id| {
                validate_probe_nonce(nonce)?;
                verify_client_identity(client_process_id)?;
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

fn verify_server_and_client_identity(
    scm_process_id: u32,
    pipe_process_id: u32,
    client_process_id: u32,
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
    let process = OwnedKernelHandle::new(unsafe {
        OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, scm_process_id)
    })?;
    let image_path = process_image_path(&process)?;
    let token = process_token(&process)?;
    let expected_image_path = expected_broker_path()?;
    ServerProcessFacts {
        scm_process_id,
        pipe_process_id,
        image_path: image_path.clone(),
        expected_image_path,
        token_user_sid: token_sid(&token)?,
        token_type: token_fixed::<TOKEN_TYPE>(&token, TokenType)?,
        is_elevated: token_fixed::<TOKEN_ELEVATION>(&token, TokenElevation)?.TokenIsElevated != 0,
        is_restricted: unsafe { IsTokenRestricted(token.raw()) } != 0,
        restricted_sids: token_group_sids(&token)?,
        actual_binary_sha256: binary_sha256(&image_path)?,
        actual_signer_sha256: verify_signature_and_signer_hash(&image_path)?,
        signature_valid: true,
        manifest,
    }
    .validate()?;
    verify_client_identity_with_manifest(client_process_id, query_identity_manifest(&service)?)
}

fn verify_client_identity(process_id: u32) -> Result<(), WindowsTransportError> {
    let manager = ServiceHandle::new(unsafe { OpenSCManagerW(null(), null(), SC_MANAGER_CONNECT) })
        .map_err(map_service_open_error)?;
    let service_name = nul_terminated(SERVICE_NAME)?;
    let service = ServiceHandle::new(unsafe {
        OpenServiceW(manager.raw(), service_name.as_ptr(), SERVICE_QUERY_CONFIG)
    })
    .map_err(map_service_open_error)?;
    verify_client_identity_with_manifest(process_id, query_identity_manifest(&service)?)
}

fn verify_client_identity_with_manifest(
    process_id: u32,
    manifest: ServiceIdentityManifest,
) -> Result<(), WindowsTransportError> {
    let expected_image_path = expected_client_path()?;
    let process = OwnedKernelHandle::new(unsafe {
        OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, process_id)
    })?;
    let image_path = process_image_path(&process)?;
    ClientProcessFacts {
        process_id,
        image_path: image_path.clone(),
        expected_image_path,
        actual_binary_sha256: binary_sha256(&image_path)?,
        actual_signer_sha256: verify_signature_and_signer_hash(&image_path)?,
        signature_valid: true,
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
) -> Result<ServiceIdentityManifest, WindowsTransportError> {
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
    parse_identity_manifest(&copy_bounded_utf16(description.lpDescription)?)
}

fn parse_identity_manifest(value: &str) -> Result<ServiceIdentityManifest, WindowsTransportError> {
    let fields: Vec<&str> = value.split('|').collect();
    if fields.len() != 9 || fields[0] != "AMEJBID2" {
        return Err(WindowsTransportError::ServerIdentityRejected);
    }
    let protocol = parse_manifest_field(fields[1], "protocol=")?
        .parse::<u16>()
        .map_err(|_| WindowsTransportError::ServerIdentityRejected)?;
    let max_frame = parse_manifest_field(fields[2], "max_frame=")?
        .parse::<usize>()
        .map_err(|_| WindowsTransportError::ServerIdentityRejected)?;
    let binary_sha256 = parse_hex_32(parse_manifest_field(fields[3], "sha256=")?)?;
    let signer_sha256 = parse_hex_32(parse_manifest_field(fields[4], "signer_sha256=")?)?;
    let publisher_utf16le =
        parse_hex(parse_manifest_field(fields[5], "publisher_utf16le=")?, 2048)?;
    let client_path_utf16le = parse_hex(
        parse_manifest_field(fields[6], "client_path_utf16le=")?,
        32_768 * 2,
    )?;
    let client_sha256 = parse_hex_32(parse_manifest_field(fields[7], "client_sha256=")?)?;
    let client_signer_sha256 =
        parse_hex_32(parse_manifest_field(fields[8], "client_signer_sha256=")?)?;
    if publisher_utf16le.is_empty()
        || publisher_utf16le.len() % 2 != 0
        || client_path_utf16le.is_empty()
        || client_path_utf16le.len() % 2 != 0
        || path_from_utf16le(&client_path_utf16le).is_err()
    {
        return Err(WindowsTransportError::ServerIdentityRejected);
    }
    Ok(ServiceIdentityManifest {
        protocol,
        max_frame,
        binary_sha256,
        signer_sha256,
        publisher_utf16le,
        client_path_utf16le,
        client_sha256,
        client_signer_sha256,
    })
}

fn parse_manifest_field<'a>(
    field: &'a str,
    prefix: &str,
) -> Result<&'a str, WindowsTransportError> {
    field
        .strip_prefix(prefix)
        .filter(|value| !value.is_empty())
        .ok_or(WindowsTransportError::ServerIdentityRejected)
}

fn parse_hex_32(value: &str) -> Result<[u8; 32], WindowsTransportError> {
    parse_hex(value, 32)?
        .try_into()
        .map_err(|_| WindowsTransportError::ServerIdentityRejected)
}

fn parse_hex(value: &str, maximum_bytes: usize) -> Result<Vec<u8>, WindowsTransportError> {
    if !value.len().is_multiple_of(2) || value.len() / 2 > maximum_bytes {
        return Err(WindowsTransportError::ServerIdentityRejected);
    }
    value
        .as_bytes()
        .chunks_exact(2)
        .map(|chunk| {
            let high = hex_nibble(chunk[0])?;
            let low = hex_nibble(chunk[1])?;
            Ok((high << 4) | low)
        })
        .collect()
}

fn hex_nibble(value: u8) -> Result<u8, WindowsTransportError> {
    match value {
        b'0'..=b'9' => Ok(value - b'0'),
        b'A'..=b'F' => Ok(value - b'A' + 10),
        _ => Err(WindowsTransportError::ServerIdentityRejected),
    }
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

fn verify_signature_and_signer_hash(path: &Path) -> Result<[u8; 32], WindowsTransportError> {
    let mut path_utf16: Vec<u16> = path.as_os_str().to_string_lossy().encode_utf16().collect();
    if path_utf16.is_empty() || path_utf16.len() >= 32_768 || path_utf16.contains(&0) {
        return Err(WindowsTransportError::ServerIdentityRejected);
    }
    path_utf16.push(0);
    let mut file = WINTRUST_FILE_INFO {
        cbStruct: u32::try_from(size_of::<WINTRUST_FILE_INFO>())
            .map_err(|_| WindowsTransportError::ServerIdentityRejected)?,
        pcwszFilePath: path_utf16.as_ptr(),
        hFile: null_mut(),
        pgKnownSubject: null_mut(),
    };
    let mut data = WINTRUST_DATA {
        cbStruct: u32::try_from(size_of::<WINTRUST_DATA>())
            .map_err(|_| WindowsTransportError::ServerIdentityRejected)?,
        dwUIChoice: WTD_UI_NONE,
        fdwRevocationChecks: WTD_REVOKE_WHOLECHAIN,
        dwUnionChoice: WTD_CHOICE_FILE,
        Anonymous: WINTRUST_DATA_0 { pFile: &mut file },
        dwStateAction: WTD_STATEACTION_VERIFY,
        dwProvFlags: WTD_CACHE_ONLY_URL_RETRIEVAL | WTD_REVOCATION_CHECK_CHAIN_EXCLUDE_ROOT,
        ..WINTRUST_DATA::default()
    };
    let mut action = WINTRUST_ACTION_GENERIC_VERIFY_V2;
    // SAFETY: all nested structures and the terminated file path remain live until the matching
    // WTD_STATEACTION_CLOSE call below. UI is disabled and network retrieval is cache-only.
    let verified = unsafe { WinVerifyTrust(null_mut(), &mut action, (&raw mut data).cast()) } == 0;
    let signer_hash = if verified {
        // SAFETY: successful VERIFY populated state data; helper outputs are checked before use.
        let provider = unsafe { WTHelperProvDataFromStateData(data.hWVTStateData) };
        let signer = if provider.is_null() {
            null_mut()
        } else {
            unsafe { WTHelperGetProvSignerFromChain(provider, 0, 0, 0) }
        };
        let certificate = if signer.is_null() {
            null_mut()
        } else {
            unsafe { WTHelperGetProvCertFromChain(signer, 0) }
        };
        if certificate.is_null() || unsafe { (*certificate).pCert }.is_null() {
            None
        } else {
            let context = unsafe { (*certificate).pCert };
            let mut digest = [0_u8; 32];
            let mut length = digest.len() as u32;
            // SAFETY: the certificate context and encoded certificate are owned by WinTrust state
            // until CLOSE; digest and length are live bounded outputs.
            let hashed = unsafe {
                CryptHashCertificate2(
                    BCRYPT_SHA256_ALGORITHM,
                    0,
                    null(),
                    (*context).pbCertEncoded,
                    (*context).cbCertEncoded,
                    digest.as_mut_ptr(),
                    &mut length,
                )
            } != 0;
            (hashed && length == digest.len() as u32).then_some(digest)
        }
    } else {
        None
    };
    data.dwStateAction = WTD_STATEACTION_CLOSE;
    // SAFETY: this closes only the state created by the matching VERIFY call above.
    let _ = unsafe { WinVerifyTrust(null_mut(), &mut action, (&raw mut data).cast()) };
    signer_hash.ok_or(WindowsTransportError::ServerIdentityRejected)
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

fn expected_client_path() -> Result<PathBuf, WindowsTransportError> {
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
        .join("Application")
        .join("cedarflake_ame.exe"))
}

fn path_utf16le(path: &Path) -> Result<Vec<u8>, WindowsTransportError> {
    let text = path.as_os_str().to_string_lossy();
    if text.is_empty() || text.encode_utf16().count() >= 32_768 {
        return Err(WindowsTransportError::ServerIdentityRejected);
    }
    Ok(text
        .encode_utf16()
        .flat_map(u16::to_le_bytes)
        .collect::<Vec<_>>())
}

fn path_from_utf16le(bytes: &[u8]) -> Result<PathBuf, WindowsTransportError> {
    if bytes.is_empty() || !bytes.len().is_multiple_of(2) || bytes.len() / 2 >= 32_768 {
        return Err(WindowsTransportError::ServerIdentityRejected);
    }
    let units: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
        .collect();
    if units.contains(&0) {
        return Err(WindowsTransportError::ServerIdentityRejected);
    }
    let text =
        String::from_utf16(&units).map_err(|_| WindowsTransportError::ServerIdentityRejected)?;
    let path = PathBuf::from(text);
    if !path.is_absolute() {
        return Err(WindowsTransportError::ServerIdentityRejected);
    }
    Ok(path)
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

fn process_token(process: &OwnedKernelHandle) -> Result<OwnedKernelHandle, WindowsTransportError> {
    let mut token = null_mut();
    // SAFETY: process is live and token is a live output slot.
    if unsafe { OpenProcessToken(process.raw(), TOKEN_QUERY, &mut token) } == 0 {
        return Err(WindowsTransportError::Io(io::Error::last_os_error()));
    }
    OwnedKernelHandle::new(token)
}

fn token_sid(token: &OwnedKernelHandle) -> Result<String, WindowsTransportError> {
    let bytes = token_information(token, TokenUser)?;
    let sid = token_user_sid_slice(&bytes)?;
    sid_string(sid.as_ptr().cast_mut().cast())
}

fn token_sid_bytes(token: &OwnedKernelHandle) -> Result<Vec<u8>, WindowsTransportError> {
    let bytes = token_information(token, TokenUser)?;
    Ok(token_user_sid_slice(&bytes)?.to_vec())
}

fn token_user_sid_slice(bytes: &[u8]) -> Result<&[u8], WindowsTransportError> {
    if bytes.len() < size_of::<TOKEN_USER>() {
        return Err(WindowsTransportError::ServerIdentityRejected);
    }
    // SAFETY: the fixed header fits; read_unaligned does not form a reference with Vec<u8>'s
    // alignment. No SID memory is accessed until its complete structural length is in `bytes`.
    let user = unsafe { bytes.as_ptr().cast::<TOKEN_USER>().read_unaligned() };
    let sid = user.User.Sid.cast::<u8>();
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

fn luid_bytes(value: windows_sys::Win32::Foundation::LUID) -> [u8; 8] {
    let mut bytes = [0_u8; 8];
    bytes[..4].copy_from_slice(&value.LowPart.to_le_bytes());
    bytes[4..].copy_from_slice(&value.HighPart.to_le_bytes());
    bytes
}

fn token_fixed<T: Copy>(
    token: &OwnedKernelHandle,
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

fn token_group_sids(token: &OwnedKernelHandle) -> Result<Vec<String>, WindowsTransportError> {
    let bytes = token_information(token, TokenRestrictedSids)?;
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
        result.push(sid_string(group.Sid)?);
    }
    Ok(result)
}

fn token_information(
    token: &OwnedKernelHandle,
    information_class: i32,
) -> Result<Vec<u8>, WindowsTransportError> {
    let mut required = 0_u32;
    // SAFETY: this sizing query intentionally supplies no buffer.
    let _ = unsafe {
        GetTokenInformation(token.raw(), information_class, null_mut(), 0, &mut required)
    };
    if required == 0 || required > 1024 * 1024 {
        return Err(WindowsTransportError::ServerIdentityRejected);
    }
    let mut bytes = vec![0_u8; required as usize];
    // SAFETY: bytes is exactly the bounded requested size and all output pointers are live.
    if unsafe {
        GetTokenInformation(
            token.raw(),
            information_class,
            bytes.as_mut_ptr().cast(),
            required,
            &mut required,
        )
    } == 0
    {
        return Err(WindowsTransportError::Io(io::Error::last_os_error()));
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

fn paths_equal(left: &Path, right: &Path) -> bool {
    left.as_os_str()
        .to_string_lossy()
        .eq_ignore_ascii_case(&right.as_os_str().to_string_lossy())
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

    type ServerFactsMutation = Box<dyn Fn(&mut ServerProcessFacts)>;
    type ClientFactsMutation = Box<dyn Fn(&mut ClientProcessFacts)>;

    fn valid_facts() -> ServerProcessFacts {
        let digest = [5; 32];
        ServerProcessFacts {
            scm_process_id: 42,
            pipe_process_id: 42,
            image_path: PathBuf::from(
                r"C:\Program Files\Cedarflake Ame\Journal Broker\cedarflake_ame_journal_broker.exe",
            ),
            expected_image_path: PathBuf::from(
                r"c:\PROGRAM FILES\Cedarflake Ame\Journal Broker\cedarflake_ame_journal_broker.exe",
            ),
            token_user_sid: LOCAL_SYSTEM_SID.to_owned(),
            token_type: TokenPrimary,
            is_elevated: true,
            is_restricted: true,
            restricted_sids: vec![SERVICE_SID.to_owned()],
            manifest: ServiceIdentityManifest {
                protocol: PROTOCOL_VERSION,
                max_frame: MAX_FRAME_BYTES,
                binary_sha256: digest,
                signer_sha256: digest,
                publisher_utf16le: vec![67, 0, 78, 0],
                client_path_utf16le: path_utf16le(&PathBuf::from(
                    r"C:\Program Files\Cedarflake Ame\Application\cedarflake_ame.exe",
                ))
                .expect("client path"),
                client_sha256: digest,
                client_signer_sha256: digest,
            },
            actual_binary_sha256: digest,
            actual_signer_sha256: digest,
            signature_valid: true,
        }
    }

    fn valid_client_facts() -> ClientProcessFacts {
        let manifest = valid_facts().manifest;
        ClientProcessFacts {
            process_id: 43,
            image_path: PathBuf::from(
                r"c:\PROGRAM FILES\Cedarflake Ame\Application\cedarflake_ame.exe",
            ),
            expected_image_path: PathBuf::from(
                r"C:\Program Files\Cedarflake Ame\Application\cedarflake_ame.exe",
            ),
            actual_binary_sha256: manifest.client_sha256,
            actual_signer_sha256: manifest.client_signer_sha256,
            signature_valid: true,
            manifest,
        }
    }

    #[test]
    fn server_identity_requires_scm_pipe_pid_path_and_restricted_service_token() {
        assert!(valid_facts().validate().is_ok());

        let mutations: Vec<ServerFactsMutation> = vec![
            Box::new(|facts| facts.pipe_process_id += 1),
            Box::new(|facts| facts.image_path = PathBuf::from(r"C:\Temp\broker.exe")),
            Box::new(|facts| facts.token_user_sid = "S-1-5-21-1".to_owned()),
            Box::new(|facts| facts.token_type = windows_sys::Win32::Security::TokenImpersonation),
            Box::new(|facts| facts.is_elevated = false),
            Box::new(|facts| facts.is_restricted = false),
            Box::new(|facts| facts.restricted_sids.clear()),
            Box::new(|facts| facts.manifest.protocol += 1),
            Box::new(|facts| facts.manifest.max_frame += 1),
            Box::new(|facts| facts.actual_binary_sha256[0] ^= 1),
            Box::new(|facts| facts.actual_signer_sha256[0] ^= 1),
            Box::new(|facts| facts.signature_valid = false),
        ];
        for mutation in mutations {
            let mut facts = valid_facts();
            mutation(&mut facts);
            assert!(matches!(
                facts.validate(),
                Err(WindowsTransportError::ServerIdentityRejected)
            ));
        }
    }

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

    #[test]
    fn identity_manifest_parser_is_exact_and_bounded() {
        let digest = "05".repeat(32);
        let client_path =
            ConvertTestPath::new(r"C:\Program Files\Cedarflake Ame\Application\cedarflake_ame.exe");
        let text = format!(
            "AMEJBID2|protocol={PROTOCOL_VERSION}|max_frame={MAX_FRAME_BYTES}|sha256={digest}|signer_sha256={digest}|publisher_utf16le=43004E00|client_path_utf16le={}|client_sha256={digest}|client_signer_sha256={digest}",
            client_path.hex
        );
        let parsed = parse_identity_manifest(&text).expect("valid manifest");
        assert_eq!(parsed.binary_sha256, [5; 32]);
        assert_eq!(parsed.signer_sha256, [5; 32]);
        assert_eq!(parsed.publisher_utf16le, vec![67, 0, 78, 0]);
        assert_eq!(parsed.client_path_utf16le, client_path.bytes);
        assert_eq!(parsed.client_sha256, [5; 32]);
        assert_eq!(parsed.client_signer_sha256, [5; 32]);

        for invalid in [
            text.replace("AMEJBID2", "AMEJBID1"),
            text.replace("|sha256=", "|extra=1|sha256="),
            text.replace("43004E00", ""),
            text.replace(&client_path.hex, "4300"),
            text.to_ascii_lowercase(),
        ] {
            assert!(parse_identity_manifest(&invalid).is_err());
        }
    }

    #[test]
    fn client_identity_requires_fixed_path_hash_signer_and_valid_signature() {
        assert!(valid_client_facts().validate().is_ok());
        let mutations: Vec<ClientFactsMutation> = vec![
            Box::new(|facts| facts.process_id = 0),
            Box::new(|facts| facts.image_path = PathBuf::from(r"C:\Users\Public\ame.exe")),
            Box::new(|facts| facts.manifest.client_path_utf16le[0] ^= 1),
            Box::new(|facts| facts.actual_binary_sha256[0] ^= 1),
            Box::new(|facts| facts.actual_signer_sha256[0] ^= 1),
            Box::new(|facts| facts.signature_valid = false),
        ];
        for mutation in mutations {
            let mut facts = valid_client_facts();
            mutation(&mut facts);
            assert!(matches!(
                facts.validate(),
                Err(WindowsTransportError::ServerIdentityRejected)
            ));
        }
    }

    #[test]
    fn client_process_identity_is_bound_to_pipe_token_facts() {
        let identity = VerifiedClientProcessIdentity {
            process_id: 43,
            session_id: 3,
            user_sid: vec![1, 2, 3],
            authentication_id: [4; 8],
            is_elevated: false,
            opaque_binary_identity: [5; 32],
        };
        assert!(identity.matches_pipe_token(43, 3, &[1, 2, 3], [4; 8], false));
        assert!(!identity.matches_pipe_token(44, 3, &[1, 2, 3], [4; 8], false));
        assert!(!identity.matches_pipe_token(43, 4, &[1, 2, 3], [4; 8], false));
        assert!(!identity.matches_pipe_token(43, 3, &[1, 2], [4; 8], false));
        assert!(!identity.matches_pipe_token(43, 3, &[1, 2, 3], [6; 8], false));
        assert!(!identity.matches_pipe_token(43, 3, &[1, 2, 3], [4; 8], true));
    }

    fn write_token_user_header(bytes: &mut [u8], sid_offset: usize) {
        let sid = unsafe { bytes.as_mut_ptr().add(sid_offset) }.cast();
        let header = TOKEN_USER {
            User: windows_sys::Win32::Security::SID_AND_ATTRIBUTES {
                Sid: sid,
                Attributes: 0,
            },
        };
        // SAFETY: tests reserve a complete TOKEN_USER header and deliberately exercise unaligned
        // storage; write_unaligned initializes it without forming an unaligned reference.
        unsafe {
            bytes
                .as_mut_ptr()
                .cast::<TOKEN_USER>()
                .write_unaligned(header)
        };
    }

    fn local_system_sid_bytes() -> [u8; 12] {
        [1, 1, 0, 0, 0, 0, 0, 5, 18, 0, 0, 0]
    }

    #[test]
    fn token_user_sid_parser_accepts_unaligned_complete_buffer() {
        let sid_offset = size_of::<TOKEN_USER>();
        let sid = local_system_sid_bytes();
        let mut allocation = vec![0_u8; 1 + sid_offset + sid.len()];
        let bytes = &mut allocation[1..];
        assert_ne!(
            bytes.as_ptr() as usize % std::mem::align_of::<TOKEN_USER>(),
            0
        );
        bytes[sid_offset..].copy_from_slice(&sid);
        write_token_user_header(bytes, sid_offset);

        assert_eq!(token_user_sid_slice(bytes).expect("valid SID"), sid);
    }

    #[test]
    fn token_user_sid_parser_rejects_out_of_buffer_and_malformed_sid() {
        let sid_offset = size_of::<TOKEN_USER>();
        let sid = local_system_sid_bytes();
        let mut complete = vec![0_u8; sid_offset + sid.len()];
        complete[sid_offset..].copy_from_slice(&sid);
        write_token_user_header(&mut complete, sid_offset);
        assert!(token_user_sid_slice(&complete[..complete.len() - 1]).is_err());

        let mut outside = vec![0_u8; sid_offset + sid.len()];
        let outside_offset = outside.len();
        write_token_user_header(&mut outside, outside_offset);
        assert!(token_user_sid_slice(&outside).is_err());

        let mut short_header = vec![0_u8; sid_offset + 7];
        write_token_user_header(&mut short_header, sid_offset);
        assert!(token_user_sid_slice(&short_header).is_err());

        let mut malformed = vec![0_u8; sid_offset + sid.len()];
        malformed[sid_offset..].copy_from_slice(&sid);
        malformed[sid_offset] = 0;
        write_token_user_header(&mut malformed, sid_offset);
        assert!(token_user_sid_slice(&malformed).is_err());
    }

    struct ConvertTestPath {
        bytes: Vec<u8>,
        hex: String,
    }

    impl ConvertTestPath {
        fn new(value: &str) -> Self {
            let bytes = value
                .encode_utf16()
                .flat_map(u16::to_le_bytes)
                .collect::<Vec<_>>();
            let hex = bytes.iter().map(|byte| format!("{byte:02X}")).collect();
            Self { bytes, hex }
        }
    }
}
