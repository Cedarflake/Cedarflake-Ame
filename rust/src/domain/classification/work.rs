use super::{ClassificationContent, ClassificationRecordId};
use crate::domain::LibraryRootGeneration;
use crate::domain::organization::OrganizationSelection;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClassificationDispatchMode {
    Disabled,
    Automatic,
    Paused,
    Stopped,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClassificationDispatchState {
    pub revision: u64,
    pub epoch: u64,
    pub mode: ClassificationDispatchMode,
    pub run_id: Option<ClassificationRecordId>,
    pub session_id: Option<ClassificationRecordId>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ClassificationDispatchCommand {
    Enable { run_id: ClassificationRecordId },
    ReplaceRun { run_id: ClassificationRecordId },
    Pause,
    Continue,
    Disable,
    Stop,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClassificationWorkPriority {
    Requested,
    NewContent,
    Backlog,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClassificationRetryOutcome {
    Unchanged,
    Queued,
    QueuedWhilePaused,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ClassificationWorkTarget {
    VerifySource(OrganizationSelection),
    AnalyzeContent {
        content: ClassificationContent,
        source: OrganizationSelection,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClassificationWorkLease {
    pub work_id: u64,
    pub lease_id: u64,
    pub epoch: u64,
    pub session_id: ClassificationRecordId,
    pub run_id: ClassificationRecordId,
    pub priority: ClassificationWorkPriority,
    pub target: ClassificationWorkTarget,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClassificationWorkFailure {
    Unavailable,
    Transient,
    Permanent,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClassificationWorkIssue {
    pub kind: ClassificationWorkFailure,
    pub code: String,
    pub observed_unix_ms: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClassificationFailureRecovery {
    SourceAvailability,
    RetryAt(u64),
    ChangedInput,
}

impl ClassificationWorkIssue {
    pub fn recovery(&self, previous_failures: u64) -> ClassificationFailureRecovery {
        use ClassificationFailureRecovery::{ChangedInput, RetryAt, SourceAvailability};
        match self.kind {
            ClassificationWorkFailure::Unavailable => SourceAvailability,
            ClassificationWorkFailure::Transient => match previous_failures {
                0 => RetryAt(self.observed_unix_ms.saturating_add(5_000)),
                1 => RetryAt(self.observed_unix_ms.saturating_add(30_000)),
                _ => ChangedInput,
            },
            ClassificationWorkFailure::Permanent => ChangedInput,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClassificationBacklogPage {
    pub visited_sources: u32,
    pub queued_sources: u32,
    pub has_more: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClassificationRecoveryRoot {
    pub id: String,
    pub generation: LibraryRootGeneration,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClassificationRecoveryCursor {
    pub root: ClassificationRecoveryRoot,
    pub run_id: ClassificationRecordId,
    pub after_failure_id: u64,
    pub through_failure_id: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClassificationRecoveryPage {
    pub released_sources: u32,
    pub resumed_tasks: u32,
    pub next: Option<ClassificationRecoveryCursor>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClassificationVerificationResult {
    Completed,
    Cancelled,
    Superseded,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClassificationAnalysisResult {
    Completed { used_cached_feature: bool },
    Cancelled,
    Superseded,
}
