use std::collections::{HashMap, HashSet};

use rusqlite::{OptionalExtension, TransactionBehavior, params};

use crate::domain::{
    AssetLocationView, CatalogDeltaBatch, CatalogDeltaPublication, CatalogDeltaPublicationStatus,
    DerivedEvidenceDisposition, FileIdentityEvidence, IncrementalCatalogRoot,
    IncrementalReconciliationOutcome, LibraryChangeCatchUpEvidence, LibraryChangeId,
    LibraryRootGeneration, PreviewStatus, RetainedPreviewExpectation, ScanError,
};
use crate::ports::IncrementalCatalogRepository;

use super::{
    SqliteCatalog, database_error, load_catalog_revision, persist_location, read_stored_asset,
    sqlite_integer, sqlite_unsigned, stored_asset_view,
};

const MAX_DELTA_MUTATIONS: usize = 256;
const MAX_REMOVALS_PER_MUTATION: usize = 4;
const MAX_DELTA_COMPLETIONS: usize = 128;
const MAX_CATCH_UP_LINEAGE_PER_CHANGE: usize = 64;

impl IncrementalCatalogRepository for SqliteCatalog {
    fn load_incremental_catalog_roots(&self) -> Result<Vec<IncrementalCatalogRoot>, ScanError> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT roots.id, roots.path, roots.active_scan_id, state.generation,
                        EXISTS(
                          SELECT 1 FROM scan_runs AS running
                          WHERE running.root_id = roots.id
                            AND running.status IN ('running', 'paused')
                        ), catalog.revision, state.last_consistency_audit_unix_ms
                 FROM library_roots AS roots
                 JOIN library_change_root_state AS state ON state.root_id = roots.id
                 CROSS JOIN catalog_state AS catalog
                 WHERE state.is_active = 1
                 ORDER BY roots.id",
            )
            .map_err(database_error)?;
        let rows = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, bool>(4)?,
                    row.get::<_, i64>(5)?,
                    row.get::<_, Option<i64>>(6)?,
                ))
            })
            .map_err(database_error)?;
        let mut roots = Vec::new();
        for row in rows {
            let (
                root_id,
                root_path,
                active_scan_id,
                generation,
                has_running_scan,
                revision,
                last_consistency_audit_unix_ms,
            ) = row.map_err(database_error)?;
            let generation = sqlite_unsigned(generation, "root generation")?;
            let root_generation = LibraryRootGeneration::new(generation).ok_or_else(|| {
                ScanError::new(
                    "catalog_root_generation_invalid",
                    "The incremental catalog root has an invalid generation",
                )
            })?;
            roots.push(IncrementalCatalogRoot {
                root_id,
                root_path,
                root_generation,
                active_scan_id,
                has_running_scan,
                catalog_revision: sqlite_unsigned(revision, "catalog revision")?,
                last_consistency_audit_unix_ms,
            });
        }
        Ok(roots)
    }

    fn load_incremental_catalog_root(
        &self,
        root_id: &str,
    ) -> Result<Option<IncrementalCatalogRoot>, ScanError> {
        validate_root_id(root_id)?;
        let stored = self
            .connection
            .query_row(
                "SELECT roots.path, roots.active_scan_id, state.generation, state.is_active,
                        EXISTS(
                          SELECT 1 FROM scan_runs AS running
                          WHERE running.root_id = roots.id
                            AND running.status IN ('running', 'paused')
                        ), catalog.revision, state.last_consistency_audit_unix_ms
                 FROM library_roots AS roots
                 JOIN library_change_root_state AS state ON state.root_id = roots.id
                 CROSS JOIN catalog_state AS catalog
                 WHERE roots.id = ?1",
                [root_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, Option<String>>(1)?,
                        row.get::<_, i64>(2)?,
                        row.get::<_, bool>(3)?,
                        row.get::<_, bool>(4)?,
                        row.get::<_, i64>(5)?,
                        row.get::<_, Option<i64>>(6)?,
                    ))
                },
            )
            .optional()
            .map_err(database_error)?;
        let Some((
            root_path,
            active_scan_id,
            generation,
            is_active,
            has_running_scan,
            revision,
            last_consistency_audit_unix_ms,
        )) = stored
        else {
            return Ok(None);
        };
        if !is_active {
            return Ok(None);
        }
        let generation = sqlite_unsigned(generation, "root generation")?;
        let root_generation = LibraryRootGeneration::new(generation).ok_or_else(|| {
            ScanError::new(
                "catalog_root_generation_invalid",
                "The incremental catalog root has an invalid generation",
            )
        })?;
        Ok(Some(IncrementalCatalogRoot {
            root_id: root_id.to_owned(),
            root_path,
            root_generation,
            active_scan_id,
            has_running_scan,
            catalog_revision: sqlite_unsigned(revision, "catalog revision")?,
            last_consistency_audit_unix_ms,
        }))
    }

    fn load_incremental_location_by_relative_path(
        &self,
        root_id: &str,
        relative_path: &str,
    ) -> Result<Option<AssetLocationView>, ScanError> {
        validate_root_id(root_id)?;
        if relative_path.is_empty() || relative_path.contains('\0') {
            return Err(ScanError::new(
                "catalog_relative_path_invalid",
                "An incremental catalog path must be non-empty and contain no NUL bytes",
            ));
        }
        load_incremental_location(
            self,
            "locations.root_id = ?1 AND locations.relative_path = ?2",
            params![root_id, relative_path],
        )
    }

    fn load_incremental_location_by_file_identity(
        &self,
        identity: &FileIdentityEvidence,
        catch_up_lineage: &[LibraryChangeCatchUpEvidence],
    ) -> Result<Option<AssetLocationView>, ScanError> {
        if identity.scheme.is_empty()
            || identity.value.is_empty()
            || identity.scheme.contains('\0')
            || identity.value.contains('\0')
        {
            return Err(ScanError::new(
                "catalog_file_identity_invalid",
                "Incremental file identity evidence must be non-empty and contain no NUL bytes",
            ));
        }
        let active = load_incremental_location(
            self,
            "locations.file_identity_scheme = ?1 AND locations.file_identity_value = ?2",
            params![identity.scheme, identity.value],
        )?;
        if active.is_some() {
            return Ok(active);
        }
        for evidence in catch_up_lineage {
            if let Some(location) = load_catch_up_handoff_location(self, identity, evidence)? {
                return Ok(Some(location));
            }
        }
        Ok(None)
    }

    fn load_incremental_locations_in_subtree(
        &self,
        root_id: &str,
        relative_subtree: &str,
        limit: u32,
    ) -> Result<Vec<AssetLocationView>, ScanError> {
        validate_root_id(root_id)?;
        if relative_subtree.contains('\0') || limit == 0 || limit > 257 {
            return Err(ScanError::new(
                "catalog_subtree_window_invalid",
                "An authoritative catalog subtree window must be non-empty and bounded",
            ));
        }
        let mut statement = self
            .connection
            .prepare(
                "SELECT locations.asset_id, locations.location_id, locations.root_id,
                        locations.absolute_path, locations.relative_path,
                        locations.preview_path, locations.file_size,
                        locations.created_unix_ms, locations.modified_unix_ms,
                        locations.width, locations.height,
                        locations.preview_status, locations.preview_issue_code,
                        locations.preview_issue_message, locations.metadata_engine_id,
                        locations.metadata_engine_version, locations.capture_local_time,
                        locations.capture_offset_minutes, locations.capture_time_source,
                        locations.capture_raw_value, locations.file_identity_scheme,
                        locations.file_identity_value
                 FROM library_roots AS roots
                 JOIN asset_locations AS locations ON locations.scan_id = roots.active_scan_id
                 WHERE locations.root_id = ?1
                   AND (
                     ?2 = ''
                     OR locations.relative_path = ?2
                     OR substr(locations.relative_path, 1, length(?2) + 1)
                          = (?2 || '/')
                   )
                 ORDER BY locations.relative_path, locations.location_id
                 LIMIT ?3",
            )
            .map_err(database_error)?;
        let rows = statement
            .query_map(
                params![root_id, relative_subtree, i64::from(limit)],
                read_stored_asset,
            )
            .map_err(database_error)?;
        let mut locations = Vec::new();
        for row in rows {
            locations.push(stored_asset_view(row.map_err(database_error)?)?);
        }
        Ok(locations)
    }

    fn publish_catalog_delta(
        &mut self,
        batch: &CatalogDeltaBatch,
        completed_unix_ms: i64,
    ) -> Result<CatalogDeltaPublication, ScanError> {
        validate_delta_batch(batch)?;
        self.flush_pending_locations()?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(database_error)?;
        let current_revision = load_catalog_revision(&transaction)?;
        let root_state = transaction
            .query_row(
                "SELECT roots.active_scan_id, state.generation, state.is_active,
                        EXISTS(
                          SELECT 1 FROM scan_runs AS running
                          WHERE running.root_id = roots.id
                            AND running.status IN ('running', 'paused')
                        )
                 FROM library_roots AS roots
                 JOIN library_change_root_state AS state ON state.root_id = roots.id
                 WHERE roots.id = ?1",
                [&batch.root_id],
                |row| {
                    Ok((
                        row.get::<_, Option<String>>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, bool>(2)?,
                        row.get::<_, bool>(3)?,
                    ))
                },
            )
            .optional()
            .map_err(database_error)?;
        let Some((active_scan_id, stored_generation, is_active, has_running_scan)) = root_state
        else {
            return Ok(publication(
                CatalogDeltaPublicationStatus::RootGenerationChanged,
                current_revision,
            ));
        };
        if !is_active
            || sqlite_unsigned(stored_generation, "root generation")?
                != batch.root_generation.value()
        {
            return Ok(publication(
                CatalogDeltaPublicationStatus::RootGenerationChanged,
                current_revision,
            ));
        }
        if has_running_scan {
            return Ok(publication(
                CatalogDeltaPublicationStatus::RootScanInProgress,
                current_revision,
            ));
        }
        let Some(active_scan_id) = active_scan_id else {
            return Ok(publication(
                CatalogDeltaPublicationStatus::NoPublishedCatalog,
                current_revision,
            ));
        };
        let active_scan_is_published = transaction
            .query_row(
                "SELECT status = 'completed' FROM scan_runs WHERE id = ?1 AND root_id = ?2",
                params![active_scan_id, batch.root_id],
                |row| row.get::<_, bool>(0),
            )
            .optional()
            .map_err(database_error)?
            .unwrap_or(false);
        if !active_scan_is_published {
            return Ok(publication(
                CatalogDeltaPublicationStatus::NoPublishedCatalog,
                current_revision,
            ));
        }
        if current_revision != batch.expected_catalog_revision {
            return Ok(publication(
                CatalogDeltaPublicationStatus::StaleCatalogRevision,
                current_revision,
            ));
        }
        let mut completed_root_authority = false;
        let mut catch_up_evidence_by_change = HashMap::new();
        for completion in &batch.completions {
            let leased = transaction
                .query_row(
                    "SELECT status, lease_generation, root_id, root_generation, scope,
                            catch_up_source, catch_up_watermark
                     FROM library_change_queue WHERE id = ?1",
                    [sqlite_integer(completion.change_id.value(), "change ID")?],
                    |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, i64>(1)?,
                            row.get::<_, String>(2)?,
                            row.get::<_, i64>(3)?,
                            row.get::<_, String>(4)?,
                            row.get::<_, Option<String>>(5)?,
                            row.get::<_, Option<String>>(6)?,
                        ))
                    },
                )
                .optional()
                .map_err(database_error)?;
            let Some((
                status,
                lease_generation,
                root_id,
                root_generation,
                scope,
                catch_up_source,
                catch_up_watermark,
            )) = leased
            else {
                return Ok(publication(
                    CatalogDeltaPublicationStatus::StaleLease,
                    current_revision,
                ));
            };
            if status != "leased"
                || sqlite_unsigned(lease_generation, "lease generation")?
                    != completion.lease_generation
                || root_id != batch.root_id
                || sqlite_unsigned(root_generation, "root generation")?
                    != batch.root_generation.value()
            {
                return Ok(publication(
                    CatalogDeltaPublicationStatus::StaleLease,
                    current_revision,
                ));
            }
            completed_root_authority |= scope == "root";
            let primary_evidence = match (catch_up_source, catch_up_watermark) {
                (Some(source), Some(watermark)) => {
                    Some(LibraryChangeCatchUpEvidence { source, watermark })
                }
                (None, None) => None,
                _ => {
                    return Err(ScanError::new(
                        "catalog_delta_catch_up_evidence_invalid",
                        "A leased catalog delta contains incomplete catch-up evidence",
                    ));
                }
            };
            let lineage = load_change_catch_up_lineage(
                &transaction,
                completion.change_id,
                primary_evidence.as_ref(),
            )?;
            catch_up_evidence_by_change.insert(completion.change_id, lineage);
        }

        for mutation in &batch.mutations {
            if let Some(expectation) = &mutation.retained_preview_expectation
                && !retained_preview_matches(&transaction, expectation)?
            {
                return Ok(publication(
                    CatalogDeltaPublicationStatus::StalePreviewState,
                    current_revision,
                ));
            }
        }

        let affected_location_ids = batch
            .mutations
            .iter()
            .flat_map(|mutation| {
                mutation.remove_location_ids.iter().chain(
                    mutation
                        .upsert_location
                        .iter()
                        .map(|location| &location.location_id),
                )
            })
            .cloned()
            .collect::<HashSet<_>>();
        let mut affected_asset_ids = HashSet::new();
        let mut affected_artifact_keys = HashSet::new();
        let locations_before = load_affected_state(
            &transaction,
            &active_scan_id,
            &batch.root_id,
            &affected_location_ids,
            &mut affected_asset_ids,
            &mut affected_artifact_keys,
        )?;
        for mutation in &batch.mutations {
            if let Some(location) = &mutation.upsert_location {
                affected_asset_ids.insert(location.asset_id.clone());
            }
        }

        for mutation in &batch.mutations {
            if let Some(lineage) = catch_up_evidence_by_change.get(&mutation.change_id) {
                for evidence in lineage {
                    retain_catch_up_handoff_snapshots(
                        &transaction,
                        &active_scan_id,
                        &batch.root_id,
                        &mutation.remove_location_ids,
                        evidence,
                        completed_unix_ms,
                    )?;
                }
            }
            let mut removals = mutation
                .remove_location_ids
                .iter()
                .map(String::as_str)
                .collect::<HashSet<_>>();
            if let Some(location) = &mutation.upsert_location {
                removals.insert(&location.location_id);
            }
            for location_id in removals {
                transaction
                    .execute(
                        "DELETE FROM preview_artifact_locations
                         WHERE location_id = ?3
                           AND EXISTS (
                             SELECT 1 FROM asset_locations AS locations
                             WHERE locations.scan_id = ?1 AND locations.root_id = ?2
                               AND locations.location_id = preview_artifact_locations.location_id
                           )",
                        params![active_scan_id, batch.root_id, location_id],
                    )
                    .map_err(database_error)?;
                transaction
                    .execute(
                        "DELETE FROM asset_locations
                         WHERE scan_id = ?1 AND root_id = ?2 AND location_id = ?3",
                        params![active_scan_id, batch.root_id, location_id],
                    )
                    .map_err(database_error)?;
            }
            if let Some(location) = &mutation.upsert_location {
                persist_location(&transaction, &active_scan_id, &batch.root_id, location)?;
                if matches!(location.preview_status, PreviewStatus::Ready)
                    && !location.preview_path.is_empty()
                {
                    transaction
                        .execute(
                            "INSERT OR IGNORE INTO preview_artifact_locations(
                               artifact_key, location_id
                             )
                             SELECT artifact_key, ?1 FROM preview_artifacts
                             WHERE artifact_path = ?2 AND lifecycle_state = 'ready'",
                            params![location.location_id, location.preview_path],
                        )
                        .map_err(database_error)?;
                    let owner = transaction
                        .query_row(
                            "SELECT EXISTS(
                               SELECT 1
                               FROM preview_artifact_locations AS owners
                               JOIN preview_artifacts AS artifacts
                                 ON artifacts.artifact_key = owners.artifact_key
                               WHERE owners.location_id = ?1
                                 AND artifacts.artifact_path = ?2
                                 AND artifacts.lifecycle_state = 'ready'
                             )",
                            params![location.location_id, location.preview_path],
                            |row| row.get::<_, bool>(0),
                        )
                        .map_err(database_error)?;
                    if !owner {
                        return Err(ScanError::new(
                            "catalog_delta_preview_owner_missing",
                            "A retained ready preview no longer has a compatible artifact owner",
                        ));
                    }
                }
            }
        }
        mark_affected_preview_artifacts_stale(&transaction, &affected_artifact_keys)?;
        delete_affected_orphan_assets(&transaction, &affected_asset_ids)?;
        let locations_after = count_affected_locations(
            &transaction,
            &active_scan_id,
            &batch.root_id,
            &affected_location_ids,
        )?;
        let location_delta = locations_after - locations_before;
        let updated_scan = transaction
            .execute(
                "UPDATE scan_runs
                 SET asset_count = asset_count + ?3
                 WHERE id = ?1 AND root_id = ?2 AND status = 'completed'
                   AND asset_count + ?3 >= 0",
                params![active_scan_id, batch.root_id, location_delta],
            )
            .map_err(database_error)?;
        if updated_scan != 1 {
            return Err(ScanError::new(
                "catalog_delta_asset_count_invalid",
                "The published scan asset count could not accept the bounded delta",
            ));
        }

        let published_revision = if batch.mutations.is_empty() {
            current_revision
        } else {
            let updated = transaction
                .execute("UPDATE catalog_state SET revision = revision + 1", [])
                .map_err(database_error)?;
            if updated != 1 {
                return Err(ScanError::new(
                    "catalog_revision_unavailable",
                    "The catalog revision state is missing or invalid",
                ));
            }
            current_revision.checked_add(1).ok_or_else(|| {
                ScanError::new(
                    "catalog_revision_overflow",
                    "The catalog revision exceeded the supported range",
                )
            })?
        };
        for completion in &batch.completions {
            let updated = transaction
                .execute(
                    "UPDATE library_change_queue
                     SET status = 'completed', next_retry_unix_ms = NULL,
                         lease_expires_unix_ms = NULL,
                         last_failure_code = ?1, last_failure_message = ?2,
                         catalog_revision_at_success = ?3, updated_unix_ms = ?4
                     WHERE id = ?5 AND status = 'leased' AND lease_generation = ?6",
                    params![
                        completion.issue.as_ref().map(|issue| &issue.code),
                        completion.issue.as_ref().map(|issue| &issue.message),
                        sqlite_integer(published_revision, "catalog revision")?,
                        completed_unix_ms,
                        sqlite_integer(completion.change_id.value(), "change ID")?,
                        sqlite_integer(completion.lease_generation, "lease generation")?,
                    ],
                )
                .map_err(database_error)?;
            if updated != 1 {
                return Err(ScanError::new(
                    "catalog_delta_lease_changed",
                    "A library change lease changed while its catalog delta was publishing",
                ));
            }
        }
        let completed_evidence = catch_up_evidence_by_change
            .into_values()
            .flatten()
            .map(|evidence| (evidence.source, evidence.watermark))
            .collect::<HashSet<_>>();
        for (source, watermark) in completed_evidence {
            cleanup_terminal_catch_up_handoffs(&transaction, &source, &watermark)?;
        }
        if completed_root_authority {
            transaction
                .execute(
                    "UPDATE library_change_root_state
                     SET last_consistency_audit_unix_ms = ?2, updated_unix_ms = ?2
                     WHERE root_id = ?1 AND generation = ?3 AND is_active = 1",
                    params![
                        batch.root_id,
                        completed_unix_ms,
                        sqlite_integer(batch.root_generation.value(), "root generation")?,
                    ],
                )
                .map_err(database_error)?;
        }
        transaction.commit().map_err(database_error)?;
        Ok(CatalogDeltaPublication {
            status: CatalogDeltaPublicationStatus::Applied,
            catalog_revision: published_revision,
            applied_mutation_count: u32::try_from(batch.mutations.len()).map_err(|_| {
                ScanError::new(
                    "catalog_delta_count_overflow",
                    "The catalog delta mutation count exceeded the supported range",
                )
            })?,
            completed_change_count: u32::try_from(batch.completions.len()).map_err(|_| {
                ScanError::new(
                    "catalog_delta_count_overflow",
                    "The completed change count exceeded the supported range",
                )
            })?,
        })
    }
}

