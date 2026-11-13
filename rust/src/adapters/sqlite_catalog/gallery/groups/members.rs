use rusqlite::types::Value;

use crate::domain::GalleryQuery;
use crate::domain::gallery_groups::GalleryGroupFilter;
use crate::domain::similar_images::{SIMILARITY_SCHEME, review::SimilarityDegree};

use super::super::super::exact_duplicate_review::DECISION_JOIN;
use super::super::duplicates::{DISPLAY_FINGERPRINTS, compatible_fingerprint};
use super::super::filters::{gallery_source, push_source_filters};

/// Every group operation uses the same scoped, compatible physical membership.
pub(in super::super::super) fn query(query: &GalleryQuery, parameters: &mut Vec<Value>) -> String {
    build(query, parameters, false)
}

pub(in super::super::super) fn inspection(
    query: &GalleryQuery,
    parameters: &mut Vec<Value>,
) -> String {
    build(query, parameters, true)
}

pub(in super::super::super) fn visible(
    query: &GalleryQuery,
    parameters: &mut Vec<Value>,
) -> String {
    let members = build(query, parameters, false);
    let count = match query.group_filter {
        GalleryGroupFilter::Similar => "COUNT(DISTINCT content_key)",
        _ => "COUNT(*)",
    };
    format!(
        "WITH members AS ({members}), visible_groups AS (
           SELECT entry_id FROM members GROUP BY entry_id HAVING {count} > 1
         )
         SELECT members.* FROM members JOIN visible_groups USING(entry_id)"
    )
}

fn build(query: &GalleryQuery, parameters: &mut Vec<Value>, include_ignored: bool) -> String {
    let mut scope = query.clone();
    scope.group_filter = GalleryGroupFilter::Unrestricted;
    let source = gallery_source(&scope, parameters);
    let mut clauses = Vec::new();
    push_source_filters(&scope, &mut clauses, parameters);
    let conditions = if clauses.is_empty() {
        "1".to_owned()
    } else {
        clauses.join(" AND ")
    };
    let rank = if scope.semantic_results.is_some() {
        "CAST(semantic_result.key AS INTEGER)"
    } else {
        "0"
    };
    let compatible = compatible_fingerprint("locations");
    let exact_preference = "locations.location_id = decision.preferred_location_id
        AND locations.asset_id = decision.preferred_asset_id";
    let content = "f.digest || ':' || f.byte_length";
    let (group_columns, joins, membership) = match query.group_filter {
        GalleryGroupFilter::Similar => (
            "'similar:' || m.group_id AS entry_id, NULL AS exact_digest,
                NULL AS exact_bytes, m.group_id AS similar_group_id, m.distance,
                EXISTS(SELECT 1 FROM similarity_preferences AS preferred
                  WHERE preferred.group_id = m.group_id AND preferred.fingerprint_scheme = f.scheme
                    AND preferred.digest = f.digest AND preferred.byte_length = f.byte_length) AS preferred".to_owned(),
            format!("JOIN similarity_evidence AS e ON e.fingerprint_scheme = f.scheme
                AND e.digest = f.digest AND e.byte_length = f.byte_length
                AND e.scheme = '{SIMILARITY_SCHEME}' AND e.disposition = 'ready'
              JOIN similarity_members AS m ON m.evidence_id = e.evidence_id
              CROSS JOIN similarity_groups AS g ON g.group_id = m.group_id AND g.scheme = '{SIMILARITY_SCHEME}'"),
            format!("m.distance <= {} AND COALESCE((SELECT ignored FROM similarity_decisions
                WHERE group_id = m.group_id), 0) = 0
              AND NOT EXISTS(SELECT 1 FROM similarity_exclusions AS excluded
                WHERE excluded.group_id = m.group_id AND excluded.fingerprint_scheme = f.scheme
                  AND excluded.digest = f.digest AND excluded.byte_length = f.byte_length)",
              SimilarityDegree::Standard.distance()),
        ),
        _ => (
            format!("'exact:' || {content} AS entry_id, f.digest AS exact_digest,
              f.byte_length AS exact_bytes, NULL AS similar_group_id, 0 AS distance,
              COALESCE({exact_preference}, 0) AS preferred"),
            String::new(),
            if include_ignored { "1" } else { "COALESCE(decision.ignored, 0) = 0" }.to_owned(),
        ),
    };
    // Resolve each member before looking up its group; starting from the scheme's
    // groups makes SQLite scan the fingerprint relation again for every group.
    format!(
        "SELECT locations.*, {group_columns}, {content} AS content_key,
        f.scheme AS content_scheme, f.digest AS content_digest, f.byte_length AS content_bytes,
        f.is_pending, COALESCE({exact_preference}, 0) AS location_preferred,
        {rank} AS semantic_rank
      {source} JOIN {DISPLAY_FINGERPRINTS} ON {compatible} {DECISION_JOIN} {joins}
      WHERE ({conditions}) AND ({membership})"
    )
}
