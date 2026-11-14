use std::borrow::Cow;
use std::path::Path;
use std::sync::Arc;

use rusqlite::{Connection, Transaction, params_from_iter};

use crate::domain::{
    CatalogCursor, CatalogSnapshot, GalleryQuery, GalleryQueryAnchor, GalleryQuerySnapshot,
    GalleryTimeAnchor, GalleryTimeline, ScanError,
};
use crate::ports::GalleryQueryRepository;

use super::gallery::{
    GalleryAssetAnchor, build_gallery_asset_query, gallery_cursor_for_asset, load_gallery_timeline,
    resolve_gallery_anchor_cursor, resolve_gallery_asset_anchor, resolve_gallery_location_anchor,
    validate_gallery_query,
};
use super::gallery_read_projection::GalleryReadProjection;
use super::{
    MAX_CATALOG_PAGE_ITEMS, SqliteCatalog, database_error, load_catalog_revision, load_root_views,
    read_stored_asset, stored_asset_view,
};

pub(super) struct GalleryReadTransaction<'connection, 'query> {
    transaction: Transaction<'connection>,
    catalog_path: String,
    query: Cow<'query, GalleryQuery>,
    query_id: &'query str,
    revision: u64,
}

impl<'connection, 'query> GalleryReadTransaction<'connection, 'query> {
    pub(super) fn begin(
        connection: &'connection mut Connection,
        path: &Path,
        query: &'query GalleryQuery,
        query_id: &'query str,
        cache: &super::gallery_projection_cache::GalleryProjectionCache,
    ) -> Result<Self, ScanError> {
        validate_gallery_query(query)?;
        let transaction = connection.transaction().map_err(database_error)?;
        super::gallery::organization::validate_collection(&transaction, query)?;
        let revision = load_catalog_revision(&transaction)?;
        let query = cache
            .semantic_query(query, revision)
            .map_or_else(|| Cow::Borrowed(query), Cow::Owned);
        Ok(Self {
            transaction,
            catalog_path: path.to_string_lossy().into_owned(),
            query,
            query_id,
            revision,
        })
    }

    pub(super) fn commit(self) -> Result<(), ScanError> {
        self.transaction.commit().map_err(database_error)
    }

    pub(super) fn load_projection(
        &self,
        cache: &super::gallery_projection_cache::GalleryProjectionCache,
    ) -> Result<Option<Arc<GalleryReadProjection>>, ScanError> {
        cache.load(
            &self.query,
            self.query_id,
            self.revision,
            || {
                GalleryReadProjection::load(
                    &self.transaction,
                    &self.query,
                    self.query_id,
                    self.revision,
                )
            },
            |projection| projection.refresh_geometry(&self.transaction),
        )
    }

    pub(super) fn load_projection_page(
        &self,
        projection: &GalleryReadProjection,
        start: u64,
        max_items: u32,
    ) -> Result<CatalogSnapshot, ScanError> {
        projection.load_page(&self.transaction, &self.catalog_path, start, max_items)
    }

