use rusqlite::{Transaction, params_from_iter, types::Value};

use crate::domain::exact_duplicates::EXACT_FINGERPRINT_SCHEME;
use crate::domain::{GalleryCollection, GalleryQuery, ScanError};

use super::super::exact_duplicate_review::representative_expression;
use super::duplicates::{DISPLAY_FINGERPRINTS, compatible_fingerprint};
use super::filters::push_membership_filters;
use super::organization::{compatible_binding, group_expression};
use super::{BuiltGalleryQuery, database_error, gallery_source, push_source_filters};

pub(super) fn uses_grouped_organization(query: &GalleryQuery) -> bool {
    query.folds_duplicates()
        && (query.organization_queue.is_some() || query.collection != GalleryCollection::Library)
}

pub(super) fn build_group_query(query: &GalleryQuery) -> BuiltGalleryQuery {
    let mut clauses = Vec::new();
    let mut parameters = Vec::new();
    let source = gallery_source(query, &mut parameters);
    push_source_filters(query, &mut clauses, &mut parameters);
    let scope = if clauses.is_empty() {
        "1".to_owned()
    } else {
        clauses.join(" AND ")
    };
    let joins = representative_joins();
    let representative = representative_expression("locations");
    let group = group_expression("locations");
    BuiltGalleryQuery {
        sql: format!(
            "SELECT {representative} AS location_id, COUNT(*) AS source_items
             {source} {joins} WHERE {scope} GROUP BY {group}"
        ),
        parameters,
    }
}

pub(super) fn push_group_filter(
    query: &GalleryQuery,
    clauses: &mut Vec<String>,
    parameters: &mut Vec<Value>,
) {
    let grouped = build_group_query(query);
    clauses.push(format!(
        "locations.location_id IN (SELECT location_id FROM ({}))",
        grouped.sql
    ));
    parameters.extend(grouped.parameters);
}

pub(super) fn representative_for_anchor(
    transaction: &Transaction<'_>,
    query: &GalleryQuery,
    location_id: &str,
    asset_id: Option<&str>,
) -> Result<Option<String>, ScanError> {
    let mut scope = Vec::new();
    let mut parameters = Vec::new();
    let source = gallery_source(query, &mut parameters);
    push_source_filters(query, &mut scope, &mut parameters);
    parameters.push(Value::Text(location_id.to_owned()));
    let asset_guard = if let Some(asset_id) = asset_id {
        parameters.push(Value::Text(asset_id.to_owned()));
        "AND requested.asset_id = ?"
    } else {
        ""
    };
    let current_group = group_expression("locations");
    let requested_group = group_expression("requested");
    scope.push(format!(
        "{current_group} = (
          SELECT {requested_group} FROM asset_locations AS requested
          JOIN library_roots AS requested_root ON requested.scan_id = requested_root.active_scan_id
          WHERE requested.location_id = ? {asset_guard})"
    ));
    let joins = representative_joins();
    let representative = representative_expression("locations");
    let eligibility = requested_membership(query, location_id, asset_id);
    let mut guarded_parameters = eligibility.parameters;
    guarded_parameters.extend(parameters);
    transaction
        .query_row(
            &format!(
                "SELECT CASE WHEN ({eligibility}) THEN (
                 SELECT {representative}
                 {source} {joins}
                 WHERE {scope}) END",
                eligibility = eligibility.sql,
                scope = scope.join(" AND "),
            ),
            params_from_iter(guarded_parameters),
            |row| row.get(0),
        )
        .map_err(database_error)
}

fn requested_membership(
    query: &GalleryQuery,
    location_id: &str,
    asset_id: Option<&str>,
) -> BuiltGalleryQuery {
    let mut parameters = Vec::new();
    let source = gallery_source(query, &mut parameters);
    let mut clauses = vec!["locations.location_id = ?".to_owned()];
    parameters.push(Value::Text(location_id.to_owned()));
    if let Some(asset_id) = asset_id {
        clauses.push("locations.asset_id = ?".to_owned());
        parameters.push(Value::Text(asset_id.to_owned()));
    }
    // A group has one organization decision even when its requested copy is outside
    // the physical query scope; a qualifying copy is selected by the outer read.
    push_membership_filters(query, &mut clauses, &mut parameters);
    BuiltGalleryQuery {
        sql: format!(
            "SELECT EXISTS(SELECT 1 {source} WHERE {})",
            clauses.join(" AND "),
        ),
        parameters,
    }
}

fn representative_joins() -> String {
    let bound = compatible_binding("locations");
    let compatible = compatible_fingerprint("locations");
    // Joining the union view materializes all historical fingerprints. Resolve the
    // compatible evidence version first so the optional join remains a point lookup.
    format!(
        "LEFT JOIN organization_bindings AS binding ON {bound}
         LEFT JOIN content_fingerprints AS f
           ON f.source_generation = (
             SELECT f.evidence_source_generation FROM {DISPLAY_FINGERPRINTS} WHERE {compatible})
           AND f.scheme = '{EXACT_FINGERPRINT_SCHEME}'
         LEFT JOIN exact_duplicate_decisions AS decision
           ON decision.scheme = COALESCE(f.scheme, binding.fingerprint_scheme)
           AND decision.digest = COALESCE(f.digest, binding.digest)
           AND decision.byte_length = COALESCE(f.byte_length, binding.byte_length)"
    )
}
