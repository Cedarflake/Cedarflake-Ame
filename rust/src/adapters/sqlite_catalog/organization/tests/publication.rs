use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use super::transitions::{change, create_album, decision};
use super::*;

#[test]
fn earlier_unhandled_copy_is_attached_when_the_handled_member_finishes_later() {
    let (_directory, mut catalog) = fixture();
    let album = create_album(&mut catalog, "Later proof");
    change(
        &mut catalog,
        "assign",
        &["a"],
        OrganizationAction::AssignAlbum {
            album_id: album.clone(),
        },
    );
    publish(&mut catalog, "c", 'a');
    let first = selected(&catalog, "c").source_generation;
    catalog
        .publish_exact_analysis_batch("organization-fixture", &[first])
        .unwrap();
    publish(&mut catalog, "a", 'a');
    let second = selected(&catalog, "a").source_generation;
    catalog
        .publish_exact_analysis_batch("organization-fixture", &[second])
        .unwrap();
    assert_eq!(
        decision(&catalog, "c"),
        (Some(album.clone()), "organized".into(), false)
    );
    catalog.unregister_root("root-a").unwrap();
    assert_eq!(
        decision(&catalog, "c"),
        (Some(album), "organized".into(), false)
    );
}

#[test]
fn publishing_one_copy_does_not_revisit_fifty_thousand_prior_lineages() {
    let (_directory, mut catalog) = fixture();
    publish(&mut catalog, "a", 'a');
    let album = create_album(&mut catalog, "Large group");
    change(
        &mut catalog,
        "assign",
        &["a"],
        OrganizationAction::AssignAlbum {
            album_id: album.clone(),
        },
    );
    catalog.connection.execute_batch(
        "WITH RECURSIVE sequence(value) AS (VALUES(1) UNION ALL SELECT value + 1 FROM sequence WHERE value < 50000)
         INSERT INTO organization_records
         SELECT 'lineage-' || value, album_id, disposition, favorite, organization_revision, favorite_revision,
           created_unix_ms, updated_unix_ms, NULL FROM sequence, organization_records;
         INSERT INTO organization_record_groups
         SELECT record_id, (SELECT group_id FROM organization_record_groups LIMIT 1)
         FROM organization_records WHERE record_id LIKE 'lineage-%';
         INSERT INTO organization_bindings
         SELECT record.record_id, record.record_id, 100 + CAST(substr(record.record_id, 9) AS INTEGER),
           binding.byte_length, NULL, NULL, NULL, binding.fingerprint_scheme, binding.digest
         FROM organization_records AS record, organization_bindings AS binding
         WHERE record.record_id LIKE 'lineage-%';"
    ).unwrap();
    publish(&mut catalog, "c", 'a');
    let generation = selected(&catalog, "c").source_generation;
    let ticks = Arc::new(AtomicUsize::new(0));
    let observed = ticks.clone();
    catalog
        .connection
        .progress_handler(
            100,
            Some(move || {
                observed.fetch_add(1, Ordering::Relaxed);
                false
            }),
        )
        .unwrap();
    let result = catalog.publish_exact_analysis_batch("organization-fixture", &[generation]);
    catalog
        .connection
        .progress_handler(0, None::<fn() -> bool>)
        .unwrap();
    result.unwrap();
    let steps = ticks.load(Ordering::Relaxed) * 100;
    assert!(
        steps < 20_000,
        "single-copy publication used at least {steps} SQLite steps"
    );
    assert_eq!(
        decision(&catalog, "c"),
        (Some(album), "organized".into(), false)
    );
}

#[test]
fn stale_generation_publication_cannot_attach_changed_content_to_an_old_album() {
    let (_directory, mut catalog) = fixture();
    let album = create_album(&mut catalog, "Old");
    change(
        &mut catalog,
        "assign",
        &["a"],
        OrganizationAction::AssignAlbum { album_id: album },
    );
    publish(&mut catalog, "a", 'a');
    let old_generation = selected(&catalog, "a").source_generation;
    catalog.connection.execute("UPDATE asset_locations SET source_generation = source_generation + 100 WHERE location_id = 'a'", []).unwrap();
    catalog
        .publish_exact_analysis_batch("organization-fixture", &[old_generation])
        .unwrap();
    let prepared = prepare(&catalog, &["a"]);
    catalog
        .set_organization_favorite("new-default", &prepared, false)
        .unwrap();
    assert_eq!(decision(&catalog, "a"), (None, "pending".into(), false));
}
