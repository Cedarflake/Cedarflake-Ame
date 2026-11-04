use rusqlite::{Connection, params};

use crate::domain::ScanError;
use crate::domain::exact_duplicates::EXACT_FINGERPRINT_SCHEME;
use crate::domain::similar_images::review::{
    SIMILARITY_REVIEW_PAGE_SIZE, SimilarReviewGroup, SimilarReviewPage, SimilarityDegree,
};
use crate::domain::similar_images::{SIMILARITY_SCHEME, SimilarContentKey};
use crate::ports::similar_images::review::{SimilarMemberCandidate, SimilarMemberCandidates};

use super::super::super::exact_duplicates::queries::{CANDIDATE_COLUMNS, CURRENT_LOCATIONS};
use super::super::super::exact_duplicates::{read_candidate, read_unsigned};
use super::super::super::{database_error, sqlite_integer};
use super::{decisions, projection};

pub(super) fn count_groups(
    connection: &Connection,
    degree: SimilarityDegree,
    ignored: bool,
) -> Result<u64, ScanError> {
    connection
        .query_row(
            &format!(
                "{} SELECT COUNT(*) FROM (
                SELECT v.group_id FROM visible_members AS v
                LEFT JOIN similarity_decisions AS choice ON choice.group_id = v.group_id
                WHERE COALESCE(choice.ignored, 0) = ?5
                GROUP BY v.group_id HAVING COUNT(*) >= 2)",
                projection::members_cte()
            ),
            params![
                EXACT_FINGERPRINT_SCHEME,
                SIMILARITY_SCHEME,
                degree.distance(),
                "",
                ignored
            ],
            |row| read_unsigned(row, 0),
        )
        .map_err(database_error)
}

pub(super) fn count_members(
    connection: &Connection,
    group_id: &str,
    degree: SimilarityDegree,
) -> Result<u64, ScanError> {
    connection
        .query_row(
            &format!(
                "{} SELECT COUNT(*) FROM visible_members",
                projection::members_cte()
            ),
            params![
                EXACT_FINGERPRINT_SCHEME,
                SIMILARITY_SCHEME,
                degree.distance(),
                group_id
            ],
            |row| read_unsigned(row, 0),
        )
        .map_err(database_error)
}

pub(super) fn groups(
    connection: &Connection,
    degree: SimilarityDegree,
    ignored: bool,
    after: &str,
    offset: u64,
    total_groups: u64,
) -> Result<SimilarReviewPage, ScanError> {
    let sql = format!(
        "{}, grouped AS (
            SELECT v.group_id, COUNT(*) AS member_count,
                   COALESCE(MIN(CASE WHEN choice.preferred_scheme = v.content_scheme
                       AND choice.preferred_digest = v.content_digest
                       AND choice.preferred_byte_length = v.content_bytes
                       THEN v.representative END), MIN(v.representative)) AS representative
            FROM visible_members AS v
            LEFT JOIN similarity_decisions AS choice ON choice.group_id = v.group_id
            WHERE COALESCE(choice.ignored, 0) = ?5 AND v.group_id > ?6
            GROUP BY v.group_id HAVING COUNT(*) >= 2
            ORDER BY v.group_id LIMIT ?7 OFFSET ?8
        )
        SELECT {CANDIDATE_COLUMNS}, grouped.group_id, grouped.member_count,
               choice.preferred_scheme, choice.preferred_digest, choice.preferred_byte_length,
               COALESCE(choice.ignored, 0), COALESCE(choice.revision, 0)
        FROM {CURRENT_LOCATIONS}
        JOIN grouped ON grouped.representative = l.location_id
        LEFT JOIN similarity_decisions AS choice ON choice.group_id = grouped.group_id
        ORDER BY grouped.group_id",
        projection::members_cte(),
    );
    let mut statement = connection.prepare(&sql).map_err(database_error)?;
    let rows = statement
        .query_map(
            params![
                EXACT_FINGERPRINT_SCHEME,
                SIMILARITY_SCHEME,
                degree.distance(),
                "",
                ignored,
                after,
                (SIMILARITY_REVIEW_PAGE_SIZE + 1) as i64,
                sqlite_integer(offset, "group offset")?,
            ],
            |row| {
                let candidate = read_candidate(row)?;
                let group_id = row.get(31)?;
                let member_count = read_unsigned(row, 32)?;
                let decision = decisions::read(row, 33)?;
                Ok(candidate.map(|candidate| SimilarReviewGroup {
                    group_id,
                    member_count,
                    representative: candidate.location,
                    decision,
                }))
            },
        )
        .map_err(database_error)?;
    let mut groups = rows
        .map(|row| row.map_err(database_error)?)
        .collect::<Result<Vec<_>, _>>()?;
    let next_group_id = if groups.len() > SIMILARITY_REVIEW_PAGE_SIZE {
        groups.truncate(SIMILARITY_REVIEW_PAGE_SIZE);
        groups.last().map(|group| group.group_id.clone())
    } else {
        None
    };
    Ok(SimilarReviewPage {
        groups,
        total_groups,
        next_group_id,
    })
}

pub(super) fn members(
    connection: &Connection,
    group_id: &str,
    degree: SimilarityDegree,
    after: Option<&SimilarContentKey>,
) -> Result<SimilarMemberCandidates, ScanError> {
    let total_members = count_members(connection, group_id, degree)?;
    let decision = decisions::load(connection, group_id)?;
    let sql = format!(
        "{} SELECT {CANDIDATE_COLUMNS}, v.content_scheme, v.content_digest, v.content_bytes,
                   v.location_count, v.distance
        FROM {CURRENT_LOCATIONS}
        JOIN visible_members AS v ON v.representative = l.location_id
        WHERE (v.content_scheme, v.content_digest, v.content_bytes) > (?5, ?6, ?7)
        ORDER BY v.content_scheme, v.content_digest, v.content_bytes LIMIT ?8",
        projection::members_cte(),
    );
    let mut statement = connection.prepare(&sql).map_err(database_error)?;
    let rows = statement
        .query_map(
            params![
                EXACT_FINGERPRINT_SCHEME,
                SIMILARITY_SCHEME,
                degree.distance(),
                group_id,
                after.map_or("", |key| key.scheme.as_str()),
                after.map_or("", |key| key.digest.as_str()),
                sqlite_integer(after.map_or(0, |key| key.byte_length), "content length")?,
                (SIMILARITY_REVIEW_PAGE_SIZE + 1) as i64,
            ],
            |row| {
                let candidate = read_candidate(row)?;
                let content = SimilarContentKey {
                    scheme: row.get(31)?,
                    digest: row.get(32)?,
                    byte_length: read_unsigned(row, 33)?,
                };
                let location_count = read_unsigned(row, 34)?;
                let distance = row.get(35)?;
                Ok(candidate.map(|candidate| SimilarMemberCandidate {
                    content,
                    candidate,
                    location_count,
                    distance,
                }))
            },
        )
        .map_err(database_error)?;
    let mut members = rows
        .map(|row| row.map_err(database_error)?)
        .collect::<Result<Vec<_>, _>>()?;
    let next = if members.len() > SIMILARITY_REVIEW_PAGE_SIZE {
        members.truncate(SIMILARITY_REVIEW_PAGE_SIZE);
        members.last().map(|member| member.content.clone())
    } else {
        None
    };
    Ok(SimilarMemberCandidates {
        members,
        total_members,
        next,
        decision,
    })
}