fn load_incremental_location<P>(
    catalog: &SqliteCatalog,
    predicate: &str,
    parameters: P,
) -> Result<Option<AssetLocationView>, ScanError>
where
    P: rusqlite::Params,
{
    let query = format!(
        "SELECT locations.asset_id, locations.location_id, locations.root_id,
                locations.absolute_path, locations.relative_path,
                locations.preview_path, locations.file_size,
                locations.created_unix_ms, locations.modified_unix_ms,
                locations.width, locations.height,
                locations.preview_status, locations.preview_issue_code,
                locations.preview_issue_message, locations.metadata_engine_id,
                locations.metadata_engine_version, locations.capture_local_time,
                locations.capture_offset_minutes, locations.capture_time_source,
                locations.capture_raw_value, locations.file_identity_scheme,
                locations.file_identity_value
         FROM library_roots AS roots
         JOIN asset_locations AS locations ON locations.scan_id = roots.active_scan_id
         WHERE {predicate}
         ORDER BY locations.location_id
         LIMIT 1"
    );
    catalog
        .connection
        .query_row(&query, parameters, read_stored_asset)
        .optional()
        .map_err(database_error)?
        .map(stored_asset_view)
        .transpose()
}

pub(super) fn load_scan_location_by_file_identity(
    catalog: &SqliteCatalog,
    scan_id: &str,
    identity: &FileIdentityEvidence,
) -> Result<Option<AssetLocationView>, ScanError> {
    let active = load_incremental_location(
        catalog,
        "locations.file_identity_scheme = ?1 AND locations.file_identity_value = ?2",
        params![identity.scheme, identity.value],
    )?;
    if active.is_some() {
        return Ok(active);
    }
    let normalized = catalog
        .connection
        .query_row(
            "SELECT items.asset_id, items.source_location_id, items.root_id,
                    items.absolute_path, items.relative_path, items.preview_path,
                    items.file_size, items.created_unix_ms, items.modified_unix_ms,
                    items.width, items.height, items.preview_status,
                    items.preview_issue_code, items.preview_issue_message,
                    items.metadata_engine_id, items.metadata_engine_version,
                    items.capture_local_time, items.capture_offset_minutes,
                    items.capture_time_source, items.capture_raw_value,
                    items.file_identity_scheme, items.file_identity_value
             FROM scan_run_catch_up_lineage AS current_lineage
             JOIN library_change_scan_handoff_lineage AS handoff_lineage
               ON handoff_lineage.catch_up_source = current_lineage.catch_up_source
              AND handoff_lineage.catch_up_watermark = current_lineage.catch_up_watermark
             JOIN library_change_scan_handoff_batches AS batches
               ON batches.id = handoff_lineage.batch_id
             JOIN library_change_scan_handoff_items AS items
               ON items.batch_id = batches.id
             WHERE current_lineage.scan_id = ?1
               AND items.file_identity_scheme = ?2
               AND items.file_identity_value = ?3
             ORDER BY current_lineage.enrolled_unix_ms DESC,
                      batches.updated_unix_ms DESC, batches.id
             LIMIT 1",
            params![scan_id, identity.scheme, identity.value],
            read_stored_asset,
        )
        .optional()
        .map_err(database_error)?
        .map(stored_asset_view)
        .transpose()?;
    if normalized.is_some() {
        return Ok(normalized);
    }
    catalog
        .connection
        .query_row(
            "SELECT handoffs.asset_id, handoffs.source_location_id, handoffs.root_id,
                    handoffs.absolute_path, handoffs.relative_path, handoffs.preview_path,
                    handoffs.file_size, handoffs.created_unix_ms,
                    handoffs.modified_unix_ms, handoffs.width, handoffs.height,
                    handoffs.preview_status, handoffs.preview_issue_code,
                    handoffs.preview_issue_message, handoffs.metadata_engine_id,
                    handoffs.metadata_engine_version, handoffs.capture_local_time,
                    handoffs.capture_offset_minutes, handoffs.capture_time_source,
                    handoffs.capture_raw_value, handoffs.file_identity_scheme,
                    handoffs.file_identity_value
             FROM scan_run_catch_up_lineage AS lineage
             JOIN library_change_catch_up_handoffs AS handoffs
               ON handoffs.catch_up_source = lineage.catch_up_source
              AND handoffs.catch_up_watermark = lineage.catch_up_watermark
             WHERE lineage.scan_id = ?1
               AND handoffs.file_identity_scheme = ?2
               AND handoffs.file_identity_value = ?3
             ORDER BY lineage.enrolled_unix_ms DESC, handoffs.updated_unix_ms DESC,
                      lineage.catch_up_source, lineage.catch_up_watermark
             LIMIT 1",
            params![scan_id, identity.scheme, identity.value],
            read_stored_asset,
        )
        .optional()
        .map_err(database_error)?
        .map(stored_asset_view)
        .transpose()
}

