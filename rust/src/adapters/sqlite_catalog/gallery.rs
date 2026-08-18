use rusqlite::types::Value;
use rusqlite::{OptionalExtension, Transaction, params, params_from_iter};

use crate::domain::{
    AssetLocationView, CatalogCursor, GalleryLocationAnchorResolution, GalleryQuery,
    GallerySortDirection, GallerySortKey, GalleryTimeAnchor, ScanError,
};

use super::{
    database_error, normalize_relative_folder, read_stored_asset, sqlite_integer, stored_asset_view,
};

pub(super) struct BuiltGalleryQuery {
    pub(super) sql: String,
    pub(super) parameters: Vec<Value>,
}

struct GalleryAnchorRequest<'a> {
    requested_location_id: &'a str,
    anchor_column: &'static str,
    anchor_value: &'a str,
    max_items: u32,
}

pub(super) fn resolve_gallery_location_anchor(
    transaction: &Transaction<'_>,
    revision: u64,
    query: &GalleryQuery,
    query_id: &str,
    requested_location_id: &str,
    max_items: u32,
) -> Result<(GalleryLocationAnchorResolution, Option<CatalogCursor>), ScanError> {
    resolve_gallery_anchor(
        transaction,
        revision,
        query,
        query_id,
        GalleryAnchorRequest {
            requested_location_id,
            anchor_column: "locations.location_id",
            anchor_value: requested_location_id,
            max_items,
        },
    )
}

pub(super) fn resolve_gallery_asset_anchor(
    transaction: &Transaction<'_>,
    revision: u64,
    query: &GalleryQuery,
    query_id: &str,
    requested_location_id: &str,
    asset_id: &str,
    max_items: u32,
) -> Result<(GalleryLocationAnchorResolution, Option<CatalogCursor>), ScanError> {
    resolve_gallery_anchor(
        transaction,
        revision,
        query,
        query_id,
        GalleryAnchorRequest {
            requested_location_id,
            anchor_column: "locations.asset_id",
            anchor_value: asset_id,
            max_items,
        },
    )
}