    pub(super) fn load_anchored_snapshot(
        &self,
        max_items: u32,
        anchor: &GalleryQueryAnchor,
        projection: Option<&GalleryReadProjection>,
    ) -> Result<CatalogSnapshot, ScanError> {
        if max_items == 0 || max_items > MAX_CATALOG_PAGE_ITEMS {
            return Err(ScanError::new(
                "catalog_page_limit_invalid",
                format!(
                    "The catalog page limit must be between 1 and {MAX_CATALOG_PAGE_ITEMS} items"
                ),
            ));
        }
        if anchor.requested_location_id.trim().is_empty()
            || anchor
                .asset_id
                .as_ref()
                .is_some_and(|value| value.trim().is_empty())
        {
            return Err(ScanError::new(
                "catalog_asset_anchor_invalid",
                "A gallery anchor requires non-empty location and optional asset identifiers",
            ));
        }
        let (resolution, predecessor) = match (projection, anchor.asset_id.as_deref()) {
            (Some(projection), _) => Ok((
                projection.resolve_anchor(&self.transaction, &self.query, anchor, max_items)?,
                None,
            )),
            (None, _)
                if anchor.group_key.is_some()
                    && super::gallery::groups::is_filtered(&self.query) =>
            {
                super::gallery::resolve_gallery_group_anchor(
                    &self.transaction,
                    self.revision,
                    &self.query,
                    self.query_id,
                    max_items,
                    anchor,
                )
            }
            (None, Some(asset_id)) => resolve_gallery_asset_anchor(
                &self.transaction,
                self.revision,
                &self.query,
                self.query_id,
                max_items,
                GalleryAssetAnchor {
                    requested_location_id: &anchor.requested_location_id,
                    asset_id,
                    fallback_ordinal: anchor.fallback_ordinal,
                },
            ),
            (None, None) => resolve_gallery_location_anchor(
                &self.transaction,
                self.revision,
                &self.query,
                self.query_id,
                &anchor.requested_location_id,
                max_items,
            ),
        }?;
        #[cfg(test)]
        AFTER_QUERY_ANCHOR_READ.with(|hook| {
            if let Some(hook) = hook.borrow_mut().take() {
                hook();
            }
        });
        let mut snapshot = match projection {
            Some(projection) => {
                self.load_projection_page(projection, resolution.window_start_ordinal, max_items)?
            }
            None => self.load_snapshot(max_items, predecessor.as_ref(), None, None)?,
        };
        snapshot.query_anchor_resolution = Some(resolution);
        Ok(snapshot)
    }
    pub(super) fn load_snapshot(
        &self,
        max_items: u32,
        after: Option<&CatalogCursor>,
        before: Option<&CatalogCursor>,
        anchor: Option<&GalleryTimeAnchor>,
    ) -> Result<CatalogSnapshot, ScanError> {
        if max_items == 0 || max_items > MAX_CATALOG_PAGE_ITEMS {
            return Err(ScanError::new(
                "catalog_page_limit_invalid",
                format!(
                    "The catalog page limit must be between 1 and {MAX_CATALOG_PAGE_ITEMS} items"
                ),
            ));
        }
        if usize::from(after.is_some())
            + usize::from(before.is_some())
            + usize::from(anchor.is_some())
            > 1
        {
            return Err(ScanError::new(
                "catalog_query_invalid",
                "A gallery request accepts only one page cursor or anchor",
            ));
        }

        let catalog_path = self.catalog_path.clone();
        let transaction = &self.transaction;
        let query = &self.query;
        let query_id = self.query_id;
        let revision = self.revision;
        if after.is_some_and(|cursor| cursor.revision != revision || cursor.query_id != query_id) {
            return Err(ScanError::new(
                "catalog_cursor_stale",
                "The catalog or gallery query changed after this page cursor was created",
            ));
        }
        if before.is_some_and(|cursor| cursor.revision != revision || cursor.query_id != query_id) {
            return Err(ScanError::new(
                "catalog_cursor_stale",
                "The catalog or gallery query changed after this page cursor was created",
            ));
        }
        if anchor.is_some_and(|value| value.revision != revision || value.query_id != query_id) {
            return Err(ScanError::new(
                "catalog_cursor_stale",
                "The catalog or gallery query changed after this time anchor was created",
            ));
        }

        let roots = load_root_views(transaction)?;

        let requested = usize::try_from(max_items).map_err(|_| {
            ScanError::new(
                "catalog_page_limit_invalid",
                "The catalog page limit is outside the supported range",
            )
        })?;
        let sql_limit = i64::from(max_items).saturating_add(1);
        let resolved_anchor_cursor = anchor
            .filter(|value| value.item_offset > 0)
            .map(|value| {
                resolve_gallery_anchor_cursor(transaction, revision, query, query_id, value)
            })
            .transpose()?;
        let effective_after = after.or(resolved_anchor_cursor.as_ref());
        let effective_anchor = if resolved_anchor_cursor.is_some() {
            None
        } else {
            anchor
        };
        let built =
            build_gallery_asset_query(query, effective_after, before, effective_anchor, sql_limit)?;
        let mut asset_statement = transaction.prepare(&built.sql).map_err(database_error)?;
        let mut asset_rows = asset_statement
            .query(params_from_iter(built.parameters.iter()))
            .map_err(database_error)?;
        let mut stored_assets = Vec::new();
        while let Some(row) = asset_rows.next().map_err(database_error)? {
            let stored = read_stored_asset(row).map_err(database_error)?;
            stored_assets.push(crate::domain::gallery_groups::GalleryEntry {
                source: stored_asset_view(stored)?,
                group: super::gallery::groups::read(row)?,
            });
        }
        drop(asset_rows);
        drop(asset_statement);

        let has_more = stored_assets.len() > requested;
        stored_assets.truncate(requested);
        if before.is_some() {
            stored_assets.reverse();
        }
        let previous_cursor = if before.is_some() && has_more {
            stored_assets
                .first()
                .map(|asset| {
                    gallery_cursor_for_asset(transaction, revision, query_id, query, asset)
                })
                .transpose()?
        } else if effective_after.is_some() || anchor.is_some() {
            stored_assets
                .first()
                .map(|asset| {
                    gallery_cursor_for_asset(transaction, revision, query_id, query, asset)
                })
                .transpose()?
        } else {
            None
        };
        let next_cursor = if before.is_some() {
            stored_assets
                .last()
                .map(|asset| {
                    gallery_cursor_for_asset(transaction, revision, query_id, query, asset)
                })
                .transpose()?
        } else if has_more {
            stored_assets
                .last()
                .map(|asset| {
                    gallery_cursor_for_asset(transaction, revision, query_id, query, asset)
                })
                .transpose()?
        } else {
            None
        };
        let assets = stored_assets;

        Ok(CatalogSnapshot {
            catalog_path,
            revision,
            query_id: query_id.to_owned(),
            roots,
            assets,
            previous_cursor,
            next_cursor,
            query_anchor_resolution: None,
        })
    }

