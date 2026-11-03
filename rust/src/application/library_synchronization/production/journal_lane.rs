use std::sync::Arc;

use crate::domain::{
    LibraryChangeQueuePolicy, LibraryRootGeneration, LibrarySynchronizationSnapshot, ScanError,
};
use crate::ports::journal_baseline::JournalBaselineAdmission;
use crate::ports::journal_recovery::JournalRecoveryRepository;
use crate::ports::journal_runtime::{JournalRuntimeConnection, JournalRuntimeSession};
use crate::ports::scan_admission::FirstImportReadiness;
use crate::ports::{
    CatalogRepository, IncrementalCatalogRepository, LibraryChangeReadiness,
    MetadataInventoryRepository, PersistentJournalRepository,
};

use super::super::journal_baseline;
use super::journal_volume::JournalVolumeWork;
use super::journal_work::JournalWork;
use super::{journal_recovery_admission, journal_volume, select_rotated_key};

pub(super) struct JournalLane {
    fallback_session: super::journal_fallback_session::JournalFallbackSession,
    volume_cursor: Option<String>,
    root_cursor: Option<String>,
    next_action_is_read: bool,
    failures: Arc<super::journal_work::fallback::JournalReadFailures>,
}

impl Default for JournalLane {
    fn default() -> Self {
        Self {
            fallback_session: Default::default(),
            volume_cursor: None,
            root_cursor: None,
            next_action_is_read: true,
            failures: Arc::default(),
        }
    }
}

impl JournalLane {
    #[cfg(test)]
    pub(super) fn after_page_for_test() -> Self {
        Self {
            next_action_is_read: false,
            ..Self::default()
        }
    }

    pub(super) fn select<Repository, Connection: JournalRuntimeConnection>(
        &mut self,
        catalog: &mut Repository,
        snapshot: &LibrarySynchronizationSnapshot,
        now_unix_ms: i64,
        connection: &Connection,
        policy: LibraryChangeQueuePolicy,
    ) -> Result<Option<JournalWork<Connection::Session>>, ScanError>
    where
        Repository: CatalogRepository
            + IncrementalCatalogRepository
            + LibraryChangeReadiness
            + MetadataInventoryRepository
            + FirstImportReadiness
            + JournalBaselineAdmission
            + JournalRecoveryRepository,
    {
        self.fallback_session
            .prepare(catalog, now_unix_ms, policy, connection.is_connected())?;
        self.failures
            .retain_roots(|id| snapshot.roots.iter().any(|root| root.root_id == id));
        if let Some((root_id, root_generation)) =
            self.next_root(catalog, snapshot, now_unix_ms, policy)?
        {
            self.root_cursor = Some(root_id.clone());
            self.next_action_is_read = true;
            return Ok(Some(JournalWork::Drain {
                root_id,
                root_generation,
            }));
        }
        let session = match connection.session() {
            Some(session) => session,
            None if !connection.is_connected() => {
                if let Some(work) = journal_baseline::select_opening_work(
                    catalog,
                    snapshot,
                    self.root_cursor.as_deref(),
                    None,
                )? {
                    journal_baseline::persist_unavailable_session(catalog, &work, now_unix_ms)?;
                }
                return Ok(None);
            }
            _ => return Ok(None),
        };

        if self.next_action_is_read
            && let Some(work) = journal_recovery_admission::select(catalog, snapshot)?
        {
            self.root_cursor = work
                .roots
                .first()
                .map(|root| root.checkpoint.root_id.clone());
            self.next_action_is_read = false;
            return Ok(Some(JournalWork::Page {
                work,
                failures: Arc::clone(&self.failures),
                session,
            }));
        }

        if let Some(work) =
            journal_baseline::select_closing_work(catalog, snapshot, self.root_cursor.as_deref())?
        {
            self.root_cursor = Some(work.root_id().to_owned());
            return Ok(Some(JournalWork::Closing {
                failures: Arc::clone(&self.failures),
                work,
                session,
            }));
        }
        if let Some(work) = journal_baseline::select_opening_work(
            catalog,
            snapshot,
            self.root_cursor.as_deref(),
            session.identity_source().as_deref(),
        )? {
            self.root_cursor = Some(work.root_id().to_owned());
            return Ok(Some(JournalWork::Opening { work, session }));
        }

        if self.next_action_is_read
            && let Some(work) = self.next_volume(catalog, snapshot)?
        {
            for root in &work.roots {
                if !catalog.journal_admission_has_headroom(
                    &root.checkpoint.root_id,
                    root.checkpoint.root_generation,
                    policy,
                )? {
                    return Ok(None);
                }
            }
            self.volume_cursor = Some(work.key.clone());
            self.next_action_is_read = false;
            return Ok(Some(JournalWork::Page {
                work,
                failures: Arc::clone(&self.failures),
                session,
            }));
        }
        self.next_action_is_read = true;
        Ok(None)
    }

