use rusqlite::types::Value;
use rusqlite::{OptionalExtension, Transaction, params_from_iter};

use crate::domain::{CatalogCursor, GalleryQuery, GallerySortKey, GalleryTimeAnchor, ScanError};

use super::{
    database_error, gallery_cursor_for_asset, gallery_direction_sql, gallery_order_expressions,
    push_gallery_filters, read_stored_asset, sqlite_integer, stored_asset_view,
    validate_month_key_text,
};

pub(in super::super) fn resolve_gallery_anchor_cursor(
    transaction: &Transaction<'_>,
    revision: u64,
    query: &GalleryQuery,
    query_id: &str,
    anchor: &GalleryTimeAnchor,
) -> Result<CatalogCursor, ScanError> {
    let order = gallery_order_expressions(&query.sort_key);
    let Some(month_expression) = order.month else {
        return Err(ScanError::new(
            "catalog_time_anchor_unavailable",
            "Name-sorted gallery results do not have a chronological time anchor",
        ));
    };
    let mut clauses = Vec::new();
    let mut parameters = Vec::new();
    push_gallery_filters(query, &mut clauses, &mut parameters);
    match &anchor.month_key {
        Some(month_key) => {
            let month = validate_month_key_text(month_key)?;
            clauses.push(format!("{month_expression} = ?"));
            parameters.push(Value::Text(month_key.clone()));
            if matches!(
                query.sort_key,
                GallerySortKey::CaptureTime | GallerySortKey::CreatedTime
            ) {
                clauses.push(format!(
                    "{} = 0 AND {} >= ? AND {} < ?",
                    order.missing, order.text, order.text
                ));
                parameters.push(Value::Text(month_key.clone()));
                // This is a lexical prefix boundary; December ends at YYYY-13.
                let exclusive_end = format!("{}{:02}", &month_key[..5], month + 1);
                parameters.push(Value::Text(exclusive_end));
            }
        }
        None if matches!(query.sort_key, GallerySortKey::ModifiedTime) => {
            return Err(ScanError::new(
                "catalog_time_anchor_invalid",
                "Modification-time results do not contain an unknown-date section",
            ));
        }
        None => {
            clauses.push(format!("{month_expression} IS NULL"));
            clauses.push(format!("{} = 1", order.missing));
        }
    }
    let preceding_offset = sqlite_integer(
        anchor.item_offset.saturating_sub(1),
        "gallery time-anchor item offset",
    )?;
    parameters.push(Value::Integer(preceding_offset));
    let direction = gallery_direction_sql(&query.sort_direction);
    let sql = format!(
        "SELECT locations.asset_id, locations.location_id, locations.root_id,
                locations.scan_id,
                locations.absolute_path, locations.relative_path,
                locations.preview_path, locations.file_size,
                locations.created_unix_ms, locations.modified_unix_ms,
                locations.width, locations.height,
                locations.preview_status, locations.preview_issue_code,
                locations.preview_issue_message, locations.metadata_engine_id,
                locations.metadata_engine_version, locations.capture_local_time,
                locations.capture_offset_minutes, locations.capture_time_source,
                locations.capture_raw_value, locations.file_identity_scheme,
                locations.file_identity_value, locations.source_revision_token,
                locations.source_generation
         FROM library_roots AS roots
         JOIN asset_locations AS locations
           ON locations.scan_id = roots.active_scan_id
         WHERE {where_clause}
         ORDER BY {missing}, {text} {direction}, {number} {direction},
                  locations.root_id, locations.location_id
         LIMIT 1 OFFSET ?",
        where_clause = clauses.join(" AND "),
        missing = order.missing,
        text = order.text,
        number = order.number,
    );
    let mut statement = transaction.prepare(&sql).map_err(database_error)?;
    let stored = statement
        .query_row(params_from_iter(parameters.iter()), read_stored_asset)
        .optional()
        .map_err(database_error)?
        .ok_or_else(|| {
            ScanError::new(
                "catalog_time_anchor_invalid",
                "The selected position is outside its gallery time bucket",
            )
        })?;
    let asset = stored_asset_view(stored)?;
    gallery_cursor_for_asset(transaction, revision, query_id, query, &asset)
}
