use crate::adapters::sqlite_catalog::tests::support::{
    load_default_snapshot, load_default_timeline, publish_gallery_fixture,
};

use tempfile::tempdir;

use crate::adapters::sqlite_catalog::*;

#[test]
fn unregistering_a_root_removes_only_catalog_state_and_preserves_source_bytes() {
    let directory = tempdir().expect("temporary directory");
    let source_directory = directory.path().join("source");
    std::fs::create_dir(&source_directory).expect("source directory");
    let source_file = source_directory.join("one.png");
    let source_bytes = b"irreplaceable source bytes";
    std::fs::write(&source_file, source_bytes).expect("source fixture");
    let path = directory.path().join("catalog.sqlite3");
    let mut catalog = SqliteCatalog::open(path).expect("catalog");
    let root_path = source_directory.to_string_lossy();
    publish_gallery_fixture(
        &mut catalog,
        "remove-scan",
        "remove-root",
        &root_path,
        &[("one", Some("2026-01-02T03:04:05.000000000"), 10)],
    );
    let revision_before = load_default_snapshot(&mut catalog, 10, None)
        .expect("snapshot before removal")
        .revision;

    assert!(
        catalog
            .unregister_root("remove-root")
            .expect("unregister root")
    );

    let snapshot = load_default_snapshot(&mut catalog, 10, None).expect("snapshot after removal");
    assert!(snapshot.roots.is_empty());
    assert!(snapshot.assets.is_empty());
    assert_eq!(snapshot.revision, revision_before + 1);
    assert_eq!(
        load_default_timeline(&mut catalog)
            .expect("timeline after removal")
            .total_items,
        0
    );
    assert_eq!(
        std::fs::read(&source_file).expect("source remains readable"),
        source_bytes
    );
    assert!(
        !catalog
            .unregister_root("remove-root")
            .expect("second unregister is idempotent")
    );
}