fn load_catch_up_handoff_location(
    catalog: &SqliteCatalog,
    identity: &FileIdentityEvidence,
    evidence: &LibraryChangeCatchUpEvidence,
) -> Result<Option<AssetLocationView>, ScanError> {
    let normalized = catalog
        .connection
        .query_row(
            "SELECT items.asset_id, items.source_location_id, items.root_id,
                    items.absolute_path, items.relative_path, items.preview_path,
                    items.file_size, items.created_unix_ms, items.modified_unix_ms,
                    items.width, items.height, items.preview_status,
                    items.preview_issue_code, items.preview_issue_message,
                    items.metadata_engine_id, items.metadata_engine_version,
                    items.capture_local_time, items.capture_offset_minutes,
                    items.capture_time_source, items.capture_raw_value,
                    items.file_identity_scheme, items.file_identity_value
             FROM library_change_scan_handoff_lineage AS lineage
             JOIN library_change_scan_handoff_batches AS batches ON batches.id = lineage.batch_id
             JOIN library_change_scan_handoff_items AS items ON items.batch_id = batches.id
             WHERE lineage.catch_up_source = ?1 AND lineage.catch_up_watermark = ?2
               AND items.file_identity_scheme = ?3 AND items.file_identity_value = ?4
             ORDER BY lineage.enrolled_unix_ms DESC, batches.updated_unix_ms DESC, batches.id
             LIMIT 1",
            params![
                evidence.source,
                evidence.watermark,
                identity.scheme,
                identity.value,
            ],
            read_stored_asset,
        )
        .optional()
        .map_err(database_error)?
        .map(stored_asset_view)
        .transpose()?;
    if normalized.is_some() {
        return Ok(normalized);
    }
    catalog
        .connection
        .query_row(
            "SELECT asset_id, source_location_id, root_id, absolute_path, relative_path,
                    preview_path, file_size, created_unix_ms, modified_unix_ms,
                    width, height, preview_status, preview_issue_code,
                    preview_issue_message, metadata_engine_id, metadata_engine_version,
                    capture_local_time, capture_offset_minutes, capture_time_source,
                    capture_raw_value, file_identity_scheme, file_identity_value
             FROM library_change_catch_up_handoffs
             WHERE catch_up_source = ?1 AND catch_up_watermark = ?2
               AND file_identity_scheme = ?3 AND file_identity_value = ?4",
            params![
                evidence.source,
                evidence.watermark,
                identity.scheme,
                identity.value,
            ],
            read_stored_asset,
        )
        .optional()
        .map_err(database_error)?
        .map(stored_asset_view)
        .transpose()
}

