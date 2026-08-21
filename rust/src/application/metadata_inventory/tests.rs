use std::fs;
use std::path::Path;
use std::sync::atomic::AtomicBool;

use image::{Rgb, RgbImage};
use tempfile::{TempDir, tempdir};

use crate::adapters::{FileDiscovery, LocalMetadataInventory, SqliteCatalog};
use crate::application::StoragePaths;
use crate::application::process_ready_library_changes;
use crate::application::scan_library::{run_scan_with_storage, stable_id};
use crate::domain::{
    LibraryChangeQueuePolicy, LibraryRootGeneration, MetadataInventoryEntry,
    MetadataInventoryEntryKind, MetadataInventoryPage, MetadataInventoryPlaceholderState,
    MetadataInventoryRunRequest, MetadataInventoryRunStatus, MetadataInventoryScope, ScanError,
    ScanRequest,
};
use crate::ports::{
    IncrementalCatalogRepository, LibraryChangeQueue, MetadataInventoryRepository,
    MetadataInventorySource,
};

use super::{retry_terminalization, run_metadata_inventory};

#[test]
fn closed_process_metadata_changes_converge_through_bounded_inventory_pages() {
    let mut fixture = InventoryFixture::new(&[
        "keep.png",
        "modify.png",
        "delete.png",
        "rename.png",
        "album/moved.png",
    ]);
    write_png(
        &fixture.source.path().join("modify.png"),
        4,
        3,
        [90, 80, 70],
    );
    fs::remove_file(fixture.source.path().join("delete.png")).expect("delete fixture");
    fs::rename(
        fixture.source.path().join("rename.png"),
        fixture.source.path().join("renamed.png"),
    )
    .expect("rename fixture");
    fs::rename(
        fixture.source.path().join("album"),
        fixture.source.path().join("moved-album"),
    )
    .expect("move directory fixture");
    write_png(
        &fixture.source.path().join("新增图片.png"),
        3,
        2,
        [20, 40, 60],
    );
    let long_name = format!("{}.png", "长路径".repeat(30));
    write_png(&fixture.source.path().join(&long_name), 2, 3, [60, 40, 20]);

    let report = fixture.run_inventory(2, &AtomicBool::new(false));

    assert!(report.is_complete);
    assert!(report.staged_entry_count >= 7);
    assert!(report.candidate_count >= 6);
    let incremental = process_ready_library_changes(
        &mut fixture.catalog,
        &fixture.root_id,
        LibraryRootGeneration::initial(),
        2_000,
        queue_policy(),
    )
    .expect("process inventory candidates");
    assert!(incremental.completed_count >= 6);
    assert!(fixture.location("keep.png").is_some());
    assert_eq!(
        fixture
            .location("modify.png")
            .map(|location| (location.width, location.height)),
        Some((4, 3))
    );
    assert!(fixture.location("delete.png").is_none());
    assert!(fixture.location("rename.png").is_none());
    assert!(fixture.location("renamed.png").is_some());
    assert!(fixture.location("album/moved.png").is_none());
    assert!(fixture.location("moved-album/moved.png").is_some());
    assert!(fixture.location("新增图片.png").is_some());
    assert!(fixture.location(&long_name).is_some());
}

#[test]
fn cancelled_inventory_keeps_absence_unproven_and_catalog_unchanged() {
    let mut fixture = InventoryFixture::new(&["retained.png"]);
    fs::remove_file(fixture.source.path().join("retained.png")).expect("delete fixture");
    let cancellation = AtomicBool::new(true);

    let report = fixture.run_inventory(1, &cancellation);

    assert!(report.is_cancelled);
    assert!(!report.is_complete);
    let run = fixture
        .catalog
        .load_metadata_inventory_run("inventory-1")
        .expect("load inventory")
        .expect("inventory run");
    assert_eq!(run.status, MetadataInventoryRunStatus::Cancelled);
    assert!(!run.enumeration_complete);
    assert!(!run.absence_authority);
    assert!(fixture.location("retained.png").is_some());
    let metrics = fixture
        .catalog
        .load_library_change_root_queue_metrics(
            &fixture.root_id,
            LibraryRootGeneration::initial(),
            2_000,
            queue_policy(),
        )
        .expect("queue metrics");
    assert_eq!(metrics.pending_count + metrics.retry_wait_count, 0);
}

