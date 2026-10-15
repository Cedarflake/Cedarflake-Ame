use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::adapters::SqliteCatalog;
use crate::domain::{
    LibraryChangeSourceHealth, LibraryRootAvailability, LibraryRootGeneration,
    LibrarySynchronizationSnapshot, PersistentJournalCapabilityState, PersistentJournalCheckpoint,
    PersistentJournalContinuityState, ScanError,
};
use crate::ports::{IncrementalCatalogRepository, PersistentJournalRepository};

const MAX_ROOTS_PER_BATCH: usize = 8;

pub(super) struct JournalVolumeWork {
    pub(super) key: String,
    pub(super) roots: Vec<JournalRootWork>,
}

pub(super) struct JournalRootWork {
    pub(super) root_path: PathBuf,
    pub(super) checkpoint: PersistentJournalCheckpoint,
}

pub(super) fn select(
    catalog: &SqliteCatalog,
    snapshot: &LibrarySynchronizationSnapshot,
    cursor: Option<&str>,
) -> Result<Option<JournalVolumeWork>, ScanError> {
    let capabilities = catalog.load_persistent_journal_capabilities()?;
    let roots = catalog.load_incremental_catalog_roots()?;
    let mut volumes = BTreeMap::<String, JournalVolumeWork>::new();
    let mut admitted_counts = BTreeMap::<String, usize>::new();
    for root in roots {
        let Some(status) = snapshot.roots.iter().find(|status| {
            status.root_id == root.root_id
                && status.availability == LibraryRootAvailability::Available
                && status.source_health == LibraryChangeSourceHealth::Healthy
        }) else {
            continue;
        };
        let generation = LibraryRootGeneration::new(status.root_generation).ok_or_else(|| {
            ScanError::new(
                "library_root_generation_invalid",
                "The synchronization root generation is invalid",
            )
        })?;
        if !capabilities.iter().any(|capability| {
            capability.root_id == root.root_id
                && capability.root_generation == generation
                && capability.state == PersistentJournalCapabilityState::Supported
                && matches!(
                    capability.continuity,
                    PersistentJournalContinuityState::Current
                        | PersistentJournalContinuityState::CatchingUp
                )
        }) {
            continue;
        }
        let Some(checkpoint) =
            catalog.load_persistent_journal_checkpoint(&root.root_id, generation)?
        else {
            continue;
        };
        if !matches!(
            checkpoint.continuity,
            PersistentJournalContinuityState::Current
                | PersistentJournalContinuityState::CatchingUp
        ) || checkpoint.failure.is_some()
        {
            continue;
        }
        let base_key = format!(
            "{}|{}|{}",
            checkpoint.volume.volume_guid,
            checkpoint.volume.volume_serial,
            checkpoint.journal_id.value(),
        );
        // String-key rotation stays stable; only admitted roots determine numeric batch capacity.
        let admitted_count = admitted_counts.entry(base_key.clone()).or_default();
        let chunk_index = *admitted_count / MAX_ROOTS_PER_BATCH;
        *admitted_count += 1;
        let key = format!("{base_key}|{chunk_index}");
        let volume = volumes
            .entry(key.clone())
            .or_insert_with(|| JournalVolumeWork {
                key,
                roots: Vec::new(),
            });
        volume.roots.push(JournalRootWork {
            root_path: PathBuf::from(root.root_path),
            checkpoint,
        });
    }
    let Some(key) = super::select_rotated_key(volumes.keys(), cursor) else {
        return Ok(None);
    };
    Ok(volumes.remove(&key))
}

#[cfg(test)]
mod tests;
