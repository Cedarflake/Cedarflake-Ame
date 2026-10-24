use rusqlite::{Connection, params};

use crate::domain::ScanError;
use crate::domain::exact_duplicates::EXACT_FINGERPRINT_SCHEME;

use super::{database_error, read_unsigned};

// Native IDs include the volume identity. A matching filename or catalog-local version is never
// evidence, and contrary compatible content evidence defeats the metadata reuse path.
const NATIVE_MATCH: &str = "current.file_identity_scheme = 'windows-file-id-128-v1'
    AND current.file_identity_scheme = evidence.file_identity_scheme
    AND current.file_identity_value = evidence.file_identity_value
    AND current.source_revision_token = evidence.source_revision_token
    AND current.source_revision_token GLOB 'windows-file-change-time-100ns-v1:*'
    AND current.byte_length = evidence.byte_length
    AND (evidence.fingerprint_scheme IS NOT ?1 OR current.digest = ''
      OR current.digest = evidence.digest)";

pub(super) fn groups(
    connection: &Connection,
    checkpoint: &mut dyn FnMut(u64) -> Result<(), ScanError>,
) -> Result<(), ScanError> {
    let mut after = 0_i64;
    loop {
        let (last, count): (Option<i64>, u64) = connection.query_row(
            "SELECT MAX(rowid), COUNT(*) FROM (
               SELECT rowid FROM recovery.archive_binding WHERE rowid > ?1 ORDER BY rowid LIMIT 256)",
            [after], |row| Ok((row.get(0)?, read_unsigned(row, 1)?)),
        ).map_err(database_error)?;
        let Some(last) = last else { break };
        connection.execute(
            &format!(
                "INSERT OR IGNORE INTO temp.organization_restore_matched_groups
                 SELECT member.group_id FROM recovery.archive_binding AS evidence
                 JOIN recovery.archive_group AS member ON member.record_id = evidence.record_id
                 WHERE evidence.rowid > ?2 AND evidence.rowid <= ?3 AND (
                   EXISTS(SELECT 1 FROM temp.organization_restore_current AS current
                     WHERE evidence.fingerprint_scheme = ?1 AND current.fingerprint_scheme = ?1
                       AND current.digest = evidence.digest AND current.byte_length = evidence.byte_length
                       AND current.fingerprint_published = 1)
                   OR EXISTS(SELECT 1 FROM temp.organization_restore_native AS current
                     WHERE {NATIVE_MATCH}))"
            ), params![EXACT_FINGERPRINT_SCHEME, after, last],
        ).map_err(database_error)?;
        after = last;
        checkpoint(count)?;
    }
    Ok(())
}

pub(super) fn preferred_copies(
    connection: &Connection,
    checkpoint: &mut dyn FnMut(u64) -> Result<(), ScanError>,
) -> Result<(u64, u64), ScanError> {
    let mut after = 0_i64;
    let mut matched = 0;
    let mut total = 0;
    loop {
        let (last, count): (Option<i64>, u64) = connection
            .query_row(
                "SELECT MAX(rowid), COUNT(*) FROM (
               SELECT rowid FROM recovery.archive_duplicate_decision
               WHERE rowid > ?1 AND preferred_location_id IS NOT NULL ORDER BY rowid LIMIT 256)",
                [after],
                |row| Ok((row.get(0)?, read_unsigned(row, 1)?)),
            )
            .map_err(database_error)?;
        let Some(last) = last else { break };
        let resolved: u64 = connection.query_row(
            &format!(
                "SELECT COUNT(*) FROM recovery.archive_duplicate_decision AS decision
                 JOIN recovery.archive_version_evidence AS evidence
                   ON evidence.location_id = decision.preferred_location_id
                     AND evidence.asset_id = decision.preferred_asset_id
                 WHERE decision.rowid > ?2 AND decision.rowid <= ?3
                   AND decision.scheme = ?1 AND evidence.fingerprint_scheme = decision.scheme
                   AND evidence.digest = decision.digest AND evidence.byte_length = decision.byte_length
                   AND evidence.native_copy_count = 1
                   AND (SELECT COALESCE(SUM(current.copies), 0)
                     FROM temp.organization_restore_native AS current
                     WHERE {NATIVE_MATCH}) = 1"
            ), params![EXACT_FINGERPRINT_SCHEME, after, last], |row| read_unsigned(row, 0),
        ).map_err(database_error)?;
        total += count;
        matched += resolved;
        after = last;
        checkpoint(count)?;
    }
    Ok((matched, total - matched))
}
