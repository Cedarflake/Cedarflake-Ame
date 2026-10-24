use crate::domain::organization_queue::{OrganizationCounts, OrganizationQueue};

use super::*;

mod aggregation;
mod history;

fn queue(kind: OrganizationQueue) -> GalleryQuery {
    GalleryQuery {
        organization_queue: Some(kind),
        sort_key: GallerySortKey::FileName,
        sort_direction: GallerySortDirection::Ascending,
        ..GalleryQuery::default()
    }
}

fn counts(catalog: &mut SqliteCatalog, query: &GalleryQuery) -> OrganizationCounts {
    catalog
        .load_query_snapshot(1, query, "queue", None)
        .unwrap()
        .organization_counts
        .unwrap()
}

#[test]
fn new_versions_precede_older_work_without_changing_ordinary_gallery_sorting() {
    let (_directory, mut catalog) = fixture();
    let pending = queue(OrganizationQueue::Pending);
    assert_eq!(ids(&mut catalog, &pending), ["c", "b", "a"]);
    let ordinary = GalleryQuery {
        organization_queue: None,
        ..pending.clone()
    };
    assert_eq!(ids(&mut catalog, &ordinary), ["a", "b", "c"]);
    assert_eq!(
        counts(&mut catalog, &pending),
        OrganizationCounts {
            total: 3,
            pending: 3,
            ..OrganizationCounts::default()
        }
    );
    assert!(
        catalog
            .load_query_snapshot(1, &ordinary, "ordinary", None)
            .unwrap()
            .organization_counts
            .is_none()
    );
}

#[test]
fn queue_pages_layout_counts_and_hidden_member_anchor_share_the_first_group_admission() {
    let (_directory, mut catalog) = fixture();
    publish_batch(&mut catalog, &["a", "c"], 'a');
    let pending = queue(OrganizationQueue::Pending);
    assert_eq!(ids(&mut catalog, &pending), ["b", "a"]);
    let first = catalog
        .load_query_snapshot(1, &pending, "queue", None)
        .unwrap();
    assert_eq!(first.snapshot.assets[0].location_id, "b");
    assert_eq!(
        (
            first.timeline.total_items,
            first.timeline.total_source_items
        ),
        (2, 3)
    );
    assert!(first.timeline.buckets.is_empty());
    assert_eq!(first.organization_counts.unwrap().pending, 2);
    let second = catalog
        .load_snapshot(
            1,
            &pending,
            "queue",
            first.snapshot.next_cursor.as_ref(),
            None,
            None,
        )
        .unwrap();
    assert_eq!(second.assets[0].location_id, "a");
    let backward = catalog
        .load_snapshot(
            1,
            &pending,
            "queue",
            None,
            second.previous_cursor.as_ref(),
            None,
        )
        .unwrap();
    assert_eq!(backward.assets[0].location_id, "b");
    let layout = catalog
        .load_gallery_layout_manifest_chunk(2, &pending, "queue", None)
        .unwrap();
    assert_eq!(layout.location_ids, ["b", "a"]);
    let hidden = selected(&catalog, "c");
    let anchored = catalog
        .load_snapshot_around_asset(1, &pending, "queue", "c", &hidden.asset_id, 1)
        .unwrap();
    let resolved = anchored.query_anchor_resolution.unwrap();
    assert_eq!(resolved.location_id.as_deref(), Some("a"));
    assert_eq!(resolved.ordinal, Some(1));
    let scoped = GalleryQuery {
        root_id: Some("root-c".into()),
        ..pending
    };
    assert_eq!(ids(&mut catalog, &scoped), ["c"]);
    assert_eq!(counts(&mut catalog, &scoped).total, 1);
}

