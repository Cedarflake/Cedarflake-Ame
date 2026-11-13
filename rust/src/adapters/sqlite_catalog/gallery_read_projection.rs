use std::collections::BTreeMap;

use rusqlite::{Transaction, params_from_iter};

use crate::domain::{
    CatalogCursor, CatalogSnapshot, GalleryLayoutManifestChunk, GalleryLocationAnchorResolution,
    GalleryQuery, GalleryQueryAnchor, GallerySortDirection, GallerySortKey, GalleryTimeBucket,
    GalleryTimeline, ScanError,
};

use super::gallery::{
    ASSET_COLUMNS, build_gallery_layout_manifest_query, load_source_count, project_name_sections,
    uses_grouped_organization,
};
use super::gallery_layout_manifest::{
    MAX_INLINE_LAYOUT_ITEMS, StoredLayoutManifestItem, continuation, encode_chunk,
};
use super::{
    MAX_CATALOG_PAGE_ITEMS, MAX_LAYOUT_MANIFEST_CHUNK_ITEMS, database_error, load_root_views,
    read_stored_asset, sqlite_u32, sqlite_unsigned, stored_asset_view,
};

/// One bounded ordered selection supplies geometry, navigation and detail windows.
/// Reuse requires the same query and catalog revision in the caller's read transaction.
pub(super) struct GalleryReadProjection {
    items: Vec<StoredLayoutManifestItem>,
    pub(super) timeline: GalleryTimeline,
}

impl GalleryReadProjection {
    pub(super) fn location_ids(&self) -> impl Iterator<Item = &str> {
        self.items.iter().map(|item| item.location_id.as_str())
    }

