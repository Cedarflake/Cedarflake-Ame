use super::transitions::{change, create_album, decision};
use super::*;

#[test]
fn prepared_album_write_ignores_unrelated_publications_and_independent_favorites() {
    let (_directory, mut catalog) = fixture();
    let album = create_album(&mut catalog, "Destination");
    let prepared = prepare(&catalog, &["a"]);
    change(&mut catalog, "defer-b", &["b"], OrganizationAction::Defer);
    catalog
        .connection
        .execute("UPDATE catalog_state SET revision = revision + 1", [])
        .unwrap();
    catalog
        .set_organization_favorite("favorite-a", &prepared, true)
        .unwrap();
    let action = OrganizationAction::AssignAlbum {
        album_id: album.clone(),
    };
    let receipt = catalog
        .change_organization("assign-a", &prepared, &action)
        .unwrap();
    assert_eq!(
        decision(&catalog, "a"),
        (Some(album), "organized".into(), true)
    );
    assert_eq!(
        catalog
            .change_organization("assign-a", &prepared, &action)
            .unwrap(),
        receipt
    );
}

#[test]
fn prepared_decision_rejects_a_later_change_to_the_same_field() {
    let (_directory, mut catalog) = fixture();
    let prepared = prepare(&catalog, &["a"]);
    change(&mut catalog, "defer-a", &["a"], OrganizationAction::Defer);
    assert_eq!(
        catalog
            .change_organization("ignore-a", &prepared, &OrganizationAction::Ignore)
            .unwrap_err()
            .code,
        "organization_selection_changed"
    );
    catalog
        .set_organization_favorite("favorite-a", &prepared, true)
        .unwrap();
    assert_eq!(
        catalog
            .set_organization_favorite("unfavorite-a", &prepared, false)
            .unwrap_err()
            .code,
        "organization_selection_changed"
    );
    assert_eq!(decision(&catalog, "a"), (None, "deferred".into(), true));
    assert_eq!(
        scalar(
            &catalog,
            "SELECT COUNT(*) FROM organization_operations WHERE operation_id IN ('ignore-a', 'unfavorite-a')"
        ),
        0
    );
}
