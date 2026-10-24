use super::transitions::{change, create_album, decision};
use super::*;

fn revision(catalog: &SqliteCatalog) -> u64 {
    catalog
        .load_organization_albums(None)
        .unwrap()
        .organization_revision
}

fn undo(
    catalog: &mut SqliteCatalog,
    original: &str,
) -> Result<OrganizationWriteReceipt, ScanError> {
    catalog.undo_organization_operation(&format!("undo-{original}"), original, revision(catalog))
}

#[test]
fn undo_restores_only_the_affected_field_and_survives_reopen_and_retry() {
    let (directory, mut catalog) = fixture();
    publish(&mut catalog, "a", 'a');
    publish(&mut catalog, "c", 'a');
    let album = create_album(&mut catalog, "Album");
    change(
        &mut catalog,
        "assign",
        &["a"],
        OrganizationAction::AssignAlbum { album_id: album },
    );
    let prepared = prepare(&catalog, &["c"]);
    catalog
        .set_organization_favorite("favorite", &prepared, true)
        .unwrap();
    let expected_revision = revision(&catalog);
    let receipt = undo(&mut catalog, "assign").unwrap();
    assert_eq!(receipt.changed_groups, 1);
    for id in ["a", "c"] {
        assert_eq!(decision(&catalog, id), (None, "pending".into(), true));
    }
    drop(catalog);
    let mut catalog = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
    assert_eq!(
        catalog
            .undo_organization_operation("undo-assign", "assign", expected_revision)
            .unwrap(),
        receipt
    );
    assert_eq!(
        catalog
            .undo_organization_operation("undo-assign", "favorite", expected_revision)
            .unwrap_err()
            .code,
        "organization_operation_conflict"
    );
    undo(&mut catalog, "favorite").unwrap();
    assert_eq!(decision(&catalog, "c"), (None, "pending".into(), false));
}

#[test]
fn undo_rejects_newer_decisions_and_content_replacement_without_consuming_the_request() {
    let (_directory, mut catalog) = fixture();
    change(&mut catalog, "ignore", &["a"], OrganizationAction::Ignore);
    change(&mut catalog, "defer", &["a"], OrganizationAction::Defer);
    assert_eq!(
        undo(&mut catalog, "ignore").unwrap_err().code,
        "organization_undo_unavailable"
    );
    catalog.connection.execute("UPDATE asset_locations SET source_generation = source_generation + 100 WHERE location_id = 'a'", []).unwrap();
    assert_eq!(
        undo(&mut catalog, "defer").unwrap_err().code,
        "organization_undo_unavailable"
    );
    assert_eq!(
        scalar(
            &catalog,
            "SELECT COUNT(*) FROM organization_operations WHERE operation_kind = 'undo'"
        ),
        0
    );
}

#[test]
fn undo_album_deletion_restores_all_assignments_and_keeps_later_favorites() {
    let (_directory, mut catalog) = fixture();
    let album = create_album(&mut catalog, "Album");
    change(
        &mut catalog,
        "assign",
        &["a", "b"],
        OrganizationAction::AssignAlbum {
            album_id: album.clone(),
        },
    );
    catalog
        .delete_organization_album("delete", revision(&catalog), &album)
        .unwrap();
    let prepared = prepare(&catalog, &["a"]);
    catalog
        .set_organization_favorite("favorite", &prepared, true)
        .unwrap();
    assert_eq!(undo(&mut catalog, "delete").unwrap().changed_groups, 2);
    assert_eq!(
        decision(&catalog, "a"),
        (Some(album.clone()), "organized".into(), true)
    );
    assert_eq!(
        decision(&catalog, "b"),
        (Some(album.clone()), "organized".into(), false)
    );
    assert_eq!(
        catalog.load_organization_albums(None).unwrap().albums[0].album_id,
        album
    );
}

#[test]
fn undo_does_not_restore_a_retired_destination_or_discard_later_album_members() {
    let (_directory, mut catalog) = fixture();
    let first = create_album(&mut catalog, "First");
    let second = create_album(&mut catalog, "Second");
    change(
        &mut catalog,
        "assign",
        &["b"],
        OrganizationAction::AssignAlbum {
            album_id: first.clone(),
        },
    );
    change(
        &mut catalog,
        "move",
        &["b"],
        OrganizationAction::AssignAlbum {
            album_id: second.clone(),
        },
    );
    catalog
        .delete_organization_album("delete-first", revision(&catalog), &first)
        .unwrap();
    assert_eq!(
        undo(&mut catalog, "move").unwrap_err().code,
        "organization_undo_unavailable"
    );
    assert_eq!(
        undo(&mut catalog, "create-Second").unwrap_err().code,
        "organization_undo_unavailable"
    );
    assert_eq!(
        decision(&catalog, "b"),
        (Some(second), "organized".into(), false)
    );
    assert_eq!(
        catalog.load_organization_albums(None).unwrap().albums.len(),
        1
    );
}

#[test]
fn undo_album_metadata_checks_later_names_and_keeps_creation_idempotent() {
    let (_directory, mut catalog) = fixture();
    let album = create_album(&mut catalog, "Original");
    catalog
        .rename_organization_album("rename", revision(&catalog), &album, "Renamed")
        .unwrap();
    let collision = catalog
        .create_organization_album("name-collision", revision(&catalog), "Original")
        .unwrap()
        .album_id;
    assert_eq!(
        undo(&mut catalog, "rename").unwrap_err().code,
        "organization_album_name_exists"
    );
    catalog
        .delete_organization_album("remove-collision", revision(&catalog), &collision)
        .unwrap();
    undo(&mut catalog, "rename").unwrap();
    assert_eq!(
        catalog.load_organization_albums(None).unwrap().albums[0].name,
        "Original"
    );
    assert_eq!(
        undo(&mut catalog, "create-Original").unwrap_err().code,
        "organization_undo_unavailable"
    );
    let fresh = create_album(&mut catalog, "Fresh");
    undo(&mut catalog, "create-Fresh").unwrap();
    assert!(
        !catalog
            .load_organization_albums(None)
            .unwrap()
            .albums
            .iter()
            .any(|entry| entry.album_id == fresh)
    );
}

#[test]
fn missing_targets_and_partial_failures_cannot_publish_a_partial_undo() {
    let (_directory, mut catalog) = fixture();
    change(
        &mut catalog,
        "ignore",
        &["a", "b"],
        OrganizationAction::Ignore,
    );
    catalog
        .connection
        .execute_batch(
            "CREATE TRIGGER reject_undo BEFORE UPDATE OF disposition ON organization_records
         WHEN NEW.disposition = 'pending' BEGIN SELECT RAISE(ABORT, 'injected undo failure'); END;",
        )
        .unwrap();
    assert!(undo(&mut catalog, "ignore").is_err());
    for id in ["a", "b"] {
        assert_eq!(decision(&catalog, id), (None, "ignored".into(), false));
    }
    catalog.connection.execute_batch("DROP TRIGGER reject_undo;
        DELETE FROM organization_records WHERE record_id IN (SELECT binding.record_id FROM organization_bindings AS binding
          JOIN asset_locations AS location ON location.asset_id = binding.asset_id AND location.source_generation = binding.source_generation
          WHERE location.location_id = 'b');").unwrap();
    assert_eq!(
        undo(&mut catalog, "ignore").unwrap_err().code,
        "organization_undo_unavailable"
    );
    assert_eq!(decision(&catalog, "a"), (None, "ignored".into(), false));
    assert_eq!(
        scalar(
            &catalog,
            "SELECT COUNT(*) FROM organization_operations WHERE operation_kind = 'undo'"
        ),
        0
    );
}
