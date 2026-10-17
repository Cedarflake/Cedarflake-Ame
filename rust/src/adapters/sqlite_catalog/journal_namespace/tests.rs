use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::domain::{LibraryRootGeneration, ScanCheckpoint, ScanRequest};
use crate::ports::CatalogRepository;

use super::*;

#[test]
fn identity_lookup_work_is_bounded_by_requested_identities() {
    for count in [64, 10_000] {
        let (_storage, mut catalog) = fixture();
        publish_root(&mut catalog, "root", "scan");
        seed(&catalog, "root", "scan", count);
        let requested: Vec<_> = (20_000..20_256).map(identity).collect();
        let operations = Arc::new(AtomicUsize::new(0));
        let observed = Arc::clone(&operations);
        catalog
            .connection
            .progress_handler(
                1,
                Some(move || observed.fetch_add(1, Ordering::Relaxed) >= 20_000),
            )
            .unwrap();
        let result = catalog.load_journal_identity_locations(&root("root"), &requested);
        catalog
            .connection
            .progress_handler(0, None::<fn() -> bool>)
            .unwrap();
        let operations = operations.load(Ordering::Relaxed);
        eprintln!("journal_namespace rows={count} requested=256 operations={operations}");
        assert!(
            result
                .expect("bounded identity lookup must not scan unrelated catalog rows")
                .is_empty()
        );
        assert!(operations > 0 && operations < 20_000);
    }
}

#[test]
fn identity_lookup_preserves_aliases_directories_and_root_generation() {
    let (_storage, mut catalog) = fixture();
    publish_root(&mut catalog, "root", "scan");
    publish_root(&mut catalog, "other", "other-scan");
    seed(&catalog, "root", "scan", 8);
    seed(&catalog, "other", "other-scan", 8);
    catalog
        .connection
        .execute(
            "UPDATE asset_locations SET file_identity_value = ?1
         WHERE scan_id = 'scan' AND location_id = 'location-2'",
            [identity(1).value],
        )
        .unwrap();
    let requested = [
        identity(1),
        identity(100_001),
        identity(1),
        identity(90_000),
    ];
    let found = catalog
        .load_journal_identity_locations(&root("root"), &requested)
        .unwrap();
    assert_eq!(
        found
            .iter()
            .map(|item| (item.relative_path.as_str(), item.is_directory))
            .collect::<Vec<_>>(),
        [
            ("图片-1.png", false),
            ("图片-2.png", false),
            ("目录-1", true)
        ]
    );
    assert_eq!(found[0].identity, identity(1));
    assert_eq!(found[1].identity, identity(1));
    assert_eq!(found[2].identity, identity(100_001));
    assert!(
        catalog
            .load_journal_identity_locations(&root("missing"), &requested)
            .unwrap()
            .is_empty()
    );
    let mut stale = root("root");
    stale.root_generation = LibraryRootGeneration::new(2).unwrap();
    assert!(
        catalog
            .load_journal_identity_locations(&stale, &requested)
            .unwrap()
            .is_empty()
    );
    catalog
        .connection
        .execute(
            "UPDATE library_change_root_state SET is_active = 0 WHERE root_id = 'root'",
            [],
        )
        .unwrap();
    assert!(
        catalog
            .load_journal_identity_locations(&root("root"), &requested)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn identity_lookup_keeps_input_and_output_bounds() {
    let (_storage, mut catalog) = fixture();
    publish_root(&mut catalog, "root", "scan");
    assert!(
        catalog
            .load_journal_identity_locations(&root("root"), &[])
            .unwrap()
            .is_empty()
    );
    let oversized: Vec<_> = (1..=257).map(identity).collect();
    assert_eq!(
        catalog
            .load_journal_identity_locations(&root("root"), &oversized)
            .unwrap_err()
            .code,
        "identity_journal_namespace_capacity"
    );
    seed(&catalog, "root", "scan", 4_097);
    catalog
        .connection
        .execute(
            "UPDATE asset_locations SET file_identity_value = ?1 WHERE scan_id = 'scan'",
            [identity(1).value],
        )
        .unwrap();
    assert_eq!(
        catalog
            .load_journal_identity_locations(&root("root"), &[identity(1)])
            .unwrap_err()
            .code,
        "identity_journal_namespace_capacity"
    );
}

fn fixture() -> (tempfile::TempDir, SqliteCatalog) {
    let storage = tempfile::tempdir().unwrap();
    let catalog = SqliteCatalog::open(storage.path().join("catalog.sqlite3")).unwrap();
    (storage, catalog)
}

fn publish_root(catalog: &mut SqliteCatalog, root_id: &str, scan_id: &str) {
    let request = ScanRequest {
        scan_id: scan_id.to_owned(),
        root_path: root_id.to_owned(),
        max_items: None,
        max_entries: None,
        preview_edge: 128,
    };
    catalog.begin_scan(&request, root_id, root_id).unwrap();
    catalog.claim_next_directory(scan_id).unwrap();
    catalog
        .complete_directory(scan_id, &ScanCheckpoint::default())
        .unwrap();
    catalog
        .prove_live_only_first_import_handoff_for_test(scan_id)
        .unwrap();
    catalog.publish_scan(scan_id, root_id, 0, 0).unwrap();
}

fn root(root_id: &str) -> IdentityJournalRoot {
    IdentityJournalRoot {
        root_id: root_id.to_owned(),
        root_generation: LibraryRootGeneration::initial(),
    }
}

fn identity(id: u64) -> FileIdentityEvidence {
    FileIdentityEvidence {
        scheme: "windows-file-id-128-v1".to_owned(),
        value: format!("0000000000000001:{id:032x}"),
    }
}

fn seed(catalog: &SqliteCatalog, root_id: &str, scan_id: &str, count: u32) {
    let transaction = catalog.connection.unchecked_transaction().unwrap();
    transaction
        .execute(
            "INSERT INTO assets(id, created_unix_ms) VALUES (?1, 1)",
            [scan_id],
        )
        .unwrap();
    transaction
        .execute(
            "WITH RECURSIVE sequence(value) AS (
           VALUES (1) UNION ALL SELECT value + 1 FROM sequence WHERE value < ?3
         )
         INSERT INTO asset_locations(
           scan_id, asset_id, location_id, root_id, absolute_path, relative_path, preview_path,
           file_size, modified_unix_ms, width, height, file_identity_scheme, file_identity_value,
           source_generation)
         SELECT ?2, ?2, 'location-' || value, ?1, ?1 || '/图片-' || value || '.png',
                '图片-' || value || '.png', '', 42, 1, 8, 8,
                'windows-file-id-128-v1', printf('0000000000000001:%032x', value), 1
         FROM sequence",
            params![root_id, scan_id, count],
        )
        .unwrap();
    transaction.execute(
        "WITH RECURSIVE sequence(value) AS (
           VALUES (1) UNION ALL SELECT value + 1 FROM sequence WHERE value < ?2
         )
         INSERT INTO scan_directory_identities(scan_id, relative_path, identity_scheme, identity_value)
         SELECT ?1, '目录-' || value, 'windows-file-id-128-v1',
                printf('0000000000000001:%032x', value + 100000)
         FROM sequence", params![scan_id, count],
    ).unwrap();
    transaction.commit().unwrap();
}
