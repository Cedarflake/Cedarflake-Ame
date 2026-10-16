use crate::domain::exact_duplicate_review::{ExactDuplicatePreference, ExactDuplicateReviewKey};
use crate::domain::{GalleryDuplicateMode, GalleryQuery};
use crate::ports::exact_duplicate_review::ExactDuplicateReviewRepository;

use super::*;

fn key() -> ExactDuplicateReviewKey {
    ExactDuplicateReviewKey {
        digest: "a".repeat(64),
        byte_length: 20,
    }
}

fn preference(catalog: &SqliteCatalog, id: &str) -> ExactDuplicatePreference {
    let asset = candidate(catalog, id).location;
    ExactDuplicatePreference {
        location_id: asset.location_id,
        asset_id: asset.asset_id,
        source_generation: asset.source_generation,
    }
}

fn grouped_fixture() -> (tempfile::TempDir, SqliteCatalog) {
    let (directory, mut catalog) = fixture();
    for id in ["a", "b", "c"] {
        assert!(publish(&mut catalog, id, &key().digest));
    }
    let mut progress = ExactAnalysisProgress::running("run".to_owned());
    progress.phase = ExactAnalysisPhase::Completed;
    catalog.finish_exact_analysis(&progress).unwrap();
    (directory, catalog)
}

fn gallery_ids(catalog: &mut SqliteCatalog, root: Option<&str>) -> Vec<String> {
    let query = GalleryQuery {
        duplicate_mode: GalleryDuplicateMode::Merged,
        root_id: root.map(str::to_owned),
        ..GalleryQuery::default()
    };
    catalog
        .load_snapshot(10, &query, "query", None, None, None)
        .unwrap()
        .assets
        .into_iter()
        .map(|asset| asset.location_id)
        .collect()
}

#[test]
fn preferred_copy_survives_restart_and_respects_gallery_scope() {
    let (directory, mut catalog) = grouped_fixture();
    let preferred = preference(&catalog, "c");
    catalog
        .set_duplicate_preference(&key(), Some(&preferred))
        .unwrap();
    assert_eq!(gallery_ids(&mut catalog, None), ["c"]);
    assert_eq!(gallery_ids(&mut catalog, Some("root-a")), ["a"]);
    assert_eq!(
        catalog.load_review_groups(None, false).unwrap().groups[0]
            .representative
            .location_id,
        "c"
    );
    drop(catalog);
    let mut catalog = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
    assert_eq!(gallery_ids(&mut catalog, None), ["c"]);
    assert_eq!(
        catalog
            .load_review_members(&key(), "")
            .unwrap()
            .decision
            .preferred_location_id
            .as_deref(),
        Some("c")
    );
    catalog.set_duplicate_preference(&key(), None).unwrap();
    assert_eq!(gallery_ids(&mut catalog, None), ["a"]);
}

#[test]
fn ignore_moves_between_review_lists_without_changing_gallery_membership() {
    let (directory, mut catalog) = grouped_fixture();
    let preferred = preference(&catalog, "b");
    catalog
        .set_duplicate_preference(&key(), Some(&preferred))
        .unwrap();
    catalog.set_duplicate_ignored(&key(), true).unwrap();
    assert!(
        catalog
            .load_review_groups(None, false)
            .unwrap()
            .groups
            .is_empty()
    );
    assert_eq!(
        catalog.load_review_groups(None, true).unwrap().groups.len(),
        1
    );
    assert_eq!(gallery_ids(&mut catalog, None), ["b"]);
    drop(catalog);
    let mut catalog = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
    assert!(
        catalog
            .load_review_members(&key(), "")
            .unwrap()
            .decision
            .ignored
    );
    let restored = catalog.set_duplicate_ignored(&key(), false).unwrap();
    assert_eq!(restored.preferred_location_id.as_deref(), Some("b"));
    assert_eq!(
        catalog
            .load_review_groups(None, false)
            .unwrap()
            .groups
            .len(),
        1
    );
}

#[test]
fn changed_removed_and_replaced_preference_falls_back_without_erasing_choice() {
    let (_directory, mut catalog) = grouped_fixture();
    let preferred = preference(&catalog, "c");
    catalog
        .set_duplicate_preference(&key(), Some(&preferred))
        .unwrap();
    catalog.set_duplicate_ignored(&key(), true).unwrap();
    catalog.connection.execute("UPDATE asset_locations SET source_generation = source_generation + 100 WHERE location_id = 'c'", []).unwrap();
    assert_eq!(gallery_ids(&mut catalog, Some("root-a")), ["a"]);
    assert_eq!(
        catalog.load_review_groups(None, true).unwrap().groups[0]
            .representative
            .location_id,
        "a"
    );
    assert_eq!(
        catalog
            .set_duplicate_preference(&key(), Some(&preferred))
            .unwrap_err()
            .code,
        "exact_duplicate_group_changed"
    );
    assert_eq!(
        catalog
            .load_review_members(&key(), "")
            .unwrap()
            .decision
            .preferred_location_id
            .as_deref(),
        Some("c")
    );
    catalog
        .connection
        .execute("DELETE FROM asset_locations WHERE location_id = 'c'", [])
        .unwrap();
    assert_eq!(gallery_ids(&mut catalog, None), ["a"]);
    assert_eq!(
        catalog
            .load_review_members(&key(), "")
            .unwrap()
            .decision
            .preferred_location_id
            .as_deref(),
        Some("c")
    );
}