#[test]
fn placeholder_evidence_is_staged_and_enqueued_without_media_inspection() {
    let mut fixture = InventoryFixture::new(&[]);
    let mut source = FixedInventorySource {
        page: Some(MetadataInventoryPage {
            page_index: 1,
            entries: vec![MetadataInventoryEntry {
                relative_path: "cloud-only.bin".to_owned(),
                kind: MetadataInventoryEntryKind::File,
                file_size: Some(123),
                modified_unix_ms: 1_500,
                file_identity: None,
                placeholder_state: MetadataInventoryPlaceholderState::Offline,
                is_reparse_point: false,
            }],
            cursor: Some("cloud-only.bin".to_owned()),
            is_complete: true,
        }),
    };
    let request = fixture.request();

    let report = run_metadata_inventory(
        &mut fixture.catalog,
        &mut source,
        &request,
        2_000,
        4,
        queue_policy(),
        &AtomicBool::new(false),
    )
    .expect("metadata-only placeholder inventory");

    assert!(report.is_complete);
    assert_eq!(report.candidate_count, 1);
    let metrics = fixture
        .catalog
        .load_library_change_root_queue_metrics(
            &fixture.root_id,
            LibraryRootGeneration::initial(),
            2_000,
            queue_policy(),
        )
        .expect("queue metrics");
    assert_eq!(metrics.pending_count, 1);
}

#[test]
fn source_failure_terminates_the_durable_inventory_run() {
    let mut fixture = InventoryFixture::new(&[]);
    let request = fixture.request();
    let mut source = FailingInventorySource;

    let error = run_metadata_inventory(
        &mut fixture.catalog,
        &mut source,
        &request,
        2_000,
        4,
        queue_policy(),
        &AtomicBool::new(false),
    )
    .expect_err("inventory source failure");
    let run = fixture
        .catalog
        .load_metadata_inventory_run(&request.run_id)
        .expect("load failed inventory")
        .expect("failed inventory run");

    assert_eq!(error.code, "metadata_inventory_fixture_failure");
    assert_eq!(run.status, MetadataInventoryRunStatus::Failed);
    assert_eq!(
        run.last_issue_code.as_deref(),
        Some("metadata_inventory_fixture_failure")
    );
    assert!(!run.absence_authority);
}

#[test]
fn terminalization_retries_one_transient_catalog_contention_failure() {
    let mut attempts = 0;

    retry_terminalization(|| {
        attempts += 1;
        if attempts == 1 {
            Err(ScanError::new(
                "catalog_database_busy",
                "fixture writer still owns the catalog",
            ))
        } else {
            Ok(())
        }
    })
    .expect("second terminalization attempt");

    assert_eq!(attempts, 2);
}

#[test]
fn newer_epoch_supersedes_an_orphaned_active_run_and_cleanup_stays_bounded() {
    let mut fixture = InventoryFixture::new(&[]);
    let first = fixture.request();
    fixture
        .catalog
        .begin_metadata_inventory(&first)
        .expect("begin first inventory");
    fixture
        .catalog
        .stage_metadata_inventory_page(
            &first.run_id,
            &MetadataInventoryPage {
                page_index: 1,
                entries: vec![metadata_entry("orphaned.txt")],
                cursor: Some("orphaned.txt".to_owned()),
                is_complete: false,
            },
            2_100,
        )
        .expect("stage orphaned entry");
    let second = MetadataInventoryRunRequest {
        run_id: "inventory-2".to_owned(),
        epoch: 2,
        started_unix_ms: 3_000,
        ..first.clone()
    };

    fixture
        .catalog
        .begin_metadata_inventory(&second)
        .expect("newer epoch");

    let superseded = fixture
        .catalog
        .load_metadata_inventory_run(&first.run_id)
        .expect("load first run")
        .expect("superseded run");
    assert_eq!(superseded.status, MetadataInventoryRunStatus::Superseded);
    let cleanup = fixture
        .catalog
        .cleanup_terminal_metadata_inventories(4_000, 1, 1)
        .expect("bounded cleanup");
    assert_eq!(cleanup.removed_entry_count, 1);
    assert_eq!(cleanup.removed_run_count, 1);
    assert!(!cleanup.has_more);
    assert!(
        fixture
            .catalog
            .load_metadata_inventory_run(&first.run_id)
            .expect("load cleaned run")
            .is_none()
    );
    assert_eq!(
        fixture
            .catalog
            .load_metadata_inventory_run(&second.run_id)
            .expect("load active run")
            .expect("active run")
            .status,
        MetadataInventoryRunStatus::Running
    );
}

