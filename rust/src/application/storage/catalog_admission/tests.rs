use std::fs;
use std::sync::mpsc;
use std::thread;

use rusqlite::Connection;
use tempfile::{TempDir, tempdir};

use super::*;
use crate::adapters::SqliteCatalog;
use crate::application::scan_library::{
    cancel_scan, resume_scan_with_storage, run_scan_with_storage,
};
use crate::domain::{ScanEvent, ScanRequest, StorageSettingsUpdate};
use crate::ports::CatalogRepository;

use super::super::configuration_update::save_configuration;
use super::super::{DEFAULT_PREVIEW_BUDGET_BYTES, load_configured_storage};

#[test]
fn pending_catalog_relocation_rejects_production_start_and_resume_before_catalog_writes() {
    let fixture = Fixture::new();
    let before = fixture.catalog_counts();
    save_configuration(&fixture.storage, fixture.relocation()).expect("save empty catalog target");
    for resume in [false, true] {
        let mut events = Vec::new();
        let publish = |event| {
            events.push(event);
            true
        };
        let result = if resume {
            resume_scan_with_storage(fixture.request("resume"), publish, fixture.storage.clone())
        } else {
            run_scan_with_storage(fixture.request("start"), publish, fixture.storage.clone())
        };
        let error = result.expect_err("old catalog must not admit a new import");
        assert_eq!(error.code, "catalog_location_restart_required");
        assert!(error.message.contains("重启 Ame"));
        assert!(events.is_empty());
        assert_eq!(fixture.catalog_counts(), before);
    }
    assert!(
        !fixture.target_catalog().exists(),
        "rejection cannot create the target catalog"
    );
    assert_eq!(
        fs::read_dir(&fixture.source)
            .expect("source unchanged")
            .count(),
        0
    );
}

#[test]
fn activated_target_can_import_and_preview_budget_or_equivalent_path_updates_do_not_block() {
    let fixture = Fixture::new();
    let configured =
        save_configuration(&fixture.storage, fixture.relocation()).expect("pending target");
    let mut restarted = fixture.storage.clone();
    restarted.catalog_path = configured.catalog_path.into();
    let mut completed = false;
    run_scan_with_storage(
        fixture.request("new-process"),
        |event| {
            completed |= matches!(event, ScanEvent::Completed { .. });
            true
        },
        restarted.clone(),
    )
    .expect("restarted process uses configured catalog");
    assert!(completed);
    assert_eq!(fixture.catalog_counts(), (0, 0));
    let mut update = fixture.update();
    update.preview_cache_directory = Some(
        fixture
            .directory
            .path()
            .join("next-previews")
            .to_string_lossy()
            .into_owned(),
    );
    update.preview_budget_bytes *= 2;
    save_configuration(&restarted, update).expect("preview and budget update");
    run_scan_with_storage(fixture.request("preview-only"), |_| true, restarted.clone())
        .expect("preview-only restart does not block imports");
    let alias_parent = fixture.directory.path().join("alias-parent");
    fs::create_dir_all(&alias_parent).expect("alias parent");
    let mut update = fixture.update();
    update.catalog_directory = Some(
        alias_parent
            .join("..")
            .join("target")
            .to_string_lossy()
            .into_owned(),
    );
    let saved = save_configuration(&restarted, update)
        .expect("equivalent active catalog is not relocation");
    assert_eq!(Path::new(&saved.catalog_path), restarted.catalog_path);
    run_scan_with_storage(fixture.request("equivalent-path"), |_| true, restarted)
        .expect("equivalent path remains admitted");
}

