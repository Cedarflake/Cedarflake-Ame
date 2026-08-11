use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use crate::adapters::{
    LocalPreviewStore, PREVIEW_ALGORITHM_ID, PREVIEW_ALGORITHM_VERSION,
    PREVIEW_ORIENTATION_CONTRACT, SqliteCatalog, is_ame_preview_cache_entry,
    is_managed_preview_cleanup_entry,
};
use crate::domain::ScanError;
use crate::ports::CatalogRepository;

use super::{StoragePaths, acquire_preview_reclamation};

const RECLAMATION_TARGET_NUMERATOR: u64 = 4;
const RECLAMATION_TARGET_DENOMINATOR: u64 = 5;
const RECLAMATION_BATCH: u32 = 256;
const MAX_RECLAMATION_PASSES: u32 = 16;
const MAX_UNINDEXED_RECLAMATION_ENTRIES: usize = 65_536;

pub(crate) fn reclaim_preview_capacity(
    storage: &StoragePaths,
    preview_store: &LocalPreviewStore,
    protected_location_ids: &[String],
    required_bytes: u64,
) -> Result<u64, ScanError> {
    let _exclusive_access = acquire_preview_reclamation()?;
    let low_watermark = storage
        .preview_budget_bytes
        .saturating_mul(RECLAMATION_TARGET_NUMERATOR)
        / RECLAMATION_TARGET_DENOMINATOR;
    let reservation_target = storage.preview_budget_bytes.saturating_sub(required_bytes);
    let target = low_watermark.min(reservation_target);
    let starting_bytes = preview_store.used_bytes();
    if starting_bytes <= target {
        return Ok(0);
    }

    let mut catalog = SqliteCatalog::open(storage.catalog_path.clone())?;
    remove_interrupted_and_unreferenced(&storage.preview_root, target, preview_store, &catalog)?;
    for _ in 0..MAX_RECLAMATION_PASSES {
        if preview_store.used_bytes() <= target {
            break;
        }
        let candidates = catalog.load_preview_reclamation_candidates(
            protected_location_ids,
            PREVIEW_ALGORITHM_ID,
            PREVIEW_ALGORITHM_VERSION,
            PREVIEW_ORIENTATION_CONTRACT,
            &format!(
                "{}{}",
                storage.preview_root.to_string_lossy(),
                std::path::MAIN_SEPARATOR,
            ),
            RECLAMATION_BATCH,
        )?;
        if candidates.is_empty() {
            break;
        }
        let mut removed_in_pass = 0_u64;
        for candidate in candidates {
            if preview_store.used_bytes() <= target {
                break;
            }
            let path = Path::new(&candidate.path);
            if path.parent() != Some(storage.preview_root.as_path())
                || !is_managed_preview_cleanup_entry(path)
            {
                continue;
            }
            let byte_size = path.metadata().map_or(0, |metadata| metadata.len());
            match fs::remove_file(path) {
                Ok(()) => {}
                Err(error) if error.kind() == ErrorKind::NotFound => {}
                Err(_) => continue,
            }
            if catalog.remove_reclaimed_preview(&candidate)? {
                removed_in_pass = removed_in_pass.saturating_add(byte_size);
            }
            preview_store.release(byte_size);
        }
        if removed_in_pass == 0 {
            break;
        }
    }
    Ok(starting_bytes.saturating_sub(preview_store.used_bytes()))
}

