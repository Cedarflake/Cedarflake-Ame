use rusqlite::types::Value;
use rusqlite::{OptionalExtension, Transaction, params_from_iter};

use crate::domain::{CatalogCursor, GalleryQuery, GallerySortKey, ScanError};

use super::super::sqlite_integer;
use super::{
    database_error, gallery_cursor_comparison, gallery_direction_sql, gallery_order_expressions,
    gallery_source, gallery_window_direction_sql, push_cursor_filter, push_gallery_filters,
};

pub(super) struct GalleryAnchorPosition {
    cursor: CatalogCursor,
    known_ordinal: Option<u64>,
}

impl GalleryAnchorPosition {
    pub(super) fn for_cursor(cursor: CatalogCursor) -> Self {
        Self {
            cursor,
            known_ordinal: None,
        }
    }
}

pub(super) struct GalleryAnchorWindow {
    pub(super) location_id: String,
    pub(super) ordinal: u64,
    pub(super) start_ordinal: u64,
    pub(super) predecessor: Option<CatalogCursor>,
}

pub(super) fn resolve_window(
    transaction: &Transaction<'_>,
    query: &GalleryQuery,
    position: GalleryAnchorPosition,
    max_items: u32,
) -> Result<GalleryAnchorWindow, ScanError> {
    let anchor = position.cursor;
    let order = gallery_order_expressions(query);
    let mut clauses = Vec::new();
    let mut parameters = Vec::new();
    let source = gallery_source(query, &mut parameters);
    push_gallery_filters(query, &mut clauses, &mut parameters);
    push_cursor_filter(
        &anchor,
        &order,
        query.order_direction(),
        true,
        &mut clauses,
        &mut parameters,
    );
    if query.organization_queue.is_none()
        && !anchor.primary_missing
        && matches!(
            query.sort_key,
            GallerySortKey::CaptureTime | GallerySortKey::CreatedTime
        )
    {
        // The exact cursor predicate keeps ties; these equivalent bounds let the
        // ordered index exclude dates outside the preceding range.
        let comparison = gallery_cursor_comparison(query.order_direction(), true);
        clauses.push(format!(
            "{} = 0 AND {} {comparison}= ?",
            order.missing, order.text
        ));
        parameters.push(Value::Text(anchor.primary_text.clone()));
    }
    let preceding_where = clauses.join(" AND ");
    let ordinal = match position.known_ordinal {
        Some(ordinal) => ordinal,
        None => {
            let count: i64 = transaction
                .query_row(
                    &format!("SELECT COUNT(*) {source} WHERE {preceding_where}"),
                    params_from_iter(&parameters),
                    |row| row.get(0),
                )
                .map_err(database_error)?;
            u64::try_from(count).map_err(|_| {
                ScanError::new(
                    "catalog_location_anchor_invalid",
                    "The gallery location anchor ordinal is outside the supported range",
                )
            })?
        }
    };
    let start_ordinal = ordinal.saturating_sub(u64::from(max_items / 2));
    let predecessor = if start_ordinal == 0 {
        None
    } else {
        let reverse_offset = ordinal - start_ordinal;
        let direction = gallery_window_direction_sql(query.order_direction(), true);
        parameters.push(Value::Integer(sqlite_integer(
            reverse_offset,
            "gallery location anchor offset",
        )?));
        let sql = format!(
            "SELECT locations.root_id, locations.location_id,
                    {missing}, {text}, {number}
             {source} WHERE {preceding_where}
             ORDER BY {missing} DESC, {text} {direction}, {number} {direction},
                      locations.root_id DESC, locations.location_id DESC
             LIMIT 1 OFFSET ?",
            missing = order.missing,
            text = order.text,
            number = order.number,
        );
        Some(
            transaction
                .query_row(&sql, params_from_iter(parameters), |row| {
                    read_cursor(row, anchor.revision, &anchor.query_id)
                })
                .map_err(database_error)?,
        )
    };
    Ok(GalleryAnchorWindow {
        location_id: anchor.location_id,
        ordinal,
        start_ordinal,
        predecessor,
    })
}

pub(super) fn resolve_fallback(
    transaction: &Transaction<'_>,
    revision: u64,
    query: &GalleryQuery,
    query_id: &str,
    requested_ordinal: u64,
) -> Result<Option<GalleryAnchorPosition>, ScanError> {
    let order = gallery_order_expressions(query);
    let mut clauses = Vec::new();
    let mut parameters = Vec::new();
    let source = gallery_source(query, &mut parameters);
    push_gallery_filters(query, &mut clauses, &mut parameters);
    let scope = if clauses.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", clauses.join(" AND "))
    };
    let direction = gallery_direction_sql(query.order_direction());
    let sql = format!(
        "SELECT locations.root_id, locations.location_id,
                {missing}, {text}, {number}
         {source} {scope}
         ORDER BY {missing}, {text} {direction}, {number} {direction},
                  locations.root_id, locations.location_id
         LIMIT 1 OFFSET ?",
        missing = order.missing,
        text = order.text,
        number = order.number,
    );
    let mut statement = transaction.prepare(&sql).map_err(database_error)?;
    if let Ok(offset) = i64::try_from(requested_ordinal) {
        parameters.push(Value::Integer(offset));
        let cursor = statement
            .query_row(params_from_iter(&parameters), |row| {
                read_cursor(row, revision, query_id)
            })
            .optional()
            .map_err(database_error)?;
        parameters.pop();
        if let Some(cursor) = cursor {
            // This read proves the position in the same transaction as the following page.
            return Ok(Some(GalleryAnchorPosition {
                cursor,
                known_ordinal: Some(requested_ordinal),
            }));
        }
        if requested_ordinal == 0 {
            return Ok(None);
        }
    }
    let total: i64 = transaction
        .query_row(
            &format!("SELECT COUNT(*) {source} {scope}"),
            params_from_iter(&parameters),
            |row| row.get(0),
        )
        .map_err(database_error)?;
    let total = u64::try_from(total).map_err(|_| {
        ScanError::new(
            "catalog_asset_anchor_invalid",
            "The gallery asset fallback count is outside the supported range",
        )
    })?;
    if total == 0 {
        return Ok(None);
    }
    let ordinal = requested_ordinal.min(total - 1);
    parameters.push(Value::Integer(sqlite_integer(
        ordinal,
        "gallery asset fallback ordinal",
    )?));
    statement
        .query_row(params_from_iter(parameters), |row| {
            read_cursor(row, revision, query_id)
        })
        .optional()
        .map(|cursor| {
            cursor.map(|cursor| GalleryAnchorPosition {
                cursor,
                known_ordinal: Some(ordinal),
            })
        })
        .map_err(database_error)
}

fn read_cursor(
    row: &rusqlite::Row<'_>,
    revision: u64,
    query_id: &str,
) -> rusqlite::Result<CatalogCursor> {
    Ok(CatalogCursor {
        revision,
        query_id: query_id.to_owned(),
        root_id: row.get(0)?,
        location_id: row.get(1)?,
        primary_missing: row.get::<_, i64>(2)? != 0,
        primary_text: row.get(3)?,
        primary_number: row.get(4)?,
    })
}
