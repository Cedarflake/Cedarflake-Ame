use super::*;

#[test]
fn changed_source_rebases_a_peer_publication_before_preparation() {
    let (fixture, ready) = ready_source_fixture("source-publication-conflict");
    let source_bytes = fs::read(&fixture.source_path).expect("fixture bytes");
    let connection = Connection::open(&fixture.storage.catalog_path).expect("catalog");
    connection
        .execute(
            "UPDATE asset_locations SET preview_status = 'pending', preview_path = ''
             WHERE location_id = ?1",
            [&ready.location_id],
        )
        .expect("cold preview fixture");
    connection
        .execute_batch(
            "CREATE TRIGGER fixture_peer_catalog_publication
             AFTER INSERT ON library_change_queue
             WHEN NEW.origin = 'consistency_audit'
             BEGIN UPDATE catalog_state SET revision = revision + 1; END;",
        )
        .expect("deterministic peer revision publication");
    drop(connection);
    let original = active_location(&fixture);
    fs::write(&fixture.source_path, &source_bytes).expect("rewrite owned source");
    let error = materialize_preview_with_storage(
        preview_request_for(&original, 256, false),
        fixture.storage.clone(),
    )
    .expect_err("old source request cannot publish");
    let connection = Connection::open(&fixture.storage.catalog_path).expect("catalog");
    let (id, status, failure, attempts, retry_at): (i64, String, Option<String>, i64, Option<i64>) =
        connection
            .query_row(
                "SELECT id, status, last_failure_code, attempt_count, next_retry_unix_ms
             FROM library_change_queue WHERE origin = 'consistency_audit'",
                [],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                    ))
                },
            )
            .expect("completed path reconciliation");
    assert_eq!(status, "completed");
    assert_eq!(failure, None);
    assert_eq!(retry_at, None);
    assert_eq!(attempts, 1);
    assert_eq!(
        fs::read(&fixture.source_path).expect("source bytes"),
        source_bytes
    );
    assert_eq!(error.code, "preview_request_superseded");
    connection
        .execute_batch("DROP TRIGGER fixture_peer_catalog_publication")
        .expect("end peer publication");
    drop(connection);

    let current = active_location(&fixture);
    assert!(current.source_generation > original.source_generation);
    assert_ne!(current.source_revision, original.source_revision);
    assert_eq!(current.preview_status, PreviewStatus::Pending);
    let generated = materialize_preview_with_storage(
        preview_request_for(&current, 256, false),
        fixture.storage.clone(),
    )
    .expect("current source preview");
    assert_eq!(generated.preview_status, PreviewStatus::Ready);
    assert_eq!(
        fs::read(&fixture.source_path).expect("final source bytes"),
        source_bytes
    );
    let connection = Connection::open(&fixture.storage.catalog_path).expect("catalog");
    let (count, final_status, final_attempts): (i64, String, i64) = connection
        .query_row(
            "SELECT (SELECT COUNT(*) FROM library_change_queue), status, attempt_count
             FROM library_change_queue WHERE id = ?1",
            [id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("same path owner completion");
    assert_eq!(count, 1);
    assert_eq!(final_status, "completed");
    assert_eq!(final_attempts, 1);
}

#[test]
fn failed_durable_retry_is_not_reported_as_retired_preview_work() {
    let (fixture, ready) = ready_source_fixture("source-retry-storage-failure");
    let source_bytes = fs::read(&fixture.source_path).expect("fixture bytes");
    let connection = Connection::open(&fixture.storage.catalog_path).expect("catalog");
    connection
        .execute_batch(
            "CREATE TRIGGER fixture_peer_catalog_publication
             AFTER INSERT ON library_change_queue
             WHEN NEW.origin = 'consistency_audit'
             BEGIN UPDATE catalog_state SET revision = revision + 1; END;
             CREATE TRIGGER fixture_retry_storage_failure
             BEFORE UPDATE OF status ON library_change_queue
             WHEN NEW.status = 'retry_wait'
             BEGIN SELECT RAISE(ABORT, 'fixture retry storage failure'); END;",
        )
        .expect("peer commit followed by failed retry persistence");
    drop(connection);
    fs::write(&fixture.source_path, &source_bytes).expect("rewrite owned source");
    let _publication_failure =
        crate::adapters::set_before_catalog_delta_commit_hook(&ready.root_id, || {
            Err(ScanError::new(
                "root_publication_namespace_changed",
                "fixture publication namespace failure",
            ))
        });
    let error = materialize_preview_with_storage(
        preview_request_for(&ready, 256, false),
        fixture.storage.clone(),
    )
    .expect_err("storage failure must remain visible");
    assert_ne!(error.code, "preview_request_superseded");
    assert!(error.message.contains("fixture retry storage failure"));
    assert_eq!(active_location(&fixture), ready);
    assert_eq!(
        fs::read(&fixture.source_path).expect("source bytes"),
        source_bytes
    );
    let connection = Connection::open(&fixture.storage.catalog_path).expect("catalog");
    let (status, failure): (String, Option<String>) = connection
        .query_row(
            "SELECT status, last_failure_code FROM library_change_queue
             WHERE origin = 'consistency_audit'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("retry transaction rolled back");
    assert_eq!(status, "leased");
    assert_eq!(failure, None);
}
