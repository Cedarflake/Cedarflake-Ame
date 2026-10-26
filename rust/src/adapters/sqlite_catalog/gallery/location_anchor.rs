use rusqlite::types::Value;
use rusqlite::{OptionalExtension, Transaction, params_from_iter};

use crate::domain::{CatalogCursor, GalleryLocationAnchorResolution, GalleryQuery, ScanError};

use super::anchor_position::{self, GalleryAnchorPosition};
use super::filters::push_anchor_filters;
use super::{database_error, gallery_order_expressions, gallery_source};

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
    let order = gallery_order_expressions(query);
    let representative = if request.anchor_column == "locations.location_id" {
        super::duplicates::representative_for_anchor(
            transaction,
            query,
            request.anchor_value,
            request.required_asset_id,
        )?
    } else {
        None
    };
    let anchor_value = representative.as_deref().unwrap_or(request.anchor_value);
    let mut anchor_clauses = Vec::new();
    let mut anchor_parameters = Vec::new();
    let source = gallery_source(query, &mut anchor_parameters);
    push_anchor_filters(query, &mut anchor_clauses, &mut anchor_parameters);
    anchor_clauses.push(format!("{} = ?", request.anchor_column));
    anchor_parameters.push(Value::Text(anchor_value.to_owned()));
    if let Some(asset_id) = request
        .required_asset_id
        .filter(|_| representative.is_none())
    {
        anchor_clauses.push("locations.asset_id = ?".to_owned());
        anchor_parameters.push(Value::Text(asset_id.to_owned()));
    }
    let anchor_sql = format!(
        "SELECT locations.root_id, locations.location_id,
                {missing}, {text}, {number}
         {source} WHERE {where_clause}
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
    let position = match anchor {
        Some(cursor) => Some(GalleryAnchorPosition::for_cursor(cursor)),
        None => match request.fallback_ordinal {
            Some(ordinal) => {
                anchor_position::resolve_fallback(transaction, revision, query, query_id, ordinal)?
            }
            None => None,
        },
    };
    let Some(position) = position else {
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
    let window = anchor_position::resolve_window(transaction, query, position, request.max_items)?;
    Ok((
        GalleryLocationAnchorResolution {
            requested_location_id: request.requested_location_id.to_owned(),
            location_id: Some(window.location_id),
            ordinal: Some(window.ordinal),
            window_start_ordinal: window.start_ordinal,
        },
        window.predecessor,
    ))
}
