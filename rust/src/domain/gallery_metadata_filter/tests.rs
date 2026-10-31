use super::*;

#[test]
fn calendar_dates_and_numeric_bounds_reject_invalid_filters() {
    for date in [
        "2026-02-29",
        "2026-04-31",
        "0000-01-01",
        "2026-13-01",
        "2026-1-01",
        "😀-01-01",
    ] {
        let filter = GalleryMetadataFilter {
            date: GalleryDateRange {
                start: Some(date.into()),
                ..Default::default()
            },
            ..Default::default()
        };
        assert!(filter.validate().is_err(), "{date}");
    }
    let leap = GalleryMetadataFilter {
        date: GalleryDateRange {
            start: Some("2024-02-29".into()),
            end: Some("2024-03-01".into()),
            ..Default::default()
        },
        ..Default::default()
    };
    assert!(leap.validate().is_ok());
    for range in [
        GalleryNumberRange {
            minimum: Some(2),
            maximum: Some(1),
        },
        GalleryNumberRange {
            minimum: None,
            maximum: Some(u64::MAX),
        },
    ] {
        assert!(
            GalleryMetadataFilter {
                bytes: range,
                ..Default::default()
            }
            .validate()
            .is_err()
        );
    }
    assert!(
        GalleryMetadataFilter {
            date: GalleryDateRange {
                start: Some("2026-01-02".into()),
                end: Some("2026-01-01".into()),
                ..Default::default()
            },
            ..Default::default()
        }
        .validate()
        .is_err()
    );
}

#[test]
fn normalization_retains_literal_camera_text_and_empty_date_identity() {
    let mut filter = GalleryMetadataFilter {
        extension: " .JPG ".into(),
        camera_text: " Maker%_One ".into(),
        date: GalleryDateRange {
            field: GalleryDateField::Modified,
            ..Default::default()
        },
        ..Default::default()
    };
    filter.normalize();
    assert_eq!(filter.extension, "jpg");
    assert_eq!(filter.camera_text, "Maker%_One");
    assert_eq!(filter.date.field, GalleryDateField::Capture);
    assert!(filter.validate().is_ok());
    filter.extension = "jpg' OR 1=1".into();
    assert!(filter.validate().is_err());
}
