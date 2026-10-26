use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, mpsc};
use std::time::{Duration, Instant};

use rusqlite::backup::{Backup, StepResult};
use rusqlite::{Connection, OpenFlags};
use serde_json::json;

use crate::domain::organization_batch::OrganizationBatchAction;
use crate::domain::organization_queue::OrganizationQueue;
use crate::domain::organization_review::{
    OrganizationContinuationStatus, OrganizationReviewSession,
};
use crate::domain::{GalleryCollection, GalleryDuplicateMode, GalleryQuery, ScanError};
use crate::ports::GalleryQueryRepository;
use crate::ports::organization_batch::{OrganizationBatchControl, OrganizationBatchRepository};
use crate::ports::organization_review::OrganizationReviewRepository;

use super::transitions::create_album;
use super::*;

mod anchor_stages;
mod cancellation;
mod read_stages;

const PAGE_SIZE: u32 = 128;

struct Profile {
    output: File,
}

impl Profile {
    fn record(&mut self, value: serde_json::Value) {
        serde_json::to_writer(&mut self.output, &value).unwrap();
        writeln!(self.output).unwrap();
        self.output.flush().unwrap();
        eprintln!("{value}");
    }

    fn measure<T>(&mut self, operation: &str, action: impl FnOnce() -> Result<T, ScanError>) -> T {
        let started = Instant::now();
        let result = action();
        self.record(json!({
            "operation": operation,
            "elapsed_ms": started.elapsed().as_secs_f64() * 1000.0,
            "error_code": result.as_ref().err().map(|error| &error.code),
        }));
        result.unwrap()
    }
}

#[test]
#[ignore = "manual retained-catalog profile; requires an isolated snapshot under build/test/derived"]
fn profile_retained_organization_queries() {
    let (directory, catalog_path, mut profile) = prepare_copy("organization-profile.jsonl");
    let mut catalog = profile.measure("upgrade_and_open_copy", || {
        SqliteCatalog::open(catalog_path.clone())
    });
    assert_eq!(
        scalar(&catalog, "SELECT COUNT(*) FROM organization_records"),
        0,
        "profile requires a pre-organization snapshot"
    );
    let pending = GalleryQuery {
        organization_queue: Some(OrganizationQueue::Pending),
        ..GalleryQuery::default()
    };
    measure_query(&mut profile, &mut catalog, "pending_initial", &pending);
    let first = catalog
        .load_query_snapshot(PAGE_SIZE, &pending, "profile-pending", None)
        .unwrap();
    let anchor = first.snapshot.assets.last().unwrap();
    let selected_anchor = selected(&catalog, &anchor.location_id);
    let review = catalog.begin_organization_review(&pending).unwrap();
    profile.measure("save_continuation", || {
        catalog.checkpoint_organization_review(
            &OrganizationReviewSession {
                position_sequence: 1,
                ..review.clone()
            },
            &selected_anchor,
        )
    });
    catalog.end_organization_review(&review.session_id).unwrap();
    for sample in 0..3 {
        let continuation = profile.measure(&format!("continuation_warm_{sample}"), || {
            catalog.load_organization_continuation(None)
        });
        assert_eq!(continuation.status, OrganizationContinuationStatus::Ready);
        assert_eq!(continuation.target.as_ref(), Some(&selected_anchor));
    }

    let album_id = create_album(&mut catalog, "Retained profile");
    let library = GalleryQuery {
        duplicate_mode: GalleryDuplicateMode::Merged,
        ..GalleryQuery::default()
    };
    apply_batch(
        &mut profile,
        &catalog_path,
        &catalog,
        library,
        "assign_library",
        OrganizationBatchAction::Organize {
            action: OrganizationAction::AssignAlbum {
                album_id: album_id.clone(),
            },
        },
    );
    let album = GalleryQuery {
        collection: GalleryCollection::Album { album_id },
        ..GalleryQuery::default()
    };
    apply_batch(
        &mut profile,
        &catalog_path,
        &catalog,
        album.clone(),
        "favorite_album",
        OrganizationBatchAction::Favorite { favorite: true },
    );
    drop(catalog);
    let mut catalog = profile.measure("reopen_populated_copy", || {
        SqliteCatalog::open(catalog_path.clone())
    });
    for (name, query) in [
        ("album", album.clone()),
        (
            "favorites",
            GalleryQuery {
                collection: GalleryCollection::Favorites,
                duplicate_mode: GalleryDuplicateMode::Merged,
                ..GalleryQuery::default()
            },
        ),
        ("pending_after_assignment", pending),
    ] {
        measure_query(&mut profile, &mut catalog, name, &query);
    }
    let prepared = catalog
        .prepare_organization_selection(std::slice::from_ref(&selected_anchor))
        .unwrap();
    profile.measure("single_unfavorite_commit", || {
        catalog.set_organization_favorite("profile-single-unfavorite", &prepared, false)
    });
    let favorite_result = profile.measure("favorites_after_single_commit", || {
        catalog.load_query_snapshot(
            PAGE_SIZE,
            &GalleryQuery {
                collection: GalleryCollection::Favorites,
                duplicate_mode: GalleryDuplicateMode::Merged,
                ..GalleryQuery::default()
            },
            "profile-favorite-changed",
            None,
        )
    });
    let changed = catalog
        .load_query_snapshot(PAGE_SIZE, &album, "profile-changed", None)
        .unwrap();
    assert_eq!(
        favorite_result.timeline.total_items + 1,
        changed.timeline.total_items
    );
    profile.record(json!({
        "complete": true,
        "album_groups": changed.timeline.total_items,
        "album_physical_files": changed.timeline.total_source_items,
        "source_media_reads": 0,
        "scope": "catalog_ports_only_no_client_or_preview",
    }));
    drop(catalog);
    directory.close().unwrap();
}

