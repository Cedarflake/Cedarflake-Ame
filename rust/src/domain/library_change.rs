use std::fmt::{Display, Formatter};

use super::{FileIdentityEvidence, LibraryRootAvailability};

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct LibraryRootGeneration(u64);

impl LibraryRootGeneration {
    pub const fn initial() -> Self {
        Self(1)
    }

    pub const fn new(value: u64) -> Option<Self> {
        if value == 0 { None } else { Some(Self(value)) }
    }

    pub const fn value(self) -> u64 {
        self.0
    }

    pub const fn next(self) -> Option<Self> {
        match self.0.checked_add(1) {
            Some(value) => Some(Self(value)),
            None => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum LibraryChangeOrigin {
    LiveNotification,
    StartupCatchUp,
    UserRefresh,
    ConsistencyAudit,
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum LibraryChangeScope {
    Path,
    Subtree,
    Root,
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum LibraryChangeObservationKind {
    Created,
    Modified,
    Removed,
    Renamed { is_reliably_paired: bool },
    DirectoryChanged,
    EvidenceGap,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LibraryChangeObservation {
    pub root_id: String,
    pub root_generation: LibraryRootGeneration,
    pub sequence: u64,
    pub observed_unix_ms: i64,
    pub kind: LibraryChangeObservationKind,
    pub scope: LibraryChangeScope,
    pub relative_path: String,
    pub previous_relative_path: Option<String>,
    pub origin: LibraryChangeOrigin,
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum LibraryChangeIntentKind {
    Reconcile,
    RenameCandidate,
    FreshnessUnknown,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LibraryChangeIntent {
    pub root_id: String,
    pub root_generation: LibraryRootGeneration,
    pub kind: LibraryChangeIntentKind,
    pub scope: LibraryChangeScope,
    pub relative_path: String,
    pub previous_relative_path: Option<String>,
    pub origin: LibraryChangeOrigin,
    pub first_observed_unix_ms: i64,
    pub most_recent_observed_unix_ms: i64,
    pub first_sequence: u64,
    pub most_recent_sequence: u64,
    pub coalesced_observation_count: u32,
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum LibraryChangeSourceHealth {
    Healthy,
    Starting,
    Degraded,
    Failed,
    Stopped,
    Unsupported,
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum CatalogFreshnessState {
    Synchronized,
    Updating,
    NeedsReconciliation,
    Unavailable,
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum CatalogFreshnessCause {
    NoPendingChanges,
    PendingChanges,
    RootUnavailable,
    ChangeSourceUnhealthy,
    EvidenceGap,
    BoundedCapacityExceeded,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LibraryChangePlanningContext {
    pub root_id: String,
    pub root_generation: LibraryRootGeneration,
    pub availability: LibraryRootAvailability,
    pub source_health: LibraryChangeSourceHealth,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LibraryChangePlanningLimits {
    pub max_observations: usize,
    pub max_intents: usize,
}

impl Default for LibraryChangePlanningLimits {
    fn default() -> Self {
        Self {
            max_observations: 4096,
            max_intents: 1024,
        }
    }
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum LibraryChangePlanningIssue {
    InvalidRelativePath,
    ObservationLimitExceeded,
    IntentLimitExceeded,
    ChangeEvidenceGap,
    ChangeSourceUnhealthy,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LibraryChangePlanningResult {
    pub root_id: String,
    pub root_generation: LibraryRootGeneration,
    pub freshness: CatalogFreshnessState,
    pub freshness_cause: CatalogFreshnessCause,
    pub intents: Vec<LibraryChangeIntent>,
    pub issues: Vec<LibraryChangePlanningIssue>,
    pub received_observation_count: u64,
    pub superseded_observation_count: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LibraryChangePlanningError {
    pub code: String,
    pub message: String,
}

impl LibraryChangePlanningError {
    pub(crate) fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
}

impl Display for LibraryChangePlanningError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for LibraryChangePlanningError {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReconciliationFileEvidence {
    pub relative_path: String,
    pub file_size: u64,
    pub modified_unix_ms: i64,
    pub file_identity: Option<FileIdentityEvidence>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReconciliationObservedState {
    Present(ReconciliationFileEvidence),
    Missing { is_authoritative: bool },
    RetryableFailure { code: String },
    TerminalIssue { code: String },
    Skipped { code: String },
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum IncrementalReconciliationOutcome {
    Unchanged,
    Added,
    Modified,
    RenamedOrMoved,
    Replaced,
    Removed,
    Skipped,
    RetryableFailure,
    TerminalIssue,
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum DerivedEvidenceDisposition {
    RetainCompatible,
    InvalidateDerived,
    NoReusableEvidence,
    RemoveFromCurrentProjection,
    PreserveLastTrustworthy,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IncrementalReconciliationDecision {
    pub outcome: IncrementalReconciliationOutcome,
    pub evidence_disposition: DerivedEvidenceDisposition,
    pub current: Option<ReconciliationFileEvidence>,
    pub issue_code: Option<String>,
}