#[test]
fn ordinary_actions_update_shared_queue_counts_and_favorites_remain_independent() {
    let (_directory, mut catalog) = fixture();
    let pending = queue(OrganizationQueue::Pending);
    let deferred = queue(OrganizationQueue::Deferred);
    let ignored = queue(OrganizationQueue::Ignored);
    let prepared = prepare(&catalog, &["c"]);
    catalog
        .set_organization_favorite("favorite", &prepared, true)
        .unwrap();
    assert_eq!(counts(&mut catalog, &pending).pending, 3);
    let favorites = GalleryQuery {
        collection: GalleryCollection::Favorites,
        ..pending.clone()
    };
    assert_eq!(ids(&mut catalog, &favorites), ["c"]);
    assert_eq!(counts(&mut catalog, &favorites).total, 1);
    let album = create_album(&mut catalog, "Organized");
    assign(&mut catalog, "assign", &["a"], &album);
    change(&mut catalog, "defer", &["b"], OrganizationAction::Defer);
    change(&mut catalog, "ignore", &["c"], OrganizationAction::Ignore);
    assert!(ids(&mut catalog, &pending).is_empty());
    assert_eq!(ids(&mut catalog, &deferred), ["b"]);
    assert_eq!(ids(&mut catalog, &ignored), ["c"]);
    let expected = OrganizationCounts {
        total: 3,
        pending: 0,
        deferred: 1,
        organized: 1,
        ignored: 1,
    };
    for query in [&pending, &deferred, &ignored] {
        assert_eq!(counts(&mut catalog, query), expected);
    }
    let ignored_favorites = GalleryQuery {
        organization_queue: Some(OrganizationQueue::Ignored),
        ..favorites
    };
    assert_eq!(ids(&mut catalog, &ignored_favorites), ["c"]);
    change(
        &mut catalog,
        "restore",
        &["c"],
        OrganizationAction::RestorePending,
    );
    assert_eq!(ids(&mut catalog, &pending), ["c"]);
    assert_eq!(counts(&mut catalog, &pending).pending, 1);
}

#[test]
fn late_conflicts_enter_deferred_once_and_independent_changed_content_returns_to_the_front() {
    let (_directory, mut catalog) = fixture();
    let first = create_album(&mut catalog, "First");
    let second = create_album(&mut catalog, "Second");
    assign(&mut catalog, "assign-a", &["a"], &first);
    assign(&mut catalog, "assign-c", &["c"], &second);
    publish_batch(&mut catalog, &["a", "c"], 'a');
    let pending = queue(OrganizationQueue::Pending);
    let deferred = queue(OrganizationQueue::Deferred);
    assert_eq!(ids(&mut catalog, &deferred), ["a"]);
    assert_eq!(
        counts(&mut catalog, &pending),
        OrganizationCounts {
            total: 2,
            pending: 1,
            deferred: 1,
            ..OrganizationCounts::default()
        }
    );
    assign(&mut catalog, "resolve", &["c"], &second);
    assert!(ids(&mut catalog, &deferred).is_empty());
    super::evolution::invalidate(&mut catalog, "c");
    publish_batch(&mut catalog, &["c"], 'b');
    assert_eq!(ids(&mut catalog, &pending), ["c", "b"]);
    assert_eq!(
        counts(&mut catalog, &pending),
        OrganizationCounts {
            total: 3,
            pending: 2,
            organized: 1,
            ..OrganizationCounts::default()
        }
    );
}

#[test]
fn representative_replacement_and_derived_cache_cleanup_keep_a_deferred_groups_order() {
    let (_directory, mut catalog) = fixture();
    publish_batch(&mut catalog, &["a", "c"], 'a');
    change(
        &mut catalog,
        "defer",
        &["a", "b"],
        OrganizationAction::Defer,
    );
    let deferred = queue(OrganizationQueue::Deferred);
    let preferred = selected(&catalog, "c");
    catalog
        .set_duplicate_preference(
            &ExactDuplicateReviewKey {
                digest: "a".repeat(64),
                byte_length: 20,
            },
            Some(&ExactDuplicatePreference {
                location_id: "c".into(),
                asset_id: preferred.asset_id,
                source_generation: preferred.source_generation,
            }),
        )
        .unwrap();
    assert_eq!(ids(&mut catalog, &deferred), ["b", "c"]);
    catalog
        .connection
        .execute_batch(
            "DELETE FROM gallery_content_fingerprints; DELETE FROM content_fingerprints;",
        )
        .unwrap();
    assert_eq!(ids(&mut catalog, &deferred), ["b", "c"]);
    assert_eq!(counts(&mut catalog, &deferred).deferred, 2);
}

