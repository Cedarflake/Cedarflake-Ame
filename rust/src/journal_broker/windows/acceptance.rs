use std::io;
use std::mem::size_of;
use std::path::Path;
use std::process::ExitCode;
use std::ptr::{null, null_mut};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use windows_sys::Win32::Foundation::{
    CloseHandle, GENERIC_READ, GENERIC_WRITE, HANDLE, INVALID_HANDLE_VALUE,
};
use windows_sys::Win32::Security::{
    GetTokenInformation, TOKEN_ELEVATION, TOKEN_QUERY, TokenElevation,
};
use windows_sys::Win32::Storage::FileSystem::{CreateFileW, OPEN_EXISTING, ReadFile, WriteFile};
use windows_sys::Win32::System::Pipes::WaitNamedPipeW;
use windows_sys::Win32::System::RemoteDesktop::ProcessIdToSessionId;
use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

use super::connection::ServerConnectionIdentity;
use super::transport::{PRODUCTION_PIPE_NAME, WindowsNamedPipeTransport};
use crate::journal_broker::client::{AdapterPoll, BrokerClientTransport};
use crate::journal_broker::framing::{decode_frame, encode_frame};
use crate::journal_broker::service::{
    describe_disposable_root_for_acceptance, open_directory_handle,
};
use crate::journal_broker::wire::{BrokerRequest, decode_response, encode_request};
use crate::journal_broker::{
    BrokerFailure, BrokerFailureCode, BrokerResponse, CallerClaim, CandidateScope,
    JournalCapability, PersistentChangeJournal, PersistentChangeJournalConnection,
    PersistentChangeJournalOperationError, QueryJournalRequest, ReadJournalRangeRequest,
    RegisterRootRequest, RootAuthorization,
};

const ACCEPTANCE_TOKEN: &str = "CEDARFLAKE_AME_WINDOWS_JOURNAL_BROKER_ACCEPTANCE_V1";
const ACCEPTANCE_READ_WINDOW: i64 = 64 * 1024;
const ACCEPTANCE_TIMEOUT_MS: u32 = 10_000;
const RESULT_PIPE_TIMEOUT_MS: u32 = 10_000;
const MAX_RESULT_BYTES: usize = 64 * 1024;

