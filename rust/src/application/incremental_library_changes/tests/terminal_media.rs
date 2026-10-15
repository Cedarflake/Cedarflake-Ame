use super::*;

#[test]
fn scan_and_incremental_changes_agree_on_download_errors_with_image_suffixes() {
    use crate::application::{StoragePaths, run_scan_with_storage};
    use crate::domain::ScanEvent;

    let mut fixture = seed_catalog(tempdir().expect("source directory"), &[]);
    write_png_with_format(
        &fixture.source.path().join("valid.data"),
        31,
        17,
        [30, 60, 90],
    );
    let error_bytes = br#"{"retcode":-1,"retmsg":"download failed"}"#;
    let paths = (0..32)
        .map(|index| format!("download-{index:02}.{}", ["jpg", "png"][index % 2]))
        .collect::<Vec<_>>();
    for path in &paths {
        fs::write(fixture.source.path().join(path), error_bytes).expect("download error fixture");
    }
    let catalog_path = fixture.catalog.catalog_path().to_path_buf();
    drop(fixture.catalog);
    let mut completion = None;
    run_scan_with_storage(
        ScanRequest {
            scan_id: "download-errors-scan".to_owned(),
            root_path: fixture.root_path.clone(),
            max_items: None,
            max_entries: None,
            preview_edge: 128,
        },
        |event| {
            if let ScanEvent::Completed { asset_count, .. } = event {
                completion = Some(asset_count);
            }
            true
        },
        StoragePaths {
            catalog_path: catalog_path.clone(),
            preview_root: fixture._storage.path().join("previews"),
            preview_budget_bytes: 64 * 1024 * 1024,
            settings_path: fixture._storage.path().join("settings.sqlite3"),
        },
    )
    .expect("scan excludes unrecognized files");
    assert_eq!(completion, Some(1));
    fixture.catalog = SqliteCatalog::open(catalog_path.clone()).expect("reopen scanned catalog");
    let valid = fixture
        .location("valid.data")
        .expect("recognized image without image suffix");

    let changes = paths
        .iter()
        .enumerate()
        .map(|(index, path)| {
            catch_up_intent(
                &fixture.root_id,
                path,
                u64::try_from(index + 1).expect("sequence"),
            )
        })
        .collect::<Vec<_>>();
    fixture.enqueue(&changes);
    let revision = fixture.revision();
    let report = fixture.process();
    assert_eq!(report.completed_count, 32);
    assert_eq!(report.retried_count, 0);
    assert_eq!(report.applied_mutation_count, 0);
    assert_eq!(fixture.revision(), revision);
    assert_eq!(fixture.location("valid.data"), Some(valid));
    drop(fixture.catalog);
    fixture.catalog = SqliteCatalog::open(catalog_path).expect("reopen completed catch-up");
    assert_eq!(fixture.process().leased_count, 0);
    assert_eq!(
        fixture
            .catalog
            .load_terminal_media_evidence_by_relative_paths(&fixture.root_id, &paths)
            .expect("negative observations survive restart")
            .len(),
        32
    );
    for path in &paths {
        assert!(fixture.location(path).is_none());
        assert_eq!(
            fs::read(fixture.source.path().join(path)).expect("unchanged source"),
            error_bytes
        );
    }
}

#[test]
fn paired_rename_retains_current_negative_evidence_when_previous_path_is_absent() {
    verify_paired_negative_evidence(false);
}

#[test]
fn paired_rename_does_not_publish_previous_negative_evidence_under_current_lease() {
    verify_paired_negative_evidence(true);
}

