use crate::ports::{CatalogRepository, IncrementalCatalogRepository};

use super::*;

fn fixture() -> (tempfile::TempDir, SqliteCatalog) {
    let directory = tempfile::tempdir().unwrap();
    let mut catalog = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
    super::super::tests::publish_gallery_fixture(
        &mut catalog,
        "scan-a",
        "root-a",
        "C:\\Fixture",
        &[("a", None, 1), ("b", None, 2)],
    );
    super::super::tests::publish_gallery_fixture(
        &mut catalog,
        "scan-b",
        "root-b",
        "D:\\Fixture",
        &[("c", None, 3)],
    );
    catalog.connection.execute_batch(
        "INSERT INTO library_root_publication_namespaces
           (root_id, root_generation, identity_scheme, identity_value, authority_kind,
            established_catalog_revision, established_unix_ms, updated_unix_ms)
         SELECT root_id, generation, 'windows-file-id-128-v1',
            '0000000000000001:00000000000000000000000000000001', 'foreground_scan', 0, 0, 0
         FROM library_change_root_state;
         UPDATE asset_locations SET source_revision_token = 'windows-file-change-time-100ns-v1:0000000000000001';"
    ).unwrap();
    catalog
        .begin_exact_analysis(&ExactAnalysisRun {
            run_id: "run".to_owned(),
            started_unix_ms: 1,
        })
        .unwrap();
    (directory, catalog)
}

fn candidate(catalog: &SqliteCatalog, id: &str) -> FingerprintCandidate {
    let location = catalog.load_active_location(id).unwrap().unwrap();
    let root = catalog
        .load_incremental_catalog_root(&location.root_id)
        .unwrap()
        .unwrap();
    FingerprintCandidate { location, root }
}

fn publish(catalog: &mut SqliteCatalog, id: &str, digest: &str) -> bool {
    let candidate = candidate(catalog, id);
    catalog
        .publish_fingerprint(
            "run",
            &candidate,
            &ContentFingerprint {
                digest: digest.to_owned(),
                byte_length: 20,
            },
            &AtomicBool::new(false),
        )
        .unwrap()
}

#[test]
fn multi_volume_copies_group_by_bytes_and_reuse_evidence_after_reopen() {
    let (directory, mut catalog) = fixture();
    assert_eq!(catalog.load_fingerprint_candidates("").unwrap().len(), 3);
    assert!(publish(&mut catalog, "a", &"a".repeat(64)));
    assert!(publish(&mut catalog, "b", &"b".repeat(64)));
    assert!(publish(&mut catalog, "c", &"a".repeat(64)));
    let groups = catalog.load_duplicate_groups(None).unwrap();
    assert_eq!(groups.groups.len(), 1);
    assert_eq!(groups.groups[0].location_count, 2);
    assert_eq!(groups.groups[0].physical_file_count, 2);
    assert_eq!(groups.groups[0].representative_location_id, "a");
    drop(catalog);
    let catalog = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
    assert!(catalog.load_fingerprint_candidates("").unwrap().is_empty());
    assert_eq!(
        catalog
            .load_duplicate_locations(&"a".repeat(64), 20, "")
            .unwrap()
            .len(),
        2
    );
}

#[test]
fn stale_generation_revision_path_and_cancelled_work_cannot_publish() {
    let (_directory, mut catalog) = fixture();
    let original = candidate(&catalog, "a");
    let fingerprint = ContentFingerprint {
        digest: "a".repeat(64),
        byte_length: 20,
    };
    assert!(
        !catalog
            .publish_fingerprint("run", &original, &fingerprint, &AtomicBool::new(true))
            .unwrap()
    );
    for change in [
        "source_generation = source_generation + 100",
        "source_revision_token = 'windows-file-change-time-100ns-v1:0000000000000002'",
        "absolute_path = 'C:\\Replaced.png'",
    ] {
        let transaction = catalog.connection.unchecked_transaction().unwrap();
        transaction
            .execute_batch(&format!(
                "UPDATE asset_locations SET {change} WHERE location_id = 'a'"
            ))
            .unwrap();
        assert!(!publication::publish(&transaction, "run", &original, &fingerprint).unwrap());
        transaction.rollback().unwrap();
    }
    assert!(publish(&mut catalog, "a", &"a".repeat(64)));
}

