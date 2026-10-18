use crate::adapters::sqlite_catalog::tests::support::{
    fixture_request, publication_identity, publish_fixture,
};

use tempfile::tempdir;

use crate::adapters::sqlite_catalog::*;

#[cfg(windows)]
#[test]
fn staged_source_change_never_mutates_the_active_projection_before_publication() {
    let directory = tempdir().expect("temporary directory");
    let catalog_path = directory.path().join("catalog.sqlite3");
    let source = tempdir().expect("source directory");
    let source_path = source.path().join("same.jpg");
    fs::write(&source_path, b"source-version-one").expect("initial source");
    let root_path = source.path().to_string_lossy().into_owned();
    let discovery = crate::adapters::FileDiscovery::new(&root_path).expect("source discovery");
    let first_file = match discovery.visit_relative_path("same.jpg").outcome {
        crate::adapters::FileVisitOutcome::File(file) => file,
        _ => panic!("initial source file"),
    };
    let mut catalog = SqliteCatalog::open(catalog_path.clone()).expect("catalog");
    let first_scan = fixture_request("source-v1-scan", &root_path);
    catalog
        .begin_scan(&first_scan, "source-root", &root_path)
        .expect("begin initial scan");
    catalog
        .prove_live_only_first_import_handoff_for_test(&first_scan.scan_id)
        .expect("prove initial source first-import handoff");
    let first_location = AssetLocationView {
        asset_id: "source-asset".to_owned(),
        location_id: "source-location".to_owned(),
        root_id: "source-root".to_owned(),
        scan_id: first_scan.scan_id.clone(),
        absolute_path: first_file.absolute_path.clone(),
        display_path: first_file.absolute_path.clone(),
        relative_path: first_file.relative_path.clone(),
        preview_path: "C:\\Cache\\source-v1.jpg".to_owned(),
        file_size: first_file.file_size,
        created_unix_ms: first_file.created_unix_ms,
        modified_unix_ms: first_file.modified_unix_ms,
        file_identity: first_file.file_identity.clone(),
        source_revision: first_file.source_revision.clone(),
        source_generation: 0,
        width: 10,
        height: 10,
        preview_status: PreviewStatus::Ready,
        preview_issue_code: None,
        preview_issue_message: None,
        metadata_engine_id: "fixture".to_owned(),
        metadata_engine_version: "1".to_owned(),
        capture_time: None,
    };
    catalog
        .stage_location(&first_scan.scan_id, "source-root", &first_location)
        .expect("stage initial source");
    catalog
        .publish_scan(&first_scan.scan_id, "source-root", 1, 0)
        .expect("publish initial source");
    let active_before = catalog
        .load_active_location("source-location")
        .expect("load initial source")
        .expect("initial source location");
    let assert_active_unchanged = |actual: &AssetLocationView| {
        assert_eq!(actual.asset_id, active_before.asset_id);
        assert_eq!(actual.scan_id, active_before.scan_id);
        assert_eq!(actual.file_size, active_before.file_size);
        assert_eq!(actual.modified_unix_ms, active_before.modified_unix_ms);
        assert_eq!(actual.file_identity, active_before.file_identity);
        assert_eq!(actual.source_revision, active_before.source_revision);
        assert_eq!(actual.source_generation, active_before.source_generation);
        assert_eq!(actual.preview_path, active_before.preview_path);
        assert_eq!(
            std::mem::discriminant(&actual.preview_status),
            std::mem::discriminant(&active_before.preview_status),
        );
    };

    fs::write(&source_path, b"source-version-two").expect("change source before staging");
    let second_file = match discovery.visit_relative_path("same.jpg").outcome {
        crate::adapters::FileVisitOutcome::File(file) => file,
        _ => panic!("changed source file"),
    };
    assert_eq!(second_file.file_identity, first_file.file_identity);
    assert_ne!(second_file.source_revision, first_file.source_revision);
    let second_scan = fixture_request("source-v2-scan", &root_path);
    catalog
        .begin_scan(&second_scan, "source-root", &root_path)
        .expect("begin replacement scan");
    let mut second_location = first_location;
    second_location.scan_id = second_scan.scan_id.clone();
    second_location.preview_path.clear();
    second_location.file_size = second_file.file_size;
    second_location.created_unix_ms = second_file.created_unix_ms;
    second_location.modified_unix_ms = second_file.modified_unix_ms;
    second_location.file_identity = second_file.file_identity.clone();
    second_location.source_revision = second_file.source_revision.clone();
    second_location.source_generation = 0;
    second_location.preview_status = PreviewStatus::Pending;
    catalog
        .stage_location(&second_scan.scan_id, "source-root", &second_location)
        .expect("stage changed source");
    assert_eq!(
        catalog
            .count_staged_file_states(&second_scan.scan_id)
            .expect("flush staged source"),
        1
    );
    let active_after_flush = catalog
        .load_active_location("source-location")
        .expect("load active source after staging")
        .expect("active source after staging");
    assert_active_unchanged(&active_after_flush);

    fs::write(&source_path, b"source-version-three").expect("change source after staged flush");
    catalog
        .abandon_scan(&second_scan.scan_id, "stale", 1)
        .expect("abandon stale replacement scan");
    let active_after_abandon = catalog
        .load_active_location("source-location")
        .expect("load active source after abandon")
        .expect("active source after abandon");
    assert_active_unchanged(&active_after_abandon);
    drop(catalog);
    let reopened = SqliteCatalog::open(catalog_path).expect("reopen coherent catalog");
    let active_after_reopen = reopened
        .load_active_location("source-location")
        .expect("load active source after reopen")
        .expect("active source after reopen");
    assert_active_unchanged(&active_after_reopen);
}

