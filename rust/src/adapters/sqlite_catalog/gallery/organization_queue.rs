use rusqlite::types::Value;
use rusqlite::{Transaction, params_from_iter};

use crate::domain::organization_queue::OrganizationCounts;
use crate::domain::{GalleryQuery, ScanError};

use super::super::{database_error, sqlite_unsigned};
use super::duplicates::{DISPLAY_FINGERPRINTS, compatible_fingerprint};
use super::{organization, push_source_filters};

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

pub(in super::super) fn load_counts(
    transaction: &Transaction<'_>,
    query: &GalleryQuery,
) -> Result<OrganizationCounts, ScanError> {
    let mut scope = query.clone();
    scope.organization_queue = None;
    let mut clauses = Vec::new();
    let mut parameters = Vec::new();
    push_source_filters(&scope, &mut clauses, &mut parameters);
    let scope = if clauses.is_empty() {
        "1".to_owned()
    } else {
        clauses.join(" AND ")
    };
    let group = organization::group_expression("locations");
    let disposition = disposition_expression("locations");
    // A canonical group has one disposition; counts do not need a preferred representative.
    let counts: [i64; 5] = transaction
        .query_row(
            &format!(
                "SELECT COUNT(*),
                        COALESCE(SUM(disposition = 'pending'), 0),
                        COALESCE(SUM(disposition = 'deferred'), 0),
                        COALESCE(SUM(disposition = 'organized'), 0),
                        COALESCE(SUM(disposition = 'ignored'), 0)
                 FROM (
                   SELECT {disposition} AS disposition FROM library_roots AS roots
                   CROSS JOIN asset_locations AS locations
                     ON locations.root_id = roots.id AND locations.scan_id = roots.active_scan_id
                   WHERE {scope} GROUP BY {group}
                 )"
            ),
            params_from_iter(parameters),
            |row| {
                Ok([
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ])
            },
        )
        .map_err(database_error)?;
    Ok(OrganizationCounts {
        total: sqlite_unsigned(counts[0], "organization total groups")?,
        pending: sqlite_unsigned(counts[1], "organization pending groups")?,
        deferred: sqlite_unsigned(counts[2], "organization deferred groups")?,
        organized: sqlite_unsigned(counts[3], "organization organized groups")?,
        ignored: sqlite_unsigned(counts[4], "organization ignored groups")?,
    })
}
