use std::collections::HashSet;

use rusqlite::{OptionalExtension, TransactionBehavior, params};

use crate::domain::{
    AssetLocationView, CatalogDeltaBatch, CatalogDeltaPublication, CatalogDeltaPublicationStatus,
    DerivedEvidenceDisposition, FileIdentityEvidence, IncrementalCatalogRoot,
    IncrementalReconciliationOutcome, LibraryRootGeneration, PreviewStatus, ScanError,
};
use crate::ports::IncrementalCatalogRepository;

use super::{
    SqliteCatalog, database_error, delete_orphan_assets, load_catalog_revision,
    mark_unreferenced_preview_artifacts_stale, persist_location, read_stored_asset, sqlite_integer,
    sqlite_unsigned, stored_asset_view,
};

const MAX_DELTA_MUTATIONS: usize = 256;
const MAX_REMOVALS_PER_MUTATION: usize = 4;
const MAX_DELTA_COMPLETIONS: usize = 128;

impl IncrementalCatalogRepository for SqliteCatalog {
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
                        ), catalog.revision
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
                    ))
                },
            )
            .optional()
            .map_err(database_error)?;
        let Some((root_path, active_scan_id, generation, is_active, has_running_scan, revision)) =
            stored
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
        load_incremental_location(
            self,
            "locations.file_identity_scheme = ?1 AND locations.file_identity_value = ?2",
            params![identity.scheme, identity.value],
        )
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
        for completion in &batch.completions {
            let leased = transaction
                .query_row(
                    "SELECT status, lease_generation, root_id, root_generation
                     FROM library_change_queue WHERE id = ?1",
                    [sqlite_integer(completion.change_id.value(), "change ID")?],
                    |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, i64>(1)?,
                            row.get::<_, String>(2)?,
                            row.get::<_, i64>(3)?,
                        ))
                    },
                )
                .optional()
                .map_err(database_error)?;
            let Some((status, lease_generation, root_id, root_generation)) = leased else {
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
        }

        for mutation in &batch.mutations {
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
                }
            }
        }
        mark_unreferenced_preview_artifacts_stale(&transaction)?;
        delete_orphan_assets(&transaction)?;
        transaction
            .execute(
                "UPDATE scan_runs
                 SET asset_count = (
                   SELECT COUNT(*) FROM asset_locations
                   WHERE scan_id = ?1 AND root_id = ?2
                 )
                 WHERE id = ?1 AND root_id = ?2 AND status = 'completed'",
                params![active_scan_id, batch.root_id],
            )
            .map_err(database_error)?;

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
            IncrementalReconciliationOutcome::Removed => {
                mutation.evidence_disposition
                    == DerivedEvidenceDisposition::RemoveFromCurrentProjection
                    && mutation.upsert_location.is_none()
                    && !mutation.remove_location_ids.is_empty()
            }
            IncrementalReconciliationOutcome::Unchanged
            | IncrementalReconciliationOutcome::Skipped
            | IncrementalReconciliationOutcome::RetryableFailure
            | IncrementalReconciliationOutcome::TerminalIssue => false,
        };
        if !valid_evidence_contract {
            return Err(ScanError::new(
                "catalog_delta_evidence_contract_invalid",
                "A catalog delta mutation has inconsistent reconciliation evidence",
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
