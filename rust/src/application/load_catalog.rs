use std::path::Path;

#[cfg(test)]
use crate::adapters::SqliteCatalog;
use crate::adapters::{SqliteCatalogReadExecutor, inspect_root_availability};
use crate::domain::{
    AssetLocationView, CatalogCursor, CatalogSnapshot, GalleryLayoutManifestChunk,
    GalleryLayoutManifestCursor, GalleryQuery, GallerySortDirection, GallerySortKey,
    GalleryTimeAnchor, GalleryTimeline, LibraryChangeLane, LibraryFolderCursor, LibraryFolderPage,
    PreviewStatus, ScanError,
};
use crate::ports::CatalogRepository;

use super::preview_health::reconcile_snapshot_previews;
use super::storage_paths;

pub fn load_catalog(
    max_items: u32,
    query: GalleryQuery,
    after: Option<CatalogCursor>,
    before: Option<CatalogCursor>,
) -> Result<CatalogSnapshot, ScanError> {
    load_catalog_window(max_items, query, after, before, None, None)
}

fn load_catalog_window(
    max_items: u32,
    query: GalleryQuery,
    after: Option<CatalogCursor>,
    before: Option<CatalogCursor>,
    anchor: Option<GalleryTimeAnchor>,
    anchor_location_id: Option<String>,
) -> Result<CatalogSnapshot, ScanError> {
    let query = normalize_gallery_query(query);
    let query_id = gallery_query_identity(&query);
    let storage = storage_paths()?;
    let reader = prepare_catalog_reader(&storage.catalog_path)?;
    let mut snapshot = match anchor_location_id.as_deref() {
        Some(location_id) => {
            reader.load_snapshot_around_location(max_items, &query, &query_id, location_id)
        }
        None => reader.load_snapshot(
            max_items,
            &query,
            &query_id,
            after.as_ref(),
            before.as_ref(),
            anchor.as_ref(),
        ),
    }?;
    finish_loaded_snapshot(&storage, &mut snapshot)?;
    Ok(snapshot)
}

fn finish_loaded_snapshot(
    storage: &super::StoragePaths,
    snapshot: &mut CatalogSnapshot,
) -> Result<(), ScanError> {
    finish_loaded_preview_state(storage, snapshot)?;
    super::preview_recovery::start_preview_recovery(storage.clone());
    for root in &mut snapshot.roots {
        let evidence = inspect_root_availability(&root.path);
        root.availability = evidence.availability;
        root.availability_message = evidence.message;
    }
    Ok(())
}

fn finish_loaded_preview_state(
    storage: &super::StoragePaths,
    snapshot: &mut CatalogSnapshot,
) -> Result<(), ScanError> {
    let mut catalog =
        super::catalog_session::open_catalog(&storage.catalog_path, LibraryChangeLane::Recovery)?;
    reconcile_snapshot_previews(&mut catalog, &storage.preview_root, snapshot)?;
    let visible_preview_artifacts = snapshot
        .assets
        .iter()
        .filter(|asset| {
            matches!(asset.preview_status, PreviewStatus::Ready) && !asset.preview_path.is_empty()
        })
        .map(|asset| (asset.location_id.clone(), asset.preview_path.clone()))
        .collect::<Vec<_>>();
    catalog.touch_preview_artifacts(&visible_preview_artifacts)?;
    Ok(())
}

pub fn load_gallery_timeline(query: GalleryQuery) -> Result<GalleryTimeline, ScanError> {
    let query = normalize_gallery_query(query);
    let query_id = gallery_query_identity(&query);
    let storage = storage_paths()?;
    prepare_catalog_reader(&storage.catalog_path)?.load_gallery_timeline(&query, &query_id)
}

pub fn load_gallery_layout_manifest_chunk(
    max_items: u32,
    query: GalleryQuery,
    after: Option<GalleryLayoutManifestCursor>,
) -> Result<GalleryLayoutManifestChunk, ScanError> {
    let query = normalize_gallery_query(query);
    let query_id = gallery_query_identity(&query);
    let storage = storage_paths()?;
    prepare_catalog_reader(&storage.catalog_path)?.load_gallery_layout_manifest_chunk(
        max_items,
        &query,
        &query_id,
        after.as_ref(),
    )
}

