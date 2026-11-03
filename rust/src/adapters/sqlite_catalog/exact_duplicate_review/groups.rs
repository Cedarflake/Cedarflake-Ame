use rusqlite::{Connection, params};

use crate::domain::exact_duplicate_review::{
    ExactDuplicateDecision, ExactDuplicateReviewGroup, ExactDuplicateReviewPage,
    ExactDuplicateReviewRange,
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
    let page = load_page(&transaction, after, ignored, total_groups, 0)?;
    transaction.commit().map_err(database_error)?;
    Ok(page)
}

pub(super) fn seek(
    connection: &Connection,
    index: u64,
    ignored: bool,
) -> Result<ExactDuplicateReviewRange, ScanError> {
    let transaction = connection.unchecked_transaction().map_err(database_error)?;
    let total_groups = count(&transaction, ignored)?;
    // Clamp and align against this snapshot, including a library that shrank since the request.
    let page_size = EXACT_DUPLICATE_PAGE_SIZE as u64;
    let start_index = index.min(total_groups.saturating_sub(1)) / page_size * page_size;
    let page = load_page(&transaction, None, ignored, total_groups, start_index)?;
    transaction.commit().map_err(database_error)?;
    Ok(ExactDuplicateReviewRange { start_index, page })
}

fn load_page(
    connection: &Connection,
    after: Option<&ExactDuplicateGroupCursor>,
    ignored: bool,
    total_groups: u64,
    offset: u64,
) -> Result<ExactDuplicateReviewPage, ScanError> {
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
        ORDER BY f.digest, f.byte_length LIMIT ?5 OFFSET ?6
    )
    SELECT {CANDIDATE_COLUMNS}, g.digest AS review_digest, g.byte_length AS review_byte_length,
           g.locations AS review_locations, g.physical_files AS review_physical_files,
           decision.preferred_location_id AS review_preferred_location_id,
           decision.preferred_asset_id AS review_preferred_asset_id,
           COALESCE(decision.ignored, 0) AS review_ignored
    FROM {CURRENT_LOCATIONS} JOIN grouped AS g ON l.location_id = g.representative
    LEFT JOIN exact_duplicate_decisions AS decision
      ON decision.scheme = ?1 AND decision.digest = g.digest AND decision.byte_length = g.byte_length
    ORDER BY g.digest, g.byte_length");
    let mut statement = connection.prepare(&sql).map_err(database_error)?;
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
                (EXACT_DUPLICATE_PAGE_SIZE + 1) as i64,
                sqlite_integer(offset, "duplicate group offset")?
            ],
            |row| {
                let candidate = read_candidate(row)?;
                let digest = row.get("review_digest")?;
                let byte_length =
                    read_unsigned(row, row.as_ref().column_index("review_byte_length")?)?;
                let location_count =
                    read_unsigned(row, row.as_ref().column_index("review_locations")?)?;
                let physical_file_count =
                    read_unsigned(row, row.as_ref().column_index("review_physical_files")?)?;
                let decision = ExactDuplicateDecision {
                    preferred_location_id: row.get("review_preferred_location_id")?,
                    preferred_asset_id: row.get("review_preferred_asset_id")?,
                    ignored: row.get("review_ignored")?,
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
