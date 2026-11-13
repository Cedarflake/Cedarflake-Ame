use rusqlite::{Connection, params_from_iter};

use crate::domain::file_operation_selection::DuplicateCleanupKind;
use crate::domain::gallery_groups::GalleryGroupFilter;
use crate::domain::similar_images::review::SimilarityDegree;
use crate::domain::{GalleryQuery, ScanError};

use super::super::super::super::{database_error, gallery};

pub(in super::super) fn kind(query: &GalleryQuery) -> Result<DuplicateCleanupKind, ScanError> {
    match query.group_filter {
        GalleryGroupFilter::Exact => Ok(DuplicateCleanupKind::Exact),
        GalleryGroupFilter::Similar => Ok(DuplicateCleanupKind::Similar {
            degree: SimilarityDegree::Standard,
        }),
        GalleryGroupFilter::Unrestricted => Err(ScanError::new(
            "file_operation_selection_invalid",
            "Gallery cleanup requires a group filter",
        )),
    }
}

pub(super) fn members(query: &GalleryQuery, all: bool, frozen: bool) -> gallery::BuiltGalleryQuery {
    let selected = if frozen {
        let mut parameters = Vec::new();
        let members = gallery::groups::member_query(query, &mut parameters);
        gallery::BuiltGalleryQuery {
            sql: format!(
                "SELECT * FROM ({members}) WHERE entry_id IN (SELECT group_id FROM temp.cleanup_groups)"
            ),
            parameters,
        }
    } else {
        gallery::groups::selection::selected_members(
            query,
            gallery::groups::selection::Input::Files,
            all,
        )
    };
    let keep = match query.group_filter {
        GalleryGroupFilter::Similar => {
            "CASE WHEN retained.choices > 0 THEN member.preferred ELSE member.content_key = retained.automatic_content END"
        }
        _ => {
            "member.location_id = COALESCE(retained.preferred_location, retained.automatic_location)"
        }
    };
    gallery::BuiltGalleryQuery {
        sql: format!("WITH members AS ({}), visible_contents AS (
           SELECT entry_id, content_key,
             COALESCE(MIN(CASE WHEN location_preferred THEN location_id END), MIN(location_id)) AS representative
           FROM members GROUP BY entry_id, content_key
         ), retained AS (
           SELECT entry_id, SUM(preferred) AS choices,
             MIN(CASE WHEN preferred THEN location_id END) AS preferred_location,
             MIN(location_id) AS automatic_location,
             (SELECT content_key FROM visible_contents AS candidate WHERE candidate.entry_id = members.entry_id
               ORDER BY representative LIMIT 1) AS automatic_content
           FROM members GROUP BY entry_id
         )
         SELECT member.entry_id, member.content_key, member.asset_id, member.location_id,
           member.source_generation, state.generation, member.file_size, {keep} AS keep
         FROM members AS member JOIN retained USING(entry_id)
         JOIN library_change_root_state AS state ON state.root_id = member.root_id
         WHERE state.is_active = 1 AND member.is_pending = 0", selected.sql),
        parameters: selected.parameters,
    }
}

pub(super) fn require_ready(
    connection: &Connection,
    query: &GalleryQuery,
    all: bool,
) -> Result<(), ScanError> {
    if !all {
        gallery::groups::selection::require_inputs(
            connection,
            query,
            gallery::groups::selection::Input::Files,
        )?;
    }
    let members = gallery::groups::selection::selected_members(
        query,
        gallery::groups::selection::Input::Files,
        all,
    );
    let pending: bool = connection
        .query_row(
            &format!(
                "SELECT EXISTS(SELECT 1 FROM ({}) WHERE is_pending = 1)",
                members.sql
            ),
            params_from_iter(members.parameters),
            |row| row.get(0),
        )
        .map_err(database_error)?;
    if pending {
        Err(ScanError::new(
            "organization_evidence_pending",
            "Selected group membership is still being verified",
        ))
    } else {
        Ok(())
    }
}