pub(super) fn run_process() -> ExitCode {
    if std::env::args_os().nth(1).as_deref()
        == Some(std::ffi::OsStr::new("--acceptance-protocol-info"))
    {
        println!("binary=cedarflake_ame_broker_acceptance_client protocol=1");
        return ExitCode::SUCCESS;
    }
    let arguments = match AcceptanceArguments::parse() {
        Ok(arguments) => arguments,
        Err(error) => {
            eprintln!("acceptance_client_arguments_rejected:{error}");
            return ExitCode::FAILURE;
        }
    };
    let process_id = std::process::id();
    let result = run_acceptance(&arguments);
    let (status, payload) = match result {
        Ok(payload) => ("passed", payload),
        Err(error) => ("failed", error),
    };
    let record = format!(
        "AME_BROKER_LIMITED_RESULT_V1 nonce={} phase={} instance={} pid={} status={} payload_hex={}",
        arguments.nonce,
        arguments.phase,
        arguments.instance,
        process_id,
        status,
        encode_hex(payload.as_bytes())
    );
    if let Err(error) = write_result(&arguments.pipe_name, record.as_bytes()) {
        eprintln!("acceptance_client_result_failed:{error}");
        return ExitCode::FAILURE;
    }
    if status == "passed" {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

struct AcceptanceArguments {
    pipe_name: String,
    nonce: String,
    phase: String,
    instance: String,
    root: String,
    internal_marker: String,
    external_marker: String,
    authorization_token: String,
}

impl AcceptanceArguments {
    fn parse() -> Result<Self, String> {
        let mut values = std::collections::HashMap::new();
        let mut arguments = std::env::args().skip(1);
        while let Some(name) = arguments.next() {
            let value = arguments
                .next()
                .ok_or_else(|| format!("missing value for {name}"))?;
            if !matches!(
                name.as_str(),
                "--pipe"
                    | "--nonce"
                    | "--phase"
                    | "--instance"
                    | "--root"
                    | "--internal-marker"
                    | "--external-marker"
                    | "--authorization-token"
            ) || values.insert(name.clone(), value).is_some()
            {
                return Err(format!("unknown or duplicate argument {name}"));
            }
        }
        let take = |name: &str| {
            values
                .get(name)
                .cloned()
                .ok_or_else(|| format!("missing {name}"))
        };
        let parsed = Self {
            pipe_name: take("--pipe")?,
            nonce: take("--nonce")?,
            phase: take("--phase")?,
            instance: take("--instance")?,
            root: take("--root")?,
            internal_marker: take("--internal-marker")?,
            external_marker: take("--external-marker")?,
            authorization_token: take("--authorization-token")?,
        };
        if !parsed.pipe_name.starts_with("CedarflakeAme.Acceptance.")
            || parsed.pipe_name.len() != "CedarflakeAme.Acceptance.".len() + 32
            || !is_ascii_hex(&parsed.pipe_name["CedarflakeAme.Acceptance.".len()..])
        {
            return Err("pipe name is outside the scoped acceptance namespace".to_owned());
        }
        if parsed.nonce.len() != 64 || !is_ascii_hex(&parsed.nonce) {
            return Err("nonce must be 256-bit hexadecimal".to_owned());
        }
        if parsed.instance.len() != 32 || !is_ascii_hex(&parsed.instance) {
            return Err("instance must be 128-bit hexadecimal".to_owned());
        }
        if !matches!(parsed.phase.as_str(), "first" | "second") {
            return Err("phase is invalid".to_owned());
        }
        if parsed.authorization_token != ACCEPTANCE_TOKEN {
            return Err("authorization token is invalid".to_owned());
        }
        if parsed.root.is_empty()
            || !is_marker(&parsed.internal_marker)
            || !is_marker(&parsed.external_marker)
        {
            return Err("root or marker input is invalid".to_owned());
        }
        Ok(parsed)
    }
}

fn is_ascii_hex(value: &str) -> bool {
    value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn is_marker(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 255
        && !value.contains(['\\', '/', '\0'])
        && value != "."
        && value != ".."
}

fn run_acceptance(arguments: &AcceptanceArguments) -> Result<String, String> {
    assert_limited_token()?;
    let (caller, root) = acceptance_request_facts(&arguments.root)?;
    let root_handle = open_directory_handle(&root.canonical_root_utf16)
        .map_err(|error| format!("open metadata-only root handle: {error}"))?;
    let client = match super::transport::WindowsPersistentChangeJournal.connect() {
        PersistentChangeJournalConnection::Connected(client) => client,
        PersistentChangeJournalConnection::LiveOnly(reason) => {
            return Err(format!("installed broker unavailable: {reason:?}"));
        }
    };
    let root_capability = client
        .register_root(RegisterRootRequest {
            caller: caller.clone(),
            root: root.clone(),
            client_root_handle: root_handle.raw() as usize as u64,
            timeout_ms: ACCEPTANCE_TIMEOUT_MS,
        })
        .map_err(|error| format!("register root: {error}"))?;
    let query = client
        .query_journal(QueryJournalRequest {
            caller: caller.clone(),
            root: root.clone(),
            root_capability,
            timeout_ms: ACCEPTANCE_TIMEOUT_MS,
        })
        .map_err(|error| format!("query existing journal: {error}"))?;
    let (journal_id, first_usn, next_usn) = match query {
        BrokerResponse::Journal {
            capability: JournalCapability::Supported,
            journal_id: Some(journal_id),
            first_usn: Some(first_usn),
            next_usn: Some(next_usn),
            ..
        } => (journal_id, first_usn, next_usn),
        response => return Err(format!("unexpected journal response: {response:?}")),
    };
    let start_usn = next_usn
        .saturating_sub(ACCEPTANCE_READ_WINDOW)
        .max(first_usn);
    if start_usn >= next_usn {
        return Err("disposable fixture produced no readable USN range".to_owned());
    }
    let read_request = ReadJournalRangeRequest {
        caller: caller.clone(),
        root: root.clone(),
        root_capability,
        journal_id,
        start_usn,
        end_usn: next_usn,
        max_records: 1_024,
        max_evidence_bytes: 512 * 1_024,
        timeout_ms: ACCEPTANCE_TIMEOUT_MS,
    };
    let read = client
        .read_range(read_request.clone())
        .map_err(|error| format!("read bounded journal range: {error}"))?;
    let (covered_until_usn, candidates) = match read {
        BrokerResponse::ReadRange {
            covered_until_usn,
            candidates,
            ..
        } => (covered_until_usn, candidates),
        response => return Err(format!("unexpected read response: {response:?}")),
    };
    if covered_until_usn != next_usn {
        return Err("broker did not prove the requested covered-until boundary".to_owned());
    }
    if candidates.iter().any(|candidate| match &candidate.scope {
        CandidateScope::Root => false,
        CandidateScope::RelativePath(path) => path.contains(&arguments.external_marker),
    }) {
        return Err("root-external marker was disclosed".to_owned());
    }
    if !candidates.iter().any(|candidate| {
        matches!(
            &candidate.scope,
            CandidateScope::RelativePath(path) if path.ends_with(&arguments.internal_marker)
        )
    }) {
        return Err("internal marker was not returned".to_owned());
    }
    let pending = client
        .begin_read_range(read_request)
        .map_err(|error| format!("begin cancellable read: {error}"))?;
    let cancel_outcome = match pending.cancel(caller.clone()) {
        Ok(()) => match pending.wait() {
            Err(PersistentChangeJournalOperationError::Cancelled) => "cancelled",
            other => return Err(format!("cancelled read terminal mismatch: {other:?}")),
        },
        Err(PersistentChangeJournalOperationError::RequestNotActive) => {
            pending
                .wait()
                .map_err(|error| format!("completed cancellation race: {error}"))?;
            "completed-race"
        }
        Err(error) => return Err(format!("cancel read: {error}")),
    };
    client
        .close()
        .map_err(|error| format!("close installed broker client: {error}"))?;
    let mismatch_identity = prove_protocol_mismatch(caller, root)?;
    Ok(format!(
        "journal_id={journal_id} start_usn={start_usn} covered_until_usn={covered_until_usn} candidates={} cancel={cancel_outcome} identity={} generation={} nonce={}",
        candidates.len(),
        encode_hex(&mismatch_identity.connection_id),
        mismatch_identity.connection_generation,
        mismatch_identity.connection_nonce
    ))
}

fn prove_protocol_mismatch(
    caller: CallerClaim,
    root: RootAuthorization,
) -> Result<ServerConnectionIdentity, String> {
    let (mut transport, identity) = WindowsNamedPipeTransport::connect(PRODUCTION_PIPE_NAME)
        .map_err(|error| format!("connect mismatch transport: {error}"))?;
    let request_id = 41_u64;
    let mut payload = encode_request(&BrokerRequest::QueryJournal {
        request_id,
        request: QueryJournalRequest {
            caller,
            root,
            root_capability: crate::journal_broker::RootCapability([7; 32]),
            timeout_ms: ACCEPTANCE_TIMEOUT_MS,
        },
    })
    .map_err(|error| format!("encode mismatch request: {error}"))?;
    payload[8..10].copy_from_slice(&(crate::journal_broker::PROTOCOL_VERSION + 1).to_le_bytes());
    let frame =
        encode_frame(&payload).map_err(|error| format!("frame mismatch request: {error}"))?;
    let deadline = Instant::now() + Duration::from_secs(2);
    while matches!(
        transport
            .try_send_frame(&frame)
            .map_err(|error| format!("send mismatch request: {error}"))?,
        AdapterPoll::Pending
    ) {
        wait_until(deadline, "mismatch send")?;
    }
    let response = loop {
        match transport
            .poll_response(request_id)
            .map_err(|error| format!("poll mismatch response: {error}"))?
        {
            AdapterPoll::Ready(frame) => {
                let payload = decode_frame(&frame)
                    .map_err(|error| format!("decode mismatch frame: {error}"))?;
                break decode_response(payload)
                    .map_err(|error| format!("decode mismatch response: {error}"))?;
            }
            AdapterPoll::Pending => wait_until(deadline, "mismatch response")?,
        }
    };
    if !matches!(
        response,
        BrokerResponse::Failure {
            request_id: 41,
            failure: BrokerFailure {
                code: BrokerFailureCode::ProtocolMismatch
            },
            ..
        }
    ) {
        return Err(format!("unexpected mismatch response: {response:?}"));
    }
    loop {
        match transport
            .begin_close()
            .map_err(|error| format!("close mismatch transport: {error}"))?
        {
            AdapterPoll::Ready(()) => break,
            AdapterPoll::Pending => wait_until(deadline, "mismatch close")?,
        }
    }
    Ok(identity)
}

fn wait_until(deadline: Instant, operation: &str) -> Result<(), String> {
    if Instant::now() >= deadline {
        return Err(format!("{operation} timed out"));
    }
    std::thread::sleep(Duration::from_millis(1));
    Ok(())
}

fn acceptance_request_facts(root_path: &str) -> Result<(CallerClaim, RootAuthorization), String> {
    let root = describe_disposable_root_for_acceptance(Path::new(root_path))
        .map_err(|error| format!("describe disposable root: {error}"))?;
    let process_id = std::process::id();
    let mut session_id = 0_u32;
    // SAFETY: session_id is a live output slot and process_id names this process.
    if unsafe { ProcessIdToSessionId(process_id, &mut session_id) } == 0 {
        return Err(format!(
            "query client session: {}",
            io::Error::last_os_error()
        ));
    }
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"cedarflake-ame-broker-acceptance-client-v1\0");
    hasher.update(&process_id.to_le_bytes());
    hasher.update(
        &SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
            .to_le_bytes(),
    );
    let mut client_instance = [0_u8; 16];
    client_instance.copy_from_slice(&hasher.finalize().as_bytes()[..16]);
    Ok((
        CallerClaim {
            process_id,
            session_id,
            client_instance,
        },
        root,
    ))
}

fn assert_limited_token() -> Result<(), String> {
    let mut token = null_mut();
    // SAFETY: the process pseudo-handle is valid and token is a live output slot.
    if unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) } == 0 {
        return Err(format!("open client token: {}", io::Error::last_os_error()));
    }
    let token = OwnedHandle(token);
    let mut elevation = TOKEN_ELEVATION::default();
    let mut returned = 0_u32;
    // SAFETY: token is live, elevation is initialized and exactly sized, and no pointer escapes.
    if unsafe {
        GetTokenInformation(
            token.0,
            TokenElevation,
            (&raw mut elevation).cast(),
            u32::try_from(size_of::<TOKEN_ELEVATION>())
                .map_err(|_| "token elevation size overflow")?,
            &mut returned,
        )
    } == 0
        || returned as usize != size_of::<TOKEN_ELEVATION>()
    {
        return Err(format!(
            "query client elevation: {}",
            io::Error::last_os_error()
        ));
    }
    if elevation.TokenIsElevated != 0 {
        return Err("acceptance client token is elevated".to_owned());
    }
    Ok(())
}

