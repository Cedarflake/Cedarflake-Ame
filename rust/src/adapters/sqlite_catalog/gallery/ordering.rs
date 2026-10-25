use std::borrow::Cow;

use crate::domain::{GalleryQuery, GallerySortKey};

pub(super) struct GalleryOrderExpressions {
    pub(super) missing: &'static str,
    pub(super) text: &'static str,
    pub(super) number: Cow<'static, str>,
    pub(super) date: Option<&'static str>,
    pub(super) month: Option<&'static str>,
}

pub(super) fn gallery_order_expressions(query: &GalleryQuery) -> GalleryOrderExpressions {
    let mut order = match query.sort_key {
        GallerySortKey::CaptureTime => GalleryOrderExpressions {
            missing: "(COALESCE(locations.capture_local_time, locations.file_local_time) IS NULL)",
            text: "IFNULL(COALESCE(locations.capture_local_time, locations.file_local_time), '')",
            number: Cow::Borrowed("locations.modified_unix_ms"),
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
            number: Cow::Borrowed("locations.modified_unix_ms"),
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
            number: Cow::Borrowed("locations.modified_unix_ms"),
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
            number: Cow::Borrowed("CAST(0 AS INTEGER)"),
            date: None,
            month: None,
        },
        GallerySortKey::LibraryAdmission => GalleryOrderExpressions {
            missing: "CAST(0 AS INTEGER)",
            text: "CAST('' AS TEXT)",
            number: Cow::Owned(super::organization_queue::admission_expression("locations")),
            date: None,
            month: None,
        },
    };
    if query.organization_queue.is_some() {
        order.date = None;
        order.month = None;
    }
    order
}
