mod codec;
mod path;

pub(crate) use codec::{
    decode_request, decode_response, encode_request, encode_response, inspect_request_header,
};
pub(crate) use path::{
    NameComparisonSemantics, NormalizedWindowsPath, RootRelativeScope, normalize_backend_path,
    scope_under_root, validate_root_path,
};

pub const PROTOCOL_VERSION: u16 = 5;
pub(crate) const PROTOCOL_MAGIC: [u8; 8] = *b"AMEJNL5\0";
pub(crate) const LEGACY_PROTOCOL_MAGIC_V1: [u8; 8] = *b"AMEJNL1\0";
pub(crate) const LEGACY_PROTOCOL_MAGIC_V2: [u8; 8] = *b"AMEJNL2\0";
pub(crate) const LEGACY_PROTOCOL_MAGIC_V3: [u8; 8] = *b"AMEJNL3\0";
pub(crate) const LEGACY_PROTOCOL_MAGIC_V4: [u8; 8] = *b"AMEJNL4\0";
pub(crate) const MAX_ROOT_ID_BYTES: usize = 256;
pub(crate) const MAX_VOLUME_ID_BYTES: usize = 512;
pub(crate) const MAX_PATH_UTF16_UNITS: usize = 32_767;
pub(crate) const MAX_PATH_DEPTH: usize = 256;
pub(crate) const MAX_RECORDS: u32 = 1_024;
pub(crate) const MAX_EVIDENCE_BYTES: u32 = 512 * 1_024;
pub(crate) const MAX_TIMEOUT_MS: u32 = 10_000;
pub(crate) const MAX_SHARED_ROOTS: usize = 8;
pub(crate) const MAX_SHARED_HANDOFFS: usize = 1_024;
pub(crate) const MAX_SHARED_PENDING_RENAMES: usize = 1_024;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CallerClaim {
    pub process_id: u32,
    pub session_id: u32,
    pub client_instance: [u8; 16],
}

