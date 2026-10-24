use rusqlite::{Connection, params};

use crate::domain::ScanError;
use crate::domain::exact_duplicates::EXACT_FINGERPRINT_SCHEME;

use super::super::super::database_error;
use super::super::super::exact_duplicates::queries::{COMPATIBLE_FINGERPRINT, CURRENT_LOCATIONS};

pub(super) fn reconcile(connection: &Connection, generations: &[i64]) -> Result<(), ScanError> {
    connection.execute_batch(
        "CREATE TEMP TABLE IF NOT EXISTS organization_recovery_batch (source_generation INTEGER PRIMARY KEY);
         DELETE FROM temp.organization_recovery_batch;
         CREATE TEMP TABLE IF NOT EXISTS organization_recovery_candidates (
           source_generation INTEGER NOT NULL, admission_id INTEGER NOT NULL,
           PRIMARY KEY(source_generation, admission_id)
         ) WITHOUT ROWID;
         DELETE FROM temp.organization_recovery_candidates;",
    ).map_err(database_error)?;
    let mut insert = connection
        .prepare_cached("INSERT INTO temp.organization_recovery_batch VALUES (?1)")
        .map_err(database_error)?;
    for generation in generations {
        insert.execute([generation]).map_err(database_error)?;
    }
    connection.execute(
        &format!(
            "WITH current AS MATERIALIZED (
               SELECT DISTINCT l.source_generation, l.file_identity_scheme, l.file_identity_value,
                 l.source_revision_token, l.file_size, f.digest
               FROM temp.organization_recovery_batch AS batch
               CROSS JOIN {CURRENT_LOCATIONS}
               LEFT JOIN content_fingerprints AS f ON {COMPATIBLE_FINGERPRINT}
               WHERE l.source_generation = batch.source_generation
                 AND l.file_identity_scheme = 'windows-file-id-128-v1'
                 AND l.source_revision_token GLOB 'windows-file-change-time-100ns-v1:*'
             )
             INSERT INTO temp.organization_recovery_candidates
             SELECT current.source_generation, MIN(admitted.admission_id) FROM current
             CROSS JOIN organization_recovery_versions AS evidence
               ON evidence.file_identity_scheme = current.file_identity_scheme
                 AND evidence.file_identity_value = current.file_identity_value
                 AND evidence.source_revision_token = current.source_revision_token
                 AND evidence.byte_length = current.file_size
             JOIN organization_recovery_admissions AS admitted ON admitted.source_generation = evidence.source_generation
             GROUP BY current.source_generation
             HAVING COUNT(DISTINCT CASE WHEN evidence.fingerprint_scheme = ?1 THEN evidence.digest END) <= 1
               AND (MIN(current.digest) IS NULL
                 OR MIN(CASE WHEN evidence.fingerprint_scheme = ?1 THEN evidence.digest END) IS NULL
                 OR MIN(current.digest) = MIN(CASE WHEN evidence.fingerprint_scheme = ?1 THEN evidence.digest END))"
        ), [EXACT_FINGERPRINT_SCHEME],
    ).map_err(database_error)?;
    connection.execute(
        &format!(
            "WITH current AS MATERIALIZED (
               SELECT DISTINCT l.source_generation, f.scheme, f.digest, f.byte_length
               FROM temp.organization_recovery_batch AS batch CROSS JOIN {CURRENT_LOCATIONS}
               JOIN content_fingerprints AS f ON {COMPATIBLE_FINGERPRINT}
               JOIN gallery_content_fingerprints AS published
                 ON published.source_generation = f.source_generation AND published.scheme = f.scheme
               WHERE l.source_generation = batch.source_generation
             ), history AS MATERIALIZED (
               SELECT content.scheme, content.digest, content.byte_length, MIN(admitted.admission_id) AS admission_id
               FROM (SELECT DISTINCT scheme, digest, byte_length FROM current) AS content
               CROSS JOIN organization_recovery_versions AS evidence
                 ON evidence.fingerprint_scheme = content.scheme AND evidence.digest = content.digest AND evidence.byte_length = content.byte_length
               JOIN organization_recovery_admissions AS admitted ON admitted.source_generation = evidence.source_generation
               GROUP BY content.scheme, content.digest, content.byte_length
             )
             INSERT INTO temp.organization_recovery_candidates
             SELECT current.source_generation, history.admission_id FROM current
             JOIN history USING(scheme, digest, byte_length) WHERE 1
             ON CONFLICT(source_generation, admission_id) DO NOTHING"
        ), params![EXACT_FINGERPRINT_SCHEME],
    ).map_err(database_error)?;
    connection.execute_batch(
        "DELETE FROM organization_recovery_admission_links
         WHERE source_generation IN (SELECT source_generation FROM temp.organization_recovery_batch);
         INSERT INTO organization_recovery_admission_links
         SELECT candidate.source_generation, history.source_generation FROM (
           SELECT source_generation, MIN(admission_id) AS admission_id
           FROM temp.organization_recovery_candidates GROUP BY source_generation
         ) AS candidate JOIN organization_recovery_admissions AS history USING(admission_id);
         INSERT INTO organization_recovery_admission_sources
         SELECT linked.recovered_generation, location.root_id
         FROM temp.organization_recovery_batch AS batch
         CROSS JOIN organization_recovery_admission_links AS linked USING(source_generation)
         CROSS JOIN asset_locations AS location ON location.source_generation = linked.source_generation
         JOIN library_roots AS root ON root.id = location.root_id AND root.active_scan_id = location.scan_id
         WHERE 1 ON CONFLICT(source_generation, root_id) DO NOTHING;",
    ).map_err(database_error)?;
    super::duplicates::reconcile(connection)
}