#[test]
fn configuration_save_wins_race_and_waiting_scan_rechecks_the_committed_target() {
    let fixture = Fixture::new();
    let admission = admission_for(&fixture.storage).expect("gate");
    let writer = Connection::open(&fixture.storage.settings_path).expect("settings fault owner");
    writer
        .execute_batch("BEGIN IMMEDIATE")
        .expect("hold settings write boundary");
    thread::scope(|scope| {
        let (saved_tx, saved_rx) = mpsc::channel();
        let save_fixture = &fixture;
        scope.spawn(move || {
            saved_tx
                .send(save_configuration(
                    &save_fixture.storage,
                    save_fixture.relocation(),
                ))
                .expect("save outcome");
        });
        let mut saved_boundary = None;
        wait_for(|| {
            saved_boundary = admission.occupied.try_lock().ok().filter(|state| **state);
            saved_boundary.is_some()
        });
        let saved_boundary = saved_boundary.expect("database wait leaves the state lock available");
        let scan = scope.spawn(|| {
            with_scan_start(
                &fixture.storage,
                &fixture.source,
                || -> Result<(), ScanError> {
                    panic!("pending relocation cannot reach the root registration callback")
                },
            )
        });
        wait_for(|| Arc::strong_count(&admission) >= 3);
        assert!(matches!(
            saved_rx.try_recv(),
            Err(mpsc::TryRecvError::Empty)
        ));
        drop(saved_boundary);
        writer
            .execute_batch("COMMIT")
            .expect("release settings writer");
        saved_rx
            .recv()
            .expect("save result")
            .expect("configuration saved");
        let error = scan
            .join()
            .expect("scan thread")
            .expect_err("pending target rejected");
        assert_eq!(error.code, "catalog_location_restart_required");
    });
    assert_eq!(fixture.catalog_counts(), (0, 0));
}

#[test]
fn root_begin_wins_race_and_waiting_configuration_save_cannot_detach_it() {
    let fixture = Fixture::new();
    let admission = admission_for(&fixture.storage).expect("gate");
    thread::scope(|scope| {
        let error = with_scan_start(&fixture.storage, &fixture.source, || {
            let begin_boundary = admission
                .occupied
                .try_lock()
                .expect("root begin must not hold the registry lock");
            assert!(
                *begin_boundary,
                "root registration owns the transition permit"
            );
            let save = scope.spawn(|| save_configuration(&fixture.storage, fixture.relocation()));
            wait_for(|| Arc::strong_count(&admission) >= 3);
            drop(begin_boundary);
            let mut catalog = SqliteCatalog::open(fixture.storage.catalog_path.clone())?;
            catalog.begin_scan(
                &fixture.request("registered-first"),
                "root",
                fixture.source.to_str().expect("fixture path"),
            )?;
            Ok(save)
        })
        .expect("begin committed")
        .join()
        .expect("save thread")
        .expect_err("new root prevents relocation");
        assert_eq!(error.code, "catalog_relocation_requires_migration");
    });
    assert_eq!(fixture.catalog_counts(), (1, 1));
    assert_eq!(
        Path::new(
            &load_configured_storage(&fixture.storage)
                .expect("settings")
                .catalog_path
        ),
        fixture.storage.catalog_path
    );
    assert_eq!(
        save_configuration(&fixture.storage, fixture.relocation())
            .expect_err("published or unpublished root prevents relocation")
            .code,
        "catalog_relocation_requires_migration"
    );
}

#[test]
fn admission_releases_on_failure_or_unwind_and_waiting_is_bounded() {
    let fixture = Fixture::new();
    let admission = admission_for(&fixture.storage).expect("gate");
    let permit = reserve_catalog_transition(&fixture.storage).expect("permit");
    let error = admission
        .clone()
        .reserve(Instant::now())
        .err()
        .expect("bounded admission");
    assert_eq!(error.code, "storage_catalog_admission_timeout");
    drop(permit);
    assert!(
        with_scan_start(&fixture.storage, &fixture.source, || Err::<(), _>(
            ScanError::new("fixture", "failed begin")
        ))
        .is_err()
    );
    let failed = std::panic::catch_unwind(|| {
        with_scan_start(
            &fixture.storage,
            &fixture.source,
            || -> Result<(), ScanError> { panic!("interrupted begin") },
        )
    });
    assert!(failed.is_err());
    with_scan_start(&fixture.storage, &fixture.source, || Ok(()))
        .expect("unwind releases only its permit");
    let mut invalid = fixture.update();
    invalid.catalog_directory = Some("relative".to_owned());
    assert!(save_configuration(&fixture.storage, invalid).is_err());
    save_configuration(&fixture.storage, fixture.update()).expect("failed save releases admission");
}