fn load_change_catch_up_lineage(
    transaction: &rusqlite::Transaction<'_>,
    change_id: LibraryChangeId,
    primary_evidence: Option<&LibraryChangeCatchUpEvidence>,
) -> Result<Vec<LibraryChangeCatchUpEvidence>, ScanError> {
    let mut statement = transaction
        .prepare_cached(
            "SELECT catch_up_source, catch_up_watermark
             FROM library_change_queue_catch_up_lineage
             WHERE change_id = ?1
             ORDER BY enrolled_unix_ms DESC, catch_up_source, catch_up_watermark
             LIMIT ?2",
        )
        .map_err(database_error)?;
    let rows = statement
        .query_map(
            params![
                sqlite_integer(change_id.value(), "change ID")?,
                i64::try_from(MAX_CATCH_UP_LINEAGE_PER_CHANGE + 1).unwrap_or(i64::MAX),
            ],
            |row| {
                Ok(LibraryChangeCatchUpEvidence {
                    source: row.get(0)?,
                    watermark: row.get(1)?,
                })
            },
        )
        .map_err(database_error)?;
    let mut lineage = Vec::new();
    for row in rows {
        lineage.push(row.map_err(database_error)?);
    }
    if lineage.len() > MAX_CATCH_UP_LINEAGE_PER_CHANGE
        || primary_evidence.is_some_and(|evidence| !lineage.contains(evidence))
        || (primary_evidence.is_none() && !lineage.is_empty())
    {
        return Err(ScanError::new(
            "catalog_delta_catch_up_lineage_invalid",
            "A catalog delta lease has invalid or unbounded catch-up watermark lineage",
        ));
    }
    Ok(lineage)
}

