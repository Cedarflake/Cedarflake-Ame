use rusqlite::types::Value;
use rusqlite::{OptionalExtension, Transaction, params_from_iter};

use crate::domain::semantic_results::SemanticResultManifest;
use crate::domain::{CatalogCursor, GalleryQuery, ScanError};

use super::{database_error, gallery_source, push_gallery_filters, sqlite_integer};

pub(super) fn source(
    manifest: &SemanticResultManifest,
    parameters: &mut Vec<Value>,
) -> &'static str {
    let members: Vec<_> = manifest
        .items
        .iter()
        .map(|item| serde_json::json!([item.asset_id, item.location_id, item.source_generation]))
        .collect();
    parameters.push(Value::Text(serde_json::Value::Array(members).to_string()));
    "FROM json_each(?) AS semantic_result
     CROSS JOIN asset_locations AS locations
       ON json_extract(semantic_result.value, '$[0]') = locations.asset_id
      AND json_extract(semantic_result.value, '$[1]') = locations.location_id
      AND json_extract(semantic_result.value, '$[2]') = locations.source_generation
     JOIN library_roots AS roots
       ON roots.id = locations.root_id AND roots.active_scan_id = locations.scan_id"
}

pub(super) fn resolve_predecessor(
    transaction: &Transaction<'_>,
    revision: u64,
    query: &GalleryQuery,
    query_id: &str,
    ordinal: u64,
) -> Result<CatalogCursor, ScanError> {
    let mut parameters = Vec::new();
    let source = gallery_source(query, &mut parameters);
    let mut clauses = Vec::new();
    push_gallery_filters(query, &mut clauses, &mut parameters);
    let scope = if clauses.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", clauses.join(" AND "))
    };
    parameters.push(Value::Integer(sqlite_integer(
        ordinal.saturating_sub(1),
        "semantic result offset",
    )?));
    let number = super::gallery_order_expressions(query).number;
    let group_columns = super::groups::columns(query);
    let identity = super::groups::identity(query);
    let sql = format!(
        "SELECT locations.root_id, locations.location_id, {number}, {group_columns}
         {source} {scope}
         ORDER BY {number}, locations.root_id, {identity}
         LIMIT 1 OFFSET ?"
    );
    transaction
        .query_row(&sql, params_from_iter(parameters), |row| {
            Ok(CatalogCursor {
                revision,
                query_id: query_id.to_owned(),
                root_id: row.get(0)?,
                location_id: row.get(1)?,
                group_key: super::groups::read_key(row)?,
                primary_missing: false,
                primary_text: String::new(),
                primary_number: row.get(2)?,
            })
        })
        .optional()
        .map_err(database_error)?
        .ok_or_else(|| {
            ScanError::new(
                "catalog_time_anchor_invalid",
                "The selected position is outside the semantic results",
            )
        })
}
