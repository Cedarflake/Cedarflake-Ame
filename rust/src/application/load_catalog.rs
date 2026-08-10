use std::path::Path;

use crate::adapters::{SqliteCatalog, inspect_root_availability};
use crate::domain::{
    CatalogCursor, CatalogSnapshot, GalleryLayoutManifestChunk, GalleryLayoutManifestCursor,
    GalleryQuery, GallerySortDirection, GallerySortKey, GalleryTimeAnchor, GalleryTimeline,
    LibraryFolderCursor, LibraryFolderPage, PreviewStatus, ScanError,
};
use crate::ports::CatalogRepository;

use super::storage_paths;

pub fn load_catalog(
    max_items: u32,
    query: GalleryQuery,
    after: Option<CatalogCursor>,
    before: Option<CatalogCursor>,
) -> Result<CatalogSnapshot, ScanError> {
    load_catalog_window(max_items, query, after, before, None)
}

fn load_catalog_window(
    max_items: u32,
    query: GalleryQuery,
    after: Option<CatalogCursor>,
    before: Option<CatalogCursor>,
    anchor: Option<GalleryTimeAnchor>,
) -> Result<CatalogSnapshot, ScanError> {
    let query = normalize_gallery_query(query);
    let query_id = gallery_query_identity(&query);
    let storage = storage_paths()?;
    let mut catalog = SqliteCatalog::open(storage.catalog_path)?;
    let mut snapshot = catalog.load_snapshot(
        max_items,
        &query,
        &query_id,
        after.as_ref(),
        before.as_ref(),
        anchor.as_ref(),
    )?;
    for root in &mut snapshot.roots {
        let evidence = inspect_root_availability(&root.path);
        root.availability = evidence.availability;
        root.availability_message = evidence.message;
    }
    for asset in &mut snapshot.assets {
        if matches!(asset.preview_status, PreviewStatus::Ready)
            && (asset.preview_path.is_empty() || !Path::new(&asset.preview_path).is_file())
        {
            asset.preview_path.clear();
            asset.preview_status = PreviewStatus::Pending;
            asset.preview_issue_code = None;
            asset.preview_issue_message = None;
        }
    }
    Ok(snapshot)
}

pub fn load_gallery_timeline(query: GalleryQuery) -> Result<GalleryTimeline, ScanError> {
    let query = normalize_gallery_query(query);
    let query_id = gallery_query_identity(&query);
    let storage = storage_paths()?;
    let mut catalog = SqliteCatalog::open(storage.catalog_path)?;
    catalog.load_gallery_timeline(&query, &query_id)
}

pub fn load_gallery_layout_manifest_chunk(
    max_items: u32,
    query: GalleryQuery,
    after: Option<GalleryLayoutManifestCursor>,
) -> Result<GalleryLayoutManifestChunk, ScanError> {
    let query = normalize_gallery_query(query);
    let query_id = gallery_query_identity(&query);
    let storage = storage_paths()?;
    let mut catalog = SqliteCatalog::open(storage.catalog_path)?;
    catalog.load_gallery_layout_manifest_chunk(max_items, &query, &query_id, after.as_ref())
}

pub fn load_library_folders(
    root_id: String,
    parent_relative_path: String,
    max_items: u32,
    after: Option<LibraryFolderCursor>,
) -> Result<LibraryFolderPage, ScanError> {
    let storage = storage_paths()?;
    let mut catalog = SqliteCatalog::open(storage.catalog_path)?;
    catalog.load_folder_page(
        &root_id,
        &normalize_relative_folder(&parent_relative_path),
        max_items,
        after.as_ref(),
    )
}

pub fn unregister_library_root(root_id: String) -> Result<bool, ScanError> {
    if root_id.trim().is_empty() {
        return Err(ScanError::new(
            "catalog_root_id_invalid",
            "A library root identifier is required",
        ));
    }
    let storage = storage_paths()?;
    let mut catalog = SqliteCatalog::open(storage.catalog_path)?;
    catalog.unregister_root(&root_id)
}

