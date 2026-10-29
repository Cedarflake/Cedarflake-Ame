use super::transitions::{change, create_album, decision};
use super::*;

fn revision(catalog: &SqliteCatalog) -> u64 {
    catalog
        .load_organization_albums(None)
        .unwrap()
        .organization_revision
}

fn publish_batch(catalog: &mut SqliteCatalog, ids: &[&str]) -> Result<(), ScanError> {
    let generations = ids
        .iter()
        .map(|id| selected(catalog, id).source_generation)
        .collect::<Vec<_>>();
    catalog.publish_exact_analysis_batch("organization-fixture", &generations)
}

fn assign(catalog: &mut SqliteCatalog, id: &str, album: &str) {
    change(
        catalog,
        &format!("assign-{id}"),
        &[id],
        OrganizationAction::AssignAlbum {
            album_id: album.into(),
        },
    );
}

fn undo(
    catalog: &mut SqliteCatalog,
    original: &str,
) -> Result<OrganizationWriteReceipt, ScanError> {
    catalog
        .undo_organization_operation(&format!("undo-{original}"), original, revision(catalog))
        .map(|receipt| receipt.operation)
}

fn conflict_fixture() -> (tempfile::TempDir, SqliteCatalog, String, String) {
    let (directory, mut catalog) = fixture();
    let first = create_album(&mut catalog, "First");
    let second = create_album(&mut catalog, "Second");
    assign(&mut catalog, "a", &first);
    assign(&mut catalog, "c", &second);
    publish(&mut catalog, "a", 'a');
    publish(&mut catalog, "c", 'a');
    (directory, catalog, first, second)
}

#[test]
fn late_cross_root_conflict_publishes_atomically_and_survives_reopen_without_new_user_operations() {
    let (directory, mut catalog, first, second) = conflict_fixture();
    assert_eq!(
        decision(&catalog, "a"),
        (Some(first), "organized".into(), false)
    );
    assert_eq!(
        decision(&catalog, "c"),
        (Some(second), "organized".into(), false)
    );
    let operations = scalar(&catalog, "SELECT COUNT(*) FROM organization_operations");
    let organization_revision = revision(&catalog);
    let catalog_revision = scalar(&catalog, "SELECT revision FROM catalog_state");
    publish_batch(&mut catalog, &["a", "c"]).unwrap();
    for id in ["a", "c"] {
        assert_eq!(decision(&catalog, id), (None, "deferred".into(), false));
    }
    assert_eq!(
        scalar(&catalog, "SELECT COUNT(*) FROM organization_conflicts"),
        2
    );
    assert_eq!(
        scalar(
            &catalog,
            "SELECT COUNT(DISTINCT group_id) FROM organization_record_groups"
        ),
        1
    );
    assert_eq!(
        scalar(&catalog, "SELECT revision FROM catalog_state"),
        catalog_revision + 1
    );
    assert_eq!(revision(&catalog), organization_revision);
    assert_eq!(
        scalar(&catalog, "SELECT COUNT(*) FROM organization_operations"),
        operations
    );
    publish_batch(&mut catalog, &["a", "c"]).unwrap();
    assert_eq!(
        scalar(&catalog, "SELECT revision FROM catalog_state"),
        catalog_revision + 1
    );
    drop(catalog);
    let catalog = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
    assert_eq!(decision(&catalog, "c"), (None, "deferred".into(), false));
}

