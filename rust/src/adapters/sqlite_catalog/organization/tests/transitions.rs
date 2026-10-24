use super::*;

pub(super) fn create_album(catalog: &mut SqliteCatalog, name: &str) -> String {
    let revision = catalog
        .load_organization_albums(None)
        .unwrap()
        .organization_revision;
    catalog
        .create_organization_album(&format!("create-{name}"), revision, name)
        .unwrap()
        .album_id
}

pub(super) fn change(
    catalog: &mut SqliteCatalog,
    operation: &str,
    ids: &[&str],
    action: OrganizationAction,
) -> OrganizationWriteReceipt {
    let prepared = prepare(catalog, ids);
    catalog
        .change_organization(operation, &prepared, &action)
        .unwrap()
}

pub(super) fn decision(catalog: &SqliteCatalog, id: &str) -> (Option<String>, String, bool) {
    catalog.connection.query_row(
        "SELECT record.album_id, record.disposition, record.favorite FROM organization_records AS record
         JOIN organization_bindings AS binding ON binding.record_id = record.record_id
         JOIN asset_locations AS location ON location.asset_id = binding.asset_id
           AND location.source_generation = binding.source_generation WHERE location.location_id = ?1",
        [id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?))
    ).unwrap()
}

#[test]
fn assignment_move_and_removal_apply_to_the_whole_group_and_preserve_favorites() {
    let (_directory, mut catalog) = fixture();
    publish(&mut catalog, "a", 'a');
    publish(&mut catalog, "c", 'a');
    let first = create_album(&mut catalog, "First");
    let second = create_album(&mut catalog, "Second");
    let prepared = prepare(&catalog, &["a"]);
    catalog
        .set_organization_favorite("favorite", &prepared, true)
        .unwrap();
    assert_eq!(
        change(
            &mut catalog,
            "assign",
            &["c", "a"],
            OrganizationAction::AssignAlbum {
                album_id: first.clone()
            }
        )
        .changed_groups,
        1
    );
    for id in ["a", "c"] {
        assert_eq!(
            decision(&catalog, id),
            (Some(first.clone()), "organized".into(), true)
        );
    }
    change(
        &mut catalog,
        "move",
        &["a"],
        OrganizationAction::AssignAlbum {
            album_id: second.clone(),
        },
    );
    for id in ["a", "c"] {
        assert_eq!(
            decision(&catalog, id),
            (Some(second.clone()), "organized".into(), true)
        );
    }
    change(
        &mut catalog,
        "remove",
        &["c"],
        OrganizationAction::RemoveAlbum,
    );
    for id in ["a", "c"] {
        assert_eq!(decision(&catalog, id), (None, "ignored".into(), true));
    }
}

#[test]
fn pending_deferred_ignored_and_album_completion_share_one_transaction_path() {
    let (_directory, mut catalog) = fixture();
    for (operation, action, expected) in [
        ("defer", OrganizationAction::Defer, "deferred"),
        ("ignore", OrganizationAction::Ignore, "ignored"),
        ("pending", OrganizationAction::RestorePending, "pending"),
    ] {
        change(&mut catalog, operation, &["b"], action);
        assert_eq!(decision(&catalog, "b"), (None, expected.into(), false));
    }
    let album_id = create_album(&mut catalog, "完成");
    let prepared = prepare(&catalog, &["b"]);
    let action = OrganizationAction::AssignAlbum {
        album_id: album_id.clone(),
    };
    let first = catalog
        .change_organization("complete", &prepared, &action)
        .unwrap();
    assert_eq!(
        catalog
            .change_organization("complete", &prepared, &action)
            .unwrap(),
        first
    );
    assert_eq!(
        decision(&catalog, "b"),
        (Some(album_id), "organized".into(), false)
    );
    assert_eq!(
        change(&mut catalog, "same-assignment", &["b"], action).changed_groups,
        0
    );
}

