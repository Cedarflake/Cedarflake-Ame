use rusqlite::Connection;

use crate::domain::ScanError;

use super::schema_shape;

const OBJECTS: &[(&str, &str, &str)] = &[
    (
        "view",
        "classification_human_projection",
        "CREATE VIEW classification_human_projection AS
         SELECT group_id,
           CASE WHEN local_choices > 0 THEN local_choices ELSE restored_choices END AS choice_count,
           CASE WHEN local_choices > 0 THEN local_category ELSE restored_category END AS category_id,
           COALESCE(local_deferred, restored_deferred, 0) AS deferred
         FROM (
           SELECT member.group_id,
             COUNT(DISTINCT CASE WHEN decision.choice_restored = 0 THEN decision.category_id END) AS local_choices,
             COUNT(DISTINCT CASE WHEN decision.choice_restored = 1 THEN decision.category_id END) AS restored_choices,
             MIN(CASE WHEN decision.choice_restored = 0 THEN decision.category_id END) AS local_category,
             MIN(CASE WHEN decision.choice_restored = 1 THEN decision.category_id END) AS restored_category,
             MAX(CASE WHEN decision.review_restored = 0 AND decision.review_revision > 0
                 THEN decision.deferred END) AS local_deferred,
             MAX(CASE WHEN decision.review_restored = 1 AND decision.review_revision > 0
                 THEN decision.deferred END) AS restored_deferred
           FROM organization_record_groups AS member
           JOIN classification_decisions AS decision USING(record_id)
           GROUP BY member.group_id
         )",
    ),
    (
        "view",
        "classification_prediction_projection",
        "CREATE VIEW classification_prediction_projection AS
         SELECT prediction.publication_id, prediction.fingerprint_scheme, prediction.digest,
           prediction.byte_length,
           (SELECT json_extract(score.value, '$.category')
            FROM json_each(COALESCE(
              json_extract(prediction.payload, '$.refined.categories'),
              json_extract(prediction.payload, '$.initial.categories'))) AS score
            ORDER BY json_extract(score.value, '$.similarity') DESC,
              json_extract(score.value, '$.category') COLLATE BINARY LIMIT 1) AS category_id,
           CASE json_extract(prediction.payload, '$.review')
             WHEN 'routine_sampling' THEN 'suggested'
             ELSE 'needs_review'
           END AS review
         FROM classification_predictions AS prediction
         JOIN classification_state AS state
           ON state.singleton = 1 AND state.configuration_revision = prediction.configuration_revision
         JOIN classification_runs AS run ON run.run_id = prediction.run_id
           AND run.primary_rules = state.active_rules",
    ),
];

pub(super) fn validate(connection: &Connection) -> Result<(), ScanError> {
    for (kind, name, statement) in OBJECTS {
        if !schema_shape::schema_object_sql_matches(connection, kind, name, statement)? {
            return Err(ScanError::new(
                "classification_projection_invalid",
                "The classification query projection is incompatible",
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