#[test]
fn source_overlap_is_rechecked_before_the_begin_callback() {
    let fixture = Fixture::new();
    let mut update = fixture.update();
    update.preview_cache_directory = Some(fixture.source.to_string_lossy().into_owned());
    save_configuration(&fixture.storage, update).expect("no configured roots yet");
    let error = with_scan_start(
        &fixture.storage,
        &fixture.source,
        || -> Result<(), ScanError> {
            panic!("source overlap must be rejected before any catalog access")
        },
    )
    .expect_err("source overlap");
    assert_eq!(error.code, "source_root_overlaps_storage");
    assert_eq!(fixture.catalog_counts(), (0, 0));
}

#[test]
fn production_scan_releases_admission_before_started_and_after_cancellation() {
    let fixture = Fixture::new();
    let request = fixture.request("cancel-after-start");
    let admission = admission_for(&fixture.storage).expect("gate");
    let mut started = false;
    let mut cancelled = false;
    run_scan_with_storage(
        request.clone(),
        |event| {
            if matches!(event, ScanEvent::Started { .. }) {
                started = true;
                let permit = admission
                    .clone()
                    .reserve(Instant::now())
                    .expect("registration permit must end before event delivery or enumeration");
                drop(permit);
                save_configuration(&fixture.storage, fixture.update())
                    .expect("a running scan does not retain storage admission");
                assert!(cancel_scan(&request.scan_id));
            }
            cancelled |= matches!(event, ScanEvent::Cancelled { .. });
            true
        },
        fixture.storage.clone(),
    )
    .expect("controlled cancellation");
    assert!(started && cancelled);
    save_configuration(&fixture.storage, fixture.update())
        .expect("terminal cancellation leaves storage admission available");
    assert_eq!(
        fs::read_dir(&fixture.source)
            .expect("source unchanged")
            .count(),
        0
    );
}

fn wait_for(mut condition: impl FnMut() -> bool) {
    let deadline = Instant::now() + ADMISSION_TIMEOUT;
    while !condition() {
        assert!(
            Instant::now() < deadline,
            "controlled admission did not reach its boundary"
        );
        thread::yield_now();
    }
}

struct Fixture {
    directory: TempDir,
    source: std::path::PathBuf,
    storage: StoragePaths,
}

impl Fixture {
    fn new() -> Self {
        let directory = tempdir().expect("isolated storage");
        let source = directory.path().join("source");
        fs::create_dir_all(&source).expect("empty source");
        let storage = StoragePaths {
            catalog_path: directory.path().join("active").join("ame.sqlite3"),
            preview_root: directory.path().join("previews"),
            preview_budget_bytes: DEFAULT_PREVIEW_BUDGET_BYTES,
            settings_path: directory.path().join("settings.sqlite3"),
        };
        SqliteCatalog::open(storage.catalog_path.clone()).expect("empty catalog");
        load_configured_storage(&storage).expect("initialized settings");
        Self {
            directory,
            source,
            storage,
        }
    }

    fn update(&self) -> StorageSettingsUpdate {
        StorageSettingsUpdate {
            catalog_directory: None,
            preview_cache_directory: None,
            preview_budget_bytes: self.storage.preview_budget_bytes,
        }
    }

    fn relocation(&self) -> StorageSettingsUpdate {
        StorageSettingsUpdate {
            catalog_directory: Some(
                self.directory
                    .path()
                    .join("target")
                    .to_string_lossy()
                    .into_owned(),
            ),
            ..self.update()
        }
    }

    fn target_catalog(&self) -> std::path::PathBuf {
        self.directory.path().join("target").join("ame.sqlite3")
    }

    fn request(&self, suffix: &str) -> ScanRequest {
        ScanRequest {
            scan_id: format!("{}-{suffix}", self.directory.path().display()),
            root_path: self.source.to_string_lossy().into_owned(),
            max_items: None,
            max_entries: None,
            preview_edge: 128,
        }
    }

    fn catalog_counts(&self) -> (i64, i64) {
        Connection::open(&self.storage.catalog_path)
            .expect("catalog evidence")
            .query_row(
                "SELECT (SELECT COUNT(*) FROM library_roots), (SELECT COUNT(*) FROM scan_runs)",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("row counts")
    }
}
