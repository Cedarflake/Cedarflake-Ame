use super::RequestContext;
use crate::journal_broker::wire::{
    CandidateKind, MAX_PATH_UTF16_UNITS, NameComparisonSemantics, NormalizedWindowsPath,
    RootAuthorization, RootCapability, normalize_backend_path,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AuthenticatedConnection {
    pub(super) connection_id: [u8; 16],
    pub(super) connection_generation: u64,
    pub(super) opaque_token_binding: [u8; 32],
    pub(super) observed_client_binary_identity: [u8; 32],
    pub(super) observed_process_id: u32,
    pub(super) observed_session_id: u32,
    pub(super) observed_token_type: i32,
    pub(super) observed_impersonation_level: i32,
    pub(super) observed_is_elevated: bool,
}

pub(in crate::journal_broker) struct AuthenticatedPipeFacts {
    pub(in crate::journal_broker) opaque_token_binding: [u8; 32],
    pub(in crate::journal_broker) client_binary_identity: [u8; 32],
    pub(in crate::journal_broker) process_id: u32,
    pub(in crate::journal_broker) session_id: u32,
    pub(in crate::journal_broker) token_type: i32,
    pub(in crate::journal_broker) impersonation_level: i32,
    pub(in crate::journal_broker) is_elevated: bool,
}

impl AuthenticatedConnection {
    pub(in crate::journal_broker) fn from_pipe_token(
        connection_id: [u8; 16],
        connection_generation: u64,
        facts: AuthenticatedPipeFacts,
    ) -> Result<Self, AuthorizerError> {
        if connection_id == [0; 16]
            || connection_generation == 0
            || facts.opaque_token_binding == [0; 32]
            || facts.client_binary_identity == [0; 32]
            || facts.process_id == 0
            || facts.token_type != 2
            || facts.impersonation_level < 2
        {
            return Err(AuthorizerError::CallerRejected);
        }
        Ok(Self {
            connection_id,
            connection_generation,
            opaque_token_binding: facts.opaque_token_binding,
            observed_client_binary_identity: facts.client_binary_identity,
            observed_process_id: facts.process_id,
            observed_session_id: facts.session_id,
            observed_token_type: facts.token_type,
            observed_impersonation_level: facts.impersonation_level,
            observed_is_elevated: facts.is_elevated,
        })
    }
}

#[derive(Clone, Debug)]
pub(crate) struct CallerAuthorization {
    pub(super) connection_id: [u8; 16],
    pub(super) connection_generation: u64,
    pub(super) client_instance: [u8; 16],
    pub(super) opaque_namespace: [u8; 32],
}

impl CallerAuthorization {
    pub(in crate::journal_broker) fn from_verified_claim(
        authenticated: &AuthenticatedConnection,
        claim: &crate::journal_broker::wire::CallerClaim,
    ) -> Result<Self, AuthorizerError> {
        if claim.process_id != authenticated.observed_process_id
            || claim.session_id != authenticated.observed_session_id
            || claim.client_instance == [0; 16]
        {
            return Err(AuthorizerError::CallerRejected);
        }
        Ok(Self {
            connection_id: authenticated.connection_id,
            connection_generation: authenticated.connection_generation,
            client_instance: claim.client_instance,
            opaque_namespace: authenticated.opaque_token_binding,
        })
    }
}

#[derive(Clone, Debug)]
pub(crate) struct DeclaredRootAuthorization {
    pub(super) caller: CallerAuthorization,
    pub(super) root: RootAuthorization,
}

impl DeclaredRootAuthorization {
    pub(in crate::journal_broker) fn after_access_check(
        caller: &CallerAuthorization,
        root: &RootAuthorization,
    ) -> Result<Self, AuthorizerError> {
        root.validate()
            .map_err(|_| AuthorizerError::RootUnauthorized)?;
        Ok(Self {
            caller: caller.clone(),
            root: root.clone(),
        })
    }
}

#[derive(Clone, Debug)]
pub(crate) struct PinnedBrokerRoot {
    pub(super) volume_id: String,
    pub(super) root_identity: Vec<u8>,
    pub(super) canonical_root_utf16: Vec<u16>,
    pub(super) name_semantics: RootNameSemanticsProof,
}

#[derive(Clone, Debug)]
pub(crate) struct RootNameSemanticsProof {
    components: Vec<NameComparisonSemantics>,
}

impl RootNameSemanticsProof {
    pub(super) fn from_pinned_directory_handles(
        components: &[NameComparisonSemantics],
    ) -> Result<Self, AuthorizerError> {
        if components.len() > super::super::wire::MAX_PATH_DEPTH {
            return Err(AuthorizerError::RootIdentityMismatch);
        }
        Ok(Self {
            components: components.to_vec(),
        })
    }

    pub(super) fn components(&self) -> &[NameComparisonSemantics] {
        &self.components
    }
}

impl PinnedBrokerRoot {
    #[cfg_attr(
        not(test),
        allow(
            dead_code,
            reason = "R2c-O must query name semantics from the pinned handle"
        )
    )]
    pub(super) fn from_caller_pinned_handle(
        volume_id: &str,
        root_identity: &[u8],
        canonical_root_utf16: &[u16],
        name_semantics: RootNameSemanticsProof,
    ) -> Result<Self, BackendError> {
        if volume_id.is_empty()
            || volume_id.len() > super::super::wire::MAX_VOLUME_ID_BYTES
            || !matches!(root_identity.len(), 8 | 16)
        {
            return Err(BackendError::RootUnavailable);
        }
        let normalized = normalize_backend_path(canonical_root_utf16)
            .map_err(|_| BackendError::RootUnavailable)?;
        if normalized.component_count() != name_semantics.components().len() {
            return Err(BackendError::RootUnavailable);
        }
        Ok(Self {
            volume_id: volume_id.to_owned(),
            root_identity: root_identity.to_vec(),
            canonical_root_utf16: canonical_root_utf16.to_vec(),
            name_semantics,
        })
    }
}

