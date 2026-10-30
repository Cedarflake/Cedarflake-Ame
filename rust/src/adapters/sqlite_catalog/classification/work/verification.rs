use std::sync::atomic::{AtomicBool, Ordering};

use rusqlite::{Connection, params};

use crate::domain::ScanError;
use crate::domain::classification::work::{ClassificationWorkLease, ClassificationWorkTarget};
use crate::domain::exact_duplicates::{
    ContentFingerprint, EXACT_FINGERPRINT_ENGINE, EXACT_FINGERPRINT_PARAMETERS,
    EXACT_FINGERPRINT_SCHEME, EXACT_FINGERPRINT_VERSION, FingerprintCandidate,
};
use crate::domain::organization::OrganizationSelection;
use crate::ports::classification_source::{
    ClassificationSourcePreparation, ClassificationSourceRepository,
};

use super::super::super::{SqliteCatalog, database_error, exact_duplicates, sqlite_integer};
use super::{completion, invalid, leases, queue};

impl ClassificationSourceRepository for SqliteCatalog {
    fn prepare_classification_source(
        &mut self,
        lease: &ClassificationWorkLease,
        cancelled: &AtomicBool,
    ) -> Result<ClassificationSourcePreparation, ScanError> {
        let transaction = self.begin_write()?;
        let Some(source) = current_source(&transaction, lease)? else {
            return Ok(ClassificationSourcePreparation::Superseded);
        };
        if cancelled.load(Ordering::Acquire) {
            return Ok(ClassificationSourcePreparation::Superseded);
        }
        let Some(candidate) = exact_duplicates::queries::load_selected(&transaction, source)?
        else {
            let phase = if queue::current_source(&transaction, source.source_generation)?.is_some()
            {
                "queued"
            } else {
                "retired"
            };
            leases::checkpoint(&transaction, lease.work_id, phase)?;
            transaction.commit().map_err(database_error)?;
            return Ok(ClassificationSourcePreparation::Superseded);
        };
        if exact_duplicates::gallery_publication::publish_verified_source(&transaction, source)? {
            if !completion::verified(&transaction, lease)? {
                return Err(invalid());
            }
            if cancelled.load(Ordering::Acquire) {
                return Ok(ClassificationSourcePreparation::Superseded);
            }
            transaction.commit().map_err(database_error)?;
            return Ok(ClassificationSourcePreparation::Completed);
        }
        transaction.commit().map_err(database_error)?;
        Ok(ClassificationSourcePreparation::Read(Box::new(candidate)))
    }

    fn commit_classification_source(
        &mut self,
        lease: &ClassificationWorkLease,
        candidate: &FingerprintCandidate,
        fingerprint: &ContentFingerprint,
        now_unix_ms: u64,
        cancelled: &AtomicBool,
    ) -> Result<bool, ScanError> {
        if cancelled.load(Ordering::Acquire) {
            return Ok(false);
        }
        let transaction = self.begin_write()?;
        let Some(source) = current_source(&transaction, lease)? else {
            return Ok(false);
        };
        if source.asset_id != candidate.location.asset_id
            || source.location_id != candidate.location.location_id
            || source.source_generation != candidate.location.source_generation
        {
            return Err(invalid());
        }
        let run_id = format!("classification-{}", lease.run_id.as_str());
        transaction.execute(
            "INSERT OR IGNORE INTO exact_analysis_runs(run_id, scheme, engine_id, engine_version, parameters, started_unix_ms)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![run_id, EXACT_FINGERPRINT_SCHEME, EXACT_FINGERPRINT_ENGINE, EXACT_FINGERPRINT_VERSION,
                EXACT_FINGERPRINT_PARAMETERS, sqlite_integer(now_unix_ms, "verification start time")?],
        ).map_err(database_error)?;
        let compatible_run: bool = transaction
            .query_row(
                "SELECT scheme = ?2 AND engine_id = ?3 AND engine_version = ?4 AND parameters = ?5
             FROM exact_analysis_runs WHERE run_id = ?1",
                params![
                    run_id,
                    EXACT_FINGERPRINT_SCHEME,
                    EXACT_FINGERPRINT_ENGINE,
                    EXACT_FINGERPRINT_VERSION,
                    EXACT_FINGERPRINT_PARAMETERS
                ],
                |row| row.get(0),
            )
            .map_err(database_error)?;
        if !compatible_run {
            return Err(invalid());
        }
        let result =
            exact_duplicates::publication::publish(&transaction, &run_id, candidate, fingerprint)?;
        if result == exact_duplicates::publication::FingerprintPublication::Superseded {
            return Ok(false);
        }
        if !exact_duplicates::gallery_publication::publish_verified_source(&transaction, source)?
            || !completion::verified(&transaction, lease)?
        {
            return Err(invalid());
        }
        if cancelled.load(Ordering::Acquire) {
            return Ok(false);
        }
        transaction.commit().map_err(database_error)?;
        Ok(true)
    }
}

fn current_source<'a>(
    connection: &Connection,
    lease: &'a ClassificationWorkLease,
) -> Result<Option<&'a OrganizationSelection>, ScanError> {
    if leases::current(connection, lease)?.is_none() {
        return Ok(None);
    }
    let ClassificationWorkTarget::VerifySource(source) = &lease.target else {
        return Err(invalid());
    };
    Ok(Some(source))
}
