use super::*;
use crate::domain::GalleryTimeBucket;

fn timeline(direction: &GallerySortDirection, unknown: bool) -> GalleryTimeline {
    let mut buckets = vec![
        GalleryTimeBucket {
            month_key: Some("2010-01".to_owned()),
            item_count: 3,
            aspect_ratio_milli_sum: 3000,
        },
        GalleryTimeBucket {
            month_key: Some("2012-03".to_owned()),
            item_count: 5,
            aspect_ratio_milli_sum: 5000,
        },
        GalleryTimeBucket {
            month_key: Some("2026-09".to_owned()),
            item_count: 2,
            aspect_ratio_milli_sum: 2000,
        },
    ];
    if matches!(direction, GallerySortDirection::Descending) {
        buckets.reverse();
    }
    if unknown {
        buckets.push(GalleryTimeBucket {
            month_key: None,
            item_count: 2,
            aspect_ratio_milli_sum: 2000,
        });
    }
    GalleryTimeline {
        revision: 7,
        query_id: "query".to_owned(),
        total_items: if unknown { 12 } else { 10 },
        buckets,
    }
}

#[test]
fn current_time_intent_preserves_ordered_fallbacks_and_endpoint_clamping() {
    for (direction, requested, offset, unknown, expected, resolved_offset, ordinal) in [
        (
            GallerySortDirection::Descending,
            Some("2011-06"),
            99,
            true,
            Some("2010-01"),
            0,
            7,
        ),
        (
            GallerySortDirection::Ascending,
            Some("2011-06"),
            99,
            true,
            Some("2012-03"),
            0,
            3,
        ),
        (
            GallerySortDirection::Descending,
            Some("2012-03"),
            99,
            true,
            Some("2012-03"),
            4,
            6,
        ),
        (
            GallerySortDirection::Ascending,
            Some("2012-03"),
            99,
            true,
            Some("2012-03"),
            4,
            7,
        ),
        (
            GallerySortDirection::Descending,
            Some("1990-01"),
            10,
            false,
            Some("2010-01"),
            2,
            9,
        ),
        (
            GallerySortDirection::Ascending,
            Some("2030-01"),
            10,
            false,
            Some("2026-09"),
            1,
            9,
        ),
        (
            GallerySortDirection::Descending,
            None,
            99,
            true,
            None,
            1,
            11,
        ),
        (
            GallerySortDirection::Descending,
            None,
            99,
            false,
            Some("2010-01"),
            2,
            9,
        ),
    ] {
        let intent = GalleryTimeIntent {
            month_key: requested.map(str::to_owned),
            item_offset: offset,
        };
        let result = intent
            .resolve(&timeline(&direction, unknown), &direction)
            .expect("valid intent")
            .expect("nonempty timeline");
        assert_eq!(result.anchor.month_key.as_deref(), expected);
        assert_eq!(result.anchor.item_offset, resolved_offset);
        assert_eq!(result.ordinal, ordinal);
        assert_eq!(result.anchor.revision, 7);
        assert_eq!(result.anchor.query_id, "query");
    }
}

#[test]
fn current_time_intent_has_no_anchor_for_empty_results() {
    let empty = GalleryTimeline {
        revision: 8,
        query_id: "query".to_owned(),
        total_items: 0,
        buckets: vec![],
    };
    let intent = GalleryTimeIntent {
        month_key: Some("2012-03".to_owned()),
        item_offset: 10,
    };
    assert!(
        intent
            .resolve(&empty, &GallerySortDirection::Descending)
            .expect("empty timeline")
            .is_none()
    );
}