#[test]
fn terminal_inventory_cleanup_deletes_staging_in_bounded_batches() {
    let mut fixture = InventoryFixture::new(&[]);
    let request = fixture.request();
    fixture
        .catalog
        .begin_metadata_inventory(&request)
        .expect("begin inventory");
    fixture
        .catalog
        .stage_metadata_inventory_page(
            &request.run_id,
            &MetadataInventoryPage {
                page_index: 1,
                entries: vec![
                    metadata_entry("a.txt"),
                    metadata_entry("b.txt"),
                    metadata_entry("c.txt"),
                ],
                cursor: Some("c.txt".to_owned()),
                is_complete: false,
            },
            2_100,
        )
        .expect("stage entries");
    fixture
        .catalog
        .terminate_metadata_inventory(
            &request.run_id,
            MetadataInventoryRunStatus::Failed,
            Some(("fixture_failure", "fixture failure")),
            2_200,
        )
        .expect("terminate inventory");

    let first = fixture
        .catalog
        .cleanup_terminal_metadata_inventories(3_000, 2, 1)
        .expect("first cleanup batch");
    assert_eq!(first.removed_entry_count, 2);
    assert_eq!(first.removed_run_count, 0);
    assert!(first.has_more);
    let second = fixture
        .catalog
        .cleanup_terminal_metadata_inventories(3_000, 2, 1)
        .expect("second cleanup batch");
    assert_eq!(second.removed_entry_count, 1);
    assert_eq!(second.removed_run_count, 1);
    assert!(!second.has_more);
}

#[cfg(windows)]
#[test]
fn one_previous_path_is_not_reused_for_multiple_hard_link_candidates() {
    let mut fixture = InventoryFixture::new(&["old.png"]);
    fs::rename(
        fixture.source.path().join("old.png"),
        fixture.source.path().join("new-a.png"),
    )
    .expect("rename fixture");
    fs::hard_link(
        fixture.source.path().join("new-a.png"),
        fixture.source.path().join("new-b.png"),
    )
    .expect("hard link fixture");

    let report = fixture.run_inventory(16, &AtomicBool::new(false));
    let incremental = process_ready_library_changes(
        &mut fixture.catalog,
        &fixture.root_id,
        LibraryRootGeneration::initial(),
        2_000,
        queue_policy(),
    )
    .expect("process hard link candidates");
    let first = fixture.location("new-a.png").expect("first hard link");
    let second = fixture.location("new-b.png").expect("second hard link");

    assert!(report.is_complete);
    assert_eq!(report.candidate_count, 2);
    assert_eq!(incremental.completed_count, 2);
    assert!(fixture.location("old.png").is_none());
    assert_eq!(first.asset_id, second.asset_id);
}

struct FixedInventorySource {
    page: Option<MetadataInventoryPage>,
}

impl MetadataInventorySource for FixedInventorySource {
    fn next_page(
        &mut self,
        _max_entries: u32,
        _cancelled: &AtomicBool,
    ) -> Result<MetadataInventoryPage, ScanError> {
        self.page.take().ok_or_else(|| {
            ScanError::new(
                "fixed_inventory_exhausted",
                "The fixed inventory page was already consumed",
            )
        })
    }
}

