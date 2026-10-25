use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use rusqlite::{Connection, TransactionBehavior};
use tempfile::{TempDir, tempdir};

use crate::adapters::sqlite_catalog::tests::support::{publish_fixture, publish_preview_artifact};
use crate::ports::{CatalogRepository, GalleryQueryRepository};

use crate::adapters::sqlite_catalog::*;

fn fixture() -> (TempDir, SqliteCatalog, Vec<(String, String)>) {
    let directory = tempdir().unwrap();
    let mut catalog = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
    publish_fixture(&mut catalog, "scan", "root", "C:\\PreviewUsage", "location");
    let artifact = publish_preview_artifact(
        &mut catalog,
        "location",
        "artifact",
        "C:\\AmeCache\\usage.jpg",
    );
    catalog
        .connection
        .execute(
            "UPDATE catalog_state SET next_source_generation = MAX(next_source_generation,
         (SELECT MAX(source_generation) + 1 FROM asset_locations))",
            [],
        )
        .unwrap();
    catalog
        .connection
        .execute("UPDATE preview_artifacts SET last_used_unix_ms = 0", [])
        .unwrap();
    (directory, catalog, vec![("location".into(), artifact.path)])
}

#[test]
fn usage_publication_does_not_wait_for_an_in_process_writer() {
    let (_directory, mut catalog, visible) = fixture();
    let mut reader = SqliteCatalog::open(catalog.path.clone()).unwrap();
    let transaction = catalog.begin_write().unwrap();
    let (sender, receiver) = mpsc::channel();
    let reader_visible = visible.clone();
    let worker = thread::spawn(move || {
        let result = reader
            .load_query_snapshot(8, &GalleryQuery::default(), "while-writing", None)
            .and_then(|snapshot| {
                let touched = reader.touch_preview_artifacts(&reader_visible)?;
                Ok((snapshot, touched))
            });
        sender.send(result).unwrap();
    });
    let result = receiver.recv_timeout(Duration::from_millis(250));
    drop(transaction);
    worker.join().unwrap();
    let (snapshot, touched) = result.expect("loaded page waited for the writer").unwrap();
    assert_eq!(touched, 0);
    assert_eq!(snapshot.snapshot.assets.len(), 1);
    assert_eq!(snapshot.snapshot.assets[0].preview_path, visible[0].1);
    assert!(matches!(
        snapshot.snapshot.assets[0].preview_status,
        PreviewStatus::Ready
    ));
    assert_eq!(catalog.touch_preview_artifacts(&visible).unwrap(), 1);
}

#[test]
fn external_writer_defers_usage_without_losing_the_next_idle_touch() {
    let (_directory, mut catalog, visible) = fixture();
    let mut blocker = Connection::open(&catalog.path).unwrap();
    let transaction = blocker
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .unwrap();
    let started = Instant::now();
    let outcome = catalog.touch_preview_artifacts(&visible);
    let elapsed = started.elapsed();
    drop(transaction);
    assert_eq!(outcome.unwrap(), 0);
    assert!(elapsed < Duration::from_millis(250));
    let timeout: i64 = catalog
        .connection
        .query_row("PRAGMA busy_timeout", [], |row| row.get(0))
        .unwrap();
    assert_eq!(
        timeout,
        i64::try_from(SQLITE_BUSY_TIMEOUT.as_millis()).unwrap()
    );
    assert_eq!(catalog.touch_preview_artifacts(&visible).unwrap(), 1);
    assert_eq!(catalog.touch_preview_artifacts(&visible).unwrap(), 0);
}

#[test]
fn usage_errors_remain_visible_and_allow_later_retry() {
    let (_directory, mut catalog, visible) = fixture();
    catalog
        .connection
        .execute_batch(
            "CREATE TRIGGER reject_usage BEFORE UPDATE ON preview_artifacts
         BEGIN SELECT RAISE(ABORT, 'usage fixture failure'); END;",
        )
        .unwrap();
    assert!(catalog.touch_preview_artifacts(&visible).is_err());
    let used: i64 = catalog
        .connection
        .query_row(
            "SELECT last_used_unix_ms FROM preview_artifacts",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(used, 0);
    catalog
        .connection
        .execute_batch("DROP TRIGGER reject_usage;")
        .unwrap();
    assert_eq!(catalog.touch_preview_artifacts(&visible).unwrap(), 1);
    let oversized = vec![visible[0].clone(); 4097];
    assert_eq!(
        catalog
            .touch_preview_artifacts(&oversized)
            .unwrap_err()
            .code,
        "preview_touch_batch_too_large"
    );
}