#[test]
fn pages_and_all_progress_counts_use_one_snapshot_during_a_concurrent_save() {
    let (directory, mut catalog) = fixture();
    let pending = queue(OrganizationQueue::Pending);
    let path = directory.path().join("catalog.sqlite3");
    set_after_query_snapshot_read_hook(move || {
        let mut writer = SqliteCatalog::open(path).unwrap();
        change(&mut writer, "defer", &["c"], OrganizationAction::Defer);
    });
    let previous = catalog
        .load_query_snapshot(1, &pending, "queue", None)
        .unwrap();
    assert_eq!(previous.snapshot.assets[0].location_id, "c");
    assert_eq!(previous.timeline.total_items, 3);
    assert_eq!(previous.organization_counts.unwrap().pending, 3);
    let current = catalog
        .load_query_snapshot(1, &pending, "queue", None)
        .unwrap();
    assert_eq!(current.snapshot.assets[0].location_id, "b");
    assert_eq!(current.timeline.total_items, 2);
    let counts = current.organization_counts.unwrap();
    assert_eq!((counts.total, counts.pending, counts.deferred), (3, 2, 1));
    assert!(current.snapshot.revision > previous.snapshot.revision);
}

#[test]
fn new_arrivals_prepend_without_replacing_the_requested_logical_anchor() {
    let (_directory, mut catalog) = fixture();
    publish_batch(&mut catalog, &["a", "c"], 'a');
    let pending = queue(OrganizationQueue::Pending);
    let selected = selected(&catalog, "a");
    crate::adapters::sqlite_catalog::tests::publish_gallery_fixture(
        &mut catalog,
        "new-scan",
        "new-root",
        "C:\\New",
        &[("new", Some("1900-01-01T00:00:00"), 0)],
    );
    assert_eq!(ids(&mut catalog, &pending), ["new", "b", "a"]);
    let snapshot = catalog
        .load_query_snapshot(
            2,
            &pending,
            "new-queue",
            Some(&crate::domain::GalleryQueryAnchor {
                requested_location_id: "a".into(),
                asset_id: Some(selected.asset_id),
                fallback_ordinal: 1,
            }),
        )
        .unwrap();
    let anchor = snapshot.snapshot.query_anchor_resolution.unwrap();
    assert_eq!(anchor.location_id.as_deref(), Some("a"));
    assert_eq!(anchor.ordinal, Some(2));
    assert_eq!(snapshot.organization_counts.unwrap().pending, 3);
}

#[test]
fn empty_scopes_return_zero_counts_and_pending_evidence_retains_existing_work() {
    let (_directory, mut catalog) = fixture();
    let empty = GalleryQuery {
        root_id: Some("absent".into()),
        ..queue(OrganizationQueue::Pending)
    };
    assert_eq!(counts(&mut catalog, &empty), OrganizationCounts::default());
    catalog.connection.execute(
        "UPDATE asset_locations SET file_identity_scheme = 'fixture', file_identity_value = location_id", []
    ).unwrap();
    publish_batch(&mut catalog, &["a", "c"], 'a');
    change(&mut catalog, "defer", &["a"], OrganizationAction::Defer);
    super::evolution::invalidate(&mut catalog, "a");
    let deferred = queue(OrganizationQueue::Deferred);
    let snapshot = catalog
        .load_query_snapshot(1, &deferred, "pending-evidence", None)
        .unwrap();
    assert_eq!(snapshot.snapshot.assets[0].location_id, "a");
    assert_eq!(snapshot.timeline.total_source_items, 2);
    assert_eq!(
        snapshot.organization_counts.unwrap(),
        OrganizationCounts {
            total: 2,
            pending: 1,
            deferred: 1,
            ..OrganizationCounts::default()
        }
    );
    assert_eq!(
        catalog
            .prepare_organization_selection(&[selected(&catalog, "a")])
            .unwrap_err()
            .code,
        "organization_evidence_pending"
    );
}
