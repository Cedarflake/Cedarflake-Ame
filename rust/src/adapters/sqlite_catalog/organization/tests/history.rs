use super::transitions::{change, create_album, decision};
use super::*;

fn step(
    catalog: &mut SqliteCatalog,
    id: &str,
    direction: OrganizationHistoryDirection,
) -> OrganizationHistoryReceipt {
    let history = catalog.load_organization_history().unwrap();
    let original = match direction {
        OrganizationHistoryDirection::Undo => history.undo_operation_id,
        OrganizationHistoryDirection::Redo => history.redo_operation_id,
    }
    .unwrap();
    catalog
        .step_organization_history(id, direction, &original, history.organization_revision)
        .unwrap()
}

#[test]
fn consecutive_same_field_changes_can_be_undone_and_redone_in_order() {
    let (_directory, mut catalog) = fixture();
    for (id, value) in [("favorite", true), ("unfavorite", false)] {
        let prepared = prepare(&catalog, &["a"]);
        catalog
            .set_organization_favorite(id, &prepared, value)
            .unwrap();
    }
    for (id, direction, favorite) in [
        ("undo-second", OrganizationHistoryDirection::Undo, true),
        ("undo-first", OrganizationHistoryDirection::Undo, false),
        ("redo-first", OrganizationHistoryDirection::Redo, true),
        ("redo-second", OrganizationHistoryDirection::Redo, false),
        (
            "undo-second-again",
            OrganizationHistoryDirection::Undo,
            true,
        ),
    ] {
        let before = catalog.load_organization_history().unwrap();
        let receipt = step(&mut catalog, id, direction);
        assert_eq!(
            receipt.operation.organization_revision,
            before.organization_revision + 1
        );
        assert_eq!(decision(&catalog, "a").2, favorite);
    }
}

#[test]
fn history_crosses_groups_and_fields_and_preserves_batch_boundaries() {
    let (_directory, mut catalog) = fixture();
    change(&mut catalog, "defer-a", &["a"], OrganizationAction::Defer);
    let prepared = prepare(&catalog, &["b", "c"]);
    catalog
        .set_organization_favorite("favorite-batch", &prepared, true)
        .unwrap();
    change(&mut catalog, "ignore-a", &["a"], OrganizationAction::Ignore);
    step(
        &mut catalog,
        "undo-ignore",
        OrganizationHistoryDirection::Undo,
    );
    assert_eq!(decision(&catalog, "a").1, "deferred");
    assert!(decision(&catalog, "b").2);
    assert_eq!(
        step(
            &mut catalog,
            "undo-batch",
            OrganizationHistoryDirection::Undo
        )
        .operation
        .changed_groups,
        2
    );
    for id in ["b", "c"] {
        assert!(!decision(&catalog, id).2);
    }
    step(
        &mut catalog,
        "undo-defer",
        OrganizationHistoryDirection::Undo,
    );
    assert_eq!(decision(&catalog, "a").1, "pending");
    assert!(
        catalog
            .load_organization_history()
            .unwrap()
            .undo_operation_id
            .is_none()
    );
    step(
        &mut catalog,
        "redo-defer",
        OrganizationHistoryDirection::Redo,
    );
    step(
        &mut catalog,
        "redo-batch",
        OrganizationHistoryDirection::Redo,
    );
    step(
        &mut catalog,
        "redo-ignore",
        OrganizationHistoryDirection::Redo,
    );
    assert_eq!(decision(&catalog, "a").1, "ignored");
    for id in ["b", "c"] {
        assert!(decision(&catalog, id).2);
    }
    assert!(
        catalog
            .load_organization_history()
            .unwrap()
            .redo_operation_id
            .is_none()
    );
}

