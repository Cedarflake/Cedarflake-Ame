use rusqlite::types::Value;

use crate::domain::GalleryQuery;

use super::duplicates::{DISPLAY_FINGERPRINTS, compatible_fingerprint};
use super::organization;

pub(in super::super) use super::organization_counts::load_counts;

fn disposition_expression(location: &str) -> String {
    let record = organization::record_expression(location);
    format!(
        "COALESCE((SELECT disposition FROM organization_records
                   WHERE record_id = {record}), 'pending')"
    )
}

pub(super) fn push_filter(
    query: &GalleryQuery,
    clauses: &mut Vec<String>,
    parameters: &mut Vec<Value>,
) {
    if let Some(queue) = query.organization_queue {
        clauses.push(format!("{} = ?", disposition_expression("locations")));
        parameters.push(Value::Text(queue.disposition().stored().to_owned()));
    }
}

pub(super) fn admission_expression(location: &str) -> String {
    let record = organization::record_expression(location);
    let compatible = compatible_fingerprint(location);
    // Only admitted source versions and published content evidence can supply an older position.
    // The current representative and a newly discovered copy cannot replace the group's first entry.
    format!(
        "(SELECT MIN(admission_id) FROM (
           SELECT admitted.admission_id FROM organization_queue_versions AS admitted
           WHERE admitted.source_generation = {location}.source_generation
           UNION ALL
           SELECT MIN(admitted.admission_id)
           FROM organization_record_groups AS lineage
           JOIN organization_bindings AS binding ON binding.record_id = lineage.record_id
           JOIN organization_queue_versions AS admitted
             ON admitted.source_generation = binding.source_generation
           WHERE lineage.group_id = {record}
           UNION ALL
           SELECT MIN(admitted.admission_id)
           FROM content_fingerprints AS history
           JOIN gallery_content_fingerprints AS published
             ON published.source_generation = history.source_generation
               AND published.scheme = history.scheme
           JOIN organization_queue_versions AS admitted
             ON admitted.source_generation = history.source_generation
           WHERE (history.scheme, history.digest, history.byte_length) = (
             SELECT f.scheme, f.digest, f.byte_length
             FROM {DISPLAY_FINGERPRINTS} WHERE {compatible})
         ))"
    )
}