fn measure_query(
    profile: &mut Profile,
    catalog: &mut SqliteCatalog,
    name: &str,
    query: &GalleryQuery,
) {
    for sample in 0..4 {
        let snapshot = profile.measure(&format!("{name}_snapshot_{sample}"), || {
            catalog.load_query_snapshot(PAGE_SIZE, query, "profile", None)
        });
        assert!(snapshot.snapshot.assets.len() <= PAGE_SIZE as usize);
        profile.record(json!({
            "query": name,
            "sample": sample,
            "groups": snapshot.timeline.total_items,
            "physical_files": snapshot.timeline.total_source_items,
            "returned_assets": snapshot.snapshot.assets.len(),
            "timeline_buckets": snapshot.timeline.buckets.len(),
        }));
        if sample == 0 {
            let layout = profile.measure(&format!("{name}_layout"), || {
                catalog.load_gallery_layout_manifest_chunk(PAGE_SIZE, query, "profile", None)
            });
            assert!(layout.location_ids.len() <= PAGE_SIZE as usize);
            assert_eq!(layout.total_items, snapshot.timeline.total_items);
            if let Some(cursor) = snapshot.snapshot.next_cursor.as_ref() {
                let next = profile.measure(&format!("{name}_next_page"), || {
                    catalog.load_snapshot(PAGE_SIZE, query, "profile", Some(cursor), None, None)
                });
                assert!(next.assets.len() <= PAGE_SIZE as usize);
            }
        }
    }
}

fn apply_batch(
    profile: &mut Profile,
    path: &Path,
    catalog: &SqliteCatalog,
    query: GalleryQuery,
    name: &str,
    action: OrganizationBatchAction,
) {
    let control = Arc::new(OrganizationBatchControl::new());
    let mut batch = SqliteOrganizationBatch::new(
        SqliteCatalog::open(path.to_path_buf()).unwrap(),
        query,
        scalar(catalog, "SELECT revision FROM catalog_state"),
        true,
        Arc::clone(&control),
    )
    .unwrap();
    let (finished, waiting) = mpsc::channel();
    let cancellation = Arc::clone(&control);
    let watchdog = std::thread::spawn(move || {
        if waiting.recv_timeout(Duration::from_secs(180)).is_err() {
            cancellation
                .cancelled
                .store(true, std::sync::atomic::Ordering::Release);
        }
    });
    let summary = profile.measure(&format!("{name}_prepare"), || batch.prepare());
    profile.record(json!({ "batch": name, "groups": summary.group_count }));
    let receipt = profile.measure(&format!("{name}_commit"), || {
        batch.apply(&format!("profile-{name}"), &action)
    });
    assert_eq!(receipt.changed_groups, summary.group_count);
    finished.send(()).unwrap();
    watchdog.join().unwrap();
}

fn prepare_copy(report_name: &str) -> (tempfile::TempDir, PathBuf, Profile) {
    let repository = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .canonicalize()
        .unwrap();
    let derived = repository
        .join("build/test/derived")
        .canonicalize()
        .unwrap();
    assert!(derived.starts_with(&repository));
    let input = PathBuf::from(
        std::env::var_os("AME_R4_RETAINED_SNAPSHOT").expect("isolated snapshot is required"),
    )
    .canonicalize()
    .unwrap();
    assert!(input.starts_with(&derived));
    reject_reparse_ancestors(&input, &repository);
    let report = input.parent().unwrap().join(report_name);
    let output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(report)
        .expect("profile report must not already exist");
    let mut profile = Profile { output };
    let directory = tempfile::Builder::new()
        .prefix("r4-profile-")
        .tempdir_in(&derived)
        .unwrap();
    let destination = directory.path().join("catalog.sqlite3");
    let connection = Connection::open_with_flags(&input, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    connection.execute_batch("PRAGMA query_only = ON").unwrap();
    let started = Instant::now();
    let mut copy = Connection::open(&destination).unwrap();
    {
        let backup = Backup::new(&connection, &mut copy).unwrap();
        loop {
            assert!(started.elapsed() < Duration::from_secs(300));
            match backup.step(256).unwrap() {
                StepResult::Done => break,
                StepResult::More => std::thread::yield_now(),
                _ => panic!("isolated snapshot unexpectedly busy"),
            }
        }
    }
    drop(copy);
    profile.record(json!({
        "profile": "R4 catalog operations on isolated retained snapshot",
        "build": if cfg!(debug_assertions) { "debug" } else { "release" },
        "cache_condition": "fresh connections; OS cache not flushed",
        "copy_ms": started.elapsed().as_secs_f64() * 1000.0,
        "page_limit": PAGE_SIZE,
        "batch_operational_timeout_seconds": 180,
        "source_media_reads": 0,
    }));
    (directory, destination, profile)
}

fn reject_reparse_ancestors(path: &Path, repository: &Path) {
    for ancestor in path
        .ancestors()
        .take_while(|path| path.starts_with(repository))
    {
        let metadata = std::fs::symlink_metadata(ancestor).unwrap();
        assert!(!metadata.file_type().is_symlink());
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            assert_eq!(metadata.file_attributes() & 0x400, 0);
        }
    }
}
