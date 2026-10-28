use rusqlite::{Transaction, params_from_iter};

use crate::domain::organization_queue::OrganizationCounts;
use crate::domain::{GalleryQuery, ScanError};

use super::super::{database_error, sqlite_unsigned};
use super::filters::push_location_filters;
use super::organization::{
    collection_condition, group_expression_for_record, resolved_record_joins,
};

pub(in super::super) fn load_counts(
    transaction: &Transaction<'_>,
    query: &GalleryQuery,
) -> Result<OrganizationCounts, ScanError> {
    let mut clauses = Vec::new();
    let mut parameters = Vec::new();
    if let Some(condition) = collection_condition(&query.collection, &mut parameters) {
        clauses.push(condition.to_owned());
    }
    push_location_filters(query, &mut clauses, &mut parameters);
    let scope = if clauses.is_empty() {
        "1".to_owned()
    } else {
        clauses.join(" AND ")
    };
    let membership = resolved_record_joins("locations");
    let group = group_expression_for_record("locations", "record.record_id");
    // Counts span all queues in the physical/collection scope. Resolve each record once;
    // unrecorded content groups stay pending without selecting a preferred copy.
    let counts: [i64; 5] = transaction
        .query_row(
            &format!(
                "SELECT COUNT(*),
                        COALESCE(SUM(disposition = 'pending'), 0),
                        COALESCE(SUM(disposition = 'deferred'), 0),
                        COALESCE(SUM(disposition = 'organized'), 0),
                        COALESCE(SUM(disposition = 'ignored'), 0)
                 FROM (
                   SELECT COALESCE(record.disposition, 'pending') AS disposition
                   FROM library_roots AS roots CROSS JOIN asset_locations AS locations
                     ON locations.root_id = roots.id AND locations.scan_id = roots.active_scan_id
                   {membership}
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
