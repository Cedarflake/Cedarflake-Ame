use crate::adapters::sqlite_catalog::tests::support::{
    load_default_snapshot, preview_lifecycle_state, preview_reference_count,
    publish_gallery_fixture, publish_preview_artifact,
};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use tempfile::tempdir;

use crate::ports::{CatalogMaintenanceAttempt, CatalogMaintenanceControl, CatalogSpaceRepository};

use crate::adapters::sqlite_catalog::*;

#[test]
fn unregistering_root_detaches_preview_references_before_location_identity_can_return() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let mut catalog = SqliteCatalog::open(path).expect("catalog");
    let root_id = "unregister-preview-root";
    let location_id = "unregister-preview-location";
    publish_gallery_fixture(
        &mut catalog,
        "unregister-preview-scan",
        root_id,
        "C:\\UnregisterPreviewSource",
        &[(location_id, None, 30)],
    );
    let artifact = publish_preview_artifact(
        &mut catalog,
        location_id,
        "unregister-preview-artifact",
        "C:\\AmeCache\\unregister-preview.jpg",
    );

    assert!(catalog.unregister_root(root_id).expect("unregister root"));
    assert_eq!(preview_reference_count(&catalog, &artifact.artifact_key), 0);
    assert_eq!(
        preview_lifecycle_state(&catalog, &artifact.artifact_key),
        "stale"
    );

    publish_gallery_fixture(
        &mut catalog,
        "readded-preview-scan",
        root_id,
        "C:\\UnregisterPreviewSource",
        &[(location_id, None, 40)],
    );
    let candidates = catalog
        .load_preview_reclamation_candidates(
            &[location_id.to_owned()],
            "ame-jpeg-thumbnail",
            2,
            "exif-display-v1",
            "c:\\amecache\\",
            8,
        )
        .expect("reclamation candidates after re-registering root");
    assert_eq!(
        candidates
            .iter()
            .map(|candidate| candidate.artifact_key.as_str())
            .collect::<Vec<_>>(),
        [artifact.artifact_key.as_str()]
    );
}

#[test]
fn unregistering_root_preserves_shared_assets_and_preview_owners() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let mut catalog = SqliteCatalog::open(path.clone()).expect("catalog");
    publish_gallery_fixture(
        &mut catalog,
        "shared-remove-scan",
        "shared-remove-root",
        "C:\\SharedRemove",
        &[("shared-remove-location", None, 30)],
    );
    publish_gallery_fixture(
        &mut catalog,
        "shared-keep-scan",
        "shared-keep-root",
        "C:\\SharedKeep",
        &[("shared-keep-location", None, 30)],
    );
    let shared_asset_id = catalog
        .load_active_location("shared-remove-location")
        .expect("removed location query")
        .expect("removed location")
        .asset_id;
    let replaced_asset_id = catalog
        .load_active_location("shared-keep-location")
        .expect("kept location query")
        .expect("kept location")
        .asset_id;
    catalog
        .connection
        .execute(
            "UPDATE asset_locations SET asset_id = ?1 WHERE location_id = ?2",
            params![shared_asset_id, "shared-keep-location"],
        )
        .expect("share logical asset across roots");
    catalog
        .connection
        .execute("DELETE FROM assets WHERE id = ?1", [replaced_asset_id])
        .expect("remove replaced fixture asset");

    let artifact = publish_preview_artifact(
        &mut catalog,
        "shared-remove-location",
        "shared-remove-artifact",
        "C:\\AmeCache\\shared-remove.jpg",
    );
    publish_preview_artifact(
        &mut catalog,
        "shared-keep-location",
        "shared-remove-artifact",
        "C:\\AmeCache\\shared-remove.jpg",
    );
    assert_eq!(preview_reference_count(&catalog, &artifact.artifact_key), 2);

    assert!(
        catalog
            .unregister_root("shared-remove-root")
            .expect("unregister one shared root")
    );

    let kept = catalog
        .load_active_location("shared-keep-location")
        .expect("kept location query")
        .expect("kept shared location");
    assert_eq!(kept.asset_id, shared_asset_id);
    assert_eq!(preview_reference_count(&catalog, &artifact.artifact_key), 1);
    assert_eq!(
        preview_lifecycle_state(&catalog, &artifact.artifact_key),
        "ready"
    );
    let shared_asset_count: i64 = catalog
        .connection
        .query_row(
            "SELECT COUNT(*) FROM assets WHERE id = ?1",
            [shared_asset_id.clone()],
            |row| row.get(0),
        )
        .expect("shared asset count");
    assert_eq!(shared_asset_count, 1);

    drop(catalog);
    let cancelled = Arc::new(AtomicBool::new(true));
    let control = CatalogMaintenanceControl::new(cancelled);
    let maintenance = SqliteCatalogSpaceMaintenance::new(path.clone());
    assert!(matches!(
        maintenance
            .try_reclaim_incremental(256, &control)
            .expect("cancelled post-removal maintenance result"),
        CatalogMaintenanceAttempt::Interrupted
    ));

    let catalog = SqliteCatalog::open(path).expect("reopen after interrupted maintenance");
    let removed_root_count: i64 = catalog
        .connection
        .query_row(
            "SELECT COUNT(*) FROM library_roots WHERE id = 'shared-remove-root'",
            [],
            |row| row.get(0),
        )
        .expect("removed root count after maintenance interruption");
    assert_eq!(removed_root_count, 0);
    let kept = catalog
        .load_active_location("shared-keep-location")
        .expect("kept location query after maintenance interruption")
        .expect("kept shared location after maintenance interruption");
    assert_eq!(kept.asset_id, shared_asset_id);
    assert_eq!(preview_reference_count(&catalog, &artifact.artifact_key), 1);
    assert_eq!(
        preview_lifecycle_state(&catalog, &artifact.artifact_key),
        "ready"
    );
}

