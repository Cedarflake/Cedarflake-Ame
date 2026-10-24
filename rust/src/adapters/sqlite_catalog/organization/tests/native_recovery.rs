use crate::domain::GalleryQuery;
use crate::domain::organization_queue::OrganizationQueue;
use crate::domain::organization_review::OrganizationReviewSession;
use crate::ports::organization_review::OrganizationReviewRepository;

use super::transitions::{change, create_album, decision};
use super::*;

mod cost;
mod publication;

fn native_fixture() -> (tempfile::TempDir, SqliteCatalog) {
    let (directory, catalog) = fixture();
    catalog
        .connection
        .execute_batch(
            "UPDATE asset_locations SET file_identity_scheme = 'windows-file-id-128-v1',
           file_identity_value = printf('%016x:%032x', 1,
             CASE location_id WHEN 'a' THEN 1 WHEN 'b' THEN 2 ELSE 3 END);",
        )
        .unwrap();
    (directory, catalog)
}

fn retire_derived_bindings(catalog: &SqliteCatalog) {
    catalog
        .connection
        .execute_batch(
            "UPDATE organization_bindings SET asset_id = 'old-' || asset_id,
           source_generation = source_generation + 10000;
         UPDATE catalog_state SET next_source_generation = next_source_generation + 10001;
         UPDATE organization_records SET unassociated_since_unix_ms = 100;",
        )
        .unwrap();
}

fn publish_root(catalog: &SqliteCatalog, root: &str) -> Result<(), ScanError> {
    let transaction = catalog.connection.unchecked_transaction().unwrap();
    super::super::publication::root(&transaction, root)?;
    transaction.commit().unwrap();
    Ok(())
}

fn active_bindings(catalog: &SqliteCatalog) -> u64 {
    scalar(catalog,
        "SELECT COUNT(*) FROM organization_bindings AS binding JOIN asset_locations AS location
           ON location.asset_id = binding.asset_id AND location.source_generation = binding.source_generation",
    )
}

#[test]
fn organization_native_recovery_restores_album_favorite_and_continuation_after_rebuild() {
    let (directory, mut catalog) = native_fixture();
    let album = create_album(&mut catalog, "Saved");
    change(
        &mut catalog,
        "assign",
        &["a"],
        OrganizationAction::AssignAlbum {
            album_id: album.clone(),
        },
    );
    let favorite = prepare(&catalog, &["a"]);
    catalog
        .set_organization_favorite("favorite", &favorite, true)
        .unwrap();
    let session = catalog
        .begin_organization_review(&GalleryQuery {
            organization_queue: Some(OrganizationQueue::Pending),
            ..GalleryQuery::default()
        })
        .unwrap();
    catalog
        .checkpoint_organization_review(
            &OrganizationReviewSession {
                session_id: session.session_id,
                position_sequence: 1,
            },
            &selected(&catalog, "b"),
        )
        .unwrap();
    let revision = scalar(&catalog, "SELECT revision FROM organization_state");
    retire_derived_bindings(&catalog);
    assert_eq!(active_bindings(&catalog), 0);
    publish_root(&catalog, "root-a").unwrap();
    assert_eq!(
        decision(&catalog, "a"),
        (Some(album.clone()), "organized".into(), true)
    );
    assert_eq!(
        catalog
            .load_organization_continuation()
            .unwrap()
            .target
            .unwrap()
            .location_id,
        "b"
    );
    assert_eq!(active_bindings(&catalog), 2);
    assert_eq!(
        scalar(
            &catalog,
            "SELECT COUNT(*) FROM organization_records WHERE unassociated_since_unix_ms IS NOT NULL"
        ),
        0
    );
    publish_root(&catalog, "root-a").unwrap();
    assert_eq!(
        scalar(&catalog, "SELECT revision FROM organization_state"),
        revision
    );
    assert_eq!(
        scalar(&catalog, "SELECT COUNT(*) FROM organization_bindings"),
        4
    );
    drop(catalog);
    let catalog = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
    assert_eq!(
        decision(&catalog, "a"),
        (Some(album), "organized".into(), true)
    );
    assert_eq!(
        catalog
            .load_organization_continuation()
            .unwrap()
            .target
            .unwrap()
            .location_id,
        "b"
    );
}

#[test]
fn organization_native_recovery_rejects_replaced_changed_or_unproven_files() {
    for mutation in [
        "UPDATE asset_locations SET file_identity_value = '0000000000000002:00000000000000000000000000000001'",
        "UPDATE asset_locations SET source_revision_token = 'windows-file-change-time-100ns-v1:0000000000000002'",
        "UPDATE asset_locations SET file_size = 21",
        "UPDATE asset_locations SET file_identity_scheme = NULL, file_identity_value = NULL",
        "UPDATE asset_locations SET source_revision_token = NULL",
    ] {
        let (_directory, mut catalog) = native_fixture();
        let selected = prepare(&catalog, &["a"]);
        catalog
            .set_organization_favorite("favorite", &selected, true)
            .unwrap();
        retire_derived_bindings(&catalog);
        catalog.connection.execute_batch(mutation).unwrap();
        publish_root(&catalog, "root-a").unwrap();
        assert_eq!(active_bindings(&catalog), 0, "{mutation}");
        assert_eq!(
            scalar(
                &catalog,
                "SELECT COUNT(*) FROM organization_records WHERE favorite = 1"
            ),
            1
        );
    }
}

