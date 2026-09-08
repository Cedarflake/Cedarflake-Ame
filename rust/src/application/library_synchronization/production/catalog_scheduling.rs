use super::*;

impl ProductionSynchronization {
    pub(super) fn schedule_catalog_work(
        &mut self,
        catalog: &mut SqliteCatalog,
        snapshot: &LibrarySynchronizationSnapshot,
        poll_unix_ms: i64,
        change_capture_unix_ms: i64,
        storage: &crate::application::storage::StoragePaths,
    ) -> Result<(), ScanError> {
        // Synchronous writes cannot wait behind this same poll's retained ingress position.
        // Worker retirement and read-only status projection remain outside this boundary.
        if self.runtime.has_reserved_ingress() {
            return Ok(());
        }
        catalog.finalize_ready_first_import_journal_baseline(poll_unix_ms)?;
        self.schedule_next_live_work(catalog, snapshot, poll_unix_ms, storage)?;
        self.schedule_next_journal_work(catalog, snapshot, change_capture_unix_ms, storage)?;
        if self.recovery.is_none()
            && let Some(work) = ready_recovery_work(self, catalog, snapshot, poll_unix_ms)?
        {
            let root_id = work.root_id().to_owned();
            let root_generation = work.root_generation();
            let continuity_revision =
                self.runtime
                    .root_continuity_revision(&root_id)
                    .ok_or_else(|| {
                        ScanError::new(
                            "library_continuity_root_missing",
                            "The selected continuity root is no longer active",
                        )
                    })?;
            match work {
                ReadyRecoveryWork::CandidateDrain { .. } => {
                    self.start_recovery_candidate_drain(
                        root_id,
                        root_generation,
                        continuity_revision,
                        poll_unix_ms,
                        storage.catalog_path.clone(),
                    )?;
                }
                ReadyRecoveryWork::LegacyUnownedDrain { .. } => {
                    self.start_legacy_unowned_recovery_drain(
                        root_id,
                        root_generation,
                        continuity_revision,
                        poll_unix_ms,
                        storage.catalog_path.clone(),
                    )?;
                }
                ReadyRecoveryWork::Control { .. } => {
                    if let Some(leased) = catalog.lease_metadata_inventory_recovery(
                        &root_id,
                        root_generation,
                        poll_unix_ms,
                        self.runtime.queue_policy(),
                    )? {
                        let leased_for_defer = leased.clone();
                        if let Err(start_error) = self.start_automatic_recovery(
                            root_id,
                            root_generation,
                            continuity_revision,
                            leased,
                            poll_unix_ms,
                            storage.catalog_path.clone(),
                        ) {
                            if let Err(defer_error) = catalog.defer_library_change(
                                leased_for_defer.change.id,
                                leased_for_defer.lease_generation,
                                poll_unix_ms,
                            ) {
                                return Err(ScanError::new(
                                    "authoritative_recovery_worker_start_cleanup_failed",
                                    format!(
                                        "{}; leased work could not be deferred: {}",
                                        start_error.message, defer_error.message
                                    ),
                                ));
                            }
                            return Err(start_error);
                        }
                    }
                }
            }
        }
        Ok(())
    }
}
