use super::*;

#[test]
fn a_later_group_failure_rolls_back_earlier_groups_and_the_operation_receipt() {
    let (directory, catalog) = fixture();
    let (mut batch, _) = open(directory.path(), &catalog, GalleryQuery::default(), true);
    let summary = batch.prepare().unwrap();
    catalog.connection.execute_batch(
        "CREATE TRIGGER fail_later_group BEFORE INSERT ON organization_operation_changes
         WHEN (SELECT COUNT(*) FROM organization_operation_changes WHERE operation_id = NEW.operation_id) = 2
         BEGIN SELECT RAISE(ABORT, 'injected later group failure'); END;"
    ).unwrap();
    assert!(batch.apply("atomic-batch", &favorite(true)).is_err());
    assert_eq!(
        scalar(&catalog, "SELECT COUNT(*) FROM organization_records"),
        0
    );
    assert_eq!(
        scalar(&catalog, "SELECT COUNT(*) FROM organization_operations"),
        0
    );
    assert_eq!(
        scalar(&catalog, "SELECT revision FROM organization_state"),
        summary.organization_revision
    );
    catalog
        .connection
        .execute_batch("DROP TRIGGER fail_later_group;")
        .unwrap();
    let receipt = batch.apply("atomic-batch", &favorite(true)).unwrap();
    assert_eq!(receipt.changed_groups, 3);
    assert_eq!(
        batch.apply("atomic-batch", &favorite(true)).unwrap(),
        receipt
    );
    assert_eq!(
        batch
            .apply("atomic-batch", &favorite(false))
            .unwrap_err()
            .code,
        "organization_operation_conflict"
    );
    assert_eq!(
        scalar(&catalog, "SELECT COUNT(*) FROM organization_operations"),
        1
    );
}

#[test]
fn dialog_wait_releases_transactions_and_external_decisions_reject_the_stale_batch() {
    let (directory, mut catalog) = fixture();
    let (mut batch, _) = open(directory.path(), &catalog, GalleryQuery::default(), true);
    batch.prepare().unwrap();
    create_album(&mut catalog, "Written while dialog waits");
    assert_eq!(
        batch
            .apply("stale-decision", &favorite(true))
            .unwrap_err()
            .code,
        "organization_selection_changed"
    );
    assert_eq!(
        scalar(&catalog, "SELECT COUNT(*) FROM organization_records"),
        0
    );
    assert_eq!(
        scalar(
            &catalog,
            "SELECT COUNT(*) FROM organization_operations WHERE operation_id = 'stale-decision'"
        ),
        0
    );
}

#[test]
fn catalog_changes_and_unpublished_content_cannot_reuse_prepared_authority() {
    let (directory, mut catalog) = fixture();
    let (mut batch, _) = open(directory.path(), &catalog, GalleryQuery::default(), true);
    batch.prepare().unwrap();
    let revision = scalar(&catalog, "SELECT revision FROM catalog_state");
    publish(&mut catalog, "a", 'a');
    assert_eq!(
        scalar(&catalog, "SELECT revision FROM catalog_state"),
        revision
    );
    assert_eq!(
        batch
            .apply("stale-content", &favorite(true))
            .unwrap_err()
            .code,
        "organization_selection_changed"
    );
    assert_eq!(
        scalar(&catalog, "SELECT COUNT(*) FROM organization_records"),
        0
    );
    drop(batch);

    let (mut batch, _) = open(directory.path(), &catalog, GalleryQuery::default(), true);
    batch.prepare().unwrap();
    crate::adapters::sqlite_catalog::tests::publish_gallery_fixture(
        &mut catalog,
        "scan-new",
        "root-new",
        "E:\\New",
        &[("new", None, 4)],
    );
    assert_eq!(
        batch
            .apply("stale-catalog", &favorite(true))
            .unwrap_err()
            .code,
        "organization_selection_changed"
    );
    assert_eq!(
        scalar(&catalog, "SELECT COUNT(*) FROM organization_records"),
        0
    );
}