fn resolve_gallery_anchor(
    transaction: &Transaction<'_>,
    revision: u64,
    query: &GalleryQuery,
    query_id: &str,
    request: GalleryAnchorRequest<'_>,
) -> Result<(GalleryLocationAnchorResolution, Option<CatalogCursor>), ScanError> {
    let order = gallery_order_expressions(&query.sort_key);
    let mut anchor_clauses = Vec::new();
    let mut anchor_parameters = Vec::new();
    push_gallery_filters(query, &mut anchor_clauses, &mut anchor_parameters);
    anchor_clauses.push(format!("{} = ?", request.anchor_column));
    anchor_parameters.push(Value::Text(request.anchor_value.to_owned()));
    let anchor_sql = format!(
        "SELECT locations.root_id, locations.location_id,
                {missing}, {text}, {number}
         FROM library_roots AS roots
         JOIN asset_locations AS locations
           ON locations.scan_id = roots.active_scan_id
         WHERE {where_clause}
         LIMIT 1",
        missing = order.missing,
        text = order.text,
        number = order.number,
        where_clause = anchor_clauses.join(" AND "),
    );
    let anchor = transaction
        .query_row(
            &anchor_sql,
            params_from_iter(anchor_parameters.iter()),
            |row| {
                Ok(CatalogCursor {
                    revision,
                    query_id: query_id.to_owned(),
                    root_id: row.get(0)?,
                    location_id: row.get(1)?,
                    primary_missing: row.get::<_, i64>(2)? != 0,
                    primary_text: row.get(3)?,
                    primary_number: row.get(4)?,
                })
            },
        )
        .optional()
        .map_err(database_error)?;
    let Some(anchor) = anchor else {
        return Ok((
            GalleryLocationAnchorResolution {
                requested_location_id: request.requested_location_id.to_owned(),
                location_id: None,
                ordinal: None,
                window_start_ordinal: 0,
            },
            None,
        ));
    };

    let mut preceding_clauses = Vec::new();
    let mut preceding_parameters = Vec::new();
    push_gallery_filters(query, &mut preceding_clauses, &mut preceding_parameters);
    push_cursor_filter(
        &anchor,
        &order,
        &query.sort_direction,
        true,
        &mut preceding_clauses,
        &mut preceding_parameters,
    );
    let preceding_where = preceding_clauses.join(" AND ");
    let ordinal_i64: i64 = transaction
        .query_row(
            &format!(
                "SELECT COUNT(*)
                 FROM library_roots AS roots
                 JOIN asset_locations AS locations
                   ON locations.scan_id = roots.active_scan_id
                 WHERE {preceding_where}"
            ),
            params_from_iter(preceding_parameters.iter()),
            |row| row.get(0),
        )
        .map_err(database_error)?;
    let ordinal = u64::try_from(ordinal_i64).map_err(|_| {
        ScanError::new(
            "catalog_location_anchor_invalid",
            "The gallery location anchor ordinal is outside the supported range",
        )
    })?;
    let window_start_ordinal = ordinal.saturating_sub(u64::from(request.max_items / 2));
    let start_after = if window_start_ordinal == 0 {
        None
    } else {
        let reverse_offset = ordinal.saturating_sub(window_start_ordinal);
        let reverse_direction = gallery_window_direction_sql(&query.sort_direction, true);
        let reverse_offset = sqlite_integer(reverse_offset, "gallery location anchor offset")?;
        let predecessor_sql = format!(
            "SELECT locations.root_id, locations.location_id,
                    {missing}, {text}, {number}
             FROM library_roots AS roots
             JOIN asset_locations AS locations
               ON locations.scan_id = roots.active_scan_id
             WHERE {preceding_where}
             ORDER BY {missing} DESC, {text} {reverse_direction},
                      {number} {reverse_direction},
                      locations.root_id DESC, locations.location_id DESC
             LIMIT 1 OFFSET ?",
            missing = order.missing,
            text = order.text,
            number = order.number,
        );
        let mut predecessor_parameters = preceding_parameters;
        predecessor_parameters.push(Value::Integer(reverse_offset));
        Some(
            transaction
                .query_row(
                    &predecessor_sql,
                    params_from_iter(predecessor_parameters.iter()),
                    |row| {
                        Ok(CatalogCursor {
                            revision,
                            query_id: query_id.to_owned(),
                            root_id: row.get(0)?,
                            location_id: row.get(1)?,
                            primary_missing: row.get::<_, i64>(2)? != 0,
                            primary_text: row.get(3)?,
                            primary_number: row.get(4)?,
                        })
                    },
                )
                .map_err(database_error)?,
        )
    };
    Ok((
        GalleryLocationAnchorResolution {
            requested_location_id: request.requested_location_id.to_owned(),
            location_id: Some(anchor.location_id),
            ordinal: Some(ordinal),
            window_start_ordinal,
        },
        start_after,
    ))
}

struct GalleryOrderExpressions {
    missing: &'static str,
    text: &'static str,
    number: &'static str,
    date: Option<&'static str>,
    month: Option<&'static str>,
}

