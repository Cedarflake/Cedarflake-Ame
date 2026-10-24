use rusqlite::OptionalExtension;

use crate::domain::organization::{OrganizationAction, OrganizationSelection};
use crate::ports::organization::OrganizationRepository;

use super::*;

fn album_for(catalog: &SqliteCatalog, location_id: &str) -> Option<String> {
    catalog
        .connection
        .query_row(
            "SELECT canonical.album_id FROM asset_locations AS location
         JOIN organization_bindings AS binding ON binding.asset_id = location.asset_id
           AND binding.source_generation = location.source_generation
         JOIN organization_record_groups AS member ON member.record_id = binding.record_id
         JOIN organization_records AS canonical ON canonical.record_id = member.group_id
         WHERE location.location_id = ?1",
            [location_id],
            |row| row.get(0),
        )
        .optional()
        .unwrap()
        .flatten()
}

#[test]
fn journal_rename_preserves_organization_and_content_change_requires_new_proof() {
    let (directory, mut catalog) = fixture();
    publish_fingerprint(&mut catalog, "location-a", 'a');
    publish_fingerprint(&mut catalog, "location-b", 'a');
    let album = catalog
        .create_organization_album("create-album", 0, "Kept")
        .unwrap()
        .album_id;
    let before = candidate(&catalog, "location-b");
    let selection = catalog
        .prepare_organization_selection(&[OrganizationSelection {
            asset_id: before.location.asset_id.clone(),
            location_id: before.location.location_id.clone(),
            source_generation: before.location.source_generation,
        }])
        .unwrap();
    catalog
        .change_organization(
            "assign",
            &selection,
            &OrganizationAction::AssignAlbum {
                album_id: album.clone(),
            },
        )
        .unwrap();
    let mut moved = before.location.clone();
    moved.location_id = "moved".into();
    moved.relative_path = "nested/moved.jpg".into();
    moved.absolute_path = "C:/source/nested/moved.jpg".into();
    moved.display_path.clone_from(&moved.absolute_path);
    apply_change(
        &mut catalog,
        Some(&before.location),
        Some(moved),
        IncrementalReconciliationOutcome::RenamedOrMoved,
    );
    assert_eq!(album_for(&catalog, "moved"), Some(album.clone()));
    let old = candidate(&catalog, "moved");
    let mut changed = old.location.clone();
    changed.modified_unix_ms += 1;
    changed.source_revision.as_mut().unwrap().value = "0000000000000002".into();
    changed.source_generation = 0;
    apply_change(
        &mut catalog,
        Some(&old.location),
        Some(changed),
        IncrementalReconciliationOutcome::Modified,
    );
    assert_eq!(album_for(&catalog, "moved"), None);
    assert_eq!(album_for(&catalog, "location-a"), Some(album.clone()));
    assert!(
        !catalog
            .publish_fingerprint("run", &old, &fingerprint('a'), &AtomicBool::new(false))
            .unwrap()
    );
    publish_fingerprint(&mut catalog, "moved", 'b');
    let generation = candidate(&catalog, "moved").location.source_generation;
    catalog
        .publish_exact_analysis_batch("run", &[generation])
        .unwrap();
    assert_eq!(album_for(&catalog, "moved"), None);
    drop(catalog);
    let catalog = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
    assert_eq!(album_for(&catalog, "location-a"), Some(album));
    assert_eq!(album_for(&catalog, "moved"), None);
}
