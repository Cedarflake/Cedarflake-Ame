use super::*;

use crate::application::scan_library::run_scan_with_storage;

#[test]
fn preview_of_a_renamed_directory_preserves_the_asset_before_journal_catchup() {
    let fixture = preview_fixture("preview-directory-rename");
    let source_root = fixture
        .source_path
        .parent()
        .expect("source root")
        .to_path_buf();
    let old_directory = source_root.join("old");
    fs::create_dir(&old_directory).expect("generated old directory");
    fs::rename(&fixture.source_path, old_directory.join("source.png"))
        .expect("place generated file in a child directory");
    run_scan_with_storage(
        ScanRequest {
            scan_id: "preview-directory-baseline".to_owned(),
            root_path: source_root.to_string_lossy().into_owned(),
            max_items: Some(8),
            max_entries: Some(16),
            preview_edge: 256,
        },
        |_| true,
        fixture.storage.clone(),
    )
    .expect("published real directory baseline");
    let catalog = SqliteCatalog::open(fixture.storage.catalog_path.clone()).expect("catalog");
    let root = catalog
        .load_incremental_catalog_root(&fixture.request.expected_root_id)
        .expect("root query")
        .expect("registered fixture root");
    let before = catalog
        .load_incremental_location_by_relative_path(&root.root_id, "old/source.png")
        .expect("old location")
        .expect("published old location");
    drop(catalog);
    let ready = materialize_preview_with_storage(
        preview_request_for(&before, 256, false),
        fixture.storage.clone(),
    )
    .expect("cached old preview");
    let bytes = fs::read(old_directory.join("source.png")).expect("generated source bytes");
    fs::rename(&old_directory, source_root.join("new")).expect("rename generated directory");
    crate::adapters::reset_source_enumeration_instrumentation(&root.root_path);
    let error = materialize_preview_with_storage(
        preview_request_for(&ready, 256, false),
        fixture.storage.clone(),
    )
    .expect_err("old preview request retires after relocation");
    assert_eq!(error.code, "preview_request_superseded");
    let catalog = SqliteCatalog::open(fixture.storage.catalog_path.clone()).expect("full reopen");
    assert!(
        catalog
            .load_incremental_location_by_relative_path(&root.root_id, "old/source.png")
            .expect("old membership query")
            .is_none()
    );
    let current = catalog
        .load_incremental_location_by_relative_path(&root.root_id, "new/source.png")
        .expect("new membership query")
        .expect("published relocated source");
    assert_eq!(current.asset_id, ready.asset_id);
    assert_eq!(current.file_identity, ready.file_identity);
    assert_eq!(
        crate::adapters::source_entry_read_count(&root.root_path),
        0,
        "identity reconstruction must not enumerate the directory"
    );
    drop(catalog);
    let preview = materialize_preview_with_storage(
        preview_request_for(&current, 256, false),
        fixture.storage.clone(),
    )
    .expect("preview at the new location");
    assert_eq!(preview.preview_status, PreviewStatus::Ready);
    assert_eq!(
        fs::read(source_root.join("new/source.png")).expect("source after reconciliation"),
        bytes
    );
}

#[test]
fn relocated_source_admission_revalidates_the_previous_location_generation() {
    let (fixture, ready) = ready_source_fixture("relocation-admission");
    let mut catalog = SqliteCatalog::open(fixture.storage.catalog_path.clone()).expect("catalog");
    let root = catalog
        .load_incremental_catalog_root(&ready.root_id)
        .expect("root query")
        .expect("root");
    let mut request = preview_request_for(&ready, 256, false);
    request.expected_source_generation += 1;
    let intent = LibraryChangeIntent {
        root_id: root.root_id.clone(),
        root_generation: root.root_generation,
        kind: LibraryChangeIntentKind::RenameCandidate,
        scope: LibraryChangeScope::Path,
        relative_path: "new.png".to_owned(),
        previous_relative_path: Some(ready.relative_path.clone()),
        origin: LibraryChangeOrigin::ConsistencyAudit,
        first_observed_unix_ms: 100,
        most_recent_observed_unix_ms: 100,
        first_sequence: 0,
        most_recent_sequence: 0,
        coalesced_observation_count: 1,
    };
    let admission = catalog
        .admit_source_reconciliation(
            &request,
            &intent,
            100,
            LibraryChangeQueuePolicy {
                debounce_millis: 0,
                ..Default::default()
            },
        )
        .expect("stale request admission");
    assert!(matches!(
        admission,
        SourceReconciliationAdmission::RequestSuperseded
    ));
    let connection = Connection::open(&fixture.storage.catalog_path).expect("catalog state");
    let count: i64 = connection
        .query_row("SELECT COUNT(*) FROM library_change_queue", [], |row| {
            row.get(0)
        })
        .expect("queue count");
    assert_eq!(count, 0);
    assert_eq!(
        catalog
            .load_active_location(&ready.location_id)
            .expect("retained source")
            .expect("active source")
            .asset_id,
        ready.asset_id
    );
}
