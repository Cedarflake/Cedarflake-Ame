use std::sync::Arc;

use crate::adapters::SqliteCatalog;
use crate::domain::{
    LibraryChangeQueuePolicy, LibraryRootGeneration, LibrarySynchronizationSnapshot, ScanError,
};
use crate::journal_broker::{CallerClaim, PersistentChangeJournalConnection};

use super::super::journal_baseline;
use super::journal_volume::JournalVolumeWork;
use super::journal_work::JournalWork;
use super::{journal_recovery_admission, journal_volume, select_rotated_key};

pub(super) struct JournalLane {
    volume_cursor: Option<String>,
    root_cursor: Option<String>,
    next_action_is_read: bool,
}

impl Default for JournalLane {
    fn default() -> Self {
        Self {
            volume_cursor: None,
            root_cursor: None,
            next_action_is_read: true,
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

    pub(super) fn select(
        &mut self,
        catalog: &mut SqliteCatalog,
        snapshot: &LibrarySynchronizationSnapshot,
        now_unix_ms: i64,
        connection: &PersistentChangeJournalConnection,
        caller: Option<&CallerClaim>,
        policy: LibraryChangeQueuePolicy,
    ) -> Result<Option<JournalWork>, ScanError> {
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
        let (session, caller) = match (connection, caller) {
            (PersistentChangeJournalConnection::Connected(session), Some(caller)) => {
                (Arc::clone(session), caller.clone())
            }
            (PersistentChangeJournalConnection::LiveOnly(_), _) => {
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

        if let Some(work) = journal_recovery_admission::select(catalog, snapshot)? {
            return Ok(Some(JournalWork::Page {
                work,
                session,
                caller,
            }));
        }

        if let Some(work) =
            journal_baseline::select_closing_work(catalog, snapshot, self.root_cursor.as_deref())?
        {
            self.root_cursor = Some(work.root_id().to_owned());
            return Ok(Some(JournalWork::Closing {
                work,
                session,
                caller,
            }));
        }
        if let Some(work) = journal_baseline::select_opening_work(
            catalog,
            snapshot,
            self.root_cursor.as_deref(),
            session.identity_source().as_deref(),
        )? {
            self.root_cursor = Some(work.root_id().to_owned());
            return Ok(Some(JournalWork::Opening {
                work,
                session,
                caller,
            }));
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
                session,
                caller,
            }));
        }
        self.next_action_is_read = true;
        Ok(None)
    }

    pub(super) fn capacity_debt_roots(
        &self,
        catalog: &SqliteCatalog,
        snapshot: &LibrarySynchronizationSnapshot,
        connection: &PersistentChangeJournalConnection,
        caller: Option<&CallerClaim>,
        policy: LibraryChangeQueuePolicy,
    ) -> Result<Vec<(String, LibraryRootGeneration)>, ScanError> {
        if !self.next_action_is_read
            || !matches!(connection, PersistentChangeJournalConnection::Connected(_))
            || caller.is_none()
        {
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
        catalog: &SqliteCatalog,
        snapshot: &LibrarySynchronizationSnapshot,
    ) -> Result<Option<JournalVolumeWork>, ScanError> {
        journal_volume::select(catalog, snapshot, self.volume_cursor.as_deref())
    }

    fn next_root(
        &self,
        catalog: &SqliteCatalog,
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
