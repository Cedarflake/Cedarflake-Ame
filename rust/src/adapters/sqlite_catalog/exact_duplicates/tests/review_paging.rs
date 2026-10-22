use crate::domain::exact_duplicate_review::{ExactDuplicatePreference, ExactDuplicateReviewKey};

use super::*;

#[test]
fn review_seek_matches_keyset_pages_and_clamps_to_current_filtered_membership() {
    let (_directory, mut catalog) = fixture();
    let ids = (0..260)
        .map(|index| format!("seek-{index:04}"))
        .collect::<Vec<_>>();
    let locations = ids
        .iter()
        .map(|id| (id.as_str(), None, 1))
        .collect::<Vec<_>>();
    super::super::super::tests::publish_gallery_fixture(
        &mut catalog,
        "scan-seek",
        "root-seek",
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
         FROM library_change_root_state WHERE root_id = 'root-seek';",
        )
        .unwrap();
    for (index, id) in ids.iter().enumerate() {
        assert!(publish(&mut catalog, id, &format!("{:064x}", index / 2)));
    }

    for (requested, start, length) in [(0, 0, 64), (70, 64, 64), (129, 128, 2), (u64::MAX, 128, 2)]
    {
        let range = catalog.seek_review_groups(requested, false).unwrap();
        assert_eq!(range.start_index, start);
        assert_eq!(range.page.total_groups, 130);
        assert_eq!(range.page.groups.len(), length);
        assert_eq!(range.page.groups[0].group.digest, format!("{start:064x}"));
        assert_eq!(range.page.next.is_some(), start < 128);
    }
    let first = catalog.seek_review_groups(0, false).unwrap();
    let next = catalog
        .load_review_groups(first.page.next.as_ref(), false)
        .unwrap();
    let middle = catalog.seek_review_groups(70, false).unwrap();
    assert_eq!(
        next.groups
            .iter()
            .map(|group| &group.group.digest)
            .collect::<Vec<_>>(),
        middle
            .page
            .groups
            .iter()
            .map(|group| &group.group.digest)
            .collect::<Vec<_>>(),
    );

    let ignored_key = ExactDuplicateReviewKey {
        digest: format!("{:064x}", 5),
        byte_length: 20,
    };
    catalog.set_duplicate_ignored(&ignored_key, true).unwrap();
    let preferred_key = ExactDuplicateReviewKey {
        digest: format!("{:064x}", 65),
        byte_length: 20,
    };
    let preferred = candidate(&catalog, "seek-0131").location;
    catalog
        .set_duplicate_preference(
            &preferred_key,
            Some(&ExactDuplicatePreference {
                location_id: preferred.location_id.clone(),
                asset_id: preferred.asset_id,
                source_generation: preferred.source_generation,
            }),
        )
        .unwrap();
    let filtered = catalog.seek_review_groups(64, false).unwrap();
    assert_eq!(
        (
            filtered.start_index,
            filtered.page.total_groups,
            filtered.page.groups.len()
        ),
        (64, 129, 64)
    );
    assert_eq!(filtered.page.groups[0].group.digest, preferred_key.digest);
    assert_eq!(
        filtered.page.groups[0].representative.location_id,
        preferred.location_id
    );
    let ignored = catalog.seek_review_groups(u64::MAX, true).unwrap();
    assert_eq!(
        (
            ignored.start_index,
            ignored.page.total_groups,
            ignored.page.groups.len()
        ),
        (0, 1, 1)
    );
    assert_eq!(ignored.page.groups[0].group.digest, ignored_key.digest);
    assert!(ignored.page.groups[0].decision.ignored);

    catalog
        .connection
        .execute(
            "UPDATE asset_locations SET source_generation = source_generation + 1000
         WHERE location_id IN ('seek-0258', 'seek-0259')",
            [],
        )
        .unwrap();
    let shrunk = catalog.seek_review_groups(129, false).unwrap();
    assert_eq!(
        (
            shrunk.start_index,
            shrunk.page.total_groups,
            shrunk.page.groups.len()
        ),
        (64, 128, 64)
    );
    assert!(shrunk.page.next.is_none());
}

#[test]
fn review_seek_returns_an_empty_origin_when_no_current_group_matches() {
    let (_directory, catalog) = fixture();
    for ignored in [false, true] {
        let range = catalog.seek_review_groups(u64::MAX, ignored).unwrap();
        assert_eq!((range.start_index, range.page.total_groups), (0, 0));
        assert!(range.page.groups.is_empty());
        assert!(range.page.next.is_none());
    }
}
