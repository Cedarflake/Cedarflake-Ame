use rusqlite::params;

use super::*;

fn legacy_mixed_checkpoint() -> PausedHardLinks {
    let fixture = paused_rewritten_hard_links();
    let original = &fixture.original;
    let revision = original
        .source_revision
        .as_ref()
        .expect("Windows source revision");
    let connection = Connection::open(&fixture.storage.catalog_path).expect("legacy checkpoint");
    assert_eq!(
        connection
            .execute(
                "UPDATE asset_locations
         SET source_generation = ?1, source_revision_token = ?2, file_size = ?3,
             modified_unix_ms = ?4, width = ?5, height = ?6
         WHERE scan_id = ?7 AND relative_path = 'B/image.png'",
                params![
                    i64::try_from(original.source_generation).expect("stored generation"),
                    format!("{}:{}", revision.scheme, revision.value),
                    i64::try_from(original.file_size).expect("fixture file size"),
                    original.modified_unix_ms,
                    original.width,
                    original.height,
                    fixture.request.scan_id
                ],
            )
            .expect("restore the unvisited alias as retained by the prior writer"),
        1
    );
    let generations: (i64, i64) = connection
        .query_row(
            "SELECT MIN(source_generation), MAX(source_generation) FROM asset_locations",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("mixed legacy generations");
    assert!(generations.0 < generations.1);
    drop(connection);
    fixture
}

fn stored_state(fixture: &PausedHardLinks) -> Vec<Vec<rusqlite::types::Value>> {
    let connection = Connection::open(&fixture.storage.catalog_path).expect("stored checkpoint");
    let mut statement = connection
        .prepare(
            "SELECT location.*, EXISTS (
             SELECT 1 FROM scan_resume_pending_locations AS pending
             WHERE pending.scan_id = location.scan_id AND pending.location_id = location.location_id
         ) FROM asset_locations AS location ORDER BY location_id",
        )
        .expect("checkpoint rows and resume membership");
    let columns = statement.column_count();
    statement
        .query_map([], |row| {
            (0..columns).map(|column| row.get(column)).collect()
        })
        .expect("checkpoint state")
        .collect::<Result<Vec<_>, _>>()
        .expect("checkpoint values")
}

fn assert_sources_unchanged(fixture: &PausedHardLinks) {
    for path in &fixture.paths {
        assert_eq!(
            std::fs::read(path).expect("preserved source"),
            fixture.current_bytes
        );
    }
}

