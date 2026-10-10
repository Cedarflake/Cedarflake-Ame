use super::*;

pub(super) type PublicationHook = Box<dyn FnMut(&mut SqliteCatalog, usize)>;
pub(super) type IdentityReadObserver =
    Box<dyn Fn(&FileIdentityEvidence, &[LibraryChangeCatchUpEvidence])>;

pub(super) struct RevisionRacingCatalog {
    pub(super) catalog: SqliteCatalog,
    pub(super) on_publication: Option<PublicationHook>,
    pub(super) on_identity_read: Option<IdentityReadObserver>,
    pub(super) races: Vec<(String, String)>,
    pub(super) next_race: usize,
    pub(super) publication_attempts: usize,
    pub(super) rewrite_on_first_publication: Option<PathBuf>,
    pub(super) cancel_after_first_publication: Option<Arc<AtomicBool>>,
    pub(super) cancel_after_leasing: Option<Arc<AtomicBool>>,
    pub(super) guarded_rename_during_publication: Option<(PathBuf, PathBuf)>,
}

impl RevisionRacingCatalog {
    pub(super) fn new(catalog: SqliteCatalog, races: Vec<(String, String)>) -> Self {
        Self {
            catalog,
            on_publication: None,
            on_identity_read: None,
            races,
            next_race: 0,
            publication_attempts: 0,
            rewrite_on_first_publication: None,
            cancel_after_first_publication: None,
            cancel_after_leasing: None,
            guarded_rename_during_publication: None,
        }
    }

    fn advance_competing_revision(&mut self) -> Result<(), ScanError> {
        let Some((scan_id, root_id)) = self.races.get(self.next_race).cloned() else {
            return Ok(());
        };
        if self.next_race == 0
            && let Some(path) = self.rewrite_on_first_publication.take()
        {
            write_png(&path, 4, 3, [91, 92, 93]);
        }
        self.catalog.publish_scan(&scan_id, &root_id, 0, 0)?;
        self.next_race += 1;
        if self.next_race == 1
            && let Some(cancelled) = &self.cancel_after_first_publication
        {
            cancelled.store(true, Ordering::Release);
        }
        Ok(())
    }
}

impl IncrementalCatalogRepository for RevisionRacingCatalog {
    fn load_incremental_catalog_roots(
        &self,
    ) -> Result<Vec<crate::domain::IncrementalCatalogRoot>, ScanError> {
        self.catalog.load_incremental_catalog_roots()
    }

    fn load_incremental_catalog_root(
        &self,
        root_id: &str,
    ) -> Result<Option<crate::domain::IncrementalCatalogRoot>, ScanError> {
        self.catalog.load_incremental_catalog_root(root_id)
    }

    fn load_incremental_location_by_relative_path(
        &self,
        root_id: &str,
        relative_path: &str,
    ) -> Result<Option<AssetLocationView>, ScanError> {
        self.catalog
            .load_incremental_location_by_relative_path(root_id, relative_path)
    }

    fn load_incremental_locations_by_relative_paths(
        &self,
        root_id: &str,
        relative_paths: &[String],
    ) -> Result<Vec<AssetLocationView>, ScanError> {
        self.catalog
            .load_incremental_locations_by_relative_paths(root_id, relative_paths)
    }

    fn load_incremental_location_by_file_identity(
        &self,
        identity: &FileIdentityEvidence,
        catch_up_lineage: &[LibraryChangeCatchUpEvidence],
    ) -> Result<Option<AssetLocationView>, ScanError> {
        if let Some(observer) = &self.on_identity_read {
            observer(identity, catch_up_lineage);
        }
        self.catalog
            .load_incremental_location_by_file_identity(identity, catch_up_lineage)
    }

    fn load_incremental_locations_in_subtree(
        &self,
        root_id: &str,
        relative_subtree: &str,
        limit: u32,
    ) -> Result<Vec<AssetLocationView>, ScanError> {
        self.catalog
            .load_incremental_locations_in_subtree(root_id, relative_subtree, limit)
    }

    fn load_terminal_media_evidence_by_relative_paths(
        &self,
        root_id: &str,
        relative_paths: &[String],
    ) -> Result<Vec<TerminalMediaEvidence>, ScanError> {
        self.catalog
            .load_terminal_media_evidence_by_relative_paths(root_id, relative_paths)
    }

