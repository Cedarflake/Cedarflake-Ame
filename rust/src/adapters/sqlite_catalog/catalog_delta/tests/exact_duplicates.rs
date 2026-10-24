use std::sync::atomic::AtomicBool;

use crate::domain::exact_duplicates::{ContentFingerprint, ExactAnalysisRun, FingerprintCandidate};
use crate::ports::exact_duplicates::ExactDuplicateRepository;

use super::*;

mod organization;

#[test]
fn incremental_content_changes_rehash_only_changed_sources_and_reject_late_evidence() {
    let (_directory, mut catalog) = fixture();
    publish_fingerprint(&mut catalog, "location-a", 'a');
    publish_fingerprint(&mut catalog, "location-b", 'a');
    publish_fingerprint(&mut catalog, "location-c", 'b');
    assert_group_members(&catalog, 'a', &["location-a", "location-b"]);

    let before = candidate(&catalog, "location-b");
    let mut changed = before.location.clone();
    changed.modified_unix_ms += 1;
    changed.source_revision.as_mut().unwrap().value = "0000000000000002".to_owned();
    changed.source_generation = 0;
    apply_change(
        &mut catalog,
        Some(&before.location),
        Some(changed),
        IncrementalReconciliationOutcome::Modified,
    );
    assert!(
        catalog
            .load_duplicate_groups(None)
            .unwrap()
            .groups
            .is_empty()
    );
    assert_pending(&catalog, &["location-b"]);
    assert!(
        !catalog
            .publish_fingerprint("run", &before, &fingerprint('a'), &AtomicBool::new(false),)
            .unwrap()
    );
    publish_fingerprint(&mut catalog, "location-b", 'b');
    assert_group_members(&catalog, 'b', &["location-b", "location-c"]);

    let before = candidate(&catalog, "location-b").location;
    let mut replacement = before.clone();
    replacement.file_identity = Some(identity(9));
    replacement.source_generation = 0;
    apply_change(
        &mut catalog,
        Some(&before),
        Some(replacement),
        IncrementalReconciliationOutcome::Replaced,
    );
    assert!(
        catalog
            .load_duplicate_groups(None)
            .unwrap()
            .groups
            .is_empty()
    );
    assert_pending(&catalog, &["location-b"]);
    publish_fingerprint(&mut catalog, "location-b", 'a');
    assert_group_members(&catalog, 'a', &["location-a", "location-b"]);
    assert_pending(&catalog, &[]);
}

#[test]
fn incremental_move_removal_and_addition_preserve_unchanged_evidence_across_reopen() {
    let (directory, mut catalog) = fixture();
    for id in ["location-a", "location-b", "location-c"] {
        publish_fingerprint(&mut catalog, id, 'a');
    }
    let old = candidate(&catalog, "location-b").location;
    let mut moved = old.clone();
    moved.location_id = "location-moved".to_owned();
    moved.relative_path = "nested/moved.jpg".to_owned();
    moved.absolute_path = "C:/source/nested/moved.jpg".to_owned();
    moved.display_path.clone_from(&moved.absolute_path);
    apply_change(
        &mut catalog,
        Some(&old),
        Some(moved),
        IncrementalReconciliationOutcome::RenamedOrMoved,
    );
    assert_eq!(
        candidate(&catalog, "location-moved")
            .location
            .source_generation,
        old.source_generation,
    );
    assert_pending(&catalog, &[]);
    assert_group_members(
        &catalog,
        'a',
        &["location-a", "location-c", "location-moved"],
    );

    let removed = candidate(&catalog, "location-c").location;
    apply_change(
        &mut catalog,
        Some(&removed),
        None,
        IncrementalReconciliationOutcome::Removed,
    );
    assert_group_members(&catalog, 'a', &["location-a", "location-moved"]);
    assert_pending(&catalog, &[]);

    let mut added = location("asset-d", "location-d", "root-a", "new.jpg");
    added.file_identity = Some(identity(4));
    added.source_generation = 0;
    apply_change(
        &mut catalog,
        None,
        Some(added),
        IncrementalReconciliationOutcome::Added,
    );
    assert_pending(&catalog, &["location-d"]);
    drop(catalog);
    let mut catalog = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
    assert_pending(&catalog, &["location-d"]);
    assert_group_members(&catalog, 'a', &["location-a", "location-moved"]);
    publish_fingerprint(&mut catalog, "location-d", 'a');
    assert_group_members(
        &catalog,
        'a',
        &["location-a", "location-d", "location-moved"],
    );
    assert_pending(&catalog, &[]);
}

