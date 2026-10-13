use std::fs;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use image::{Rgb, RgbImage};
use rusqlite::{Connection, TransactionBehavior};
use tempfile::tempdir;

use crate::adapters::{LocalPreviewStore, SqliteCatalog};
use crate::application::{PREVIEW_LIFECYCLE_TEST_LOCK, StoragePaths};
use crate::domain::{
    AssetLocationView, GalleryQuery, PreviewReclamationCandidate, PreviewRequest, PreviewStatus,
    ScanEvent, ScanRequest,
};

use super::*;

struct Fixture {
    _directory: tempfile::TempDir,
    storage: StoragePaths,
    catalog: SqliteCatalog,
    store: LocalPreviewStore,
    ready: AssetLocationView,
    candidate: PreviewReclamationCandidate,
    source_bytes: Vec<u8>,
}

impl Fixture {
    fn new() -> Self {
        let directory = tempdir().expect("preview-health fixture");
        let source = directory.path().join("source");
        fs::create_dir(&source).expect("source directory");
        let path = source.join("source.png");
        RgbImage::from_pixel(32, 24, Rgb([24, 96, 192]))
            .save(&path)
            .expect("fixture PNG");
        let source_bytes = fs::read(&path).expect("original bytes");
        let storage = StoragePaths {
            catalog_path: directory.path().join("catalog.sqlite3"),
            preview_root: directory.path().join("previews"),
            preview_budget_bytes: 64 * 1024 * 1024,
            settings_path: directory.path().join("settings.sqlite3"),
        };
        let mut completed = false;
        crate::application::scan_library::run_scan_with_storage(
            ScanRequest {
                scan_id: "preview-health-scan".to_owned(),
                root_path: source.to_string_lossy().into_owned(),
                max_items: None,
                max_entries: None,
                preview_edge: 256,
            },
            |event| {
                completed |= matches!(event, ScanEvent::Completed { .. });
                true
            },
            storage.clone(),
        )
        .expect("production fixture scan");
        assert!(completed);
        let mut catalog = SqliteCatalog::open(storage.catalog_path.clone()).expect("catalog");
        let location = catalog
            .load_snapshot(1, &GalleryQuery::default(), "health", None, None, None)
            .expect("fixture snapshot")
            .assets
            .pop()
            .expect("fixture location");
        let store =
            LocalPreviewStore::new(storage.preview_root.clone(), storage.preview_budget_bytes)
                .expect("preview store");
        let ready = crate::application::preview::materialize_preview_with_store(
            request(&location),
            storage.clone(),
            &store,
        )
        .expect("production materialization");
        assert!(matches!(ready.preview_status, PreviewStatus::Ready));
        let candidate = catalog
            .load_preview_recovery_artifacts(
                &format!(
                    "{}{}",
                    storage.preview_root.to_string_lossy(),
                    std::path::MAIN_SEPARATOR
                ),
                None,
                1,
            )
            .expect("recovery hints")
            .pop()
            .expect("ready artifact");
        Self {
            _directory: directory,
            storage,
            catalog,
            store,
            ready,
            candidate,
            source_bytes,
        }
    }

    fn regenerate(&self) -> AssetLocationView {
        crate::application::preview::materialize_preview_with_store(
            request(&self.ready),
            self.storage.clone(),
            &self.store,
        )
        .expect("foreground ForceRegenerate")
    }

    fn assert_ready(&self) {
        let location = self
            .catalog
            .load_active_location(&self.ready.location_id)
            .expect("active location")
            .expect("location");
        assert!(matches!(location.preview_status, PreviewStatus::Ready));
        assert_eq!(location.preview_path, self.ready.preview_path);
        assert_eq!((location.width, location.height), (32, 24));
        let connection =
            Connection::open(&self.storage.catalog_path).expect("verification connection");
        let (bytes, owners): (i64, i64) = connection.query_row(
            "SELECT byte_size, (SELECT COUNT(*) FROM preview_artifact_locations WHERE artifact_key = ?1)
             FROM preview_artifacts WHERE artifact_key = ?1", [&self.candidate.artifact_key],
            |row| Ok((row.get(0)?, row.get(1)?)),
        ).expect("current artifact");
        assert_eq!(
            u64::try_from(bytes).expect("nonnegative artifact bytes"),
            fs::metadata(&location.preview_path)
                .expect("current bytes")
                .len()
        );
        assert_eq!(owners, 1);
        assert_eq!(
            fs::read(&location.absolute_path).expect("source after"),
            self.source_bytes
        );
    }
}

