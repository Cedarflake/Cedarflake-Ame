use std::fs;

use image::{Rgb, RgbImage};
use rusqlite::Connection;
use sha2::{Digest, Sha256};

use crate::adapters::{SqliteCatalog, SqliteFileOperationJournal, WindowsFileOperations};
use crate::application::{StoragePaths, scan_library};
use crate::domain::file_operation_journal::{AuthorizedFileOperation, FileOperationCatalogBinding};
use crate::domain::file_operations::{FileOperationDestination, FileOperationRequest};
use crate::domain::organization::OrganizationSelection;
use crate::domain::{AssetLocationView, IncrementalCatalogRoot, ScanRequest};
use crate::ports::file_operation_catalog::FileOperationCatalog;
use crate::ports::file_operation_output::{FileOperationOutput, FileOperationOutputReader};
use crate::ports::organization::OrganizationRepository;
use crate::ports::{CatalogRepository, IncrementalCatalogRepository};

use super::*;
mod exports;

#[test]
#[ignore = "requires guest-local Windows Sandbox fixtures"]
fn file_operation_sandbox_recovers_native_move_and_publishes_copy_without_replay() {
    assert!(
        std::env::var("USERNAME").is_ok_and(|name| name.eq_ignore_ascii_case("WDAGUtilityAccount"))
    );
    let base = std::path::PathBuf::from(std::env::var_os("LOCALAPPDATA").unwrap()).join("AmeR8");
    fs::create_dir_all(&base).unwrap();
    let fixture = tempfile::Builder::new()
        .prefix("catalog-operation-")
        .tempdir_in(base)
        .unwrap();
    let root_path = fixture.path().join("source");
    let destination = root_path.join("organized");
    let exports = fixture.path().join("exports");
    fs::create_dir_all(&destination).unwrap();
    fs::create_dir(&exports).unwrap();
    let source = root_path.join("图片.png");
    RgbImage::from_pixel(16, 12, Rgb([32, 128, 224]))
        .save(&source)
        .unwrap();
    let original_bytes = fs::read(&source).unwrap();
    let sentinel = fixture.path().join("sibling.txt");
    fs::write(&sentinel, b"outside operation selection").unwrap();
    let storage = StoragePaths {
        catalog_path: fixture.path().join("catalog.sqlite3"),
        preview_root: fixture.path().join("previews"),
        preview_budget_bytes: 1024 * 1024,
        settings_path: fixture.path().join("settings.sqlite3"),
    };
    scan_library::run_scan_with_storage(
        ScanRequest {
            scan_id: "native-operation".into(),
            root_path: root_path.to_string_lossy().into_owned(),
            max_items: None,
            max_entries: None,
            preview_edge: 128,
        },
        |_| true,
        storage.clone(),
    )
    .unwrap();
    let engine = WindowsFileOperations::new(None);
    let control = FileOperationControl::default();
    let now = || 10;
    let mut journal = SqliteFileOperationJournal::open(storage.catalog_path.clone()).unwrap();
    let root = journal.publication_roots().unwrap().remove(0);
    let mut catalog = SqliteCatalog::open(storage.catalog_path.clone()).unwrap();
    let original = catalog
        .load_incremental_location_by_relative_path(&root.root_id, "图片.png")
        .unwrap()
        .unwrap();
    let prepared = catalog
        .prepare_organization_selection(&[selection(&original)])
        .unwrap();
    catalog
        .set_organization_favorite("before-native-move", &prepared, true)
        .unwrap();
    let authorized = authorization(
        &engine,
        &root,
        &original,
        "native-move",
        FileAction::Move,
        &destination,
        "移动.png",
    );
    let pending = journal.authorize(&[authorized]).unwrap().remove(0);
    let faults = Connection::open(&storage.catalog_path).unwrap();
    faults.execute_batch(
        "CREATE TRIGGER interrupt_native_receipt BEFORE UPDATE OF phase ON file_operation_items
         WHEN NEW.phase = 'native_completed' BEGIN SELECT RAISE(ABORT, 'injected receipt loss'); END;",
    ).unwrap();
    assert!(
        FileOperationExecutor::new(&mut journal, &engine, &control, &now)
            .execute(pending.id)
            .is_err()
    );
    let moved_path = destination.join("移动.png");
    assert!(!source.exists());
    assert_eq!(fs::read(&moved_path).unwrap(), original_bytes);
    assert_eq!(
        journal.load(pending.id).unwrap().checkpoint.phase,
        Phase::Executing
    );
    faults
        .execute_batch("DROP TRIGGER interrupt_native_receipt;")
        .unwrap();
    drop(journal);
    let mut journal = SqliteFileOperationJournal::open(storage.catalog_path.clone()).unwrap();
    let recovered = FileOperationExecutor::new(&mut journal, &engine, &control, &now)
        .reconcile(pending.id)
        .unwrap();
    assert_eq!(recovered.checkpoint.phase, Phase::NativeCompleted);
    assert!(
        FileOperationExecutor::new(&mut journal, &engine, &control, &now)
            .execute(pending.id)
            .is_err()
    );
    let output = recovered
        .checkpoint
        .receipt
        .as_ref()
        .unwrap()
        .destination
        .as_ref()
        .unwrap();
    {
        let held = engine
            .hold_output(output, Some((&root, "organized/移动.png")))
            .unwrap();
        assert_eq!(held.metadata().unwrap().inspection.width, 16);
        assert!(
            fs::OpenOptions::new()
                .write(true)
                .open(&moved_path)
                .is_err()
        );
        held.revalidate().unwrap();
    }
    faults
        .execute_batch(
            "CREATE TRIGGER interrupt_publication BEFORE UPDATE OF phase ON file_operation_items
         WHEN NEW.phase = 'published' BEGIN SELECT RAISE(ABORT, 'injected catalog failure'); END;",
        )
        .unwrap();
    assert!(publish_file_operation(&mut journal, &engine, pending.id, 11).is_err());
    assert!(
        catalog
            .load_active_location(&original.location_id)
            .unwrap()
            .is_some()
    );
    assert_eq!(fs::read(&moved_path).unwrap(), original_bytes);
    faults
        .execute_batch("DROP TRIGGER interrupt_publication;")
        .unwrap();
    let published = publish_file_operation(&mut journal, &engine, pending.id, 12).unwrap();
    assert_eq!(published.checkpoint.phase, Phase::Published);
    assert!(
        catalog
            .load_active_location(&original.location_id)
            .unwrap()
            .is_none()
    );
    let moved = catalog
        .load_incremental_location_by_relative_path(&root.root_id, "organized/移动.png")
        .unwrap()
        .unwrap();
    assert_eq!(moved.asset_id, original.asset_id);
    assert!(
        catalog
            .inspect_organization_selection(&[selection(&moved)])
            .unwrap()
            .decisions[0]
            .favorite
    );
    fs::write(exports.join("移动.png"), b"occupied destination").unwrap();
    let selected = FileOperationSelectionSession::for_catalog(
        storage.catalog_path.clone(),
        crate::domain::file_operation_selection::FileOperationSelectionScope::Locations,
    )
    .unwrap();
    selected.append_locations(vec![selection(&moved)]).unwrap();
    assert_eq!(selected.prepare().unwrap().file_count, 1);
    let task = selected
        .execute(
            "native-copy".into(),
            crate::domain::file_operation_selection::FileOperationCommand {
                organization_rule:
                    crate::domain::file_operation_exports::FileOrganizationRule::Direct,
                action: FileAction::Copy,
                destination_directory: Some(exports.to_string_lossy().into_owned()),
                new_name: None,
            },
            None,
        )
        .unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
    loop {
        use crate::domain::file_operation_task::FileOperationTaskPhase;
        let progress = task.snapshot();
        if !matches!(
            progress.phase,
            FileOperationTaskPhase::Starting | FileOperationTaskPhase::Running
        ) {
            assert_eq!(
                progress.phase,
                FileOperationTaskPhase::Finished,
                "{:?}",
                progress.failure
            );
            assert_eq!(progress.published_items, 1);
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "native task did not finish"
        );
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    task.close().unwrap();
    selected.close().unwrap();
    let copied = journal
        .batch_page("native-copy", None, 1)
        .unwrap()
        .remove(0);
    assert_eq!(copied.checkpoint.phase, Phase::Published);
    assert_eq!(
        copied.checkpoint.destination.as_ref().unwrap().name,
        "移动(1).png"
    );
    assert_eq!(
        fs::read(exports.join("移动(1).png")).unwrap(),
        original_bytes
    );
    assert_eq!(
        fs::read(exports.join("移动.png")).unwrap(),
        b"occupied destination"
    );
    assert_eq!(fs::read(&moved_path).unwrap(), original_bytes);
    let rename = authorization(
        &engine,
        &root,
        &moved,
        "native-case",
        FileAction::Rename,
        &destination,
        "移动.PNG",
    );
    let rename = journal.authorize(&[rename]).unwrap().remove(0);
    faults.execute_batch(
        "CREATE TRIGGER interrupt_native_receipt BEFORE UPDATE OF phase ON file_operation_items
         WHEN NEW.phase = 'native_completed' BEGIN SELECT RAISE(ABORT, 'injected case rename receipt loss'); END;",
    ).unwrap();
    assert!(
        FileOperationExecutor::new(&mut journal, &engine, &control, &now)
            .execute(rename.id)
            .is_err()
    );
    assert!(
        fs::read_dir(&destination)
            .unwrap()
            .any(|entry| entry.unwrap().file_name() == "移动.PNG")
    );
    faults
        .execute_batch("DROP TRIGGER interrupt_native_receipt;")
        .unwrap();
    let recovered = FileOperationExecutor::new(&mut journal, &engine, &control, &now)
        .reconcile(rename.id)
        .unwrap();
    assert_eq!(recovered.checkpoint.phase, Phase::NativeCompleted);
    publish_file_operation(&mut journal, &engine, rename.id, 14).unwrap();
    let renamed_path = destination.join("移动.PNG");
    let hard_link = destination.join("other-link.png");
    fs::hard_link(&renamed_path, &hard_link).unwrap();
    let collision = FileOperationRequest {
        action: FileAction::Rename,
        source: engine
            .inspect_source(renamed_path.to_str().unwrap())
            .unwrap(),
        destination: Some(FileOperationDestination {
            directory: engine
                .inspect_directory(destination.to_str().unwrap())
                .unwrap(),
            name: "other-link.png".into(),
        }),
    };
    assert_eq!(
        engine.execute(&collision, &control).unwrap_err().kind,
        Kind::Collision
    );
    assert_eq!(fs::read(&renamed_path).unwrap(), original_bytes);
    assert_eq!(fs::read(&hard_link).unwrap(), original_bytes);
    let extended_copy = exports.join("移动(1).png").canonicalize().unwrap();
    let recycled = engine
        .execute(
            &FileOperationRequest {
                action: FileAction::Recycle,
                source: engine
                    .inspect_source(extended_copy.to_str().unwrap())
                    .unwrap(),
                destination: None,
            },
            &control,
        )
        .unwrap();
    assert_eq!(
        recycled.recovery,
        crate::domain::file_operations::FileOperationRecovery::SystemRecycleBin
    );
    assert!(!extended_copy.exists());
    assert_eq!(fs::read(&renamed_path).unwrap(), original_bytes);
    assert_eq!(journal.publication_roots().unwrap().len(), 1);
    assert_eq!(fs::read(&sentinel).unwrap(), b"outside operation selection");
    println!(
        "native move receipt recovery, held output, catalog rollback, favorite continuity, selected numbered copy, case rename recovery, hard-link collision, extended-path recycle: PASS; bytes={}, sha256={:x}",
        original_bytes.len(),
        Sha256::digest(&original_bytes)
    );
}

fn selection(location: &AssetLocationView) -> OrganizationSelection {
    OrganizationSelection {
        asset_id: location.asset_id.clone(),
        location_id: location.location_id.clone(),
        source_generation: location.source_generation,
    }
}

fn authorization(
    engine: &WindowsFileOperations,
    root: &IncrementalCatalogRoot,
    source: &AssetLocationView,
    batch: &str,
    action: FileAction,
    directory: &std::path::Path,
    name: &str,
) -> AuthorizedFileOperation {
    AuthorizedFileOperation {
        batch_id: batch.into(),
        ordinal: 0,
        authorized_unix_ms: 1,
        catalog: FileOperationCatalogBinding {
            asset_id: source.asset_id.clone(),
            location_id: source.location_id.clone(),
            source_generation: source.source_generation,
            root_id: root.root_id.clone(),
            root_generation: root.root_generation.value(),
        },
        request: FileOperationRequest {
            action,
            source: engine.inspect_source(&source.absolute_path).unwrap(),
            destination: Some(FileOperationDestination {
                directory: engine
                    .inspect_directory(directory.to_str().unwrap())
                    .unwrap(),
                name: name.into(),
            }),
        },
    }
}