#[test]
fn agreeing_albums_merge_with_latest_favorite_intent_and_removal_covers_every_copy() {
    let (_directory, mut catalog) = fixture();
    let album = create_album(&mut catalog, "Common");
    assign(&mut catalog, "a", &album);
    assign(&mut catalog, "c", &album);
    let prepared = prepare(&catalog, &["c"]);
    catalog
        .set_organization_favorite("favorite", &prepared, true)
        .unwrap();
    publish(&mut catalog, "a", 'a');
    publish(&mut catalog, "c", 'a');
    publish_batch(&mut catalog, &["a", "c"]).unwrap();
    for id in ["a", "c"] {
        assert_eq!(
            decision(&catalog, id),
            (Some(album.clone()), "organized".into(), true)
        );
    }
    assert_eq!(
        scalar(&catalog, "SELECT COUNT(*) FROM organization_conflicts"),
        0
    );
    assert_eq!(
        undo(&mut catalog, "assign-a").unwrap_err().code,
        "organization_undo_unavailable"
    );
    change(
        &mut catalog,
        "remove-group",
        &["c"],
        OrganizationAction::RemoveAlbum,
    );
    for id in ["a", "c"] {
        assert_eq!(decision(&catalog, id), (None, "ignored".into(), true));
    }
    undo(&mut catalog, "remove-group").unwrap();
    for id in ["a", "c"] {
        assert_eq!(
            decision(&catalog, id),
            (Some(album.clone()), "organized".into(), true)
        );
    }
}

#[test]
fn resolving_conflict_and_undo_restore_competing_choices_without_restoring_two_memberships() {
    let (_directory, mut catalog, first, _) = conflict_fixture();
    publish_batch(&mut catalog, &["a", "c"]).unwrap();
    let prepared = prepare(&catalog, &["a"]);
    assert_eq!(
        catalog
            .change_organization("ignore-conflict", &prepared, &OrganizationAction::Ignore)
            .unwrap_err()
            .code,
        "organization_conflict_unresolved"
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
        scalar(&catalog, "SELECT COUNT(*) FROM organization_conflicts"),
        0
    );
    assert_eq!(
        decision(&catalog, "a"),
        (Some(first), "organized".into(), false)
    );
    undo(&mut catalog, "resolve").unwrap();
    assert_eq!(decision(&catalog, "a"), (None, "deferred".into(), false));
    assert_eq!(decision(&catalog, "c"), (None, "deferred".into(), false));
    assert_eq!(
        scalar(&catalog, "SELECT COUNT(*) FROM organization_conflicts"),
        2
    );
}

#[test]
fn retiring_a_conflicting_album_keeps_the_conflict_and_retiring_a_confirmed_album_ignores_the_group()
 {
    let (directory, mut catalog, first, second) = conflict_fixture();
    publish_batch(&mut catalog, &["a", "c"]).unwrap();
    let receipt = catalog
        .delete_organization_album("delete-conflict-choice", revision(&catalog), &second)
        .unwrap();
    assert_eq!(receipt.changed_groups, 0);
    assert_eq!(decision(&catalog, "c"), (None, "deferred".into(), false));
    assert_eq!(
        scalar(&catalog, "SELECT COUNT(*) FROM organization_conflicts"),
        2
    );
    drop(catalog);
    let mut catalog = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
    change(
        &mut catalog,
        "resolve",
        &["a"],
        OrganizationAction::AssignAlbum {
            album_id: first.clone(),
        },
    );
    assert_eq!(
        catalog
            .delete_organization_album("delete-assigned", revision(&catalog), &first)
            .unwrap()
            .changed_groups,
        1
    );
    assert_eq!(decision(&catalog, "c"), (None, "ignored".into(), false));
    undo(&mut catalog, "delete-assigned").unwrap();
    assert_eq!(
        decision(&catalog, "a"),
        (Some(first), "organized".into(), false)
    );
}

#[test]
fn a_new_copy_inherits_existing_work_and_follows_guarded_undo() {
    let (_directory, mut catalog) = fixture();
    publish(&mut catalog, "a", 'a');
    let album = create_album(&mut catalog, "Existing");
    assign(&mut catalog, "a", &album);
    publish(&mut catalog, "c", 'a');
    publish_batch(&mut catalog, &["a", "c"]).unwrap();
    assert_eq!(
        decision(&catalog, "c"),
        (Some(album), "organized".into(), false)
    );
    assert_eq!(
        scalar(&catalog, "SELECT COUNT(*) FROM organization_records"),
        1
    );
    undo(&mut catalog, "assign-a").unwrap();
    for id in ["a", "c"] {
        assert_eq!(decision(&catalog, id), (None, "pending".into(), false));
    }
}

