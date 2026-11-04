use rusqlite::{OptionalExtension, Transaction, params};

use crate::domain::ScanError;
use crate::domain::exact_duplicates::{
    EXACT_FINGERPRINT_ENGINE, EXACT_FINGERPRINT_PARAMETERS, EXACT_FINGERPRINT_SCHEME,
    EXACT_FINGERPRINT_VERSION,
};
use crate::domain::similar_images::{
    SIMILARITY_PARAMETERS, SIMILARITY_SCHEME, SimilarAnalysisPhase, SimilarAnalysisProgress,
    SimilarAnalysisRun,
};

use super::super::{database_error, sqlite_integer, unix_time_ms};

const LEASE_MILLIS: i64 = 60_000;

pub(super) fn begin(
    transaction: &Transaction<'_>,
    run: &SimilarAnalysisRun,
) -> Result<(), ScanError> {
    let now = unix_time_ms();
    let active: Option<(String, String, i64)> = transaction
        .query_row(
            "SELECT active_run_id, source_run_id, lease_until_unix_ms
         FROM similarity_dispatch JOIN similarity_runs ON run_id = active_run_id",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()
        .map_err(database_error)?;
    if let Some((id, source_run_id, until)) = active {
        if until > now {
            return Err(ScanError::new(
                "similarity_analysis_busy",
                "A similarity analysis already owns the catalog",
            ));
        }
        transaction.execute(
            "INSERT INTO similarity_outcomes VALUES (?1, 'failed', 0, 0, 0, 'similarity_lease_expired', ?2)",
            params![id, now],
        ).map_err(database_error)?;
        transaction.execute(
            "INSERT INTO exact_analysis_outcomes VALUES (?1, 'failed', 0, 0, 0, 0, 'similarity_lease_expired', ?2)",
            params![source_run_id, now],
        ).map_err(database_error)?;
    }
    transaction.execute(
        "INSERT INTO exact_analysis_runs(run_id, scheme, engine_id, engine_version, parameters, started_unix_ms)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            run.source_run_id,
            EXACT_FINGERPRINT_SCHEME,
            EXACT_FINGERPRINT_ENGINE,
            EXACT_FINGERPRINT_VERSION,
            EXACT_FINGERPRINT_PARAMETERS,
            run.started_unix_ms,
        ],
    ).map_err(database_error)?;
    transaction
        .execute(
            "INSERT INTO similarity_runs VALUES (?1, ?2, ?3, 'image_hasher', '3.1.1', ?4, ?5)",
            params![
                run.run_id,
                run.source_run_id,
                SIMILARITY_SCHEME,
                SIMILARITY_PARAMETERS,
                run.started_unix_ms
            ],
        )
        .map_err(database_error)?;
    transaction.execute(
        "UPDATE similarity_dispatch SET active_run_id = ?1, lease_until_unix_ms = ?2 WHERE singleton = 1",
        params![run.run_id, now + LEASE_MILLIS],
    ).map_err(database_error)?;
    Ok(())
}

pub(super) fn require_owner(
    transaction: &Transaction<'_>,
    run: &SimilarAnalysisRun,
) -> Result<(), ScanError> {
    let active: bool = transaction.query_row(
        "SELECT EXISTS(SELECT 1 FROM similarity_dispatch AS active
         JOIN similarity_runs AS run ON run.run_id = active.active_run_id
         WHERE active.active_run_id = ?1 AND run.source_run_id = ?2 AND active.lease_until_unix_ms > ?3)",
        params![run.run_id, run.source_run_id, unix_time_ms()], |row| row.get(0),
    ).map_err(database_error)?;
    if active {
        Ok(())
    } else {
        Err(ScanError::new(
            "similarity_run_retired",
            "The similarity analysis no longer owns publication",
        ))
    }
}

pub(super) fn renew(
    transaction: &Transaction<'_>,
    run: &SimilarAnalysisRun,
) -> Result<(), ScanError> {
    require_owner(transaction, run)?;
    transaction
        .execute(
            "UPDATE similarity_dispatch SET lease_until_unix_ms = ?1 WHERE active_run_id = ?2",
            params![unix_time_ms() + LEASE_MILLIS, run.run_id],
        )
        .map_err(database_error)?;
    Ok(())
}

pub(super) fn finish(
    transaction: &Transaction<'_>,
    run: &SimilarAnalysisRun,
    progress: &SimilarAnalysisProgress,
) -> Result<(), ScanError> {
    require_owner(transaction, run)?;
    let phase = match progress.phase {
        SimilarAnalysisPhase::Completed => "completed",
        SimilarAnalysisPhase::Cancelled => "cancelled",
        SimilarAnalysisPhase::Failed => "failed",
        SimilarAnalysisPhase::Running => {
            return Err(ScanError::new(
                "similarity_not_finished",
                "A running analysis has no terminal outcome",
            ));
        }
    };
    let now = unix_time_ms();
    transaction
        .execute(
            "INSERT INTO similarity_outcomes VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                run.run_id,
                phase,
                sqlite_integer(progress.checked_sources, "checked sources")?,
                sqlite_integer(progress.analyzed_contents, "analyzed contents")?,
                sqlite_integer(progress.skipped_sources, "skipped sources")?,
                progress.issue_code,
                now
            ],
        )
        .map_err(database_error)?;
    transaction.execute(
        "INSERT INTO exact_analysis_outcomes(run_id, phase, checked_files, fingerprinted_files, skipped_files, bytes_read, issue_code, finished_unix_ms)
         SELECT ?1, ?2, ?3, COUNT(*), ?4, COALESCE(SUM(byte_length), 0), ?5, ?6
         FROM content_fingerprints WHERE run_id = ?1",
        params![
            run.source_run_id,
            phase,
            sqlite_integer(progress.checked_sources, "checked sources")?,
            sqlite_integer(progress.skipped_sources, "skipped sources")?,
            progress.issue_code,
            now,
        ],
    ).map_err(database_error)?;
    transaction.execute(
        "UPDATE similarity_dispatch SET active_run_id = NULL, lease_until_unix_ms = NULL WHERE active_run_id = ?1", [&run.run_id],
    ).map_err(database_error)?;
    Ok(())
}