impl CallerClaim {
    pub(crate) fn validate(&self) -> Result<(), WireError> {
        if self.process_id == 0 || self.client_instance.iter().all(|byte| *byte == 0) {
            return Err(WireError::InvalidValue("caller_claim"));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RootAuthorization {
    pub root_id: String,
    pub root_generation: u64,
    pub volume_id: String,
    pub root_identity: Vec<u8>,
    pub canonical_root_utf16: Vec<u16>,
}

impl RootAuthorization {
    pub(crate) fn validate(&self) -> Result<(), WireError> {
        validate_identifier(&self.root_id, MAX_ROOT_ID_BYTES)?;
        validate_identifier(&self.volume_id, MAX_VOLUME_ID_BYTES)?;
        if self.root_generation == 0 || !matches!(self.root_identity.len(), 8 | 16) {
            return Err(WireError::InvalidValue("root_authorization"));
        }
        if validate_root_path(&self.canonical_root_utf16)?.component_count() == 0 {
            return Err(WireError::InvalidValue("volume_root"));
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RootCapability(pub [u8; 32]);

impl RootCapability {
    pub(crate) fn validate(self) -> Result<(), WireError> {
        if self.0 == [0; 32] {
            return Err(WireError::InvalidValue("root_capability"));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RegisterRootRequest {
    pub caller: CallerClaim,
    pub root: RootAuthorization,
    pub client_root_handle: u64,
    pub timeout_ms: u32,
}

impl RegisterRootRequest {
    pub(crate) fn validate(&self) -> Result<(), WireError> {
        self.caller.validate()?;
        self.root.validate()?;
        if self.client_root_handle == 0 || self.root.root_identity.len() != 16 {
            return Err(WireError::InvalidValue("root_registration"));
        }
        validate_timeout(self.timeout_ms)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QueryJournalRequest {
    pub caller: CallerClaim,
    pub root: RootAuthorization,
    pub root_capability: RootCapability,
    pub timeout_ms: u32,
}

impl QueryJournalRequest {
    pub(crate) fn validate(&self) -> Result<(), WireError> {
        self.caller.validate()?;
        self.root.validate()?;
        self.root_capability.validate()?;
        validate_timeout(self.timeout_ms)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReadJournalRangeRequest {
    pub caller: CallerClaim,
    pub root: RootAuthorization,
    pub root_capability: RootCapability,
    pub journal_id: u64,
    pub start_usn: i64,
    pub end_usn: i64,
    pub max_records: u32,
    pub max_evidence_bytes: u32,
    pub timeout_ms: u32,
}

impl ReadJournalRangeRequest {
    pub(crate) fn validate(&self) -> Result<(), WireError> {
        self.caller.validate()?;
        self.root.validate()?;
        self.root_capability.validate()?;
        if self.journal_id == 0 || self.start_usn < 0 || self.end_usn <= self.start_usn {
            return Err(WireError::InvalidValue("journal_range"));
        }
        if !(1..=MAX_RECORDS).contains(&self.max_records) {
            return Err(WireError::InvalidValue("max_records"));
        }
        if !(1..=MAX_EVIDENCE_BYTES).contains(&self.max_evidence_bytes) {
            return Err(WireError::InvalidValue("max_evidence_bytes"));
        }
        validate_timeout(self.timeout_ms)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SharedJournalRootRequest {
    pub root: RootAuthorization,
    pub root_capability: RootCapability,
    pub start_usn: i64,
}

impl SharedJournalRootRequest {
    pub(crate) fn validate(&self) -> Result<(), WireError> {
        self.root.validate()?;
        self.root_capability.validate()?;
        if self.start_usn < 0 {
            return Err(WireError::InvalidValue("journal_start"));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReadJournalVolumeRequest {
    pub caller: CallerClaim,
    pub roots: Vec<SharedJournalRootRequest>,
    pub pending_renames: Vec<SharedJournalPendingRenameRequest>,
    pub journal_id: u64,
    pub end_usn: i64,
    pub max_records: u32,
    pub max_evidence_bytes: u32,
    pub timeout_ms: u32,
}

impl ReadJournalVolumeRequest {
    pub(crate) fn validate(&self) -> Result<(), WireError> {
        self.caller.validate()?;
        if self.journal_id == 0
            || !(1..=MAX_SHARED_ROOTS).contains(&self.roots.len())
            || !(1..=MAX_RECORDS).contains(&self.max_records)
            || !(1..=MAX_EVIDENCE_BYTES).contains(&self.max_evidence_bytes)
        {
            return Err(WireError::InvalidValue("shared_journal_range"));
        }
        let volume_id = &self.roots[0].root.volume_id;
        let mut identities = std::collections::BTreeSet::new();
        for root in &self.roots {
            root.validate()?;
            if &root.root.volume_id != volume_id
                || root.start_usn >= self.end_usn
                || !identities.insert((root.root.root_id.as_str(), root.root.root_generation))
            {
                return Err(WireError::InvalidValue("shared_root_set"));
            }
        }
        if self.pending_renames.len() > MAX_SHARED_PENDING_RENAMES {
            return Err(WireError::InvalidValue("shared_pending_rename_count"));
        }
        let mut carry_ids = std::collections::BTreeSet::new();
        for pending in &self.pending_renames {
            pending.validate(&self.caller, volume_id, self.journal_id, self.end_usn)?;
            if !carry_ids.insert(pending.carry_id.as_str()) {
                return Err(WireError::InvalidValue("shared_pending_rename_set"));
            }
        }
        validate_timeout(self.timeout_ms)
    }

    pub(crate) fn minimum_start_usn(&self) -> i64 {
        self.roots
            .iter()
            .map(|root| root.start_usn)
            .min()
            .expect("validated shared request has roots")
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SharedJournalPendingRenameRequest {
    pub carry_id: String,
    pub source_range_id: String,
    pub root: RootAuthorization,
    pub root_capability: Option<RootCapability>,
    pub file_reference: Vec<u8>,
    pub old_usn: i64,
    pub previous_relative_path: String,
    pub is_directory: bool,
}

impl SharedJournalPendingRenameRequest {
    pub(crate) fn validate(
        &self,
        _caller: &CallerClaim,
        volume_id: &str,
        journal_id: u64,
        requested_end_usn: i64,
    ) -> Result<(), WireError> {
        validate_identifier(&self.carry_id, 512)?;
        validate_identifier(&self.source_range_id, 512)?;
        self.root.validate()?;
        if let Some(capability) = self.root_capability {
            capability.validate()?;
        }
        path::validate_relative_path(&self.previous_relative_path)?;
        if self.root.volume_id != volume_id
            || journal_id == 0
            || !matches!(self.file_reference.len(), 8 | 16)
            || self.old_usn < 0
            || self.old_usn >= requested_end_usn
        {
            return Err(WireError::InvalidValue("shared_pending_rename"));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BrokerRequest {
    RegisterRoot {
        request_id: u64,
        request: RegisterRootRequest,
    },
    QueryJournal {
        request_id: u64,
        request: QueryJournalRequest,
    },
    ReadRange {
        request_id: u64,
        request: ReadJournalRangeRequest,
    },
    ReadVolume {
        request_id: u64,
        request: ReadJournalVolumeRequest,
    },
    Cancel {
        request_id: u64,
        caller: CallerClaim,
        target_request_id: u64,
    },
}

impl BrokerRequest {
    pub fn request_id(&self) -> u64 {
        match self {
            Self::RegisterRoot { request_id, .. }
            | Self::QueryJournal { request_id, .. }
            | Self::ReadRange { request_id, .. }
            | Self::ReadVolume { request_id, .. }
            | Self::Cancel { request_id, .. } => *request_id,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResponseBinding {
    pub client_instance: [u8; 16],
    pub root_id: String,
    pub root_generation: u64,
    pub volume_id: String,
}

impl ResponseBinding {
    pub(crate) fn from_request(caller: &CallerClaim, root: &RootAuthorization) -> Self {
        Self {
            client_instance: caller.client_instance,
            root_id: root.root_id.clone(),
            root_generation: root.root_generation,
            volume_id: root.volume_id.clone(),
        }
    }

    pub(crate) fn validate(&self) -> Result<(), WireError> {
        if self.client_instance.iter().all(|byte| *byte == 0) || self.root_generation == 0 {
            return Err(WireError::InvalidValue("response_binding"));
        }
        validate_identifier(&self.root_id, MAX_ROOT_ID_BYTES)?;
        validate_identifier(&self.volume_id, MAX_VOLUME_ID_BYTES)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JournalCapability {
    Supported,
    LiveOnly,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CandidateKind {
    Path,
    Subtree,
    Rename,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CandidateScope {
    Root,
    RelativePath(String),
}

impl CandidateScope {
    pub(crate) fn validate(&self) -> Result<(), WireError> {
        match self {
            Self::Root => Ok(()),
            Self::RelativePath(path) => path::validate_relative_path(path),
        }
    }

    pub(crate) fn evidence_bytes(&self) -> Option<usize> {
        match self {
            Self::Root => Some(1),
            Self::RelativePath(path) => path.encode_utf16().count().checked_mul(2),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrokerCandidate {
    pub scope: CandidateScope,
    pub previous_scope: Option<CandidateScope>,
    pub file_reference: Vec<u8>,
    pub usn: i64,
    pub kind: CandidateKind,
    pub is_directory: bool,
}

impl BrokerCandidate {
    pub(crate) fn validate(&self) -> Result<(), WireError> {
        self.scope.validate()?;
        if let Some(previous) = &self.previous_scope {
            previous.validate()?;
            if matches!(previous, CandidateScope::Root) {
                return Err(WireError::InvalidValue("previous_root_candidate"));
            }
        }
        let is_rename = matches!(self.kind, CandidateKind::Rename);
        if !matches!(self.file_reference.len(), 8 | 16)
            || self.usn < 0
            || self.previous_scope.is_some() != is_rename
            || (matches!(self.kind, CandidateKind::Subtree) && !self.is_directory)
            || (matches!(self.scope, CandidateScope::Root) && !self.is_directory)
        {
            return Err(WireError::InvalidValue("candidate"));
        }
        if matches!(self.scope, CandidateScope::Root) && self.kind != CandidateKind::Subtree {
            return Err(WireError::InvalidValue("root_candidate"));
        }
        Ok(())
    }

    pub(crate) fn evidence_bytes(&self) -> Option<usize> {
        let previous_bytes = match &self.previous_scope {
            Some(scope) => scope.evidence_bytes()?,
            None => 0,
        };
        self.scope
            .evidence_bytes()?
            .checked_add(previous_bytes)?
            .checked_add(self.file_reference.len())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrokerFailureCode {
    ProtocolMismatch,
    MalformedFrame,
    RequestTooLarge,
    CallerRejected,
    RootUnauthorized,
    RootIdentityMismatch,
    VolumeMismatch,
    JournalUnavailable,
    JournalDiscontinuous,
    RecordUnsupported,
    EvidenceLimitExceeded,
    TimedOut,
    Cancelled,
    RequestNotActive,
    ActiveLimitExceeded,
    TerminalDeliveryBackpressure,
    BackendUnavailable,
    Internal,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BrokerFailure {
    pub code: BrokerFailureCode,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SharedJournalRootOutcome {
    pub binding: ResponseBinding,
    pub requested_start_usn: i64,
    pub covered_until_usn: Option<i64>,
    pub is_complete: bool,
    pub candidates: Vec<BrokerCandidate>,
    pub failure: Option<BrokerFailure>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SharedJournalHandoff {
    pub previous_binding: ResponseBinding,
    pub current_binding: ResponseBinding,
    pub file_reference: Vec<u8>,
    pub usn: i64,
    pub previous_relative_path: String,
    pub current_relative_path: String,
    pub is_directory: bool,
    pub previous_carry_id: Option<String>,
}

impl SharedJournalHandoff {
    pub(crate) fn validate(
        &self,
        client_instance: [u8; 16],
        volume_id: &str,
        requested_end_usn: i64,
    ) -> Result<(), WireError> {
        self.previous_binding.validate()?;
        self.current_binding.validate()?;
        path::validate_relative_path(&self.previous_relative_path)?;
        path::validate_relative_path(&self.current_relative_path)?;
        if let Some(carry_id) = &self.previous_carry_id {
            validate_identifier(carry_id, 512)?;
        }
        if self.previous_binding.client_instance != client_instance
            || self.current_binding.client_instance != client_instance
            || self.previous_binding.volume_id != volume_id
            || self.current_binding.volume_id != volume_id
            || (
                self.previous_binding.root_id.as_str(),
                self.previous_binding.root_generation,
            ) == (
                self.current_binding.root_id.as_str(),
                self.current_binding.root_generation,
            )
            || !matches!(self.file_reference.len(), 8 | 16)
            || self.usn < 0
            || self.usn >= requested_end_usn
        {
            return Err(WireError::InvalidValue("shared_handoff"));
        }
        Ok(())
    }

    pub(crate) fn evidence_bytes(&self) -> Option<usize> {
        self.previous_relative_path
            .encode_utf16()
            .count()
            .checked_mul(2)?
            .checked_add(
                self.current_relative_path
                    .encode_utf16()
                    .count()
                    .checked_mul(2)?,
            )?
            .checked_add(self.file_reference.len())
            .and_then(|size| {
                self.previous_carry_id
                    .as_ref()
                    .map_or(Some(size), |carry_id| size.checked_add(carry_id.len()))
            })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SharedJournalPendingRename {
    pub binding: ResponseBinding,
    pub file_reference: Vec<u8>,
    pub old_usn: i64,
    pub previous_relative_path: String,
    pub is_directory: bool,
}

impl SharedJournalPendingRename {
    pub(crate) fn validate(
        &self,
        client_instance: [u8; 16],
        volume_id: &str,
        requested_end_usn: i64,
    ) -> Result<(), WireError> {
        self.binding.validate()?;
        path::validate_relative_path(&self.previous_relative_path)?;
        if self.binding.client_instance != client_instance
            || self.binding.volume_id != volume_id
            || !matches!(self.file_reference.len(), 8 | 16)
            || self.old_usn < 0
            || self.old_usn >= requested_end_usn
        {
            return Err(WireError::InvalidValue("shared_pending_rename"));
        }
        Ok(())
    }

    pub(crate) fn evidence_bytes(&self) -> Option<usize> {
        self.previous_relative_path
            .encode_utf16()
            .count()
            .checked_mul(2)?
            .checked_add(self.file_reference.len())
    }
}

impl SharedJournalRootOutcome {
    pub(crate) fn validate(
        &self,
        requested_end_usn: i64,
        maximum_records: usize,
    ) -> Result<(), WireError> {
        self.binding.validate()?;
        if self.requested_start_usn < 0 || self.requested_start_usn >= requested_end_usn {
            return Err(WireError::InvalidValue("shared_root_range"));
        }
        match (self.covered_until_usn, self.failure) {
            (Some(covered), None) => {
                if covered <= self.requested_start_usn
                    || covered > requested_end_usn
                    || self.is_complete != (covered == requested_end_usn)
                    || self.candidates.len() > maximum_records
                {
                    return Err(WireError::InvalidValue("shared_root_proof"));
                }
                let mut last_usn = None;
                for candidate in &self.candidates {
                    candidate.validate()?;
                    if candidate.usn < self.requested_start_usn
                        || candidate.usn >= covered
                        || last_usn.is_some_and(|previous| candidate.usn <= previous)
                    {
                        return Err(WireError::InvalidValue("candidate_usn"));
                    }
                    last_usn = Some(candidate.usn);
                }
            }
            (None, Some(_)) if !self.is_complete && self.candidates.is_empty() => {}
            _ => return Err(WireError::InvalidValue("shared_root_outcome")),
        }
        Ok(())
    }
}

impl BrokerFailure {
    pub(crate) const fn new(code: BrokerFailureCode) -> Self {
        Self { code }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BrokerResponse {
    RootRegistered {
        request_id: u64,
        binding: ResponseBinding,
        root_capability: RootCapability,
    },
    Journal {
        request_id: u64,
        binding: ResponseBinding,
        capability: JournalCapability,
        journal_id: Option<u64>,
        first_usn: Option<i64>,
        next_usn: Option<i64>,
    },
    ReadRangeAccepted {
        request_id: u64,
        binding: ResponseBinding,
        journal_id: u64,
        requested_start_usn: i64,
        requested_end_usn: i64,
        max_records: u32,
        max_evidence_bytes: u32,
    },
    ReadVolumeAccepted {
        request_id: u64,
        client_instance: [u8; 16],
        volume_id: String,
        journal_id: u64,
        requested_start_usn: i64,
        requested_end_usn: i64,
        root_count: u8,
        max_records: u32,
        max_evidence_bytes: u32,
    },
    ReadVolume {
        request_id: u64,
        client_instance: [u8; 16],
        volume_id: String,
        journal_id: u64,
        requested_end_usn: i64,
        max_records: u32,
        max_evidence_bytes: u32,
        outcomes: Vec<SharedJournalRootOutcome>,
        handoffs: Vec<SharedJournalHandoff>,
        pending_renames: Vec<SharedJournalPendingRename>,
    },
    ReadRange {
        request_id: u64,
        binding: ResponseBinding,
        journal_id: u64,
        requested_start_usn: i64,
        requested_end_usn: i64,
        max_records: u32,
        max_evidence_bytes: u32,
        covered_until_usn: i64,
        is_complete: bool,
        candidates: Vec<BrokerCandidate>,
    },
    Cancelled {
        request_id: u64,
        client_instance: [u8; 16],
        target_request_id: u64,
    },
    Failure {
        request_id: u64,
        client_instance: Option<[u8; 16]>,
        failure: BrokerFailure,
    },
}

impl BrokerResponse {
    pub fn request_id(&self) -> u64 {
        match self {
            Self::RootRegistered { request_id, .. }
            | Self::Journal { request_id, .. }
            | Self::ReadRangeAccepted { request_id, .. }
            | Self::ReadRange { request_id, .. }
            | Self::ReadVolumeAccepted { request_id, .. }
            | Self::ReadVolume { request_id, .. }
            | Self::Cancelled { request_id, .. }
            | Self::Failure { request_id, .. } => *request_id,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct InspectedRequestHeader {
    pub request_id: u64,
    pub version: u16,
    pub is_current_magic: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum WireError {
    InvalidMagic,
    UnsupportedVersion(u16),
    UnknownKind(u16),
    UnexpectedEnd,
    TrailingBytes,
    LengthExceeded,
    LengthOverflow,
    InvalidUtf8,
    InvalidUtf16,
    InvalidValue(&'static str),
    NameComparisonUnavailable,
}

fn validate_identifier(value: &str, maximum: usize) -> Result<(), WireError> {
    if value.is_empty() || value.len() > maximum || value.contains('\0') {
        return Err(WireError::InvalidValue("identifier"));
    }
    Ok(())
}

fn validate_timeout(timeout_ms: u32) -> Result<(), WireError> {
    if !(1..=MAX_TIMEOUT_MS).contains(&timeout_ms) {
        return Err(WireError::InvalidValue("timeout_ms"));
    }
    Ok(())
}

impl std::fmt::Display for WireError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for WireError {}
