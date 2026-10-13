use std::collections::BTreeMap;

use super::*;

pub(super) fn paths(fixture: &InventoryFixture) -> Vec<String> {
    bindings(fixture).into_keys().collect()
}

fn bindings(fixture: &InventoryFixture) -> BTreeMap<String, String> {
    let connection =
        rusqlite::Connection::open(fixture.catalog.catalog_path()).expect("read isolated catalog");
    let mut statement = connection
        .prepare(
            "SELECT directory.relative_path, directory.identity_value
             FROM scan_directory_identities AS directory
             JOIN library_roots AS root ON root.active_scan_id = directory.scan_id
             WHERE root.id = ?1 ORDER BY directory.relative_path",
        )
        .expect("published directory bindings");
    statement
        .query_map([&fixture.root_id], |row| Ok((row.get(0)?, row.get(1)?)))
        .expect("load bindings")
        .collect::<Result<_, _>>()
        .expect("valid bindings")
}

fn reopen(fixture: &mut InventoryFixture) {
    let path = fixture.catalog.catalog_path().to_path_buf();
    fixture.catalog = SqliteCatalog::open(path).expect("full catalog reopen");
}

#[test]
fn completed_inventory_retains_renamed_and_empty_directories_across_reopen() {
    let mut fixture = InventoryFixture::new(&["old/retained.png"]);
    let previous = bindings(&fixture);
    let source_bytes =
        fs::read(fixture.source.path().join("old/retained.png")).expect("original fixture bytes");
    fs::rename(
        fixture.source.path().join("old"),
        fixture.source.path().join("new"),
    )
    .expect("fixture directory rename");
    fs::create_dir(fixture.source.path().join("empty")).expect("empty fixture directory");

    let report = fixture.run_inventory(1, &AtomicBool::new(false));
    assert!(report.is_complete);
    reopen(&mut fixture);
    let current = bindings(&fixture);
    assert_eq!(
        current.keys().map(String::as_str).collect::<Vec<_>>(),
        ["", "empty", "new"]
    );
    assert_eq!(current.get(""), previous.get(""));
    assert_eq!(current.get("new"), previous.get("old"));
    assert_eq!(
        fs::read(fixture.source.path().join("new/retained.png")).expect("source after inventory"),
        source_bytes
    );
}

#[test]
fn subtree_snapshot_does_not_replace_similarly_prefixed_siblings() {
    let mut fixture = InventoryFixture::new(&[
        "album/old/retained.png",
        "album-extra/retained.png",
        "album_2/retained.png",
    ]);
    let previous = bindings(&fixture);
    fs::rename(
        fixture.source.path().join("album/old"),
        fixture.source.path().join("album/new"),
    )
    .expect("rename inside subtree");
    let mut request = fixture.request();
    request.scope = MetadataInventoryScope::Subtree {
        relative_path: "album".to_owned(),
    };
    let mut source = LocalMetadataInventory::new(&fixture.root_path, &request.scope)
        .expect("subtree metadata source");
    let report = run_metadata_inventory(
        &mut fixture.catalog,
        &mut source,
        &request,
        2_000,
        1,
        queue_policy(),
        &AtomicBool::new(false),
    )
    .expect("complete scoped inventory");
    assert!(report.is_complete);
    reopen(&mut fixture);
    let current = bindings(&fixture);
    for path in ["", "album", "album-extra", "album_2"] {
        assert_eq!(current.get(path), previous.get(path), "retained {path}");
    }
    assert!(!current.contains_key("album/old"));
    assert_eq!(current.get("album/new"), previous.get("album/old"));
}

#[test]
fn cancellation_preserves_the_previous_directory_snapshot() {
    let mut fixture = InventoryFixture::new(&["old/retained.png"]);
    let previous = bindings(&fixture);
    fs::rename(
        fixture.source.path().join("old"),
        fixture.source.path().join("new"),
    )
    .expect("fixture directory rename");

    let report = fixture.run_inventory(1, &AtomicBool::new(true));
    assert!(report.is_cancelled);
    reopen(&mut fixture);
    assert_eq!(bindings(&fixture), previous);
}

#[test]
fn failed_directory_publication_rolls_back_the_snapshot_and_completion() {
    let mut fixture = InventoryFixture::new(&["old/retained.png"]);
    let previous = bindings(&fixture);
    fs::rename(
        fixture.source.path().join("old"),
        fixture.source.path().join("new"),
    )
    .expect("fixture directory rename");
    let connection = rusqlite::Connection::open(fixture.catalog.catalog_path())
        .expect("isolated failure fixture");
    connection
        .execute_batch(
            "CREATE TRIGGER reject_inventory_directory BEFORE INSERT ON scan_directory_identities
             BEGIN SELECT RAISE(ABORT, 'controlled directory publication failure'); END;",
        )
        .expect("fail after snapshot deletion");
    let request = fixture.request();
    let mut source =
        LocalMetadataInventory::new(&fixture.root_path, &request.scope).expect("metadata source");
    assert!(
        run_metadata_inventory(
            &mut fixture.catalog,
            &mut source,
            &request,
            2_000,
            1,
            queue_policy(),
            &AtomicBool::new(false),
        )
        .is_err()
    );
    connection
        .execute_batch("DROP TRIGGER reject_inventory_directory")
        .expect("retire controlled failure");
    drop(connection);
    reopen(&mut fixture);
    assert_eq!(bindings(&fixture), previous);
    let run = fixture
        .catalog
        .load_metadata_inventory_run(&request.run_id)
        .expect("run status")
        .expect("retained failed run");
    assert_ne!(run.status, MetadataInventoryRunStatus::Completed);
}