fn verify_paired_negative_evidence(previous_remains: bool) {
    let source = tempdir().expect("source directory");
    let previous = "previous.data";
    let current = "current.data";
    let previous_bytes = b"ordinary document, not media";
    let current_bytes = b"another ordinary document";
    fs::write(source.path().join(previous), previous_bytes).expect("previous source");
    let mut fixture = seed_catalog(source, &[]);
    fixture.enqueue(&[intent(&fixture.root_id, previous, None, 1)]);
    let initial = fixture.process();
    assert_eq!(initial.completed_count, 1);
    assert_eq!(initial.applied_mutation_count, 0);
    assert_eq!(
        fixture
            .catalog
            .load_terminal_media_evidence_by_relative_paths(
                &fixture.root_id,
                &[previous.to_owned()],
            )
            .expect("previous negative observation")
            .len(),
        1
    );
    if previous_remains {
        fs::write(fixture.source.path().join(current), current_bytes).expect("new current source");
    } else {
        fs::rename(
            fixture.source.path().join(previous),
            fixture.source.path().join(current),
        )
        .expect("controlled source rename");
    }
    let revision = fixture.revision();
    fixture.enqueue(&[intent(&fixture.root_id, current, Some(previous), 2)]);
    let renamed = fixture.process();
    assert_eq!(renamed.completed_count, 1);
    assert_eq!(renamed.retried_count, 0);
    assert_eq!(renamed.applied_mutation_count, 0);
    assert_eq!(
        fixture.revision(),
        revision,
        "negative observations are not gallery mutations"
    );
    let paths = [previous.to_owned(), current.to_owned()];
    let evidence = fixture
        .catalog
        .load_terminal_media_evidence_by_relative_paths(&fixture.root_id, &paths)
        .expect("only current-path evidence belongs to the rename lease");
    assert_eq!(evidence.len(), 1);
    assert_eq!(evidence[0].relative_path, current);
    assert!(evidence[0].file_identity.is_some());
    assert!(evidence[0].source_revision.is_some());
    assert!(evidence[0].source_generation > 0);
    assert!(fixture.location(previous).is_none());
    assert!(fixture.location(current).is_none());
    let catalog_path = fixture.catalog.catalog_path().to_path_buf();
    drop(fixture.catalog);
    fixture.catalog =
        SqliteCatalog::open(catalog_path).expect("full schema reopen after paired completion");
    assert_eq!(
        fixture
            .catalog
            .load_terminal_media_evidence_by_relative_paths(&fixture.root_id, &paths,)
            .expect("durable current negative observation"),
        evidence
    );
    assert_eq!(
        fixture.process().leased_count,
        0,
        "rename does not enter retry growth"
    );
    assert_eq!(
        fs::read(fixture.source.path().join(current)).expect("current bytes"),
        if previous_remains {
            current_bytes.as_slice()
        } else {
            previous_bytes.as_slice()
        }
    );
    if previous_remains {
        assert_eq!(
            fs::read(fixture.source.path().join(previous)).expect("previous bytes"),
            previous_bytes
        );
    } else {
        assert!(!fixture.source.path().join(previous).exists());
    }
    assert_eq!(
        fs::read_dir(fixture.source.path())
            .expect("source entries")
            .count(),
        1 + usize::from(previous_remains)
    );
}

#[test]
fn locked_discovery_new_unknown_suffix_alias_preserves_terminal_identity_fanout() {
    let source = tempdir().expect("source directory");
    write_png(&source.path().join("primary.png"), 4, 3, [30, 40, 50]);
    let mut fixture = seed_catalog(source, &["primary.png"]);
    let original = fixture.location("primary.png").expect("published image");
    let primary = fixture.source.path().join("primary.png");
    let alias = fixture.source.path().join("new.data");
    fs::hard_link(&primary, &alias).expect("new unregistered same-identity alias");
    let corrupt_bytes = b"ordinary bytes replacing known media";
    fs::write(&alias, corrupt_bytes).expect("controlled in-place source change");
    assert!(fixture.location("new.data").is_none());
    fixture.enqueue(&[intent(&fixture.root_id, "new.data", None, 1)]);

    let report = fixture.process();

    assert_eq!(report.completed_count, 1);
    assert_eq!(report.retried_count, 0);
    assert_eq!(report.applied_mutation_count, 1);
    let failed = fixture
        .location("new.data")
        .expect("known identity is not an ordinary document");
    assert_eq!(failed.asset_id, original.asset_id);
    assert_eq!(failed.file_identity, original.file_identity);
    assert!(failed.source_generation > original.source_generation);
    for name in ["primary.png", "new.data"] {
        let location = fixture
            .location(name)
            .expect("both known physical aliases remain visible");
        assert_eq!(location.asset_id, original.asset_id);
        assert_eq!(location.source_generation, failed.source_generation);
        assert_eq!(location.source_revision, failed.source_revision);
        assert!(matches!(location.preview_status, PreviewStatus::Failed));
        assert_eq!(
            location.preview_issue_code.as_deref(),
            Some("media_type_unsupported")
        );
        assert_eq!((location.width, location.height), (0, 0));
        assert!(location.preview_path.is_empty());
        assert_eq!(
            fs::read(fixture.source.path().join(name)).expect("unchanged arranged bytes"),
            corrupt_bytes
        );
    }

    write_png(&primary, 5, 2, [60, 70, 80]);
    let repaired_bytes = fs::read(&primary).expect("repaired fixture bytes");
    fixture.enqueue(&[intent(&fixture.root_id, "new.data", None, 2)]);
    let repaired = fixture.process();
    assert_eq!(repaired.completed_count, 1);
    assert_eq!(repaired.retried_count, 0);
    for name in ["primary.png", "new.data"] {
        let location = fixture.location(name).expect("recovered physical alias");
        assert_eq!(location.asset_id, original.asset_id);
        assert!(location.source_generation > failed.source_generation);
        assert!(matches!(location.preview_status, PreviewStatus::Pending));
        assert_eq!((location.width, location.height), (5, 2));
        assert!(location.preview_issue_code.is_none());
        assert_eq!(
            fs::read(fixture.source.path().join(name)).expect("repaired bytes preserved"),
            repaired_bytes
        );
    }
    assert_eq!(
        fs::read_dir(fixture.source.path())
            .expect("source entries")
            .count(),
        2
    );
}