#[test]
fn hardlink_paths_remain_distinct_and_a_generation_change_hides_old_matches() {
    let (_directory, mut catalog) = fixture();
    catalog.connection.execute_batch(
        "UPDATE asset_locations SET source_generation = (SELECT source_generation FROM asset_locations WHERE location_id = 'a'),
           modified_unix_ms = 1,
           file_identity_scheme = 'windows-file-id-128-v1',
           file_identity_value = '0000000000000001:00000000000000000000000000000002'
         WHERE location_id IN ('a', 'b');"
    ).unwrap();
    assert!(publish(&mut catalog, "a", &"a".repeat(64)));
    let group = catalog
        .load_duplicate_groups(None)
        .unwrap()
        .groups
        .remove(0);
    assert_eq!((group.location_count, group.physical_file_count), (2, 1));
    assert!(
        catalog
            .load_fingerprint_candidates("")
            .unwrap()
            .iter()
            .all(|candidate| candidate.location.location_id == "c")
    );
    catalog.connection.execute("UPDATE asset_locations SET source_generation = source_generation + 100 WHERE location_id IN ('a', 'b')", []).unwrap();
    assert!(
        catalog
            .load_duplicate_groups(None)
            .unwrap()
            .groups
            .is_empty()
    );
}

#[test]
fn a_unique_size_is_not_read_and_removed_members_leave_no_group() {
    let (_directory, mut catalog) = fixture();
    catalog
        .connection
        .execute(
            "UPDATE asset_locations SET file_size = 21 WHERE location_id = 'c'",
            [],
        )
        .unwrap();
    assert_eq!(catalog.load_fingerprint_candidates("").unwrap().len(), 2);
    publish(&mut catalog, "a", &"a".repeat(64));
    publish(&mut catalog, "b", &"a".repeat(64));
    catalog
        .connection
        .execute("DELETE FROM asset_locations WHERE location_id = 'b'", [])
        .unwrap();
    assert!(
        catalog
            .load_duplicate_groups(None)
            .unwrap()
            .groups
            .is_empty()
    );
}

#[test]
fn group_and_location_pages_do_not_repeat_or_omit_boundary_members() {
    let (_directory, mut catalog) = fixture();
    let ids = (0..140)
        .map(|index| format!("page-{index:04}"))
        .collect::<Vec<_>>();
    let locations = ids
        .iter()
        .map(|id| (id.as_str(), None, 1))
        .collect::<Vec<_>>();
    super::super::tests::publish_gallery_fixture(
        &mut catalog,
        "scan-page",
        "root-page",
        "E:\\Fixture",
        &locations,
    );
    catalog
        .connection
        .execute_batch(
            "INSERT INTO library_root_publication_namespaces
           (root_id, root_generation, identity_scheme, identity_value, authority_kind,
            established_catalog_revision, established_unix_ms, updated_unix_ms)
         SELECT root_id, generation, 'windows-file-id-128-v1',
            '0000000000000003:00000000000000000000000000000001', 'foreground_scan', 0, 0, 0
         FROM library_change_root_state WHERE root_id = 'root-page';",
        )
        .unwrap();
    for (index, id) in ids.iter().enumerate() {
        assert!(publish(&mut catalog, id, &format!("{:064x}", index / 2)));
    }
    let first = catalog.load_duplicate_groups(None).unwrap();
    assert_eq!(first.groups.len(), 64);
    let second = catalog.load_duplicate_groups(first.next.as_ref()).unwrap();
    assert_eq!(second.groups.len(), 6);
    assert!(second.next.is_none());
    let all = first
        .groups
        .iter()
        .chain(&second.groups)
        .map(|group| group.digest.clone())
        .collect::<std::collections::HashSet<_>>();
    assert_eq!(all.len(), 70);
    let group = &first.groups[0];
    let members = catalog
        .load_duplicate_locations(&group.digest, 20, "")
        .unwrap();
    assert_eq!(members.len(), 2);
    let after = catalog
        .load_duplicate_locations(&group.digest, 20, &members[0].location.location_id)
        .unwrap();
    assert_eq!(after.len(), 1);
    assert_eq!(
        after[0].location.location_id,
        members[1].location.location_id
    );
}
