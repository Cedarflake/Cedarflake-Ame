use rusqlite::{Connection, Transaction, params};

use crate::domain::ScanError;
use crate::domain::exact_duplicates::{
    ContentFingerprint, EXACT_FINGERPRINT_SCHEME, FingerprintCandidate,
    MAX_FINGERPRINT_SOURCE_BYTES,
};
use crate::domain::organization::OrganizationSelection;
use crate::domain::similar_images::{
    SIMILARITY_SCHEME, SIMILARITY_SOURCE_PAGE_SIZE, SimilarAnalysisRun, SimilarContentKey,
    SimilarSource,
};

use super::super::exact_duplicates::queries::{
    CANDIDATE_COLUMNS, COMPATIBLE_FINGERPRINT, CURRENT_LOCATIONS,
};
use super::super::exact_duplicates::{gallery_publication, publication, read_candidate};
use super::super::{database_error, sqlite_integer, unix_time_ms};

pub(super) fn load(connection: &Connection, after: &str) -> Result<Vec<SimilarSource>, ScanError> {
    let sql = format!(
        "SELECT {CANDIDATE_COLUMNS}, f.digest FROM {CURRENT_LOCATIONS}
         LEFT JOIN content_fingerprints AS f ON {COMPATIBLE_FINGERPRINT}
         WHERE l.location_id > ?2 AND l.file_size BETWEEN 1 AND ?3
           AND (f.source_generation IS NULL OR NOT EXISTS (
             SELECT 1 FROM similarity_evidence AS e WHERE e.scheme = ?4
               AND e.fingerprint_scheme = f.scheme AND e.digest = f.digest AND e.byte_length = f.byte_length))
           AND NOT EXISTS(SELECT 1 FROM similarity_source_failures AS failed
             WHERE failed.source_generation = l.source_generation AND failed.root_id = l.root_id
               AND failed.root_generation = state.generation AND failed.location_id = l.location_id
               AND failed.scheme = ?4 AND (failed.retry_unix_ms IS NULL OR failed.retry_unix_ms > ?5))
         ORDER BY l.location_id LIMIT ?6"
    );
    let mut statement = connection.prepare(&sql).map_err(database_error)?;
    let rows = statement
        .query_map(
            params![
                EXACT_FINGERPRINT_SCHEME,
                after,
                MAX_FINGERPRINT_SOURCE_BYTES as i64,
                SIMILARITY_SCHEME,
                unix_time_ms(),
                SIMILARITY_SOURCE_PAGE_SIZE as i64
            ],
            |row| Ok((read_candidate(row)?, row.get::<_, Option<String>>(31)?)),
        )
        .map_err(database_error)?;
    rows.map(|row| {
        let (candidate, digest) = row.map_err(database_error)?;
        let candidate = candidate?;
        let content = digest.map(|digest| SimilarContentKey {
            scheme: EXACT_FINGERPRINT_SCHEME.into(),
            digest,
            byte_length: candidate.location.file_size,
        });
        Ok(SimilarSource { candidate, content })
    })
    .collect()
}

pub(super) fn verify(
    transaction: &Transaction<'_>,
    run: &SimilarAnalysisRun,
    candidate: &FingerprintCandidate,
    fingerprint: &ContentFingerprint,
) -> Result<bool, ScanError> {
    if publication::publish(transaction, &run.source_run_id, candidate, fingerprint)?
        == publication::FingerprintPublication::Superseded
    {
        return Ok(false);
    }
    let location = &candidate.location;
    gallery_publication::publish_verified_source(
        transaction,
        &OrganizationSelection {
            asset_id: location.asset_id.clone(),
            location_id: location.location_id.clone(),
            source_generation: location.source_generation,
        },
    )
}

pub(super) fn record_failure(
    transaction: &Transaction<'_>,
    candidate: &FingerprintCandidate,
    issue_code: &str,
) -> Result<(), ScanError> {
    let location = &candidate.location;
    transaction
        .execute(
            "INSERT INTO similarity_source_failures VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
         ON CONFLICT(source_generation, root_id, root_generation, location_id, scheme)
         DO UPDATE SET issue_code = excluded.issue_code, retry_unix_ms = excluded.retry_unix_ms",
            params![
                sqlite_integer(location.source_generation, "source generation")?,
                location.root_id,
                sqlite_integer(candidate.root.root_generation.value(), "root generation")?,
                location.location_id,
                SIMILARITY_SCHEME,
                issue_code,
                unix_time_ms() + 60_000
            ],
        )
        .map_err(database_error)?;
    Ok(())
}
