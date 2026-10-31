use rusqlite::types::Value;

use crate::domain::classification::CategoryId;
use crate::domain::classification_review::ClassificationReviewFilter;
use crate::domain::{GalleryCollection, GalleryQuery, ScanError};

use super::duplicates::{DISPLAY_FINGERPRINTS, compatible_fingerprint};
use super::organization::record_expression;

pub(super) fn validate(query: &GalleryQuery) -> Result<(), ScanError> {
    let GalleryCollection::SmartAlbum { category_id, .. } = &query.collection else {
        return Ok(());
    };
    if query.organization_queue.is_some() || query.new_since_admission.is_some() {
        return Err(ScanError::new(
            "classification_scope_invalid",
            "Smart albums cannot use organization queues",
        ));
    }
    if let Some(category) = category_id {
        CategoryId::new(category.clone()).map_err(|_| {
            ScanError::new(
                "classification_scope_invalid",
                "The category identifier is invalid",
            )
        })?;
    }
    Ok(())
}

pub(super) fn push_filter(
    query: &GalleryQuery,
    clauses: &mut Vec<String>,
    parameters: &mut Vec<Value>,
) {
    let GalleryCollection::SmartAlbum {
        category_id,
        review,
    } = &query.collection
    else {
        return;
    };
    let mut conditions = Vec::new();
    if let Some(category) = category_id {
        conditions.push("category_id = ?");
        parameters.push(Value::Text(category.clone()));
    }
    match review {
        ClassificationReviewFilter::All => {}
        ClassificationReviewFilter::Pending => {
            conditions.push("review IN ('needs_review', 'deferred', 'conflict')");
        }
        ClassificationReviewFilter::Confirmed => conditions.push("review = 'confirmed'"),
    }
    if conditions.is_empty() {
        return;
    }
    let record = record_expression("locations");
    let fingerprint = compatible_fingerprint("locations");
    // The projection shares the gallery's published/retained display membership.
    // Classification commands independently require fresh content and field authority.
    clauses.push(format!(
        "EXISTS (
           WITH human AS MATERIALIZED (
             SELECT * FROM classification_human_projection WHERE group_id = {record}
           ), prediction AS MATERIALIZED (
             SELECT prediction.category_id, prediction.review
             FROM {DISPLAY_FINGERPRINTS}
             JOIN classification_prediction_projection AS prediction
               ON prediction.fingerprint_scheme = f.scheme AND prediction.digest = f.digest
                 AND prediction.byte_length = f.byte_length
             WHERE {fingerprint}
             ORDER BY prediction.publication_id DESC LIMIT 1
           ), effective AS (
             SELECT CASE
                 WHEN human.choice_count > 1 THEN NULL
                 WHEN human.choice_count = 1 THEN human.category_id
                 ELSE prediction.category_id
               END AS category_id,
               CASE
                 WHEN human.choice_count > 1 THEN 'conflict'
                 WHEN human.choice_count = 1 THEN 'confirmed'
                 WHEN human.deferred = 1 THEN 'deferred'
                 ELSE COALESCE(prediction.review, 'unclassified')
               END AS review
             FROM (SELECT 1) AS subject LEFT JOIN human ON 1 LEFT JOIN prediction ON 1
           )
           SELECT 1 FROM effective WHERE {}
         )",
        conditions.join(" AND ")
    ));
}
