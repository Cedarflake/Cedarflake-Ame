use rusqlite::types::Value;

use crate::domain::exact_duplicates::EXACT_FINGERPRINT_SCHEME;
use crate::domain::file_operation_selection::DuplicateCleanupKind;
use crate::domain::similar_images::SIMILARITY_SCHEME;

use super::super::super::super::exact_duplicate_review::{
    DECISION_JOIN, representative_expression,
};
use super::super::super::super::exact_duplicates::queries::{
    COMPATIBLE_FINGERPRINT, CURRENT_LOCATIONS,
};
use super::super::super::super::gallery::BuiltGalleryQuery;
use super::super::super::super::similar_images::review::projection::{MemberScope, members_cte};

pub(super) fn members(kind: &DuplicateCleanupKind, all: bool, frozen: bool) -> BuiltGalleryQuery {
    let input = if frozen {
        "cleanup_groups"
    } else {
        "cleanup_requested"
    };
    let scope = |group: &str| {
        if all {
            "1".into()
        } else {
            format!(
                "EXISTS(SELECT 1 FROM temp.{input} AS requested WHERE requested.group_id = {group})"
            )
        }
    };
    match kind {
        DuplicateCleanupKind::Exact => BuiltGalleryQuery {
            sql: format!(
                "WITH retained AS (
                   SELECT f.digest, f.byte_length, {} AS location_id
                   FROM {CURRENT_LOCATIONS}
                   JOIN content_fingerprints AS f ON {COMPATIBLE_FINGERPRINT}
                   {DECISION_JOIN}
                   WHERE COALESCE(decision.ignored, 0) = 0 AND {}
                   GROUP BY f.digest, f.byte_length
                 )
                 SELECT f.digest || ':' || f.byte_length AS group_id,
                        f.digest || ':' || f.byte_length AS content_key,
                        l.asset_id, l.location_id, l.source_generation, state.generation, l.file_size,
                        l.location_id = retained.location_id AS keep
                 FROM {CURRENT_LOCATIONS}
                 JOIN content_fingerprints AS f ON {COMPATIBLE_FINGERPRINT}
                 JOIN retained ON retained.digest = f.digest AND retained.byte_length = f.byte_length",
                representative_expression("l"), scope("f.digest || ':' || f.byte_length")),
            parameters: vec![Value::Text(EXACT_FINGERPRINT_SCHEME.into())],
        },
        DuplicateCleanupKind::Similar { degree } => BuiltGalleryQuery {
            sql: format!(
                "{}, retained AS (
                   SELECT group_id, SUM(preferred) AS choices, MIN(representative) AS automatic
                   FROM visible_members GROUP BY group_id
                 )
                 SELECT v.group_id, v.content_digest || ':' || v.content_bytes AS content_key,
                           l.asset_id, l.location_id, l.source_generation, state.generation, l.file_size,
                           CASE WHEN retained.choices > 0 THEN v.preferred
                             ELSE v.representative = retained.automatic END AS keep
                 FROM visible_members AS v
                 JOIN retained USING(group_id)
                 JOIN content_fingerprints AS f ON f.scheme = v.content_scheme
                   AND f.digest = v.content_digest AND f.byte_length = v.content_bytes
                 CROSS JOIN {CURRENT_LOCATIONS}
                 LEFT JOIN similarity_decisions AS choice ON choice.group_id = v.group_id
                 WHERE {COMPATIBLE_FINGERPRINT} AND COALESCE(choice.ignored, 0) = 0 AND {}",
                members_cte(MemberScope::AllGroups), scope("v.group_id")),
            parameters: vec![Value::Text(EXACT_FINGERPRINT_SCHEME.into()),
                             Value::Text(SIMILARITY_SCHEME.into()), Value::Integer(degree.distance().into())],
        },
    }
}