#[test]
fn deleting_an_album_ignores_its_groups_and_keeps_independent_favorites() {
    let (directory, mut catalog) = fixture();
    let album = create_album(&mut catalog, "Album");
    change(
        &mut catalog,
        "assign",
        &["a", "b"],
        OrganizationAction::AssignAlbum {
            album_id: album.clone(),
        },
    );
    let prepared = prepare(&catalog, &["a"]);
    catalog
        .set_organization_favorite("favorite", &prepared, true)
        .unwrap();
    let revision = catalog
        .load_organization_albums(None)
        .unwrap()
        .organization_revision;
    let deleted = catalog
        .delete_organization_album("delete", revision, &album)
        .unwrap();
    assert_eq!(deleted.changed_groups, 2);
    assert_eq!(
        catalog
            .delete_organization_album("delete", revision, &album)
            .unwrap(),
        deleted
    );
    assert_eq!(
        catalog.load_organization_albums(None).unwrap().total_albums,
        0
    );
    assert_eq!(decision(&catalog, "a"), (None, "ignored".into(), true));
    assert_eq!(decision(&catalog, "b"), (None, "ignored".into(), false));
    drop(catalog);
    let catalog = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
    assert_eq!(decision(&catalog, "a"), (None, "ignored".into(), true));
    assert_eq!(
        catalog.load_organization_albums(None).unwrap().total_albums,
        0
    );
}

#[test]
fn album_names_retries_and_paging_are_bounded_and_durable() {
    let (_directory, mut catalog) = fixture();
    let first = catalog
        .create_organization_album("create", 0, "  风景  ")
        .unwrap();
    assert_eq!(
        catalog
            .create_organization_album("create", 0, "风景")
            .unwrap(),
        first
    );
    assert_eq!(
        catalog
            .create_organization_album("different", first.operation.organization_revision, "风景")
            .unwrap_err()
            .code,
        "organization_album_name_exists"
    );
    assert_eq!(
        catalog
            .rename_organization_album(
                "rename",
                first.operation.organization_revision,
                &first.album_id,
                "旅行"
            )
            .unwrap()
            .album_id,
        first.album_id
    );
    assert_eq!(
        catalog.load_organization_albums(None).unwrap().albums[0].name,
        "旅行"
    );
    for index in 0..65 {
        create_album(&mut catalog, &format!("album-{index}"));
    }
    let page = catalog.load_organization_albums(None).unwrap();
    assert_eq!((page.albums.len(), page.total_albums), (64, 66));
    let next = catalog
        .load_organization_albums(page.next_album_id.as_deref())
        .unwrap();
    assert_eq!(next.albums.len(), 2);
    assert!(next.next_album_id.is_none());
    assert!(page.albums.iter().all(|first| {
        next.albums
            .iter()
            .all(|second| first.album_id != second.album_id)
    }));
}

#[test]
fn a_failed_album_assignment_does_not_publish_progress_or_consume_its_request() {
    let (_directory, mut catalog) = fixture();
    let album = create_album(&mut catalog, "Album");
    let prepared = prepare(&catalog, &["a", "b"]);
    catalog
        .connection
        .execute_batch(
            "CREATE TRIGGER fail_assignment BEFORE UPDATE OF disposition ON organization_records
         WHEN (SELECT COUNT(*) FROM organization_records WHERE disposition = 'organized') = 1
         BEGIN SELECT RAISE(ABORT, 'injected assignment failure'); END;",
        )
        .unwrap();
    let action = OrganizationAction::AssignAlbum { album_id: album };
    assert!(
        catalog
            .change_organization("assign", &prepared, &action)
            .is_err()
    );
    assert_eq!(
        scalar(&catalog, "SELECT COUNT(*) FROM organization_records"),
        0
    );
    assert_eq!(
        scalar(
            &catalog,
            "SELECT COUNT(*) FROM organization_operations WHERE operation_id = 'assign'"
        ),
        0
    );
    catalog
        .connection
        .execute_batch("DROP TRIGGER fail_assignment")
        .unwrap();
    assert_eq!(
        catalog
            .change_organization("assign", &prepared, &action)
            .unwrap()
            .changed_groups,
        2
    );
}
