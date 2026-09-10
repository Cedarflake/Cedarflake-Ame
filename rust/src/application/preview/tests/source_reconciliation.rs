use std::os::windows::fs::OpenOptionsExt;

use crate::domain::{
    LibraryChangeIntent, LibraryChangeIntentKind, LibraryChangeOrigin, LibraryChangeQueuePolicy,
    LibraryChangeScope,
};
use crate::ports::{
    IncrementalCatalogRepository, LibraryChangeQueue, SourceReconciliationRepository,
};
use rusqlite::{Connection, params};

use super::*;

fn ready_source_fixture(suffix: &str) -> (PreviewFixture, AssetLocationView) {
    let fixture = preview_fixture(suffix);
    let identity = crate::adapters::file_identity_evidence(&fixture.source_path)
        .expect("file identity query")
        .expect("file identity");
    let connection = Connection::open(&fixture.storage.catalog_path).expect("fixture catalog");
    connection
        .execute(
            "UPDATE asset_locations SET file_identity_scheme = ?1, file_identity_value = ?2
         WHERE location_id = ?3",
            params![identity.scheme, identity.value, fixture.request.location_id],
        )
        .expect("retain source identity");
    drop(connection);
    let ready = materialize_preview_with_storage(fixture.request.clone(), fixture.storage.clone())
        .expect("initial preview");
    assert_eq!(ready.preview_status, PreviewStatus::Ready);
    (fixture, ready)
}

fn active_location(fixture: &PreviewFixture) -> AssetLocationView {
    SqliteCatalog::open(fixture.storage.catalog_path.clone())
        .expect("catalog")
        .load_active_location(&fixture.request.location_id)
        .expect("location read")
        .expect("active location")
}

fn queue_row(path: &Path, id: i64) -> Vec<rusqlite::types::Value> {
    let connection = Connection::open(path).expect("catalog");
    let mut statement = connection
        .prepare("SELECT * FROM library_change_queue WHERE id = ?1")
        .expect("queue statement");
    let count = statement.column_count();
    statement
        .query_row([id], |row| (0..count).map(|index| row.get(index)).collect())
        .expect("queue row")
}

#[test]
fn source_reconciliation_preserves_existing_authority_and_rejects_stale_admission() {
    for scope in [LibraryChangeScope::Root, LibraryChangeScope::Path] {
        let fixture = preview_fixture("source-admission");
        let mut catalog =
            SqliteCatalog::open(fixture.storage.catalog_path.clone()).expect("catalog");
        let root = catalog
            .load_incremental_catalog_root(&fixture.request.expected_root_id)
            .expect("root query")
            .expect("root");
        let policy = LibraryChangeQueuePolicy {
            debounce_millis: 0,
            max_lease_batch: 1,
            ..LibraryChangeQueuePolicy::default()
        };
        let intent = LibraryChangeIntent {
            root_id: root.root_id,
            root_generation: root.root_generation,
            kind: LibraryChangeIntentKind::Reconcile,
            scope: LibraryChangeScope::Path,
            relative_path: "source.png".to_owned(),
            previous_relative_path: None,
            origin: LibraryChangeOrigin::ConsistencyAudit,
            first_observed_unix_ms: 100,
            most_recent_observed_unix_ms: 100,
            first_sequence: 0,
            most_recent_sequence: 0,
            coalesced_observation_count: 1,
        };
        let mut existing = intent.clone();
        existing.scope = scope;
        if scope == LibraryChangeScope::Root {
            existing.kind = LibraryChangeIntentKind::FreshnessUnknown;
            existing.relative_path.clear();
        } else {
            existing.origin = LibraryChangeOrigin::MetadataInventory;
        }
        catalog
            .enqueue_library_change_intents(&[existing], 100, policy)
            .expect("existing work");
        let connection = Connection::open(&fixture.storage.catalog_path).expect("catalog");
        let id: i64 = connection
            .query_row("SELECT MAX(id) FROM library_change_queue", [], |row| {
                row.get(0)
            })
            .expect("existing ID");
        drop(connection);
        let before = queue_row(&fixture.storage.catalog_path, id);
        let mut stale = fixture.request.clone();
        stale.expected_source_generation += 1;
        assert!(
            catalog
                .admit_source_reconciliation(&stale, &intent, 101, policy)
                .expect("stale admission")
                .is_none()
        );
        assert_eq!(queue_row(&fixture.storage.catalog_path, id), before);
        let admitted = catalog
            .admit_source_reconciliation(&fixture.request, &intent, 101, policy)
            .expect("bounded source admission");
        assert_eq!(admitted.is_some(), scope == LibraryChangeScope::Root);
        assert_eq!(queue_row(&fixture.storage.catalog_path, id), before);
        assert!(
            catalog
                .admit_source_reconciliation(&fixture.request, &intent, 102, policy)
                .expect("duplicate admission")
                .is_none()
        );
        assert_eq!(queue_row(&fixture.storage.catalog_path, id), before);
    }
}

