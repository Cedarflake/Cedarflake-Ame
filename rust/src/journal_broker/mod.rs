#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "R2c-N compiles the sealed client seam before R2c-O connects the Windows adapter"
    )
)]
mod client;
mod framing;
#[cfg(any(windows, test))]
mod service;
#[cfg(test)]
pub(crate) use service::integration_test_support::target_translation_recovery_fixture;
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "R2c-N request codec entrypoints remain internal until the Windows host is connected"
    )
)]
mod wire;

#[cfg(windows)]
mod windows;

pub use framing::{BrokerFrameError, MAX_FRAME_BYTES};
pub use wire::{
    BrokerCandidate, BrokerFailure, BrokerFailureCode, BrokerResponse, CallerClaim, CandidateKind,
    CandidateScope, JournalCapability, PROTOCOL_VERSION, QueryJournalRequest,
    ReadJournalRangeRequest, ReadJournalVolumeRequest, RegisterRootRequest, ResponseBinding,
    RootAuthorization, RootCapability, SharedJournalHandoff, SharedJournalPendingRename,
    SharedJournalPendingRenameRequest, SharedJournalRootOutcome, SharedJournalRootRequest,
};

pub const BROKER_BINARY_NAME: &str = "cedarflake_ame_journal_broker";
pub const BROKER_EXECUTABLE_NAME: &str = "cedarflake_ame_journal_broker.exe";

pub(crate) trait PersistentChangeJournal: Send + Sync {
    fn connect(&self) -> PersistentChangeJournalConnection;
}

#[allow(
    dead_code,
    reason = "R2c-O admits query and bounded-read operations before R2c-P checkpoint wiring"
)]
pub(crate) trait PersistentChangeJournalSession: Send + Sync {
    fn register_root(
        &self,
        request: RegisterRootRequest,
    ) -> Result<RootCapability, PersistentChangeJournalOperationError>;

    fn query_journal(
        &self,
        request: QueryJournalRequest,
    ) -> Result<BrokerResponse, PersistentChangeJournalOperationError>;

    fn begin_read_range(
        &self,
        request: ReadJournalRangeRequest,
    ) -> Result<Box<dyn PersistentChangeJournalRead>, PersistentChangeJournalOperationError>;

    fn read_range(
        &self,
        request: ReadJournalRangeRequest,
    ) -> Result<BrokerResponse, PersistentChangeJournalOperationError> {
        self.begin_read_range(request)?.wait()
    }

    fn begin_read_volume(
        &self,
        request: ReadJournalVolumeRequest,
    ) -> Result<Box<dyn PersistentChangeJournalRead>, PersistentChangeJournalOperationError>;

    fn read_volume(
        &self,
        request: ReadJournalVolumeRequest,
    ) -> Result<BrokerResponse, PersistentChangeJournalOperationError> {
        self.begin_read_volume(request)?.wait()
    }

    fn close(&self) -> Result<(), PersistentChangeJournalOperationError>;
}

#[allow(
    dead_code,
    reason = "R2c-O admits cancellable pending reads before R2c-P checkpoint wiring"
)]
pub(crate) trait PersistentChangeJournalRead: Send {
    fn cancel(&self, caller: CallerClaim) -> Result<(), PersistentChangeJournalOperationError>;

    fn wait(self: Box<Self>) -> Result<BrokerResponse, PersistentChangeJournalOperationError>;
}

#[allow(
    dead_code,
    reason = "R2c-O retains the exact LiveOnly cause before R2c-Q product-state projection"
)]
#[derive(Clone)]
pub(crate) enum PersistentChangeJournalConnection {
    Connected(std::sync::Arc<dyn PersistentChangeJournalSession>),
    LiveOnly(PersistentChangeJournalLiveOnlyReason),
}

impl PersistentChangeJournalConnection {
    pub(crate) fn close(&self) -> Result<(), PersistentChangeJournalOperationError> {
        match self {
            Self::Connected(session) => session.close(),
            Self::LiveOnly(_) => Ok(()),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(
    dead_code,
    reason = "R2c-O admits the persistent journal port before R2c-P wires catalog checkpoints"
)]
pub(crate) enum PersistentChangeJournalLiveOnlyReason {
    PortableDistribution,
    BrokerAbsent,
    ProtocolMismatch,
    TransportUnavailable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub(crate) enum PersistentChangeJournalOperationError {
    #[error("persistent change journal session is closed")]
    Closed,
    #[error("persistent change journal session close is already in progress")]
    CloseInProgress,
    #[error("persistent change journal request is invalid")]
    InvalidRequest,
    #[error("persistent change journal request capacity is exhausted")]
    CapacityExceeded,
    #[error("persistent change journal protocol is incompatible")]
    ProtocolMismatch,
    #[error("persistent change journal request was cancelled")]
    Cancelled,
    #[error("persistent change journal request is no longer active")]
    RequestNotActive,
    #[error("persistent change journal response failed integrity validation")]
    ResponseIntegrity,
    #[error("persistent change journal transport is unavailable")]
    TransportUnavailable,
    #[error("persistent change journal broker rejected the request: {0:?}")]
    Remote(BrokerFailureCode),
}

#[cfg(windows)]
#[allow(
    dead_code,
    reason = "R2c-O admits the production adapter before R2c-P wires catalog checkpoints"
)]
pub(crate) fn production_persistent_change_journal() -> std::sync::Arc<dyn PersistentChangeJournal>
{
    std::sync::Arc::new(windows::transport::WindowsPersistentChangeJournal)
}

#[cfg(windows)]
pub(crate) fn production_client_allows_broker_activation() -> bool {
    windows::service_identity::current_process_has_installed_client_identity()
}

#[cfg(windows)]
pub(crate) struct ProductionPersistentJournalRoot {
    pub(crate) authorization: RootAuthorization,
    pub(crate) client_root_handle: u64,
    pub(crate) volume_serial: u64,
    _handle: service::OwnedFileHandle,
}

#[cfg(windows)]
pub(crate) fn describe_production_persistent_journal_root(
    root_id: &str,
    root_generation: u64,
    root_path: &std::path::Path,
) -> Result<ProductionPersistentJournalRoot, PersistentChangeJournalOperationError> {
    let (authorization, handle, volume_serial) =
        service::describe_client_root(root_path, root_id, root_generation)
            .map_err(|_| PersistentChangeJournalOperationError::InvalidRequest)?;
    let client_root_handle = handle.raw() as usize as u64;
    Ok(ProductionPersistentJournalRoot {
        authorization,
        client_root_handle,
        volume_serial,
        _handle: handle,
    })
}

#[cfg(windows)]
pub(crate) fn production_persistent_journal_caller_claim()
-> Result<CallerClaim, PersistentChangeJournalOperationError> {
    windows::production_caller_claim()
}

pub fn run_broker_process() -> std::process::ExitCode {
    #[cfg(windows)]
    {
        windows::run_broker_process()
    }
    #[cfg(not(windows))]
    {
        eprintln!("journal_broker_platform_unsupported");
        std::process::ExitCode::FAILURE
    }
}

#[cfg(all(windows, feature = "broker-acceptance-client"))]
pub fn run_broker_acceptance_client_process() -> std::process::ExitCode {
    windows::run_acceptance_client_process()
}

#[cfg(test)]
mod tests;