fn write_result(pipe_name: &str, bytes: &[u8]) -> Result<(), String> {
    if bytes.is_empty() || bytes.len() > MAX_RESULT_BYTES {
        return Err("result record is outside its bound".to_owned());
    }
    let full_name = format!(r"\\.\pipe\{pipe_name}");
    let name = nul_terminated(&full_name)?;
    // SAFETY: name is NUL-terminated and live; the wait is bounded.
    if unsafe { WaitNamedPipeW(name.as_ptr(), RESULT_PIPE_TIMEOUT_MS) } == 0 {
        return Err(format!(
            "wait for result pipe: {}",
            io::Error::last_os_error()
        ));
    }
    // SAFETY: all pointers are null or reference the live terminated name. The checked handle has
    // one RAII owner and requests write-only access to the parent-owned result channel.
    let raw = unsafe {
        CreateFileW(
            name.as_ptr(),
            GENERIC_READ | GENERIC_WRITE,
            0,
            null(),
            OPEN_EXISTING,
            0,
            null_mut(),
        )
    };
    if raw.is_null() || raw == INVALID_HANDLE_VALUE {
        return Err(format!("open result pipe: {}", io::Error::last_os_error()));
    }
    let pipe = OwnedHandle(raw);
    let mut framed = Vec::with_capacity(bytes.len() + 4);
    framed.extend_from_slice(
        &u32::try_from(bytes.len())
            .map_err(|_| "result length overflow")?
            .to_le_bytes(),
    );
    framed.extend_from_slice(bytes);
    let mut offset = 0_usize;
    while offset < framed.len() {
        let remaining = &framed[offset..];
        let mut written = 0_u32;
        // SAFETY: pipe is live, remaining is a valid bounded input slice, written is a live output
        // slot, and WriteFile does not retain either pointer after this synchronous call.
        if unsafe {
            WriteFile(
                pipe.0,
                remaining.as_ptr().cast(),
                u32::try_from(remaining.len()).map_err(|_| "result length overflow")?,
                &mut written,
                null_mut(),
            )
        } == 0
            || written == 0
        {
            return Err(format!("write result pipe: {}", io::Error::last_os_error()));
        }
        offset = offset
            .checked_add(written as usize)
            .ok_or_else(|| "result write offset overflow".to_owned())?;
    }
    let mut acknowledgement = 0_u8;
    let mut acknowledged = 0_u32;
    // SAFETY: pipe is live, acknowledgement is a valid one-byte output slot, acknowledged is a
    // live count, and the synchronous call retains no pointer.
    if unsafe {
        ReadFile(
            pipe.0,
            (&raw mut acknowledgement).cast(),
            1,
            &mut acknowledged,
            null_mut(),
        )
    } == 0
        || acknowledged != 1
        || acknowledgement != 0xA5
    {
        return Err(format!(
            "read result acknowledgement: {}",
            io::Error::last_os_error()
        ));
    }
    Ok(())
}

fn nul_terminated(value: &str) -> Result<Vec<u16>, String> {
    let mut encoded: Vec<u16> = value.encode_utf16().collect();
    if encoded.is_empty() || encoded.len() >= 32_767 || encoded.contains(&0) {
        return Err("Win32 pipe path is invalid".to_owned());
    }
    encoded.push(0);
    Ok(encoded)
}

fn encode_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

struct OwnedHandle(HANDLE);

impl Drop for OwnedHandle {
    fn drop(&mut self) {
        // SAFETY: constructors reject null/invalid handles and this unique owner closes once.
        let _ = unsafe { CloseHandle(self.0) };
    }
}