#[test]
fn unregistering_root_does_not_rewrite_unrelated_orphan_state() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let mut catalog = SqliteCatalog::open(path).expect("catalog");
    publish_gallery_fixture(
        &mut catalog,
        "bounded-remove-scan",
        "bounded-remove-root",
        "C:\\BoundedRemove",
        &[("bounded-remove-location", None, 30)],
    );
    publish_gallery_fixture(
        &mut catalog,
        "bounded-keep-scan",
        "bounded-keep-root",
        "C:\\BoundedKeep",
        &[("bounded-keep-location", None, 40)],
    );
    let removed_artifact = publish_preview_artifact(
        &mut catalog,
        "bounded-remove-location",
        "bounded-remove-artifact",
        "C:\\AmeCache\\bounded-remove.jpg",
    );
    let unrelated_artifact = publish_preview_artifact(
        &mut catalog,
        "bounded-keep-location",
        "bounded-unrelated-artifact",
        "C:\\AmeCache\\bounded-unrelated.jpg",
    );
    catalog
        .connection
        .execute(
            "DELETE FROM preview_artifact_locations WHERE artifact_key = ?1",
            [&unrelated_artifact.artifact_key],
        )
        .expect("make unrelated preview artifact unreferenced");
    catalog
        .connection
        .execute(
            "INSERT INTO assets(id, created_unix_ms) VALUES ('unrelated-orphan-asset', 1)",
            [],
        )
        .expect("insert unrelated orphan asset");

    assert!(
        catalog
            .unregister_root("bounded-remove-root")
            .expect("unregister bounded root")
    );

    assert_eq!(
        preview_lifecycle_state(&catalog, &removed_artifact.artifact_key),
        "stale"
    );
    assert_eq!(
        preview_lifecycle_state(&catalog, &unrelated_artifact.artifact_key),
        "ready"
    );
    let unrelated_orphan_count: i64 = catalog
        .connection
        .query_row(
            "SELECT COUNT(*) FROM assets WHERE id = 'unrelated-orphan-asset'",
            [],
            |row| row.get(0),
        )
        .expect("unrelated orphan count");
    assert_eq!(unrelated_orphan_count, 1);
    assert!(
        catalog
            .load_active_location("bounded-keep-location")
            .expect("unrelated location query")
            .is_some()
    );
}

