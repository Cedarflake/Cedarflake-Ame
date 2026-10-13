use super::*;

#[test]
fn unpublished_resume_reuses_unchanged_inspection_after_catalog_reopen() {
    let mut fixture = EntryFixture::new();
    let inspector = RecordingInspector::success(Vec::new());
    let file = fixture.file("image.png");
    let outcome = fixture.apply(file.clone(), &inspector, &mut |_| true);
    assert!(matches!(
        outcome,
        ScanEntryOutcome::Applied {
            accepted_items: 1,
            ..
        }
    ));
    assert_eq!(fixture.staged_count(), 1);
    assert_eq!(inspector.calls.get(), 1);
    reopen_and_resume(&mut fixture);
    fixture.apply(file, &inspector, &mut |_| true);
    assert_eq!(
        inspector.calls.get(),
        1,
        "unchanged retained metadata must not be parsed twice"
    );
    assert_eq!(fixture.staged_count(), 1);
}

#[test]
fn repeated_interruptions_preserve_work_not_yet_revisited() {
    let mut fixture = EntryFixture::new();
    let inspector = RecordingInspector::success(Vec::new());
    let first = fixture.file("first.png");
    let second = fixture.file("second.png");
    fixture.apply(first.clone(), &inspector, &mut |_| true);
    fixture.apply(second.clone(), &inspector, &mut |_| true);
    assert_eq!(fixture.staged_count(), 2);
    reopen_and_resume(&mut fixture);
    fixture.apply(first, &inspector, &mut |_| true);
    assert_eq!(fixture.staged_count(), 2);
    reopen_and_resume(&mut fixture);
    fixture.apply(second, &inspector, &mut |_| true);
    assert_eq!(
        inspector.calls.get(),
        2,
        "both original inspections survive either interruption"
    );
}

#[test]
fn changed_or_missing_file_evidence_requires_fresh_inspection() {
    for boundary in [
        "revision",
        "identity",
        "no_revision",
        "no_identity",
        "engine",
    ] {
        let mut fixture = EntryFixture::new();
        let inspector = RecordingInspector::success(Vec::new());
        let mut file = fixture.file("image.png");
        fixture.apply(file.clone(), &inspector, &mut |_| true);
        assert_eq!(fixture.staged_count(), 1);
        reopen_and_resume(&mut fixture);
        match boundary {
            "revision" => {
                file.source_revision.as_mut().expect("revision").value =
                    "0000000000000002".to_owned()
            }
            "identity" => {
                file.file_identity.as_mut().expect("identity").value =
                    "0000000000000001:00000000000000000000000000000002".to_owned()
            }
            "no_revision" => file.source_revision = None,
            "no_identity" => file.file_identity = None,
            "engine" => {
                rusqlite::Connection::open(fixture.catalog.catalog_path())
                    .expect("fixture connection")
                    .execute(
                        "UPDATE asset_locations SET metadata_engine_version = 'old'",
                        [],
                    )
                    .expect("previous engine output");
            }
            _ => unreachable!(),
        }
        let prepared = PreparedScanFile::load(
            &fixture.catalog,
            &inspector,
            "entry-scan",
            "entry-root",
            file,
            false,
        )
        .expect("prepare retained input");
        prepared.inspect(&inspector).expect("fresh inspection");
        assert_eq!(inspector.calls.get(), 2, "boundary {boundary}");
    }
}

#[test]
fn retained_membership_blocks_publication_until_reconciled() {
    let mut fixture = EntryFixture::new();
    let inspector = RecordingInspector::success(Vec::new());
    fixture.apply(fixture.file("image.png"), &inspector, &mut |_| true);
    assert_eq!(fixture.staged_count(), 1);
    reopen_and_resume(&mut fixture);
    let connection = rusqlite::Connection::open(fixture.catalog.catalog_path())
        .expect("publication guard fixture");
    assert!(
        connection
            .execute(
                "UPDATE scan_runs SET status = 'completed' WHERE id = 'entry-scan'",
                []
            )
            .is_err()
    );
    let published: Option<String> = connection
        .query_row(
            "SELECT active_scan_id FROM library_roots WHERE id = 'entry-root'",
            [],
            |row| row.get(0),
        )
        .expect("publication remains absent");
    assert!(published.is_none());
    drop(connection);
    fixture
        .catalog
        .finish_retained_import_inventory("entry-scan")
        .expect("retire unobserved membership");
    assert_eq!(fixture.staged_count(), 0);
    let connection =
        rusqlite::Connection::open(fixture.catalog.catalog_path()).expect("closed evidence");
    assert_eq!(
        connection
            .query_row("SELECT COUNT(*) FROM assets", [], |row| row
                .get::<_, i64>(0))
            .expect("no orphaned staged asset"),
        0
    );
}

