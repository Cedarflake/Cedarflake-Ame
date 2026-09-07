use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use rusqlite::{Connection, OpenFlags, params};
use tempfile::{TempDir, tempdir};

use crate::adapters::{SqliteCatalog, set_before_scan_projection_replacement_hook};
use crate::application::StoragePaths;
use crate::domain::{GalleryQuery, ScanEvent, ScanRequest};
use crate::media_fixtures::{MediaFixtureFormat, encode_rgb_quadrants};
use crate::ports::CatalogRepository;

use super::super::{cancel_scan, run_scan_with_storage};

mod first_import_tests;
mod retry_callback_tests;

#[test]
fn accepted_cancel_before_projection_commit_preserves_the_published_baseline() {
    let fixture = Fixture::new();
    let request = fixture.replacement();
    let accepted = Arc::new(AtomicBool::new(false));
    let observed = Arc::clone(&accepted);
    let catalog_path = fixture.paths.catalog_path.clone();
    let initial_scan = fixture.initial_scan.clone();
    let replacement_scan = request.scan_id.clone();
    let _hook = set_before_scan_projection_replacement_hook(&request.scan_id, move |_| {
        // A separate reader proves the publication transaction has not committed yet.
        assert_eq!(
            publication_state(&catalog_path, &replacement_scan),
            (initial_scan, "running".to_owned(), 1),
        );
        let cancelled = cancel_scan(&replacement_scan);
        observed.store(cancelled, Ordering::Release);
        assert!(
            cancelled,
            "the executing scan accepted the real control request"
        );
    });
    let mut events = Vec::new();
    run_scan_with_storage(
        request.clone(),
        |event| {
            events.push(event);
            true
        },
        fixture.paths.clone(),
    )
    .expect("cancellation settles through the normal scan lifecycle");

    assert!(
        accepted.load(Ordering::Acquire),
        "the commit-window hook ran"
    );
    let state = publication_state(&fixture.paths.catalog_path, &request.scan_id);
    eprintln!(
        "publication_cancel_before_commit: active_scan={} replacement_status={} active_locations={}",
        state.0, state.1, state.2
    );
    assert!(
        matches!(events.last(), Some(ScanEvent::Cancelled { .. })),
        "accepted pre-commit cancellation must be terminal Cancelled, got {:?}",
        events.last(),
    );
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, ScanEvent::Completed { .. }))
    );
    assert_eq!(
        state,
        (fixture.initial_scan.clone(), "cancelled".to_owned(), 1)
    );
    let reader = read_catalog(&fixture.paths.catalog_path);
    let (old_locations, discarded_staging): (i64, i64) = reader
        .query_row(
            "SELECT (SELECT COUNT(*) FROM asset_locations WHERE scan_id = ?1),
                (SELECT COUNT(*) FROM asset_locations WHERE scan_id = ?2)",
            params![fixture.initial_scan, request.scan_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("retained baseline and discarded replacement");
    assert_eq!((old_locations, discarded_staging), (1, 0));
    drop(reader);
    fixture.assert_reopen(1, &fixture.initial_scan);
    fixture.assert_source_bytes();
}

#[test]
fn accepted_cancel_after_publication_commit_does_not_undo_completed_state() {
    let fixture = Fixture::new();
    let request = fixture.replacement();
    let mut accepted = false;
    let mut events = Vec::new();
    run_scan_with_storage(
        request.clone(),
        |event| {
            if matches!(event, ScanEvent::Completed { .. }) {
                assert_eq!(
                    publication_state(&fixture.paths.catalog_path, &request.scan_id),
                    (request.scan_id.clone(), "completed".to_owned(), 2),
                );
                accepted = cancel_scan(&request.scan_id);
            }
            events.push(event);
            true
        },
        fixture.paths.clone(),
    )
    .expect("committed publication remains authoritative");
    assert!(
        accepted,
        "the late request still reached the draining execution"
    );
    assert!(matches!(
        events.last(),
        Some(ScanEvent::Completed { asset_count: 2, .. })
    ));
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, ScanEvent::Cancelled { .. }))
    );
    assert_eq!(
        publication_state(&fixture.paths.catalog_path, &request.scan_id),
        (request.scan_id.clone(), "completed".to_owned(), 2),
    );
    fixture.assert_reopen(2, &request.scan_id);
    fixture.assert_source_bytes();
}

