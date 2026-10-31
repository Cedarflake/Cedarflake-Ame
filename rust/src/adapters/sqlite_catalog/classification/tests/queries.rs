use crate::domain::classification_review::ClassificationReviewFilter;
use crate::domain::{GalleryCollection, GalleryQuery, GallerySortDirection, GallerySortKey};
use crate::ports::exact_duplicates::ExactDuplicateRepository;
use crate::ports::{CatalogRepository, GalleryQueryRepository};

use super::*;

mod continuation;
mod evolution;
mod predictions;
mod publication;

#[test]
fn smart_album_rejects_organization_queue_and_invalid_category() {
    use crate::domain::organization_queue::OrganizationQueue;
    let (_directory, mut catalog) = fixture();
    for query in [
        GalleryQuery {
            organization_queue: Some(OrganizationQueue::Pending),
            ..query(None, ClassificationReviewFilter::All)
        },
        query(Some("bad/category"), ClassificationReviewFilter::All),
    ] {
        assert_eq!(
            catalog
                .load_query_snapshot(1, &query, "invalid", None)
                .unwrap_err()
                .code,
            "classification_scope_invalid"
        );
    }
}

fn query(category: Option<&str>, review: ClassificationReviewFilter) -> GalleryQuery {
    GalleryQuery {
        collection: GalleryCollection::SmartAlbum {
            category_id: category.map(str::to_owned),
            review,
        },
        sort_key: GallerySortKey::FileName,
        sort_direction: GallerySortDirection::Ascending,
        ..Default::default()
    }
}

fn publish_group(catalog: &mut SqliteCatalog, names: &[&str], digest: char) {
    for name in names {
        publish(catalog, name, digest);
    }
    let generations = names
        .iter()
        .map(|name| selected(catalog, name).source_generation)
        .collect::<Vec<_>>();
    catalog
        .publish_exact_analysis_batch("organization-fixture", &generations)
        .unwrap();
}

fn ids(catalog: &mut SqliteCatalog, query: &GalleryQuery) -> Vec<String> {
    let snapshot = catalog
        .load_query_snapshot(20, query, "classification", None)
        .unwrap();
    assert_eq!(
        snapshot.timeline.total_items,
        snapshot.snapshot.assets.len() as u64
    );
    snapshot
        .snapshot
        .assets
        .into_iter()
        .map(|asset| asset.location_id)
        .collect()
}

fn correct(catalog: &mut SqliteCatalog, operation: &str, image: &str, destination: &str) {
    let prepared = prepare(catalog, &[image]);
    catalog
        .change_classification(
            &id(operation),
            &prepared,
            &ClassificationAction::Correct(category(destination)),
        )
        .unwrap();
}

#[test]
fn pages_counts_layout_and_hidden_copy_anchor_share_classification_membership() {
    let (_directory, mut catalog) = fixture();
    publish_group(&mut catalog, &["a", "c"], 'a');
    publish_group(&mut catalog, &["b"], 'b');
    let run = setup(&mut catalog);
    predict(&mut catalog, &run, "a", "first-a", "drawing");
    predict(&mut catalog, &run, "b", "first-b", "drawing");
    let query = query(Some("drawing"), ClassificationReviewFilter::All);
    let first = catalog
        .load_query_snapshot(1, &query, "drawing", None)
        .unwrap();
    assert_eq!(
        (
            first.timeline.total_items,
            first.timeline.total_source_items
        ),
        (2, 3)
    );
    assert_eq!(first.snapshot.assets[0].location_id, "a");
    let next = catalog
        .load_snapshot(
            1,
            &query,
            "drawing",
            first.snapshot.next_cursor.as_ref(),
            None,
            None,
        )
        .unwrap();
    assert_eq!(next.assets[0].location_id, "b");
    assert!(next.next_cursor.is_none());
    let previous = catalog
        .load_snapshot(
            1,
            &query,
            "drawing",
            None,
            next.previous_cursor.as_ref(),
            None,
        )
        .unwrap();
    assert_eq!(previous.assets[0].location_id, "a");
    let layout = catalog
        .load_gallery_layout_manifest_chunk(1, &query, "drawing", None)
        .unwrap();
    assert_eq!(layout.location_ids, ["a"]);
    assert_eq!(layout.total_items, 2);
    let following = catalog
        .load_gallery_layout_manifest_chunk(1, &query, "drawing", layout.next_cursor.as_ref())
        .unwrap();
    assert_eq!(following.location_ids, ["b"]);
    let hidden = selected(&catalog, "c");
    let anchored = catalog
        .load_snapshot_around_asset(1, &query, "drawing", "c", &hidden.asset_id, 1)
        .unwrap();
    assert_eq!(
        anchored
            .query_anchor_resolution
            .unwrap()
            .location_id
            .as_deref(),
        Some("a")
    );
    let scoped = GalleryQuery {
        root_id: Some("root-c".into()),
        search_text: "c.png".into(),
        ..query.clone()
    };
    let scoped = catalog
        .load_query_snapshot(1, &scoped, "physical-copy", None)
        .unwrap();
    assert_eq!(scoped.snapshot.assets[0].location_id, "c");
    assert_eq!(
        (
            scoped.timeline.total_items,
            scoped.timeline.total_source_items
        ),
        (1, 1)
    );
    let dated = GalleryQuery {
        sort_key: GallerySortKey::ModifiedTime,
        ..query
    };
    let timeline = catalog.load_gallery_timeline(&dated, "date").unwrap();
    assert_eq!(
        timeline
            .buckets
            .iter()
            .map(|bucket| bucket.item_count)
            .sum::<u64>(),
        2
    );
}

