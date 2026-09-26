use std::process::ExitCode;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use windows_sys::Win32::System::RemoteDesktop::ProcessIdToSessionId;

#[cfg(feature = "broker-acceptance-client")]
mod acceptance;
mod connection;
mod host;
pub(super) mod name;
mod service_host;
pub(in crate::journal_broker) mod service_identity;
pub(super) mod transport;
mod usn;

use super::service::{
    AuthorizedPinnedRoot, BackendError, BoundedJournalPageBuilder, ExistingJournalBackend,
    ExistingJournalMetadata, ExistingJournalState, ExistingJournalVolumeRead, JournalRange,
    JournalReadProof, RequestContext, SharedBackendJournalPage, WindowsBrokerSecurity,
};
use super::wire::PROTOCOL_VERSION;
use super::{BROKER_BINARY_NAME, MAX_FRAME_BYTES};

static NEXT_CLIENT_INSTANCE: AtomicU64 = AtomicU64::new(1);

pub(super) fn production_caller_claim()
-> Result<super::CallerClaim, super::PersistentChangeJournalOperationError> {
    let process_id = std::process::id();
    let mut session_id = 0_u32;
    // SAFETY: session_id is a live output slot and process_id names this process.
    if unsafe { ProcessIdToSessionId(process_id, &mut session_id) } == 0 {
        return Err(super::PersistentChangeJournalOperationError::TransportUnavailable);
    }
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"cedarflake-ame-production-journal-client-v1\0");
    hasher.update(&process_id.to_le_bytes());
    hasher.update(&session_id.to_le_bytes());
    hasher.update(
        &NEXT_CLIENT_INSTANCE
            .fetch_add(1, Ordering::AcqRel)
            .to_le_bytes(),
    );
    hasher.update(
        &SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
            .to_le_bytes(),
    );
    let mut client_instance = [0_u8; 16];
    client_instance.copy_from_slice(&hasher.finalize().as_bytes()[..16]);
    Ok(super::CallerClaim {
        process_id,
        session_id,
        client_instance,
    })
}

pub(super) fn run_broker_process() -> ExitCode {
    let arguments = std::env::args_os().skip(1).collect::<Vec<_>>();
    if arguments.first().map(std::ffi::OsString::as_os_str)
        == Some(std::ffi::OsStr::new("--identity-probe"))
    {
        let Some(probe_arguments) = arguments[1..]
            .iter()
            .map(|argument| argument.clone().into_string().ok())
            .collect::<Option<Vec<_>>>()
        else {
            return ExitCode::FAILURE;
        };
        return service_identity::run_identity_probe_process(&probe_arguments);
    }

    if std::env::args_os().nth(1).as_deref() == Some(std::ffi::OsStr::new("--protocol-info")) {
        println!(
            "binary={BROKER_BINARY_NAME} protocol={PROTOCOL_VERSION} max_frame={MAX_FRAME_BYTES}"
        );
        return ExitCode::SUCCESS;
    }

    if std::env::args_os().nth(1).as_deref() == Some(std::ffi::OsStr::new("--console")) {
        return host::run_disposable_console_host();
    }

    service_host::run_service_dispatcher()
}

#[cfg(feature = "broker-acceptance-client")]
pub(super) fn run_acceptance_client_process() -> ExitCode {
    acceptance::run_process()
}

struct WindowsExistingJournalBackend {
    security: WindowsBrokerSecurity,
}

impl WindowsExistingJournalBackend {
    fn new(security: WindowsBrokerSecurity) -> Self {
        Self { security }
    }
}

impl ExistingJournalBackend for WindowsExistingJournalBackend {
    fn query_existing_journal(
        &self,
        root: &AuthorizedPinnedRoot,
        context: &mut RequestContext<'_>,
    ) -> Result<ExistingJournalState, BackendError> {
        let _ = context.bounded_step()?;
        let root = self
            .security
            .pinned_root(root)
            .map_err(|_| BackendError::RootUnavailable)?;
        usn::query_existing_journal(&root, context).map(ExistingJournalState::Supported)
    }