#[test]
fn content_split_detaches_old_work_and_matching_another_group_adopts_its_decision() {
    let (_directory, mut catalog) = fixture();
    publish(&mut catalog, "a", 'a');
    publish(&mut catalog, "c", 'a');
    publish(&mut catalog, "b", 'b');
    let original_album = create_album(&mut catalog, "Original");
    let other_album = create_album(&mut catalog, "Other");
    assign(&mut catalog, "a", &original_album);
    assign(&mut catalog, "b", &other_album);
    catalog.connection.execute("UPDATE asset_locations SET source_generation = source_generation + 100 WHERE location_id = 'c'", []).unwrap();
    publish(&mut catalog, "c", 'c');
    publish_batch(&mut catalog, &["c"]).unwrap();
    let prepared = prepare(&catalog, &["c"]);
    catalog
        .set_organization_favorite("new-content-default", &prepared, false)
        .unwrap();
    assert_eq!(decision(&catalog, "c"), (None, "pending".into(), false));
    assert_eq!(
        decision(&catalog, "a"),
        (Some(original_album.clone()), "organized".into(), false)
    );
    catalog.connection.execute("UPDATE asset_locations SET source_generation = source_generation + 100 WHERE location_id = 'c'", []).unwrap();
    publish(&mut catalog, "c", 'b');
    publish_batch(&mut catalog, &["c"]).unwrap();
    assert_eq!(
        decision(&catalog, "c"),
        (Some(other_album), "organized".into(), false)
    );
    assert_eq!(
        decision(&catalog, "a"),
        (Some(original_album), "organized".into(), false)
    );
}

#[test]
fn failed_group_publication_rolls_back_gallery_and_decisions_before_retry() {
    let (_directory, mut catalog, first, second) = conflict_fixture();
    let before = scalar(&catalog, "SELECT revision FROM catalog_state");
    catalog.connection.execute_batch("CREATE TRIGGER reject_group_conflict BEFORE INSERT ON organization_conflicts BEGIN SELECT RAISE(ABORT, 'injected conflict failure'); END;").unwrap();
    assert!(publish_batch(&mut catalog, &["a", "c"]).is_err());
    assert_eq!(
        scalar(
            &catalog,
            "SELECT COUNT(*) FROM gallery_content_fingerprints"
        ),
        0
    );
    assert_eq!(
        scalar(&catalog, "SELECT revision FROM catalog_state"),
        before
    );
    assert_eq!(
        decision(&catalog, "a"),
        (Some(first), "organized".into(), false)
    );
    assert_eq!(
        decision(&catalog, "c"),
        (Some(second), "organized".into(), false)
    );
    catalog
        .connection
        .execute_batch("DROP TRIGGER reject_group_conflict")
        .unwrap();
    publish_batch(&mut catalog, &["a", "c"]).unwrap();
    assert_eq!(decision(&catalog, "a"), (None, "deferred".into(), false));
}

#[test]
fn terminal_publication_reconciles_unpublished_evidence_with_one_gallery_revision() {
    let (_directory, mut catalog, _, _) = conflict_fixture();
    let before = scalar(&catalog, "SELECT revision FROM catalog_state");
    let transaction = catalog.connection.transaction().unwrap();
    super::super::super::gallery::duplicates::publish_completed_evidence(&transaction).unwrap();
    transaction.commit().unwrap();
    assert_eq!(
        scalar(&catalog, "SELECT revision FROM catalog_state"),
        before + 1
    );
    assert_eq!(decision(&catalog, "c"), (None, "deferred".into(), false));
}
