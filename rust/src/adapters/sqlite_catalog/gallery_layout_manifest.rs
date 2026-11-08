use std::collections::HashMap;

use rusqlite::{Transaction, params_from_iter};

use crate::domain::{
    CatalogCursor, GalleryLayoutDateGroup, GalleryLayoutManifestChunk, GalleryLayoutManifestCursor,
    GalleryQuery, ScanError,
};

use super::gallery::{
    self, build_gallery_count_query, build_gallery_layout_manifest_query, validate_gallery_query,
};
use super::{
    LAYOUT_FLAG_DIMENSIONS_KNOWN, MAX_LAYOUT_MANIFEST_CHUNK_ITEMS, SqliteCatalog, database_error,
    load_catalog_revision, sqlite_u32, sqlite_unsigned,
};

pub(super) struct StoredLayoutManifestItem {
    pub(super) location_id: String,
    pub(super) root_id: String,
    pub(super) width: u32,
    pub(super) height: u32,
    pub(super) date_key: Option<String>,
    pub(super) primary_missing: bool,
    pub(super) primary_text: String,
    pub(super) primary_number: i64,
}

pub(super) const MAX_INLINE_LAYOUT_ITEMS: u64 = 64 * 1024 * 1024 / 192;

impl StoredLayoutManifestItem {
    pub(super) fn aspect_ratio_milli(&self) -> u16 {
        if self.width == 0 || self.height == 0 {
            1_000
        } else {
            ((u64::from(self.width) * 1_000) / u64::from(self.height)).clamp(200, 5_000) as u16
        }
    }
}

pub(super) fn load_chunk(
    catalog: &mut SqliteCatalog,
    max_items: u32,
    query: &GalleryQuery,
    query_id: &str,
    after: Option<&GalleryLayoutManifestCursor>,
) -> Result<GalleryLayoutManifestChunk, ScanError> {
    if max_items == 0 || max_items > MAX_LAYOUT_MANIFEST_CHUNK_ITEMS {
        return Err(ScanError::new(
            "catalog_layout_chunk_limit_invalid",
            format!(
                "The gallery layout chunk limit must be between 1 and \
                 {MAX_LAYOUT_MANIFEST_CHUNK_ITEMS} items"
            ),
        ));
    }
    validate_gallery_query(query)?;
    let transaction = catalog.connection.transaction().map_err(database_error)?;
    gallery::organization::validate_collection(&transaction, query)?;
    let revision = load_catalog_revision(&transaction)?;
    if after.is_some_and(|cursor| {
        cursor.revision != revision
            || cursor.query_id != query_id
            || cursor.after.revision != revision
            || cursor.after.query_id != query_id
            || cursor.next_ordinal > cursor.total_items
    }) {
        return Err(ScanError::new(
            "catalog_layout_cursor_stale",
            "The catalog or gallery query changed after this layout cursor was created",
        ));
    }

    let total_items = if let Some(cursor) = after {
        cursor.total_items
    } else {
        let count_query = build_gallery_count_query(query);
        let count = transaction
            .query_row(
                &count_query.sql,
                params_from_iter(count_query.parameters.iter()),
                |row| row.get::<_, i64>(0),
            )
            .map_err(database_error)?;
        sqlite_unsigned(count, "gallery layout item count")?
    };
    let start_ordinal = after.map_or(0, |cursor| cursor.next_ordinal);
    let requested = usize::try_from(max_items).map_err(|_| {
        ScanError::new(
            "catalog_layout_chunk_limit_invalid",
            "The gallery layout chunk limit is outside the supported range",
        )
    })?;
    let built = build_gallery_layout_manifest_query(
        query,
        after.map(|cursor| &cursor.after),
        i64::from(max_items).saturating_add(1),
    )?;
    let mut statement = transaction.prepare(&built.sql).map_err(database_error)?;
    let mut rows = statement
        .query(params_from_iter(built.parameters.iter()))
        .map_err(database_error)?;
    let mut items = Vec::with_capacity(requested.saturating_add(1));
    while let Some(row) = rows.next().map_err(database_error)? {
        items.push(StoredLayoutManifestItem {
            location_id: row.get(0).map_err(database_error)?,
            root_id: row.get(1).map_err(database_error)?,
            width: sqlite_u32(row.get(2).map_err(database_error)?, "layout width")?,
            height: sqlite_u32(row.get(3).map_err(database_error)?, "layout height")?,
            date_key: row.get(4).map_err(database_error)?,
            primary_missing: row.get::<_, i64>(5).map_err(database_error)? != 0,
            primary_text: row.get(6).map_err(database_error)?,
            primary_number: row.get(7).map_err(database_error)?,
        });
    }
    drop(rows);
    drop(statement);

    let has_more = items.len() > requested;
    items.truncate(requested);
    let loaded_items = u64::try_from(items.len()).map_err(|_| {
        ScanError::new(
            "catalog_layout_count_invalid",
            "The gallery layout chunk exceeds the supported range",
        )
    })?;
    let next_ordinal = start_ordinal.checked_add(loaded_items).ok_or_else(|| {
        ScanError::new(
            "catalog_layout_count_invalid",
            "The gallery layout ordinal exceeds the supported range",
        )
    })?;
    if next_ordinal > total_items || (has_more && next_ordinal >= total_items) {
        return Err(ScanError::new(
            "catalog_layout_count_invalid",
            "The gallery layout cursor does not match the complete query count",
        ));
    }

    let next_cursor = if has_more {
        continuation(&items, revision, query_id, total_items, next_ordinal)
    } else {
        None
    };

    let chunk = encode_chunk(
        items,
        revision,
        query_id,
        total_items,
        start_ordinal,
        next_cursor,
    )?;
    transaction.commit().map_err(database_error)?;
    Ok(chunk)
}

