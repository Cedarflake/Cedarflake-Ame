mod anchor_position;
mod annotations;
mod classification;
pub(super) mod duplicates;
mod filters;
mod location_anchor;
mod member_locations;
mod metadata_filters;
mod ordering;
pub(super) mod organization;
mod organization_counts;
mod organization_groups;
pub(super) mod organization_queue;
pub(super) mod retained_membership;
pub(super) mod review;
mod scroll_sections;
mod search;
pub(super) mod selection;
mod time_anchor;
mod timeline;

pub(super) use timeline::load_gallery_timeline;

use filters::{gallery_source, push_gallery_filters, push_source_filters};

pub(super) use location_anchor::{
    GalleryAssetAnchor, resolve_gallery_asset_anchor, resolve_gallery_location_anchor,
};

pub(super) use time_anchor::resolve_gallery_anchor_cursor;

use rusqlite::types::Value;
use rusqlite::{OptionalExtension, Transaction, params};

use ordering::{GalleryOrderExpressions, gallery_order_expressions};

use crate::domain::{
    AssetLocationView, CatalogCursor, GalleryQuery, GallerySortDirection, GallerySortKey,
    GalleryTimeAnchor, ScanError,
};

use super::{
    database_error, normalize_relative_folder, read_stored_asset, sqlite_integer, stored_asset_view,
};

pub(super) struct BuiltGalleryQuery {
    pub(super) sql: String,
    pub(super) parameters: Vec<Value>,
}

pub(super) fn validate_gallery_query(query: &GalleryQuery) -> Result<(), ScanError> {
    classification::validate(query)?;
    query.metadata_filter.validate()?;
    if let Some(boundary) = query.new_since_admission {
        super::sqlite_integer(boundary, "new-arrival boundary")?;
    }
    if query.search_text.chars().count() > 512 {
        return Err(ScanError::new(
            "catalog_search_invalid",
            "Gallery search text cannot exceed 512 characters",
        ));
    }
    if query.folder_relative_path.is_some() && query.root_id.is_none() {
        return Err(ScanError::new(
            "catalog_source_scope_invalid",
            "A folder scope requires a library root",
        ));
    }
    if let Some(folder) = &query.folder_relative_path {
        let normalized = normalize_relative_folder(folder);
        if normalized.is_empty()
            || normalized.starts_with('/')
            || normalized.split('/').any(|component| component == "..")
        {
            return Err(ScanError::new(
                "catalog_source_scope_invalid",
                "A folder scope must stay inside its library root",
            ));
        }
    }
    Ok(())
}

pub(super) fn build_gallery_asset_query(
    query: &GalleryQuery,
    after: Option<&CatalogCursor>,
    before: Option<&CatalogCursor>,
    anchor: Option<&GalleryTimeAnchor>,
    sql_limit: i64,
) -> Result<BuiltGalleryQuery, ScanError> {
    let order = gallery_order_expressions(query);
    let is_backward = before.is_some();
    let primary_direction = gallery_window_direction_sql(query.order_direction(), is_backward);
    let missing_direction = if is_backward { "DESC" } else { "ASC" };
    let tie_direction = if is_backward { "DESC" } else { "ASC" };
    let mut clauses = Vec::new();
    let mut parameters = Vec::new();
    let source = gallery_source(query, &mut parameters);
    push_gallery_filters(query, &mut clauses, &mut parameters);
    if let Some(anchor) = anchor {
        push_time_anchor_filter(query, anchor, &order, &mut clauses, &mut parameters)?;
    }
    if let Some(cursor) = after {
        push_cursor_filter(
            cursor,
            &order,
            query.order_direction(),
            false,
            &mut clauses,
            &mut parameters,
        );
    }
    if let Some(cursor) = before {
        push_cursor_filter(
            cursor,
            &order,
            query.order_direction(),
            true,
            &mut clauses,
            &mut parameters,
        );
    }
    let where_clause = if clauses.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", clauses.join(" AND "))
    };
    parameters.push(Value::Integer(sql_limit));
    Ok(BuiltGalleryQuery {
        sql: format!(
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
                    locations.source_generation, locations.camera_make, locations.camera_model
             {source}
             {where_clause}
             ORDER BY {missing} {missing_direction},
                      {text} {primary_direction}, {number} {primary_direction},
                      locations.root_id {tie_direction},
                      locations.location_id {tie_direction}
             LIMIT ?",
            missing = order.missing,
            text = order.text,
            number = order.number,
            missing_direction = missing_direction,
            primary_direction = primary_direction,
            tie_direction = tie_direction,
        ),
        parameters,
    })
}

