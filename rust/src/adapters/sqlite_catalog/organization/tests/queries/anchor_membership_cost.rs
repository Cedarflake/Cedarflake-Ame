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