pub fn load_catalog_at_time(
    max_items: u32,
    query: GalleryQuery,
    anchor: GalleryTimeAnchor,
) -> Result<CatalogSnapshot, ScanError> {
    load_catalog_window(max_items, query, None, None, Some(anchor))
}

fn normalize_gallery_query(mut query: GalleryQuery) -> GalleryQuery {
    query.search_text = query.search_text.trim().to_owned();
    query.folder_relative_path = query
        .folder_relative_path
        .map(|folder| normalize_relative_folder(&folder));
    query
}

fn normalize_relative_folder(folder: &str) -> String {
    folder.replace('\\', "/").trim_matches('/').to_owned()
}

fn gallery_query_identity(query: &GalleryQuery) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"ame-gallery-query-v1\0");
    update_query_identity(&mut hasher, query.root_id.as_deref().unwrap_or_default());
    update_query_identity(
        &mut hasher,
        query.folder_relative_path.as_deref().unwrap_or_default(),
    );
    hasher.update(&[u8::from(query.include_descendants)]);
    update_query_identity(&mut hasher, &query.search_text);
    hasher.update(&[match query.sort_key {
        GallerySortKey::CaptureTime => 0,
        GallerySortKey::CreatedTime => 1,
        GallerySortKey::ModifiedTime => 2,
        GallerySortKey::FileName => 3,
    }]);
    hasher.update(&[match query.sort_direction {
        GallerySortDirection::Ascending => 0,
        GallerySortDirection::Descending => 1,
    }]);
    hasher.finalize().to_hex().to_string()
}