    fn publish_catalog_delta(
        &mut self,
        batch: &CatalogDeltaBatch,
        completed_unix_ms: i64,
    ) -> Result<CatalogDeltaPublication, ScanError> {
        self.publication_attempts += 1;
        if let Some(hook) = &mut self.on_publication {
            hook(&mut self.catalog, self.publication_attempts);
        }
        if let Some((source, destination)) = &self.guarded_rename_during_publication {
            let error = fs::rename(source, destination)
                .expect_err("the configured namespace guard must remain held through publication");
            assert_eq!(error.raw_os_error(), Some(32));
        }
        self.advance_competing_revision()?;
        self.catalog.publish_catalog_delta(batch, completed_unix_ms)
    }
}

impl LibraryChangeQueue for RevisionRacingCatalog {
    fn enqueue_library_change_intents(
        &mut self,
        intents: &[LibraryChangeIntent],
        enqueued_unix_ms: i64,
        policy: LibraryChangeQueuePolicy,
    ) -> Result<LibraryChangeEnqueueReport, ScanError> {
        self.catalog
            .enqueue_library_change_intents(intents, enqueued_unix_ms, policy)
    }

    fn enqueue_metadata_inventory_candidates(
        &mut self,
        authority: &crate::domain::LeasedLibraryChange,
        intents: &[LibraryChangeIntent],
        enqueued_unix_ms: i64,
        policy: LibraryChangeQueuePolicy,
    ) -> Result<Option<LibraryChangeEnqueueReport>, ScanError> {
        self.catalog.enqueue_metadata_inventory_candidates(
            authority,
            intents,
            enqueued_unix_ms,
            policy,
        )
    }

    fn enqueue_library_change_intents_with_catch_up(
        &mut self,
        intents: &[LibraryChangeIntent],
        evidence: &LibraryChangeCatchUpEvidence,
        enqueued_unix_ms: i64,
        policy: LibraryChangeQueuePolicy,
    ) -> Result<LibraryChangeEnqueueReport, ScanError> {
        self.catalog.enqueue_library_change_intents_with_catch_up(
            intents,
            evidence,
            enqueued_unix_ms,
            policy,
        )
    }

    fn enqueue_library_change_catch_up_batches(
        &mut self,
        batches: &[LibraryChangeCatchUpQueueBatch],
        enqueued_unix_ms: i64,
        policy: LibraryChangeQueuePolicy,
    ) -> Result<Vec<LibraryChangeEnqueueReport>, ScanError> {
        self.catalog
            .enqueue_library_change_catch_up_batches(batches, enqueued_unix_ms, policy)
    }

    fn lease_library_changes(
        &mut self,
        root_id: &str,
        root_generation: LibraryRootGeneration,
        now_unix_ms: i64,
        policy: LibraryChangeQueuePolicy,
    ) -> Result<Vec<crate::domain::LeasedLibraryChange>, ScanError> {
        self.catalog
            .lease_library_changes(root_id, root_generation, now_unix_ms, policy)
    }

    fn lease_path_library_changes(
        &mut self,
        root_id: &str,
        root_generation: LibraryRootGeneration,
        now_unix_ms: i64,
        policy: LibraryChangeQueuePolicy,
    ) -> Result<Vec<crate::domain::LeasedLibraryChange>, ScanError> {
        self.catalog
            .lease_path_library_changes(root_id, root_generation, now_unix_ms, policy)
    }

    fn lease_path_library_changes_in_lane(
        &mut self,
        root_id: &str,
        root_generation: LibraryRootGeneration,
        lane: LibraryChangeLane,
        now_unix_ms: i64,
        policy: LibraryChangeQueuePolicy,
    ) -> Result<Vec<crate::domain::LeasedLibraryChange>, ScanError> {
        let leased = self.catalog.lease_path_library_changes_in_lane(
            root_id,
            root_generation,
            lane,
            now_unix_ms,
            policy,
        )?;
        if let Some(cancelled) = &self.cancel_after_leasing {
            cancelled.store(true, Ordering::Release);
        }
        Ok(leased)
    }

    fn lease_metadata_inventory_recovery_candidates(
        &mut self,
        root_id: &str,
        root_generation: LibraryRootGeneration,
        now_unix_ms: i64,
        policy: LibraryChangeQueuePolicy,
    ) -> Result<Vec<crate::domain::LeasedLibraryChange>, ScanError> {
        self.catalog.lease_metadata_inventory_recovery_candidates(
            root_id,
            root_generation,
            now_unix_ms,
            policy,
        )
    }

