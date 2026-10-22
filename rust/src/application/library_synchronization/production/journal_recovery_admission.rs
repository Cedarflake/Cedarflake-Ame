use std::path::PathBuf;

use crate::adapters::SqliteCatalog;
use crate::application::persistent_journal_continuity::can_reprobe_journal_failure;
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
        if !can_reprobe_journal_failure(&checkpoint) {
            continue;
        }
        let Some(root) = catalog.load_incremental_catalog_root(&capability.root_id)? else {
            continue;
        };
        if root.root_generation != capability.root_generation || root.has_running_scan {
            continue;
        }
        // A fresh root/boundary probe must create durable recovery before inventory.
        // Pending recovery excludes subsequent probes of the retained failure.
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