fn update_query_identity(hasher: &mut blake3::Hasher, value: &str) {
    hasher.update(&(value.len() as u64).to_le_bytes());
    hasher.update(value.as_bytes());
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::path::PathBuf;

    use crate::domain::{LibraryRootAvailability, PreviewStatus};

    use super::*;

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

        assert_ne!(
            gallery_query_identity(&base),
            gallery_query_identity(&ascending)
        );
        assert_ne!(
            gallery_query_identity(&base),
            gallery_query_identity(&direct_children)
        );
    }

    #[cfg(windows)]
    #[test]
    #[ignore = "requires explicit access to the retained real-library acceptance catalog"]
    fn user_authorized_combined_catalog_load_acceptance() {
        const CONSENT: &str = "CEDARFLAKE_AME_READ_ONLY_ACCEPTANCE_V1";
        assert_eq!(
            std::env::var("CEDARFLAKE_AME_ACCEPTANCE_CONSENT")
                .expect("explicit acceptance consent is required"),
            CONSENT,
        );
        let storage_root = canonical_environment_path(
            "CEDARFLAKE_AME_ACCEPTANCE_STORAGE_ROOT",
            "acceptance storage",
        );
        assert_eq!(
            canonical_environment_path("CEDARFLAKE_AME_TEST_STORAGE_ROOT", "test storage"),
            storage_root,
            "production load API must use the retained acceptance storage",
        );
        let expected_roots = [
            canonical_environment_path("CEDARFLAKE_AME_COMBINED_ROOT_A", "first root"),
            canonical_environment_path("CEDARFLAKE_AME_COMBINED_ROOT_B", "second root"),
        ]
        .into_iter()
        .map(|path| normalized_path(&path))
        .collect::<HashSet<_>>();

        let mut cursor = None;
        let mut expected_revision = None;
        let mut location_ids = HashSet::new();
        let mut page_count = 0_u64;
        let mut expected_location_count = None;
        let mut previous_gallery_key = None;
        let mut known_capture_times = 0_u64;
        let mut unknown_capture_times = 0_u64;
        let mut newest_capture_time = None;
        let mut oldest_capture_time = None;
        loop {
            let snapshot = load_catalog(512, GalleryQuery::default(), cursor, None)
                .expect("production catalog page");
            page_count += 1;
            assert_eq!(
                PathBuf::from(&snapshot.catalog_path)
                    .canonicalize()
                    .expect("loaded catalog path"),
                storage_root.join("catalog").join("ame.sqlite3"),
            );
            if let Some(revision) = expected_revision {
                assert_eq!(
                    snapshot.revision, revision,
                    "catalog revision changed mid-read"
                );
            } else {
                expected_revision = Some(snapshot.revision);
            }

            let actual_roots = snapshot
                .roots
                .iter()
                .map(|root| normalized_path(Path::new(&root.path)))
                .collect::<HashSet<_>>();
            assert_eq!(actual_roots, expected_roots);
            assert!(snapshot.roots.iter().all(|root| {
                root.active_scan_id.is_some()
                    && matches!(root.availability, LibraryRootAvailability::Available)
            }));
            let root_location_count = snapshot
                .roots
                .iter()
                .map(|root| root.asset_count)
                .sum::<u64>();
            if let Some(expected) = expected_location_count {
                assert_eq!(root_location_count, expected);
            } else {
                expected_location_count = Some(root_location_count);
            }

            for asset in snapshot.assets {
                let capture_time_key = asset
                    .capture_time
                    .as_ref()
                    .map(|capture| capture.local_time.clone())
                    .unwrap_or_default();
                let gallery_key = (
                    asset.capture_time.is_none(),
                    capture_time_key.clone(),
                    asset.modified_unix_ms,
                    asset.root_id.clone(),
                    asset.location_id.clone(),
                );
                if let Some(previous) = &previous_gallery_key {
                    assert!(
                        gallery_key_follows(previous, &gallery_key),
                        "real catalog gallery order regressed: {previous:?} then {gallery_key:?}"
                    );
                }
                previous_gallery_key = Some(gallery_key);
                if asset.capture_time.is_some() {
                    known_capture_times += 1;
                    newest_capture_time.get_or_insert_with(|| capture_time_key.clone());
                    oldest_capture_time = Some(capture_time_key);
                } else {
                    unknown_capture_times += 1;
                }
                assert!(
                    matches!(asset.preview_status, PreviewStatus::Pending),
                    "the catalog-only acceptance unexpectedly generated a preview"
                );
                assert!(
                    location_ids.insert(asset.location_id),
                    "a location appeared in more than one bounded page"
                );
            }
            cursor = snapshot.next_cursor;
            if cursor.is_none() {
                break;
            }
        }

        let expected_location_count = expected_location_count.expect("root location count");
        assert!(expected_location_count > 0);
        assert_eq!(
            u64::try_from(location_ids.len()).expect("loaded location count"),
            expected_location_count,
        );
        println!(
            "AME_COMBINED_CATALOG_ACCEPTANCE roots={} locations={} pages={} revision={} \
             capture_known={} capture_unknown={} newest_capture={:?} oldest_capture={:?} \
             order=gallery_time_v1 previews=pending availability=available",
            expected_roots.len(),
            expected_location_count,
            page_count,
            expected_revision.expect("catalog revision"),
            known_capture_times,
            unknown_capture_times,
            newest_capture_time,
            oldest_capture_time,
        );
    }

    #[cfg(windows)]
    fn gallery_key_follows(
        previous: &(bool, String, i64, String, String),
        current: &(bool, String, i64, String, String),
    ) -> bool {
        if previous.0 != current.0 {
            return !previous.0 && current.0;
        }
        if previous.1 != current.1 {
            return previous.1 > current.1;
        }
        if previous.2 != current.2 {
            return previous.2 > current.2;
        }
        if previous.3 != current.3 {
            return previous.3 < current.3;
        }
        previous.4 < current.4
    }

    #[cfg(windows)]
    fn canonical_environment_path(name: &str, label: &str) -> PathBuf {
        PathBuf::from(std::env::var(name).unwrap_or_else(|_| panic!("{label} is required")))
            .canonicalize()
            .unwrap_or_else(|error| panic!("{label} must be available: {error}"))
    }

    #[cfg(windows)]
    fn normalized_path(path: &Path) -> String {
        path.to_string_lossy()
            .replace('/', "\\")
            .trim_end_matches('\\')
            .to_lowercase()
    }
}
