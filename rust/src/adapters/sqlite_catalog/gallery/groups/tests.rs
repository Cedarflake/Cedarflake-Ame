use std::sync::{Arc, atomic::AtomicBool};

use crate::adapters::sqlite_catalog::SqliteCatalog;
use crate::adapters::sqlite_catalog::file_operations::SqliteFileOperationSelection;
use crate::adapters::sqlite_catalog::organization::tests::{fixture, publish};
use crate::domain::exact_duplicates::{ExactAnalysisPhase, ExactAnalysisProgress};
use crate::domain::file_operation_exports::FileOrganizationRule;
use crate::domain::file_operation_selection::FileOperationSelectionScope as Scope;
use crate::domain::gallery_groups::{GalleryGroupFilter, GalleryGroupKey};
use crate::domain::{GalleryQuery, GalleryQueryAnchor, GallerySortDirection, GallerySortKey};
use crate::ports::exact_duplicates::ExactDuplicateRepository;
use crate::ports::file_operation_selection::FileOperationSelectionRepository;
use crate::ports::semantic_catalog::SemanticCatalog;
use crate::ports::{CatalogRepository, GalleryQueryRepository};

fn publish_exact(catalog: &mut SqliteCatalog) {
    let mut progress = ExactAnalysisProgress::running("organization-fixture".into());
    progress.phase = ExactAnalysisPhase::Completed;
    catalog.finish_exact_analysis(&progress).unwrap();
}

fn query(filter: GalleryGroupFilter) -> GalleryQuery {
    GalleryQuery {
        group_filter: filter,
        sort_key: GallerySortKey::FileName,
        sort_direction: GallerySortDirection::Ascending,
        ..GalleryQuery::default()
    }
}

fn assert_semantic_locations(catalog: &SqliteCatalog, query: &GalleryQuery, expected: &[&str]) {
    let mut locations = Vec::new();
    catalog
        .semantic_snapshot()
        .unwrap()
        .scan(query, &mut |window| {
            locations.extend(
                window
                    .into_iter()
                    .map(|candidate| candidate.source.location_id),
            );
            Ok(())
        })
        .unwrap();
    assert_eq!(locations, expected);
}

#[test]
fn exact_gallery_filters_scope_before_counts_members_and_cleanup() {
    let (directory, mut catalog) = fixture();
    for id in ["a", "b", "c"] {
        publish(&mut catalog, id, 'a');
    }
    publish_exact(&mut catalog);
    let scoped = GalleryQuery {
        root_id: Some("root-a".into()),
        ..query(GalleryGroupFilter::Exact)
    };
    let page = catalog
        .load_snapshot(10, &scoped, "groups", None, None, None)
        .unwrap();
    assert_eq!(page.assets.len(), 1);
    let group = page.assets[0].group.as_ref().unwrap();
    assert_eq!(group.member_count, 2);
    let members = catalog
        .load_gallery_group_members(&scoped, &group.key, "")
        .unwrap();
    assert_eq!(members.total, 2);
    assert_eq!(
        members
            .members
            .iter()
            .map(|m| m.source.location_id.as_str())
            .collect::<Vec<_>>(),
        ["a", "b"]
    );
    let outside = GalleryQuery {
        root_id: Some("root-c".into()),
        ..scoped.clone()
    };
    assert_semantic_locations(&catalog, &scoped, &["a", "b"]);
    assert_semantic_locations(&catalog, &outside, &[]);
    assert!(
        catalog
            .load_snapshot(10, &outside, "outside", None, None, None)
            .unwrap()
            .assets
            .is_empty()
    );
    let mut cleanup = SqliteFileOperationSelection::new(
        SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap(),
        Scope::GalleryCleanup {
            query: Box::new(scoped.clone()),
            catalog_revision: page.revision,
            all_matching: false,
        },
        Arc::new(AtomicBool::new(false)),
    )
    .unwrap();
    cleanup.append_ids(&[group.key.entry_id()]).unwrap();
    assert_eq!(cleanup.prepare().unwrap().file_count, 1);
    assert_eq!(cleanup.page(None).unwrap()[0].location.location_id, "b");
    assert_eq!(cleanup.retained_page("").unwrap()[0].location_id, "a");
    cleanup.freeze_rule(FileOrganizationRule::Direct).unwrap();
    let mut all = SqliteFileOperationSelection::new(
        SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap(),
        Scope::Gallery {
            query: Box::new(scoped),
            catalog_revision: page.revision,
            all_matching: false,
        },
        Arc::new(AtomicBool::new(false)),
    )
    .unwrap();
    all.append_ids(&[group.key.entry_id()]).unwrap();
    assert_eq!(all.prepare().unwrap().file_count, 2);
    assert!(
        all.page(None)
            .unwrap()
            .iter()
            .all(|candidate| candidate.location.root_id == "root-a")
    );

    use crate::domain::organization_review::OrganizationReviewSession;
    use crate::ports::organization_review::OrganizationReviewRepository;
    let review_query = GalleryQuery {
        organization_queue: Some(crate::domain::organization_queue::OrganizationQueue::Pending),
        ..query(GalleryGroupFilter::Exact)
    };
    let session = catalog.begin_organization_review(&review_query).unwrap();
    catalog
        .checkpoint_organization_review(
            &OrganizationReviewSession {
                position_sequence: 1,
                ..session.clone()
            },
            &crate::adapters::sqlite_catalog::organization::tests::selected(&catalog, "a"),
        )
        .unwrap();
    catalog
        .end_organization_review(&session.session_id)
        .unwrap();
    drop(catalog);
    let catalog = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
    assert_eq!(
        catalog
            .load_organization_continuation(None)
            .unwrap()
            .query
            .unwrap()
            .group_filter,
        GalleryGroupFilter::Exact
    );
}

