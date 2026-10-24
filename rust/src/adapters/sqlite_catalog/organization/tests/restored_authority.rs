use super::transitions::{change, create_album, decision};
use super::*;

fn mark_restored(catalog: &SqliteCatalog, location_id: &str) {
    catalog.connection.execute(
        "INSERT INTO organization_restored_fields
         SELECT record.record_id, field.name,
           CASE field.name WHEN 'favorite' THEN record.favorite_revision ELSE record.organization_revision END
         FROM organization_records AS record
         JOIN organization_bindings AS binding USING(record_id)
         JOIN asset_locations AS location
           ON location.asset_id = binding.asset_id AND location.source_generation = binding.source_generation
         CROSS JOIN (SELECT 'favorite' AS name UNION ALL SELECT 'organization') AS field
         WHERE location.location_id = ?1 AND CASE field.name WHEN 'favorite'
           THEN record.favorite_revision ELSE record.organization_revision END > 0",
        [location_id],
    ).unwrap();
}

fn merge(catalog: &mut SqliteCatalog) {
    super::queries::publish_batch(catalog, &["a", "c"], 'a');
}

#[test]
fn archive_restored_fields_do_not_overwrite_existing_local_album_or_unfavorite() {
    let (_directory, mut catalog) = fixture();
    let local = create_album(&mut catalog, "Current");
    let archived = create_album(&mut catalog, "Archived");
    change(
        &mut catalog,
        "local-album",
        &["a"],
        OrganizationAction::AssignAlbum {
            album_id: local.clone(),
        },
    );
    for (operation, favorite) in [("local-favorite", true), ("local-unfavorite", false)] {
        let target = prepare(&catalog, &["a"]);
        catalog
            .set_organization_favorite(operation, &target, favorite)
            .unwrap();
    }
    change(
        &mut catalog,
        "archive-album",
        &["c"],
        OrganizationAction::AssignAlbum { album_id: archived },
    );
    let target = prepare(&catalog, &["c"]);
    catalog
        .set_organization_favorite("archive-favorite", &target, true)
        .unwrap();
    mark_restored(&catalog, "c");
    merge(&mut catalog);
    assert_eq!(
        decision(&catalog, "a"),
        (Some(local), "organized".into(), false)
    );
    assert_eq!(decision(&catalog, "a"), decision(&catalog, "c"));
}

#[test]
fn local_ignored_progress_survives_a_later_archive_album_binding() {
    let (_directory, mut catalog) = fixture();
    let album = create_album(&mut catalog, "Archived");
    change(&mut catalog, "ignore", &["a"], OrganizationAction::Ignore);
    change(
        &mut catalog,
        "archive-album",
        &["c"],
        OrganizationAction::AssignAlbum { album_id: album },
    );
    mark_restored(&catalog, "c");
    merge(&mut catalog);
    assert_eq!(decision(&catalog, "c"), (None, "ignored".into(), false));
}

#[test]
fn restored_album_conflicts_keep_their_authority_until_an_explicit_local_choice() {
    let (_directory, mut catalog) = fixture();
    let first = create_album(&mut catalog, "First");
    let second = create_album(&mut catalog, "Second");
    change(
        &mut catalog,
        "first",
        &["a"],
        OrganizationAction::AssignAlbum {
            album_id: first.clone(),
        },
    );
    change(
        &mut catalog,
        "second",
        &["c"],
        OrganizationAction::AssignAlbum { album_id: second },
    );
    mark_restored(&catalog, "a");
    mark_restored(&catalog, "c");
    merge(&mut catalog);
    assert_eq!(decision(&catalog, "a"), (None, "deferred".into(), false));
    assert_eq!(
        scalar(&catalog, "SELECT COUNT(*) FROM organization_conflicts"),
        2
    );
    change(
        &mut catalog,
        "resolve",
        &["c"],
        OrganizationAction::AssignAlbum {
            album_id: first.clone(),
        },
    );
    assert_eq!(
        decision(&catalog, "a"),
        (Some(first), "organized".into(), false)
    );
    let marker_is_current = scalar(&catalog,
        "SELECT COUNT(*) FROM organization_restored_fields AS retained
         JOIN organization_record_groups AS member ON member.record_id = retained.record_id AND member.group_id = member.record_id
         JOIN organization_records AS record ON record.record_id = member.group_id
         WHERE retained.field = 'organization' AND retained.applied_revision = record.organization_revision",
    );
    assert_eq!(marker_is_current, 0);
}