#[derive(Clone, Debug)]
pub(crate) struct AuthorizedPinnedRoot {
    pub(super) caller: CallerAuthorization,
    pub(super) declared_root: RootAuthorization,
    pub(super) pinned_root: PinnedBrokerRoot,
}

impl AuthorizedPinnedRoot {
    pub(super) fn after_identity_check(
        caller: &CallerAuthorization,
        declared: &DeclaredRootAuthorization,
        pinned_root: PinnedBrokerRoot,
    ) -> Result<Self, AuthorizerError> {
        if caller.connection_id != declared.caller.connection_id
            || caller.client_instance != declared.caller.client_instance
            || caller.opaque_namespace != declared.caller.opaque_namespace
        {
            return Err(AuthorizerError::RootIdentityMismatch);
        }
        Ok(Self {
            caller: caller.clone(),
            declared_root: declared.root.clone(),
            pinned_root,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ExistingJournalMetadata {
    pub(super) journal_id: u64,
    pub(super) first_usn: i64,
    pub(super) next_usn: i64,
}

impl ExistingJournalMetadata {
    #[cfg_attr(
        not(test),
        allow(
            dead_code,
            reason = "R2c-O will construct metadata from FSCTL query output"
        )
    )]
    pub(in crate::journal_broker) fn from_query(
        journal_id: u64,
        first_usn: i64,
        next_usn: i64,
    ) -> Result<Self, BackendError> {
        Self {
            journal_id,
            first_usn,
            next_usn,
        }
        .validate()
    }

    pub(super) fn validate(self) -> Result<Self, BackendError> {
        if self.journal_id == 0 || self.first_usn < 0 || self.next_usn < self.first_usn {
            return Err(BackendError::JournalDiscontinuous);
        }
        Ok(self)
    }

    pub(in crate::journal_broker) fn parts(self) -> (u64, i64, i64) {
        (self.journal_id, self.first_usn, self.next_usn)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(
    not(test),
    allow(
        dead_code,
        reason = "the Windows host remains fail-closed until R2c-O admission"
    )
)]
pub(crate) enum ExistingJournalState {
    Supported(ExistingJournalMetadata),
    LiveOnly,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct JournalRange {
    pub(super) start_usn: i64,
    pub(super) end_usn: i64,
}

impl JournalRange {
    #[cfg_attr(
        not(test),
        allow(
            dead_code,
            reason = "R2c-O backend construction is compile-tested while fail-closed"
        )
    )]
    pub(in crate::journal_broker) fn new(
        start_usn: i64,
        end_usn: i64,
    ) -> Result<Self, BackendError> {
        if start_usn < 0 || end_usn <= start_usn {
            return Err(BackendError::JournalDiscontinuous);
        }
        Ok(Self { start_usn, end_usn })
    }

    pub(in crate::journal_broker) fn bounds(self) -> (i64, i64) {
        (self.start_usn, self.end_usn)
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct JournalReadLimits {
    pub(super) max_records: usize,
    pub(super) max_evidence_bytes: usize,
}

impl JournalReadLimits {
    #[cfg_attr(
        not(test),
        allow(
            dead_code,
            reason = "R2c-O backend construction is compile-tested while fail-closed"
        )
    )]
    pub(in crate::journal_broker) fn new(
        max_records: usize,
        max_evidence_bytes: usize,
    ) -> Result<Self, BackendError> {
        if !(1..=super::super::wire::MAX_RECORDS as usize).contains(&max_records)
            || !(1..=super::super::wire::MAX_EVIDENCE_BYTES as usize).contains(&max_evidence_bytes)
        {
            return Err(BackendError::EvidenceLimitExceeded);
        }
        Ok(Self {
            max_records,
            max_evidence_bytes,
        })
    }

    pub(in crate::journal_broker) fn parts(self) -> (usize, usize) {
        (self.max_records, self.max_evidence_bytes)
    }
}

#[derive(Clone, Debug)]
pub(crate) struct BackendJournalCandidate {
    pub(super) volume_id: String,
    pub(super) absolute_path_utf16: Vec<u16>,
    pub(super) previous_absolute_path_utf16: Option<Vec<u16>>,
    pub(super) file_reference: Vec<u8>,
    pub(super) usn: i64,
    pub(super) kind: CandidateKind,
    pub(super) is_directory: bool,
}

#[cfg_attr(
    not(test),
    allow(
        dead_code,
        reason = "only an admitted R2c-O host may construct journal records"
    )
)]
impl BackendJournalCandidate {
    #[allow(clippy::too_many_arguments)]
    pub(in crate::journal_broker) fn copy_from_bounded(
        volume_id: &str,
        absolute_path_utf16: &[u16],
        previous_absolute_path_utf16: Option<&[u16]>,
        file_reference: &[u8],
        usn: i64,
        kind: CandidateKind,
        is_directory: bool,
    ) -> Result<Self, BackendError> {
        let has_previous = previous_absolute_path_utf16.is_some();
        if volume_id.is_empty()
            || volume_id.len() > super::super::wire::MAX_VOLUME_ID_BYTES
            || absolute_path_utf16.len() > MAX_PATH_UTF16_UNITS
            || previous_absolute_path_utf16.is_some_and(|path| path.len() > MAX_PATH_UTF16_UNITS)
            || !matches!(file_reference.len(), 8 | 16)
            || usn < 0
            || has_previous != matches!(kind, CandidateKind::Rename)
            || (matches!(kind, CandidateKind::Subtree) && !is_directory)
        {
            return Err(BackendError::RecordUnsupported);
        }
        normalize_backend_path(absolute_path_utf16).map_err(|_| BackendError::RecordUnsupported)?;
        if let Some(previous) = previous_absolute_path_utf16 {
            normalize_backend_path(previous).map_err(|_| BackendError::RecordUnsupported)?;
        }
        Ok(Self {
            volume_id: volume_id.to_owned(),
            absolute_path_utf16: absolute_path_utf16.to_vec(),
            previous_absolute_path_utf16: previous_absolute_path_utf16.map(<[u16]>::to_vec),
            file_reference: file_reference.to_vec(),
            usn,
            kind,
            is_directory,
        })
    }