#[test]
fn similar_gallery_shared_covers_keep_distinct_paging_and_anchor_identity() {
    use crate::domain::similar_images::{
        SIMILARITY_SCHEME, SimilarAnalysisPhase, SimilarAnalysisProgress, SimilarAnalysisRun,
        SimilarContentKey, SimilarEvidence, SimilarVariant, group_id,
    };
    use crate::ports::similar_images::SimilarAnalysisRepository;

    let (directory, mut catalog) = fixture();
    for (id, digest) in [("a", 'a'), ("b", 'b'), ("c", 'c')] {
        publish(&mut catalog, id, digest);
    }
    publish_exact(&mut catalog);
    let run = SimilarAnalysisRun {
        run_id: "gallery-groups".into(),
        source_run_id: "gallery-sources".into(),
        started_unix_ms: 1,
    };
    catalog.begin_similarity(&run).unwrap();
    for source in catalog.load_similarity_sources("").unwrap() {
        catalog
            .publish_similarity(
                &run,
                &source.candidate,
                &source.content.unwrap(),
                &SimilarEvidence::Ready {
                    variants: vec![SimilarVariant {
                        candidate: [255, 0, 0, 0],
                        confirmation: [0x55; 32],
                    }],
                },
                &AtomicBool::new(false),
            )
            .unwrap();
    }
    for digest in ['b', 'c'] {
        let reference = SimilarContentKey {
            scheme: crate::domain::exact_duplicates::EXACT_FINGERPRINT_SCHEME.into(),
            digest: digest.to_string().repeat(64),
            byte_length: 20,
        };
        let id = group_id(SIMILARITY_SCHEME, &reference).unwrap();
        catalog
            .connection
            .execute(
                "INSERT INTO similarity_groups VALUES (?1, ?2, ?3, ?4, ?5)",
                rusqlite::params![
                    id,
                    SIMILARITY_SCHEME,
                    reference.scheme,
                    reference.digest,
                    20
                ],
            )
            .unwrap();
        catalog
            .connection
            .execute(
                "INSERT INTO similarity_references SELECT ?1, evidence_id
             FROM similarity_evidence WHERE digest = ?2",
                rusqlite::params![id, reference.digest],
            )
            .unwrap();
        catalog
            .connection
            .execute(
                "INSERT INTO similarity_members SELECT ?1, evidence_id,
               CASE WHEN digest = ?2 THEN 0 ELSE 4 END, ?4
             FROM similarity_evidence WHERE digest IN (?2, ?3)",
                rusqlite::params![id, reference.digest, "a".repeat(64), run.run_id],
            )
            .unwrap();
        catalog
            .connection
            .execute(
                "INSERT INTO similarity_reference_variants
             SELECT ?1, variant, word0, word1, word2, word3, confirmation
             FROM similarity_variants JOIN similarity_references USING(evidence_id)
             WHERE group_id = ?1",
                [&id],
            )
            .unwrap();
    }
    let mut progress = SimilarAnalysisProgress::running(run.run_id.clone());
    progress.phase = SimilarAnalysisPhase::Completed;
    catalog.finish_similarity(&run, &progress).unwrap();
    let query = query(GalleryGroupFilter::Similar);
    let page = catalog
        .load_snapshot(10, &query, "similar", None, None, None)
        .unwrap();
    assert!(page.assets.len() >= 2, "fixture needs overlapping groups");
    assert!(page.assets.iter().all(|entry| entry.location_id == "a"));
    let keys: Vec<_> = page.assets.iter().map(|entry| entry.entry_id()).collect();
    assert_ne!(keys[0], keys[1]);
    assert_semantic_locations(&catalog, &query, &["a", "b", "c"]);
    for (root, expected) in [("root-a", vec!["a", "b"]), ("root-c", vec![])] {
        assert_semantic_locations(
            &catalog,
            &GalleryQuery {
                root_id: Some(root.into()),
                ..query.clone()
            },
            &expected,
        );
    }
    let first = catalog
        .load_snapshot(1, &query, "similar", None, None, None)
        .unwrap();
    let second = catalog
        .load_snapshot(1, &query, "similar", first.next_cursor.as_ref(), None, None)
        .unwrap();
    assert_eq!(second.assets[0].entry_id(), keys[1]);
    let snapshot = catalog
        .load_query_snapshot(
            1,
            &query,
            "similar",
            Some(&GalleryQueryAnchor {
                requested_location_id: "a".into(),
                asset_id: None,
                fallback_ordinal: 0,
                group_key: second.assets[0].group.as_ref().map(|g| g.key.clone()),
            }),
        )
        .unwrap();
    assert_eq!(snapshot.snapshot.assets[0].entry_id(), keys[1]);
    assert!(matches!(
        snapshot.snapshot.assets[0].group.as_ref().unwrap().key,
        GalleryGroupKey::Similar { .. }
    ));
    let mut cleanup = SqliteFileOperationSelection::new(
        SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap(),
        Scope::GalleryCleanup {
            query: Box::new(query),
            catalog_revision: page.revision,
            all_matching: true,
        },
        Arc::new(AtomicBool::new(false)),
    )
    .unwrap();
    assert_eq!(cleanup.prepare().unwrap().file_count, 2);
    assert_eq!(
        cleanup
            .retained_page("")
            .unwrap()
            .iter()
            .map(|m| m.location_id.as_str())
            .collect::<Vec<_>>(),
        ["a"]
    );
    cleanup.freeze_rule(FileOrganizationRule::Direct).unwrap();
}