#[test]
fn cancellation_after_a_record_write_rolls_back_every_change() {
    let (directory, catalog) = fixture();
    let (mut batch, control) = open(directory.path(), &catalog, GalleryQuery::default(), true);
    let summary = batch.prepare().unwrap();
    batch.cancel_on_record_write();
    assert_eq!(
        batch
            .apply("cancelled-batch", &favorite(true))
            .unwrap_err()
            .code,
        "organization_selection_cancelled"
    );
    assert_eq!(control.snapshot().phase, OrganizationBatchPhase::Cancelled);
    drop(batch);
    drop(catalog);
    let reopened = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
    assert_eq!(
        scalar(&reopened, "SELECT COUNT(*) FROM organization_records"),
        0
    );
    assert_eq!(
        scalar(&reopened, "SELECT COUNT(*) FROM organization_operations"),
        0
    );
    assert_eq!(
        scalar(&reopened, "SELECT revision FROM organization_state"),
        summary.organization_revision
    );
    assert_eq!(
        scalar(&reopened, "SELECT revision FROM catalog_state"),
        summary.catalog_revision
    );
}

#[test]
fn batch_album_assignment_and_undo_preserve_independent_favorites() {
    let (directory, mut catalog) = fixture();
    let album = create_album(&mut catalog, "Destination");
    catalog
        .set_organization_favorite("favorite-b", &prepare(&catalog, &["b"]), true)
        .unwrap();
    let (mut batch, _) = open(directory.path(), &catalog, GalleryQuery::default(), true);
    batch.prepare().unwrap();
    let action = organize(OrganizationAction::AssignAlbum {
        album_id: album.clone(),
    });
    let receipt = batch.apply("assign-every-group", &action).unwrap();
    assert_eq!(receipt.changed_groups, 3);
    for id in ["a", "b", "c"] {
        assert_eq!(
            decision(&catalog, id),
            (Some(album.clone()), "organized".into(), id == "b")
        );
    }
    catalog
        .set_organization_favorite("favorite-c-later", &prepare(&catalog, &["c"]), true)
        .unwrap();
    assert_eq!(batch.apply("assign-every-group", &action).unwrap(), receipt);
    let revision = scalar(&catalog, "SELECT revision FROM organization_state");
    let undone = catalog
        .undo_organization_operation("undo-batch", "assign-every-group", revision)
        .unwrap();
    assert_eq!(undone.changed_groups, 3);
    for id in ["a", "b", "c"] {
        assert_eq!(decision(&catalog, id), (None, "pending".into(), id != "a"));
    }
}

#[test]
fn resolving_a_batch_conflict_and_undo_restores_one_deferred_group() {
    let (directory, mut catalog) = fixture();
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
    publish_batch(&mut catalog, &["a", "c"], 'a');
    let query = GalleryQuery {
        root_id: Some("root-c".into()),
        ..GalleryQuery::default()
    };
    let (mut batch, _) = open(directory.path(), &catalog, query, true);
    assert_eq!(batch.prepare().unwrap().conflicting_count, 1);
    let receipt = batch
        .apply(
            "resolve-batch",
            &organize(OrganizationAction::AssignAlbum {
                album_id: first.clone(),
            }),
        )
        .unwrap();
    assert_eq!(receipt.changed_groups, 1);
    for id in ["a", "c"] {
        assert_eq!(
            decision(&catalog, id),
            (Some(first.clone()), "organized".into(), false)
        );
    }
    catalog
        .undo_organization_operation(
            "undo-resolution",
            "resolve-batch",
            receipt.organization_revision,
        )
        .unwrap();
    for id in ["a", "c"] {
        assert_eq!(decision(&catalog, id), (None, "deferred".into(), false));
    }
    let (mut restored, _) = open(directory.path(), &catalog, GalleryQuery::default(), true);
    assert_eq!(restored.prepare().unwrap().conflicting_count, 1);
}
