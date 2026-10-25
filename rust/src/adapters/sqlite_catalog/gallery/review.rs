use rusqlite::{Connection, OptionalExtension, params_from_iter, types::Value};

use crate::domain::organization::OrganizationSelection;
use crate::domain::{CatalogCursor, GalleryQuery, ScanError};

use super::super::{database_error, exact_duplicates::read_unsigned};
use super::{gallery_direction_sql, gallery_order_expressions, organization, push_cursor_filter};

pub(in super::super) struct ReviewBoundary {
    pub admission_id: i64,
    pub root_id: String,
    pub location_id: String,
    pub primary_missing: bool,
    pub primary_text: String,
    pub primary_number: i64,
}

pub(in super::super) struct ReviewCandidate {
    pub selection: OrganizationSelection,
    pub boundary: ReviewBoundary,
}

pub(in super::super) enum ReviewRead<'a> {
    Current(&'a str),
    After {
        boundary: &'a ReviewBoundary,
        exclude_record: &'a str,
    },
    First,
}

pub(in super::super) fn candidate(
    connection: &Connection,
    query: &GalleryQuery,
    request: ReviewRead<'_>,
) -> Result<Option<ReviewCandidate>, ScanError> {
    let mut clauses = Vec::new();
    let mut parameters = Vec::new();
    organization::push_group_filter(query, &mut clauses, &mut parameters);
    if let ReviewRead::Current(record_id) = request {
        clauses.push(format!(
            "{} = (SELECT group_id FROM organization_record_groups WHERE record_id = ?)",
            organization::record_expression("locations")
        ));
        parameters.push(Value::Text(record_id.to_owned()));
    }
    let admission = super::organization_queue::admission_expression("locations");
    let order = gallery_order_expressions(query);
    let direction = gallery_direction_sql(query.order_direction());
    if let ReviewRead::After {
        boundary: after,
        exclude_record,
    } = request
    {
        clauses.push(format!(
            "{} IS NOT (SELECT group_id FROM organization_record_groups WHERE record_id = ?)",
            organization::record_expression("locations")
        ));
        parameters.push(Value::Text(exclude_record.to_owned()));
        let cursor = CatalogCursor {
            revision: 0,
            query_id: String::new(),
            primary_missing: after.primary_missing,
            primary_text: after.primary_text.clone(),
            primary_number: after.primary_number,
            root_id: after.root_id.clone(),
            location_id: after.location_id.clone(),
        };
        push_cursor_filter(
            &cursor,
            &order,
            query.order_direction(),
            false,
            &mut clauses,
            &mut parameters,
        );
    }
    connection
        .query_row(
            &format!(
                "SELECT locations.asset_id, locations.location_id, locations.source_generation,
                    locations.root_id, {admission}, {missing}, {text}, {number}
             FROM library_roots AS roots
             JOIN asset_locations AS locations ON locations.scan_id = roots.active_scan_id
             WHERE {} ORDER BY {missing}, {text} {direction}, {number} {direction},
               locations.root_id, locations.location_id LIMIT 1",
                clauses.join(" AND "),
                missing = order.missing,
                text = order.text,
                number = order.number,
            ),
            params_from_iter(parameters),
            |row| {
                Ok(ReviewCandidate {
                    selection: OrganizationSelection {
                        asset_id: row.get(0)?,
                        location_id: row.get(1)?,
                        source_generation: read_unsigned(row, 2)?,
                    },
                    boundary: ReviewBoundary {
                        admission_id: row.get(4)?,
                        root_id: row.get(3)?,
                        location_id: row.get(1)?,
                        primary_missing: row.get(5)?,
                        primary_text: row.get(6)?,
                        primary_number: row.get(7)?,
                    },
                })
            },
        )
        .optional()
        .map_err(database_error)
}