    pub(in crate::journal_broker) fn evidence_bytes(&self) -> Result<usize, BackendError> {
        self.absolute_path_utf16
            .len()
            .checked_mul(2)
            .and_then(|current| {
                self.previous_absolute_path_utf16
                    .as_ref()
                    .map_or(Some(current), |previous| {
                        previous
                            .len()
                            .checked_mul(2)
                            .and_then(|size| current.checked_add(size))
                    })
            })
            .and_then(|size| size.checked_add(self.file_reference.len()))
            .ok_or(BackendError::EvidenceLimitExceeded)
    }

    pub(in crate::journal_broker) fn usn(&self) -> i64 {
        self.usn
    }

    pub(in crate::journal_broker) fn kind(&self) -> CandidateKind {
        self.kind
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct JournalReadProof {
    pub(super) covered_until_usn: i64,
    pub(super) is_complete: bool,
    pub(super) after: ExistingJournalMetadata,
}

impl JournalReadProof {
    #[cfg_attr(
        not(test),
        allow(
            dead_code,
            reason = "R2c-O will construct proof from post-read FSCTL metadata"
        )
    )]
    pub(in crate::journal_broker) fn after_read(
        covered_until_usn: i64,
        is_complete: bool,
        after: ExistingJournalMetadata,
    ) -> Result<Self, BackendError> {
        after.validate()?;
        if covered_until_usn < 0 {
            return Err(BackendError::JournalDiscontinuous);
        }
        Ok(Self {
            covered_until_usn,
            is_complete,
            after,
        })
    }