fn read_catalog(path: &Path) -> Connection {
    Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .expect("independent read-only catalog connection")
}

fn publication_state(path: &Path, scan_id: &str) -> (String, String, i64) {
    read_catalog(path)
        .query_row(
            "SELECT roots.active_scan_id, (SELECT status FROM scan_runs WHERE id = ?1),
                (SELECT COUNT(*) FROM asset_locations WHERE scan_id = roots.active_scan_id)
         FROM library_roots AS roots",
            [scan_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("committed publication state")
}

struct Fixture {
    source: TempDir,
    _derived: TempDir,
    paths: StoragePaths,
    initial_scan: String,
    baseline_bytes: Vec<u8>,
    incoming_bytes: Vec<u8>,
}

impl Fixture {
    fn new() -> Self {
        let fixture = Self::unpublished();
        let mut completed = false;
        run_scan_with_storage(
            fixture.request(fixture.initial_scan.clone()),
            |event| {
                completed |= matches!(event, ScanEvent::Completed { asset_count: 1, .. });
                true
            },
            fixture.paths.clone(),
        )
        .expect("real published baseline");
        assert!(completed);
        fixture.assert_reopen(1, &fixture.initial_scan);
        fixture.add_incoming();
        fixture
    }

    fn unpublished() -> Self {
        let source = tempdir().expect("disposable source");
        let derived = tempdir().expect("isolated derived storage");
        let baseline_bytes =
            encode_rgb_quadrants(MediaFixtureFormat::Png, 16, 12).expect("baseline pixels");
        let incoming_bytes =
            encode_rgb_quadrants(MediaFixtureFormat::Png, 23, 17).expect("incoming pixels");
        fs::write(source.path().join("baseline.png"), &baseline_bytes).expect("baseline source");
        let paths = StoragePaths {
            catalog_path: derived.path().join("catalog.sqlite3"),
            preview_root: derived.path().join("previews"),
            preview_budget_bytes: 64 * 1024 * 1024,
            settings_path: derived.path().join("settings.sqlite3"),
        };
        let initial_scan = format!(
            "publication-control-baseline-{}",
            source
                .path()
                .file_name()
                .expect("unique fixture name")
                .to_string_lossy(),
        );
        Self {
            source,
            _derived: derived,
            paths,
            initial_scan,
            baseline_bytes,
            incoming_bytes,
        }
    }

    fn add_incoming(&self) {
        fs::write(
            self.source.path().join("incoming.png"),
            &self.incoming_bytes,
        )
        .expect("external writer adds owned fixture");
    }

    fn request(&self, scan_id: String) -> ScanRequest {
        ScanRequest {
            scan_id,
            root_path: self.source.path().to_string_lossy().into_owned(),
            max_items: None,
            max_entries: None,
            preview_edge: 128,
        }
    }

    fn replacement(&self) -> ScanRequest {
        self.request(format!("{}-replacement", self.initial_scan))
    }

    fn assert_reopen(&self, count: usize, scan_id: &str) {
        let snapshot = SqliteCatalog::open(self.paths.catalog_path.clone())
            .expect("full schema proof after terminal settlement")
            .load_snapshot(
                10,
                &GalleryQuery::default(),
                "publication-control",
                None,
                None,
                None,
            )
            .expect("published catalog is readable");
        assert_eq!(snapshot.assets.len(), count);
        assert_eq!(snapshot.roots.len(), 1);
        assert_eq!(snapshot.roots[0].active_scan_id.as_deref(), Some(scan_id));
    }

    fn assert_source_bytes(&self) {
        assert_eq!(
            fs::read(self.source.path().join("baseline.png")).expect("baseline bytes"),
            self.baseline_bytes
        );
        assert_eq!(
            fs::read(self.source.path().join("incoming.png")).expect("incoming bytes"),
            self.incoming_bytes
        );
        assert_eq!(
            fs::read_dir(self.source.path())
                .expect("source entries")
                .count(),
            2
        );
    }
}