pub fn load_library_folders(
    root_id: String,
    parent_relative_path: String,
    max_items: u32,
    after: Option<LibraryFolderCursor>,
) -> Result<LibraryFolderPage, ScanError> {
    let storage = storage_paths()?;
    prepare_catalog_reader(&storage.catalog_path)?.load_folder_page(
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
    super::preempt_catalog_reclamation(&storage.catalog_path);
    let mut catalog =
        super::catalog_session::open_catalog(&storage.catalog_path, LibraryChangeLane::Recovery)?;
    let removed = catalog.unregister_root(&root_id)?;
    drop(catalog);
    if removed {
        super::schedule_catalog_reclamation(storage.catalog_path);
    }
    Ok(removed)
}

pub fn load_catalog_at_time(
    max_items: u32,
    query: GalleryQuery,
    anchor: GalleryTimeAnchor,
) -> Result<CatalogSnapshot, ScanError> {
    load_catalog_window(max_items, query, None, None, Some(anchor), None)
}

pub fn load_catalog_around_location(
    max_items: u32,
    query: GalleryQuery,
    anchor_location_id: String,
) -> Result<CatalogSnapshot, ScanError> {
    if anchor_location_id.trim().is_empty() {
        return Err(ScanError::new(
            "catalog_location_anchor_invalid",
            "A gallery location anchor identifier is required",
        ));
    }
    load_catalog_window(max_items, query, None, None, None, Some(anchor_location_id))
}

pub fn load_catalog_around_asset(
    max_items: u32,
    query: GalleryQuery,
    requested_location_id: String,
    anchor_asset_id: String,
    fallback_ordinal: u64,
) -> Result<CatalogSnapshot, ScanError> {
    if requested_location_id.trim().is_empty() || anchor_asset_id.trim().is_empty() {
        return Err(ScanError::new(
            "catalog_asset_anchor_invalid",
            "A gallery asset anchor requires stable location and asset identifiers",
        ));
    }
    let query = normalize_gallery_query(query);
    let query_id = gallery_query_identity(&query);
    let storage = storage_paths()?;
    let mut snapshot = prepare_catalog_reader(&storage.catalog_path)?.load_snapshot_around_asset(
        max_items,
        &query,
        &query_id,
        &requested_location_id,
        &anchor_asset_id,
        fallback_ordinal,
    )?;
    finish_loaded_snapshot(&storage, &mut snapshot)?;
    Ok(snapshot)
}

pub fn load_catalog_asset_by_id(
    asset_id: String,
    preferred_location_id: Option<String>,
) -> Result<Option<AssetLocationView>, ScanError> {
    if asset_id.trim().is_empty()
        || preferred_location_id
            .as_deref()
            .is_some_and(|location_id| location_id.trim().is_empty())
    {
        return Err(ScanError::new(
            "catalog_asset_identity_invalid",
            "A stable asset lookup requires a non-empty asset and optional location identifier",
        ));
    }
    let storage = storage_paths()?;
    prepare_catalog_reader(&storage.catalog_path)?
        .load_active_location_by_asset_id(&asset_id, preferred_location_id.as_deref())
}

fn prepare_catalog_reader(path: &Path) -> Result<SqliteCatalogReadExecutor, ScanError> {
    super::catalog_session::open_catalog_reader(path)
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
    use std::fs;
    use std::path::PathBuf;
    use std::process::Command;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::sync::mpsc;
    use std::thread;
    use std::time::Duration;

    use tempfile::tempdir;

    use crate::adapters::{
        PREVIEW_ALGORITHM_ID, PREVIEW_ALGORITHM_VERSION, PREVIEW_CACHE_VERSION,
        PREVIEW_ORIENTATION_CONTRACT,
    };
    use crate::domain::{
        AssetLocationView, LibraryRootAvailability, PreviewArtifact, PreviewStatus, ScanRequest,
        SourceRevisionEvidence,
    };

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

    #[test]
    fn current_catalog_read_preparation_validates_once_then_reuses_the_session() {
        let storage = tempdir().expect("storage");
        let catalog_path = storage.path().join("catalog").join("ame.sqlite3");
        drop(SqliteCatalog::open(catalog_path.clone()).expect("initialize catalog"));
        super::super::catalog_session::reset_catalog_session(&catalog_path);
        crate::adapters::reset_full_schema_validation_count(&catalog_path);

        let reader = prepare_catalog_reader(&catalog_path).expect("read executor");
        let timeline = reader
            .load_gallery_timeline(&GalleryQuery::default(), "current-schema-query")
            .expect("current catalog read");
        let second_reader = prepare_catalog_reader(&catalog_path).expect("reused read executor");
        let second_timeline = second_reader
            .load_gallery_timeline(&GalleryQuery::default(), "second-current-schema-query")
            .expect("second current catalog read");

        assert!(timeline.buckets.is_empty());
        assert!(second_timeline.buckets.is_empty());
        assert_eq!(
            crate::adapters::full_schema_validation_count(&catalog_path),
            1,
            "gallery reads must share one process-owned schema validation",
        );
    }

    #[test]
    fn consecutive_loaded_snapshot_finishes_share_one_validated_catalog_session() {
        let directory = tempdir().expect("storage");
        let storage = super::super::StoragePaths {
            catalog_path: directory.path().join("catalog").join("ame.sqlite3"),
            preview_root: directory.path().join("previews"),
            preview_budget_bytes: 64 * 1024 * 1024,
            settings_path: directory.path().join("settings").join("storage.sqlite3"),
        };
        drop(SqliteCatalog::open(storage.catalog_path.clone()).expect("initialize catalog"));
        super::super::catalog_session::reset_catalog_session(&storage.catalog_path);
        crate::adapters::reset_full_schema_validation_count(&storage.catalog_path);
        let mut snapshot = CatalogSnapshot {
            catalog_path: storage.catalog_path.to_string_lossy().into_owned(),
            revision: 0,
            query_id: "session-cache-query".to_owned(),
            roots: Vec::new(),
            assets: Vec::new(),
            previous_cursor: None,
            next_cursor: None,
            query_anchor_resolution: None,
        };

        finish_loaded_preview_state(&storage, &mut snapshot).expect("first finish");
        finish_loaded_preview_state(&storage, &mut snapshot).expect("second finish");

        assert_eq!(
            crate::adapters::full_schema_validation_count(&storage.catalog_path),
            1,
            "repeated loaded-snapshot writes must reuse one validated catalog session",
        );
    }

    #[test]
    fn public_catalog_reads_remain_available_during_a_concurrent_wal_writer() {
        const CHILD_TEST: &str = "application::load_catalog::tests::public_catalog_read_wal_child";
        let storage = tempdir().expect("storage");
        let catalog_path = storage.path().join("catalog").join("ame.sqlite3");
        drop(SqliteCatalog::open(catalog_path.clone()).expect("initialize catalog"));
        let stop = Arc::new(AtomicBool::new(false));
        let writes = Arc::new(AtomicUsize::new(0));
        let writer_stop = Arc::clone(&stop);
        let writer_count = Arc::clone(&writes);
        let (ready_sender, ready_receiver) = mpsc::sync_channel(1);
        let writer = thread::spawn(move || -> Result<(), String> {
            let connection = rusqlite::Connection::open(catalog_path)
                .map_err(|error| format!("WAL writer open: {error}"))?;
            connection
                .busy_timeout(Duration::from_secs(5))
                .map_err(|error| format!("writer busy timeout: {error}"))?;
            ready_sender
                .send(())
                .map_err(|error| format!("writer readiness: {error}"))?;
            while !writer_stop.load(Ordering::Acquire) {
                connection
                    .execute_batch(
                        "BEGIN IMMEDIATE;
                         UPDATE catalog_state SET revision = revision + 1;
                         COMMIT;",
                    )
                    .map_err(|error| format!("WAL writer transaction: {error}"))?;
                writer_count.fetch_add(1, Ordering::Release);
            }
            Ok(())
        });
        ready_receiver
            .recv_timeout(Duration::from_secs(5))
            .expect("WAL writer readiness");
        while writes.load(Ordering::Acquire) == 0 && !writer.is_finished() {
            thread::yield_now();
        }

        let status = Command::new(std::env::current_exe().expect("current test executable"))
            .args([
                "--exact",
                CHILD_TEST,
                "--ignored",
                "--nocapture",
                "--test-threads=1",
            ])
            .env("CEDARFLAKE_AME_TEST_STORAGE_ROOT", storage.path())
            .env("CEDARFLAKE_AME_PUBLIC_WAL_READ_CHILD", "1")
            .status()
            .expect("public catalog read child");
        stop.store(true, Ordering::Release);
        writer
            .join()
            .expect("WAL writer join")
            .expect("WAL writer completion");

        assert!(
            status.success(),
            "public catalog reads failed under WAL writes"
        );
        assert!(writes.load(Ordering::Acquire) >= 2);
    }

    #[test]
    #[ignore = "subprocess helper for the concurrent public catalog-read regression"]
    fn public_catalog_read_wal_child() {
        assert_eq!(
            std::env::var("CEDARFLAKE_AME_PUBLIC_WAL_READ_CHILD").as_deref(),
            Ok("1"),
        );
        for _ in 0..256 {
            load_gallery_timeline(GalleryQuery::default())
                .expect("production public catalog timeline read");
        }
    }

    #[test]
    fn changed_preview_root_resets_the_location_without_deleting_the_old_artifact() {
        let storage = tempdir().expect("storage");
        let catalog_path = storage.path().join("catalog").join("ame.sqlite3");
        let old_preview_root = storage.path().join("old-previews");
        let new_preview_root = storage.path().join("new-previews");
        fs::create_dir_all(&old_preview_root).expect("old preview root");
        fs::create_dir_all(&new_preview_root).expect("new preview root");
        let old_artifact =
            old_preview_root.join(format!("{PREVIEW_CACHE_VERSION}-{}.jpg", "a".repeat(64)));
        fs::write(&old_artifact, b"owned derived artifact").expect("old artifact");
        let mut catalog = SqliteCatalog::open(catalog_path).expect("catalog");
        let request = ScanRequest {
            scan_id: "preview-root-scan".to_owned(),
            root_path: storage.path().join("source").to_string_lossy().into_owned(),
            max_items: None,
            max_entries: None,
            preview_edge: 512,
        };
        catalog
            .begin_scan(&request, "preview-root", &request.root_path)
            .expect("begin scan");
        catalog
            .prove_live_only_first_import_handoff_for_test(&request.scan_id)
            .expect("prove preview-root fixture first-import handoff");
        catalog
            .stage_location(
                &request.scan_id,
                "preview-root",
                &AssetLocationView {
                    asset_id: "preview-root-asset".to_owned(),
                    location_id: "preview-root-location".to_owned(),
                    root_id: "preview-root".to_owned(),
                    scan_id: request.scan_id.clone(),
                    absolute_path: storage
                        .path()
                        .join("source")
                        .join("one.png")
                        .to_string_lossy()
                        .into_owned(),
                    display_path: "source\\one.png".to_owned(),
                    relative_path: "one.png".to_owned(),
                    preview_path: String::new(),
                    file_size: 100,
                    created_unix_ms: Some(10),
                    modified_unix_ms: 20,
                    file_identity: None,
                    source_revision: Some(SourceRevisionEvidence {
                        scheme: "windows-file-change-time-100ns-v1".to_owned(),
                        value: "0000000000000001".to_owned(),
                    }),
                    source_generation: 0,
                    width: 4_032,
                    height: 3_024,
                    preview_status: PreviewStatus::Pending,
                    preview_issue_code: None,
                    preview_issue_message: None,
                    metadata_engine_id: "fixture".to_owned(),
                    metadata_engine_version: "1".to_owned(),
                    capture_time: None,
                },
            )
            .expect("stage location");
        catalog
            .publish_scan(&request.scan_id, "preview-root", 1, 0)
            .expect("publish scan");
        let mut location = catalog
            .load_active_location("preview-root-location")
            .expect("active preview location query")
            .expect("active preview location");
        location.preview_path = old_artifact.to_string_lossy().into_owned();
        location.preview_status = PreviewStatus::Ready;
        catalog
            .update_active_preview(
                &location,
                Some(&PreviewArtifact {
                    artifact_key: "preview-root-artifact".to_owned(),
                    algorithm_id: PREVIEW_ALGORITHM_ID.to_owned(),
                    algorithm_version: PREVIEW_ALGORITHM_VERSION,
                    orientation_contract: PREVIEW_ORIENTATION_CONTRACT.to_owned(),
                    size_bucket: 512,
                    path: location.preview_path.clone(),
                    byte_size: fs::metadata(&old_artifact)
                        .expect("old artifact metadata")
                        .len(),
                    encoded_width: 512,
                    encoded_height: 384,
                    width: location.width,
                    height: location.height,
                }),
                None,
            )
            .expect("publish preview artifact");
        let mut snapshot = catalog
            .load_snapshot(
                10,
                &GalleryQuery::default(),
                "preview-root-query",
                None,
                None,
                None,
            )
            .expect("snapshot");

        let alias_parent = old_preview_root.join("nested");
        fs::create_dir_all(&alias_parent).expect("preview alias parent");
        let equivalent_preview_root = alias_parent.join("..");
        reconcile_snapshot_previews(&mut catalog, &equivalent_preview_root, &mut snapshot)
            .expect("equivalent preview-root reconciliation");
        assert!(matches!(
            snapshot.assets[0].preview_status,
            PreviewStatus::Ready
        ));
        assert_eq!(
            Path::new(&snapshot.assets[0].preview_path),
            old_artifact.as_path()
        );

        reconcile_snapshot_previews(&mut catalog, &new_preview_root, &mut snapshot)
            .expect("preview-root reconciliation");

        let visible = &snapshot.assets[0];
        assert!(matches!(visible.preview_status, PreviewStatus::Pending));
        assert!(visible.preview_path.is_empty());
        assert_eq!((visible.width, visible.height), (4_032, 3_024));
        let stored = catalog
            .load_active_location("preview-root-location")
            .expect("stored location query")
            .expect("stored location");
        assert!(matches!(stored.preview_status, PreviewStatus::Pending));
        assert!(stored.preview_path.is_empty());
        assert_eq!((stored.width, stored.height), (4_032, 3_024));
        assert!(old_artifact.is_file());
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
        let mut known_capture_times = 0_u64;
        let mut unknown_capture_times = 0_u64;
        let mut newest_capture_time = None;
        let mut oldest_capture_time = None;
        loop {
            let requested_after = cursor.clone();
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

            if let Some(previous_end) = requested_after.as_ref() {
                let current_start = snapshot
                    .previous_cursor
                    .as_ref()
                    .expect("a continued page must expose its first keyset boundary");
                assert_eq!(
                    snapshot.assets.first().map(|asset| &asset.location_id),
                    Some(&current_start.location_id),
                    "the previous cursor must identify the first returned location"
                );
                assert!(
                    default_gallery_cursor_follows(previous_end, current_start),
                    "real catalog page boundary regressed: {previous_end:?} then {current_start:?}"
                );
            }
            if let (Some(current_start), Some(current_end)) = (
                snapshot.previous_cursor.as_ref(),
                snapshot.next_cursor.as_ref(),
            ) {
                assert!(
                    default_gallery_cursor_follows(current_start, current_end),
                    "real catalog page endpoints regressed: {current_start:?} then {current_end:?}"
                );
            }
            if let Some(current_end) = snapshot.next_cursor.as_ref() {
                assert_eq!(
                    snapshot.assets.last().map(|asset| &asset.location_id),
                    Some(&current_end.location_id),
                    "the next cursor must identify the last returned location"
                );
            }

            for asset in snapshot.assets {
                let capture_time_key = asset
                    .capture_time
                    .as_ref()
                    .map(|capture| capture.local_time.clone())
                    .unwrap_or_default();
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
             order=effective_gallery_time_v1 previews=pending availability=available",
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
    fn default_gallery_cursor_follows(previous: &CatalogCursor, current: &CatalogCursor) -> bool {
        if previous.primary_missing != current.primary_missing {
            return !previous.primary_missing && current.primary_missing;
        }
        if previous.primary_text != current.primary_text {
            return previous.primary_text > current.primary_text;
        }
        if previous.primary_number != current.primary_number {
            return previous.primary_number > current.primary_number;
        }
        if previous.root_id != current.root_id {
            return previous.root_id < current.root_id;
        }
        previous.location_id < current.location_id
    }

    #[cfg(windows)]
    #[test]
    fn retained_acceptance_cursor_check_matches_the_default_gallery_keyset() {
        let cursor = |primary_missing: bool,
                      primary_text: &str,
                      primary_number: i64,
                      root_id: &str,
                      location_id: &str| CatalogCursor {
            revision: 1,
            query_id: "query".to_owned(),
            primary_missing,
            primary_text: primary_text.to_owned(),
            primary_number,
            root_id: root_id.to_owned(),
            location_id: location_id.to_owned(),
        };
        let newest = cursor(false, "2026-08-01T16:30:17.000", 300, "root-a", "a");
        let older = cursor(false, "2026-07-01T16:30:17.000", 300, "root-a", "a");
        let lower_number = cursor(false, &older.primary_text, 200, "root-a", "a");
        let later_root = cursor(false, &older.primary_text, 200, "root-b", "a");
        let later_location = cursor(false, &older.primary_text, 200, "root-b", "b");
        let missing = cursor(true, "", 0, "root-a", "a");

        assert!(default_gallery_cursor_follows(&newest, &older));
        assert!(default_gallery_cursor_follows(&older, &lower_number));
        assert!(default_gallery_cursor_follows(&lower_number, &later_root));
        assert!(default_gallery_cursor_follows(&later_root, &later_location));
        assert!(default_gallery_cursor_follows(&later_location, &missing));
        assert!(!default_gallery_cursor_follows(&older, &newest));
        assert!(!default_gallery_cursor_follows(&newest, &newest));
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
