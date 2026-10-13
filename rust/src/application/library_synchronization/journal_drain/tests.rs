use std::fs;
use std::path::Path;

use image::{ImageFormat, Rgb, RgbImage};
use tempfile::tempdir;

use super::*;
use crate::application::StoragePaths;
use crate::application::scan_library::run_scan_with_storage;
use crate::domain::{
    LibraryChangeIntent, LibraryChangeIntentKind, LibraryChangeOrigin, LibraryChangeScope,
    ScanRequest,
};
use crate::ports::LibraryChangeQueue;

fn write_image(path: &Path, color: [u8; 3]) {
    RgbImage::from_pixel(24, 24, Rgb(color))
        .save_with_format(path, ImageFormat::Png)
        .unwrap();
}

#[test]
fn paired_directory_rename_preserves_assets_in_one_catalog_publication() {
    let source = tempdir().unwrap();
    let derived = tempdir().unwrap();
    fs::create_dir(source.path().join("old")).unwrap();
    for index in 0..20_u8 {
        write_image(
            &source.path().join(format!("old/{index:02}.png")),
            [index, 40, 60],
        );
    }
    let storage = StoragePaths {
        catalog_path: derived.path().join("catalog/ame.sqlite3"),
        preview_root: derived.path().join("previews"),
        preview_budget_bytes: 64 * 1024 * 1024,
        settings_path: derived.path().join("settings.sqlite3"),
    };
    run_scan_with_storage(
        ScanRequest {
            scan_id: "paired-directory-rename".to_owned(),
            root_path: source.path().to_string_lossy().into_owned(),
            max_items: None,
            max_entries: None,
            preview_edge: 64,
        },
        |_| true,
        storage.clone(),
    )
    .unwrap();
    let mut catalog = SqliteCatalog::open(&storage.catalog_path).unwrap();
    let root = catalog.load_incremental_catalog_roots().unwrap().remove(0);
    let before = catalog
        .load_incremental_locations_in_subtree(&root.root_id, "old", 128)
        .unwrap();
    fs::rename(source.path().join("old"), source.path().join("new")).unwrap();
    let intent = LibraryChangeIntent {
        root_id: root.root_id.clone(),
        root_generation: root.root_generation,
        kind: LibraryChangeIntentKind::RenameCandidate,
        scope: LibraryChangeScope::Subtree,
        relative_path: "new".to_owned(),
        previous_relative_path: Some("old".to_owned()),
        origin: LibraryChangeOrigin::StartupCatchUp,
        first_observed_unix_ms: 2_000,
        most_recent_observed_unix_ms: 2_000,
        first_sequence: 1,
        most_recent_sequence: 1,
        coalesced_observation_count: 1,
    };
    let policy = LibraryChangeQueuePolicy {
        debounce_millis: 0,
        ..Default::default()
    };
    catalog
        .enqueue_library_change_intents(&[intent], 2_000, policy)
        .unwrap();
    let report = process_ready(
        &mut catalog,
        &root.root_id,
        root.root_generation,
        2_000,
        policy,
        AuthoritativeRecoveryPolicy::default(),
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(report.completed_count, 1);
    let after = catalog
        .load_incremental_locations_in_subtree(&root.root_id, "new", 128)
        .unwrap();
    assert_eq!(after.len(), 20);
    assert!(
        catalog
            .load_incremental_locations_in_subtree(&root.root_id, "old", 128)
            .unwrap()
            .is_empty()
    );
    for previous in before {
        let path = previous.relative_path.replacen("old/", "new/", 1);
        let current = after
            .iter()
            .find(|location| location.relative_path == path)
            .unwrap();
        assert_eq!(current.asset_id, previous.asset_id);
        assert_eq!(current.file_identity, previous.file_identity);
    }
    drop(catalog);
    SqliteCatalog::open(&storage.catalog_path).expect("full reopen after atomic directory rename");
}

#[test]
fn journal_directory_work_reconciles_membership_and_preserves_live_work() {
    let source = tempdir().unwrap();
    let derived = tempdir().unwrap();
    let album = source.path().join("album");
    fs::create_dir(&album).unwrap();
    write_image(&album.join("old.png"), [20, 40, 60]);
    let storage = StoragePaths {
        catalog_path: derived.path().join("catalog/ame.sqlite3"),
        preview_root: derived.path().join("previews"),
        preview_budget_bytes: 64 * 1024 * 1024,
        settings_path: derived.path().join("settings.sqlite3"),
    };
    run_scan_with_storage(
        ScanRequest {
            scan_id: "journal-scope-fixture".to_owned(),
            root_path: source.path().to_string_lossy().into_owned(),
            max_items: None,
            max_entries: None,
            preview_edge: 128,
        },
        |_| true,
        storage.clone(),
    )
    .unwrap();
    let mut catalog = SqliteCatalog::open(storage.catalog_path.clone()).unwrap();
    let root = catalog.load_incremental_catalog_roots().unwrap().remove(0);
    fs::remove_file(album.join("old.png")).unwrap();
    write_image(&album.join("new.png"), [60, 40, 20]);
    let policy = LibraryChangeQueuePolicy {
        debounce_millis: 0,
        ..LibraryChangeQueuePolicy::default()
    };
    let mut journal = LibraryChangeIntent {
        root_id: root.root_id.clone(),
        root_generation: root.root_generation,
        kind: LibraryChangeIntentKind::Reconcile,
        scope: LibraryChangeScope::Subtree,
        relative_path: "album".to_owned(),
        previous_relative_path: None,
        origin: LibraryChangeOrigin::StartupCatchUp,
        first_observed_unix_ms: 2_000,
        most_recent_observed_unix_ms: 2_000,
        first_sequence: 1,
        most_recent_sequence: 1,
        coalesced_observation_count: 1,
    };
    let mut live = journal.clone();
    live.origin = LibraryChangeOrigin::LiveNotification;
    live.relative_path = "other".to_owned();
    catalog
        .enqueue_library_change_intents(&[journal.clone(), live], 2_000, policy)
        .unwrap();
    let cancelled = AtomicBool::new(true);
    assert_eq!(
        process_ready(
            &mut catalog,
            &root.root_id,
            root.root_generation,
            2_000,
            policy,
            AuthoritativeRecoveryPolicy::default(),
            &cancelled
        )
        .unwrap()
        .leased_count,
        0
    );
    cancelled.store(false, Ordering::Release);
    let report = process_ready(
        &mut catalog,
        &root.root_id,
        root.root_generation,
        2_000,
        policy,
        AuthoritativeRecoveryPolicy::default(),
        &cancelled,
    )
    .unwrap();
    assert_eq!(report.completed_count, 1);
    assert_eq!(report.applied_mutation_count, 2);
    assert!(
        catalog
            .load_incremental_location_by_relative_path(&root.root_id, "album/old.png")
            .unwrap()
            .is_none()
    );
    assert!(
        catalog
            .load_incremental_location_by_relative_path(&root.root_id, "album/new.png")
            .unwrap()
            .is_some()
    );
    assert!(
        catalog
            .has_ready_live_authoritative_library_change(
                &root.root_id,
                root.root_generation,
                2_000,
                policy
            )
            .unwrap()
    );

    journal.relative_path = "new-directory".to_owned();
    catalog
        .enqueue_library_change_intents(&[journal], 3_000, policy)
        .unwrap();
    let lease = catalog
        .lease_journal_authoritative_library_change(
            &root.root_id,
            root.root_generation,
            3_000,
            policy,
        )
        .unwrap()
        .unwrap();
    assert!(
        catalog
            .lease_journal_authoritative_library_change(
                &root.root_id,
                root.root_generation,
                lease.lease_expires_unix_ms + 1,
                policy
            )
            .unwrap()
            .is_none()
    );
    let recovered = catalog
        .lease_journal_authoritative_library_change(
            &root.root_id,
            root.root_generation,
            lease.lease_expires_unix_ms
                + 1
                + i64::try_from(policy.retry_initial_delay_millis).unwrap(),
            policy,
        )
        .unwrap()
        .unwrap();
    assert!(recovered.lease_generation > lease.lease_generation);
    assert!(
        catalog
            .has_ready_live_authoritative_library_change(
                &root.root_id,
                root.root_generation,
                3_000,
                policy
            )
            .unwrap()
    );
    drop(catalog);
    let reopened = SqliteCatalog::open(storage.catalog_path).unwrap();
    assert!(
        reopened
            .load_incremental_location_by_relative_path(&root.root_id, "album/new.png")
            .unwrap()
            .is_some()
    );
}
