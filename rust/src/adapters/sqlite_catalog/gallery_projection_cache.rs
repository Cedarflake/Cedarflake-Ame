use std::collections::VecDeque;
use std::mem::size_of;
use std::sync::{Arc, Mutex};

use crate::domain::gallery_groups::GalleryGroupFilter;
use crate::domain::{GalleryCollection, GalleryDuplicateMode, GalleryQuery, ScanError};

use super::gallery_read_projection::GalleryReadProjection;

const RETAINED_BYTES: usize = 64 * 1024 * 1024;
const RETAINED_QUERIES: usize = 8;

#[derive(Default)]
pub(super) struct GalleryProjectionCache {
    state: Mutex<RetainedProjections>,
}

#[derive(Default)]
struct RetainedProjections {
    revision: u64,
    bytes: usize,
    entries: VecDeque<Entry>,
}

struct Entry {
    query: GalleryQuery,
    projection: Arc<GalleryReadProjection>,
    bytes: usize,
}

impl GalleryProjectionCache {
    pub(super) fn semantic_query(
        &self,
        query: &GalleryQuery,
        revision: u64,
    ) -> Option<GalleryQuery> {
        query.semantic_results.as_ref()?;
        if !query.folds_duplicates()
            || query.collection != GalleryCollection::Library
            || query.organization_queue.is_some()
        {
            return None;
        }
        let mut scoped = query.clone();
        let manifest = scoped.semantic_results.take()?;
        let membership = self.membership(&scoped, revision)?;
        let locations: std::collections::HashSet<_> = membership.location_ids().collect();
        if manifest
            .items
            .iter()
            .any(|item| !locations.contains(item.location_id.as_str()))
        {
            return None;
        }
        scoped.semantic_results = Some(manifest);
        // Membership already selected this scope's representatives in the same catalog revision.
        // Source generation and every remaining filter are still checked against the read snapshot.
        scoped.duplicate_mode = GalleryDuplicateMode::All;
        Some(scoped)
    }

    pub(super) fn membership(
        &self,
        query: &GalleryQuery,
        revision: u64,
    ) -> Option<Arc<GalleryReadProjection>> {
        let state = self.state.lock().unwrap_or_else(|error| error.into_inner());
        if state.revision != revision {
            return None;
        }
        state
            .entries
            .iter()
            .find(|entry| entry.query == *query)
            .map(|entry| Arc::clone(&entry.projection))
    }

    pub(super) fn load(
        &self,
        query: &GalleryQuery,
        query_id: &str,
        revision: u64,
        read: impl FnOnce() -> Result<Option<GalleryReadProjection>, ScanError>,
        refresh: impl FnOnce(&GalleryReadProjection) -> Result<GalleryReadProjection, ScanError>,
    ) -> Result<Option<Arc<GalleryReadProjection>>, ScanError> {
        // Similarity has its own publication clock. Its group projection and semantic manifests
        // retain their existing read path instead of borrowing catalog-revision authority.
        if query.group_filter != GalleryGroupFilter::Unrestricted
            || query.semantic_results.is_some()
            || query.metadata_filter != Default::default()
            || !query.folds_duplicates()
        {
            return read().map(|projection| projection.map(Arc::new));
        }
        {
            let mut state = self.state.lock().unwrap_or_else(|error| error.into_inner());
            state.advance(revision);
            if revision == state.revision
                && let Some(index) = state.entries.iter().position(|entry| {
                    entry.query == *query && entry.projection.timeline.query_id == query_id
                })
            {
                let entry = state.entries.remove(index).expect("located projection");
                let projection = Arc::clone(&entry.projection);
                state.entries.push_front(entry);
                drop(state);
                return refresh(&projection).map(|projection| Some(Arc::new(projection)));
            }
        }
        // SQL and encoding never hold the cache mutex; concurrent readers keep their own snapshot.
        let Some(projection) = read()?.map(Arc::new) else {
            return Ok(None);
        };
        let query = query.clone();
        let bytes = projection.retained_bytes() + query_bytes(&query) + size_of::<Entry>();
        if bytes <= RETAINED_BYTES {
            let mut state = self.state.lock().unwrap_or_else(|error| error.into_inner());
            state.advance(revision);
            if revision == state.revision
                && !state.entries.iter().any(|entry| {
                    entry.query == query && entry.projection.timeline.query_id == query_id
                })
            {
                while state.entries.len() >= RETAINED_QUERIES
                    || state.bytes + bytes > RETAINED_BYTES
                {
                    if let Some(retired) = state.entries.pop_back() {
                        state.bytes -= retired.bytes;
                    }
                }
                state.bytes += bytes;
                state.entries.push_front(Entry {
                    query,
                    projection: Arc::clone(&projection),
                    bytes,
                });
            }
        }
        Ok(Some(projection))
    }
}

impl RetainedProjections {
    fn advance(&mut self, revision: u64) {
        if revision > self.revision {
            self.entries.clear();
            self.bytes = 0;
            self.revision = revision;
        }
    }
}

pub(super) fn query_bytes(query: &GalleryQuery) -> usize {
    let collection = match &query.collection {
        GalleryCollection::Library | GalleryCollection::Favorites => 0,
        GalleryCollection::Album { album_id } => album_id.capacity(),
        GalleryCollection::SmartAlbum { category_id, .. } => {
            category_id.as_ref().map_or(0, String::capacity)
        }
    };
    let filter = &query.metadata_filter;
    collection
        + query.root_id.as_ref().map_or(0, String::capacity)
        + query
            .folder_relative_path
            .as_ref()
            .map_or(0, String::capacity)
        + query.search_text.capacity()
        + filter.camera_text.capacity()
        + filter.extensions.capacity() * size_of::<String>()
        + filter
            .extensions
            .iter()
            .map(String::capacity)
            .sum::<usize>()
        + filter.date.start.as_ref().map_or(0, String::capacity)
        + filter.date.end.as_ref().map_or(0, String::capacity)
}
