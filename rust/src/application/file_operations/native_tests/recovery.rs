use std::io::Write;
use std::sync::atomic::AtomicBool;

use crate::domain::exact_duplicates::{ContentFingerprint, ExactAnalysisRun, FingerprintCandidate};
use crate::ports::exact_duplicates::ExactDuplicateRepository;

use super::*;

pub(super) fn publish_fingerprint(
    catalog: &mut SqliteCatalog,
    root: &IncrementalCatalogRoot,
    location: &AssetLocationView,
    bytes: &[u8],
) {
    let run_id = format!("content-{}", location.source_generation);
    catalog
        .begin_exact_analysis(&ExactAnalysisRun {
            run_id: run_id.clone(),
            started_unix_ms: 1,
        })
        .unwrap();
    assert!(
        catalog
            .publish_fingerprint(
                &run_id,
                &FingerprintCandidate {
                    location: location.clone(),
                    root: root.clone(),
                },
                &ContentFingerprint {
                    digest: blake3::hash(bytes).to_hex().to_string(),
                    byte_length: bytes.len() as u64,
                },
                &AtomicBool::new(false),
            )
            .unwrap()
    );
}

#[test]
#[ignore = "requires guest-local Windows Sandbox fixtures"]
fn file_operation_sandbox_lost_receipt_requires_original_content_before_rebinding() {
    assert!(
        std::env::var("USERNAME").is_ok_and(|name| name.eq_ignore_ascii_case("WDAGUtilityAccount"))
    );
    let base = std::path::PathBuf::from(std::env::var_os("LOCALAPPDATA").unwrap()).join("AmeR8");
    fs::create_dir_all(&base).unwrap();
    let fixture = tempfile::Builder::new()
        .prefix("content-recovery-")
        .tempdir_in(base)
        .unwrap();
    let source_root = fixture.path().join("source");
    let target = source_root.join("organized");
    fs::create_dir_all(&target).unwrap();
    for name in ["unchanged.png", "edited.png", "unproven.png"] {
        RgbImage::from_pixel(16, 12, Rgb([32, 128, 224]))
            .save(source_root.join(name))
            .unwrap();
    }
    let storage = StoragePaths {
        catalog_path: fixture.path().join("catalog.sqlite3"),
        preview_root: fixture.path().join("previews"),
        preview_budget_bytes: 1024 * 1024,
        settings_path: fixture.path().join("settings.sqlite3"),
    };
    scan_library::run_scan_with_storage(
        ScanRequest {
            scan_id: "content-recovery".into(),
            root_path: source_root.to_string_lossy().into_owned(),
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
    let mut journal = SqliteFileOperationJournal::open(storage.catalog_path.clone()).unwrap();
    let root = journal.publication_roots().unwrap().remove(0);
    let mut catalog = SqliteCatalog::open(storage.catalog_path.clone()).unwrap();
    for name in ["unchanged.png", "edited.png", "unproven.png"] {
        let location = catalog
            .load_incremental_location_by_relative_path(&root.root_id, name)
            .unwrap()
            .unwrap();
        let original = fs::read(&location.absolute_path).unwrap();
        if name != "unproven.png" {
            publish_fingerprint(&mut catalog, &root, &location, &original);
        }
        let selected = catalog
            .prepare_organization_selection(&[selection(&location)])
            .unwrap();
        catalog
            .set_organization_favorite(name, &selected, true)
            .unwrap();
        let authorized = authorization(
            &engine,
            &root,
            &location,
            name,
            FileAction::Move,
            &target,
            name,
        );
        let record = journal
            .authorize(std::slice::from_ref(&authorized))
            .unwrap()
            .remove(0);
        journal
            .advance(
                record.id,
                record.checkpoint.version,
                crate::domain::file_operation_journal::FileOperationEvent::Started(
                    authorized.request.destination.clone(),
                ),
                2,
            )
            .unwrap();
        // The native move completes, but its receipt is never durably recorded.
        let moved = engine
            .execute(&authorized.request, &control)
            .unwrap()
            .destination
            .unwrap();
        let mut expected_bytes = original;
        if name == "edited.png" {
            let modified = fs::metadata(&moved.state.absolute_path)
                .unwrap()
                .modified()
                .unwrap();
            expected_bytes[0] ^= 1;
            let mut file = fs::OpenOptions::new()
                .write(true)
                .open(&moved.state.absolute_path)
                .unwrap();
            file.write_all(&expected_bytes).unwrap();
            file.set_times(fs::FileTimes::new().set_modified(modified))
                .unwrap();
            file.sync_all().unwrap();
            drop(file);
            let edited = engine.inspect_source(&moved.state.absolute_path).unwrap();
            assert_eq!(edited.state.file_identity, moved.state.file_identity);
            assert_eq!(edited.state.file_size, moved.state.file_size);
            assert_eq!(edited.state.modified_unix_ms, moved.state.modified_unix_ms);
            assert_ne!(edited.state.source_revision, moved.state.source_revision);
        }
        drop(journal);
        journal = SqliteFileOperationJournal::open(storage.catalog_path.clone()).unwrap();
        let recovered = FileOperationExecutor::new(&mut journal, &engine, &control, &|| 3)
            .reconcile(record.id)
            .unwrap();
        if name == "unchanged.png" {
            assert_eq!(recovered.checkpoint.phase, Phase::NativeCompleted);
            assert_eq!(
                publish_file_operation(&mut journal, &engine, record.id, 4)
                    .unwrap()
                    .checkpoint
                    .phase,
                Phase::Published
            );
            let current = catalog
                .load_incremental_location_by_relative_path(
                    &root.root_id,
                    &format!("organized/{name}"),
                )
                .unwrap()
                .unwrap();
            publish_fingerprint(&mut catalog, &root, &current, &expected_bytes);
        } else {
            assert_eq!(recovered.checkpoint.phase, Phase::Uncertain);
            assert!(recovered.checkpoint.receipt.is_none());
            assert!(publish_file_operation(&mut journal, &engine, record.id, 4).is_err());
            assert!(
                catalog
                    .load_incremental_location_by_relative_path(
                        &root.root_id,
                        &format!("organized/{name}")
                    )
                    .unwrap()
                    .is_none()
            );
        }
        assert!(!source_root.join(name).exists());
        assert_eq!(
            fs::read(&moved.state.absolute_path).unwrap(),
            expected_bytes
        );
        assert!(
            FileOperationExecutor::new(&mut journal, &engine, &control, &|| 5)
                .execute(record.id)
                .is_err()
        );
    }
}
