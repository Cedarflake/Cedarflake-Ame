use crate::ports::organization_conflicts::OrganizationConflictRepository;

use super::queries::{evolution::invalidate, publish_batch};
use super::transitions::{change, create_album};
use super::*;

fn conflicting(catalog: &mut SqliteCatalog) -> (String, String) {
    let first = create_album(catalog, "风景");
    let second = create_album(catalog, "Archive");
    change(
        catalog,
        "first-album",
        &["a"],
        OrganizationAction::AssignAlbum {
            album_id: first.clone(),
        },
    );
    change(
        catalog,
        "second-album",
        &["c"],
        OrganizationAction::AssignAlbum {
            album_id: second.clone(),
        },
    );
    publish_batch(catalog, &["a", "c"], 'a');
    (first, second)
}

#[test]
fn conflicts_survive_reopen_and_retired_albums_until_resolution_or_undo() {
    let (directory, mut catalog) = fixture();
    let (first, second) = conflicting(&mut catalog);
    let revision = catalog
        .load_organization_albums(None)
        .unwrap()
        .organization_revision;
    catalog
        .delete_organization_album("retire-first", revision, &first)
        .unwrap();
    drop(catalog);
    let mut catalog = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
    let operations = scalar(&catalog, "SELECT COUNT(*) FROM organization_operations");
    let page = catalog
        .load_organization_conflict_albums(&selected(&catalog, "c"), None)
        .unwrap();
    assert_eq!(page.albums.len(), 2);
    assert!(
        page.albums
            .iter()
            .any(|album| album.album_id == first && album.name == "风景" && album.retired)
    );
    assert!(
        page.albums
            .iter()
            .any(|album| album.album_id == second && !album.retired)
    );
    assert_eq!(
        catalog
            .load_organization_conflict_status()
            .unwrap()
            .conflicting_groups,
        1
    );
    let marked = catalog
        .load_organization_conflict_annotations(0, &["a".into(), "b".into(), "c".into()])
        .unwrap();
    assert_eq!(marked.location_ids, ["a", "c"]);
    assert_eq!(
        scalar(&catalog, "SELECT COUNT(*) FROM organization_operations"),
        operations
    );
    let receipt = change(
        &mut catalog,
        "resolve",
        &["c"],
        OrganizationAction::AssignAlbum { album_id: second },
    );
    assert_eq!(
        catalog
            .load_organization_conflict_status()
            .unwrap()
            .conflicting_groups,
        0
    );
    assert!(
        catalog
            .load_organization_conflict_albums(&selected(&catalog, "a"), None)
            .unwrap()
            .albums
            .is_empty()
    );
    catalog
        .undo_organization_operation(
            "undo-resolution",
            &receipt.operation_id,
            receipt.organization_revision,
        )
        .unwrap();
    assert_eq!(
        catalog
            .load_organization_conflict_status()
            .unwrap()
            .conflicting_groups,
        1
    );
    assert_eq!(
        catalog
            .load_organization_conflict_albums(&selected(&catalog, "a"), None)
            .unwrap()
            .albums
            .len(),
        2
    );
}

#[test]
fn conflict_choices_are_bounded_and_continuations_do_not_repeat_albums() {
    let (_directory, mut catalog) = fixture();
    conflicting(&mut catalog);
    let record: String = catalog
        .connection
        .query_row(
            "SELECT record_id FROM organization_conflicts LIMIT 1",
            [],
            |row| row.get(0),
        )
        .unwrap();
    for index in 0..63 {
        let album = create_album(&mut catalog, &format!("Candidate {index}"));
        catalog
            .connection
            .execute(
                "INSERT INTO organization_conflicts(record_id, album_id, decision_revision)
             SELECT ?1, ?2, revision FROM organization_state WHERE singleton = 1",
                params![record, album],
            )
            .unwrap();
    }
    let target = selected(&catalog, "a");
    let first = catalog
        .load_organization_conflict_albums(&target, None)
        .unwrap();
    let second = catalog
        .load_organization_conflict_albums(&target, first.next_album_id.as_deref())
        .unwrap();
    assert_eq!(first.albums.len(), 64);
    assert_eq!(second.albums.len(), 1);
    assert_eq!(first.organization_revision, second.organization_revision);
    assert!(second.next_album_id.is_none());
    assert!(
        first
            .albums
            .iter()
            .all(|album| album.album_id != second.albums[0].album_id)
    );
    assert_eq!(
        catalog
            .load_organization_conflict_status()
            .unwrap()
            .conflicting_groups,
        1
    );
    assert!(
        catalog
            .load_organization_conflict_annotations(0, &vec!["a".into(); 257])
            .is_err()
    );
    assert!(
        catalog
            .load_organization_conflict_annotations(u64::MAX, &["a".into()])
            .is_err()
    );
    assert!(
        catalog
            .load_organization_conflict_albums(&target, Some(""))
            .is_err()
    );
}

#[test]
fn changed_content_does_not_inherit_conflict_markers_or_authorize_stale_inspection() {
    let (_directory, mut catalog) = fixture();
    conflicting(&mut catalog);
    let old = selected(&catalog, "c");
    invalidate(&mut catalog, "c");
    publish_batch(&mut catalog, &["c"], 'b');
    assert!(
        catalog
            .load_organization_conflict_albums(&old, None)
            .is_err()
    );
    assert!(
        catalog
            .load_organization_conflict_albums(&selected(&catalog, "c"), None)
            .unwrap()
            .albums
            .is_empty()
    );
    assert_eq!(
        catalog
            .load_organization_conflict_annotations(0, &["a".into(), "c".into()])
            .unwrap()
            .location_ids,
        ["a"]
    );
    invalidate(&mut catalog, "a");
    publish_batch(&mut catalog, &["a"], 'c');
    assert_eq!(
        catalog
            .load_organization_conflict_status()
            .unwrap()
            .conflicting_groups,
        0
    );
}

#[test]
fn conflict_reads_do_not_materialize_untouched_groups_or_admit_unpublished_merges() {
    let (_directory, mut catalog) = fixture();
    let target = selected(&catalog, "a");
    assert!(
        catalog
            .load_organization_conflict_albums(&target, None)
            .unwrap()
            .albums
            .is_empty()
    );
    assert_eq!(
        scalar(&catalog, "SELECT COUNT(*) FROM organization_records"),
        0
    );
    let first = create_album(&mut catalog, "First");
    let second = create_album(&mut catalog, "Second");
    change(
        &mut catalog,
        "first",
        &["a"],
        OrganizationAction::AssignAlbum { album_id: first },
    );
    change(
        &mut catalog,
        "second",
        &["c"],
        OrganizationAction::AssignAlbum { album_id: second },
    );
    publish(&mut catalog, "a", 'a');
    publish(&mut catalog, "c", 'a');
    assert_eq!(
        catalog
            .load_organization_conflict_albums(&target, None)
            .unwrap_err()
            .code,
        "organization_evidence_pending"
    );
}