fn request(location: &AssetLocationView) -> PreviewRequest {
    PreviewRequest {
        location_id: location.location_id.clone(),
        expected_root_id: location.root_id.clone(),
        expected_scan_id: location.scan_id.clone(),
        expected_source_revision: location.source_revision.clone(),
        expected_source_generation: location.source_generation,
        preview_edge: 256,
        retry_failed: true,
        protected_location_ids: Vec::new(),
    }
}

#[test]
fn stale_missing_hint_cannot_erase_foreground_regeneration_or_snapshot_readiness() {
    let _lock = PREVIEW_LIFECYCLE_TEST_LOCK.lock().expect("lifecycle lock");
    let mut fixture = Fixture::new();
    let mut snapshot = fixture
        .catalog
        .load_snapshot(
            1,
            &GalleryQuery::default(),
            "stale-health",
            None,
            None,
            None,
        )
        .expect("old snapshot");
    fs::remove_file(&fixture.ready.preview_path).expect("remove owned derived artifact");
    assert!(
        !is_active_location_file(&fixture.ready.preview_path, &fixture.storage.preview_root)
            .expect("missing hint")
    );
    let new_ready = fixture.regenerate();
    assert!(matches!(new_ready.preview_status, PreviewStatus::Ready));
    assert_eq!(
        reconcile(
            &mut fixture.catalog,
            &fixture.storage.preview_root,
            PreviewHealthTarget::Artifact(&fixture.candidate)
        )
        .expect("fresh index observation"),
        PreviewHealthOutcome::Unchanged
    );
    reconcile_snapshot_previews(
        &mut fixture.catalog,
        &fixture.storage.preview_root,
        &mut snapshot,
    )
    .expect("fresh snapshot observation");
    assert!(matches!(
        snapshot.assets[0].preview_status,
        PreviewStatus::Ready
    ));
    fixture.assert_ready();
}

#[test]
fn size_recovery_uses_current_encoded_bytes_instead_of_an_old_candidate_observation() {
    let _lock = PREVIEW_LIFECYCLE_TEST_LOCK.lock().expect("lifecycle lock");
    let mut fixture = Fixture::new();
    fs::write(&fixture.ready.preview_path, b"old").expect("stale owned artifact");
    let stale_size = fs::metadata(&fixture.ready.preview_path)
        .expect("old size hint")
        .len();
    fixture.regenerate();
    assert_ne!(
        fs::metadata(&fixture.ready.preview_path)
            .expect("new bytes")
            .len(),
        stale_size
    );
    Connection::open(&fixture.storage.catalog_path)
        .expect("accounting fixture")
        .execute(
            "UPDATE preview_artifacts SET byte_size = ?1 WHERE artifact_key = ?2",
            rusqlite::params![
                i64::try_from(stale_size).expect("bounded fixture size"),
                fixture.candidate.artifact_key
            ],
        )
        .expect("old accounting");
    assert_eq!(
        reconcile(
            &mut fixture.catalog,
            &fixture.storage.preview_root,
            PreviewHealthTarget::Artifact(&fixture.candidate)
        )
        .expect("reconcile current size"),
        PreviewHealthOutcome::CorrectedSize
    );
    fixture.assert_ready();
}

