use rusqlite::{Connection, params};

use crate::domain::ScanError;
use crate::domain::exact_duplicates::EXACT_FINGERPRINT_SCHEME;
use crate::domain::similar_images::SIMILARITY_SCHEME;
use crate::domain::similar_images::review::{SimilarMemberSelection, SimilarReviewContext};

use super::super::super::exact_duplicate_review::{DECISION_JOIN, representative_expression};
use super::super::super::exact_duplicates::queries::{COMPATIBLE_FINGERPRINT, CURRENT_LOCATIONS};
use super::super::super::{database_error, sqlite_integer};

const ELIGIBLE_CONTENT: &str = "JOIN similarity_evidence AS e
    ON e.fingerprint_scheme = f.scheme AND e.digest = f.digest AND e.byte_length = f.byte_length
      AND e.scheme = ?2 AND e.disposition = 'ready'
    JOIN similarity_members AS m ON m.evidence_id = e.evidence_id
    JOIN similarity_groups AS g ON g.group_id = m.group_id AND g.scheme = ?2";

const NOT_EXCLUDED: &str = "NOT EXISTS (
    SELECT 1 FROM similarity_exclusions AS excluded
    WHERE excluded.group_id = m.group_id AND excluded.fingerprint_scheme = e.fingerprint_scheme
      AND excluded.digest = e.digest AND excluded.byte_length = e.byte_length)";

pub(super) fn members_cte() -> String {
    let representative = representative_expression("l");
    format!(
        "WITH visible_members AS (
            SELECT m.group_id, e.fingerprint_scheme AS content_scheme, e.digest AS content_digest,
                   e.byte_length AS content_bytes, m.distance, COUNT(*) AS location_count,
                   {representative} AS representative
            FROM {CURRENT_LOCATIONS}
            JOIN content_fingerprints AS f ON {COMPATIBLE_FINGERPRINT}
            {ELIGIBLE_CONTENT}
            {DECISION_JOIN}
            WHERE m.distance <= ?3 AND (?4 = '' OR m.group_id = ?4) AND {NOT_EXCLUDED}
            GROUP BY m.group_id, e.evidence_id
        )"
    )
}

pub(super) fn require_selected(
    connection: &Connection,
    context: &SimilarReviewContext,
    selection: &SimilarMemberSelection,
) -> Result<(), ScanError> {
    let current: bool = connection
        .query_row(
            &format!(
                "SELECT EXISTS(SELECT 1 FROM {CURRENT_LOCATIONS}
             JOIN content_fingerprints AS f ON {COMPATIBLE_FINGERPRINT}
             {ELIGIBLE_CONTENT}
             WHERE m.distance <= ?3 AND m.group_id = ?4 AND {NOT_EXCLUDED}
               AND e.fingerprint_scheme = ?5 AND e.digest = ?6 AND e.byte_length = ?7
               AND l.asset_id = ?8 AND l.location_id = ?9 AND l.source_generation = ?10)"
            ),
            params![
                EXACT_FINGERPRINT_SCHEME,
                SIMILARITY_SCHEME,
                context.degree.distance(),
                context.group_id,
                selection.content.scheme,
                selection.content.digest,
                sqlite_integer(selection.content.byte_length, "content length")?,
                selection.source.asset_id,
                selection.source.location_id,
                sqlite_integer(selection.source.source_generation, "source generation")?,
            ],
            |row| row.get(0),
        )
        .map_err(database_error)?;
    if current { Ok(()) } else { Err(stale()) }
}

pub(super) fn stale() -> ScanError {
    ScanError::new(
        "similarity_group_changed",
        "This similarity group or selected image has changed",
    )
}