/// Inline compact layouts have a bounded transfer cost. Larger queries retain
/// the existing block protocol rather than retaining every source record.
pub(super) fn load_snapshot(
    transaction: &Transaction<'_>,
    query: &GalleryQuery,
    query_id: &str,
    revision: u64,
    total_items: u64,
) -> Result<Option<Vec<GalleryLayoutManifestChunk>>, ScanError> {
    if total_items > MAX_INLINE_LAYOUT_ITEMS {
        return Ok(None);
    }
    let limit = i64::try_from(total_items.saturating_add(1)).map_err(|_| {
        ScanError::new(
            "catalog_layout_count_invalid",
            "The layout item count is outside the supported range",
        )
    })?;
    let built = build_gallery_layout_manifest_query(query, None, limit)?;
    let mut statement = transaction.prepare(&built.sql).map_err(database_error)?;
    let mut rows = statement
        .query(params_from_iter(built.parameters.iter()))
        .map_err(database_error)?;
    let mut chunks = Vec::new();
    let mut items = Vec::with_capacity(MAX_LAYOUT_MANIFEST_CHUNK_ITEMS as usize);
    let mut start_ordinal = 0;
    while let Some(row) = rows.next().map_err(database_error)? {
        items.push(StoredLayoutManifestItem {
            location_id: row.get(0).map_err(database_error)?,
            root_id: row.get(1).map_err(database_error)?,
            width: sqlite_u32(row.get(2).map_err(database_error)?, "layout width")?,
            height: sqlite_u32(row.get(3).map_err(database_error)?, "layout height")?,
            date_key: row.get(4).map_err(database_error)?,
            primary_missing: row.get::<_, i64>(5).map_err(database_error)? != 0,
            primary_text: row.get(6).map_err(database_error)?,
            primary_number: row.get(7).map_err(database_error)?,
        });
        if items.len() == MAX_LAYOUT_MANIFEST_CHUNK_ITEMS as usize {
            let count = items.len() as u64;
            let next_ordinal = start_ordinal + count;
            let next_cursor = if next_ordinal < total_items {
                continuation(&items, revision, query_id, total_items, next_ordinal)
            } else {
                None
            };
            chunks.push(encode_chunk(
                std::mem::take(&mut items),
                revision,
                query_id,
                total_items,
                start_ordinal,
                next_cursor,
            )?);
            start_ordinal += count;
        }
    }
    let count = items.len() as u64;
    if count > 0 || chunks.is_empty() {
        chunks.push(encode_chunk(
            items,
            revision,
            query_id,
            total_items,
            start_ordinal,
            None,
        )?);
    }
    if start_ordinal + count != total_items {
        return Err(ScanError::new(
            "catalog_layout_count_invalid",
            "The layout and timeline disagree inside the same read snapshot",
        ));
    }
    Ok(Some(chunks))
}

pub(super) fn continuation(
    items: &[StoredLayoutManifestItem],
    revision: u64,
    query_id: &str,
    total_items: u64,
    next_ordinal: u64,
) -> Option<GalleryLayoutManifestCursor> {
    items.last().map(|item| GalleryLayoutManifestCursor {
        revision,
        query_id: query_id.to_owned(),
        total_items,
        next_ordinal,
        after: CatalogCursor {
            revision,
            query_id: query_id.to_owned(),
            primary_missing: item.primary_missing,
            primary_text: item.primary_text.clone(),
            primary_number: item.primary_number,
            root_id: item.root_id.clone(),
            location_id: item.location_id.clone(),
        },
    })
}

pub(super) fn encode_chunk(
    items: Vec<StoredLayoutManifestItem>,
    revision: u64,
    query_id: &str,
    total_items: u64,
    start_ordinal: u64,
    next_cursor: Option<GalleryLayoutManifestCursor>,
) -> Result<GalleryLayoutManifestChunk, ScanError> {
    let mut location_ids = Vec::with_capacity(items.len());
    let mut aspect_ratio_milli = Vec::with_capacity(items.len());
    let mut date_group_indices = Vec::with_capacity(items.len());
    let mut date_groups = Vec::new();
    let mut date_group_lookup = HashMap::new();
    let mut flags = Vec::with_capacity(items.len());
    for item in items {
        let has_dimensions = item.width > 0 && item.height > 0;
        let ratio = item.aspect_ratio_milli();
        let date_group_index = if let Some(index) = date_group_lookup.get(&item.date_key) {
            *index
        } else {
            let index = u16::try_from(date_groups.len()).map_err(|_| {
                ScanError::new(
                    "catalog_layout_date_groups_invalid",
                    "The gallery layout chunk contains too many date groups",
                )
            })?;
            date_groups.push(GalleryLayoutDateGroup {
                date_key: item.date_key.clone(),
            });
            date_group_lookup.insert(item.date_key, index);
            index
        };
        location_ids.push(item.location_id);
        aspect_ratio_milli.push(ratio);
        date_group_indices.push(date_group_index);
        flags.push(if has_dimensions {
            LAYOUT_FLAG_DIMENSIONS_KNOWN
        } else {
            0
        });
    }

    Ok(GalleryLayoutManifestChunk {
        revision,
        query_id: query_id.to_owned(),
        total_items,
        start_ordinal,
        location_ids,
        aspect_ratio_milli,
        date_group_indices,
        date_groups,
        flags,
        next_cursor,
    })
}
