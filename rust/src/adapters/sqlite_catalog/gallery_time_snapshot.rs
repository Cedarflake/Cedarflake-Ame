use crate::domain::{
    GalleryQuery, GallerySortKey, GalleryTimeIntent, GalleryTimeSnapshot, ScanError,
};

use super::SqliteCatalog;
use super::gallery_snapshot::GalleryReadTransaction;

pub(super) fn load_time_snapshot(
    catalog: &mut SqliteCatalog,
    max_items: u32,
    query: &GalleryQuery,
    query_id: &str,
    intent: &GalleryTimeIntent,
) -> Result<GalleryTimeSnapshot, ScanError> {
    if matches!(query.sort_key, GallerySortKey::FileName) {
        return Err(ScanError::new(
            "catalog_time_anchor_unavailable",
            "Name-sorted gallery results do not have a chronological time anchor",
        ));
    }
    if matches!(query.sort_key, GallerySortKey::ModifiedTime) && intent.month_key.is_none() {
        return Err(ScanError::new(
            "catalog_time_anchor_invalid",
            "Modification-time results do not contain an unknown-date section",
        ));
    }
    let read =
        GalleryReadTransaction::begin(&mut catalog.connection, &catalog.path, query, query_id)?;
    let timeline = read.load_timeline()?;
    let resolved = intent.resolve(&timeline, &query.sort_direction)?;
    #[cfg(test)]
    AFTER_TIME_RESOLUTION.with(|slot| {
        if let Some(hook) = slot.borrow_mut().take() {
            hook();
        }
    });
    let anchor = resolved.as_ref().map(|value| value.anchor.clone());
    let window_start = resolved
        .as_ref()
        .map(|value| value.window_start(&timeline, max_items))
        .transpose()?;
    let window_start_ordinal = window_start.as_ref().map_or(0, |value| value.ordinal);
    let window_anchor = window_start.as_ref().map(|value| &value.anchor);
    let snapshot = read.load_snapshot(max_items, None, None, window_anchor)?;
    read.commit()?;
    Ok(GalleryTimeSnapshot {
        snapshot,
        timeline,
        anchor,
        window_start_ordinal,
    })
}

#[cfg(test)]
thread_local! {
    static AFTER_TIME_RESOLUTION: std::cell::RefCell<Option<Box<dyn FnOnce()>>> =
        const { std::cell::RefCell::new(None) };
}

#[cfg(test)]
pub(super) fn set_after_time_resolution_hook(hook: impl FnOnce() + 'static) {
    AFTER_TIME_RESOLUTION.with(|slot| *slot.borrow_mut() = Some(Box::new(hook)));
}
