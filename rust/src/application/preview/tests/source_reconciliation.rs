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
    verify_source_revision_mismatch(false);
}

#[test]
fn same_metadata_rewrite_after_decode_rejects_old_pixels_and_recovers_preview() {
    verify_source_revision_mismatch(true);
}

fn verify_source_revision_mismatch(after_decode: bool) {
    let (fixture, ready) = ready_source_fixture("source-reconciliation");
    let original_preview = fs::read(&ready.preview_path).expect("original preview bytes");
    let preview_entries = || {
        fs::read_dir(&fixture.storage.preview_root)
            .expect("preview directory")
            .map(|entry| entry.expect("preview entry").path())
            .collect::<std::collections::BTreeSet<_>>()
    };
    let original_entries = preview_entries();
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
    let changed_path = fixture.source_path.clone();
    let changed_bytes = replacement.clone();
    let rewrite_source = move || {
        fs::write(&changed_path, changed_bytes).expect("replace fixture content");
        fs::OpenOptions::new()
            .write(true)
            .open(changed_path)
            .expect("fixture handle")
            .set_times(fs::FileTimes::new().set_modified(modified))
            .expect("restore modification time");
    };
    if after_decode {
        install_preview_test_hook(
            &AFTER_PREVIEW_REVALIDATION_HOOKS,
            &ready.location_id,
            rewrite_source,
        );
    } else {
        rewrite_source();
    }
    let old_request = preview_request_for(&ready, 256, after_decode);
    let error = materialize_preview_with_storage(old_request.clone(), fixture.storage.clone())
        .expect_err("changed source supersedes the old preview request");
    assert_eq!(error.code, "preview_request_superseded");
    let error = if after_decode {
        assert_eq!(active_location(&fixture), ready);
        assert_eq!(
            fs::read(&ready.preview_path).expect("retained preview bytes"),
            original_preview
        );
        assert_eq!(preview_entries(), original_entries);
        assert_unique_ready_preview_owner(
            &fixture.storage.catalog_path,
            &ready.location_id,
            &ready.preview_path,
        );
        materialize_preview_with_storage(
            preview_request_for(&active_location(&fixture), 256, false),
            fixture.storage.clone(),
        )
        .expect_err("next ordinary demand reconciles the changed source")
    } else {
        error
    };
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
fn missing_source_reconciles_absence_without_publishing_preview_failure() {
    for has_preview in [false, true] {
        let fixture = preview_fixture("missing-source");
        let location = if has_preview {
            materialize_preview_with_storage(fixture.request.clone(), fixture.storage.clone())
                .expect("initial preview")
        } else {
            active_location(&fixture)
        };
        fs::remove_file(&fixture.source_path).expect("remove owned fixture source");
        let error = materialize_preview_with_storage(
            preview_request_for(&location, 256, false),
            fixture.storage.clone(),
        )
        .expect_err("removed source cannot generate a preview");
        assert_eq!(error.code, "preview_request_superseded");
        let catalog = SqliteCatalog::open(fixture.storage.catalog_path.clone()).expect("catalog");
        assert!(
            catalog
                .load_active_location(&location.location_id)
                .expect("location lookup")
                .is_none(),
            "only authoritative path reconciliation publishes absence"
        );
        assert!(fixture.source_path.parent().expect("root").is_dir());
        assert!(!fixture.source_path.exists());
    }
}

#[test]
fn missing_source_preserves_existing_path_work_and_retires_preview_request() {
    let (fixture, ready) = ready_source_fixture("missing-source-owned-path");
    let mut catalog = SqliteCatalog::open(fixture.storage.catalog_path.clone()).expect("catalog");
    let root = catalog
        .load_incremental_catalog_root(&ready.root_id)
        .expect("root query")
        .expect("root");
    let intent = LibraryChangeIntent {
        root_id: ready.root_id.clone(),
        root_generation: root.root_generation,
        kind: LibraryChangeIntentKind::Reconcile,
        scope: LibraryChangeScope::Path,
        relative_path: ready.relative_path.clone(),
        previous_relative_path: None,
        origin: LibraryChangeOrigin::MetadataInventory,
        first_observed_unix_ms: 100,
        most_recent_observed_unix_ms: 100,
        first_sequence: 0,
        most_recent_sequence: 0,
        coalesced_observation_count: 1,
    };
    catalog
        .enqueue_library_change_intents(&[intent], 100, LibraryChangeQueuePolicy::default())
        .expect("existing path work");
    let connection = Connection::open(&fixture.storage.catalog_path).expect("catalog");
    let id = connection
        .query_row("SELECT MAX(id) FROM library_change_queue", [], |row| {
            row.get(0)
        })
        .expect("queue ID");
    drop(connection);
    drop(catalog);
    let before = queue_row(&fixture.storage.catalog_path, id);
    fs::remove_file(&fixture.source_path).expect("remove owned fixture source");
    let error = materialize_preview_with_storage(
        preview_request_for(&ready, 256, false),
        fixture.storage.clone(),
    )
    .expect_err("removed source request retires");
    assert_eq!(error.code, "preview_request_superseded");
    assert_eq!(active_location(&fixture), ready);
    assert_eq!(queue_row(&fixture.storage.catalog_path, id), before);
    let connection = Connection::open(&fixture.storage.catalog_path).expect("catalog");
    let count: i64 = connection
        .query_row("SELECT COUNT(*) FROM library_change_queue", [], |row| {
            row.get(0)
        })
        .expect("queue count");
    assert_eq!(
        count, 1,
        "preview cannot duplicate or consume existing path work"
    );
}

#[test]
fn missing_source_observation_revalidates_a_recreated_file_before_catalog_removal() {
    let (fixture, ready) = ready_source_fixture("missing-source-recreated");
    let bytes = fs::read(&fixture.source_path).expect("fixture source bytes");
    let root = SqliteCatalog::open(fixture.storage.catalog_path.clone())
        .expect("catalog")
        .load_incremental_catalog_root(&ready.root_id)
        .expect("root query")
        .expect("root");
    fs::remove_file(&fixture.source_path).expect("remove owned fixture source");
    let issue = ScanIssue {
        path: Some(ready.absolute_path.clone()),
        code: "preview_source_missing".to_owned(),
        message: "Observed missing before path reconciliation".to_owned(),
    };
    fs::write(&fixture.source_path, &bytes).expect("recreate owned fixture source");
    let error = super::super::source_reconciliation::recover_source_mismatch(
        &fixture.storage,
        &preview_request_for(&ready, 256, false),
        &ready,
        root.root_generation,
        issue,
    );
    assert_eq!(error.code, "preview_request_superseded");
    let current = active_location(&fixture);
    assert_eq!(current.relative_path, ready.relative_path);
    assert_ne!(current.source_generation, ready.source_generation);
    assert_eq!(current.preview_status, PreviewStatus::Pending);
    assert_eq!(
        fs::read(&fixture.source_path).expect("current bytes"),
        bytes
    );
    let regenerated = materialize_preview_with_storage(
        preview_request_for(&current, 256, false),
        fixture.storage.clone(),
    )
    .expect("current source preview");
    assert_eq!(regenerated.preview_status, PreviewStatus::Ready);
}

#[test]
fn unavailable_root_is_not_evidence_of_a_missing_source() {
    let (fixture, ready) = ready_source_fixture("missing-root");
    let source_root = fixture.source_path.parent().expect("source root");
    let unavailable_root = source_root.with_extension("offline");
    fs::rename(source_root, &unavailable_root).expect("detach owned fixture root");
    let result = materialize_preview_with_storage(
        preview_request_for(&ready, 256, false),
        fixture.storage.clone(),
    );
    fs::rename(&unavailable_root, source_root).expect("restore owned fixture root");
    assert_eq!(
        result.expect_err("root is unavailable").code,
        "preview_root_unavailable"
    );
    assert_eq!(active_location(&fixture), ready);
    let connection = Connection::open(&fixture.storage.catalog_path).expect("catalog");
    let count: i64 = connection
        .query_row("SELECT COUNT(*) FROM library_change_queue", [], |row| {
            row.get(0)
        })
        .expect("queue count");
    assert_eq!(count, 0);
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