#[test]
fn reopening_and_retry_do_not_advance_the_cursor_twice_or_change_direction() {
    let (directory, mut catalog) = fixture();
    change(&mut catalog, "ignore", &["a"], OrganizationAction::Ignore);
    let before = catalog.load_organization_history().unwrap();
    let undone = step(&mut catalog, "undo", OrganizationHistoryDirection::Undo);
    drop(catalog);
    let mut catalog = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
    let reopened = catalog.load_organization_history().unwrap();
    assert_eq!(reopened.undo_operation_id, None);
    assert_eq!(reopened.redo_operation_id.as_deref(), Some("undo"));
    assert_eq!(
        catalog
            .step_organization_history(
                "undo",
                OrganizationHistoryDirection::Undo,
                "ignore",
                before.organization_revision
            )
            .unwrap(),
        undone
    );
    assert_eq!(catalog.load_organization_history().unwrap(), reopened);
    assert_eq!(
        catalog
            .step_organization_history(
                "undo",
                OrganizationHistoryDirection::Redo,
                "ignore",
                before.organization_revision
            )
            .unwrap_err()
            .code,
        "organization_operation_conflict"
    );
    step(&mut catalog, "redo", OrganizationHistoryDirection::Redo);
    assert_eq!(decision(&catalog, "a").1, "ignored");
}

#[test]
fn new_changes_discard_redo_but_noop_saves_and_stale_requests_do_not() {
    let (_directory, mut catalog) = fixture();
    change(&mut catalog, "ignore", &["a"], OrganizationAction::Ignore);
    step(&mut catalog, "undo", OrganizationHistoryDirection::Undo);
    let before = catalog.load_organization_history().unwrap();
    let prepared = prepare(&catalog, &["a"]);
    catalog
        .set_organization_favorite("no-change", &prepared, false)
        .unwrap();
    assert_eq!(catalog.load_organization_history().unwrap(), before);
    assert_eq!(
        catalog
            .step_organization_history(
                "wrong-head",
                OrganizationHistoryDirection::Redo,
                "ignore",
                before.organization_revision
            )
            .unwrap_err()
            .code,
        "organization_history_changed"
    );
    assert_eq!(catalog.load_organization_history().unwrap(), before);
    change(&mut catalog, "defer", &["b"], OrganizationAction::Defer);
    let current = catalog.load_organization_history().unwrap();
    assert_eq!(current.undo_operation_id.as_deref(), Some("defer"));
    assert_eq!(current.redo_operation_id, None);
    assert_eq!(
        catalog
            .step_organization_history(
                "stale-redo",
                OrganizationHistoryDirection::Redo,
                "undo",
                before.organization_revision
            )
            .unwrap_err()
            .code,
        "organization_selection_changed"
    );
    assert_eq!(
        scalar(
            &catalog,
            "SELECT COUNT(*) FROM organization_operations WHERE operation_id IN ('ignore', 'undo')"
        ),
        2
    );
}

#[test]
fn failed_inverse_rolls_back_values_authority_receipt_and_cursor() {
    let (_directory, mut catalog) = fixture();
    change(
        &mut catalog,
        "ignore",
        &["a", "b"],
        OrganizationAction::Ignore,
    );
    let before = catalog.load_organization_history().unwrap();
    catalog.connection.execute_batch(
        "CREATE TRIGGER reject_history BEFORE UPDATE OF disposition ON organization_records
         WHEN NEW.disposition = 'pending' BEGIN SELECT RAISE(ABORT, 'injected history failure'); END;"
    ).unwrap();
    assert!(
        catalog
            .step_organization_history(
                "undo",
                OrganizationHistoryDirection::Undo,
                "ignore",
                before.organization_revision
            )
            .is_err()
    );
    assert_eq!(catalog.load_organization_history().unwrap(), before);
    assert_eq!(
        scalar(
            &catalog,
            "SELECT COUNT(*) FROM organization_operations WHERE operation_id = 'undo'"
        ),
        0
    );
    assert_eq!(
        scalar(
            &catalog,
            "SELECT COUNT(*) FROM organization_field_authority WHERE operation_id = 'undo'"
        ),
        0
    );
    for id in ["a", "b"] {
        assert_eq!(decision(&catalog, id).1, "ignored");
    }
    catalog
        .connection
        .execute_batch("DROP TRIGGER reject_history;")
        .unwrap();
    step(&mut catalog, "undo", OrganizationHistoryDirection::Undo);
    for id in ["a", "b"] {
        assert_eq!(decision(&catalog, id).1, "pending");
    }
}

