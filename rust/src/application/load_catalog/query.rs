use crate::domain::organization_queue::OrganizationQueue;
use crate::domain::{GalleryCollection, GalleryQuery, GallerySortDirection, GallerySortKey};

pub(super) fn normalize_gallery_query(mut query: GalleryQuery) -> GalleryQuery {
    query.search_text = query.search_text.trim().to_owned();
    query.metadata_filter.normalize();
    if query.organization_queue.is_none() {
        query.new_since_admission = None;
    }
    query.folder_relative_path = query
        .folder_relative_path
        .map(|folder| normalize_relative_folder(&folder));
    if query.sort_key == GallerySortKey::LibraryAdmission {
        query.sort_direction = GallerySortDirection::Descending;
    }
    if query.organization_queue.is_some()
        || matches!(query.collection, GalleryCollection::Album { .. })
    {
        query.duplicate_mode = crate::domain::GalleryDuplicateMode::Merged;
    }
    query
}

pub(super) fn normalize_relative_folder(folder: &str) -> String {
    folder.replace('\\', "/").trim_matches('/').to_owned()
}

pub(super) fn gallery_query_identity(query: &GalleryQuery) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"ame-gallery-query-v5\0");
    if let Some(boundary) = query.new_since_admission {
        hasher.update(b"arrivals\0");
        hasher.update(&boundary.to_le_bytes());
    }
    hasher.update(&[match query.organization_queue {
        None => 0,
        Some(OrganizationQueue::Pending) => 1,
        Some(OrganizationQueue::Deferred) => 2,
        Some(OrganizationQueue::Ignored) => 3,
    }]);
    match &query.collection {
        GalleryCollection::Library => {
            hasher.update(&[0]);
        }
        GalleryCollection::Favorites => {
            hasher.update(&[1]);
        }
        GalleryCollection::Album { album_id } => {
            hasher.update(&[2]);
            update_query_identity(&mut hasher, album_id);
        }
    }
    update_query_identity(&mut hasher, query.root_id.as_deref().unwrap_or_default());
    update_query_identity(
        &mut hasher,
        query.folder_relative_path.as_deref().unwrap_or_default(),
    );
    hasher.update(&[u8::from(query.include_descendants)]);
    update_query_identity(&mut hasher, &query.search_text);
    update_metadata_identity(&mut hasher, &query.metadata_filter);
    hasher.update(&[match query.sort_key {
        GallerySortKey::CaptureTime => 0,
        GallerySortKey::CreatedTime => 1,
        GallerySortKey::ModifiedTime => 2,
        GallerySortKey::FileName => 3,
        GallerySortKey::LibraryAdmission => 4,
    }]);
    hasher.update(&[match query.sort_direction {
        GallerySortDirection::Ascending => 0,
        GallerySortDirection::Descending => 1,
    }]);
    hasher.update(&[match query.duplicate_mode {
        crate::domain::GalleryDuplicateMode::All => 0,
        crate::domain::GalleryDuplicateMode::Merged => 1,
    }]);
    hasher.finalize().to_hex().to_string()
}

fn update_query_identity(hasher: &mut blake3::Hasher, value: &str) {
    hasher.update(&(value.len() as u64).to_le_bytes());
    hasher.update(value.as_bytes());
}

