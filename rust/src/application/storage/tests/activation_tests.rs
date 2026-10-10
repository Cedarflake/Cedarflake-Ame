use std::panic::{AssertUnwindSafe, catch_unwind};

use rusqlite::Connection;

use crate::domain::{
    LibraryChangeCatchUpEvidence, LibraryChangeIntent, LibraryChangeIntentKind,
    LibraryChangeOrigin, LibraryChangeQueuePolicy, LibraryChangeScope,
};
use crate::ports::{IncrementalCatalogRepository, LibraryChangeQueue};

use super::*;

#[test]
fn interruption_before_catalog_reset_keeps_pending_ownership_for_complete_reopen_reset() {
    let storage = tempdir().expect("storage");
    let catalog_path = storage.path().join("catalog.sqlite3");
    let settings_path = storage.path().join("settings.sqlite3");
    let source = storage.path().join("source");
    let old = storage.path().join("old");
    let target = storage.path().join("target");
    publish_storage_fixture(&catalog_path, &source, &old);
    seed_handoffs(&catalog_path);
    let configured = save_pending_preview_target(&settings_path, &catalog_path, &old, &target);
    let mut reached_initialization = false;
    let interrupted = catch_unwind(AssertUnwindSafe(|| {
        activate_configured_preview_root_with(&settings_path, &catalog_path, &configured, |_, _| {
            reached_initialization = true;
            panic!("interrupted before catalog reset")
        })
    }));
    assert!(
        reached_initialization,
        "the injected interruption must be reached"
    );
    assert!(interrupted.is_err());
    assert_pending(&settings_path, &old);
    assert_eq!(ready_owner_count(&catalog_path), 3);
    assert_eq!(
        activate_configured_preview_root(&settings_path, &catalog_path, &configured)
            .expect("reopened activation"),
        target
    );
    assert_reset(&catalog_path);
    assert!(old.join("preview.jpg").exists());
}

#[test]
fn interruption_after_catalog_reset_preserves_pending_until_settings_can_retire_it() {
    let storage = tempdir().expect("storage");
    let catalog_path = storage.path().join("catalog.sqlite3");
    let settings_path = storage.path().join("settings.sqlite3");
    let source = storage.path().join("source");
    let old = storage.path().join("old");
    let target = storage.path().join("target");
    publish_storage_fixture(&catalog_path, &source, &old);
    seed_handoffs(&catalog_path);
    let configured = save_pending_preview_target(&settings_path, &catalog_path, &old, &target);
    Connection::open(&settings_path)
        .expect("settings")
        .execute_batch(
            "CREATE TRIGGER fail_retirement BEFORE UPDATE OF state ON preview_root_ownership
         BEGIN SELECT RAISE(ABORT, 'interrupted retirement'); END;",
        )
        .expect("retirement interruption");
    assert_eq!(
        activate_configured_preview_root_with(
            &settings_path,
            &catalog_path,
            &configured,
            |_, _| Ok(())
        )
        .expect("safe fallback"),
        old
    );
    assert_pending(&settings_path, &old);
    assert_reset(&catalog_path);
    // A fallback process may demand old-root previews again. The pending obligation must
    // still reset unseen handoffs on its next activation, without relying on a gallery read.
    Connection::open(&catalog_path).expect("fallback handoff").execute(
        "UPDATE library_change_scan_handoff_items SET preview_status = 'ready', preview_path = ?1",
        [old.join("preview.jpg").to_string_lossy().as_ref()],
    ).expect("fallback handoff preview");
    Connection::open(&settings_path)
        .expect("reopened settings")
        .execute_batch("DROP TRIGGER fail_retirement")
        .expect("retirement available");
    assert_eq!(
        activate_configured_preview_root(&settings_path, &catalog_path, &configured)
            .expect("idempotent reopened reset"),
        target
    );
    assert_reset(&catalog_path);
    let mut settings = SqliteStorageSettings::open(settings_path).expect("settings after");
    assert!(
        settings
            .load_pending_preview_roots()
            .expect("pending after")
            .is_empty()
    );
    assert_eq!(
        settings
            .load_retired_preview_roots()
            .expect("retired after"),
        vec![old.to_string_lossy().into_owned()]
    );
}

#[test]
fn fallback_does_not_initialize_a_former_cache_path_now_inside_a_source_root() {
    let storage = tempdir().expect("storage");
    let catalog_path = storage.path().join("catalog.sqlite3");
    let settings_path = storage.path().join("settings.sqlite3");
    let source = storage.path().join("source");
    let old = source.join("former-cache");
    let target = storage.path().join("target");
    publish_storage_fixture(&catalog_path, &source, &storage.path().join("real-cache"));
    let configured = save_pending_preview_target(&settings_path, &catalog_path, &old, &target);
    let mut initialized = Vec::new();
    let error = activate_configured_preview_root_with(
        &settings_path,
        &catalog_path,
        &configured,
        |path, _| {
            initialized.push(path.to_path_buf());
            Err(ScanError::new("target_offline", "unavailable target"))
        },
    )
    .expect_err("unsafe fallback rejected");
    assert_eq!(error.code, "target_offline");
    assert_eq!(initialized, vec![target]);
    assert!(!old.exists());
    assert_pending(&settings_path, &old);
}