    fn lease_authoritative_library_change(
        &mut self,
        root_id: &str,
        root_generation: LibraryRootGeneration,
        now_unix_ms: i64,
        policy: LibraryChangeQueuePolicy,
    ) -> Result<Option<crate::domain::LeasedLibraryChange>, ScanError> {
        self.catalog.lease_authoritative_library_change(
            root_id,
            root_generation,
            now_unix_ms,
            policy,
        )
    }

    fn complete_library_change(
        &mut self,
        change_id: LibraryChangeId,
        lease_generation: u64,
        catalog_revision_at_success: u64,
        completed_unix_ms: i64,
    ) -> Result<LibraryChangeLeaseUpdateOutcome, ScanError> {
        self.catalog.complete_library_change(
            change_id,
            lease_generation,
            catalog_revision_at_success,
            completed_unix_ms,
        )
    }

    fn retry_library_change(
        &mut self,
        change_id: LibraryChangeId,
        lease_generation: u64,
        failure: &LibraryChangeFailure,
        failed_unix_ms: i64,
        policy: LibraryChangeQueuePolicy,
    ) -> Result<LibraryChangeLeaseUpdateOutcome, ScanError> {
        self.catalog.retry_library_change(
            change_id,
            lease_generation,
            failure,
            failed_unix_ms,
            policy,
        )
    }

    fn promote_live_watcher_gap_to_metadata_inventory(
        &mut self,
        change_id: LibraryChangeId,
        lease_generation: u64,
        failure: &LibraryChangeFailure,
        promoted_unix_ms: i64,
        policy: LibraryChangeQueuePolicy,
    ) -> Result<LibraryChangeLeaseUpdateOutcome, ScanError> {
        self.catalog.promote_live_watcher_gap_to_metadata_inventory(
            change_id,
            lease_generation,
            failure,
            promoted_unix_ms,
            policy,
        )
    }

    fn defer_library_change(
        &mut self,
        change_id: LibraryChangeId,
        lease_generation: u64,
        deferred_unix_ms: i64,
    ) -> Result<LibraryChangeLeaseUpdateOutcome, ScanError> {
        self.catalog
            .defer_library_change(change_id, lease_generation, deferred_unix_ms)
    }

    fn defer_library_changes(
        &mut self,
        leases: &[crate::domain::LibraryChangeLeaseIdentity],
        deferred_unix_ms: i64,
    ) -> Result<Vec<LibraryChangeLeaseUpdateOutcome>, ScanError> {
        self.catalog.defer_library_changes(leases, deferred_unix_ms)
    }

    fn load_library_change_queue_metrics(
        &self,
        now_unix_ms: i64,
        policy: LibraryChangeQueuePolicy,
    ) -> Result<LibraryChangeQueueMetrics, ScanError> {
        self.catalog
            .load_library_change_queue_metrics(now_unix_ms, policy)
    }

    fn load_library_change_root_queue_metrics(
        &self,
        root_id: &str,
        root_generation: LibraryRootGeneration,
        now_unix_ms: i64,
        policy: LibraryChangeQueuePolicy,
    ) -> Result<LibraryChangeQueueMetrics, ScanError> {
        self.catalog.load_library_change_root_queue_metrics(
            root_id,
            root_generation,
            now_unix_ms,
            policy,
        )
    }

    fn cleanup_terminal_library_changes(
        &mut self,
        terminal_before_unix_ms: i64,
        limit: u32,
    ) -> Result<u32, ScanError> {
        self.catalog
            .cleanup_terminal_library_changes(terminal_before_unix_ms, limit)
    }
}

pub(super) fn prepare_revision_races(
    catalog: &mut SqliteCatalog,
    parent: &Path,
    count: usize,
) -> Vec<(String, String)> {
    (0..count)
        .map(|index| {
            let root_id = format!("revision-race-root-{index}");
            let scan_id = format!("revision-race-scan-{index}");
            let root_path = parent.join(&root_id);
            fs::create_dir_all(&root_path).expect("create revision race root");
            let root_path = root_path.to_string_lossy().into_owned();
            catalog
                .begin_scan(
                    &ScanRequest {
                        scan_id: scan_id.clone(),
                        root_path: root_path.clone(),
                        max_items: None,
                        max_entries: None,
                        preview_edge: 256,
                    },
                    &root_id,
                    &root_path,
                )
                .expect("begin competing revision scan");
            catalog
                .prove_live_only_first_import_handoff_for_test(&scan_id)
                .expect("prove revision-race first-import handoff");
            (scan_id, root_id)
        })
        .collect()
}