#[test]
fn cancelling_resumption_retires_its_unpublished_records() {
    let mut fixture = EntryFixture::new();
    let inspector = RecordingInspector::success(Vec::new());
    fixture.apply(fixture.file("image.png"), &inspector, &mut |_| true);
    assert_eq!(fixture.staged_count(), 1);
    reopen_and_resume(&mut fixture);
    fixture
        .catalog
        .abandon_scan("entry-scan", "cancelled", 0)
        .expect("cancel retained import");
    let connection =
        rusqlite::Connection::open(fixture.catalog.catalog_path()).expect("closed evidence");
    let counts: (i64, i64, i64) = connection
        .query_row(
            "SELECT (SELECT COUNT(*) FROM assets), (SELECT COUNT(*) FROM asset_locations),
                (SELECT COUNT(*) FROM scan_resume_pending_locations)",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("scoped retirement");
    assert_eq!(counts, (0, 0, 0));
    drop(connection);
    drop(
        SqliteCatalog::open(fixture.catalog.catalog_path().to_owned())
            .expect("valid cancelled catalog"),
    );
}

#[test]
fn failed_staging_keeps_retained_membership_until_atomic_retry() {
    let mut fixture = EntryFixture::new();
    let inspector = RecordingInspector::success(Vec::new());
    let file = fixture.file("image.png");
    fixture.apply(file.clone(), &inspector, &mut |_| true);
    assert_eq!(fixture.staged_count(), 1);
    reopen_and_resume(&mut fixture);
    fixture.apply(file, &inspector, &mut |_| true);
    let connection = rusqlite::Connection::open(fixture.catalog.catalog_path())
        .expect("transaction rollback fixture");
    connection
        .execute_batch(
            "CREATE TRIGGER reject_fixture_staging BEFORE INSERT ON asset_locations
             BEGIN SELECT RAISE(ABORT, 'fixture staging rejected'); END",
        )
        .expect("inject a persistence failure");
    assert!(
        fixture
            .catalog
            .checkpoint_scan("entry-scan", &fixture.checkpoint)
            .is_err()
    );
    let pending_count = || {
        connection
            .query_row(
                "SELECT COUNT(*) FROM scan_resume_pending_locations",
                [],
                |row| row.get::<_, i64>(0),
            )
            .expect("retained membership evidence")
    };
    assert_eq!(pending_count(), 1);
    connection
        .execute_batch("DROP TRIGGER reject_fixture_staging")
        .expect("remove the fixture failure");
    fixture
        .catalog
        .checkpoint_scan("entry-scan", &fixture.checkpoint)
        .expect("retry the same pending write");
    assert_eq!(pending_count(), 0);
    assert_eq!(fixture.staged_count(), 1);
    assert_eq!(inspector.calls.get(), 1);
}

fn reopen_and_resume(fixture: &mut EntryFixture) {
    let root_path = fixture.source.path().to_string_lossy().into_owned();
    let request = ScanRequest {
        scan_id: "entry-scan".to_owned(),
        root_path: root_path.clone(),
        max_items: None,
        max_entries: None,
        preview_edge: 128,
    };
    let catalog_path = fixture.catalog.catalog_path().to_owned();
    fixture
        .catalog
        .checkpoint_scan("entry-scan", &fixture.checkpoint)
        .expect("persist the owned boundary");
    fixture.catalog = SqliteCatalog::open(catalog_path).expect("reopen interrupted import");
    fixture.checkpoint = fixture
        .catalog
        .resume_scan(&request, "entry-root", &root_path)
        .expect("resume interrupted import");
}