fn retained_preview_matches(
    transaction: &rusqlite::Transaction<'_>,
    expectation: &RetainedPreviewExpectation,
) -> Result<bool, ScanError> {
    transaction
        .query_row(
            "SELECT EXISTS(
               SELECT 1
               FROM asset_locations AS locations
               JOIN library_roots AS roots ON roots.active_scan_id = locations.scan_id
               WHERE locations.location_id = ?1
                 AND locations.preview_path = ?2
                 AND locations.preview_status = ?3
                 AND locations.preview_issue_code IS ?4
                 AND locations.preview_issue_message IS ?5
               UNION ALL
               SELECT 1 FROM library_change_catch_up_handoffs AS handoffs
               WHERE handoffs.source_location_id = ?1
                 AND handoffs.preview_path = ?2
                 AND handoffs.preview_status = ?3
                  AND handoffs.preview_issue_code IS ?4
                  AND handoffs.preview_issue_message IS ?5
               UNION ALL
               SELECT 1 FROM library_change_scan_handoff_items AS handoffs
               WHERE handoffs.source_location_id = ?1
                 AND handoffs.preview_path = ?2
                 AND handoffs.preview_status = ?3
                 AND handoffs.preview_issue_code IS ?4
                 AND handoffs.preview_issue_message IS ?5
             )",
            params![
                expectation.location_id,
                expectation.preview_path,
                preview_status_text(&expectation.preview_status),
                expectation.preview_issue_code,
                expectation.preview_issue_message,
            ],
            |row| row.get::<_, bool>(0),
        )
        .map_err(database_error)
}

fn retain_catch_up_handoff_snapshots(
    transaction: &rusqlite::Transaction<'_>,
    scan_id: &str,
    root_id: &str,
    location_ids: &[String],
    evidence: &LibraryChangeCatchUpEvidence,
    updated_unix_ms: i64,
) -> Result<(), ScanError> {
    for location_id in location_ids {
        transaction
            .execute(
                "INSERT INTO library_change_catch_up_handoffs(
                   catch_up_source, catch_up_watermark,
                   file_identity_scheme, file_identity_value,
                   asset_id, source_location_id, root_id, absolute_path, relative_path,
                   preview_path, file_size, created_unix_ms, modified_unix_ms,
                   width, height, preview_status, preview_issue_code, preview_issue_message,
                   metadata_engine_id, metadata_engine_version, capture_local_time,
                   capture_offset_minutes, capture_time_source, capture_raw_value,
                   updated_unix_ms
                 )
                 SELECT ?1, ?2, locations.file_identity_scheme, locations.file_identity_value,
                        locations.asset_id, locations.location_id, locations.root_id,
                        locations.absolute_path, locations.relative_path,
                        locations.preview_path, locations.file_size, locations.created_unix_ms,
                        locations.modified_unix_ms, locations.width, locations.height,
                        locations.preview_status, locations.preview_issue_code,
                        locations.preview_issue_message, locations.metadata_engine_id,
                        locations.metadata_engine_version, locations.capture_local_time,
                        locations.capture_offset_minutes, locations.capture_time_source,
                        locations.capture_raw_value, ?6
                 FROM asset_locations AS locations
                 WHERE locations.scan_id = ?3 AND locations.root_id = ?4
                   AND locations.location_id = ?5
                   AND locations.file_identity_scheme IS NOT NULL
                   AND locations.file_identity_value IS NOT NULL
                 ON CONFLICT(
                   catch_up_source, catch_up_watermark,
                   file_identity_scheme, file_identity_value
                 ) DO NOTHING",
                params![
                    evidence.source,
                    evidence.watermark,
                    scan_id,
                    root_id,
                    location_id,
                    updated_unix_ms,
                ],
            )
            .map_err(database_error)?;
    }
    Ok(())
}