#[test]
fn old_mixed_checkpoint_reopens_and_resumes_without_discarding_the_asset() {
    let fixture = legacy_mixed_checkpoint();
    drop(SqliteCatalog::open(fixture.storage.catalog_path.clone()).expect("repair old checkpoint"));
    let connection = Connection::open(&fixture.storage.catalog_path).expect("repaired checkpoint");
    let retained: (i64, i64, i64, String) = connection
        .query_row(
            "SELECT (SELECT COUNT(*) FROM assets), (SELECT COUNT(*) FROM asset_locations),
                (SELECT COUNT(*) FROM scan_resume_pending_locations),
                (SELECT asset_id FROM asset_locations)",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .expect("only the unverified old alias was retired");
    assert_eq!(retained, (1, 1, 0, fixture.original.asset_id.clone()));
    drop(connection);
    let repaired = stored_state(&fixture);
    drop(SqliteCatalog::open(fixture.storage.catalog_path.clone()).expect("idempotent repair"));
    assert_eq!(stored_state(&fixture), repaired);
    let mut completed = false;
    resume_scan_with_storage(
        fixture.request.clone(),
        |event| {
            completed |= matches!(event, ScanEvent::Completed { asset_count: 2, .. });
            true
        },
        fixture.storage.clone(),
    )
    .expect("resume the old checkpoint through the application");
    assert!(completed);
    let mut catalog =
        SqliteCatalog::open(fixture.storage.catalog_path.clone()).expect("completed reopen");
    assert!(
        catalog
            .load_single_recoverable_foreground_scan()
            .expect("terminal scan")
            .is_none()
    );
    assert_sources_unchanged(&fixture);
}

#[test]
fn old_mixed_checkpoint_can_be_cancelled_through_the_application() {
    let fixture = legacy_mixed_checkpoint();
    let mut cancelled = false;
    resume_scan_with_storage(
        fixture.request.clone(),
        |event| {
            if matches!(event, ScanEvent::Started { .. }) {
                assert!(cancel_scan(&fixture.request.scan_id));
            }
            assert!(!matches!(event, ScanEvent::AssetDiscovered { .. }));
            cancelled |= matches!(event, ScanEvent::Cancelled { .. });
            true
        },
        fixture.storage.clone(),
    )
    .expect("cancel a legacy checkpoint after open repair");
    assert!(cancelled);
    let mut catalog =
        SqliteCatalog::open(fixture.storage.catalog_path.clone()).expect("cancelled reopen");
    assert!(
        catalog
            .load_single_recoverable_foreground_scan()
            .expect("terminal scan")
            .is_none()
    );
    assert_sources_unchanged(&fixture);
}

#[test]
fn mixed_checkpoint_with_no_revisited_sibling_is_rejected_unchanged() {
    let fixture = legacy_mixed_checkpoint();
    let connection = Connection::open(&fixture.storage.catalog_path).expect("unproved checkpoint");
    connection
        .execute_batch(
            "UPDATE scan_runs SET status = 'running';
         INSERT INTO scan_resume_pending_locations(scan_id, location_id)
             SELECT scan_id, location_id FROM asset_locations WHERE relative_path = 'A/image.png';
         UPDATE scan_runs SET status = 'paused';",
        )
        .expect("both aliases are unverified");
    drop(connection);
    assert_rejected_unchanged(&fixture);
}

#[test]
fn equal_generation_conflict_is_rejected_unchanged() {
    let fixture = legacy_mixed_checkpoint();
    let connection = Connection::open(&fixture.storage.catalog_path).expect("unproved generation");
    connection.execute_batch(
        "UPDATE asset_locations SET source_generation = (SELECT MAX(source_generation) FROM asset_locations)
         WHERE relative_path = 'B/image.png'",
    ).expect("no newer generation proves a safe retirement");
    drop(connection);
    assert_rejected_unchanged(&fixture);
}

#[test]
fn full_validation_failure_rolls_back_old_alias_and_resume_membership_retirement() {
    let fixture = legacy_mixed_checkpoint();
    let connection = Connection::open(&fixture.storage.catalog_path).expect("invalid allocator");
    connection.execute_batch(
        "UPDATE catalog_state SET next_source_generation = (SELECT MAX(source_generation) FROM asset_locations)",
    ).expect("unrelated invalid source-generation allocation");
    drop(connection);
    assert_rejected_unchanged(&fixture);
}

#[test]
fn matching_malformed_windows_id_cannot_authorize_old_alias_retirement() {
    let fixture = legacy_mixed_checkpoint();
    let connection = Connection::open(&fixture.storage.catalog_path).expect("malformed identity");
    connection.execute_batch(
        "UPDATE asset_locations
         SET file_identity_value = substr(file_identity_value, 1, 18) || ':' || substr(file_identity_value, 20)",
    ).expect("same malformed identity with an extra colon in both aliases");
    drop(connection);
    assert_rejected_unchanged(&fixture);
}

fn assert_rejected_unchanged(fixture: &PausedHardLinks) {
    let before = stored_state(fixture);
    let error = match SqliteCatalog::open(fixture.storage.catalog_path.clone()) {
        Ok(_) => panic!("unproved source state was admitted"),
        Err(error) => error,
    };
    assert_eq!(error.code, "catalog_source_revision_contract_unverifiable");
    assert_eq!(stored_state(fixture), before);
    assert_sources_unchanged(fixture);
}
