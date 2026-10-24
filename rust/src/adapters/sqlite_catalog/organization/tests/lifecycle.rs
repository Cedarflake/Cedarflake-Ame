use super::transitions::{change, create_album, decision};
use super::*;

#[test]
fn removing_the_final_source_retires_all_lineage_aliases_together() {
    let (_directory, mut catalog) = fixture();
    change(
        &mut catalog,
        "ignore",
        &["a", "b"],
        OrganizationAction::Ignore,
    );
    catalog.connection.execute_batch(
        "UPDATE organization_record_groups SET group_id = (SELECT MIN(record_id) FROM organization_records)"
    ).unwrap();
    catalog.unregister_root("root-a").unwrap();
    assert_eq!(
        scalar(&catalog, "SELECT COUNT(*) FROM organization_records"),
        0
    );
    assert_eq!(
        scalar(&catalog, "SELECT COUNT(*) FROM organization_record_groups"),
        0
    );
}

#[test]
fn removing_a_source_forgets_only_orphaned_groups_and_never_resurrects_them_on_reimport() {
    let (directory, mut catalog) = fixture();
    publish(&mut catalog, "a", 'a');
    publish(&mut catalog, "c", 'a');
    let album = create_album(&mut catalog, "Retained");
    change(
        &mut catalog,
        "assign",
        &["a", "b"],
        OrganizationAction::AssignAlbum {
            album_id: album.clone(),
        },
    );
    let prepared = prepare(&catalog, &["c"]);
    catalog
        .set_organization_favorite("favorite", &prepared, true)
        .unwrap();
    assert!(catalog.unregister_root("root-a").unwrap());
    assert_eq!(
        decision(&catalog, "c"),
        (Some(album), "organized".into(), true)
    );
    assert_eq!(
        scalar(&catalog, "SELECT COUNT(*) FROM organization_records"),
        1
    );
    assert_eq!(
        scalar(
            &catalog,
            "SELECT COUNT(*) FROM organization_sources WHERE root_id = 'root-a'"
        ),
        0
    );
    let revision = catalog
        .load_organization_albums(None)
        .unwrap()
        .organization_revision;
    assert_eq!(
        catalog
            .undo_organization_operation("undo-assign", "assign", revision)
            .unwrap_err()
            .code,
        "organization_undo_unavailable"
    );
    assert!(catalog.unregister_root("root-c").unwrap());
    assert!(!catalog.unregister_root("root-c").unwrap());
    for table in [
        "organization_records",
        "organization_bindings",
        "organization_sources",
        "organization_record_groups",
        "organization_operation_changes",
    ] {
        assert_eq!(
            scalar(&catalog, &format!("SELECT COUNT(*) FROM {table}")),
            0
        );
    }
    drop(catalog);
    let mut catalog = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
    super::super::super::tests::publish_gallery_fixture(
        &mut catalog,
        "reimport",
        "root-a",
        "C:\\Fixture",
        &[("a", None, 1)],
    );
    let prepared = prepare(&catalog, &["a"]);
    catalog
        .set_organization_favorite("reimport-no-favorite", &prepared, false)
        .unwrap();
    assert_eq!(decision(&catalog, "a"), (None, "pending".into(), false));
    assert_eq!(
        catalog.load_organization_albums(None).unwrap().albums.len(),
        1
    );
}

#[test]
fn failed_source_removal_rolls_back_decisions_associations_and_undo_history() {
    let (_directory, mut catalog) = fixture();
    change(&mut catalog, "defer", &["b"], OrganizationAction::Defer);
    let revision = catalog
        .load_organization_albums(None)
        .unwrap()
        .organization_revision;
    catalog
        .connection
        .execute_batch(
            "CREATE TRIGGER reject_source_removal BEFORE DELETE ON library_roots
         BEGIN SELECT RAISE(ABORT, 'injected source removal failure'); END;",
        )
        .unwrap();
    assert!(catalog.unregister_root("root-a").is_err());
    assert_eq!(decision(&catalog, "b"), (None, "deferred".into(), false));
    assert_eq!(
        scalar(&catalog, "SELECT COUNT(*) FROM organization_sources"),
        1
    );
    assert_eq!(
        catalog
            .load_organization_albums(None)
            .unwrap()
            .organization_revision,
        revision
    );
    catalog
        .undo_organization_operation("undo-defer", "defer", revision)
        .unwrap();
    assert_eq!(decision(&catalog, "b"), (None, "pending".into(), false));
}

#[test]
fn consolidation_transfers_decisions_before_the_covered_root_is_retired() {
    let (directory, mut catalog) = fixture();
    let album = create_album(&mut catalog, "Album");
    change(
        &mut catalog,
        "assign",
        &["a"],
        OrganizationAction::AssignAlbum {
            album_id: album.clone(),
        },
    );
    let transaction = catalog.connection.transaction().unwrap();
    super::super::lifecycle::transfer_source(&transaction, "root-a", "root-c").unwrap();
    super::super::super::root_unregistration::unregister_root(&transaction, "root-a").unwrap();
    transaction.commit().unwrap();
    assert_eq!(
        scalar(&catalog, "SELECT COUNT(*) FROM organization_records"),
        1
    );
    assert_eq!(
        scalar(
            &catalog,
            "SELECT COUNT(*) FROM organization_sources WHERE root_id = 'root-c'"
        ),
        1
    );
    drop(catalog);
    let mut catalog = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
    assert_eq!(
        catalog.load_organization_albums(None).unwrap().albums[0].album_id,
        album
    );
    assert!(catalog.unregister_root("root-c").unwrap());
    assert_eq!(
        scalar(&catalog, "SELECT COUNT(*) FROM organization_records"),
        0
    );
}

#[test]
fn derived_location_loss_keeps_decisions_until_the_user_removes_the_source() {
    let (directory, mut catalog) = fixture();
    change(&mut catalog, "ignore", &["a"], OrganizationAction::Ignore);
    catalog
        .connection
        .execute("DELETE FROM asset_locations WHERE root_id = 'root-a'", [])
        .unwrap();
    drop(catalog);
    let mut catalog = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
    assert_eq!(
        scalar(
            &catalog,
            "SELECT COUNT(*) FROM organization_records WHERE disposition = 'ignored'"
        ),
        1
    );
    assert_eq!(
        scalar(&catalog, "SELECT COUNT(*) FROM organization_sources"),
        1
    );
    catalog.unregister_root("root-a").unwrap();
    assert_eq!(
        scalar(&catalog, "SELECT COUNT(*) FROM organization_records"),
        0
    );
}
