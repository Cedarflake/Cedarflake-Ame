use crate::domain::ScanRequest;
use crate::ports::CatalogRepository;

use super::*;

#[test]
fn authoritative_retry_handoff_is_transactional_p0_evidence() {
    let source = tempfile::tempdir().expect("controlled source");
    let storage = tempfile::tempdir().expect("catalog storage");
    let mut catalog = SqliteCatalog::open(storage.path().join("catalog.sqlite3")).expect("catalog");
    let root_path = source.path().to_string_lossy().into_owned();
    catalog
        .begin_authoritative_scan(
            &ScanRequest {
                scan_id: "handoff-projection".to_owned(),
                root_path: root_path.clone(),
                max_items: None,
                max_entries: None,
                preview_edge: 128,
            },
            "root",
            &root_path,
        )
        .expect("authoritative owner");
    let paths = vec!["changed.png".to_owned(), "missing.png".to_owned()];
    let transaction = catalog.begin_write().expect("rollback handoff transaction");
    enqueue_authoritative_retry_paths(&transaction, "root", 1, &paths, 42)
        .expect("prepare handoff");
    drop(transaction);
    let rolled_back: i64 = catalog
        .connection
        .query_row("SELECT COUNT(*) FROM library_change_queue", [], |row| {
            row.get(0)
        })
        .expect("rollback queue count");
    assert_eq!(rolled_back, 0);

    let transaction = catalog
        .begin_write()
        .expect("committed handoff transaction");
    enqueue_authoritative_retry_paths(&transaction, "root", 1, &paths, 42)
        .expect("prepare P0 handoff");
    transaction.commit().expect("commit P0 handoff");
    let mut statement = catalog
        .connection
        .prepare(
            "SELECT queue.relative_path, queue.origin, lane.lane, queue.status
         FROM library_change_queue AS queue
         JOIN library_change_queue_lanes AS lane ON lane.change_id = queue.id
         WHERE queue.root_id = 'root' ORDER BY queue.relative_path",
        )
        .expect("durable handoff lanes");
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })
        .expect("handoff rows")
        .collect::<Result<Vec<_>, _>>()
        .expect("handoff evidence");
    assert_eq!(
        rows,
        paths
            .into_iter()
            .map(|path| (
                path,
                "live_notification".to_owned(),
                "p0_live".to_owned(),
                "pending".to_owned(),
            ))
            .collect::<Vec<_>>()
    );
}
