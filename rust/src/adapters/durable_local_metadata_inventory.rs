use std::sync::atomic::{AtomicBool, Ordering};

use crate::domain::{
    FileIdentityEvidence, LeasedLibraryChange, LibraryChangeLane, MetadataInventoryEntryKind,
    MetadataInventoryPage, MetadataInventoryRun, MetadataInventoryScope, ScanError,
};
use crate::ports::{MetadataInventorySource, MetadataInventorySourcePreparation};

use super::local_files::PublicationGuardedMetadataInventoryEntries;
use super::{
    MetadataInventorySpoolExecution, PublicationGuardedFileDiscovery, SqliteCatalog,
    SqliteCatalogSession,
};

const RAW_SOURCE_BATCH_ENTRIES: usize = 128;

pub(crate) struct DurableLocalMetadataInventory {
    discovery: PublicationGuardedFileDiscovery,
    session: SqliteCatalogSession,
    execution: MetadataInventorySpoolExecution,
    root_path: String,
    root_identity: FileIdentityEvidence,
    active_directory: Option<ActiveDirectory>,
}

struct ActiveDirectory {
    relative_directory: String,
    opening_identity: FileIdentityEvidence,
    entries: PublicationGuardedMetadataInventoryEntries,
}

impl DurableLocalMetadataInventory {
    pub(crate) fn open(
        session: SqliteCatalogSession,
        root_path: &str,
        scope: &MetadataInventoryScope,
        run: &MetadataInventoryRun,
        authority: &LeasedLibraryChange,
        expected_publication_identity: Option<&FileIdentityEvidence>,
        expected_source_identity: &FileIdentityEvidence,
    ) -> Result<Self, ScanError> {
        let discovery = PublicationGuardedFileDiscovery::new_metadata_inventory_source_guard(
            root_path,
            expected_publication_identity,
        )?;
        let root_identity = discovery
            .metadata_inventory_root_identity()?
            .ok_or_else(|| {
                ScanError::new(
                    "metadata_inventory_root_identity_unavailable",
                    "The recovery source root has no durable Windows file identity",
                )
            })?;
        discovery.require_metadata_inventory_root_identity(expected_source_identity)?;
        let (initial_entry, initial_directory) = match scope {
            MetadataInventoryScope::Root => (None, Some(String::new())),
            MetadataInventoryScope::Subtree { relative_path } => {
                match discovery.metadata_inventory_entry(relative_path) {
                    Ok(entry) => {
                        let directory = (entry.kind == MetadataInventoryEntryKind::Directory)
                            .then(|| entry.relative_path.clone());
                        (Some(entry), directory)
                    }
                    Err(issue) if issue.code == "file_missing" => (None, None),
                    Err(issue) => return Err(issue_error(issue)),
                }
            }
        };
        let mut catalog = session.open_in_lane(LibraryChangeLane::Recovery)?;
        let execution = catalog.initialize_metadata_inventory_spool(
            run,
            authority,
            &root_identity,
            initial_entry.as_ref(),
            initial_directory.as_deref(),
            run.updated_unix_ms,
        )?;
        Ok(Self {
            discovery,
            session,
            execution,
            root_path: root_path.to_owned(),
            root_identity,
            active_directory: None,
        })
    }

    fn open_next_directory(
        &mut self,
        catalog: &mut SqliteCatalog,
        updated_unix_ms: i64,
    ) -> Result<bool, ScanError> {
        if catalog.metadata_inventory_spool_is_ready(&self.execution)? {
            return Ok(false);
        }
        let relative_directory = catalog
            .next_metadata_inventory_spool_directory(&self.execution)?
            .ok_or_else(|| {
                ScanError::new(
                    "metadata_inventory_spool_frontier_invalid",
                    "The durable source spool has no resumable directory",
                )
            })?;
        let entries = self
            .discovery
            .streaming_metadata_inventory_entries_in_directory(&relative_directory)
            .map_err(issue_error)?;
        let opening_identity = entries.directory_identity().cloned().ok_or_else(|| {
            ScanError::new(
                "metadata_inventory_spool_directory_identity_unavailable",
                "The source directory has no stable Windows file identity",
            )
        })?;
        catalog.begin_metadata_inventory_spool_directory(
            &self.execution,
            &relative_directory,
            &opening_identity,
            updated_unix_ms,
        )?;
        self.active_directory = Some(ActiveDirectory {
            relative_directory,
            opening_identity,
            entries,
        });
        Ok(true)
    }
}

impl MetadataInventorySource for DurableLocalMetadataInventory {
    fn rebind_recovery_lease(&mut self, authority: &LeasedLibraryChange) -> Result<(), ScanError> {
        let catalog = self.session.open_in_lane(LibraryChangeLane::Recovery)?;
        self.execution =
            catalog.rebind_metadata_inventory_spool_execution(&self.execution, authority)?;
        Ok(())
    }