    pub(in crate::journal_broker) fn parts(self) -> (i64, bool, ExistingJournalMetadata) {
        (self.covered_until_usn, self.is_complete, self.after)
    }
}

#[derive(Clone, Debug)]
pub(crate) struct BoundedJournalPage {
    pub(super) candidates: Vec<BackendJournalCandidate>,
    pub(super) proof: JournalReadProof,
}

#[cfg_attr(
    not(test),
    allow(
        dead_code,
        reason = "only an admitted R2c-O host may populate journal pages"
    )
)]
pub(crate) struct BoundedJournalPageBuilder {
    range: JournalRange,
    limits: JournalReadLimits,
    evidence_bytes: usize,
    candidates: Vec<BackendJournalCandidate>,
}

#[cfg_attr(
    not(test),
    allow(
        dead_code,
        reason = "only an admitted R2c-O host may populate journal pages"
    )
)]
impl BoundedJournalPageBuilder {
    pub(in crate::journal_broker) fn new(range: JournalRange, limits: JournalReadLimits) -> Self {
        Self {
            range,
            limits,
            evidence_bytes: 0,
            candidates: Vec::with_capacity(limits.max_records.min(32)),
        }
    }

    pub(in crate::journal_broker) fn limits(&self) -> (usize, usize) {
        (self.limits.max_records, self.limits.max_evidence_bytes)
    }

    pub(in crate::journal_broker) fn push(
        &mut self,
        candidate: BackendJournalCandidate,
    ) -> Result<(), BackendError> {
        if self.candidates.len() >= self.limits.max_records {
            return Err(BackendError::EvidenceLimitExceeded);
        }
        if candidate.usn < self.range.start_usn || candidate.usn >= self.range.end_usn {
            return Err(BackendError::JournalDiscontinuous);
        }
        if self
            .candidates
            .last()
            .is_some_and(|previous| candidate.usn <= previous.usn)
        {
            return Err(BackendError::JournalDiscontinuous);
        }
        let next_size = self
            .evidence_bytes
            .checked_add(candidate.evidence_bytes()?)
            .ok_or(BackendError::EvidenceLimitExceeded)?;
        if next_size > self.limits.max_evidence_bytes {
            return Err(BackendError::EvidenceLimitExceeded);
        }
        self.evidence_bytes = next_size;
        self.candidates.push(candidate);
        Ok(())
    }