pub(super) fn validate_gallery_query(query: &GalleryQuery) -> Result<(), ScanError> {
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
    let order = gallery_order_expressions(&query.sort_key);
    let is_backward = before.is_some();
    let primary_direction = gallery_window_direction_sql(&query.sort_direction, is_backward);
    let missing_direction = if is_backward { "DESC" } else { "ASC" };
    let tie_direction = if is_backward { "DESC" } else { "ASC" };
    let mut clauses = Vec::new();
    let mut parameters = Vec::new();
    push_gallery_filters(query, &mut clauses, &mut parameters);
    if let Some(anchor) = anchor {
        push_time_anchor_filter(query, anchor, &order, &mut clauses, &mut parameters)?;
    }
    if let Some(cursor) = after {
        push_cursor_filter(
            cursor,
            &order,
            &query.sort_direction,
            false,
            &mut clauses,
            &mut parameters,
        );
    }
    if let Some(cursor) = before {
        push_cursor_filter(
            cursor,
            &order,
            &query.sort_direction,
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
                    locations.absolute_path, locations.relative_path,
                    locations.preview_path, locations.file_size,
                    locations.created_unix_ms, locations.modified_unix_ms,
                    locations.width, locations.height,
                    locations.preview_status, locations.preview_issue_code,
                    locations.preview_issue_message, locations.metadata_engine_id,
                    locations.metadata_engine_version, locations.capture_local_time,
                    locations.capture_offset_minutes, locations.capture_time_source,
                    locations.capture_raw_value, locations.file_identity_scheme,
                    locations.file_identity_value
             FROM library_roots AS roots
             JOIN asset_locations AS locations
               ON locations.scan_id = roots.active_scan_id
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

pub(super) fn resolve_gallery_anchor_cursor(
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
            validate_month_key_text(month_key)?;
            clauses.push(format!("{month_expression} = ?"));
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
    let preceding_offset = sqlite_integer(
        anchor.item_offset.saturating_sub(1),
        "gallery time-anchor item offset",
    )?;
    parameters.push(Value::Integer(preceding_offset));
    let direction = gallery_direction_sql(&query.sort_direction);
    let sql = format!(
        "SELECT locations.asset_id, locations.location_id, locations.root_id,
                locations.absolute_path, locations.relative_path,
                locations.preview_path, locations.file_size,
                locations.created_unix_ms, locations.modified_unix_ms,
                locations.width, locations.height,
                locations.preview_status, locations.preview_issue_code,
                locations.preview_issue_message, locations.metadata_engine_id,
                locations.metadata_engine_version, locations.capture_local_time,
                locations.capture_offset_minutes, locations.capture_time_source,
                locations.capture_raw_value, locations.file_identity_scheme,
                locations.file_identity_value
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

pub(super) fn build_gallery_timeline_query(query: &GalleryQuery) -> BuiltGalleryQuery {
    let order = gallery_order_expressions(&query.sort_key);
    let mut clauses = Vec::new();
    let mut parameters = Vec::new();
    push_gallery_filters(query, &mut clauses, &mut parameters);
    let where_clause = if clauses.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", clauses.join(" AND "))
    };
    let month = order.month.unwrap_or("NULL");
    let direction = gallery_direction_sql(&query.sort_direction);
    BuiltGalleryQuery {
        sql: format!(
            "SELECT {month} AS month_key,
                    COUNT(*),
                    SUM(
                      CASE
                        WHEN locations.width <= 0 OR locations.height <= 0 THEN 1000
                        WHEN locations.width * 5 < locations.height THEN 200
                        WHEN locations.width > locations.height * 5 THEN 5000
                        ELSE (locations.width * 1000) / locations.height
                      END
                    ) AS aspect_ratio_milli_sum
             FROM library_roots AS roots
             JOIN asset_locations AS locations
               ON locations.scan_id = roots.active_scan_id
             {where_clause}
             GROUP BY {month}
             ORDER BY (month_key IS NULL), month_key {direction}",
        ),
        parameters,
    }
}

pub(super) fn build_gallery_layout_manifest_query(
    query: &GalleryQuery,
    after: Option<&CatalogCursor>,
    sql_limit: i64,
) -> Result<BuiltGalleryQuery, ScanError> {
    let order = gallery_order_expressions(&query.sort_key);
    let mut clauses = Vec::new();
    let mut parameters = Vec::new();
    push_gallery_filters(query, &mut clauses, &mut parameters);
    if let Some(cursor) = after {
        push_cursor_filter(
            cursor,
            &order,
            &query.sort_direction,
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
    let primary_direction = gallery_direction_sql(&query.sort_direction);
    let date = order.date.unwrap_or("NULL");
    Ok(BuiltGalleryQuery {
        sql: format!(
            "SELECT locations.location_id, locations.root_id,
                    locations.width, locations.height,
                    {date} AS date_key,
                    {missing} AS primary_missing,
                    {text} AS primary_text,
                    {number} AS primary_number
             FROM library_roots AS roots
             JOIN asset_locations AS locations
               ON locations.scan_id = roots.active_scan_id
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
    push_gallery_filters(query, &mut clauses, &mut parameters);
    let where_clause = if clauses.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", clauses.join(" AND "))
    };
    BuiltGalleryQuery {
        sql: format!(
            "SELECT COUNT(*)
             FROM library_roots AS roots
             JOIN asset_locations AS locations
               ON locations.scan_id = roots.active_scan_id
             {where_clause}"
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
    let order = gallery_order_expressions(&query.sort_key);
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

fn gallery_order_expressions(sort_key: &GallerySortKey) -> GalleryOrderExpressions {
    match sort_key {
        GallerySortKey::CaptureTime => GalleryOrderExpressions {
            missing: "(COALESCE(locations.capture_local_time, locations.file_local_time) IS NULL)",
            text: "IFNULL(COALESCE(locations.capture_local_time, locations.file_local_time), '')",
            number: "locations.modified_unix_ms",
            date: Some(
                "CASE \
                   WHEN COALESCE(locations.capture_local_time, locations.file_local_time) IS NULL \
                   THEN NULL \
                   ELSE substr(COALESCE(locations.capture_local_time, locations.file_local_time), 1, 10) \
                 END",
            ),
            month: Some(
                "CASE \
                   WHEN COALESCE(locations.capture_local_time, locations.file_local_time) IS NULL \
                   THEN NULL \
                   ELSE substr(COALESCE(locations.capture_local_time, locations.file_local_time), 1, 7) \
                 END",
            ),
        },
        GallerySortKey::CreatedTime => GalleryOrderExpressions {
            missing: "(locations.file_local_time IS NULL)",
            text: "IFNULL(locations.file_local_time, '')",
            number: "locations.modified_unix_ms",
            date: Some(
                "CASE WHEN locations.file_local_time IS NULL THEN NULL \
                 ELSE substr(locations.file_local_time, 1, 10) END",
            ),
            month: Some(
                "CASE WHEN locations.file_local_time IS NULL THEN NULL \
                 ELSE substr(locations.file_local_time, 1, 7) END",
            ),
        },
        GallerySortKey::ModifiedTime => GalleryOrderExpressions {
            missing: "CAST(0 AS INTEGER)",
            text: "CAST('' AS TEXT)",
            number: "locations.modified_unix_ms",
            date: Some(
                "strftime('%Y-%m-%d', locations.modified_unix_ms / 1000, 'unixepoch', 'localtime')",
            ),
            month: Some(
                "strftime('%Y-%m', locations.modified_unix_ms / 1000, 'unixepoch', 'localtime')",
            ),
        },
        GallerySortKey::FileName => GalleryOrderExpressions {
            missing: "CAST(0 AS INTEGER)",
            text: "locations.natural_name_key",
            number: "CAST(0 AS INTEGER)",
            date: None,
            month: None,
        },
    }
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

fn push_gallery_filters(
    query: &GalleryQuery,
    clauses: &mut Vec<String>,
    parameters: &mut Vec<Value>,
) {
    if let Some(root_id) = &query.root_id {
        clauses.push("locations.root_id = ?".to_owned());
        parameters.push(Value::Text(root_id.clone()));
    }
    if let Some(folder) = &query.folder_relative_path {
        let folder = normalize_relative_folder(folder);
        if query.include_descendants {
            clauses.push(
                "(locations.parent_relative_path = ? OR \
                 substr(locations.parent_relative_path, 1, length(?) + 1) = ? || '/')"
                    .to_owned(),
            );
            parameters.extend([
                Value::Text(folder.clone()),
                Value::Text(folder.clone()),
                Value::Text(folder),
            ]);
        } else {
            clauses.push("locations.parent_relative_path = ?".to_owned());
            parameters.push(Value::Text(folder));
        }
    }
    let search = query.search_text.trim();
    if !search.is_empty() {
        clauses.push(
            "(instr(lower(locations.absolute_path), lower(?)) > 0 OR \
             instr(lower(locations.relative_path), lower(?)) > 0)"
                .to_owned(),
        );
        parameters.extend([
            Value::Text(search.to_owned()),
            Value::Text(search.to_owned()),
        ]);
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
            "Name-sorted gallery results do not have a chronological time anchor",
        ));
    };
    match &anchor.month_key {
        Some(month_key) => {
            validate_month_key_text(month_key)?;
            let comparison = gallery_cursor_comparison(&query.sort_direction, false);
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

fn validate_month_key_text(month_key: &str) -> Result<(), ScanError> {
    let bytes = month_key.as_bytes();
    let valid_shape = bytes.len() == 7
        && bytes[4] == b'-'
        && bytes[..4].iter().all(u8::is_ascii_digit)
        && bytes[5..].iter().all(u8::is_ascii_digit);
    let month = valid_shape
        .then(|| month_key[5..].parse::<u8>().ok())
        .flatten();
    if matches!(month, Some(1..=12)) {
        return Ok(());
    }
    Err(ScanError::new(
        "catalog_time_anchor_invalid",
        "A gallery month anchor must use YYYY-MM with a month from 01 through 12",
    ))
}