pub(super) fn build_gallery_layout_manifest_query(
    query: &GalleryQuery,
    after: Option<&CatalogCursor>,
    sql_limit: i64,
) -> Result<BuiltGalleryQuery, ScanError> {
    let order = gallery_order_expressions(query);
    let mut clauses = Vec::new();
    let mut parameters = Vec::new();
    let source = gallery_source(query, &mut parameters);
    push_gallery_filters(query, &mut clauses, &mut parameters);
    if let Some(cursor) = after {
        push_cursor_filter(
            cursor,
            &order,
            query.order_direction(),
            false,
            &mut clauses,
            &mut parameters,
        );
    }
    let where_clause = if clauses.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", clauses.join(" AND "))
    };
    parameters.push(Value::Integer(sql_limit));
    let primary_direction = gallery_direction_sql(query.order_direction());
    let date = order.date.unwrap_or("NULL");
    Ok(BuiltGalleryQuery {
        sql: format!(
            "SELECT locations.location_id, locations.root_id,
                    locations.width, locations.height,
                    {date} AS date_key,
                    {missing} AS primary_missing,
                    {text} AS primary_text,
                    {number} AS primary_number
             {source}
             {where_clause}
             ORDER BY {missing}, {text} {primary_direction},
                      {number} {primary_direction},
                      locations.root_id, locations.location_id
             LIMIT ?",
            missing = order.missing,
            text = order.text,
            number = order.number,
        ),
        parameters,
    })
}

pub(super) fn build_gallery_count_query(query: &GalleryQuery) -> BuiltGalleryQuery {
    let mut clauses = Vec::new();
    let mut parameters = Vec::new();
    let source = gallery_source(query, &mut parameters);
    push_gallery_filters(query, &mut clauses, &mut parameters);
    let where_clause = if clauses.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", clauses.join(" AND "))
    };
    BuiltGalleryQuery {
        sql: format!(
            "SELECT COUNT(*)
             {source}
             {where_clause}",
        ),
        parameters,
    }
}

pub(super) fn gallery_cursor_for_asset(
    transaction: &Transaction<'_>,
    revision: u64,
    query_id: &str,
    query: &GalleryQuery,
    asset: &AssetLocationView,
) -> Result<CatalogCursor, ScanError> {
    let order = gallery_order_expressions(query);
    let sql = format!(
        "SELECT {missing}, {text}, {number}
         FROM library_roots AS roots
         JOIN asset_locations AS locations
           ON locations.scan_id = roots.active_scan_id
         WHERE locations.root_id = ?1 AND locations.location_id = ?2
         LIMIT 1",
        missing = order.missing,
        text = order.text,
        number = order.number,
    );
    let (primary_missing, primary_text, primary_number) = transaction
        .query_row(&sql, params![asset.root_id, asset.location_id], |row| {
            Ok((
                row.get::<_, i64>(0)? != 0,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
            ))
        })
        .optional()
        .map_err(database_error)?
        .ok_or_else(|| {
            ScanError::new(
                "catalog_cursor_asset_unavailable",
                "The gallery item is no longer available in the active catalog",
            )
        })?;
    Ok(CatalogCursor {
        revision,
        query_id: query_id.to_owned(),
        primary_missing,
        primary_text,
        primary_number,
        root_id: asset.root_id.clone(),
        location_id: asset.location_id.clone(),
    })
}

fn gallery_direction_sql(direction: &GallerySortDirection) -> &'static str {
    match direction {
        GallerySortDirection::Ascending => "ASC",
        GallerySortDirection::Descending => "DESC",
    }
}

fn gallery_window_direction_sql(
    direction: &GallerySortDirection,
    is_backward: bool,
) -> &'static str {
    match (direction, is_backward) {
        (GallerySortDirection::Ascending, false) | (GallerySortDirection::Descending, true) => {
            "ASC"
        }
        (GallerySortDirection::Descending, false) | (GallerySortDirection::Ascending, true) => {
            "DESC"
        }
    }
}