    pub(super) fn load_timeline(&self) -> Result<GalleryTimeline, ScanError> {
        load_gallery_timeline(&self.transaction, &self.query, self.query_id, self.revision)
    }

    pub(super) fn load_layout_manifest(
        &self,
        total_items: u64,
    ) -> Result<Option<Vec<crate::domain::GalleryLayoutManifestChunk>>, ScanError> {
        super::gallery_layout_manifest::load_snapshot(
            &self.transaction,
            &self.query,
            self.query_id,
            self.revision,
            total_items,
        )
    }
}

impl GalleryQueryRepository for SqliteCatalog {
    fn load_time_snapshot(
        &mut self,
        max_items: u32,
        query: &GalleryQuery,
        query_id: &str,
        intent: &crate::domain::GalleryTimeIntent,
    ) -> Result<crate::domain::GalleryTimeSnapshot, ScanError> {
        super::gallery_time_snapshot::load_time_snapshot(self, max_items, query, query_id, intent)
    }

    fn load_query_snapshot(
        &mut self,
        max_items: u32,
        query: &GalleryQuery,
        query_id: &str,
        anchor: Option<&GalleryQueryAnchor>,
    ) -> Result<GalleryQuerySnapshot, ScanError> {
        let read = GalleryReadTransaction::begin(
            &mut self.connection,
            &self.path,
            query,
            query_id,
            &self.session.gallery_projections,
        )?;
        let projection = read.load_projection(&self.session.gallery_projections)?;
        let snapshot = match (anchor, projection.as_deref()) {
            (Some(anchor), projection) => {
                read.load_anchored_snapshot(max_items, anchor, projection)
            }
            (None, Some(projection)) => read.load_projection_page(projection, 0, max_items),
            (None, None) => read.load_snapshot(max_items, None, None, None),
        }?;
        #[cfg(test)]
        AFTER_QUERY_SNAPSHOT_READ.with(|hook| {
            if let Some(hook) = hook.borrow_mut().take() {
                hook();
            }
        });
        let timeline = match projection.as_ref() {
            Some(projection) => projection.timeline.clone(),
            None => read.load_timeline()?,
        };
        let layout_manifest = match projection.as_ref() {
            Some(projection) => Some(projection.encode_layout()?),
            None => read.load_layout_manifest(timeline.total_items)?,
        };
        let organization_counts = match projection.as_ref() {
            Some(projection) => projection.organization_counts(&read.transaction, query)?,
            None => query
                .organization_queue
                .map(|_| super::gallery::organization_queue::load_counts(&read.transaction, query))
                .transpose()?,
        };
        read.commit()?;
        Ok(GalleryQuerySnapshot {
            snapshot,
            timeline,
            organization_counts,
            layout_manifest,
        })
    }
}

#[cfg(test)]
thread_local! {
    static AFTER_QUERY_SNAPSHOT_READ: std::cell::RefCell<Option<Box<dyn FnOnce()>>> =
        const { std::cell::RefCell::new(None) };
    static AFTER_QUERY_ANCHOR_READ: std::cell::RefCell<Option<Box<dyn FnOnce()>>> =
        const { std::cell::RefCell::new(None) };
}

#[cfg(test)]
pub(super) fn set_after_query_snapshot_read_hook(hook: impl FnOnce() + 'static) {
    AFTER_QUERY_SNAPSHOT_READ.with(|slot| *slot.borrow_mut() = Some(Box::new(hook)));
}

#[cfg(test)]
pub(super) fn set_after_query_anchor_read_hook(hook: impl FnOnce() + 'static) {
    AFTER_QUERY_ANCHOR_READ.with(|slot| *slot.borrow_mut() = Some(Box::new(hook)));
}