#[test]
fn scan_publication_atomically_advances_active_hardlink_aliases() {
    let directory = tempdir().expect("temporary directory");
    let mut catalog =
        SqliteCatalog::open(directory.path().join("catalog.sqlite3")).expect("catalog");
    publish_fixture(
        &mut catalog,
        "hardlink-root-a-v1",
        "hardlink-root-a",
        "C:\\HardlinkRootA",
        "hardlink-location-a",
    );
    publish_fixture(
        &mut catalog,
        "hardlink-root-b-v1",
        "hardlink-root-b",
        "C:\\HardlinkRootB",
        "hardlink-location-b",
    );
    catalog
        .connection
        .execute_batch(
            "UPDATE asset_locations
             SET file_identity_scheme = 'windows-file-id-128-v1',
                 file_identity_value =
                   '0000000000000001:00000000000000000000000000000044',
                 source_revision_token =
                   'windows-file-change-time-100ns-v1:0000000000000044',
                 source_generation = 1, preview_path = '', preview_status = 'pending';
             UPDATE catalog_state SET next_source_generation = 2;",
        )
        .expect("establish shared active identity");
    let replacement = fixture_request("hardlink-root-a-v2", "C:\\HardlinkRootA");
    catalog
        .begin_scan(&replacement, "hardlink-root-a", "C:\\HardlinkRootA")
        .expect("begin hardlink replacement scan");
    catalog
        .connection
        .execute(
            "INSERT INTO asset_locations(
               scan_id, asset_id, location_id, root_id, absolute_path, relative_path,
               preview_path, file_size, created_unix_ms, modified_unix_ms,
               file_local_time, parent_relative_path, natural_name_key, width, height,
               preview_status, preview_issue_code, preview_issue_message,
               metadata_engine_id, metadata_engine_version, capture_local_time,
               capture_offset_minutes, capture_time_source, capture_raw_value,
               file_identity_scheme, file_identity_value, source_revision_token,
               source_generation
             )
             SELECT ?1, asset_id, location_id, root_id, absolute_path, relative_path,
                    '', 44, created_unix_ms, 55, file_local_time,
                    parent_relative_path, natural_name_key, 88, 66, 'pending', NULL, NULL,
                    'metadata-v2', '2', NULL, NULL, NULL, NULL,
                    file_identity_scheme, file_identity_value,
                    'windows-file-change-time-100ns-v1:0000000000000055', 2
             FROM asset_locations
             WHERE scan_id = 'hardlink-root-a-v1'
               AND location_id = 'hardlink-location-a'",
            [&replacement.scan_id],
        )
        .expect("stage replacement identity observation");
    catalog
        .connection
        .execute("UPDATE catalog_state SET next_source_generation = 3", [])
        .expect("advance provisional source allocator");
    let sibling_before = catalog
        .load_active_location("hardlink-location-b")
        .expect("load sibling before publication")
        .expect("sibling before publication");
    assert_eq!(sibling_before.file_size, 20);
    assert_eq!(sibling_before.source_generation, 1);

    catalog
        .publish_scan(&replacement.scan_id, "hardlink-root-a", 1, 0)
        .expect("publish replacement identity observation");

    let first = catalog
        .load_active_location("hardlink-location-a")
        .expect("load published first alias")
        .expect("published first alias");
    let second = catalog
        .load_active_location("hardlink-location-b")
        .expect("load published second alias")
        .expect("published second alias");
    for location in [first, second] {
        assert_eq!(location.file_size, 44);
        assert_eq!(location.modified_unix_ms, 55);
        assert_eq!((location.width, location.height), (88, 66));
        assert_eq!(location.metadata_engine_id, "metadata-v2");
        assert_eq!(location.metadata_engine_version, "2");
        assert_eq!(
            location.source_revision,
            Some(SourceRevisionEvidence {
                scheme: "windows-file-change-time-100ns-v1".to_owned(),
                value: "0000000000000055".to_owned(),
            })
        );
        assert_eq!(location.source_generation, 3);
        assert!(matches!(location.preview_status, PreviewStatus::Pending));
    }
}