#[test]
fn compatible_reanalysis_and_new_members_keep_ignored_and_preferred_choices() {
    let (_directory, mut catalog) = fixture();
    for id in ["a", "b"] {
        assert!(publish(&mut catalog, id, &key().digest));
    }
    assert!(publish(&mut catalog, "c", &"b".repeat(64)));
    assert_eq!(
        catalog.load_review_groups(None, false).unwrap().groups[0]
            .group
            .location_count,
        2
    );
    let preferred = preference(&catalog, "b");
    catalog
        .set_duplicate_preference(&key(), Some(&preferred))
        .unwrap();
    catalog.set_duplicate_ignored(&key(), true).unwrap();
    // Rebuild only derived evidence: durable decisions have no cascading foreign key.
    catalog
        .connection
        .execute_batch(
            "DELETE FROM gallery_content_fingerprints; DELETE FROM content_fingerprints;",
        )
        .unwrap();
    assert!(
        catalog
            .load_review_groups(None, true)
            .unwrap()
            .groups
            .is_empty()
    );
    for id in ["a", "b", "c"] {
        assert!(publish(&mut catalog, id, &key().digest));
    }
    let groups = catalog.load_review_groups(None, true).unwrap();
    assert_eq!(groups.groups[0].group.location_count, 3);
    assert_eq!(groups.groups[0].representative.location_id, "b");
    assert!(groups.groups[0].decision.ignored);
    let other = ExactDuplicateReviewKey {
        digest: "b".repeat(64),
        byte_length: 20,
    };
    let untouched = catalog.load_review_members(&other, "").unwrap().decision;
    assert!(!untouched.ignored && untouched.preferred_location_id.is_none());
}

#[test]
fn no_op_choices_keep_revision_and_stale_choice_cannot_replace_valid_one() {
    let (_directory, mut catalog) = grouped_fixture();
    let preferred = preference(&catalog, "b");
    catalog
        .set_duplicate_preference(&key(), Some(&preferred))
        .unwrap();
    let revision: i64 = catalog
        .connection
        .query_row("SELECT revision FROM catalog_state", [], |row| row.get(0))
        .unwrap();
    catalog
        .set_duplicate_preference(&key(), Some(&preferred))
        .unwrap();
    catalog.set_duplicate_ignored(&key(), true).unwrap();
    let updated_revision: i64 = catalog
        .connection
        .query_row("SELECT revision FROM catalog_state", [], |row| row.get(0))
        .unwrap();
    assert_eq!(updated_revision, revision);
    let stale = ExactDuplicatePreference {
        source_generation: preferred.source_generation + 100,
        ..preference(&catalog, "a")
    };
    assert!(
        catalog
            .set_duplicate_preference(&key(), Some(&stale))
            .is_err()
    );
    let replaced_asset = ExactDuplicatePreference {
        asset_id: "another-asset".to_owned(),
        ..preferred
    };
    assert!(
        catalog
            .set_duplicate_preference(&key(), Some(&replaced_asset))
            .is_err()
    );
    let decision = catalog.load_review_members(&key(), "").unwrap().decision;
    assert!(decision.ignored);
    assert_eq!(decision.preferred_location_id.as_deref(), Some("b"));
}

#[test]
fn ignored_filter_is_applied_before_bounded_group_pagination() {
    let (_directory, mut catalog) = fixture();
    let ids = (0..260)
        .map(|index| format!("review-{index:04}"))
        .collect::<Vec<_>>();
    let locations = ids
        .iter()
        .map(|id| (id.as_str(), None, 1))
        .collect::<Vec<_>>();
    super::super::super::tests::publish_gallery_fixture(
        &mut catalog,
        "scan-review",
        "root-review",
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
         FROM library_change_root_state WHERE root_id = 'root-review';",
        )
        .unwrap();
    for (index, id) in ids.iter().enumerate() {
        assert!(publish(&mut catalog, id, &format!("{:064x}", index / 2)));
    }
    for group_index in (0..130).step_by(2) {
        catalog
            .set_duplicate_ignored(
                &ExactDuplicateReviewKey {
                    digest: format!("{group_index:064x}"),
                    byte_length: 20,
                },
                true,
            )
            .unwrap();
    }
    for ignored in [false, true] {
        let first = catalog.load_review_groups(None, ignored).unwrap();
        assert_eq!(first.groups.len(), 64);
        let last = catalog
            .load_review_groups(first.next.as_ref(), ignored)
            .unwrap();
        assert_eq!(last.groups.len(), 1);
        assert!(last.next.is_none());
        let actual = first
            .groups
            .iter()
            .chain(&last.groups)
            .map(|entry| entry.group.digest.clone())
            .collect::<Vec<_>>();
        let start = usize::from(!ignored);
        let expected = (start..130)
            .step_by(2)
            .map(|index| format!("{index:064x}"))
            .collect::<Vec<_>>();
        assert_eq!(actual, expected);
        assert!(
            first
                .groups
                .iter()
                .chain(&last.groups)
                .all(|entry| entry.decision.ignored == ignored)
        );
    }
}