#[test]
fn snapshot_missing_hint_is_revalidated_after_real_foreground_publication() {
    let _lock = PREVIEW_LIFECYCLE_TEST_LOCK.lock().expect("lifecycle lock");
    let mut fixture = Fixture::new();
    let mut snapshot = fixture
        .catalog
        .load_snapshot(
            1,
            &GalleryQuery::default(),
            "missing-window",
            None,
            None,
            None,
        )
        .expect("ready window");
    fs::remove_file(&fixture.ready.preview_path).expect("missing owned artifact");
    let storage = fixture.storage.clone();
    let retry = request(&fixture.ready);
    AFTER_MISSING_HINT.with(|hook| {
        *hook.borrow_mut() = Some(Box::new(move || {
            let store =
                LocalPreviewStore::new(storage.preview_root.clone(), storage.preview_budget_bytes)
                    .expect("new foreground store");
            let published =
                crate::application::preview::materialize_preview_with_store(retry, storage, &store)
                    .expect("publish between hint and reconciliation");
            assert!(matches!(published.preview_status, PreviewStatus::Ready));
        }))
    });
    reconcile_snapshot_previews(
        &mut fixture.catalog,
        &fixture.storage.preview_root,
        &mut snapshot,
    )
    .expect("production snapshot health owner");
    assert!(matches!(
        snapshot.assets[0].preview_status,
        PreviewStatus::Ready
    ));
    fixture.assert_ready();
}

#[test]
fn final_missing_observation_excludes_publication_until_index_commit() {
    let _lock = PREVIEW_LIFECYCLE_TEST_LOCK.lock().expect("lifecycle lock");
    let mut fixture = Fixture::new();
    fs::remove_file(&fixture.ready.preview_path).expect("missing owned artifact");
    let (start_tx, start_rx) = mpsc::channel();
    let storage = fixture.storage.clone();
    let retry = request(&fixture.ready);
    let publisher = thread::spawn(move || {
        start_rx.recv().expect("observed missing file");
        let store =
            LocalPreviewStore::new(storage.preview_root.clone(), storage.preview_budget_bytes)
                .expect("foreground store");
        crate::application::preview::materialize_preview_with_store(retry, storage, &store)
    });
    AFTER_OBSERVATION.with(|hook| {
        *hook.borrow_mut() = Some(Box::new(move || {
            start_tx.send(()).expect("start foreground retry");
            let deadline = Instant::now() + Duration::from_secs(5);
            while crate::application::preview_cleanup::preview_generation_waiter_count() == 0 {
                assert!(
                    Instant::now() < deadline,
                    "foreground retry did not reach preview exclusion"
                );
                thread::yield_now();
            }
        }))
    });
    assert_eq!(
        reconcile(
            &mut fixture.catalog,
            &fixture.storage.preview_root,
            PreviewHealthTarget::Artifact(&fixture.candidate)
        )
        .expect("missing commit"),
        PreviewHealthOutcome::Invalidated
    );
    assert!(matches!(
        publisher
            .join()
            .expect("publisher thread")
            .expect("foreground publication")
            .preview_status,
        PreviewStatus::Ready
    ));
    fixture.assert_ready();
}

#[test]
fn busy_catalog_defers_health_work_without_resetting_the_snapshot_or_owning_preview_access() {
    let _lock = PREVIEW_LIFECYCLE_TEST_LOCK.lock().expect("lifecycle lock");
    let mut fixture = Fixture::new();
    fs::remove_file(&fixture.ready.preview_path).expect("missing owned artifact");
    let mut snapshot = fixture
        .catalog
        .load_snapshot(1, &GalleryQuery::default(), "busy-window", None, None, None)
        .expect("ready snapshot");
    let mut blocker = Connection::open(&fixture.storage.catalog_path).expect("blocking connection");
    let transaction = blocker
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .expect("blocked writer");
    let started = Instant::now();
    assert_eq!(
        reconcile(
            &mut fixture.catalog,
            &fixture.storage.preview_root,
            PreviewHealthTarget::Artifact(&fixture.candidate)
        )
        .expect("deferred attempt"),
        PreviewHealthOutcome::Deferred
    );
    assert!(
        started.elapsed() < Duration::from_millis(250),
        "health reconciliation waited for the database writer"
    );
    reconcile_snapshot_previews(
        &mut fixture.catalog,
        &fixture.storage.preview_root,
        &mut snapshot,
    )
    .expect("deferred visible repair");
    assert!(matches!(
        snapshot.assets[0].preview_status,
        PreviewStatus::Ready
    ));
    assert_eq!(snapshot.assets[0].preview_path, fixture.ready.preview_path);
    let access = crate::application::acquire_preview_generation().expect("preview access released");
    drop(access);
    transaction.commit().expect("release writer");
    assert_eq!(
        reconcile(
            &mut fixture.catalog,
            &fixture.storage.preview_root,
            PreviewHealthTarget::Artifact(&fixture.candidate)
        )
        .expect("next idle attempt"),
        PreviewHealthOutcome::Invalidated
    );
}