fn insert_publication_gate_live_change(
    catalog: &SqliteCatalog,
    root_id: &str,
    intent_kind: &str,
    scope: &str,
    relative_path: &str,
    status: &str,
    authoritative_scan_id: Option<&str>,
) -> i64 {
    let revision: i64 = catalog
        .connection
        .query_row("SELECT revision FROM catalog_state", [], |row| row.get(0))
        .expect("publication gate catalog revision");
    catalog
        .connection
        .execute(
            "INSERT INTO library_change_queue(
               root_id, root_generation, intent_kind, scope, relative_path,
               origin, first_observed_unix_ms, most_recent_observed_unix_ms,
               first_sequence, most_recent_sequence, coalesced_observation_count,
               status, ready_unix_ms, next_retry_unix_ms, lease_expires_unix_ms,
               last_failure_code, last_failure_message,
               catalog_revision_at_enqueue, authoritative_scan_id,
               created_unix_ms, updated_unix_ms
             ) VALUES (
               ?1, 1, ?2, ?3, ?4, 'live_notification', 50, 50, '1', '1', 1,
               ?5, 50, CASE WHEN ?5 = 'retry_wait' THEN 75 ELSE NULL END,
               CASE WHEN ?5 = 'leased' THEN 100 ELSE NULL END,
               CASE WHEN ?5 = 'retry_wait' THEN 'fixture_retry' ELSE NULL END,
               CASE WHEN ?5 = 'retry_wait' THEN 'Fixture retry debt' ELSE NULL END,
               ?6, ?7, 50, 50
             )",
            params![
                root_id,
                intent_kind,
                scope,
                relative_path,
                status,
                revision,
                authoritative_scan_id,
            ],
        )
        .expect("insert publication gate live change");
    catalog.connection.last_insert_rowid()
}

