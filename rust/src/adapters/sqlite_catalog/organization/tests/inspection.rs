use crate::domain::organization::{OrganizationDisposition, OrganizationInspection};

use super::queries::{evolution::invalidate, publish_batch};
use super::transitions::{change, create_album};
use super::*;

fn inspect(catalog: &SqliteCatalog, ids: &[&str]) -> OrganizationInspection {
    catalog
        .inspect_organization_selection(
            &ids.iter()
                .map(|id| selected(catalog, id))
                .collect::<Vec<_>>(),
        )
        .unwrap()
}

#[test]
fn inspecting_unhandled_groups_is_read_only_and_deduplicates_selected_members() {
    let (_directory, mut catalog) = fixture();
    publish_batch(&mut catalog, &["a", "c"], 'a');
    let before = prepare(&catalog, &["a"]);
    let result = inspect(&catalog, &["a", "a", "c", "b"]);
    assert_eq!(result.catalog_revision, before.catalog_revision);
    assert_eq!(result.organization_revision, before.organization_revision);
    assert_eq!(result.decisions.len(), 2);
    assert!(result.decisions.iter().all(|decision| !decision.favorite
        && decision.disposition == OrganizationDisposition::Pending
        && decision.album.is_none()
        && !decision.has_conflict));
    assert_eq!(
        scalar(&catalog, "SELECT COUNT(*) FROM organization_records"),
        0
    );
    assert_eq!(
        scalar(&catalog, "SELECT COUNT(*) FROM organization_operations"),
        0
    );
}

#[test]
fn inspection_reads_the_whole_groups_album_and_independent_favorite_after_reopen() {
    let (directory, mut catalog) = fixture();
    publish_batch(&mut catalog, &["a", "c"], 'a');
    let album_id = create_album(&mut catalog, "Landscape");
    change(
        &mut catalog,
        "assign",
        &["c"],
        OrganizationAction::AssignAlbum {
            album_id: album_id.clone(),
        },
    );
    let prepared = prepare(&catalog, &["a"]);
    catalog
        .set_organization_favorite("favorite", &prepared, true)
        .unwrap();
    drop(catalog);
    let catalog = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
    let decision = inspect(&catalog, &["c"]).decisions.remove(0);
    assert!(decision.favorite);
    assert_eq!(decision.disposition, OrganizationDisposition::Organized);
    let album = decision.album.unwrap();
    assert_eq!(album.album_id, album_id);
    assert_eq!(album.name, "Landscape");
    assert!(!decision.has_conflict);
}

#[test]
fn inspection_keeps_conflicts_visible_and_changed_content_unhandled() {
    let (_directory, mut catalog) = fixture();
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
    let error = catalog
        .inspect_organization_selection(&[selected(&catalog, "a")])
        .unwrap_err();
    assert_eq!(error.code, "organization_evidence_pending");
    let generations = [
        selected(&catalog, "a").source_generation,
        selected(&catalog, "c").source_generation,
    ];
    catalog
        .publish_exact_analysis_batch("organization-fixture", &generations)
        .unwrap();
    let conflict = inspect(&catalog, &["a"]).decisions.remove(0);
    assert_eq!(conflict.disposition, OrganizationDisposition::Deferred);
    assert!(conflict.has_conflict);
    assert!(conflict.album.is_none());
    invalidate(&mut catalog, "c");
    publish_batch(&mut catalog, &["c"], 'b');
    let changed = inspect(&catalog, &["c"]).decisions.remove(0);
    assert_eq!(changed.disposition, OrganizationDisposition::Pending);
    assert!(!changed.has_conflict);
    assert!(!changed.favorite);
    assert!(changed.album.is_none());
}

#[test]
fn inspection_rejects_stale_locations_and_oversized_requests() {
    let (_directory, mut catalog) = fixture();
    let old = selected(&catalog, "a");
    invalidate(&mut catalog, "a");
    assert!(catalog.inspect_organization_selection(&[old]).is_err());
    assert!(catalog.inspect_organization_selection(&[]).is_err());
    let requests = vec![selected(&catalog, "b"); 257];
    assert!(catalog.inspect_organization_selection(&requests).is_err());
}