#[test]
fn unregistering_root_reclaims_only_its_terminal_handoff_owners() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let mut catalog = SqliteCatalog::open(path).expect("catalog");
    publish_gallery_fixture(
        &mut catalog,
        "handoff-scope-remove-scan",
        "handoff-scope-remove-root",
        "C:\\HandoffScopeRemove",
        &[("handoff-scope-remove-location", None, 30)],
    );
    publish_gallery_fixture(
        &mut catalog,
        "handoff-scope-keep-scan",
        "handoff-scope-keep-root",
        "C:\\HandoffScopeKeep",
        &[
            ("handoff-scope-owned-preview-location", None, 40),
            ("handoff-scope-unrelated-preview-location", None, 41),
        ],
    );
    let owned_artifact = publish_preview_artifact(
        &mut catalog,
        "handoff-scope-owned-preview-location",
        "handoff-scope-owned-artifact",
        "C:\\AmeCache\\handoff-scope-owned.jpg",
    );
    let unrelated_artifact = publish_preview_artifact(
        &mut catalog,
        "handoff-scope-unrelated-preview-location",
        "handoff-scope-unrelated-artifact",
        "C:\\AmeCache\\handoff-scope-unrelated.jpg",
    );
    catalog
        .connection
        .execute_batch(
            "DELETE FROM preview_artifact_locations
               WHERE artifact_key IN (
                 'handoff-scope-owned-artifact',
                 'handoff-scope-unrelated-artifact'
               );
             UPDATE asset_locations
               SET preview_path = '', preview_status = 'pending'
               WHERE location_id IN (
                 'handoff-scope-owned-preview-location',
                 'handoff-scope-unrelated-preview-location'
               );
             INSERT INTO assets(id, created_unix_ms) VALUES
               ('handoff-scope-owned-asset', 50),
               ('handoff-scope-unrelated-orphan', 50);
             INSERT INTO scan_run_catch_up_lineage(
               scan_id, catch_up_source, catch_up_watermark, enrolled_unix_ms
             ) VALUES (
               'handoff-scope-remove-scan',
               'handoff-scope-source', 'handoff-scope-watermark', 50
             );
             INSERT INTO library_change_catch_up_handoffs(
               catch_up_source, catch_up_watermark,
               file_identity_scheme, file_identity_value,
               asset_id, source_location_id, root_id, absolute_path, relative_path,
               preview_path, file_size, created_unix_ms, modified_unix_ms,
               width, height, preview_status, metadata_engine_id,
               metadata_engine_version, updated_unix_ms
             ) VALUES (
               'handoff-scope-source', 'handoff-scope-watermark',
               'windows-file-id-128-v1', 'handoff-scope-identity',
               'handoff-scope-owned-asset', 'handoff-scope-source-location',
               'handoff-scope-remove-root',
               'C:/HandoffScopeRemove/owned.jpg', 'owned.jpg',
               'C:\\AmeCache\\handoff-scope-owned.jpg',
               1, 50, 50, 1, 1, 'ready', 'fixture-metadata', '1', 50
             );",
        )
        .expect("seed terminal handoff and unrelated orphan state");
    catalog
        .connection
        .execute_batch(
            "CREATE TEMP TRIGGER fail_scoped_handoff_root_unregister
             BEFORE DELETE ON library_roots
             BEGIN
               SELECT RAISE(ABORT, 'injected scoped handoff unregister failure');
             END;",
        )
        .expect("install scoped handoff unregister failure fixture");

    let error = catalog
        .unregister_root("handoff-scope-remove-root")
        .expect_err("scoped handoff cleanup must roll back atomically");
    assert_eq!(error.code, "catalog_database_error");
    assert_eq!(
        preview_lifecycle_state(&catalog, &owned_artifact.artifact_key),
        "ready"
    );
    let rolled_back: (i64, i64) = catalog
        .connection
        .query_row(
            "SELECT
               (SELECT COUNT(*) FROM assets
                WHERE id = 'handoff-scope-owned-asset'),
               (SELECT COUNT(*) FROM library_change_catch_up_handoffs
                WHERE catch_up_source = 'handoff-scope-source'
                  AND catch_up_watermark = 'handoff-scope-watermark')",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("rolled-back scoped handoff projection");
    assert_eq!(rolled_back, (1, 1));
    catalog
        .connection
        .execute_batch("DROP TRIGGER fail_scoped_handoff_root_unregister;")
        .expect("remove scoped handoff unregister failure fixture");

    assert!(
        catalog
            .unregister_root("handoff-scope-remove-root")
            .expect("unregister handoff owner root")
    );

    assert_eq!(
        preview_lifecycle_state(&catalog, &owned_artifact.artifact_key),
        "stale"
    );
    assert_eq!(
        preview_lifecycle_state(&catalog, &unrelated_artifact.artifact_key),
        "ready"
    );
    let projection: (i64, i64, i64) = catalog
        .connection
        .query_row(
            "SELECT
               (SELECT COUNT(*) FROM assets
                WHERE id = 'handoff-scope-owned-asset'),
               (SELECT COUNT(*) FROM assets
                WHERE id = 'handoff-scope-unrelated-orphan'),
               (SELECT COUNT(*) FROM library_change_catch_up_handoffs
                WHERE catch_up_source = 'handoff-scope-source'
                  AND catch_up_watermark = 'handoff-scope-watermark')",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("post-unregister handoff cleanup projection");
    assert_eq!(projection, (0, 1, 0));
}

#[test]
fn unregistering_root_rolls_back_its_complete_catalog_cleanup() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let mut catalog = SqliteCatalog::open(path).expect("catalog");
    publish_gallery_fixture(
        &mut catalog,
        "rollback-remove-scan",
        "rollback-remove-root",
        "C:\\RollbackRemove",
        &[("rollback-remove-location", None, 30)],
    );
    let artifact = publish_preview_artifact(
        &mut catalog,
        "rollback-remove-location",
        "rollback-remove-artifact",
        "C:\\AmeCache\\rollback-remove.jpg",
    );
    let revision_before = load_default_snapshot(&mut catalog, 10, None)
        .expect("snapshot before rollback fixture")
        .revision;
    catalog
        .connection
        .execute_batch(
            "CREATE TEMP TRIGGER fail_root_unregister
             BEFORE DELETE ON library_roots
             BEGIN
               SELECT RAISE(ABORT, 'injected root unregister failure');
             END;",
        )
        .expect("install unregister failure fixture");

    let error = catalog
        .unregister_root("rollback-remove-root")
        .expect_err("injected unregister failure must roll back");
    assert_eq!(error.code, "catalog_database_error");

    let snapshot =
        load_default_snapshot(&mut catalog, 10, None).expect("snapshot after rollback fixture");
    assert_eq!(snapshot.revision, revision_before);
    assert_eq!(snapshot.roots.len(), 1);
    assert!(
        catalog
            .load_active_location("rollback-remove-location")
            .expect("rolled back location query")
            .is_some()
    );
    assert_eq!(preview_reference_count(&catalog, &artifact.artifact_key), 1);
    assert_eq!(
        preview_lifecycle_state(&catalog, &artifact.artifact_key),
        "ready"
    );
}