    fn read_existing_journal_page(
        &self,
        root: &AuthorizedPinnedRoot,
        journal: ExistingJournalMetadata,
        range: JournalRange,
        page: &mut BoundedJournalPageBuilder,
        context: &mut RequestContext<'_>,
    ) -> Result<JournalReadProof, BackendError> {
        let _ = context.bounded_step()?;
        let root = self
            .security
            .pinned_root(root)
            .map_err(|_| BackendError::RootUnavailable)?;
        usn::read_existing_journal_page(&root, journal, range, page, context)
    }

    fn read_existing_journal_volume_page(
        &self,
        request: ExistingJournalVolumeRead<'_>,
        context: &mut RequestContext<'_>,
    ) -> Result<SharedBackendJournalPage, BackendError> {
        let roots = request
            .roots
            .iter()
            .map(|root| {
                self.security
                    .pinned_root(root)
                    .map_err(|_| BackendError::RootUnavailable)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let pending_renames = request
            .pending_renames
            .iter()
            .map(|pending| usn::NativePendingRenameCarry {
                root: pending
                    .root
                    .as_ref()
                    .and_then(|root| self.security.pinned_root(root).ok()),
                read_root_index: pending.read_root_index,
                file_reference: pending.file_reference.clone(),
                old_usn: pending.old_usn,
                previous_relative_path: pending.previous_relative_path.clone(),
                is_directory: pending.is_directory,
            })
            .collect::<Vec<_>>();
        usn::read_existing_journal_volume_page(
            &roots,
            request.root_start_usns,
            &pending_renames,
            request.journal,
            request.range,
            request.limits,
            context,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::journal_broker::service::{
        AuthenticatedConnection, AuthenticatedPipeFacts, BackendJournalCandidate,
        CallerAuthorization, DeclaredRootAuthorization, JournalReadLimits,
    };
    use crate::journal_broker::wire::{CallerClaim, CandidateKind, RootAuthorization};

    #[test]
    fn windows_backend_success_contract_is_constructible_without_public_fields() {
        let authenticated = AuthenticatedConnection::from_pipe_token(
            [1; 16],
            1,
            AuthenticatedPipeFacts {
                opaque_token_binding: [2; 32],
                client_binary_identity: [3; 32],
                process_id: 100,
                session_id: 3,
                token_type: 2,
                impersonation_level: 2,
                is_elevated: false,
            },
        )
        .expect("authenticated connection");
        let claim = CallerClaim {
            process_id: 100,
            session_id: 3,
            client_instance: [4; 16],
        };
        let caller = CallerAuthorization::from_verified_claim(&authenticated, &claim)
            .expect("verified caller");
        let root = RootAuthorization {
            root_id: "root-constructability".to_owned(),
            root_generation: 1,
            volume_id: "volume-constructability".to_owned(),
            root_identity: vec![5; 16],
            canonical_root_utf16: "C:\\BrokerFixture".encode_utf16().collect(),
        };
        let declared =
            DeclaredRootAuthorization::after_access_check(&caller, &root).expect("declared root");
        let _ = declared;
        let metadata = ExistingJournalMetadata::from_query(7, 0, 20).expect("metadata");
        let proof = JournalReadProof::after_read(20, true, metadata).expect("proof");
        let range = JournalRange::new(10, 20).expect("range");
        let mut page =
            BoundedJournalPageBuilder::new(range, JournalReadLimits::new(1, 512).expect("limits"));
        let path: Vec<u16> = "C:\\BrokerFixture\\A.jpg".encode_utf16().collect();
        page.push(
            BackendJournalCandidate::copy_from_bounded(
                &root.volume_id,
                &path,
                None,
                &[6; 8],
                11,
                CandidateKind::Path,
                false,
            )
            .expect("candidate"),
        )
        .expect("bounded push");
        page.finish(metadata, proof).expect("bounded page");
    }
}