#[test]
fn hardlink_changes_invalidate_aliases_and_reuse_one_new_fingerprint() {
    let (_directory, mut catalog) = fixture();
    publish_fingerprint(&mut catalog, "location-a", 'a');
    publish_fingerprint(&mut catalog, "location-b", 'a');
    publish_fingerprint(&mut catalog, "location-c", 'b');
    let before = candidate(&catalog, "location-a");
    let mut alias = location("asset-alias", "location-alias", "root-a", "alias.jpg");
    alias.file_identity = Some(identity(1));
    alias.source_generation = 0;
    apply_change(
        &mut catalog,
        None,
        Some(alias),
        IncrementalReconciliationOutcome::Added,
    );
    assert_pending(&catalog, &["location-a", "location-alias"]);
    let primary = candidate(&catalog, "location-a");
    let alias = candidate(&catalog, "location-alias");
    assert_ne!(
        primary.location.source_generation,
        before.location.source_generation,
    );
    assert_eq!(
        primary.location.source_generation,
        alias.location.source_generation,
    );
    assert!(
        !catalog
            .publish_fingerprint("run", &before, &fingerprint('a'), &AtomicBool::new(false))
            .unwrap()
    );
    assert!(
        catalog
            .load_duplicate_groups(None)
            .unwrap()
            .groups
            .is_empty()
    );
    publish_fingerprint(&mut catalog, "location-alias", 'a');
    assert_pending(&catalog, &[]);
    assert_group_members(
        &catalog,
        'a',
        &["location-a", "location-alias", "location-b"],
    );
    assert_eq!(
        catalog.load_duplicate_groups(None).unwrap().groups[0].physical_file_count,
        2
    );

    let old = candidate(&catalog, "location-alias").location;
    let mut changed = old.clone();
    changed.modified_unix_ms += 1;
    changed.source_revision.as_mut().unwrap().value = "0000000000000002".to_owned();
    changed.source_generation = 0;
    apply_change(
        &mut catalog,
        Some(&old),
        Some(changed),
        IncrementalReconciliationOutcome::Modified,
    );
    assert!(
        catalog
            .load_duplicate_groups(None)
            .unwrap()
            .groups
            .is_empty()
    );
    assert_pending(&catalog, &["location-a", "location-alias"]);
    publish_fingerprint(&mut catalog, "location-alias", 'b');
    assert_group_members(
        &catalog,
        'b',
        &["location-a", "location-alias", "location-c"],
    );
    assert_pending(&catalog, &[]);
    assert_eq!(
        catalog.load_duplicate_groups(None).unwrap().groups[0].physical_file_count,
        2
    );
}

fn fixture() -> (tempfile::TempDir, SqliteCatalog) {
    let directory = tempdir().unwrap();
    let mut catalog = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
    let locations = ['a', 'b', 'c']
        .into_iter()
        .enumerate()
        .map(|(index, suffix)| {
            let mut entry = location(
                &format!("asset-{suffix}"),
                &format!("location-{suffix}"),
                "root-a",
                &format!("{suffix}.jpg"),
            );
            entry.file_identity = Some(identity(index + 1));
            entry.source_generation = 0;
            entry
        })
        .collect::<Vec<_>>();
    seed_catalog(&mut catalog, "root-a", "C:/source", &locations);
    catalog
        .connection
        .execute_batch(
            "INSERT INTO library_root_publication_namespaces
            (root_id, root_generation, identity_scheme, identity_value, authority_kind,
             established_catalog_revision, established_unix_ms, updated_unix_ms)
         SELECT root_id, generation, 'windows-file-id-128-v1',
            '0000000000000001:000000000000000000000000000000ff', 'foreground_scan', 0, 0, 0
         FROM library_change_root_state;",
        )
        .unwrap();
    catalog
        .begin_exact_analysis(&ExactAnalysisRun {
            run_id: "run".to_owned(),
            started_unix_ms: 1,
        })
        .unwrap();
    (directory, catalog)
}

fn identity(value: usize) -> FileIdentityEvidence {
    FileIdentityEvidence {
        scheme: "windows-file-id-128-v1".to_owned(),
        value: format!("0000000000000001:{value:032x}"),
    }
}