#[test]
fn organization_native_recovery_keeps_contradictory_versions_separate_until_exact_evidence() {
    let (_directory, mut catalog) = native_fixture();
    publish(&mut catalog, "a", 'a');
    publish(&mut catalog, "c", 'b');
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
    catalog.connection.execute_batch(
        "UPDATE organization_bindings SET file_identity_value = '0000000000000001:00000000000000000000000000000001';",
    ).unwrap();
    retire_derived_bindings(&catalog);
    catalog
        .connection
        .execute_batch(
            "DELETE FROM gallery_content_fingerprints;
         DELETE FROM content_fingerprints;",
        )
        .unwrap();
    publish_root(&catalog, "root-a").unwrap();
    assert_eq!(active_bindings(&catalog), 0);
    assert_eq!(
        scalar(
            &catalog,
            "SELECT COUNT(DISTINCT group_id) FROM organization_record_groups"
        ),
        2
    );
    publish(&mut catalog, "a", 'a');
    publish_root(&catalog, "root-a").unwrap();
    assert_eq!(
        decision(&catalog, "a"),
        (Some(first), "organized".into(), false)
    );
    assert_eq!(
        scalar(
            &catalog,
            "SELECT COUNT(DISTINCT group_id) FROM organization_record_groups"
        ),
        2
    );
}

#[test]
fn organization_native_recovery_does_not_override_a_contrary_current_fingerprint() {
    let (_directory, mut catalog) = native_fixture();
    publish(&mut catalog, "a", 'a');
    let favorite = prepare(&catalog, &["a"]);
    catalog
        .set_organization_favorite("favorite", &favorite, true)
        .unwrap();
    retire_derived_bindings(&catalog);
    catalog
        .connection
        .execute_batch("UPDATE content_fingerprints SET digest = printf('%064d', 0)")
        .unwrap();
    publish_root(&catalog, "root-a").unwrap();
    assert_eq!(active_bindings(&catalog), 0);
    assert_eq!(
        scalar(
            &catalog,
            "SELECT COUNT(*) FROM organization_records WHERE favorite = 1"
        ),
        1
    );
}

#[test]
fn organization_native_recovery_rolls_back_group_conflicts_when_attachment_fails() {
    let (_directory, mut catalog) = native_fixture();
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
        &["b"],
        OrganizationAction::AssignAlbum { album_id: second },
    );
    catalog.connection.execute_batch(
        "UPDATE organization_bindings SET file_identity_value = '0000000000000001:00000000000000000000000000000001';",
    ).unwrap();
    retire_derived_bindings(&catalog);
    catalog
        .connection
        .execute_batch(
            "CREATE TRIGGER reject_recovered_binding BEFORE INSERT ON organization_bindings
         BEGIN SELECT RAISE(ABORT, 'injected association failure'); END;",
        )
        .unwrap();
    assert!(publish_root(&catalog, "root-a").is_err());
    assert_eq!(active_bindings(&catalog), 0);
    assert_eq!(
        scalar(
            &catalog,
            "SELECT COUNT(DISTINCT group_id) FROM organization_record_groups"
        ),
        2
    );
    assert_eq!(
        scalar(&catalog, "SELECT COUNT(*) FROM organization_conflicts"),
        0
    );
    catalog
        .connection
        .execute_batch("DROP TRIGGER reject_recovered_binding")
        .unwrap();
    publish_root(&catalog, "root-a").unwrap();
    assert_eq!(decision(&catalog, "a"), (None, "deferred".into(), false));
    assert_eq!(
        scalar(&catalog, "SELECT COUNT(*) FROM organization_conflicts"),
        2
    );
}

#[test]
fn organization_native_recovery_cannot_resurrect_explicitly_removed_source_decisions() {
    let (_directory, mut catalog) = native_fixture();
    let favorite = prepare(&catalog, &["a"]);
    catalog
        .set_organization_favorite("favorite", &favorite, true)
        .unwrap();
    retire_derived_bindings(&catalog);
    catalog.unregister_root("root-a").unwrap();
    crate::adapters::sqlite_catalog::tests::publish_gallery_fixture(
        &mut catalog,
        "reimport",
        "root-a",
        "C:\\Fixture",
        &[("new-a", None, 1)],
    );
    catalog
        .connection
        .execute_batch(
            "UPDATE asset_locations SET file_identity_scheme = 'windows-file-id-128-v1',
           file_identity_value = '0000000000000001:00000000000000000000000000000001',
           source_revision_token = 'windows-file-change-time-100ns-v1:0000000000000001'
         WHERE location_id = 'new-a';",
        )
        .unwrap();
    publish_root(&catalog, "root-a").unwrap();
    assert_eq!(
        scalar(&catalog, "SELECT COUNT(*) FROM organization_records"),
        0
    );
    assert_eq!(active_bindings(&catalog), 0);
}
