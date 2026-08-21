use super::{
    CatalogFreshnessCause, CatalogFreshnessState, LibraryChangeQueueHealth,
    LibraryChangeSourceHealth, LibraryRootAvailability,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LibrarySynchronizationSnapshot {
    pub is_running: bool,
    pub catalog_revision: u64,
    pub applied_mutation_count: u32,
    pub roots: Vec<LibraryRootSynchronizationStatus>,
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum LibrarySynchronizationPhase {
    WatcherStartup,
    InventoryEnumeration,
    InventoryComparison,
    QueuePublication,
    RetryWait,
    Reconciliation,
    FullScan,
    Blocked,
    Synchronized,
    Unavailable,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LibraryRootSynchronizationStatus {
    pub root_id: String,
    pub root_generation: u64,
    pub availability: LibraryRootAvailability,
    pub freshness: CatalogFreshnessState,
    pub freshness_cause: CatalogFreshnessCause,
    pub phase: LibrarySynchronizationPhase,
    pub source_health: LibraryChangeSourceHealth,
    pub queue_health: LibraryChangeQueueHealth,
    pub pending_change_count: u64,
    pub retry_wait_count: u64,
    pub freshness_unknown_count: u64,
    pub last_issue_code: Option<String>,
}