pub(super) fn retain_scan_handoff_snapshots(
    transaction: &rusqlite::Transaction<'_>,
    scan_id: &str,
    previous_scan_id: &str,
    root_id: &str,
    updated_unix_ms: i64,
) -> Result<(), ScanError> {
    transaction
        .execute(
            "INSERT INTO library_change_scan_handoff_batches(
               id, source_root_id, updated_unix_ms
             )
             SELECT ?1, ?3, ?4
             WHERE EXISTS (
               SELECT 1 FROM scan_run_catch_up_lineage AS lineage
               WHERE lineage.scan_id = ?1
                 AND (
                   EXISTS (
                     SELECT 1
                     FROM library_change_queue_catch_up_lineage AS peer_lineage
                     JOIN library_change_queue AS peer ON peer.id = peer_lineage.change_id
                     WHERE peer_lineage.catch_up_source = lineage.catch_up_source
                       AND peer_lineage.catch_up_watermark = lineage.catch_up_watermark
                       AND peer.root_id <> ?3
                       AND peer.status IN ('pending', 'leased', 'retry_wait')
                   ) OR EXISTS (
                     SELECT 1
                     FROM scan_run_catch_up_lineage AS peer_lineage
                     JOIN scan_runs AS peer_scan ON peer_scan.id = peer_lineage.scan_id
                     WHERE peer_lineage.catch_up_source = lineage.catch_up_source
                       AND peer_lineage.catch_up_watermark = lineage.catch_up_watermark
                       AND peer_scan.root_id <> ?3
                       AND peer_scan.status IN ('running', 'paused')
                   )
                 )
             ) AND EXISTS (
               SELECT 1 FROM asset_locations AS locations
               WHERE locations.scan_id = ?2 AND locations.root_id = ?3
                 AND locations.file_identity_scheme IS NOT NULL
                 AND locations.file_identity_value IS NOT NULL
                 AND NOT EXISTS (
                   SELECT 1 FROM asset_locations AS replacement
                   WHERE replacement.scan_id = ?1
                     AND replacement.file_identity_scheme = locations.file_identity_scheme
                     AND replacement.file_identity_value = locations.file_identity_value
                 )
             )",
            params![scan_id, previous_scan_id, root_id, updated_unix_ms],
        )
        .map_err(database_error)?;
    transaction
        .execute(
            "INSERT INTO library_change_scan_handoff_lineage(
               batch_id, catch_up_source, catch_up_watermark, enrolled_unix_ms
             )
             SELECT ?1, lineage.catch_up_source, lineage.catch_up_watermark,
                    lineage.enrolled_unix_ms
             FROM scan_run_catch_up_lineage AS lineage
             WHERE lineage.scan_id = ?1
               AND EXISTS (
                 SELECT 1 FROM library_change_scan_handoff_batches WHERE id = ?1
               )
               AND (
                 EXISTS (
                   SELECT 1
                   FROM library_change_queue_catch_up_lineage AS peer_lineage
                   JOIN library_change_queue AS peer ON peer.id = peer_lineage.change_id
                   WHERE peer_lineage.catch_up_source = lineage.catch_up_source
                     AND peer_lineage.catch_up_watermark = lineage.catch_up_watermark
                     AND peer.root_id <> ?2
                     AND peer.status IN ('pending', 'leased', 'retry_wait')
                 ) OR EXISTS (
                   SELECT 1
                   FROM scan_run_catch_up_lineage AS peer_lineage
                   JOIN scan_runs AS peer_scan ON peer_scan.id = peer_lineage.scan_id
                   WHERE peer_lineage.catch_up_source = lineage.catch_up_source
                     AND peer_lineage.catch_up_watermark = lineage.catch_up_watermark
                     AND peer_scan.root_id <> ?2
                     AND peer_scan.status IN ('running', 'paused')
                 )
               )",
            params![scan_id, root_id],
        )
        .map_err(database_error)?;
    transaction
        .execute(
            "INSERT INTO library_change_scan_handoff_items(
               batch_id, file_identity_scheme, file_identity_value,
               asset_id, source_location_id, root_id, absolute_path, relative_path,
               preview_path, file_size, created_unix_ms, modified_unix_ms,
               width, height, preview_status, preview_issue_code, preview_issue_message,
               metadata_engine_id, metadata_engine_version, capture_local_time,
               capture_offset_minutes, capture_time_source, capture_raw_value
             )
             SELECT ?1, locations.file_identity_scheme, locations.file_identity_value,
                    locations.asset_id, locations.location_id, locations.root_id,
                    locations.absolute_path, locations.relative_path,
                    locations.preview_path, locations.file_size, locations.created_unix_ms,
                    locations.modified_unix_ms, locations.width, locations.height,
                    locations.preview_status, locations.preview_issue_code,
                    locations.preview_issue_message, locations.metadata_engine_id,
                    locations.metadata_engine_version, locations.capture_local_time,
                    locations.capture_offset_minutes, locations.capture_time_source,
                    locations.capture_raw_value
             FROM asset_locations AS locations
             WHERE locations.scan_id = ?2 AND locations.root_id = ?3
               AND EXISTS (
                 SELECT 1 FROM library_change_scan_handoff_batches WHERE id = ?1
               )
               AND locations.file_identity_scheme IS NOT NULL
               AND locations.file_identity_value IS NOT NULL
               AND NOT EXISTS (
                 SELECT 1 FROM asset_locations AS replacement
                 WHERE replacement.scan_id = ?1
                   AND replacement.file_identity_scheme = locations.file_identity_scheme
                   AND replacement.file_identity_value = locations.file_identity_value
               )",
            params![scan_id, previous_scan_id, root_id],
        )
        .map_err(database_error)?;
    Ok(())
}

pub(super) fn cleanup_terminal_catch_up_handoffs(
    transaction: &rusqlite::Transaction<'_>,
    source: &str,
    watermark: &str,
) -> Result<(), ScanError> {
    cleanup_terminal_catch_up_handoffs_batch(
        transaction,
        &[(source.to_owned(), watermark.to_owned())],
    )
}

pub(super) fn cleanup_terminal_catch_up_handoffs_batch(
    transaction: &rusqlite::Transaction<'_>,
    evidence: &[(String, String)],
) -> Result<(), ScanError> {
    let mut removed_owner = false;
    for (source, watermark) in evidence {
        let has_active_work = transaction
            .query_row(
                "SELECT EXISTS(
                   SELECT 1
                   FROM library_change_queue_catch_up_lineage AS lineage
                   JOIN library_change_queue AS changes ON changes.id = lineage.change_id
                   WHERE lineage.catch_up_source = ?1 AND lineage.catch_up_watermark = ?2
                     AND changes.status IN ('pending', 'leased', 'retry_wait')
                   UNION ALL
                   SELECT 1
                   FROM scan_run_catch_up_lineage AS lineage
                   JOIN scan_runs AS scans ON scans.id = lineage.scan_id
                   WHERE lineage.catch_up_source = ?1 AND lineage.catch_up_watermark = ?2
                     AND scans.status IN ('running', 'paused')
                 )",
                params![source, watermark],
                |row| row.get::<_, bool>(0),
            )
            .map_err(database_error)?;
        if !has_active_work {
            removed_owner |= release_catch_up_handoff_evidence(transaction, source, watermark)?;
        }
    }
    if removed_owner {
        super::mark_unreferenced_preview_artifacts_stale(transaction)?;
        super::delete_orphan_assets(transaction)?;
    }
    Ok(())
}