#[test]
fn album_creation_rename_assignment_and_retirement_round_trip() {
    let (_directory, mut catalog) = fixture();
    let album = create_album(&mut catalog, "First");
    let revision = catalog
        .load_organization_history()
        .unwrap()
        .organization_revision;
    catalog
        .rename_organization_album("rename", revision, &album, "Second")
        .unwrap();
    change(
        &mut catalog,
        "assign",
        &["a", "b"],
        OrganizationAction::AssignAlbum {
            album_id: album.clone(),
        },
    );
    let revision = catalog
        .load_organization_history()
        .unwrap()
        .organization_revision;
    catalog
        .delete_organization_album("delete", revision, &album)
        .unwrap();
    for id in ["undo-delete", "undo-assign", "undo-rename", "undo-create"] {
        let result = step(&mut catalog, id, OrganizationHistoryDirection::Undo);
        assert_eq!(
            result.retired_album_id.as_deref(),
            if id == "undo-create" {
                Some(album.as_str())
            } else {
                None
            }
        );
    }
    assert_eq!(
        catalog.load_organization_albums(None).unwrap().total_albums,
        0
    );
    step(
        &mut catalog,
        "redo-create",
        OrganizationHistoryDirection::Redo,
    );
    assert_eq!(
        catalog.load_organization_albums(None).unwrap().albums[0].name,
        "First"
    );
    step(
        &mut catalog,
        "redo-rename",
        OrganizationHistoryDirection::Redo,
    );
    assert_eq!(
        catalog.load_organization_albums(None).unwrap().albums[0].name,
        "Second"
    );
    step(
        &mut catalog,
        "redo-assign",
        OrganizationHistoryDirection::Redo,
    );
    assert_eq!(decision(&catalog, "a").0, Some(album.clone()));
    assert_eq!(
        step(
            &mut catalog,
            "redo-delete",
            OrganizationHistoryDirection::Redo
        )
        .retired_album_id,
        Some(album)
    );
    assert_eq!(decision(&catalog, "a").1, "ignored");
    assert_eq!(decision(&catalog, "b").1, "ignored");
}

#[test]
fn redoing_creation_does_not_authorize_retiring_later_members() {
    let (_directory, mut catalog) = fixture();
    let album = create_album(&mut catalog, "Album");
    step(
        &mut catalog,
        "undo-create",
        OrganizationHistoryDirection::Undo,
    );
    step(
        &mut catalog,
        "redo-create",
        OrganizationHistoryDirection::Redo,
    );
    change(
        &mut catalog,
        "assign",
        &["a"],
        OrganizationAction::AssignAlbum {
            album_id: album.clone(),
        },
    );
    let history = catalog.load_organization_history().unwrap();
    assert_eq!(
        catalog
            .undo_organization_operation(
                "retire-used",
                "redo-create",
                history.organization_revision
            )
            .unwrap_err()
            .code,
        "organization_undo_unavailable"
    );
    assert_eq!(decision(&catalog, "a").0, Some(album));
    assert_eq!(catalog.load_organization_history().unwrap(), history);
}

#[test]
fn changed_content_and_new_group_members_still_reject_history() {
    for replace_content in [true, false] {
        let (_directory, mut catalog) = fixture();
        publish(&mut catalog, "a", 'a');
        change(&mut catalog, "ignore", &["a"], OrganizationAction::Ignore);
        if replace_content {
            catalog.connection.execute("UPDATE asset_locations SET source_generation = source_generation + 100 WHERE location_id = 'a'", []).unwrap();
        } else {
            change(&mut catalog, "ignore-c", &["c"], OrganizationAction::Ignore);
            publish(&mut catalog, "c", 'a');
            let generations = ["a", "c"].map(|id| selected(&catalog, id).source_generation);
            catalog
                .publish_exact_analysis_batch("organization-fixture", &generations)
                .unwrap();
        }
        let history = catalog.load_organization_history().unwrap();
        assert_eq!(
            catalog
                .step_organization_history(
                    "unsafe-undo",
                    OrganizationHistoryDirection::Undo,
                    history.undo_operation_id.as_deref().unwrap(),
                    history.organization_revision
                )
                .unwrap_err()
                .code,
            "organization_undo_unavailable"
        );
        assert_eq!(catalog.load_organization_history().unwrap(), history);
    }
}
