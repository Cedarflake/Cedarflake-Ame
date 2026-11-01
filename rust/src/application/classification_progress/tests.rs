use crate::domain::classification_review::ClassificationReviewFilter;
use crate::domain::{GallerySortDirection, GallerySortKey};

use super::*;

#[test]
fn smart_album_progress_folds_groups_without_losing_its_observed_scope() {
    let query = GalleryQuery {
        collection: GalleryCollection::SmartAlbum {
            category_id: Some("drawing".into()),
            review: ClassificationReviewFilter::Pending,
        },
        duplicate_mode: GalleryDuplicateMode::All,
        root_id: Some("root".into()),
        folder_relative_path: Some("subfolder".into()),
        include_descendants: false,
        search_text: "image".into(),
        sort_key: GallerySortKey::FileName,
        sort_direction: GallerySortDirection::Ascending,
        ..Default::default()
    };
    let scope = review_scope(query.clone()).unwrap();
    assert_eq!(scope.gallery.duplicate_mode, GalleryDuplicateMode::Merged);
    assert_eq!(
        scope.gallery_query(),
        GalleryQuery {
            duplicate_mode: GalleryDuplicateMode::Merged,
            ..query
        }
    );
    assert!(review_scope(GalleryQuery::default()).is_err());
}
