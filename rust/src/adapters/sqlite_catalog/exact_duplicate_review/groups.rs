use rusqlite::{Connection, params};

use crate::domain::exact_duplicate_review::{
    ExactDuplicateDecision, ExactDuplicateReviewGroup, ExactDuplicateReviewPage,
};
use crate::domain::exact_duplicates::{EXACT_DUPLICATE_PAGE_SIZE, EXACT_FINGERPRINT_SCHEME};
use crate::domain::{ExactDuplicateGroup, ExactDuplicateGroupCursor, ScanError};

use super::super::exact_duplicates::queries::{
    CANDIDATE_COLUMNS, COMPATIBLE_FINGERPRINT, CURRENT_LOCATIONS,
};
use super::super::exact_duplicates::{read_candidate, read_unsigned};
use super::super::{database_error, sqlite_integer};
use super::{DECISION_JOIN, representative_expression};

pub(super) fn load(
    connection: &Connection,
    after: Option<&ExactDuplicateGroupCursor>,
    ignored: bool,
) -> Result<ExactDuplicateReviewPage, ScanError> {
    // Count and page must describe the same catalog snapshot, including on an empty last page.
    let transaction = connection.unchecked_transaction().map_err(database_error)?;
    let total_groups = count(&transaction, ignored)?;
    let representative = representative_expression("l");
    let sql = format!("WITH grouped AS (
        SELECT f.digest, f.byte_length, COUNT(*) AS locations,
               COUNT(DISTINCT l.source_generation) AS physical_files,
               {representative} AS representative
        FROM {CURRENT_LOCATIONS}
        JOIN content_fingerprints AS f ON {COMPATIBLE_FINGERPRINT}
        {DECISION_JOIN}
        WHERE (f.digest, f.byte_length) > (?2, ?3)
          AND COALESCE(decision.ignored, 0) = ?4
        GROUP BY f.digest, f.byte_length HAVING COUNT(*) > 1
        ORDER BY f.digest, f.byte_length LIMIT ?5
    )
    SELECT {CANDIDATE_COLUMNS}, g.digest, g.byte_length, g.locations, g.physical_files,
           decision.preferred_location_id, decision.preferred_asset_id, COALESCE(decision.ignored, 0)
    FROM {CURRENT_LOCATIONS} JOIN grouped AS g ON l.location_id = g.representative
    LEFT JOIN exact_duplicate_decisions AS decision
      ON decision.scheme = ?1 AND decision.digest = g.digest AND decision.byte_length = g.byte_length
    ORDER BY g.digest, g.byte_length");
    let mut statement = transaction.prepare(&sql).map_err(database_error)?;
    let rows = statement
        .query_map(
            params![
                EXACT_FINGERPRINT_SCHEME,
                after.map_or("", |cursor| cursor.digest.as_str()),
                sqlite_integer(
                    after.map_or(0, |cursor| cursor.byte_length),
                    "content length"
                )?,
                ignored,
                (EXACT_DUPLICATE_PAGE_SIZE + 1) as i64
            ],
            |row| {
                let candidate = read_candidate(row)?;
                let digest = row.get(29)?;
                let byte_length = read_unsigned(row, 30)?;
                let location_count = read_unsigned(row, 31)?;
                let physical_file_count = read_unsigned(row, 32)?;
                let decision = ExactDuplicateDecision {
                    preferred_location_id: row.get(33)?,
                    preferred_asset_id: row.get(34)?,
                    ignored: row.get(35)?,
                };
                Ok(candidate.map(|candidate| ExactDuplicateReviewGroup {
                    group: ExactDuplicateGroup {
                        digest,
                        byte_length,
                        location_count,
                        physical_file_count,
                        representative_location_id: candidate.location.location_id.clone(),
                        representative_path: candidate.location.display_path.clone(),
                    },
                    representative: candidate.location,
                    decision,
                }))
            },
        )
        .map_err(database_error)?;
    let mut groups = rows
        .map(|row| row.map_err(database_error)?)
        .collect::<Result<Vec<_>, _>>()?;
    let next = if groups.len() > EXACT_DUPLICATE_PAGE_SIZE {
        groups.truncate(EXACT_DUPLICATE_PAGE_SIZE);
        groups.last().map(|value| ExactDuplicateGroupCursor {
            digest: value.group.digest.clone(),
            byte_length: value.group.byte_length,
        })
    } else {
        None
    };
    drop(statement);
    transaction.commit().map_err(database_error)?;
    Ok(ExactDuplicateReviewPage {
        groups,
        total_groups,
        next,
    })
}

fn count(connection: &Connection, ignored: bool) -> Result<u64, ScanError> {
    connection
        .query_row(
            &format!(
                "SELECT COUNT(*) FROM (
                SELECT f.digest, f.byte_length
                FROM {CURRENT_LOCATIONS}
                JOIN content_fingerprints AS f ON {COMPATIBLE_FINGERPRINT}
                {DECISION_JOIN}
                WHERE COALESCE(decision.ignored, 0) = ?2
                GROUP BY f.digest, f.byte_length
                HAVING COUNT(*) > 1
            )"
            ),
            params![EXACT_FINGERPRINT_SCHEME, ignored],
            |row| read_unsigned(row, 0),
        )
        .map_err(database_error)
}