    pub(super) fn capacity_debt_roots(
        &self,
        catalog: &(
             impl LibraryChangeReadiness + IncrementalCatalogRepository + PersistentJournalRepository
         ),
        snapshot: &LibrarySynchronizationSnapshot,
        connection: &impl JournalRuntimeConnection,
        policy: LibraryChangeQueuePolicy,
    ) -> Result<Vec<(String, LibraryRootGeneration)>, ScanError> {
        if !self.next_action_is_read || connection.session().is_none() {
            return Ok(Vec::new());
        }
        let Some(work) = self.next_volume(catalog, snapshot)? else {
            return Ok(Vec::new());
        };
        let mut roots = Vec::new();
        for root in work.roots {
            let checkpoint = root.checkpoint;
            if !catalog.journal_admission_has_headroom(
                &checkpoint.root_id,
                checkpoint.root_generation,
                policy,
            )? {
                roots.push((checkpoint.root_id, checkpoint.root_generation));
            }
        }
        Ok(roots)
    }

    fn next_volume(
        &self,
        catalog: &(impl IncrementalCatalogRepository + PersistentJournalRepository),
        snapshot: &LibrarySynchronizationSnapshot,
    ) -> Result<Option<JournalVolumeWork>, ScanError> {
        journal_volume::select(catalog, snapshot, self.volume_cursor.as_deref())
    }

    fn next_root(
        &self,
        catalog: &impl LibraryChangeReadiness,
        snapshot: &LibrarySynchronizationSnapshot,
        now_unix_ms: i64,
        policy: LibraryChangeQueuePolicy,
    ) -> Result<Option<(String, LibraryRootGeneration)>, ScanError> {
        let mut eligible = Vec::new();
        for root in &snapshot.roots {
            if root.availability != crate::domain::LibraryRootAvailability::Available
                || root.source_health != crate::domain::LibraryChangeSourceHealth::Healthy
            {
                continue;
            }
            let generation = LibraryRootGeneration::new(root.root_generation).ok_or_else(|| {
                ScanError::new(
                    "library_root_generation_invalid",
                    "The synchronization root generation is invalid",
                )
            })?;
            if catalog.has_ready_journal_path_library_change(
                &root.root_id,
                generation,
                now_unix_ms,
                policy,
            )? || catalog.has_ready_journal_authoritative_library_change(
                &root.root_id,
                generation,
                now_unix_ms,
                policy,
            )? {
                eligible.push(root.root_id.clone());
            }
        }
        let Some(root_id) = select_rotated_key(eligible.iter(), self.root_cursor.as_deref()) else {
            return Ok(None);
        };
        let root = snapshot
            .roots
            .iter()
            .find(|root| root.root_id == root_id)
            .expect("selected journal root comes from the snapshot");
        let generation = LibraryRootGeneration::new(root.root_generation).ok_or_else(|| {
            ScanError::new(
                "library_root_generation_invalid",
                "The synchronization root generation is invalid",
            )
        })?;
        Ok(Some((root_id, generation)))
    }
}
