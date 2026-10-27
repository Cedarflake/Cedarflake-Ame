use crate::adapters::sqlite_catalog::gallery::resolve_gallery_location_anchor;
use crate::domain::organization_queue::OrganizationQueue;

use super::fingerprint_cost::bounded_read;
use super::*;

#[test]
fn ineligible_anchor_does_not_resolve_unrelated_organization_groups() {
    let (_directory, mut catalog) = fixture();
    let first_album = create_album(&mut catalog, "First");
    let other_album = create_album(&mut catalog, "Other");
    assign(&mut catalog, "assign-a", &["a"], &first_album);
    assign(&mut catalog, "assign-b", &["b"], &other_album);
    let prepared = prepare(&catalog, &["b"]);
    catalog
        .set_organization_favorite("favorite-b", &prepared, true)
        .unwrap();
    catalog
        .connection
        .execute_batch(
            "WITH RECURSIVE sequence(value) AS (
               VALUES(1) UNION ALL SELECT value + 1 FROM sequence WHERE value < 25000
             )
             INSERT INTO asset_locations(scan_id, asset_id, location_id, root_id,
               absolute_path, relative_path, preview_path, file_size, modified_unix_ms,
               width, height, preview_status, source_generation, source_revision_token)
             SELECT current.scan_id, current.asset_id, 'alias-' || value, current.root_id,
               current.absolute_path, current.relative_path, '', current.file_size, 0,
               40, 50, 'pending', current.source_generation, current.source_revision_token
             FROM sequence CROSS JOIN asset_locations AS current
             WHERE current.scan_id = 'scan-a' AND current.location_id = 'b';",
        )
        .unwrap();
    for query in [
        album_query(&other_album),
        GalleryQuery {
            collection: GalleryCollection::Favorites,
            duplicate_mode: GalleryDuplicateMode::Merged,
            ..GalleryQuery::default()
        },
        GalleryQuery {
            organization_queue: Some(OrganizationQueue::Pending),
            ..GalleryQuery::default()
        },
    ] {
        assert_ineligible_anchor(&mut catalog, &query);
    }
    for (operation, action, queue) in [
        (
            "defer-b",
            OrganizationAction::Defer,
            OrganizationQueue::Deferred,
        ),
        (
            "ignore-b",
            OrganizationAction::Ignore,
            OrganizationQueue::Ignored,
        ),
    ] {
        change(&mut catalog, operation, &["b"], action);
        assert_ineligible_anchor(
            &mut catalog,
            &GalleryQuery {
                organization_queue: Some(queue),
                ..GalleryQuery::default()
            },
        );
    }
}

fn assert_ineligible_anchor(catalog: &mut SqliteCatalog, query: &GalleryQuery) {
    let revision = scalar(catalog, "SELECT revision FROM catalog_state");
    let (resolution, predecessor) = bounded_read(catalog, |catalog| {
        let transaction = catalog.connection.transaction().map_err(database_error)?;
        let resolved = resolve_gallery_location_anchor(
            &transaction,
            revision,
            query,
            "ineligible-anchor",
            "a",
            10,
        )?;
        transaction.commit().map_err(database_error)?;
        Ok(resolved)
    });
    assert_eq!(resolution.location_id, None);
    assert_eq!(resolution.ordinal, None);
    assert!(predecessor.is_none());
}

#[test]
fn anchor_membership_does_not_require_the_requested_copy_in_the_physical_scope() {
    let (_directory, mut catalog, album) = grouped();
    let prepared = prepare(&catalog, &["a"]);
    catalog
        .set_organization_favorite("favorite-group", &prepared, true)
        .unwrap();
    let requested = selected(&catalog, "a");
    for collection in [album.collection, GalleryCollection::Favorites] {
        let query = GalleryQuery {
            collection,
            root_id: Some("root-c".into()),
            search_text: "c.png".into(),
            duplicate_mode: GalleryDuplicateMode::Merged,
            ..GalleryQuery::default()
        };
        let page = catalog
            .load_snapshot_around_asset(10, &query, "other-copy", "a", &requested.asset_id, 0)
            .unwrap();
        assert_eq!(page.assets.len(), 1);
        assert_eq!(page.assets[0].location_id, "c");
        assert_eq!(
            page.query_anchor_resolution.unwrap().location_id.as_deref(),
            Some("c"),
        );
    }
}