    fn prepare_next_page(
        &mut self,
        max_source_entries: u32,
        cancelled: &AtomicBool,
    ) -> Result<MetadataInventorySourcePreparation, ScanError> {
        if max_source_entries == 0 {
            return Err(ScanError::new(
                "metadata_inventory_page_limit_invalid",
                "Metadata inventory source pages require a positive bound",
            ));
        }
        if cancelled.load(Ordering::Acquire) {
            return Err(cancelled_error());
        }
        let mut catalog = self.session.open_in_lane(LibraryChangeLane::Recovery)?;
        if self.active_directory.is_some() {
            catalog.validate_metadata_inventory_spool_execution(&self.execution)?;
        }
        let updated_unix_ms = now_unix_ms()?;
        if self.active_directory.is_none()
            && catalog.reset_metadata_inventory_spool_batch(
                &self.execution,
                max_source_entries.min(RAW_SOURCE_BATCH_ENTRIES as u32),
                updated_unix_ms,
            )?
        {
            return Ok(MetadataInventorySourcePreparation::Yielded);
        }
        let _publication_guard =
            PublicationGuardedFileDiscovery::new_metadata_inventory_publication_guard(
                &self.root_path,
                &self.root_identity,
            )?;
        if self.active_directory.is_none()
            && !self.open_next_directory(&mut catalog, updated_unix_ms)?
        {
            return Ok(MetadataInventorySourcePreparation::Ready);
        }
        let source_limit = usize::try_from(max_source_entries)
            .unwrap_or(usize::MAX)
            .min(RAW_SOURCE_BATCH_ENTRIES);
        let mut batch = Vec::with_capacity(source_limit);
        let mut exhausted = false;
        {
            let active = self.active_directory.as_mut().ok_or_else(|| {
                ScanError::new(
                    "metadata_inventory_spool_frontier_invalid",
                    "The durable source directory state is unavailable",
                )
            })?;
            while batch.len() < source_limit {
                if cancelled.load(Ordering::Acquire) {
                    return Err(cancelled_error());
                }
                let Some(entry) = active.entries.next() else {
                    exhausted = true;
                    break;
                };
                batch.push(entry.map_err(issue_error)?);
            }
        }
        #[cfg(test)]
        super::local_files::record_source_peak_staged_window(&self.root_path, batch.len());
        if exhausted {
            let active = self.active_directory.take().ok_or_else(|| {
                ScanError::new(
                    "metadata_inventory_spool_frontier_invalid",
                    "The exhausted source directory state is unavailable",
                )
            })?;
            let closing_identity =
                active
                    .entries
                    .finish()
                    .map_err(issue_error)?
                    .ok_or_else(|| {
                        ScanError::new(
                            "metadata_inventory_spool_directory_identity_unavailable",
                            "The source directory lost its stable Windows file identity",
                        )
                    })?;
            if closing_identity != active.opening_identity {
                return Err(ScanError::new(
                    "metadata_inventory_spool_directory_identity_changed",
                    "The source directory identity changed during bounded enumeration",
                ));
            }
            catalog.complete_metadata_inventory_spool_directory(
                &self.execution,
                &active.relative_directory,
                &closing_identity,
                &batch,
                updated_unix_ms,
            )?;
        } else {
            let active = self.active_directory.as_ref().ok_or_else(|| {
                ScanError::new(
                    "metadata_inventory_spool_frontier_invalid",
                    "The active source directory state is unavailable",
                )
            })?;
            catalog.append_metadata_inventory_spool_entries(
                &self.execution,
                &active.relative_directory,
                &batch,
                updated_unix_ms,
            )?;
        }
        Ok(MetadataInventorySourcePreparation::Yielded)
    }

    fn next_page(
        &mut self,
        max_entries: u32,
        cancelled: &AtomicBool,
    ) -> Result<MetadataInventoryPage, ScanError> {
        if cancelled.load(Ordering::Acquire) {
            return Err(cancelled_error());
        }
        let catalog = self.session.open_in_lane(LibraryChangeLane::Recovery)?;
        catalog.load_metadata_inventory_spool_page(&self.execution, max_entries)
    }
}

fn issue_error(issue: crate::domain::ScanIssue) -> ScanError {
    ScanError::new(issue.code, issue.message)
}

fn cancelled_error() -> ScanError {
    ScanError::new(
        "metadata_inventory_cancelled",
        "The metadata inventory was cancelled",
    )
}

fn now_unix_ms() -> Result<i64, ScanError> {
    let duration = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|error| {
            ScanError::new(
                "system_clock_invalid",
                format!("Could not read the system clock: {error}"),
            )
        })?;
    i64::try_from(duration.as_millis()).map_err(|_| {
        ScanError::new(
            "system_clock_invalid",
            "The system clock exceeded the supported range",
        )
    })
}
