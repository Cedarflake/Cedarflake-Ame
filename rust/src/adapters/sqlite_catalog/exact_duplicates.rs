use std::sync::atomic::{AtomicBool, Ordering};

use rusqlite::{Row, params};

use crate::domain::exact_duplicates::{
    ContentFingerprint, EXACT_DUPLICATE_PAGE_SIZE, EXACT_FINGERPRINT_ENGINE,
    EXACT_FINGERPRINT_PARAMETERS, EXACT_FINGERPRINT_SCHEME, EXACT_FINGERPRINT_VERSION,
    ExactAnalysisRun, FingerprintCandidate,
};
use crate::domain::{
    ExactAnalysisPhase, ExactAnalysisProgress, ExactDuplicateGroup, ExactDuplicateGroupCursor,
    ExactDuplicateGroupPage, IncrementalCatalogRoot, LibraryRootGeneration, ScanError,
};
use crate::ports::exact_duplicates::ExactDuplicateRepository;

use super::{
    SqliteCatalog, database_error, read_stored_asset, sqlite_integer, stored_asset_view,
    stored_file_identity, unix_time_ms,
};

pub(super) mod gallery_publication;
pub(super) mod publication;
pub(super) mod queries;

impl ExactDuplicateRepository for SqliteCatalog {
    fn begin_exact_analysis(&mut self, run: &ExactAnalysisRun) -> Result<(), ScanError> {
        let transaction = self.begin_write()?;
        transaction
            .execute(
                "INSERT INTO exact_analysis_runs(
                    run_id, scheme, engine_id, engine_version, parameters, started_unix_ms
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    run.run_id,
                    EXACT_FINGERPRINT_SCHEME,
                    EXACT_FINGERPRINT_ENGINE,
                    EXACT_FINGERPRINT_VERSION,
                    EXACT_FINGERPRINT_PARAMETERS,
                    run.started_unix_ms
                ],
            )
            .map_err(database_error)?;
        transaction.commit().map_err(database_error)
    }

    fn finish_exact_analysis(&mut self, progress: &ExactAnalysisProgress) -> Result<(), ScanError> {
        let phase = match progress.phase {
            ExactAnalysisPhase::Completed => "completed",
            ExactAnalysisPhase::Cancelled => "cancelled",
            ExactAnalysisPhase::Failed => "failed",
            ExactAnalysisPhase::Running => {
                return Err(ScanError::new(
                    "exact_analysis_not_finished",
                    "A running analysis has no terminal outcome",
                ));
            }
        };
        let transaction = self.begin_write()?;
        transaction
            .execute(
                "INSERT INTO exact_analysis_outcomes(
                    run_id, phase, checked_files, fingerprinted_files,
                    skipped_files, bytes_read, issue_code, finished_unix_ms
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    progress.run_id,
                    phase,
                    sqlite_integer(progress.checked_files, "checked files")?,
                    sqlite_integer(progress.fingerprinted_files, "fingerprinted files")?,
                    sqlite_integer(progress.skipped_files, "skipped files")?,
                    sqlite_integer(progress.bytes_read, "analysis bytes")?,
                    progress.issue_code,
                    unix_time_ms()
                ],
            )
            .map_err(database_error)?;
        super::gallery::duplicates::publish_completed_evidence(&transaction)?;
        transaction.commit().map_err(database_error)
    }

    fn load_fingerprint_candidates(
        &self,
        after_location_id: &str,
    ) -> Result<Vec<FingerprintCandidate>, ScanError> {
        queries::load_candidates(&self.connection, after_location_id)
    }

    fn publish_exact_analysis_batch(
        &mut self,
        run_id: &str,
        source_generations: &[u64],
    ) -> Result<(), ScanError> {
        let transaction = self.begin_write()?;
        gallery_publication::publish(&transaction, run_id, source_generations)?;
        transaction.commit().map_err(database_error)
    }

    fn publish_fingerprint(
        &mut self,
        run_id: &str,
        candidate: &FingerprintCandidate,
        fingerprint: &ContentFingerprint,
        cancelled: &AtomicBool,
    ) -> Result<bool, ScanError> {
        if cancelled.load(Ordering::Acquire) {
            return Ok(false);
        }
        let transaction = self.begin_write()?;
        if cancelled.load(Ordering::Acquire) {
            return Ok(false);
        }
        let accepted = publication::publish(&transaction, run_id, candidate, fingerprint)?;
        if cancelled.load(Ordering::Acquire) {
            return Ok(false);
        }
        transaction.commit().map_err(database_error)?;
        Ok(accepted == publication::FingerprintPublication::Published)
    }

    fn load_duplicate_groups(
        &self,
        after: Option<&ExactDuplicateGroupCursor>,
    ) -> Result<ExactDuplicateGroupPage, ScanError> {
        queries::load_groups(&self.connection, after)
    }

    fn load_duplicate_locations(
        &self,
        digest: &str,
        byte_length: u64,
        after_location_id: &str,
    ) -> Result<Vec<FingerprintCandidate>, ScanError> {
        queries::load_locations(&self.connection, digest, byte_length, after_location_id)
    }
}

pub(super) fn read_candidate(
    row: &Row<'_>,
) -> rusqlite::Result<Result<FingerprintCandidate, ScanError>> {
    let stored = read_stored_asset(row)?;
    let root_path = row.get(27)?;
    let generation = read_unsigned(row, 28)?;
    let scheme = row.get(29)?;
    let value = row.get(30)?;
    Ok((|| {
        let location = stored_asset_view(stored)?;
        let root = IncrementalCatalogRoot {
            root_id: location.root_id.clone(),
            root_path,
            root_generation: LibraryRootGeneration::new(generation).ok_or_else(|| {
                ScanError::new(
                    "exact_root_generation_invalid",
                    "The content source requires an active root generation",
                )
            })?,
            active_scan_id: Some(location.scan_id.clone()),
            has_running_scan: false,
            catalog_revision: 0,
            last_consistency_audit_unix_ms: None,
            publication_root_identity: stored_file_identity(scheme, value)?,
        };
        Ok(FingerprintCandidate { location, root })
    })())
}

pub(super) fn read_unsigned(row: &Row<'_>, index: usize) -> rusqlite::Result<u64> {
    let value = row.get::<_, i64>(index)?;
    u64::try_from(value).map_err(|_| rusqlite::Error::IntegralValueOutOfRange(index, value))
}

#[cfg(test)]
mod tests;