struct FailingInventorySource;

impl MetadataInventorySource for FailingInventorySource {
    fn next_page(
        &mut self,
        _max_entries: u32,
        _cancelled: &AtomicBool,
    ) -> Result<MetadataInventoryPage, ScanError> {
        Err(ScanError::new(
            "metadata_inventory_fixture_failure",
            "The metadata inventory fixture failed",
        ))
    }
}

struct InventoryFixture {
    source: TempDir,
    _storage: TempDir,
    catalog: SqliteCatalog,
    root_id: String,
    root_path: String,
}

impl InventoryFixture {
    fn new(relative_paths: &[&str]) -> Self {
        let source = tempdir().expect("source directory");
        for relative_path in relative_paths {
            let path = source.path().join(relative_path);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).expect("fixture parent");
            }
            write_png(&path, 2, 2, [10, 20, 30]);
        }
        let storage = tempdir().expect("storage directory");
        let storage_paths = StoragePaths {
            catalog_path: storage.path().join("catalog.sqlite3"),
            preview_root: storage.path().join("previews"),
            preview_budget_bytes: 64 * 1024 * 1024,
            settings_path: storage.path().join("settings.sqlite3"),
        };
        let root_path = FileDiscovery::new(&source.path().to_string_lossy())
            .expect("source discovery")
            .canonical_root()
            .expect("canonical root")
            .to_string_lossy()
            .into_owned();
        let root_id = stable_id("library-root-v1", &root_path);
        let scan_id = stable_id("metadata-inventory-initial-scan-v1", &root_path);
        run_scan_with_storage(
            ScanRequest {
                scan_id,
                root_path: root_path.clone(),
                max_items: None,
                max_entries: None,
                preview_edge: 128,
            },
            |_| true,
            storage_paths.clone(),
        )
        .expect("initial scan");
        let catalog = SqliteCatalog::open(storage_paths.catalog_path.clone()).expect("catalog");
        Self {
            source,
            _storage: storage,
            catalog,
            root_id,
            root_path,
        }
    }

    fn request(&self) -> MetadataInventoryRunRequest {
        MetadataInventoryRunRequest {
            run_id: "inventory-1".to_owned(),
            root_id: self.root_id.clone(),
            root_generation: LibraryRootGeneration::initial(),
            epoch: 1,
            scope: MetadataInventoryScope::Root,
            started_unix_ms: 2_000,
        }
    }

    fn run_inventory(
        &mut self,
        page_limit: u32,
        cancellation: &AtomicBool,
    ) -> crate::domain::MetadataInventoryReport {
        let request = self.request();
        let mut source = LocalMetadataInventory::new(&self.root_path, &request.scope)
            .expect("local metadata inventory");
        run_metadata_inventory(
            &mut self.catalog,
            &mut source,
            &request,
            2_000,
            page_limit,
            queue_policy(),
            cancellation,
        )
        .expect("run metadata inventory")
    }

    fn location(&self, relative_path: &str) -> Option<crate::domain::AssetLocationView> {
        self.catalog
            .load_incremental_location_by_relative_path(&self.root_id, relative_path)
            .expect("load location")
    }
}

fn queue_policy() -> LibraryChangeQueuePolicy {
    LibraryChangeQueuePolicy {
        debounce_millis: 0,
        max_lease_batch: 128,
        ..LibraryChangeQueuePolicy::default()
    }
}

fn metadata_entry(relative_path: &str) -> MetadataInventoryEntry {
    MetadataInventoryEntry {
        relative_path: relative_path.to_owned(),
        kind: MetadataInventoryEntryKind::File,
        file_size: Some(1),
        modified_unix_ms: 1,
        file_identity: None,
        placeholder_state: MetadataInventoryPlaceholderState::Available,
        is_reparse_point: false,
    }
}

fn write_png(path: &Path, width: u32, height: u32, color: [u8; 3]) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("PNG parent");
    }
    RgbImage::from_pixel(width, height, Rgb(color))
        .save(path)
        .expect("write PNG fixture");
}
