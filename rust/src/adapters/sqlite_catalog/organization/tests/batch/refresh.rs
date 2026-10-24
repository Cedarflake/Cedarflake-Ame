use super::*;

#[test]
fn a_created_album_can_receive_the_unchanged_frozen_selection() {
    let (directory, mut catalog) = fixture();
    let (mut batch, _) = open(directory.path(), &catalog, GalleryQuery::default(), true);
    let before = batch.prepare().unwrap();
    let album_id = create_album(&mut catalog, "New destination");
    let refreshed = batch.refresh().unwrap();
    assert!(refreshed.catalog_revision > before.catalog_revision);
    assert_eq!(refreshed.group_count, 3);
    let result = batch
        .apply(
            "new-destination",
            &organize(OrganizationAction::AssignAlbum {
                album_id: album_id.clone(),
            }),
        )
        .unwrap();
    assert_eq!(result.changed_groups, 3);
    for id in ["a", "b", "c"] {
        assert_eq!(decision(&catalog, id).0, Some(album_id.clone()));
    }
    assert_eq!(
        batch.refresh().unwrap_err().code,
        "organization_selection_changed"
    );
    assert_eq!(
        batch
            .apply(
                "new-destination",
                &organize(OrganizationAction::AssignAlbum { album_id })
            )
            .unwrap(),
        result
    );
}

#[test]
fn refreshing_never_expands_an_all_matching_collection() {
    let (directory, mut catalog) = fixture();
    catalog
        .set_organization_favorite("first-favorite", &prepare(&catalog, &["a"]), true)
        .unwrap();
    let query = GalleryQuery {
        collection: GalleryCollection::Favorites,
        ..GalleryQuery::default()
    };
    let (mut batch, _) = open(directory.path(), &catalog, query, true);
    assert_eq!(batch.prepare().unwrap().group_count, 1);
    catalog
        .set_organization_favorite("later-favorite", &prepare(&catalog, &["b"]), true)
        .unwrap();
    assert_eq!(batch.refresh().unwrap().group_count, 1);
    assert_eq!(
        batch
            .apply("remove-original", &favorite(false))
            .unwrap()
            .changed_groups,
        1
    );
    assert!(!decision(&catalog, "a").2);
    assert!(decision(&catalog, "b").2);
}

#[test]
fn refreshing_rejects_intervening_target_decisions_even_when_the_value_returns() {
    let (directory, mut catalog) = fixture();
    catalog
        .set_organization_favorite("initial-favorite", &prepare(&catalog, &["a"]), true)
        .unwrap();
    let (mut batch, _) = open(directory.path(), &catalog, GalleryQuery::default(), false);
    batch
        .append_ids(&[selected(&catalog, "a").asset_id])
        .unwrap();
    batch.prepare().unwrap();
    catalog
        .set_organization_favorite("unfavorite", &prepare(&catalog, &["a"]), false)
        .unwrap();
    catalog
        .set_organization_favorite("refavorite", &prepare(&catalog, &["a"]), true)
        .unwrap();
    assert_eq!(
        batch.refresh().unwrap_err().code,
        "organization_selection_changed"
    );
    assert!(batch.apply("rejected", &favorite(false)).is_err());
    assert!(decision(&catalog, "a").2);
}

#[test]
fn refreshing_rejects_changed_content_and_does_not_replace_unknown_write_authority() {
    let (directory, mut catalog) = fixture();
    let (mut batch, _) = open(directory.path(), &catalog, GalleryQuery::default(), true);
    batch.prepare().unwrap();
    publish(&mut catalog, "a", 'a');
    assert_eq!(
        batch.refresh().unwrap_err().code,
        "organization_selection_changed"
    );
    drop(batch);
    let (mut batch, _) = open(directory.path(), &catalog, GalleryQuery::default(), true);
    batch.prepare().unwrap();
    catalog.connection.execute_batch(
        "CREATE TRIGGER fail_write BEFORE INSERT ON organization_operations BEGIN SELECT RAISE(ABORT, 'failure'); END;"
    ).unwrap();
    assert!(batch.apply("same-operation", &favorite(true)).is_err());
    assert_eq!(
        batch.refresh().unwrap_err().code,
        "organization_selection_changed"
    );
    catalog
        .connection
        .execute_batch("DROP TRIGGER fail_write;")
        .unwrap();
    assert_eq!(
        batch
            .apply("same-operation", &favorite(true))
            .unwrap()
            .changed_groups,
        3
    );
}