#[test]
fn source_revision_mismatch_reconciles_one_path_and_recovers_preview() {
    let (fixture, ready) = ready_source_fixture("source-reconciliation");
    let modified = fixture
        .source_path
        .metadata()
        .expect("metadata")
        .modified()
        .expect("modified");
    let mut replacement = Cursor::new(Vec::new());
    image::DynamicImage::ImageRgb8(RgbImage::from_pixel(32, 24, Rgb([192, 96, 24])))
        .write_to(&mut replacement, image::ImageFormat::Png)
        .expect("replacement PNG");
    let replacement = replacement.into_inner();
    assert_eq!(replacement.len() as u64, ready.file_size);
    fs::write(&fixture.source_path, &replacement).expect("replace fixture content");
    fs::OpenOptions::new()
        .write(true)
        .open(&fixture.source_path)
        .expect("fixture handle")
        .set_times(fs::FileTimes::new().set_modified(modified))
        .expect("restore modification time");
    let old_request = preview_request_for(&ready, 256, false);
    let error = materialize_preview_with_storage(old_request.clone(), fixture.storage.clone())
        .expect_err("old source request is superseded after reconciliation");
    assert_eq!(error.code, "preview_request_superseded");
    assert!(
        error
            .message
            .contains("source_revision_changed_during_scan")
    );
    let reconciled = active_location(&fixture);
    assert_eq!(reconciled.asset_id, ready.asset_id);
    assert_eq!(reconciled.file_identity, ready.file_identity);
    assert_eq!(reconciled.file_size, ready.file_size);
    assert_eq!(reconciled.modified_unix_ms, ready.modified_unix_ms);
    assert_ne!(reconciled.source_revision, ready.source_revision);
    assert!(reconciled.source_generation > ready.source_generation);
    assert_eq!(reconciled.preview_status, PreviewStatus::Pending);
    assert!(reconciled.preview_path.is_empty());
    assert_eq!(
        materialize_preview_with_storage(old_request, fixture.storage.clone())
            .expect_err("old generation remains rejected")
            .code,
        "preview_request_superseded"
    );
    let refreshed = materialize_preview_with_storage(
        preview_request_for(&reconciled, 256, false),
        fixture.storage.clone(),
    )
    .expect("new source preview");
    assert_eq!(refreshed.preview_status, PreviewStatus::Ready);
    assert_ne!(refreshed.preview_path, ready.preview_path);
    let pixels = image::open(&refreshed.preview_path)
        .expect("new preview pixels")
        .to_rgb8();
    let actual = pixels.get_pixel(pixels.width() / 2, pixels.height() / 2).0;
    assert!(
        actual
            .into_iter()
            .zip([192_u8, 96, 24])
            .all(|(actual, expected)| actual.abs_diff(expected) <= 8)
    );
    assert_eq!(
        fs::read(&fixture.source_path).expect("source bytes"),
        replacement
    );
    let connection = Connection::open(&fixture.storage.catalog_path).expect("catalog");
    let completed: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM library_change_queue WHERE origin = 'consistency_audit'
           AND scope = 'path' AND relative_path = 'source.png' AND status = 'completed'",
            [],
            |row| row.get(0),
        )
        .expect("completed path reconciliation");
    assert_eq!(completed, 1);
}

#[test]
fn source_open_failure_keeps_its_cause_and_does_not_enqueue_reconciliation() {
    let (fixture, ready) = ready_source_fixture("source-locked");
    let locked = fs::OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(&fixture.source_path)
        .expect("exclusive fixture handle");
    let error = materialize_preview_with_storage(
        preview_request_for(&ready, 256, false),
        fixture.storage.clone(),
    )
    .expect_err("source is locked");
    assert_eq!(error.code, "preview_source_open_failed");
    drop(locked);
    assert_eq!(active_location(&fixture), ready);
    let connection = Connection::open(&fixture.storage.catalog_path).expect("catalog");
    let count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM library_change_queue WHERE origin = 'consistency_audit'",
            [],
            |row| row.get(0),
        )
        .expect("no source reconciliation");
    assert_eq!(count, 0);
    assert_eq!(
        materialize_preview_with_storage(
            preview_request_for(&ready, 256, true),
            fixture.storage.clone()
        )
        .expect("unlocked preview")
        .preview_status,
        PreviewStatus::Ready
    );
}