#[test]
fn first_import_allows_durable_exact_path_work_but_keeps_it_pending() {
    let directory = tempdir().expect("temporary directory");
    let mut catalog =
        SqliteCatalog::open(directory.path().join("catalog.sqlite3")).expect("catalog");
    let request = fixture_request("first-path-scan", "C:\\FirstPath");
    catalog
        .begin_scan(&request, "first-path-root", &request.root_path)
        .expect("begin first import");
    catalog
        .prove_live_only_first_import_handoff_for_test(&request.scan_id)
        .expect("prove first import change capture");
    let change_id = insert_publication_gate_live_change(
        &catalog,
        "first-path-root",
        "reconcile",
        "path",
        "changed.jpg",
        "pending",
        None,
    );

    catalog
        .publish_scan(&request.scan_id, "first-path-root", 0, 0)
        .expect("publish first import around durable exact path work");

    let retained: (String, Option<i64>) = catalog
        .connection
        .query_row(
            "SELECT status, catalog_revision_at_success
             FROM library_change_queue WHERE id = ?1",
            [change_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("retained first-import path work");
    assert_eq!(retained, ("pending".to_owned(), None));
}

#[test]
fn first_import_retains_non_path_live_work_without_claiming_freshness() {
    for (kind, scope, relative_path) in [
        ("freshness_unknown", "root", ""),
        ("reconcile", "root", ""),
        ("reconcile", "subtree", "incoming"),
    ] {
        for status in ["pending", "leased", "retry_wait"] {
            assert_first_import_retains_live_work(kind, scope, relative_path, status);
        }
    }
}

fn assert_first_import_retains_live_work(
    kind: &str,
    scope: &str,
    relative_path: &str,
    status: &str,
) {
    let directory = tempdir().expect("temporary directory");
    let mut catalog =
        SqliteCatalog::open(directory.path().join("catalog.sqlite3")).expect("catalog");
    let request = fixture_request("first-root-gap-scan", "C:\\FirstRootGap");
    catalog
        .begin_scan(&request, "first-root-gap", &request.root_path)
        .expect("begin first import");
    catalog
        .prove_live_only_first_import_handoff_for_test(&request.scan_id)
        .expect("prove first import change capture");
    let change_id = insert_publication_gate_live_change(
        &catalog,
        "first-root-gap",
        kind,
        scope,
        relative_path,
        status,
        None,
    );

    catalog
        .publish_scan(&request.scan_id, "first-root-gap", 0, 0)
        .expect("publish a baseline so live work can become eligible");
    let retained: (String, Option<i64>) = catalog
        .connection
        .query_row(
            "SELECT status, catalog_revision_at_success
             FROM library_change_queue WHERE id = ?1",
            [change_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("unconsumed live work");
    assert_eq!(retained, (status.to_owned(), None));
    let authority: (String, String) = catalog
        .connection
        .query_row(
            "SELECT roots.active_scan_id, journal.continuity_state
             FROM library_roots AS roots
             JOIN library_persistent_journal_root_state AS journal ON journal.root_id = roots.id
             WHERE roots.id = 'first-root-gap'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("non-current baseline authority");
    assert_eq!(authority, (request.scan_id, "live_only".to_owned()));
}

#[test]
fn replacement_blocks_pending_path_work_but_preserves_retry_wait_after_publication() {
    let directory = tempdir().expect("temporary directory");
    let mut catalog =
        SqliteCatalog::open(directory.path().join("catalog.sqlite3")).expect("catalog");
    publish_fixture(
        &mut catalog,
        "path-gate-initial",
        "path-gate-root",
        "C:\\PathGate",
        "path-gate-location",
    );
    let replacement = fixture_request("path-gate-replacement", "C:\\PathGate");
    catalog
        .begin_scan(&replacement, "path-gate-root", &replacement.root_path)
        .expect("begin replacement");
    let change_id = insert_publication_gate_live_change(
        &catalog,
        "path-gate-root",
        "reconcile",
        "path",
        "changed.jpg",
        "pending",
        Some(&replacement.scan_id),
    );
    catalog
        .connection
        .execute(
            "UPDATE scan_runs SET change_queue_high_watermark = ?2 WHERE id = ?1",
            params![replacement.scan_id, change_id],
        )
        .expect("bind replacement high watermark");

    let error = catalog
        .publish_scan(&replacement.scan_id, "path-gate-root", 0, 0)
        .expect_err("pending path work blocks replacement");
    assert_eq!(error.code, "catalog_scan_live_changes_pending");

    catalog
        .connection
        .execute(
            "UPDATE library_change_queue
             SET status = 'retry_wait', next_retry_unix_ms = 75,
                 last_failure_code = 'fixture_retry',
                 last_failure_message = 'Fixture retry debt'
             WHERE id = ?1",
            [change_id],
        )
        .expect("move path work to durable retry debt");
    catalog
        .publish_scan(&replacement.scan_id, "path-gate-root", 0, 0)
        .expect("retry-wait exact path does not hold the old snapshot hostage");

    let retained: (String, Option<String>, Option<i64>) = catalog
        .connection
        .query_row(
            "SELECT status, authoritative_scan_id, catalog_revision_at_success
             FROM library_change_queue WHERE id = ?1",
            [change_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("retained path retry debt");
    assert_eq!(retained, ("retry_wait".to_owned(), None, None));
}

#[test]
fn foreground_publication_binding_is_atomic_and_replacement_retires_the_generation() {
    let directory = tempdir().expect("catalog directory");
    let mut catalog =
        SqliteCatalog::open(directory.path().join("catalog.sqlite3")).expect("catalog");
    let root_id = "publication-root";
    let root_path = "C:\\PublicationRoot";
    let first = fixture_request("publication-first", root_path);
    let first_identity = publication_identity('1');
    catalog
        .begin_scan_with_publication_namespace(&first, root_id, root_path, &first_identity)
        .expect("bind first foreground scan");
    catalog
        .prove_live_only_first_import_handoff_for_test(&first.scan_id)
        .expect("prove publication fixture first-import handoff");
    let before_publish: (i64, i64, i64) = catalog
        .connection
        .query_row(
            "SELECT
               (SELECT COUNT(*) FROM library_scan_publication_namespace_bindings
                WHERE scan_id = ?1 AND root_generation = 1),
               (SELECT COUNT(*) FROM library_root_publication_namespaces WHERE root_id = ?2),
               (SELECT generation FROM library_change_root_state WHERE root_id = ?2)",
            rusqlite::params![first.scan_id, root_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("pre-publication binding");
    assert_eq!(before_publish, (1, 0, 1));

    catalog
        .publish_scan(&first.scan_id, root_id, 0, 0)
        .expect("publish first foreground scan");
    let first_publication: (i64, String, String, i64, String) = catalog
        .connection
        .query_row(
            "SELECT proof.root_generation, proof.identity_value, proof.authority_kind,
                    (SELECT COUNT(*) FROM library_scan_publication_namespace_bindings
                     WHERE scan_id = ?1), roots.active_scan_id
             FROM library_root_publication_namespaces AS proof
             JOIN library_roots AS roots ON roots.id = proof.root_id
             WHERE proof.root_id = ?2",
            rusqlite::params![first.scan_id, root_id],
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
        .expect("atomic foreground proof");
    assert_eq!(
        first_publication,
        (
            1,
            first_identity.value.clone(),
            "foreground_scan".to_owned(),
            0,
            first.scan_id.clone(),
        )
    );

    let replacement = fixture_request("publication-replacement", root_path);
    let replacement_identity = publication_identity('2');
    catalog
        .begin_scan_with_publication_namespace(
            &replacement,
            root_id,
            root_path,
            &replacement_identity,
        )
        .expect("bind replacement generation");
    let replacement_binding: (i64, i64, i64, String) = catalog
        .connection
        .query_row(
            "SELECT state.generation,
                    (SELECT COUNT(*) FROM library_root_publication_namespaces
                     WHERE root_id = state.root_id),
                    (SELECT COUNT(*) FROM library_scan_publication_namespace_bindings
                     WHERE scan_id = ?2 AND root_generation = state.generation),
                    roots.active_scan_id
             FROM library_change_root_state AS state
             JOIN library_roots AS roots ON roots.id = state.root_id
             WHERE state.root_id = ?1",
            rusqlite::params![root_id, replacement.scan_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .expect("replacement retirement evidence");
    assert_eq!(
        replacement_binding,
        (2, 0, 1, first.scan_id.clone()),
        "replacement must retire generation-one proof without changing the last trusted catalog"
    );

    catalog
        .abandon_scan(&replacement.scan_id, "failed", 1)
        .expect("cancel replacement scan");
    let after_cancel: (i64, i64, String) = catalog
        .connection
        .query_row(
            "SELECT
               (SELECT COUNT(*) FROM library_scan_publication_namespace_bindings
                WHERE scan_id = ?1),
               (SELECT COUNT(*) FROM library_root_publication_namespaces WHERE root_id = ?2),
               (SELECT active_scan_id FROM library_roots WHERE id = ?2)",
            rusqlite::params![replacement.scan_id, root_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("cancelled publication evidence");
    assert_eq!(after_cancel, (0, 0, first.scan_id));
}
