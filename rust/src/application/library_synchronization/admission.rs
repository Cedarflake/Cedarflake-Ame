use crate::adapters::SqliteCatalog;
use crate::domain::{
    CatalogFreshnessCause, CatalogFreshnessState, IncrementalCatalogRoot, LibraryChangeQueueHealth,
    LibraryChangeSourceHealth, LibraryRootAvailability, LibraryRootSynchronizationStatus,
    LibrarySynchronizationPhase, LibrarySynchronizationSnapshot, PersistentJournalContinuityState,
    ScanError,
};
use crate::ports::{CatalogRepository, IncrementalCatalogRepository};

use crate::application::scan_library::first_import_capture_lease;

enum RootAdmission {
    PublishedBaseline,
    CapturingFirstImport,
    BaselineRequired,
}

pub(super) struct SynchronizationAdmissions {
    roots: Vec<(IncrementalCatalogRoot, RootAdmission)>,
}

impl SynchronizationAdmissions {
    pub(super) fn load(catalog: &SqliteCatalog) -> Result<Self, ScanError> {
        Self::classify_roots(catalog, catalog.load_incremental_catalog_roots()?)
    }

    fn classify_roots(
        catalog: &SqliteCatalog,
        observed: Vec<IncrementalCatalogRoot>,
    ) -> Result<Self, ScanError> {
        let mut roots = Vec::new();
        for mut root in observed {
            let admission = if root.active_scan_id.is_some() {
                RootAdmission::PublishedBaseline
            } else if first_import_capture_lease(
                catalog.catalog_path(),
                &root.root_id,
                root.root_generation,
            )?
            .is_some()
            {
                RootAdmission::CapturingFirstImport
            } else {
                // Publication commits before registration retirement. Re-read after
                // observing no lease so an old SQL row cannot downgrade that handoff.
                let Some(current) = catalog.load_incremental_catalog_root(&root.root_id)? else {
                    continue;
                };
                if current.active_scan_id.is_some() {
                    root = current;
                    RootAdmission::PublishedBaseline
                } else if current.root_generation == root.root_generation {
                    root = current;
                    RootAdmission::BaselineRequired
                } else {
                    continue;
                }
            };
            roots.push((root, admission));
        }
        roots.extend(
            catalog
                .load_inactive_first_import_roots()?
                .into_iter()
                .map(|root| (root, RootAdmission::BaselineRequired)),
        );
        Ok(Self { roots })
    }

    pub(super) fn observing_roots(&self) -> Vec<IncrementalCatalogRoot> {
        self.roots
            .iter()
            .filter(|(_, admission)| match admission {
                RootAdmission::PublishedBaseline => true,
                RootAdmission::CapturingFirstImport => true,
                RootAdmission::BaselineRequired => false,
            })
            .map(|(root, _)| root.clone())
            .collect()
    }

    pub(super) fn append_dormant_statuses(&self, snapshot: &mut LibrarySynchronizationSnapshot) {
        for (root, admission) in &self.roots {
            snapshot.catalog_revision = snapshot.catalog_revision.max(root.catalog_revision);
            if !matches!(admission, RootAdmission::BaselineRequired) {
                continue;
            }
            snapshot
                .roots
                .retain(|status| status.root_id != root.root_id);
            snapshot.roots.push(LibraryRootSynchronizationStatus {
                root_id: root.root_id.clone(),
                root_generation: root.root_generation.value(),
                availability: LibraryRootAvailability::Unknown,
                freshness: CatalogFreshnessState::NeedsReconciliation,
                freshness_cause: CatalogFreshnessCause::EvidenceGap,
                continuity: PersistentJournalContinuityState::BaselineRequired,
                phase: LibrarySynchronizationPhase::Blocked,
                source_health: LibraryChangeSourceHealth::Stopped,
                queue_health: LibraryChangeQueueHealth::Idle,
                pending_change_count: 0,
                retry_wait_count: 0,
                freshness_unknown_count: 0,
                recovery_blocked: false,
                last_issue_code: Some("library_first_import_required".to_owned()),
            });
        }
        snapshot
            .roots
            .sort_by(|left, right| left.root_id.cmp(&right.root_id));
    }

    pub(super) fn revalidate(
        &mut self,
        catalog: &SqliteCatalog,
        runtime: &mut super::LibrarySynchronizationRuntime,
        snapshot: &mut LibrarySynchronizationSnapshot,
    ) -> Result<(), ScanError> {
        *self = Self::load(catalog)?;
        let roots = self.observing_roots();
        runtime.reconcile_roots(&roots)?;
        snapshot
            .roots
            .retain(|status| roots.iter().any(|root| root.root_id == status.root_id));
        Ok(())
    }
}

#[cfg(test)]
mod tests;
