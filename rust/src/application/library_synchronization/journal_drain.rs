use std::sync::atomic::{AtomicBool, Ordering};

use crate::adapters::SqliteCatalog;
use crate::domain::{
    IncrementalLibraryChangeReport, LibraryChangeLane, LibraryChangeQueuePolicy,
    LibraryRootGeneration, ScanError,
};
use crate::ports::IncrementalCatalogRepository;

use crate::application::authoritative_library_changes::{
    AuthoritativeRecoveryPolicy, process_leased_authoritative_library_change_cancellable,
};
use crate::application::incremental_library_changes::process_ready_library_changes_in_lane_cancellable;

#[cfg(test)]
mod tests;

pub(super) fn process_ready(
    catalog: &mut SqliteCatalog,
    root_id: &str,
    generation: LibraryRootGeneration,
    now_unix_ms: i64,
    policy: LibraryChangeQueuePolicy,
    recovery_policy: AuthoritativeRecoveryPolicy,
    cancelled: &AtomicBool,
) -> Result<IncrementalLibraryChangeReport, ScanError> {
    if cancelled.load(Ordering::Acquire) {
        return Ok(IncrementalLibraryChangeReport::default());
    }
    if catalog.has_ready_journal_path_library_change(root_id, generation, now_unix_ms, policy)? {
        return process_ready_library_changes_in_lane_cancellable(
            catalog,
            root_id,
            generation,
            LibraryChangeLane::Journal,
            now_unix_ms,
            policy,
            cancelled,
        );
    }
    let Some(root) = catalog.load_incremental_catalog_root(root_id)? else {
        return Ok(IncrementalLibraryChangeReport::default());
    };
    let Some(leased) = catalog.lease_journal_authoritative_library_change(
        root_id,
        generation,
        now_unix_ms,
        policy,
    )?
    else {
        return Ok(IncrementalLibraryChangeReport::default());
    };
    process_leased_authoritative_library_change_cancellable(
        catalog,
        &root,
        &leased,
        now_unix_ms,
        policy,
        recovery_policy,
        cancelled,
    )
    .map(|report| report.incremental)
}
