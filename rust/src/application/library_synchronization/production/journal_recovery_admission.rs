use std::path::PathBuf;

use crate::adapters::SqliteCatalog;
use crate::domain::{
    LibraryChangeSourceHealth, LibraryRootAvailability, LibrarySynchronizationPhase,
    LibrarySynchronizationSnapshot, PersistentJournalCapabilityState,
    PersistentJournalContinuityState, ScanError,
};
use crate::ports::{IncrementalCatalogRepository, PersistentJournalRepository};

use super::journal_volume::{JournalRootWork, JournalVolumeWork};

pub(super) fn select(
    catalog: &SqliteCatalog,
    snapshot: &LibrarySynchronizationSnapshot,
) -> Result<Option<JournalVolumeWork>, ScanError> {
    for capability in catalog.load_persistent_journal_capabilities()? {
        if capability.state != PersistentJournalCapabilityState::Supported
            || capability.continuity != PersistentJournalContinuityState::RecoveryRequired
        {
            continue;
        }
        if !snapshot.roots.iter().any(|root| {
            root.root_id == capability.root_id
                && root.root_generation == capability.root_generation.value()
                && root.availability == LibraryRootAvailability::Available
                && root.source_health == LibraryChangeSourceHealth::Healthy
                && root.phase != LibrarySynchronizationPhase::FullScan
                && !root.recovery_blocked
                && root.pending_change_count == 0
                && root.retry_wait_count == 0
                && root.freshness_unknown_count == 0
        }) {
            continue;
        }
        let Some(checkpoint) = catalog
            .load_persistent_journal_checkpoint(&capability.root_id, capability.root_generation)?
        else {
            continue;
        };
        if checkpoint.continuity != PersistentJournalContinuityState::RecoveryRequired
            || checkpoint
                .failure
                .as_ref()
                .is_none_or(|failure| failure.code != "identity_journal_hardlinks_not_supported")
        {
            continue;
        }
        let Some(root) = catalog.load_incremental_catalog_root(&capability.root_id)? else {
            continue;
        };
        if root.root_generation != capability.root_generation || root.has_running_scan {
            continue;
        }
        // Older versions retained the failed checkpoint without enrolling recovery. Probe one
        // root in the existing journal worker; its fresh boundary and durable recovery task
        // precede inventory, and pending recovery excludes subsequent probes.
        return Ok(Some(JournalVolumeWork {
            key: format!("recovery|{}", capability.root_id),
            roots: vec![JournalRootWork {
                root_path: PathBuf::from(root.root_path),
                checkpoint,
            }],
        }));
    }
    Ok(None)
}
