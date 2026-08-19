use super::{LibraryChangeCatchUpEvidence, LibraryChangeIntent};

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct LibraryChangeId(u64);

impl LibraryChangeId {
    pub const fn new(value: u64) -> Option<Self> {
        if value == 0 { None } else { Some(Self(value)) }
    }

    pub const fn value(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum LibraryChangeQueueStatus {
    Pending,
    Leased,
    RetryWait,
    Completed,
    Superseded,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LibraryChangeFailure {
    pub code: String,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DurableLibraryChange {
    pub id: LibraryChangeId,
    pub intent: LibraryChangeIntent,
    pub status: LibraryChangeQueueStatus,
    pub ready_unix_ms: i64,
    pub attempt_count: u32,
    pub next_retry_unix_ms: Option<i64>,
    pub lease_generation: u64,
    pub lease_expires_unix_ms: Option<i64>,
    pub last_failure: Option<LibraryChangeFailure>,
    pub catalog_revision_at_enqueue: u64,
    pub catalog_revision_at_success: Option<u64>,
    pub catch_up_source: Option<String>,
    pub catch_up_watermark: Option<String>,
    pub catch_up_lineage: Vec<LibraryChangeCatchUpEvidence>,
    pub superseded_by_change_id: Option<LibraryChangeId>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LeasedLibraryChange {
    pub change: DurableLibraryChange,
    pub lease_generation: u64,
    pub lease_expires_unix_ms: i64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LibraryChangeQueuePolicy {
    pub debounce_millis: u64,
    pub max_unresolved_changes: u32,
    pub max_lease_batch: u32,
    pub lease_duration_millis: u64,
    pub max_attempts: u32,
    pub retry_initial_delay_millis: u64,
    pub retry_maximum_delay_millis: u64,
    pub terminal_retention_millis: u64,
    pub cleanup_batch: u32,
}

impl LibraryChangeQueuePolicy {
    pub const MAX_DEBOUNCE_MILLIS: u64 = 60_000;
    pub const MAX_UNRESOLVED_CHANGES: u32 = 4_096;
    pub const MAX_LEASE_BATCH: u32 = 128;
    pub const MAX_LEASE_DURATION_MILLIS: u64 = 15 * 60 * 1_000;
    pub const MAX_ATTEMPTS: u32 = 32;
    pub const MAX_RETRY_DELAY_MILLIS: u64 = 60 * 60 * 1_000;
    pub const MAX_TERMINAL_RETENTION_MILLIS: u64 = 365 * 24 * 60 * 60 * 1_000;
    pub const MAX_CLEANUP_BATCH: u32 = 1_024;

    pub const fn is_valid(self) -> bool {
        self.debounce_millis <= Self::MAX_DEBOUNCE_MILLIS
            && self.max_unresolved_changes > 0
            && self.max_unresolved_changes <= Self::MAX_UNRESOLVED_CHANGES
            && self.max_lease_batch > 0
            && self.max_lease_batch <= Self::MAX_LEASE_BATCH
            && self.max_lease_batch <= self.max_unresolved_changes
            && self.lease_duration_millis > 0
            && self.lease_duration_millis <= Self::MAX_LEASE_DURATION_MILLIS
            && self.max_attempts > 0
            && self.max_attempts <= Self::MAX_ATTEMPTS
            && self.retry_initial_delay_millis > 0
            && self.retry_initial_delay_millis <= self.retry_maximum_delay_millis
            && self.retry_maximum_delay_millis <= Self::MAX_RETRY_DELAY_MILLIS
            && self.terminal_retention_millis > 0
            && self.terminal_retention_millis <= Self::MAX_TERMINAL_RETENTION_MILLIS
            && self.cleanup_batch > 0
            && self.cleanup_batch <= Self::MAX_CLEANUP_BATCH
    }
}

impl Default for LibraryChangeQueuePolicy {
    fn default() -> Self {
        Self {
            debounce_millis: 500,
            max_unresolved_changes: Self::MAX_UNRESOLVED_CHANGES,
            max_lease_batch: 64,
            lease_duration_millis: 30_000,
            max_attempts: 8,
            retry_initial_delay_millis: 1_000,
            retry_maximum_delay_millis: 5 * 60 * 1_000,
            terminal_retention_millis: 7 * 24 * 60 * 60 * 1_000,
            cleanup_batch: 128,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LibraryChangeEnqueueReport {
    pub inserted_count: u32,
    pub coalesced_count: u32,
    pub superseded_count: u32,
    pub stale_generation_count: u32,
    pub capacity_degraded: bool,
    pub freshness_unknown_enqueued: bool,
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum LibraryChangeLeaseUpdateOutcome {
    Applied,
    Superseded,
    LeaseMismatch,
    Missing,
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum LibraryChangeQueueHealth {
    Idle,
    Healthy,
    Delayed,
    Degraded,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LibraryChangeQueueMetrics {
    pub health: LibraryChangeQueueHealth,
    pub pending_count: u64,
    pub leased_count: u64,
    pub retry_wait_count: u64,
    pub completed_count: u64,
    pub superseded_count: u64,
    pub ready_count: u64,
    pub expired_lease_count: u64,
    pub exhausted_retry_count: u64,
    pub freshness_unknown_count: u64,
    pub oldest_ready_delay_millis: u64,
}
