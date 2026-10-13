use rusqlite::types::Value;
use rusqlite::{OptionalExtension, Transaction, params_from_iter};

use crate::domain::{
    CatalogCursor, GalleryLocationAnchorResolution, GalleryQuery, GallerySortKey, ScanError,
};

use super::super::sqlite_integer;
use super::{
    database_error, gallery_cursor_comparison, gallery_direction_sql, gallery_order_expressions,
    gallery_window_direction_sql, push_cursor_filter, push_gallery_filters,
};

struct GalleryAnchorRequest<'a> {
    requested_location_id: &'a str,
    anchor_column: &'static str,
    anchor_value: &'a str,
    required_asset_id: Option<&'a str>,
    fallback_ordinal: Option<u64>,
    max_items: u32,
}

pub(in super::super) struct GalleryAssetAnchor<'a> {
    pub requested_location_id: &'a str,
    pub asset_id: &'a str,
    pub fallback_ordinal: u64,
}

pub(in super::super) fn resolve_gallery_location_anchor(
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
            required_asset_id: None,
            fallback_ordinal: None,
            max_items,
        },
    )
}

pub(in super::super) fn resolve_gallery_asset_anchor(
    transaction: &Transaction<'_>,
    revision: u64,
    query: &GalleryQuery,
    query_id: &str,
    max_items: u32,
    anchor: GalleryAssetAnchor<'_>,
) -> Result<(GalleryLocationAnchorResolution, Option<CatalogCursor>), ScanError> {
    let preferred = resolve_gallery_anchor(
        transaction,
        revision,
        query,
        query_id,
        GalleryAnchorRequest {
            requested_location_id: anchor.requested_location_id,
            anchor_column: "locations.location_id",
            anchor_value: anchor.requested_location_id,
            required_asset_id: Some(anchor.asset_id),
            fallback_ordinal: None,
            max_items,
        },
    )?;
    if preferred.0.location_id.is_some() {
        return Ok(preferred);
    }
    resolve_gallery_anchor(
        transaction,
        revision,
        query,
        query_id,
        GalleryAnchorRequest {
            requested_location_id: anchor.requested_location_id,
            anchor_column: "locations.asset_id",
            anchor_value: anchor.asset_id,
            required_asset_id: None,
            fallback_ordinal: Some(anchor.fallback_ordinal),
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
    if let Some(asset_id) = request.required_asset_id {
        anchor_clauses.push("locations.asset_id = ?".to_owned());
        anchor_parameters.push(Value::Text(asset_id.to_owned()));
    }
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
    let mut anchor = transaction
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
    if anchor.is_none()
        && let Some(fallback_ordinal) = request.fallback_ordinal
    {
        anchor =
            gallery_cursor_at_ordinal(transaction, revision, query, query_id, fallback_ordinal)?;
    }
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
    if !anchor.primary_missing
        && matches!(
            query.sort_key,
            GallerySortKey::CaptureTime | GallerySortKey::CreatedTime
        )
    {
        // The exact cursor predicate keeps ties; these equivalent bounds let the
        // ordered index exclude dates outside the preceding range.
        let comparison = gallery_cursor_comparison(&query.sort_direction, true);
        preceding_clauses.push(format!(
            "{} = 0 AND {} {comparison}= ?",
            order.missing, order.text
        ));
        preceding_parameters.push(Value::Text(anchor.primary_text.clone()));
    }
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

fn gallery_cursor_at_ordinal(
    transaction: &Transaction<'_>,
    revision: u64,
    query: &GalleryQuery,
    query_id: &str,
    requested_ordinal: u64,
) -> Result<Option<CatalogCursor>, ScanError> {
    let order = gallery_order_expressions(&query.sort_key);
    let mut clauses = Vec::new();
    let mut parameters = Vec::new();
    push_gallery_filters(query, &mut clauses, &mut parameters);
    let where_clause = if clauses.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", clauses.join(" AND "))
    };
    let count_sql = format!(
        "SELECT COUNT(*)
         FROM library_roots AS roots
         JOIN asset_locations AS locations
           ON locations.scan_id = roots.active_scan_id
         {where_clause}"
    );
    let total_i64: i64 = transaction
        .query_row(&count_sql, params_from_iter(parameters.iter()), |row| {
            row.get(0)
        })
        .map_err(database_error)?;
    let total = u64::try_from(total_i64).map_err(|_| {
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
    let direction = gallery_direction_sql(&query.sort_direction);
    let sql = format!(
        "SELECT locations.root_id, locations.location_id,
                {missing}, {text}, {number}
         FROM library_roots AS roots
         JOIN asset_locations AS locations
           ON locations.scan_id = roots.active_scan_id
         {where_clause}
         ORDER BY {missing}, {text} {direction}, {number} {direction},
                  locations.root_id, locations.location_id
         LIMIT 1 OFFSET ?",
        missing = order.missing,
        text = order.text,
        number = order.number,
    );
    transaction
        .query_row(&sql, params_from_iter(parameters.iter()), |row| {
            Ok(CatalogCursor {
                revision,
                query_id: query_id.to_owned(),
                root_id: row.get(0)?,
                location_id: row.get(1)?,
                primary_missing: row.get::<_, i64>(2)? != 0,
                primary_text: row.get(3)?,
                primary_number: row.get(4)?,
            })
        })
        .optional()
        .map_err(database_error)
}
