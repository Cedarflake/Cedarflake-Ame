use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use rusqlite::hooks::{AuthAction, AuthContext, Authorization};
use tempfile::tempdir;

use crate::domain::{
    AssetLocationView, CaptureTimeEvidence, CaptureTimeSource, PreviewStatus, ScanRequest,
};
use crate::ports::CatalogRepository;

use super::super::SqliteCatalog;

fn catalog(path: std::path::PathBuf) -> SqliteCatalog {
    let mut catalog = SqliteCatalog::open(path).expect("isolated catalog");
    let request = ScanRequest {
        scan_id: "scan".to_owned(),
        root_path: "C:\\Pictures".to_owned(),
        max_items: Some(512),
        max_entries: Some(512),
        preview_edge: 256,
    };
    catalog
        .begin_scan(&request, "root", &request.root_path)
        .expect("staging admission");
    catalog
}

fn location(index: usize) -> AssetLocationView {
    AssetLocationView {
        asset_id: format!("asset-{index}"),
        location_id: format!("location-{index}"),
        root_id: "root".to_owned(),
        scan_id: "scan".to_owned(),
        absolute_path: format!("C:\\Pictures\\image-{index}.png"),
        display_path: format!("C:\\Pictures\\image-{index}.png"),
        relative_path: format!("image-{index}.png"),
        preview_path: String::new(),
        file_size: 100,
        created_unix_ms: None,
        modified_unix_ms: 1000,
        file_identity: None,
        source_revision: None,
        source_generation: 0,
        width: 10,
        height: 20,
        preview_status: PreviewStatus::Pending,
        preview_issue_code: None,
        preview_issue_message: None,
        metadata_engine_id: "fixture".to_owned(),
        metadata_engine_version: "1".to_owned(),
        capture_time: None,
    }
}

#[test]
fn staging_compiles_one_location_insert_across_multiple_committed_batches() {
    let directory = tempdir().expect("isolated storage");
    let mut catalog = catalog(directory.path().join("catalog.sqlite3"));
    let preparations = Arc::new(AtomicUsize::new(0));
    let observed = Arc::clone(&preparations);
    catalog
        .connection
        .authorizer(Some(move |context: AuthContext<'_>| {
            if matches!(
                context.action,
                AuthAction::Insert {
                    table_name: "asset_locations"
                }
            ) {
                observed.fetch_add(1, Ordering::Relaxed);
            }
            Authorization::Allow
        }))
        .expect("observe actual SQL preparations");
    let commits = Arc::new(AtomicUsize::new(0));
    let observed = Arc::clone(&commits);
    catalog
        .connection
        .commit_hook(Some(move || {
            observed.fetch_add(1, Ordering::Relaxed);
            false
        }))
        .expect("observe batch commits");

    for index in 0..256 {
        catalog
            .stage_location("scan", "root", &location(index))
            .expect("stage location");
    }
    assert!(catalog.pending_locations.is_empty());
    assert_eq!(commits.load(Ordering::Relaxed), 2);
    let counts: (i64, i64) = catalog.connection.query_row(
        "SELECT COUNT(*), COUNT(DISTINCT source_generation) FROM asset_locations WHERE scan_id = 'scan'",
        [], |row| Ok((row.get(0)?, row.get(1)?)),
    ).expect("complete independent rows");
    assert_eq!(counts, (256, 256));
    assert_eq!(preparations.load(Ordering::Relaxed), 1);
}

#[test]
fn rejected_commit_preserves_cached_insert_and_replaces_optional_bindings_on_retry() {
    let directory = tempdir().expect("isolated storage");
    let mut catalog = catalog(directory.path().join("catalog.sqlite3"));
    let mut first = location(1);
    first.created_unix_ms = Some(1000);
    first.capture_time = Some(CaptureTimeEvidence {
        local_time: "2000-01-02T03:04:05".to_owned(),
        offset_minutes: Some(480),
        source: CaptureTimeSource::Original,
        raw_value: "2000:01:02 03:04:05".to_owned(),
    });
    catalog
        .stage_location("scan", "root", &first)
        .expect("stage dated row");
    catalog.flush_pending_locations().expect("commit dated row");
    let reject = Arc::new(AtomicBool::new(true));
    let observed = Arc::clone(&reject);
    catalog
        .connection
        .commit_hook(Some(move || observed.swap(false, Ordering::AcqRel)))
        .expect("refuse one commit");
    let mut second = location(2);
    second.width = 30;
    catalog
        .stage_location("scan", "root", &second)
        .expect("stage undated row");
    assert!(catalog.flush_pending_locations().is_err());
    assert_eq!(catalog.pending_locations.len(), 1);
    let absent: i64 = catalog
        .connection
        .query_row(
            "SELECT COUNT(*) FROM assets WHERE id = 'asset-2'",
            [],
            |row| row.get(0),
        )
        .expect("rolled-back asset insertion");
    assert_eq!(absent, 0);
    catalog
        .flush_pending_locations()
        .expect("retry complete row transaction");
    assert!(catalog.pending_locations.is_empty());
    let rows: (i64, i64) = catalog.connection.query_row(
        "SELECT
           (SELECT COUNT(*) FROM asset_locations WHERE location_id = 'location-1'
             AND created_unix_ms = 1000 AND capture_local_time = '2000-01-02T03:04:05'
             AND capture_offset_minutes = 480 AND capture_raw_value = '2000:01:02 03:04:05' AND width = 10),
           (SELECT COUNT(*) FROM asset_locations WHERE location_id = 'location-2'
             AND created_unix_ms IS NULL AND capture_local_time IS NULL AND capture_offset_minutes IS NULL
             AND capture_time_source IS NULL AND capture_raw_value IS NULL AND width = 30)",
        [], |row| Ok((row.get(0)?, row.get(1)?)),
    ).expect("independent complete parameter bindings");
    assert_eq!(rows, (1, 1));
    first.created_unix_ms = None;
    first.capture_time = None;
    first.height = 40;
    catalog
        .stage_location("scan", "root", &first)
        .expect("stage cleared dates");
    catalog
        .flush_pending_locations()
        .expect("upsert cleared dates");
    let cleared: i64 = catalog.connection.query_row(
        "SELECT COUNT(*) FROM asset_locations WHERE location_id = 'location-1'
         AND created_unix_ms IS NULL AND capture_local_time IS NULL AND capture_offset_minutes IS NULL
         AND capture_time_source IS NULL AND capture_raw_value IS NULL AND height = 40", [], |row| row.get(0),
    ).expect("upsert replaces nullable values");
    assert_eq!(cleared, 1);
}