#[test]
fn human_correction_and_undo_move_whole_groups_without_changing_r4() {
    let (_directory, mut catalog) = fixture();
    publish_group(&mut catalog, &["a", "c"], 'a');
    let run = setup(&mut catalog);
    predict(&mut catalog, &run, "a", "first", "drawing");
    let r4 = scalar(&catalog, "SELECT revision FROM organization_state");
    let all = query(None, ClassificationReviewFilter::All);
    assert_eq!(ids(&mut catalog, &all), ["a", "b"]);
    assert_eq!(
        ids(
            &mut catalog,
            &query(None, ClassificationReviewFilter::Pending)
        ),
        ["a"]
    );
    correct(&mut catalog, "correct", "c", "photo");
    assert!(
        ids(
            &mut catalog,
            &query(Some("drawing"), ClassificationReviewFilter::All)
        )
        .is_empty()
    );
    assert_eq!(
        ids(
            &mut catalog,
            &query(Some("photo"), ClassificationReviewFilter::Confirmed)
        ),
        ["a"]
    );
    predict(&mut catalog, &run, "a", "later", "drawing");
    assert_eq!(
        ids(
            &mut catalog,
            &query(Some("photo"), ClassificationReviewFilter::Confirmed)
        ),
        ["a"]
    );
    catalog
        .undo_classification(&id("undo"), &id("correct"))
        .unwrap();
    assert_eq!(
        ids(
            &mut catalog,
            &query(Some("drawing"), ClassificationReviewFilter::Pending)
        ),
        ["a"]
    );
    assert_eq!(
        scalar(&catalog, "SELECT revision FROM organization_state"),
        r4
    );
    assert_eq!(
        scalar(&catalog, "SELECT count(*) FROM organization_operations"),
        0
    );
}

#[test]
fn all_results_keeps_unclassified_groups_and_an_empty_category_has_a_valid_anchor_query() {
    let (_directory, mut catalog) = fixture();
    setup(&mut catalog);
    assert_eq!(
        ids(&mut catalog, &query(None, ClassificationReviewFilter::All)),
        ["a", "b", "c"]
    );
    let requested = selected(&catalog, "b");
    let all = query(None, ClassificationReviewFilter::All);
    let anchored = catalog
        .load_snapshot_around_asset(1, &all, "all", "b", &requested.asset_id, 1)
        .unwrap();
    assert_eq!(
        anchored
            .query_anchor_resolution
            .unwrap()
            .location_id
            .as_deref(),
        Some("b")
    );
    let empty = query(Some("drawing"), ClassificationReviewFilter::All);
    assert!(ids(&mut catalog, &empty).is_empty());
    let anchored = catalog
        .load_snapshot_around_asset(1, &empty, "empty", "b", &requested.asset_id, 1)
        .unwrap();
    assert!(anchored.assets.is_empty());
}
