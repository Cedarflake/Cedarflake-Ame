use std::time::Duration;

use rusqlite::{Transaction, TransactionBehavior, params};

use crate::domain::{AssetLocationView, LibraryChangeLane, PreviewReclamationCandidate, ScanError};
use crate::ports::{PreviewHealthObservation, PreviewHealthOutcome, PreviewHealthTarget};

use super::{
    SQLITE_BUSY_TIMEOUT, SqliteCatalog, database_error, source_revision_token, sqlite_integer,
};

impl SqliteCatalog {
    pub(super) fn try_reconcile_preview_health_owned(
        &mut self,
        target: PreviewHealthTarget<'_>,
        observation: PreviewHealthObservation,
    ) -> Result<PreviewHealthOutcome, ScanError> {
        let Some(_permit) = self
            .write_admission
            .try_acquire(LibraryChangeLane::Recovery)
        else {
            return Ok(PreviewHealthOutcome::Deferred);
        };
        // The caller holds preview exclusion. Never wait for another catalog writer here.
        self.connection
            .busy_timeout(Duration::ZERO)
            .map_err(database_error)?;
        let result: Result<PreviewHealthOutcome, ScanError> = (|| {
            let transaction = self
                .connection
                .transaction_with_behavior(TransactionBehavior::Immediate)
                .map_err(database_error)?;
            let outcome = match target {
                PreviewHealthTarget::Artifact(candidate) => {
                    reconcile_artifact(&transaction, candidate, observation)?
                }
                PreviewHealthTarget::Location(location) => match observation {
                    PreviewHealthObservation::Missing => {
                        invalidate_location(&transaction, location)?
                    }
                    PreviewHealthObservation::File { .. } => PreviewHealthOutcome::Unchanged,
                },
            };
            transaction.commit().map_err(database_error)?;
            Ok(outcome)
        })();
        self.connection
            .busy_timeout(SQLITE_BUSY_TIMEOUT)
            .map_err(database_error)?;
        match result {
            Err(error)
                if matches!(
                    error.code.as_str(),
                    "catalog_database_busy" | "catalog_database_locked"
                ) =>
            {
                Ok(PreviewHealthOutcome::Deferred)
            }
            result => result,
        }
    }
}

fn reconcile_artifact(
    transaction: &Transaction<'_>,
    candidate: &PreviewReclamationCandidate,
    observation: PreviewHealthObservation,
) -> Result<PreviewHealthOutcome, ScanError> {
    let current: bool = transaction.query_row(
        "SELECT EXISTS(SELECT 1 FROM preview_artifacts WHERE artifact_key = ?1 AND artifact_path = ?2)",
        params![candidate.artifact_key, candidate.path], |row| row.get(0),
    ).map_err(database_error)?;
    if !current {
        return Ok(PreviewHealthOutcome::Unchanged);
    }
    if let PreviewHealthObservation::File { byte_size } = observation {
        let updated = transaction
            .execute(
                "UPDATE preview_artifacts SET byte_size = ?3
             WHERE artifact_key = ?1 AND artifact_path = ?2 AND byte_size <> ?3",
                params![
                    candidate.artifact_key,
                    candidate.path,
                    sqlite_integer(byte_size, "preview artifact size")?
                ],
            )
            .map_err(database_error)?;
        return Ok(if updated == 0 {
            PreviewHealthOutcome::Unchanged
        } else {
            PreviewHealthOutcome::CorrectedSize
        });
    }
    transaction.execute(
        "UPDATE asset_locations
         SET preview_path = '', preview_status = 'pending', preview_issue_code = NULL, preview_issue_message = NULL
         WHERE preview_path = ?1 AND location_id IN (
             SELECT location_id FROM preview_artifact_locations WHERE artifact_key = ?2
         )",
        params![candidate.path, candidate.artifact_key],
    ).map_err(database_error)?;
    transaction.execute(
        "UPDATE library_change_catch_up_handoffs
         SET preview_path = '', preview_status = 'pending', preview_issue_code = NULL, preview_issue_message = NULL
         WHERE preview_status = 'ready' AND preview_path = ?1",
        [&candidate.path],
    ).map_err(database_error)?;
    transaction.execute(
        "UPDATE library_change_scan_handoff_items
         SET preview_path = '', preview_status = 'pending', preview_issue_code = NULL, preview_issue_message = NULL
         WHERE preview_status = 'ready' AND preview_path = ?1",
        [&candidate.path],
    ).map_err(database_error)?;
    transaction
        .execute(
            "DELETE FROM preview_artifacts WHERE artifact_key = ?1 AND artifact_path = ?2",
            params![candidate.artifact_key, candidate.path],
        )
        .map_err(database_error)?;
    Ok(PreviewHealthOutcome::Invalidated)
}

fn invalidate_location(
    transaction: &Transaction<'_>,
    location: &AssetLocationView,
) -> Result<PreviewHealthOutcome, ScanError> {
    let updated = transaction.execute(
        "UPDATE asset_locations
         SET preview_path = '', preview_status = 'pending', preview_issue_code = NULL, preview_issue_message = NULL
         WHERE location_id = ?1 AND root_id = ?2 AND scan_id = ?3
           AND source_generation = ?4 AND source_revision_token IS ?5
           AND preview_path = ?6 AND preview_status = 'ready'
           AND file_size = ?7 AND modified_unix_ms = ?8 AND absolute_path = ?9
           AND file_identity_scheme IS ?10 AND file_identity_value IS ?11
           AND EXISTS (SELECT 1 FROM library_roots WHERE id = ?2 AND active_scan_id = ?3)",
        params![location.location_id, location.root_id, location.scan_id,
            sqlite_integer(location.source_generation, "source generation")?,
            location.source_revision.as_ref().map(source_revision_token).transpose()?,
            location.preview_path, sqlite_integer(location.file_size, "file size")?,
            location.modified_unix_ms, location.absolute_path,
            location.file_identity.as_ref().map(|value| &value.scheme),
            location.file_identity.as_ref().map(|value| &value.value)],
    ).map_err(database_error)?;
    if updated == 0 {
        return Ok(PreviewHealthOutcome::Unchanged);
    }
    transaction.execute(
        "UPDATE preview_artifacts SET lifecycle_state = 'stale'
         WHERE artifact_key IN (SELECT artifact_key FROM preview_artifact_locations WHERE location_id = ?1)
           AND NOT EXISTS (SELECT 1 FROM preview_artifact_locations AS owners
               WHERE owners.artifact_key = preview_artifacts.artifact_key AND owners.location_id <> ?1)
           AND NOT EXISTS (SELECT 1 FROM library_change_catch_up_handoffs AS handoffs
               WHERE handoffs.preview_status = 'ready' AND handoffs.preview_path = preview_artifacts.artifact_path)
           AND NOT EXISTS (SELECT 1 FROM library_change_scan_handoff_items AS handoffs
               WHERE handoffs.preview_status = 'ready' AND handoffs.preview_path = preview_artifacts.artifact_path)",
        [&location.location_id],
    ).map_err(database_error)?;
    transaction
        .execute(
            "DELETE FROM preview_artifact_locations WHERE location_id = ?1",
            [&location.location_id],
        )
        .map_err(database_error)?;
    Ok(PreviewHealthOutcome::Invalidated)
}