fn candidate(catalog: &SqliteCatalog, id: &str) -> FingerprintCandidate {
    FingerprintCandidate {
        location: catalog.load_active_location(id).unwrap().unwrap(),
        root: catalog
            .load_incremental_catalog_root("root-a")
            .unwrap()
            .unwrap(),
    }
}

fn fingerprint(digest: char) -> ContentFingerprint {
    ContentFingerprint {
        digest: digest.to_string().repeat(64),
        byte_length: 10,
    }
}

fn publish_fingerprint(catalog: &mut SqliteCatalog, id: &str, digest: char) {
    let source = candidate(catalog, id);
    assert!(
        catalog
            .publish_fingerprint(
                "run",
                &source,
                &fingerprint(digest),
                &AtomicBool::new(false),
            )
            .unwrap(),
        "admit fingerprint for {id} at generation {}",
        source.location.source_generation,
    );
}

fn assert_pending(catalog: &SqliteCatalog, expected: &[&str]) {
    let pending = catalog.load_fingerprint_candidates("").unwrap();
    assert_eq!(
        pending
            .iter()
            .map(|value| value.location.location_id.as_str())
            .collect::<Vec<_>>(),
        expected
    );
}

fn assert_group_members(catalog: &SqliteCatalog, digest: char, expected: &[&str]) {
    let groups = catalog.load_duplicate_groups(None).unwrap();
    assert_eq!(groups.groups.len(), 1);
    assert_eq!(groups.groups[0].location_count, expected.len() as u64);
    let members = catalog
        .load_duplicate_locations(&digest.to_string().repeat(64), 10, "")
        .unwrap();
    assert_eq!(
        members
            .iter()
            .map(|value| value.location.location_id.as_str())
            .collect::<Vec<_>>(),
        expected
    );
}

fn apply_change(
    catalog: &mut SqliteCatalog,
    old: Option<&AssetLocationView>,
    next: Option<AssetLocationView>,
    outcome: IncrementalReconciliationOutcome,
) {
    let renamed = outcome == IncrementalReconciliationOutcome::RenamedOrMoved;
    let mut change = intent(
        "root-a",
        LibraryChangeIntentKind::Reconcile,
        &next.as_ref().or(old).unwrap().relative_path,
    );
    if renamed {
        change.kind = LibraryChangeIntentKind::RenameCandidate;
        change.previous_relative_path = Some(old.unwrap().relative_path.clone());
    }
    let leased = lease_change(catalog, change);
    let root = catalog
        .load_incremental_catalog_root("root-a")
        .unwrap()
        .unwrap();
    let disposition = match outcome {
        IncrementalReconciliationOutcome::Added | IncrementalReconciliationOutcome::Replaced => {
            DerivedEvidenceDisposition::NoReusableEvidence
        }
        IncrementalReconciliationOutcome::Modified => DerivedEvidenceDisposition::InvalidateDerived,
        IncrementalReconciliationOutcome::RenamedOrMoved => {
            DerivedEvidenceDisposition::RetainCompatible
        }
        IncrementalReconciliationOutcome::Removed => {
            DerivedEvidenceDisposition::RemoveFromCurrentProjection
        }
        _ => panic!("Unsupported test mutation"),
    };
    let remove = renamed || next.is_none();
    let retained = if renamed {
        let old = old.unwrap();
        Some(RetainedPreviewExpectation {
            location_id: old.location_id.clone(),
            preview_path: old.preview_path.clone(),
            preview_status: old.preview_status.clone(),
            preview_issue_code: old.preview_issue_code.clone(),
            preview_issue_message: old.preview_issue_message.clone(),
        })
    } else {
        None
    };
    let publication = catalog
        .publish_catalog_delta(
            &CatalogDeltaBatch {
                root_id: "root-a".to_owned(),
                root_generation: root.root_generation,
                expected_catalog_revision: root.catalog_revision,
                mutations: vec![CatalogDeltaMutation {
                    change_id: leased.change.id,
                    outcome,
                    evidence_disposition: disposition,
                    remove_location_ids: if remove {
                        vec![old.unwrap().location_id.clone()]
                    } else {
                        Vec::new()
                    },
                    upsert_location: next,
                    retained_preview_expectation: retained,
                }],
                terminal_media_evidence: Vec::new(),
                completions: vec![completion(&leased)],
            },
            2_000,
        )
        .unwrap();
    assert_eq!(publication.status, CatalogDeltaPublicationStatus::Applied);
}