#[test]
fn conditional_health_writes_reject_old_source_and_path_owners_and_preserve_healthy_locations() {
    let _lock = PREVIEW_LIFECYCLE_TEST_LOCK.lock().expect("lifecycle lock");
    let mut fixture = Fixture::new();
    let mut stale = fixture.ready.clone();
    stale.source_generation += 1;
    assert_eq!(
        fixture
            .catalog
            .try_reconcile_preview_health(
                PreviewHealthTarget::Location(&stale),
                PreviewHealthObservation::Missing
            )
            .expect("stale source rejected"),
        PreviewHealthOutcome::Unchanged
    );
    stale = fixture.ready.clone();
    stale.preview_path.push_str(".old");
    assert_eq!(
        fixture
            .catalog
            .try_reconcile_preview_health(
                PreviewHealthTarget::Location(&stale),
                PreviewHealthObservation::Missing
            )
            .expect("changed path rejected"),
        PreviewHealthOutcome::Unchanged
    );
    let mut stale_artifact = fixture.candidate.clone();
    stale_artifact.path.push_str(".old");
    assert_eq!(
        fixture
            .catalog
            .try_reconcile_preview_health(
                PreviewHealthTarget::Artifact(&stale_artifact),
                PreviewHealthObservation::Missing
            )
            .expect("changed index path rejected"),
        PreviewHealthOutcome::Unchanged
    );
    assert_eq!(
        fixture
            .catalog
            .try_reconcile_preview_health(
                PreviewHealthTarget::Location(&fixture.ready),
                PreviewHealthObservation::File { byte_size: 1 }
            )
            .expect("healthy location observation"),
        PreviewHealthOutcome::Unchanged
    );
    fixture.assert_ready();
}

#[test]
fn failed_health_index_commit_rolls_back_location_and_reference_changes() {
    let _lock = PREVIEW_LIFECYCLE_TEST_LOCK.lock().expect("lifecycle lock");
    let mut fixture = Fixture::new();
    fs::remove_file(&fixture.ready.preview_path).expect("missing owned artifact");
    let connection = Connection::open(&fixture.storage.catalog_path).expect("fault connection");
    connection.execute_batch("CREATE TRIGGER reject_health_delete BEFORE DELETE ON preview_artifacts BEGIN SELECT RAISE(ABORT, 'failed index deletion'); END;").expect("atomic failure seam");
    assert!(
        reconcile(
            &mut fixture.catalog,
            &fixture.storage.preview_root,
            PreviewHealthTarget::Artifact(&fixture.candidate)
        )
        .is_err()
    );
    let location = fixture
        .catalog
        .load_active_location(&fixture.ready.location_id)
        .expect("location after rollback")
        .expect("location retained");
    assert!(matches!(location.preview_status, PreviewStatus::Ready));
    assert_eq!(location.preview_path, fixture.ready.preview_path);
    let owners: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM preview_artifact_locations WHERE artifact_key = ?1",
            [&fixture.candidate.artifact_key],
            |row| row.get(0),
        )
        .expect("ownership after rollback");
    assert_eq!(owners, 1);
    connection
        .execute_batch("DROP TRIGGER reject_health_delete")
        .expect("clear owned fault");
    assert_eq!(
        reconcile(
            &mut fixture.catalog,
            &fixture.storage.preview_root,
            PreviewHealthTarget::Artifact(&fixture.candidate)
        )
        .expect("retry after rollback"),
        PreviewHealthOutcome::Invalidated
    );
}