fn gallery_cursor_comparison(direction: &GallerySortDirection, is_backward: bool) -> &'static str {
    match (direction, is_backward) {
        (GallerySortDirection::Ascending, false) | (GallerySortDirection::Descending, true) => ">",
        (GallerySortDirection::Descending, false) | (GallerySortDirection::Ascending, true) => "<",
    }
}

fn push_time_anchor_filter(
    query: &GalleryQuery,
    anchor: &GalleryTimeAnchor,
    order: &GalleryOrderExpressions,
    clauses: &mut Vec<String>,
    parameters: &mut Vec<Value>,
) -> Result<(), ScanError> {
    let Some(month_expression) = order.month else {
        return Err(ScanError::new(
            "catalog_time_anchor_unavailable",
            "The selected gallery ordering does not have a chronological time anchor",
        ));
    };
    match &anchor.month_key {
        Some(month_key) => {
            validate_month_key_text(month_key)?;
            let comparison = gallery_cursor_comparison(query.order_direction(), false);
            clauses.push(format!(
                "({month_expression} IS NULL OR {month_expression} {comparison}= ?)"
            ));
            parameters.push(Value::Text(month_key.clone()));
        }
        None if matches!(query.sort_key, GallerySortKey::ModifiedTime) => {
            return Err(ScanError::new(
                "catalog_time_anchor_invalid",
                "Modification-time results do not contain an unknown-date section",
            ));
        }
        None => clauses.push(format!("{month_expression} IS NULL")),
    }
    Ok(())
}

fn push_cursor_filter(
    cursor: &CatalogCursor,
    order: &GalleryOrderExpressions,
    direction: &GallerySortDirection,
    is_backward: bool,
    clauses: &mut Vec<String>,
    parameters: &mut Vec<Value>,
) {
    let primary_comparison = gallery_cursor_comparison(direction, is_backward);
    let missing_comparison = if is_backward { "<" } else { ">" };
    let tie_comparison = if is_backward { "<" } else { ">" };
    clauses.push(format!(
        "(
          {missing} {missing_comparison} ?
          OR ({missing} = ? AND {text} {primary_comparison} ?)
          OR ({missing} = ? AND {text} = ? AND {number} {primary_comparison} ?)
          OR ({missing} = ? AND {text} = ? AND {number} = ?
              AND locations.root_id {tie_comparison} ?)
          OR ({missing} = ? AND {text} = ? AND {number} = ?
              AND locations.root_id = ? AND locations.location_id {tie_comparison} ?)
        )",
        missing = order.missing,
        text = order.text,
        number = order.number,
        missing_comparison = missing_comparison,
        primary_comparison = primary_comparison,
        tie_comparison = tie_comparison,
    ));
    let missing = Value::Integer(i64::from(cursor.primary_missing));
    let text = Value::Text(cursor.primary_text.clone());
    let number = Value::Integer(cursor.primary_number);
    let root = Value::Text(cursor.root_id.clone());
    parameters.extend([
        missing.clone(),
        missing.clone(),
        text.clone(),
        missing.clone(),
        text.clone(),
        number.clone(),
        missing.clone(),
        text.clone(),
        number.clone(),
        root.clone(),
        missing,
        text,
        number,
        root,
        Value::Text(cursor.location_id.clone()),
    ]);
}

fn validate_month_key_text(month_key: &str) -> Result<u8, ScanError> {
    let bytes = month_key.as_bytes();
    let valid_shape = bytes.len() == 7
        && bytes[4] == b'-'
        && bytes[..4].iter().all(u8::is_ascii_digit)
        && bytes[5..].iter().all(u8::is_ascii_digit);
    let month = valid_shape
        .then(|| month_key[5..].parse::<u8>().ok())
        .flatten();
    if let Some(month) = month.filter(|month| (1..=12).contains(month)) {
        return Ok(month);
    }
    Err(ScanError::new(
        "catalog_time_anchor_invalid",
        "A gallery month anchor must use YYYY-MM with a month from 01 through 12",
    ))
}