fn remove_interrupted_and_unreferenced(
    preview_root: &Path,
    target: u64,
    preview_store: &LocalPreviewStore,
    catalog: &SqliteCatalog,
) -> Result<(), ScanError> {
    if !preview_root.exists() {
        return Ok(());
    }
    let entries = fs::read_dir(preview_root).map_err(|error| {
        ScanError::new(
            "preview_reclamation_directory_unavailable",
            format!("Could not inspect preview storage for reclamation: {error}"),
        )
    })?;
    for entry in entries.take(MAX_UNINDEXED_RECLAMATION_ENTRIES) {
        if preview_store.used_bytes() <= target {
            break;
        }
        let Ok(entry) = entry else {
            continue;
        };
        let path = entry.path();
        if !is_ame_preview_cache_entry(&path) {
            continue;
        }
        let is_temporary = path.extension().and_then(|extension| extension.to_str()) == Some("tmp");
        let is_unreferenced =
            !is_temporary && !catalog.is_preview_artifact_path_indexed(&path.to_string_lossy())?;
        if !is_temporary && !is_unreferenced {
            continue;
        }
        let byte_size = path.metadata().map_or(0, |metadata| metadata.len());
        if fs::remove_file(path).is_ok() {
            preview_store.release(byte_size);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use tempfile::tempdir;

    use crate::adapters::PREVIEW_CACHE_VERSION;
    use crate::domain::{AssetLocationView, PreviewArtifact, PreviewStatus, ScanRequest};

    use super::*;

    #[test]
    fn reclamation_preserves_protected_demand_and_durable_dimensions() {
        let _test_lock = crate::application::PREVIEW_LIFECYCLE_TEST_LOCK
            .lock()
            .expect("preview lifecycle test lock");
        let directory = tempdir().expect("temporary directory");
        let preview_root = directory.path().join("previews");
        fs::create_dir_all(&preview_root).expect("preview root");
        let protected_path = managed_artifact_path(&preview_root, 'a');
        let reclaimable_path = managed_artifact_path(&preview_root, 'b');
        fs::write(&protected_path, vec![1_u8; 100]).expect("protected artifact");
        fs::write(&reclaimable_path, vec![2_u8; 100]).expect("reclaimable artifact");
        let storage = StoragePaths {
            catalog_path: directory.path().join("catalog").join("ame.sqlite3"),
            preview_root: preview_root.clone(),
            preview_budget_bytes: 200,
            settings_path: directory.path().join("settings.sqlite3"),
        };
        let mut catalog = SqliteCatalog::open(storage.catalog_path.clone()).expect("catalog");
        publish_artifact(
            &mut catalog,
            "protected",
            &protected_path,
            "reclaim-protected-location",
        );
        publish_artifact(
            &mut catalog,
            "reclaimable",
            &reclaimable_path,
            "reclaim-unprotected-location",
        );
        drop(catalog);
        let store = LocalPreviewStore::new(preview_root, 200).expect("preview store");

        let removed = reclaim_preview_capacity(
            &storage,
            &store,
            &["reclaim-protected-location".to_owned()],
            75,
        )
        .expect("automatic reclamation");

        assert_eq!(removed, 100);
        assert_eq!(store.used_bytes(), 100);
        assert!(protected_path.exists());
        assert!(!reclaimable_path.exists());
        let catalog = SqliteCatalog::open(storage.catalog_path).expect("reopen catalog");
        let protected = catalog
            .load_active_location("reclaim-protected-location")
            .expect("protected location query")
            .expect("protected location");
        let reclaimed = catalog
            .load_active_location("reclaim-unprotected-location")
            .expect("reclaimed location query")
            .expect("reclaimed location");
        assert!(matches!(protected.preview_status, PreviewStatus::Ready));
        assert!(matches!(reclaimed.preview_status, PreviewStatus::Pending));
        assert_eq!((reclaimed.width, reclaimed.height), (4_032, 3_024));
    }

    #[test]
    fn reclamation_counts_and_removes_legacy_previews_under_pressure() {
        let _test_lock = crate::application::PREVIEW_LIFECYCLE_TEST_LOCK
            .lock()
            .expect("preview lifecycle test lock");
        let directory = tempdir().expect("temporary directory");
        let preview_root = directory.path().join("previews");
        fs::create_dir_all(&preview_root).expect("preview root");
        let legacy_paths = (0_u32..300)
            .map(|index| preview_root.join(format!("{index:064x}.jpg")))
            .collect::<Vec<_>>();
        for path in &legacy_paths {
            fs::write(path, [1_u8]).expect("legacy preview");
        }
        let storage = StoragePaths {
            catalog_path: directory.path().join("catalog").join("ame.sqlite3"),
            preview_root: preview_root.clone(),
            preview_budget_bytes: 300,
            settings_path: directory.path().join("settings.sqlite3"),
        };
        SqliteCatalog::open(storage.catalog_path.clone()).expect("catalog");
        let store = LocalPreviewStore::new(preview_root, 300).expect("preview store");

        let removed =
            reclaim_preview_capacity(&storage, &store, &[], 300).expect("legacy reclamation");

        assert_eq!(removed, 300);
        assert_eq!(store.used_bytes(), 0);
        assert!(legacy_paths.iter().all(|path| !path.exists()));
    }

    fn managed_artifact_path(root: &Path, hash_character: char) -> PathBuf {
        root.join(format!(
            "{PREVIEW_CACHE_VERSION}-{}.jpg",
            hash_character.to_string().repeat(64),
        ))
    }

    fn publish_artifact(catalog: &mut SqliteCatalog, suffix: &str, path: &Path, location_id: &str) {
        let scan_id = format!("reclaim-{suffix}-scan");
        let root_id = format!("reclaim-{suffix}-root");
        let root_path = format!("C:\\ReclaimSource\\{suffix}");
        let request = ScanRequest {
            scan_id: scan_id.clone(),
            root_path: root_path.clone(),
            max_items: None,
            max_entries: None,
            preview_edge: 256,
        };
        catalog
            .begin_scan(&request, &root_id, &root_path)
            .expect("begin fixture scan");
        let location = AssetLocationView {
            asset_id: format!("reclaim-{suffix}-asset"),
            location_id: location_id.to_owned(),
            root_id: root_id.clone(),
            absolute_path: format!("{root_path}\\one.png"),
            display_path: format!("{root_path}\\one.png"),
            relative_path: "one.png".to_owned(),
            preview_path: path.to_string_lossy().into_owned(),
            file_size: 100,
            created_unix_ms: Some(10),
            modified_unix_ms: 20,
            file_identity: None,
            width: 4_032,
            height: 3_024,
            preview_status: PreviewStatus::Ready,
            preview_issue_code: None,
            preview_issue_message: None,
            metadata_engine_id: "fixture".to_owned(),
            metadata_engine_version: "1".to_owned(),
            capture_time: None,
        };
        catalog
            .stage_location(&scan_id, &root_id, &location)
            .expect("stage fixture location");
        catalog
            .publish_scan(&scan_id, &root_id, 1, 0)
            .expect("publish fixture scan");
        let artifact = PreviewArtifact {
            artifact_key: format!("reclaim-{suffix}-artifact"),
            algorithm_id: PREVIEW_ALGORITHM_ID.to_owned(),
            algorithm_version: PREVIEW_ALGORITHM_VERSION,
            orientation_contract: PREVIEW_ORIENTATION_CONTRACT.to_owned(),
            size_bucket: 256,
            path: location.preview_path.clone(),
            byte_size: 100,
            encoded_width: 256,
            encoded_height: 192,
            width: location.width,
            height: location.height,
        };
        catalog
            .update_active_preview(&location, Some(&artifact))
            .expect("publish fixture artifact");
    }
}
