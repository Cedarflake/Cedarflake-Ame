use super::{
    FileIdentityEvidence, MAX_SCAN_CATCH_UP_LINEAGE, OptionalExtension, SCAN_QUEUE_LEASE_MILLIS,
    ScanCheckpoint, ScanError, ScanOwner, ScanRequest, SqliteCatalog, Transaction,
    activate_root_change_queue, bind_explicit_recovery_claims_to_foreground_scan, database_error,
    params, retire_root_change_queue, sqlite_integer, unix_time_ms,
    validate_root_publication_identity,
};
use crate::domain::IncrementalCatalogRoot;

pub(super) mod root_binding;

pub(super) use root_binding::RootScanBinding;

impl SqliteCatalog {
    pub(crate) fn has_registered_root_id(&self, root_id: &str) -> Result<bool, ScanError> {
        self.connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM library_roots WHERE id = ?1)",
                [root_id],
                |row| row.get(0),
            )
            .map_err(database_error)
    }

    pub(crate) fn load_registered_root_id(
        &self,
        root_path: &str,
    ) -> Result<Option<String>, ScanError> {
        self.connection
            .query_row(
                "SELECT id FROM library_roots WHERE path = ?1",
                [root_path],
                |row| row.get(0),
            )
            .optional()
            .map_err(database_error)
    }

    pub(crate) fn load_published_root_id_by_identity(
        &self,
        identity: &FileIdentityEvidence,
    ) -> Result<Option<String>, ScanError> {
        root_binding::unique_published_root_id(&self.connection, identity)
    }

    pub(crate) fn begin_identity_recovered_scan(
        &mut self,
        request: &ScanRequest,
        expected: &IncrementalCatalogRoot,
        root_path: &str,
        identity: &FileIdentityEvidence,
    ) -> Result<ScanCheckpoint, ScanError> {
        self.begin_scan_owned(
            request,
            &expected.root_id,
            root_path,
            ScanOwner::Foreground,
            Some(identity),
            RootScanBinding::IdentityRecovered { expected, identity },
        )
    }

    pub(crate) fn begin_relocated_scan(
        &mut self,
        request: &ScanRequest,
        expected: &IncrementalCatalogRoot,
        root_path: &str,
        identity: &FileIdentityEvidence,
    ) -> Result<ScanCheckpoint, ScanError> {
        self.begin_scan_owned(
            request,
            &expected.root_id,
            root_path,
            ScanOwner::Foreground,
            Some(identity),
            RootScanBinding::Relocated(expected),
        )
    }

    pub(super) fn begin_scan_owned(
        &mut self,
        request: &ScanRequest,
        root_id: &str,
        root_path: &str,
        owner: ScanOwner,
        publication_identity: Option<&FileIdentityEvidence>,
        binding: RootScanBinding<'_>,
    ) -> Result<ScanCheckpoint, ScanError> {
        if let Some(identity) = publication_identity {
            validate_root_publication_identity(identity)?;
        }
        let now = unix_time_ms();
        let transaction = self.begin_write()?;
        binding.validate_and_retire(&transaction, root_id, root_path, now)?;
        let scan_exists = transaction
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM scan_runs WHERE id = ?1)",
                [&request.scan_id],
                |row| row.get::<_, bool>(0),
            )
            .map_err(database_error)?;
        if scan_exists {
            return Err(ScanError::new(
                "catalog_scan_already_exists",
                "A new scan cannot reuse an existing scan identifier",
            ));
        }
        transaction
            .execute(
                "INSERT INTO library_roots(id, path, created_unix_ms)
                 VALUES (?1, ?2, ?3)
                 ON CONFLICT(id) DO UPDATE SET path = excluded.path",
                params![root_id, root_path, now],
            )
            .map_err(database_error)?;
        activate_root_change_queue(&transaction, root_id, now)?;
        let has_conflicting_scan = transaction
            .query_row(
                "SELECT EXISTS(
                   SELECT 1 FROM scan_runs
                   WHERE root_id = ?1 AND status IN ('running', 'paused') AND id <> ?2
                 )",
                params![root_id, request.scan_id],
                |row| row.get::<_, bool>(0),
            )
            .map_err(database_error)?;
        if has_conflicting_scan {
            return Err(ScanError::new(
                "catalog_root_scan_in_progress",
                "Another authoritative scan already owns this library root",
            ));
        }
        if let Some(identity) = publication_identity {
            let established = transaction
                .query_row(
                    "SELECT identity_scheme, identity_value
                     FROM library_root_publication_namespaces WHERE root_id = ?1",
                    [root_id],
                    |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
                )
                .optional()
                .map_err(database_error)?;
            if established.as_ref().is_some_and(|(scheme, value)| {
                scheme != &identity.scheme || value != &identity.value
            }) {
                retire_root_change_queue(&transaction, root_id, now)?;
                activate_root_change_queue(&transaction, root_id, now)?;
            }
        }
        let (root_generation_at_start, change_queue_high_watermark) = transaction
            .query_row(
                "SELECT state.generation,
                        MAX(CASE WHEN queue.status IN ('pending', 'leased', 'retry_wait')
                          AND lanes.lane <> 'p0_live' THEN queue.id END)
                 FROM library_change_root_state AS state
                 LEFT JOIN library_change_queue AS queue
                   ON queue.root_id = state.root_id
                  AND queue.root_generation = state.generation
                 LEFT JOIN library_change_queue_lanes AS lanes
                   ON lanes.change_id = queue.id
                  AND NOT EXISTS(
                    SELECT 1 FROM library_live_gap_recovery_claims AS claim
                    WHERE claim.gap_change_id = queue.id
                  )
                  WHERE state.root_id = ?1 AND state.is_active = 1
                 GROUP BY state.generation",
                [root_id],
                |row| Ok((row.get::<_, i64>(0)?, row.get::<_, Option<i64>>(1)?)),
            )
            .map_err(database_error)?;
        transaction
            .execute(
                "INSERT INTO scan_runs(
                   id, root_id, status, started_unix_ms, max_items, max_entries, preview_edge,
                   root_generation_at_start, change_queue_high_watermark, scan_owner
                 ) VALUES (?1, ?2, 'running', ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    request.scan_id,
                    root_id,
                    now,
                    request.max_items.map(i64::from),
                    request.max_entries.map(i64::from),
                    i64::from(request.preview_edge),
                    root_generation_at_start,
                    change_queue_high_watermark,
                    owner.as_str(),
                ],
            )
            .map_err(database_error)?;
        if let Some(identity) = publication_identity {
            transaction
                .execute(
                    "INSERT INTO library_scan_publication_namespace_bindings(
                       scan_id, root_id, root_generation, identity_scheme,
                       identity_value, bound_unix_ms
                     ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params![
                        request.scan_id,
                        root_id,
                        root_generation_at_start,
                        identity.scheme,
                        identity.value,
                        now,
                    ],
                )
                .map_err(database_error)?;
        }
        if owner == ScanOwner::Foreground {
            bind_explicit_recovery_claims_to_foreground_scan(
                &transaction,
                &request.scan_id,
                root_id,
                root_generation_at_start,
                now,
            )?;
        }
        if let Some(high_watermark) = change_queue_high_watermark {
            transaction
                .execute(
                    "UPDATE library_change_queue
                     SET status = 'leased', next_retry_unix_ms = NULL,
                         lease_generation = lease_generation + 1,
                         lease_expires_unix_ms = ?1, updated_unix_ms = ?2,
                         authoritative_scan_id = ?3
                      WHERE root_id = ?4 AND root_generation = ?5 AND id <= ?6
                        AND status IN ('pending', 'retry_wait')
                        AND EXISTS(
                          SELECT 1 FROM library_change_queue_lanes AS lanes
                          WHERE lanes.change_id = library_change_queue.id
                            AND lanes.lane <> 'p0_live'
                        )
                        AND NOT EXISTS(
                          SELECT 1 FROM library_live_gap_recovery_claims AS claim
                          WHERE claim.gap_change_id = library_change_queue.id
                        )",
                    params![
                        now.saturating_add(SCAN_QUEUE_LEASE_MILLIS),
                        now,
                        request.scan_id,
                        root_id,
                        root_generation_at_start,
                        high_watermark,
                    ],
                )
                .map_err(database_error)?;
            transaction
                .execute(
                    "INSERT INTO scan_run_catch_up_lineage(
                           scan_id, catch_up_source, catch_up_watermark, enrolled_unix_ms
                         )
                         SELECT ?1, lineage.catch_up_source,
                                lineage.catch_up_watermark,
                                MAX(lineage.enrolled_unix_ms)
                         FROM library_change_queue AS changes
                         JOIN library_change_queue_catch_up_lineage AS lineage
                           ON lineage.change_id = changes.id
                         WHERE changes.root_id = ?2
                           AND changes.root_generation = ?3
                            AND changes.id <= ?4
                            AND changes.authoritative_scan_id = ?1
                            AND changes.status IN ('pending', 'leased', 'retry_wait')
                            AND NOT EXISTS(
                              SELECT 1 FROM library_live_gap_recovery_claims AS claim
                              WHERE claim.gap_change_id = changes.id
                            )
                         GROUP BY lineage.catch_up_source, lineage.catch_up_watermark",
                    params![
                        request.scan_id,
                        root_id,
                        root_generation_at_start,
                        high_watermark,
                    ],
                )
                .map_err(database_error)?;
            let lineage_count = transaction
                .query_row(
                    "SELECT COUNT(*) FROM scan_run_catch_up_lineage
                         WHERE scan_id = ?1",
                    [&request.scan_id],
                    |row| row.get::<_, i64>(0),
                )
                .map_err(database_error)?;
            if lineage_count > MAX_SCAN_CATCH_UP_LINEAGE {
                return Err(ScanError::new(
                    "catalog_scan_catch_up_lineage_limit_exceeded",
                    "The scan captured too many catch-up watermarks",
                ));
            }
        }
        transaction
            .execute(
                "INSERT INTO scan_directory_frontier(scan_id, relative_path) VALUES (?1, '')",
                [&request.scan_id],
            )
            .map_err(database_error)?;
        transaction.commit().map_err(database_error)?;
        Ok(ScanCheckpoint::default())
    }
}
