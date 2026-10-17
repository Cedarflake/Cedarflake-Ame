use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use crate::adapters::SqliteCatalog;
use crate::application::RetainedMetadataInventorySource;
use crate::domain::{LibraryChangeId, ScanError};
use crate::ports::MetadataInventoryRepository;

#[derive(Default)]
pub(super) struct RecoverySources {
    retained: BTreeMap<LibraryChangeId, RetainedMetadataInventorySource>,
}

#[derive(Clone)]
pub(super) struct RecoverySourceTransfer {
    change_id: LibraryChangeId,
    source: Arc<Mutex<Option<RetainedMetadataInventorySource>>>,
}

impl RecoverySourceTransfer {
    pub(super) fn empty(change_id: LibraryChangeId) -> Self {
        Self {
            change_id,
            source: Arc::new(Mutex::new(None)),
        }
    }

    pub(super) fn take(&self) -> Result<Option<RetainedMetadataInventorySource>, ScanError> {
        self.source
            .lock()
            .map_err(|_| {
                ScanError::new(
                    "metadata_inventory_source_transfer_poisoned",
                    "The retained metadata inventory source transfer became unavailable",
                )
            })
            .map(|mut source| source.take())
    }
}

impl RecoverySources {
    pub(super) fn prune(&mut self, catalog: &SqliteCatalog) -> Result<(), ScanError> {
        let mut stale_change_ids = Vec::new();
        for (change_id, source) in &self.retained {
            let is_current = catalog
                .load_metadata_inventory_recovery_authority(*change_id)?
                .is_some_and(|authority| {
                    authority.retired_unix_ms.is_none() && authority.run_id == source.run_id()
                });
            if !is_current {
                stale_change_ids.push(*change_id);
            }
        }
        for change_id in stale_change_ids {
            self.retained.remove(&change_id);
        }
        Ok(())
    }

    pub(super) fn transfer(&mut self, change_id: LibraryChangeId) -> RecoverySourceTransfer {
        RecoverySourceTransfer {
            change_id,
            source: Arc::new(Mutex::new(self.retained.remove(&change_id))),
        }
    }

    pub(super) fn restore_failed_start(
        &mut self,
        transfer: &RecoverySourceTransfer,
    ) -> Result<(), ScanError> {
        if let Some(source) = transfer.take()? {
            self.retained.insert(transfer.change_id, source);
        }
        Ok(())
    }

    pub(super) fn publish(
        &mut self,
        change_id: LibraryChangeId,
        source: Option<RetainedMetadataInventorySource>,
    ) {
        if let Some(source) = source {
            self.retained.insert(change_id, source);
        } else {
            self.retained.remove(&change_id);
        }
    }

    #[cfg(test)]
    pub(super) fn retained_for_test(
        &self,
    ) -> &BTreeMap<LibraryChangeId, RetainedMetadataInventorySource> {
        &self.retained
    }
}

#[cfg(test)]
mod tests;