    pub(super) fn load(
        transaction: &Transaction<'_>,
        query: &GalleryQuery,
        query_id: &str,
        revision: u64,
    ) -> Result<Option<Self>, ScanError> {
        if query.semantic_results.is_some() || query.sort_key == GallerySortKey::LibraryAdmission {
            return Ok(None);
        }
        let mut total_source_items = if query.folds_duplicates()
            && !uses_grouped_organization(query)
            && !super::gallery::groups::is_filtered(query)
        {
            Some(load_source_count(transaction, query)?)
        } else {
            None
        };
        if total_source_items.is_some_and(|count| count > MAX_INLINE_LAYOUT_ITEMS) {
            return Ok(None);
        }
        let built =
            build_gallery_layout_manifest_query(query, None, MAX_INLINE_LAYOUT_ITEMS as i64 + 1)?;
        let mut statement = transaction.prepare(&built.sql).map_err(database_error)?;
        let rows = statement
            .query_map(params_from_iter(built.parameters), |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, i64>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, i64>(7)?,
                    row.get::<_, Option<i64>>(8)?,
                    super::gallery::groups::read(row),
                ))
            })
            .map_err(database_error)?;
        let mut items = Vec::new();
        let mut months = BTreeMap::<Option<String>, GalleryTimeBucket>::new();
        for row in rows {
            let (
                location_id,
                root_id,
                width,
                height,
                date_key,
                missing,
                text,
                number,
                source_items,
                group,
            ) = row.map_err(database_error)?;
            if let Some(source_items) = source_items {
                total_source_items =
                    Some(sqlite_unsigned(source_items, "gallery source item count")?);
            }
            if items.len() as u64 == MAX_INLINE_LAYOUT_ITEMS {
                return Ok(None);
            }
            let item = StoredLayoutManifestItem {
                location_id,
                root_id,
                width: sqlite_u32(width, "layout width")?,
                height: sqlite_u32(height, "layout height")?,
                date_key,
                primary_missing: missing != 0,
                primary_text: text,
                primary_number: number,
                group: group?,
            };
            let month_key = item
                .date_key
                .as_ref()
                .map(|date| date.chars().take(7).collect::<String>());
            let bucket = months
                .entry(month_key.clone())
                .or_insert_with(|| GalleryTimeBucket {
                    month_key,
                    item_count: 0,
                    aspect_ratio_milli_sum: 0,
                });
            bucket.item_count += 1;
            bucket.aspect_ratio_milli_sum += u64::from(item.aspect_ratio_milli());
            items.push(item);
        }
        drop(statement);
        let unknown = months.remove(&None);
        let mut buckets: Vec<_> = months.into_values().collect();
        if *query.order_direction() == GallerySortDirection::Descending {
            buckets.reverse();
        }
        buckets.extend(unknown);
        let scroll_sections = if query.sort_key == GallerySortKey::FileName {
            buckets.clear();
            project_name_sections(
                items.iter().map(|item| item.primary_text.as_str()),
                query.order_direction(),
            )
        } else {
            Vec::new()
        };
        let total_items = items.len() as u64;
        Ok(Some(Self {
            items,
            timeline: GalleryTimeline {
                revision,
                query_id: query_id.to_owned(),
                total_items,
                total_source_items: total_source_items.unwrap_or(total_items),
                buckets,
                scroll_sections,
            },
        }))
    }

    pub(super) fn load_page(
        &self,
        transaction: &Transaction<'_>,
        catalog_path: &str,
        start: u64,
        max_items: u32,
    ) -> Result<CatalogSnapshot, ScanError> {
        if max_items == 0 || max_items > MAX_CATALOG_PAGE_ITEMS {
            return Err(ScanError::new(
                "catalog_page_limit_invalid",
                "The catalog page limit is outside the supported range",
            ));
        }
        let start = usize::try_from(start)
            .ok()
            .filter(|start| *start <= self.items.len())
            .ok_or_else(|| {
                ScanError::new(
                    "catalog_layout_count_invalid",
                    "The detail window is outside the selected gallery",
                )
            })?;
        let end = start
            .saturating_add(max_items as usize)
            .min(self.items.len());
        let selected = &self.items[start..end];
        let ids: Vec<_> = selected
            .iter()
            .map(|item| (item.root_id.as_str(), item.location_id.as_str()))
            .collect();
        let encoded = serde_json::to_string(&ids)
            .map_err(|error| ScanError::new("catalog_query_invalid", error.to_string()))?;
        let mut statement = transaction
            .prepare(&format!(
                "SELECT {ASSET_COLUMNS}
             FROM json_each(?1) AS selected
             CROSS JOIN asset_locations AS locations INDEXED BY asset_locations_location_id
               ON locations.root_id = json_extract(selected.value, '$[0]')
               AND locations.location_id = json_extract(selected.value, '$[1]')
             JOIN library_roots AS roots ON locations.scan_id = roots.active_scan_id
             ORDER BY CAST(selected.key AS INTEGER)"
            ))
            .map_err(database_error)?;
        let mut rows = statement.query([encoded]).map_err(database_error)?;
        let mut assets = Vec::with_capacity(selected.len());
        while let Some(row) = rows.next().map_err(database_error)? {
            let asset = stored_asset_view(read_stored_asset(row).map_err(database_error)?)?;
            let group = selected
                .get(assets.len())
                .and_then(|item| item.group.clone());
            assets.push(crate::domain::gallery_groups::GalleryEntry {
                source: asset,
                group,
            });
        }
        if assets.len() != selected.len()
            || assets.iter().zip(selected).any(|(asset, item)| {
                asset.root_id != item.root_id || asset.location_id != item.location_id
            })
        {
            return Err(ScanError::new(
                "catalog_query_snapshot_invalid",
                "The detail window does not match its selected gallery locations",
            ));
        }
        Ok(CatalogSnapshot {
            catalog_path: catalog_path.to_owned(),
            revision: self.timeline.revision,
            query_id: self.timeline.query_id.clone(),
            roots: load_root_views(transaction)?,
            assets,
            previous_cursor: (start > 0)
                .then(|| selected.first().map(|item| self.cursor(item)))
                .flatten(),
            next_cursor: (end < self.items.len())
                .then(|| selected.last().map(|item| self.cursor(item)))
                .flatten(),
            query_anchor_resolution: None,
        })
    }

    pub(super) fn resolve_anchor(
        &self,
        transaction: &Transaction<'_>,
        query: &GalleryQuery,
        anchor: &GalleryQueryAnchor,
        max_items: u32,
    ) -> Result<GalleryLocationAnchorResolution, ScanError> {
        let target = super::gallery::find_gallery_query_anchor(
            transaction,
            self.timeline.revision,
            query,
            &self.timeline.query_id,
            anchor,
        )?;
        let ordinal = match target {
            Some(cursor) => Some(self.items.iter().position(|item| {
                item.location_id == cursor.location_id && item.root_id == cursor.root_id
                    && item.group.as_ref().map(|group| &group.key) == cursor.group_key.as_ref()
            }).ok_or_else(|| ScanError::new(
                "catalog_query_snapshot_invalid",
                "The resolved gallery anchor is outside its transaction's ordered projection",
            ))? as u64),
            None if anchor.asset_id.is_some() && !self.items.is_empty() => {
                Some(anchor.fallback_ordinal.min(self.items.len() as u64 - 1))
            }
            None => None,
        };
        Ok(GalleryLocationAnchorResolution {
            requested_location_id: anchor.requested_location_id.clone(),
            location_id: ordinal.map(|index| self.items[index as usize].location_id.clone()),
            ordinal,
            window_start_ordinal: ordinal
                .map_or(0, |index| index.saturating_sub(u64::from(max_items / 2))),
        })
    }

    fn cursor(&self, item: &StoredLayoutManifestItem) -> CatalogCursor {
        CatalogCursor {
            revision: self.timeline.revision,
            query_id: self.timeline.query_id.clone(),
            primary_missing: item.primary_missing,
            primary_text: item.primary_text.clone(),
            primary_number: item.primary_number,
            root_id: item.root_id.clone(),
            location_id: item.location_id.clone(),
            group_key: item.group.as_ref().map(|group| group.key.clone()),
        }
    }

    pub(super) fn refresh_geometry(
        &self,
        transaction: &Transaction<'_>,
    ) -> Result<Self, ScanError> {
        let mut items = self.items.clone();
        let mut sums = BTreeMap::<Option<String>, u64>::new();
        for chunk in items.chunks_mut(MAX_LAYOUT_MANIFEST_CHUNK_ITEMS as usize) {
            super::gallery_layout_manifest::refresh_dimensions(transaction, chunk)?;
            for item in chunk {
                let month = item
                    .date_key
                    .as_ref()
                    .map(|date| date.chars().take(7).collect::<String>());
                *sums.entry(month).or_default() += u64::from(item.aspect_ratio_milli());
            }
        }
        let mut timeline = self.timeline.clone();
        for bucket in &mut timeline.buckets {
            bucket.aspect_ratio_milli_sum = sums.remove(&bucket.month_key).unwrap_or(0);
        }
        Ok(Self { items, timeline })
    }

    pub(super) fn encode_layout(&self) -> Result<Vec<GalleryLayoutManifestChunk>, ScanError> {
        let mut chunks = Vec::new();
        let mut remaining = self.items.iter().cloned();
        let mut start = 0;
        loop {
            let items: Vec<_> = remaining
                .by_ref()
                .take(MAX_LAYOUT_MANIFEST_CHUNK_ITEMS as usize)
                .collect();
            let count = items.len() as u64;
            let next = if start + count < self.timeline.total_items {
                continuation(
                    &items,
                    self.timeline.revision,
                    &self.timeline.query_id,
                    self.timeline.total_items,
                    start + count,
                )
            } else {
                None
            };
            chunks.push(encode_chunk(
                items,
                self.timeline.revision,
                &self.timeline.query_id,
                self.timeline.total_items,
                start,
                next,
            )?);
            start += count;
            if start == self.timeline.total_items {
                break;
            }
        }
        Ok(chunks)
    }

    pub(super) fn retained_bytes(&self) -> usize {
        use std::mem::size_of;

        size_of::<Self>()
            + self.items.capacity() * size_of::<StoredLayoutManifestItem>()
            + self
                .items
                .iter()
                .map(|item| {
                    item.location_id.capacity()
                        + item.root_id.capacity()
                        + item.date_key.as_ref().map_or(0, String::capacity)
                        + item.primary_text.capacity()
                })
                .sum::<usize>()
            + self.timeline.query_id.capacity()
            + self.timeline.buckets.capacity() * size_of::<GalleryTimeBucket>()
            + self
                .timeline
                .buckets
                .iter()
                .map(|bucket| bucket.month_key.as_ref().map_or(0, String::capacity))
                .sum::<usize>()
            + self.timeline.scroll_sections.capacity()
                * size_of::<crate::domain::gallery_scroll_section::GalleryScrollSection>()
            + self
                .timeline
                .scroll_sections
                .iter()
                .map(|section| section.key.capacity())
                .sum::<usize>()
    }
}