fn assert_pending(settings_path: &Path, old: &Path) {
    let mut settings =
        SqliteStorageSettings::open(settings_path.to_path_buf()).expect("reopened settings");
    assert_eq!(
        settings.load_pending_preview_roots().expect("pending"),
        vec![old.to_string_lossy().into_owned()]
    );
    assert!(
        settings
            .load_retired_preview_roots()
            .expect("retired")
            .is_empty()
    );
}

fn ready_owner_count(path: &Path) -> i64 {
    Connection::open(path).expect("owner query").query_row(
        "SELECT (SELECT COUNT(*) FROM asset_locations WHERE preview_status = 'ready') +
                (SELECT COUNT(*) FROM library_change_catch_up_handoffs WHERE preview_status = 'ready') +
                (SELECT COUNT(*) FROM library_change_scan_handoff_items WHERE preview_status = 'ready')",
        [], |row| row.get(0),
    ).expect("ready owners")
}

fn assert_reset(path: &Path) {
    assert_eq!(ready_owner_count(path), 0);
    let connection = Connection::open(path).expect("reset verification");
    let artifacts: i64 = connection
        .query_row("SELECT COUNT(*) FROM preview_artifacts", [], |row| {
            row.get(0)
        })
        .expect("artifact index");
    assert_eq!(artifacts, 0);
    for table in [
        "asset_locations",
        "library_change_catch_up_handoffs",
        "library_change_scan_handoff_items",
    ] {
        let (width, height, status, preview_path): (u32, u32, String, String) = connection
            .query_row(
                &format!("SELECT width, height, preview_status, preview_path FROM {table}"),
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .expect("durable dimensions");
        assert_eq!((width, height), (40, 50));
        assert_eq!(status, "pending");
        assert!(preview_path.is_empty());
    }
}

fn seed_handoffs(path: &Path) {
    let mut catalog = SqliteCatalog::open(path.to_path_buf()).expect("handoff consumer catalog");
    let root = catalog
        .load_incremental_catalog_root("storage-root")
        .expect("handoff root query")
        .expect("published handoff root");
    let report = catalog
        .enqueue_library_change_intents_with_catch_up(
            &[LibraryChangeIntent {
                root_id: root.root_id,
                root_generation: root.root_generation,
                kind: LibraryChangeIntentKind::Reconcile,
                scope: LibraryChangeScope::Path,
                relative_path: "one.png".to_owned(),
                previous_relative_path: None,
                origin: LibraryChangeOrigin::StartupCatchUp,
                first_observed_unix_ms: 1,
                most_recent_observed_unix_ms: 1,
                first_sequence: 1,
                most_recent_sequence: 1,
                coalesced_observation_count: 1,
            }],
            &LibraryChangeCatchUpEvidence {
                source: "fixture".to_owned(),
                watermark: "pending-transition".to_owned(),
            },
            1,
            LibraryChangeQueuePolicy::default(),
        )
        .expect("durable unresolved handoff consumer");
    assert_eq!(report.inserted_count, 1);
    drop(catalog);
    let connection = Connection::open(path).expect("handoff fixtures");
    connection.execute_batch(
        "INSERT INTO library_change_catch_up_handoffs(
           catch_up_source, catch_up_watermark, file_identity_scheme, file_identity_value,
           asset_id, source_location_id, root_id, absolute_path, relative_path,
           preview_path, file_size, created_unix_ms, modified_unix_ms, width, height,
           preview_status, metadata_engine_id, metadata_engine_version, updated_unix_ms,
           source_revision_token, source_generation)
         SELECT 'fixture', 'pending-transition', 'fixture-identity', 'one',
                asset_id, location_id, root_id, absolute_path, relative_path,
                preview_path, file_size, created_unix_ms, modified_unix_ms, width, height,
                preview_status, metadata_engine_id, metadata_engine_version, 1,
                source_revision_token, source_generation
         FROM asset_locations;
         INSERT INTO library_change_scan_handoff_batches(id, source_root_id, updated_unix_ms)
         VALUES ('activation-batch', 'storage-root', 1);
         INSERT INTO library_change_scan_handoff_lineage(
           batch_id, catch_up_source, catch_up_watermark, enrolled_unix_ms)
         SELECT 'activation-batch', catch_up_source, catch_up_watermark, enrolled_unix_ms
         FROM library_change_queue_catch_up_lineage
         WHERE catch_up_source = 'fixture' AND catch_up_watermark = 'pending-transition';
         INSERT INTO library_change_scan_handoff_items(
           batch_id, file_identity_scheme, file_identity_value, asset_id, source_location_id,
           root_id, absolute_path, relative_path, preview_path, file_size, created_unix_ms,
           modified_unix_ms, width, height, preview_status, metadata_engine_id, metadata_engine_version,
           source_revision_token, source_generation)
         SELECT 'activation-batch', file_identity_scheme, file_identity_value, asset_id, source_location_id,
                root_id, absolute_path, relative_path, preview_path, file_size, created_unix_ms,
                modified_unix_ms, width, height, preview_status, metadata_engine_id, metadata_engine_version,
                source_revision_token, source_generation
         FROM library_change_catch_up_handoffs;",
    ).expect("both retained handoff owners");
    drop(connection);
    SqliteCatalog::open(path.to_path_buf()).expect("complete durable handoff ownership contract");
}
