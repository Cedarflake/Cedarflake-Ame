use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::domain::organization_queue::OrganizationQueue;
use crate::domain::organization_review::{
    OrganizationContinuationStatus, OrganizationReviewSession,
};
use crate::ports::organization_review::OrganizationReviewRepository;

use super::*;

#[test]
fn collection_and_queue_reads_do_not_expand_unrelated_fingerprints() {
    let (_directory, mut catalog) = fixture();
    catalog
        .connection
        .execute(
            "UPDATE asset_locations SET file_identity_scheme = 'fixture',
               file_identity_value = location_id",
            [],
        )
        .unwrap();
    publish_batch(&mut catalog, &["a", "c"], 'a');
    let album = create_album(&mut catalog, "Visible");
    assign(&mut catalog, "assign", &["c"], &album);
    let prepared = prepare(&catalog, &["c"]);
    catalog
        .set_organization_favorite("favorite", &prepared, true)
        .unwrap();
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
    let pending = GalleryQuery {
        organization_queue: Some(OrganizationQueue::Pending),
        ..GalleryQuery::default()
    };
    let session = catalog.begin_organization_review(&pending).unwrap();
    catalog
        .checkpoint_organization_review(
            &OrganizationReviewSession {
                position_sequence: 1,
                ..session.clone()
            },
            &selected(&catalog, "b"),
        )
        .unwrap();
    catalog
        .end_organization_review(&session.session_id)
        .unwrap();
    catalog
        .connection
        .execute_batch(
            "WITH RECURSIVE sequence(value) AS (
               VALUES(1000) UNION ALL SELECT value + 1 FROM sequence WHERE value < 50999
             )
             INSERT INTO content_fingerprints
               (source_generation, scheme, digest, byte_length, run_id)
             SELECT value, 'blake3-256-full-v1', printf('%064x', value), 20,
               'organization-fixture' FROM sequence;
             INSERT INTO gallery_content_fingerprints
             SELECT source_generation, scheme FROM content_fingerprints
             WHERE source_generation >= 1000;",
        )
        .unwrap();

    for retained in [false, true] {
        if retained {
            evolution::invalidate(&mut catalog, "c");
        }
        for (query, expected) in [
            (album_query(&album), Some("c")),
            (
                GalleryQuery {
                    collection: GalleryCollection::Favorites,
                    duplicate_mode: GalleryDuplicateMode::Merged,
                    ..GalleryQuery::default()
                },
                Some("c"),
            ),
            (pending.clone(), Some("b")),
            (
                GalleryQuery {
                    organization_queue: Some(OrganizationQueue::Ignored),
                    ..GalleryQuery::default()
                },
                None,
            ),
        ] {
            let snapshot = bounded_read(&mut catalog, |catalog| {
                catalog.load_query_snapshot(1, &query, "fingerprint-history", None)
            });
            assert_eq!(
                snapshot
                    .snapshot
                    .assets
                    .first()
                    .map(|asset| asset.location_id.as_str()),
                expected,
            );
            assert_eq!(snapshot.timeline.total_items, u64::from(expected.is_some()));
            if let Some(counts) = snapshot.organization_counts {
                assert_eq!((counts.total, counts.pending, counts.organized), (2, 1, 1));
            }
        }
        let continuation = bounded_read(&mut catalog, |catalog| {
            catalog.load_organization_continuation()
        });
        assert_eq!(continuation.status, OrganizationContinuationStatus::Ready);
        assert_eq!(continuation.target, Some(selected(&catalog, "b")));
    }
}

fn bounded_read<T>(
    catalog: &mut SqliteCatalog,
    read: impl FnOnce(&mut SqliteCatalog) -> Result<T, ScanError>,
) -> T {
    let ticks = Arc::new(AtomicUsize::new(0));
    let observed = Arc::clone(&ticks);
    catalog
        .connection
        .progress_handler(
            100,
            Some(move || {
                observed.fetch_add(1, Ordering::Relaxed);
                false
            }),
        )
        .unwrap();
    let result = read(catalog);
    catalog
        .connection
        .progress_handler(0, None::<fn() -> bool>)
        .unwrap();
    let steps = ticks.load(Ordering::Relaxed) * 100;
    assert!(
        steps < 20_000,
        "a visible-group read used {steps} SQLite steps"
    );
    result.unwrap()
}