pub(super) fn cleanup_obsolete_catch_up_handoffs(
    transaction: &rusqlite::Transaction<'_>,
    updated_before_unix_ms: i64,
    limit: u32,
) -> Result<(), ScanError> {
    let evidence = {
        let mut statement = transaction
            .prepare(
                "SELECT handoffs.catch_up_source, handoffs.catch_up_watermark
                 FROM (
                   SELECT catch_up_source, catch_up_watermark, updated_unix_ms
                   FROM library_change_catch_up_handoffs
                   UNION ALL
                   SELECT lineage.catch_up_source, lineage.catch_up_watermark,
                          batches.updated_unix_ms
                   FROM library_change_scan_handoff_lineage AS lineage
                   JOIN library_change_scan_handoff_batches AS batches
                     ON batches.id = lineage.batch_id
                 ) AS handoffs
                 WHERE handoffs.updated_unix_ms < ?1
                   AND NOT EXISTS (
                     SELECT 1
                     FROM library_change_queue_catch_up_lineage AS lineage
                     JOIN library_change_queue AS changes ON changes.id = lineage.change_id
                     WHERE lineage.catch_up_source = handoffs.catch_up_source
                       AND lineage.catch_up_watermark = handoffs.catch_up_watermark
                       AND changes.status IN ('pending', 'leased', 'retry_wait')
                   )
                   AND NOT EXISTS (
                     SELECT 1
                     FROM scan_run_catch_up_lineage AS lineage
                     JOIN scan_runs AS scans ON scans.id = lineage.scan_id
                     WHERE lineage.catch_up_source = handoffs.catch_up_source
                       AND lineage.catch_up_watermark = handoffs.catch_up_watermark
                       AND scans.status IN ('running', 'paused')
                   )
                 GROUP BY handoffs.catch_up_source, handoffs.catch_up_watermark
                 ORDER BY MIN(handoffs.updated_unix_ms),
                          handoffs.catch_up_source, handoffs.catch_up_watermark
                 LIMIT ?2",
            )
            .map_err(database_error)?;
        let rows = statement
            .query_map(params![updated_before_unix_ms, i64::from(limit)], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(database_error)?;
        let mut evidence = Vec::new();
        for row in rows {
            evidence.push(row.map_err(database_error)?);
        }
        evidence
    };
    cleanup_terminal_catch_up_handoffs_batch(transaction, &evidence)
}

fn release_catch_up_handoff_evidence(
    transaction: &rusqlite::Transaction<'_>,
    source: &str,
    watermark: &str,
) -> Result<bool, ScanError> {
    let batch_ids = {
        let mut statement = transaction
            .prepare(
                "SELECT batch_id FROM library_change_scan_handoff_lineage
                 WHERE catch_up_source = ?1 AND catch_up_watermark = ?2",
            )
            .map_err(database_error)?;
        let rows = statement
            .query_map(params![source, watermark], |row| row.get::<_, String>(0))
            .map_err(database_error)?;
        let mut batch_ids = Vec::new();
        for row in rows {
            batch_ids.push(row.map_err(database_error)?);
        }
        batch_ids
    };
    let removed_legacy = transaction
        .execute(
            "DELETE FROM library_change_catch_up_handoffs
             WHERE catch_up_source = ?1 AND catch_up_watermark = ?2",
            params![source, watermark],
        )
        .map_err(database_error)?;
    transaction
        .execute(
            "DELETE FROM library_change_scan_handoff_lineage
             WHERE catch_up_source = ?1 AND catch_up_watermark = ?2",
            params![source, watermark],
        )
        .map_err(database_error)?;
    let mut removed_batches = 0_usize;
    for batch_id in batch_ids {
        removed_batches = removed_batches.saturating_add(
            transaction
                .execute(
                    "DELETE FROM library_change_scan_handoff_batches
                     WHERE id = ?1 AND NOT EXISTS (
                       SELECT 1 FROM library_change_scan_handoff_lineage
                       WHERE batch_id = ?1
                     )",
                    [batch_id],
                )
                .map_err(database_error)?,
        );
    }
    Ok(removed_legacy > 0 || removed_batches > 0)
}

fn load_affected_state(
    transaction: &rusqlite::Transaction<'_>,
    scan_id: &str,
    root_id: &str,
    location_ids: &HashSet<String>,
    asset_ids: &mut HashSet<String>,
    artifact_keys: &mut HashSet<String>,
) -> Result<i64, ScanError> {
    let mut count = 0_i64;
    for location_id in location_ids {
        let asset_id = transaction
            .query_row(
                "SELECT asset_id FROM asset_locations
                 WHERE scan_id = ?1 AND root_id = ?2 AND location_id = ?3",
                params![scan_id, root_id, location_id],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(database_error)?;
        if let Some(asset_id) = asset_id {
            count = count.checked_add(1).ok_or_else(|| {
                ScanError::new(
                    "catalog_delta_location_count_overflow",
                    "The affected location count exceeded the supported range",
                )
            })?;
            asset_ids.insert(asset_id);
        }
        let mut statement = transaction
            .prepare_cached(
                "SELECT owners.artifact_key
                 FROM preview_artifact_locations AS owners
                 JOIN asset_locations AS locations
                   ON locations.location_id = owners.location_id
                 WHERE locations.scan_id = ?1 AND locations.root_id = ?2
                   AND locations.location_id = ?3",
            )
            .map_err(database_error)?;
        let rows = statement
            .query_map(params![scan_id, root_id, location_id], |row| {
                row.get::<_, String>(0)
            })
            .map_err(database_error)?;
        for row in rows {
            artifact_keys.insert(row.map_err(database_error)?);
        }
    }
    Ok(count)
}

fn count_affected_locations(
    transaction: &rusqlite::Transaction<'_>,
    scan_id: &str,
    root_id: &str,
    location_ids: &HashSet<String>,
) -> Result<i64, ScanError> {
    let mut count = 0_i64;
    for location_id in location_ids {
        let exists = transaction
            .query_row(
                "SELECT EXISTS(
                   SELECT 1 FROM asset_locations
                   WHERE scan_id = ?1 AND root_id = ?2 AND location_id = ?3
                 )",
                params![scan_id, root_id, location_id],
                |row| row.get::<_, bool>(0),
            )
            .map_err(database_error)?;
        if exists {
            count = count.checked_add(1).ok_or_else(|| {
                ScanError::new(
                    "catalog_delta_location_count_overflow",
                    "The affected location count exceeded the supported range",
                )
            })?;
        }
    }
    Ok(count)
}

fn mark_affected_preview_artifacts_stale(
    transaction: &rusqlite::Transaction<'_>,
    artifact_keys: &HashSet<String>,
) -> Result<(), ScanError> {
    for artifact_key in artifact_keys {
        transaction
            .execute(
                "UPDATE preview_artifacts
                 SET lifecycle_state = 'stale'
                 WHERE artifact_key = ?1 AND lifecycle_state = 'ready'
                   AND NOT EXISTS (
                     SELECT 1 FROM preview_artifact_locations AS owners
                     WHERE owners.artifact_key = preview_artifacts.artifact_key
                   )
                    AND NOT EXISTS (
                      SELECT 1 FROM library_change_catch_up_handoffs AS handoffs
                      WHERE handoffs.preview_status = 'ready'
                        AND handoffs.preview_path = preview_artifacts.artifact_path
                    )
                    AND NOT EXISTS (
                      SELECT 1 FROM library_change_scan_handoff_items AS handoffs
                      WHERE handoffs.preview_status = 'ready'
                        AND handoffs.preview_path = preview_artifacts.artifact_path
                    )",
                [artifact_key],
            )
            .map_err(database_error)?;
    }
    Ok(())
}

fn delete_affected_orphan_assets(
    transaction: &rusqlite::Transaction<'_>,
    asset_ids: &HashSet<String>,
) -> Result<(), ScanError> {
    for asset_id in asset_ids {
        transaction
            .execute(
                "DELETE FROM assets
                 WHERE id = ?1 AND NOT EXISTS (
                   SELECT 1 FROM asset_locations WHERE asset_locations.asset_id = assets.id
                  ) AND NOT EXISTS (
                    SELECT 1 FROM library_change_catch_up_handoffs AS handoffs
                    WHERE handoffs.asset_id = assets.id
                  ) AND NOT EXISTS (
                    SELECT 1 FROM library_change_scan_handoff_items AS handoffs
                    WHERE handoffs.asset_id = assets.id
                  )",
                [asset_id],
            )
            .map_err(database_error)?;
    }
    Ok(())
}

fn preview_status_text(status: &PreviewStatus) -> &'static str {
    match status {
        PreviewStatus::Pending => "pending",
        PreviewStatus::Ready => "ready",
        PreviewStatus::Failed => "failed",
    }
}

