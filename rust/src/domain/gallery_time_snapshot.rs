use super::{CatalogSnapshot, GallerySortDirection, GalleryTimeAnchor, GalleryTimeline, ScanError};

#[cfg(test)]
mod tests;

#[derive(Clone, Debug)]
pub struct GalleryTimeIntent {
    pub month_key: Option<String>,
    pub item_offset: u64,
}

#[derive(Clone, Debug)]
pub struct GalleryTimeSnapshot {
    pub snapshot: CatalogSnapshot,
    pub timeline: GalleryTimeline,
    pub anchor: Option<GalleryTimeAnchor>,
    pub window_start_ordinal: u64,
}

pub(crate) struct ResolvedGalleryTimeIntent {
    pub anchor: GalleryTimeAnchor,
    pub ordinal: u64,
}

impl ResolvedGalleryTimeIntent {
    pub(crate) fn window_start(
        &self,
        timeline: &GalleryTimeline,
        max_items: u32,
    ) -> Result<Self, ScanError> {
        // Publication can change row boundaries; keep a bounded prefix before the target.
        let ordinal = self.ordinal.saturating_sub(u64::from(max_items / 2));
        let mut remaining = ordinal;
        for bucket in &timeline.buckets {
            if remaining < bucket.item_count {
                return Ok(Self {
                    anchor: GalleryTimeAnchor {
                        revision: timeline.revision,
                        query_id: timeline.query_id.clone(),
                        month_key: bucket.month_key.clone(),
                        item_offset: remaining,
                    },
                    ordinal,
                });
            }
            remaining -= bucket.item_count;
        }
        Err(invalid_count())
    }
}

impl GalleryTimeIntent {
    pub(crate) fn resolve(
        &self,
        timeline: &GalleryTimeline,
        direction: &GallerySortDirection,
    ) -> Result<Option<ResolvedGalleryTimeIntent>, ScanError> {
        if let Some(month) = &self.month_key {
            validate_month_key(month)?;
        }
        let mut preceding = 0_u64;
        let mut last = None;
        for bucket in &timeline.buckets {
            if bucket.item_count == 0 {
                continue;
            }
            let same_month = bucket.month_key == self.month_key;
            let follows_intent = match (&bucket.month_key, &self.month_key) {
                (Some(current), Some(requested)) => match direction {
                    GallerySortDirection::Ascending => current > requested,
                    GallerySortDirection::Descending => current < requested,
                },
                (None, Some(_)) => true,
                _ => false,
            };
            let offset = if same_month {
                self.item_offset.min(bucket.item_count - 1)
            } else if follows_intent {
                0
            } else {
                bucket.item_count - 1
            };
            let ordinal = preceding.checked_add(offset).ok_or_else(invalid_count)?;
            let resolved = ResolvedGalleryTimeIntent {
                anchor: GalleryTimeAnchor {
                    revision: timeline.revision,
                    query_id: timeline.query_id.clone(),
                    month_key: bucket.month_key.clone(),
                    item_offset: offset,
                },
                ordinal,
            };
            if same_month || follows_intent {
                return Ok(Some(resolved));
            }
            last = Some(resolved);
            preceding = preceding
                .checked_add(bucket.item_count)
                .ok_or_else(invalid_count)?;
        }
        Ok(last)
    }
}

fn validate_month_key(month: &str) -> Result<(), ScanError> {
    let bytes = month.as_bytes();
    if bytes.len() == 7
        && bytes[4] == b'-'
        && bytes[..4].iter().all(u8::is_ascii_digit)
        && bytes[5..].iter().all(u8::is_ascii_digit)
        && matches!(month[5..].parse::<u8>(), Ok(1..=12))
    {
        return Ok(());
    }
    Err(ScanError::new(
        "catalog_time_anchor_invalid",
        "A gallery month intent must use YYYY-MM with a month from 01 through 12",
    ))
}

fn invalid_count() -> ScanError {
    ScanError::new(
        "catalog_timeline_count_invalid",
        "The gallery timeline item count exceeds the supported range",
    )
}