#[test]
fn resolved_anchor_keeps_preferred_copy_inside_the_complete_physical_scope() {
    let (_directory, mut catalog, album) = grouped();
    let prepared = prepare(&catalog, &["a"]);
    catalog
        .set_organization_favorite("favorite-group", &prepared, true)
        .unwrap();
    let preferred = selected(&catalog, "c");
    catalog
        .set_duplicate_preference(
            &ExactDuplicateReviewKey {
                digest: "a".repeat(64),
                byte_length: 20,
            },
            Some(&ExactDuplicatePreference {
                location_id: preferred.location_id,
                asset_id: preferred.asset_id,
                source_generation: preferred.source_generation,
            }),
        )
        .unwrap();
    catalog
        .connection
        .execute(
            "UPDATE asset_locations SET relative_path = 'Album/c.png',
               parent_relative_path = 'Album' WHERE location_id = 'c'",
            [],
        )
        .unwrap();
    for collection in [
        GalleryCollection::Library,
        album.collection,
        GalleryCollection::Favorites,
    ] {
        let query = GalleryQuery {
            collection,
            duplicate_mode: GalleryDuplicateMode::Merged,
            ..GalleryQuery::default()
        };
        for (scoped, expected) in [
            (query.clone(), Some("c")),
            (
                GalleryQuery {
                    root_id: Some("root-a".into()),
                    ..query.clone()
                },
                Some("a"),
            ),
            (
                GalleryQuery {
                    root_id: Some("root-c".into()),
                    folder_relative_path: Some("Album".into()),
                    include_descendants: false,
                    search_text: "c.png".into(),
                    ..query.clone()
                },
                Some("c"),
            ),
            (
                GalleryQuery {
                    root_id: Some("root-c".into()),
                    folder_relative_path: Some("Missing".into()),
                    ..query.clone()
                },
                None,
            ),
            (
                GalleryQuery {
                    search_text: "b.png".into(),
                    ..query
                },
                None,
            ),
        ] {
            let snapshot = catalog
                .load_snapshot_around_location(1, &scoped, "scoped-anchor", "a")
                .unwrap();
            let resolution = snapshot.query_anchor_resolution.unwrap();
            assert_eq!(resolution.location_id.as_deref(), expected);
        }
    }
}

#[test]
fn resolved_anchor_rechecks_asset_identity_and_changed_collection_membership() {
    let (_directory, mut catalog, query) = grouped();
    let original = selected(&catalog, "a");
    let other = selected(&catalog, "b");
    let snapshot = catalog
        .load_snapshot_around_asset(1, &query, "changed-asset", "a", &other.asset_id, 0)
        .unwrap();
    let resolution = snapshot.query_anchor_resolution.unwrap();
    assert_eq!(resolution.location_id.as_deref(), Some("b"));
    assert_eq!(resolution.ordinal, Some(1));

    change(
        &mut catalog,
        "remove-anchored-group",
        &["a"],
        OrganizationAction::RemoveAlbum,
    );
    let snapshot = catalog
        .load_snapshot_around_asset(1, &query, "removed-anchor", "a", &original.asset_id, 0)
        .unwrap();
    let resolution = snapshot.query_anchor_resolution.unwrap();
    assert_eq!(resolution.location_id.as_deref(), Some("b"));
    assert_eq!(resolution.ordinal, Some(0));
    assert_eq!(snapshot.assets[0].location_id, "b");
}

#[test]
fn resolved_anchor_distinguishes_equal_location_ids_in_different_roots() {
    let (_directory, mut catalog) = fixture();
    crate::adapters::sqlite_catalog::tests::publish_gallery_fixture(
        &mut catalog,
        "scan-c-next",
        "root-c",
        "D:\\Fixture",
        &[("c", None, 3), ("d", None, 2)],
    );
    publish_batch(&mut catalog, &["a", "c"], 'a');
    let requested = selected(&catalog, "c");
    catalog
        .connection
        .execute(
            "UPDATE asset_locations SET location_id = 'a' WHERE root_id = 'root-c'
               AND scan_id = 'scan-c-next' AND location_id = 'c'",
            [],
        )
        .unwrap();
    let query = GalleryQuery {
        root_id: Some("root-c".into()),
        duplicate_mode: GalleryDuplicateMode::Merged,
        sort_key: GallerySortKey::ModifiedTime,
        sort_direction: GallerySortDirection::Ascending,
        ..GalleryQuery::default()
    };
    let snapshot = catalog
        .load_snapshot_around_asset(1, &query, "shared-id", "a", &requested.asset_id, 0)
        .unwrap();
    let resolution = snapshot.query_anchor_resolution.unwrap();
    assert_eq!(resolution.location_id.as_deref(), Some("a"));
    assert_eq!(resolution.ordinal, Some(1));
    assert_eq!(snapshot.assets[0].asset_id, requested.asset_id);
    assert_eq!(snapshot.assets[0].root_id, "root-c");
}