fn validate_root_id(root_id: &str) -> Result<(), ScanError> {
    if root_id.trim().is_empty() || root_id.contains('\0') {
        return Err(ScanError::new(
            "catalog_root_id_invalid",
            "The incremental catalog root ID must be non-empty and contain no NUL bytes",
        ));
    }
    Ok(())
}

fn validate_delta_batch(batch: &CatalogDeltaBatch) -> Result<(), ScanError> {
    validate_root_id(&batch.root_id)?;
    if batch.completions.is_empty() || batch.completions.len() > MAX_DELTA_COMPLETIONS {
        return Err(ScanError::new(
            "catalog_delta_completion_count_invalid",
            "A catalog delta must complete one bounded lease batch",
        ));
    }
    if batch.mutations.len() > MAX_DELTA_MUTATIONS {
        return Err(ScanError::new(
            "catalog_delta_mutation_count_invalid",
            "A catalog delta exceeded the bounded mutation count",
        ));
    }
    let mut change_ids = HashSet::new();
    for completion in &batch.completions {
        if completion.lease_generation == 0 || !change_ids.insert(completion.change_id) {
            return Err(ScanError::new(
                "catalog_delta_completion_invalid",
                "Catalog delta completions require unique nonzero leases",
            ));
        }
        if completion.issue.as_ref().is_some_and(|issue| {
            issue.code.trim().is_empty()
                || issue.message.trim().is_empty()
                || issue.code.contains('\0')
                || issue.message.contains('\0')
        }) {
            return Err(ScanError::new(
                "catalog_delta_issue_invalid",
                "A completed catalog issue must be non-empty and contain no NUL bytes",
            ));
        }
    }
    for mutation in &batch.mutations {
        if !change_ids.contains(&mutation.change_id) {
            return Err(ScanError::new(
                "catalog_delta_mutation_change_invalid",
                "Every catalog mutation must belong to a completed lease in the same batch",
            ));
        }
        let valid_evidence_contract = match mutation.outcome {
            IncrementalReconciliationOutcome::Added
            | IncrementalReconciliationOutcome::Replaced => {
                mutation.evidence_disposition == DerivedEvidenceDisposition::NoReusableEvidence
                    && mutation.upsert_location.is_some()
            }
            IncrementalReconciliationOutcome::Modified => {
                mutation.evidence_disposition == DerivedEvidenceDisposition::InvalidateDerived
                    && mutation.upsert_location.is_some()
            }
            IncrementalReconciliationOutcome::RenamedOrMoved => {
                matches!(
                    mutation.evidence_disposition,
                    DerivedEvidenceDisposition::RetainCompatible
                        | DerivedEvidenceDisposition::InvalidateDerived
                ) && mutation.upsert_location.is_some()
            }
            IncrementalReconciliationOutcome::Unchanged => {
                mutation.evidence_disposition == DerivedEvidenceDisposition::RetainCompatible
                    && mutation.upsert_location.is_some()
            }
            IncrementalReconciliationOutcome::Removed => {
                mutation.evidence_disposition
                    == DerivedEvidenceDisposition::RemoveFromCurrentProjection
                    && mutation.upsert_location.is_none()
                    && !mutation.remove_location_ids.is_empty()
            }
            IncrementalReconciliationOutcome::Skipped
            | IncrementalReconciliationOutcome::RetryableFailure
            | IncrementalReconciliationOutcome::TerminalIssue => false,
        };
        if !valid_evidence_contract {
            return Err(ScanError::new(
                "catalog_delta_evidence_contract_invalid",
                "A catalog delta mutation has inconsistent reconciliation evidence",
            ));
        }
        let retains_preview =
            mutation.evidence_disposition == DerivedEvidenceDisposition::RetainCompatible;
        if retains_preview != mutation.retained_preview_expectation.is_some()
            || mutation
                .retained_preview_expectation
                .as_ref()
                .is_some_and(|expectation| expectation.location_id.trim().is_empty())
        {
            return Err(ScanError::new(
                "catalog_delta_preview_expectation_invalid",
                "Retained preview evidence requires one non-empty prior-state expectation",
            ));
        }
        if mutation.remove_location_ids.len() > MAX_REMOVALS_PER_MUTATION
            || mutation
                .remove_location_ids
                .iter()
                .any(|location_id| location_id.trim().is_empty() || location_id.contains('\0'))
        {
            return Err(ScanError::new(
                "catalog_delta_removal_invalid",
                "A catalog delta mutation contains an invalid or unbounded removal set",
            ));
        }
        if mutation.upsert_location.as_ref().is_some_and(|location| {
            location.root_id != batch.root_id
                || location.location_id.trim().is_empty()
                || location.relative_path.trim().is_empty()
        }) {
            return Err(ScanError::new(
                "catalog_delta_location_invalid",
                "A catalog delta upsert must belong to the batch root and contain stable paths",
            ));
        }
        if mutation.remove_location_ids.is_empty() && mutation.upsert_location.is_none() {
            return Err(ScanError::new(
                "catalog_delta_mutation_empty",
                "A catalog delta mutation must remove or upsert at least one location",
            ));
        }
    }
    Ok(())
}

fn publication(
    status: CatalogDeltaPublicationStatus,
    catalog_revision: u64,
) -> CatalogDeltaPublication {
    CatalogDeltaPublication {
        status,
        catalog_revision,
        applied_mutation_count: 0,
        completed_change_count: 0,
    }
}

#[cfg(test)]
mod tests;