    pub(in crate::journal_broker) fn finish(
        self,
        before: ExistingJournalMetadata,
        proof: JournalReadProof,
    ) -> Result<BoundedJournalPage, BackendError> {
        let after = proof.after.validate()?;
        if after.journal_id != before.journal_id
            || after.first_usn > self.range.start_usn
            || after.next_usn < self.range.end_usn
            || proof.covered_until_usn <= self.range.start_usn
            || proof.covered_until_usn > self.range.end_usn
            || proof.is_complete != (proof.covered_until_usn == self.range.end_usn)
            || self
                .candidates
                .last()
                .is_some_and(|candidate| candidate.usn >= proof.covered_until_usn)
        {
            return Err(BackendError::JournalDiscontinuous);
        }
        Ok(BoundedJournalPage {
            candidates: self.candidates,
            proof,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AuthorizerError {
    CallerRejected,
    RootUnauthorized,
    RootIdentityMismatch,
    UnsupportedFilesystem,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(
    not(test),
    allow(
        dead_code,
        reason = "the fail-closed Windows backend cannot yet produce host errors"
    )
)]
pub(crate) enum BackendError {
    RootUnavailable,
    JournalUnavailable,
    JournalDiscontinuous,
    VolumeMismatch,
    RecordUnsupported,
    EvidenceLimitExceeded,
    TimedOut,
    Cancelled,
    Unavailable,
}

pub(crate) trait CallerAuthorizer: Send + Sync {
    fn reap_expired(&self, _now: std::time::Instant) {}

    fn verify_claim(
        &self,
        authenticated: &AuthenticatedConnection,
        claim: &crate::journal_broker::wire::CallerClaim,
    ) -> Result<CallerAuthorization, AuthorizerError>;

    fn register_root(
        &self,
        caller: &CallerAuthorization,
        root: &RootAuthorization,
        client_root_handle: u64,
        context: &mut RequestContext<'_>,
    ) -> Result<RootCapability, AuthorizerError>;

    fn authorize_registered_root(
        &self,
        caller: &CallerAuthorization,
        root: &RootAuthorization,
        capability: RootCapability,
        context: &mut RequestContext<'_>,
    ) -> Result<AuthorizedPinnedRoot, AuthorizerError>;

    fn filter_disclosable_candidates(
        &self,
        root: &AuthorizedPinnedRoot,
        candidates: Vec<BackendJournalCandidate>,
        context: &mut RequestContext<'_>,
    ) -> Result<Vec<BackendJournalCandidate>, AuthorizerError>;
}

pub(crate) trait ExistingJournalBackend: Send + Sync {
    fn query_existing_journal(
        &self,
        root: &AuthorizedPinnedRoot,
        context: &mut RequestContext<'_>,
    ) -> Result<ExistingJournalState, BackendError>;

    fn read_existing_journal_page(
        &self,
        root: &AuthorizedPinnedRoot,
        journal: ExistingJournalMetadata,
        range: JournalRange,
        page: &mut BoundedJournalPageBuilder,
        context: &mut RequestContext<'_>,
    ) -> Result<JournalReadProof, BackendError>;

    fn read_existing_journal_volume_page(
        &self,
        _request: ExistingJournalVolumeRead<'_>,
        _context: &mut RequestContext<'_>,
    ) -> Result<SharedBackendJournalPage, BackendError> {
        Err(BackendError::Unavailable)
    }
}

pub(crate) struct ExistingJournalVolumeRead<'a> {
    pub(crate) roots: &'a [AuthorizedPinnedRoot],
    pub(crate) root_start_usns: &'a [i64],
    pub(crate) pending_renames: &'a [BackendPendingRenameCarry],
    pub(crate) journal: ExistingJournalMetadata,
    pub(crate) range: JournalRange,
    pub(crate) limits: JournalReadLimits,
}

pub(crate) struct SharedBackendJournalPage {
    pub(crate) candidates_by_root: Vec<Result<Vec<BackendJournalCandidate>, BackendError>>,
    pub(crate) handoffs: Vec<BackendJournalHandoff>,
    pub(crate) pending_renames: Vec<BackendPendingRename>,
    pub(crate) rename_barriers_by_root: Vec<Option<i64>>,
    pub(crate) proof: Option<JournalReadProof>,
}

pub(crate) struct BackendJournalHandoff {
    pub(in crate::journal_broker) previous_root_index: Option<usize>,
    pub(in crate::journal_broker) previous_pending_index: Option<usize>,
    pub(in crate::journal_broker) current_root_index: usize,
    pub(in crate::journal_broker) previous_absolute_path_utf16: Vec<u16>,
    pub(in crate::journal_broker) current_absolute_path_utf16: Vec<u16>,
    pub(in crate::journal_broker) file_reference: Vec<u8>,
    pub(in crate::journal_broker) old_usn: i64,
    pub(in crate::journal_broker) usn: i64,
    pub(in crate::journal_broker) is_directory: bool,
}

#[derive(Clone, Debug)]
pub(crate) struct BackendPendingRenameCarry {
    pub(in crate::journal_broker) root: Option<AuthorizedPinnedRoot>,
    pub(in crate::journal_broker) read_root_index: Option<usize>,
    pub(in crate::journal_broker) file_reference: Vec<u8>,
    pub(in crate::journal_broker) old_usn: i64,
    pub(in crate::journal_broker) previous_relative_path: String,
    pub(in crate::journal_broker) is_directory: bool,
}

pub(crate) struct BackendPendingRename {
    pub(in crate::journal_broker) root_index: usize,
    pub(in crate::journal_broker) previous_absolute_path_utf16: Vec<u16>,
    pub(in crate::journal_broker) file_reference: Vec<u8>,
    pub(in crate::journal_broker) old_usn: i64,
    pub(in crate::journal_broker) is_directory: bool,
}

pub(super) fn normalized_root(
    root: &PinnedBrokerRoot,
) -> Result<NormalizedWindowsPath, BackendError> {
    normalize_backend_path(&root.canonical_root_utf16).map_err(|_| BackendError::RootUnavailable)
}