fn update_metadata_identity(
    hasher: &mut blake3::Hasher,
    filter: &crate::domain::gallery_metadata_filter::GalleryMetadataFilter,
) {
    use crate::domain::gallery_metadata_filter::GalleryDateField;
    update_query_identity(hasher, &filter.extension);
    update_query_identity(hasher, &filter.camera_text);
    for range in [&filter.bytes, &filter.width, &filter.height] {
        for value in [range.minimum, range.maximum] {
            hasher.update(&[u8::from(value.is_some())]);
            if let Some(value) = value {
                hasher.update(&value.to_le_bytes());
            }
        }
    }
    hasher.update(&[match filter.date.field {
        GalleryDateField::Capture => 0,
        GalleryDateField::Created => 1,
        GalleryDateField::Modified => 2,
    }]);
    update_query_identity(hasher, filter.date.start.as_deref().unwrap_or_default());
    update_query_identity(hasher, filter.date.end.as_deref().unwrap_or_default());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metadata_identity_separates_each_predicate_and_normalizes_empty_dates() {
        use crate::domain::gallery_metadata_filter::*;
        let base = GalleryQuery::default();
        let mut filters = vec![GalleryMetadataFilter::default()];
        filters.push(GalleryMetadataFilter {
            extension: "jpg".into(),
            ..Default::default()
        });
        filters.push(GalleryMetadataFilter {
            camera_text: "Camera".into(),
            ..Default::default()
        });
        for bound in [Some(0), Some(10)] {
            filters.push(GalleryMetadataFilter {
                bytes: GalleryNumberRange {
                    minimum: bound,
                    maximum: None,
                },
                ..Default::default()
            });
            filters.push(GalleryMetadataFilter {
                width: GalleryNumberRange {
                    minimum: None,
                    maximum: bound,
                },
                ..Default::default()
            });
            filters.push(GalleryMetadataFilter {
                height: GalleryNumberRange {
                    minimum: bound,
                    maximum: None,
                },
                ..Default::default()
            });
        }
        for field in [
            GalleryDateField::Capture,
            GalleryDateField::Created,
            GalleryDateField::Modified,
        ] {
            filters.push(GalleryMetadataFilter {
                date: GalleryDateRange {
                    field,
                    start: Some("2026-10-30".into()),
                    end: None,
                },
                ..Default::default()
            });
            filters.push(GalleryMetadataFilter {
                date: GalleryDateRange {
                    field,
                    start: None,
                    end: Some("2026-10-30".into()),
                },
                ..Default::default()
            });
        }
        let identities = filters
            .iter()
            .map(|filter| {
                gallery_query_identity(&GalleryQuery {
                    metadata_filter: filter.clone(),
                    ..base.clone()
                })
            })
            .collect::<std::collections::HashSet<_>>();
        assert_eq!(identities.len(), filters.len());
        let mut empty = base.clone();
        empty.metadata_filter.date.field = GalleryDateField::Modified;
        assert_eq!(normalize_gallery_query(empty), base);
    }

    #[test]
    fn queue_identity_preserves_sort_choices_and_separates_progress_scopes() {
        let ordinary = GalleryQuery {
            sort_key: GallerySortKey::FileName,
            sort_direction: GallerySortDirection::Ascending,
            ..GalleryQuery::default()
        };
        let mut identities = std::collections::HashSet::new();
        identities.insert(gallery_query_identity(&ordinary));
        for queue in [
            OrganizationQueue::Pending,
            OrganizationQueue::Deferred,
            OrganizationQueue::Ignored,
        ] {
            let first = normalize_gallery_query(GalleryQuery {
                organization_queue: Some(queue),
                ..ordinary.clone()
            });
            let second = normalize_gallery_query(GalleryQuery {
                organization_queue: Some(queue),
                ..GalleryQuery::default()
            });
            assert_ne!(first, second);
            assert_eq!(first.sort_key, ordinary.sort_key);
            assert_eq!(first.sort_direction, ordinary.sort_direction);
            assert!(first.folds_duplicates());
            identities.insert(gallery_query_identity(&first));
            identities.insert(gallery_query_identity(&second));
        }
        assert_eq!(identities.len(), 7);
        assert_eq!(normalize_gallery_query(ordinary.clone()), ordinary);
    }

    #[test]
    fn collection_identity_separates_albums_favorites_and_library_cursors() {
        let collections = [
            GalleryCollection::Library,
            GalleryCollection::Favorites,
            GalleryCollection::Album {
                album_id: "first".into(),
            },
            GalleryCollection::Album {
                album_id: "second".into(),
            },
        ];
        let identities = collections
            .into_iter()
            .map(|collection| {
                gallery_query_identity(&normalize_gallery_query(GalleryQuery {
                    collection,
                    ..GalleryQuery::default()
                }))
            })
            .collect::<std::collections::HashSet<_>>();
        assert_eq!(identities.len(), 4);
    }

    #[test]
    fn album_display_preference_normalizes_to_the_same_folded_query() {
        let query = GalleryQuery {
            collection: GalleryCollection::Album {
                album_id: "album".into(),
            },
            ..GalleryQuery::default()
        };
        let merged = GalleryQuery {
            duplicate_mode: crate::domain::GalleryDuplicateMode::Merged,
            ..query.clone()
        };
        assert_eq!(
            normalize_gallery_query(query),
            normalize_gallery_query(merged)
        );
    }
    #[test]
    fn query_identity_uses_normalized_search_and_folder_scope() {
        let first = normalize_gallery_query(GalleryQuery {
            root_id: Some("root-1".to_owned()),
            folder_relative_path: Some("/Memes\\Cats/".to_owned()),
            search_text: "  neko  ".to_owned(),
            ..GalleryQuery::default()
        });
        let second = GalleryQuery {
            root_id: Some("root-1".to_owned()),
            folder_relative_path: Some("Memes/Cats".to_owned()),
            search_text: "neko".to_owned(),
            ..GalleryQuery::default()
        };

        assert_eq!(
            gallery_query_identity(&first),
            gallery_query_identity(&second)
        );
    }

    #[test]
    fn query_identity_changes_with_sort_and_descendant_policy() {
        let base = GalleryQuery::default();
        let ascending = GalleryQuery {
            sort_direction: GallerySortDirection::Ascending,
            ..base.clone()
        };
        let direct_children = GalleryQuery {
            include_descendants: false,
            ..base.clone()
        };
        let merged = GalleryQuery {
            duplicate_mode: crate::domain::GalleryDuplicateMode::Merged,
            ..base.clone()
        };
        assert_ne!(
            gallery_query_identity(&base),
            gallery_query_identity(&merged)
        );

        assert_ne!(
            gallery_query_identity(&base),
            gallery_query_identity(&ascending)
        );
        assert_ne!(
            gallery_query_identity(&base),
            gallery_query_identity(&direct_children)
        );
    }
}
